// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon tensions` — where your own commitments pull against each other.
//!
//! One call, every live commitment in context. That is affordable because a
//! canon is small by design: thirty commitments is under two thousand tokens,
//! so all-pairs comparison needs no retrieval and no candidate enumerator.
//! Past roughly a hundred it stops being affordable, and that limit is stated
//! in the spec rather than discovered by a user.
//!
//! **What comes back is a proposal, not a ruling.** Every pair is minted as
//! [`Disposition::Open`] and nothing is written to the log: a tension becomes
//! a fact about the canon only when a person runs `accept` or `dismiss`.
//! Pairs already ruled on are filtered out through [`Canon::is_settled`], so
//! a decision someone already made is never re-served as news.

use canon_core::{ActId, Canon, Commitment, Conflict, Disposition};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{self, Client, ModelError};

const SYSTEM: &str = "\
You compare normative commitments and find genuine tensions.

A tension is a pair that cannot both be fully honoured in some realistic \
situation. The reason must name that situation.

Rules:
- Precision over recall. Report a pair only if you can name the situation.
- Commitments about different subjects are not a tension.
- A general rule and a specific case of it are not a tension.
- One commitment being harder to keep than another is not a tension.
- Report each pair once.
- Write the reason as one or two sentences describing the situation. Quote or \
paraphrase a commitment; never refer to one by its number.";

#[derive(Debug, Deserialize)]
struct Found {
    #[serde(default)]
    tensions: Vec<Pair>,
}

#[derive(Debug, Deserialize)]
struct Pair {
    a: usize,
    b: usize,
    #[serde(default)]
    reason: String,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "tensions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "a": { "type": "integer", "description": "number of the first commitment" },
                        "b": { "type": "integer", "description": "number of the second commitment" },
                        "reason": {
                            "type": "string",
                            "description": "the situation in which both cannot be honoured",
                        },
                    },
                    "required": ["a", "b", "reason"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["tensions"],
        "additionalProperties": false,
    })
}

/// Find tensions among these commitments.
///
/// Commitments are numbered rather than identified by hash: a model asked to
/// echo `can-4f19a2b3c1d0` transposes characters, and a transposed id is a
/// tension attributed to the wrong rule. A number out of range is dropped
/// with a warning naming it — an unusable answer is reported, never quietly
/// rounded to a neighbour (§18.3).
///
/// Shared with `draft`, which runs this over what was just accepted.
pub fn detect(client: &Client, commitments: &[&Commitment]) -> Result<Vec<Conflict>, ModelError> {
    if commitments.len() < 2 {
        return Ok(Vec::new());
    }
    let mut user = String::from("Commitments:\n");
    for (i, c) in commitments.iter().enumerate() {
        user.push_str(&format!("{}. {}\n", i + 1, c.text));
    }
    user.push_str("\nReturn every pair in tension, with the situation that forces the choice.");

    let found: Found = client.complete_json(SYSTEM, &user, "tensions", &schema())?;

    let id_of = |n: usize| -> Option<ActId> {
        (n >= 1 && n <= commitments.len()).then(|| commitments[n - 1].id.clone())
    };
    let mut out: Vec<Conflict> = Vec::new();
    for p in found.tensions {
        let (Some(a), Some(b)) = (id_of(p.a), id_of(p.b)) else {
            eprintln!(
                "warning: dropped a proposed tension naming commitment {} and {} — \
                 only 1..{} exist",
                p.a,
                p.b,
                commitments.len()
            );
            continue;
        };
        if a == b {
            eprintln!(
                "warning: dropped a proposed tension of commitment {} with itself",
                p.a
            );
            continue;
        }
        if out.iter().any(|c| c.is_pair(&a, &b)) {
            continue;
        }
        out.push(Conflict {
            a,
            b,
            // `at: 0` because nothing happened: no act was written, so there
            // is no moment to record. The fold never mints this.
            at: 0,
            disposition: Disposition::Open {
                reason: p.reason.trim().to_string(),
            },
        });
    }
    Ok(out)
}

/// Drop pairs someone already ruled on. Returns `(open, settled_count)`.
pub fn unsettled(canon: &Canon, found: Vec<Conflict>) -> (Vec<Conflict>, usize) {
    let before = found.len();
    let open: Vec<Conflict> = found
        .into_iter()
        .filter(|c| !canon.is_settled(&c.a, &c.b))
        .collect();
    let settled = before - open.len();
    (open, settled)
}

/// The shared rendering — `tensions` and the end of `draft` show the same
/// thing, so they call the same code rather than drifting into two versions
/// of it the way `why` once did.
pub fn render(canon: &Canon, open: &[Conflict], settled: usize) -> String {
    let mut out = String::new();
    if open.is_empty() {
        out.push_str("no tensions found.\n");
        if settled > 0 {
            out.push_str(&format!(
                "  {settled} pair(s) were already ruled on and are not re-shown — `canon list --json` for them.\n"
            ));
        }
        return out;
    }
    out.push_str(&format!(
        "{} tension(s) — proposed, and nobody has ruled on them yet.\n\n",
        open.len()
    ));
    let text = |id: &ActId| {
        canon
            .get(id)
            .map(|c| c.text.clone())
            .unwrap_or_else(|| "(not in this canon)".into())
    };
    for c in open {
        out.push_str(&format!("  {}  {}\n", c.a, text(&c.a)));
        out.push_str(&format!("  {}  {}\n", c.b, text(&c.b)));
        if let Disposition::Open { reason } = &c.disposition {
            if !reason.is_empty() {
                out.push_str(&format!("  why: {reason}\n"));
            }
        }
        out.push_str(&format!(
            "  carry it knowingly:  canon accept {} {} -m \"<what this protects>\"\n",
            c.a, c.b
        ));
        out.push_str(&format!(
            "  or it is not one:    canon dismiss {} {}\n\n",
            c.a, c.b
        ));
    }
    if settled > 0 {
        out.push_str(&format!(
            "{settled} further pair(s) were already ruled on and are not re-shown.\n"
        ));
    }
    out
}

pub fn run(args: &[String]) -> i32 {
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return crate::cmds::fail(e),
    };
    let live: Vec<&Commitment> = canon.active().collect();
    let json = crate::cmds::has(args, "--json");
    if live.len() < 2 {
        if json {
            println!("[]");
        } else {
            println!("fewer than two live commitments — nothing to compare.");
        }
        return 0;
    }
    let client = match model::client_for(&dir, crate::cmds::has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!(
        "comparing {} commitments on {}",
        live.len(),
        client.describe()
    );
    let found = match detect(&client, &live) {
        Ok(f) => f,
        Err(e) => return model::report(e),
    };
    let (open, settled) = unsettled(&canon, found);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&open).unwrap_or_else(|_| "[]".into())
        );
    } else {
        print!("{}", render(&canon, &open, settled));
    }
    // Exit 0 even when tensions are found: this is a report, not an
    // adjudication. Exit 1 belongs to `check`, which rules on a proposal —
    // and the personal profile must never emit it at all.
    0
}
