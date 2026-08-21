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

/// A pair the model proposed, still in terms of the list it was given.
///
/// Indices, not ids: `draft` compares candidate texts that have no id yet,
/// and `tensions` compares commitments that do. One engine, and the caller
/// attaches identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposed {
    pub a: usize,
    pub b: usize,
    pub reason: String,
}

/// How many commitments go into one comparison.
///
/// **Measured, not chosen.** One call over sixty commitments asks a model to
/// weigh 1,770 pairs in a single pass, and it does not: replaying the same
/// sixty extracted rules through only this step found 1 of 11 planted
/// tensions in one call, 3 of 11 in blocks of twenty, and 5 of 11 in blocks
/// of twelve — with zero false positives on the seven labelled compatible
/// pairs at every size. Recall was the casualty of the batch size, and
/// discrimination never was. The spec claimed this regime held to about a
/// hundred commitments; it does not, and that claim has been corrected.
const BATCH: usize = 12;

/// The engine: find tensions among these texts, answering in list positions.
///
/// Above [`BATCH`] the comparison is **block-pairwise**: the list is cut into
/// blocks and every block is compared with itself and with every other block,
/// so no pair goes unexamined. That costs `k(k+1)/2` calls for `k` blocks —
/// quadratic, and the reason the spec names a ceiling on how large a canon
/// this tool serves.
pub fn detect_over(client: &Client, texts: &[&str]) -> Result<Vec<Proposed>, ModelError> {
    if texts.len() < 2 {
        return Ok(Vec::new());
    }
    if texts.len() <= BATCH {
        return one_pass(client, texts, &(0..texts.len()).collect::<Vec<_>>());
    }

    let blocks: Vec<Vec<usize>> = (0..texts.len())
        .collect::<Vec<_>>()
        .chunks(BATCH)
        .map(<[usize]>::to_vec)
        .collect();
    let passes = blocks.len() * (blocks.len() + 1) / 2;
    eprintln!(
        "{} commitments is past what one comparison holds — {passes} passes over blocks of {BATCH}",
        texts.len()
    );

    let mut out: Vec<Proposed> = Vec::new();
    let mut done = 0;
    for x in 0..blocks.len() {
        for y in x..blocks.len() {
            let idx: Vec<usize> = if x == y {
                blocks[x].clone()
            } else {
                blocks[x].iter().chain(&blocks[y]).copied().collect()
            };
            done += 1;
            eprint!("\rcomparing {done}/{passes}…");
            let _ = std::io::Write::flush(&mut std::io::stderr());
            for p in one_pass(client, texts, &idx)? {
                // The same pair can surface in more than one pass. First
                // reason wins; a pair is one tension however often it is
                // noticed.
                if !out.iter().any(|q| is_same(q, &p)) {
                    out.push(p);
                }
            }
        }
    }
    eprintln!("\r{passes} passes done, {} pair(s) proposed", out.len());
    Ok(out)
}

fn is_same(a: &Proposed, b: &Proposed) -> bool {
    (a.a, a.b) == (b.a, b.b) || (a.a, a.b) == (b.b, b.a)
}

/// One comparison over the texts at `idx`, answering in GLOBAL positions.
fn one_pass(client: &Client, texts: &[&str], idx: &[usize]) -> Result<Vec<Proposed>, ModelError> {
    if idx.len() < 2 {
        return Ok(Vec::new());
    }
    let mut user = String::from("Commitments:\n");
    for (i, g) in idx.iter().enumerate() {
        user.push_str(&format!("{}. {}\n", i + 1, texts[*g]));
    }
    user.push_str("\nReturn every pair in tension, with the situation that forces the choice.");

    let found: Found = client.complete_json(SYSTEM, &user, "tensions", &schema())?;

    let mut out: Vec<Proposed> = Vec::new();
    for p in found.tensions {
        let in_range = |n: usize| n >= 1 && n <= idx.len();
        if !in_range(p.a) || !in_range(p.b) {
            eprintln!(
                "\nwarning: dropped a proposed tension naming commitment {} and {} — only 1..{} were offered",
                p.a,
                p.b,
                idx.len()
            );
            continue;
        }
        // Back to positions in the caller's list, which is what identity is
        // attached to. A pass never returns its own numbering.
        let (a, b) = (idx[p.a - 1], idx[p.b - 1]);
        if a == b {
            eprintln!("\nwarning: dropped a proposed tension of a commitment with itself");
            continue;
        }
        let proposed = Proposed {
            a,
            b,
            reason: p.reason.trim().to_string(),
        };
        if !out.iter().any(|q| is_same(q, &proposed)) {
            out.push(proposed);
        }
    }
    Ok(out)
}

/// Find tensions among commitments that already have ids.
pub fn detect(client: &Client, commitments: &[&Commitment]) -> Result<Vec<Conflict>, ModelError> {
    let texts: Vec<&str> = commitments.iter().map(|c| c.text.as_str()).collect();
    Ok(detect_over(client, &texts)?
        .into_iter()
        .map(|p| Conflict {
            a: commitments[p.a].id.clone(),
            b: commitments[p.b].id.clone(),
            // `at: 0` because nothing happened: no act was written, so there
            // is no moment to record. The fold never mints this.
            at: 0,
            disposition: Disposition::Open { reason: p.reason },
        })
        .collect())
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

#[cfg(test)]
mod tests;
