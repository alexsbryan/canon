// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon check` — the adjudication, and the three ways of saying it.
//!
//! One model call. Every live commitment goes into context and the answer
//! comes back as bearings: which commitments touch this proposal, which way
//! each pulls, and why. [`Standing::cited`] then refuses any position naming a
//! commitment this canon does not have or giving no reason, so a rendered
//! conflict always points at a rule a person can read.
//!
//! **The profiles are not cosmetic.** `code` renders a verdict because a
//! codebase wants one and CI reads exit codes. `house` renders which ACT the
//! proposal needs, because a household's output is an agenda. `personal`
//! renders stakes and **never a verdict, never exit 1** — a tool that ruled
//! on someone's inner life would do harm the codebase profile cannot, and
//! that is enforced here by [`exit_code`] and pinned by a test.

use canon_core::{Canon, Commitment, Disposition, Outcome, Position, Pull, Standing, Status};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{self, Client, ModelError};
use crate::profile::Profile;
use crate::store;

const SYSTEM: &str = "\
You judge how a proposal sits against a body of commitments.

For every commitment that bears on the proposal, return:
- commitment: its number
- pull: \"against\" if acting on the proposal would break or strain it, \
\"toward\" if the proposal serves it
- because: one sentence naming what in the proposal touches what in the \
commitment

Rules:
- Only commitments that genuinely bear on the proposal. Most will not.
- Never cite a commitment you cannot give a reason for.
- If nothing bears on the proposal, return an empty list. That is an answer.
- Judge the proposal as written. Do not improve it.";

/// The model's answer shape.
///
/// **Still called `bearings` on the wire, deliberately.** A `Position` may be
/// sourced from a commitment or from an actor; this reader only ever produces
/// the first kind, because reading a canon is the only thing it does. Renaming
/// the schema key would change the prompt, and the prompt is measured. The two
/// names are not one concept spelled twice: a bearing is what a commitment has
/// on a proposal, and it is a strict subset of what a position can be.
#[derive(Debug, Deserialize)]
struct Judged {
    #[serde(default)]
    bearings: Vec<JudgedOne>,
}

#[derive(Debug, Deserialize)]
struct JudgedOne {
    commitment: usize,
    #[serde(default)]
    pull: String,
    #[serde(default)]
    because: String,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "bearings": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "commitment": { "type": "integer", "description": "number of the commitment" },
                        "pull": { "type": "string", "enum": ["toward", "against"] },
                        "because": { "type": "string" },
                    },
                    "required": ["commitment", "pull", "because"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["bearings"],
        "additionalProperties": false,
    })
}

/// Ask how this proposal stands. Returns the standing and anything refused.
pub fn assess(
    client: &Client,
    canon: &Canon,
    proposal: &str,
) -> Result<(Standing, Vec<Position>), ModelError> {
    let live: Vec<&Commitment> = canon.active().collect();
    let mut user = String::from("Commitments:\n");
    for (i, c) in live.iter().enumerate() {
        user.push_str(&format!("{}. {}\n", i + 1, c.text));
    }
    user.push_str(&format!("\nProposal:\n{proposal}\n"));
    let judged: Judged = client.complete_json(SYSTEM, &user, "bearings", &schema())?;

    let mut positions = Vec::new();
    for b in judged.bearings {
        // An out-of-range number is dropped rather than clamped: clamping
        // attributes a conflict to whichever rule happens to be last.
        let Some(c) = (b.commitment >= 1)
            .then(|| live.get(b.commitment - 1))
            .flatten()
        else {
            eprintln!(
                "warning: dropped a position naming commitment {} — only 1..{} exist",
                b.commitment,
                live.len()
            );
            continue;
        };
        let pull = match b.pull.trim().to_ascii_lowercase().as_str() {
            "against" => Pull::Against,
            "toward" | "towards" | "for" => Pull::Toward,
            other => {
                eprintln!("warning: dropped a position with an unreadable pull `{other}`");
                continue;
            }
        };
        positions.push(Position::of(c.id.clone(), pull, b.because.trim()));
    }
    Ok(Standing::cited(canon, proposal, positions))
}

/// The exit-code contract, decided in ONE place.
///
/// `Personal` returns 0 whatever the outcome. That is the invariant, not a
/// default: a personal canon reports stakes, and an exit 1 is a machine
/// saying a life choice failed a check.
pub fn exit_code(profile: Profile, outcome: Outcome) -> i32 {
    match (profile, outcome) {
        (Profile::Personal, _) => 0,
        (_, Outcome::Supported) => 0,
        (_, Outcome::Conflicts) => 1,
        (_, Outcome::Unaddressed) => 2,
    }
}

fn cite(canon: &Canon, b: &Position, indent: &str) -> String {
    // A position an ACTOR took cites no commitment, so there is nothing for
    // this renderer to quote. It is rendered where actors are rendered, not
    // silently as a rule that does not exist.
    let Some(c) = b.commitment().and_then(|id| canon.get(id)) else {
        return String::new();
    };
    let status = match &c.status {
        Status::Active => "in force, never superseded".to_string(),
        Status::Superseded { by } => format!("superseded by {by}"),
        Status::Retracted { at } => format!("retracted {}", store::ymd(*at)),
    };
    format!(
        "{indent}{}  \"{}\"\n{indent}{}asserted {}, {status}\n{indent}{}because: {}\n",
        c.id,
        c.text,
        " ".repeat(c.id.as_str().len() + 2),
        store::ymd(c.asserted_at),
        " ".repeat(c.id.as_str().len() + 2),
        b.because,
    )
}

/// Contradictions among the cited commitments that someone already chose to
/// carry. Without this the personal profile re-litigates a decision its user
/// already made, which is the one thing it must not do.
fn carried(canon: &Canon, standing: &Standing) -> Vec<String> {
    let mut out = Vec::new();
    for c in canon.tolerated() {
        let both = standing
            .cited_commitments()
            .any(|p| p.commitment() == Some(&c.a))
            && standing
                .cited_commitments()
                .any(|p| p.commitment() == Some(&c.b));
        if !both {
            continue;
        }
        if let Disposition::Tolerated { rationale, revisit } = &c.disposition {
            let when = store::ymd(c.at);
            out.push(match revisit {
                Some(r) => format!("accepted {when}: \"{rationale}\" (revisit by {r})"),
                None => format!("accepted {when}: \"{rationale}\""),
            });
        }
    }
    out
}

pub fn render(profile: Profile, canon: &Canon, standing: &Standing) -> String {
    match profile {
        Profile::Code => render_code(canon, standing),
        Profile::House => render_house(canon, standing),
        Profile::Personal => render_personal(canon, standing),
    }
}

fn render_code(canon: &Canon, standing: &Standing) -> String {
    let mut out = String::new();
    match standing.outcome() {
        Outcome::Conflicts => {
            out.push_str("CONFLICT\n");
            for b in standing.against() {
                out.push_str(&cite(canon, b, "  "));
            }
            let toward: Vec<&Position> = standing.toward().collect();
            if !toward.is_empty() {
                out.push_str("\nalso bears on it, in favour:\n");
                for b in toward {
                    out.push_str(&cite(canon, b, "  "));
                }
            }
        }
        Outcome::Supported => {
            out.push_str("SUPPORTED\n");
            for b in standing.toward() {
                out.push_str(&cite(canon, b, "  "));
            }
        }
        Outcome::Unaddressed => {
            // Not approval. The canon is silent, and silence is reported as
            // silence.
            out.push_str("UNADDRESSED\n");
            out.push_str("  nothing in this canon bears on this proposal.\n");
            out.push_str(&format!(
                "  record the gap:  canon question \"{}\"\n",
                standing.proposal
            ));
        }
    }
    out
}

fn render_house(canon: &Canon, standing: &Standing) -> String {
    // A household's output is an agenda: which act does this need?
    let mut out = String::new();
    match standing.outcome() {
        Outcome::Conflicts => {
            out.push_str("THIS NEEDS AN AMENDMENT\n");
            out.push_str("  it runs against a rule the house already has:\n\n");
            for b in standing.against() {
                out.push_str(&cite(canon, b, "  "));
                if let Some(c) = b.commitment().and_then(|id| canon.get(id)) {
                    out.push_str(&format!(
                        "  amend it:  canon supersede {} \"<the new rule>\" -m \"<why>\"\n",
                        c.id
                    ));
                    out.push_str(&format!(
                        "  or carry both knowingly:  canon accept {} <other> -m \"<what this protects>\"\n\n",
                        c.id
                    ));
                }
            }
        }
        Outcome::Supported => {
            out.push_str("ALREADY COVERED\n");
            out.push_str("  no act needed; the house has already decided this:\n\n");
            for b in standing.toward() {
                out.push_str(&cite(canon, b, "  "));
            }
        }
        Outcome::Unaddressed => {
            out.push_str("THIS NEEDS A NEW RULE\n");
            out.push_str("  nothing the house has decided bears on it.\n\n");
            out.push_str(&format!(
                "  write one:  canon add \"<the rule>\"\n  or record the gap for the next meeting:  canon question \"{}\"\n",
                standing.proposal
            ));
        }
    }
    out
}

fn render_personal(canon: &Canon, standing: &Standing) -> String {
    // Never a verdict. No CONFLICT, no SUPPORTED, no ruling — what has a
    // stake in this, and which way each pulls.
    let mut out = String::new();
    if standing.positions.is_empty() {
        out.push_str("  NOTHING WITH A STAKE\n");
        out.push_str("    none of your commitments speak to this either way.\n");
        return out;
    }
    out.push_str("  STAKE\n");
    let width = standing
        .positions
        .iter()
        .filter_map(|b| b.commitment().and_then(|id| canon.get(id)))
        .map(|c| c.text.chars().count())
        .max()
        .unwrap_or(0)
        .min(52);
    for b in &standing.positions {
        let Some(c) = b.commitment().and_then(|id| canon.get(id)) else {
            continue;
        };
        let arrow = match b.pull {
            Pull::Against => "pulls against",
            Pull::Toward => "pulls toward",
        };
        let quoted = format!("\"{}\"", c.text);
        out.push_str(&format!(
            "    {}  {:<width$}  <- {arrow}\n",
            c.id,
            quoted,
            width = width + 2
        ));
        // Aligned under the commitment text, not under its id: the reason
        // belongs to the words, and a reader scans the left edge of the text.
        out.push_str(&format!(
            "    {}  {}\n",
            " ".repeat(c.id.as_str().len()),
            b.because
        ));
    }
    for line in carried(canon, standing) {
        out.push_str(&format!("    {line}\n"));
    }
    out
}

/// The `--json` payload.
///
/// The personal profile's payload carries NO `outcome` field. An outcome is a
/// verdict whichever way it is serialized, and "never renders a verdict" has
/// to hold for the machine-readable surface too or it does not hold.
pub fn payload(profile: Profile, standing: &Standing) -> Value {
    let mut v = json!({
        "proposal": standing.proposal,
        "profile": profile.as_str(),
        "positions": standing.positions,
    });
    if profile != Profile::Personal {
        v["outcome"] = serde_json::to_value(standing.outcome()).unwrap_or(Value::Null);
    }
    v
}

pub fn run(args: &[String]) -> i32 {
    let pos = crate::cmds::positionals(args);
    let Some(proposal) = pos.first() else {
        return crate::cmds::fail("usage: canon check \"<proposal>\"");
    };
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return crate::cmds::fail(e),
    };
    let profile = match Profile::load(&dir) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(e),
    };
    if canon.active().next().is_none() {
        eprintln!("this canon has no live commitments — nothing to check against.");
        return 2;
    }
    let client = match model::client_for(&dir, crate::cmds::has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!("checking on {}", client.describe());
    let (standing, refused) = match assess(&client, &canon, proposal) {
        Ok(v) => v,
        Err(e) => return model::report(e),
    };
    // Refusals are named, not swallowed: a shorter answer with no explanation
    // is indistinguishable from a canon that had less to say.
    for b in &refused {
        // The reason has to match what was actually checked. A drop message
        // built from a different field than the comparison is how a real
        // mismatch renders as nothing and hides itself — the support step
        // shipped exactly that defect once.
        let (what, why) = match &b.source {
            canon_core::Source::Commitment(id) => (
                id.to_string(),
                if b.because.trim().is_empty() {
                    "no reason given"
                } else {
                    "not a commitment in this canon"
                },
            ),
            canon_core::Source::Actor(a) => (
                format!("`{a}`"),
                if b.because.trim().is_empty() {
                    "no reason given"
                } else {
                    "no actor named"
                },
            ),
        };
        eprintln!("warning: refused an uncitable position on {what} — {why}");
    }
    if crate::cmds::has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload(profile, &standing)).unwrap_or_default()
        );
    } else {
        print!("{}", render(profile, &canon, &standing));
    }
    exit_code(profile, standing.outcome())
}

#[cfg(test)]
mod tests;
