// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon rebase --onto <url>@<gen>` — carrying your law onto a different base.
//!
//! Three-way, like every rebase: the seed you adopted, the law you wrote on
//! top of it, and the base you want to land on. The first two are pure
//! computation ([`Divergence`]); the third is not. Deciding whether "quiet
//! hours moved to 10pm on weeknights" still means anything against a charter
//! that reorganised its articles is a semantic question, so it costs one
//! model call — affordable because a canon that fits in a chat message fits
//! in a context window.
//!
//! **It emits a proposal and writes nothing.** Every carried change comes out
//! as a command the person running it can read, edit, or ignore; a conflict
//! comes out marked and with no command at all. Nothing auto-resolves, and
//! anything they do run is authored by them — which is the point, because
//! after a rebase the law is theirs to answer for.
//!
//! The number that matters is the first line: **how much of your law survives
//! before you commit to the move.**

use canon_core::{Canon, Divergence, Fate, Snapshot};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::cmds::{fail, flag, has};
use crate::lineage;
use crate::model::{self, Client, ModelError};

const SYSTEM: &str = "\
You are checking whether changes someone made to a set of rules still apply \
to a different, newer set of rules.

For each change, choose one fate:
- carries: the target has the rule this change was about, and the change \
still makes sense against it. Name the target rule's number.
- already: the target already says what this change was trying to say. It is \
redundant now.
- conflicts: the target says something incompatible with this change.
- orphaned: the target has no rule this change is about.

Rules:
- Judge each change on its own.
- `already` and `conflicts` both require naming a target rule.
- Say which target rule in `target`, or null for orphaned.
- A change that ADDS a new rule is never orphaned. There was no earlier rule for it to be about, so it is `carries` unless the target already says it (`already`) or says something incompatible (`conflicts`).
- One sentence in `because`.";

#[derive(Debug, Deserialize)]
struct Mapped {
    #[serde(default)]
    changes: Vec<MappedOne>,
}

#[derive(Debug, Deserialize)]
struct MappedOne {
    change: usize,
    #[serde(default)]
    fate: String,
    #[serde(default)]
    target: Option<usize>,
    #[serde(default)]
    because: String,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "changes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "change": { "type": "integer", "description": "number of the change" },
                        "fate": {
                            "type": "string",
                            "enum": ["carries", "already", "conflicts", "orphaned"],
                        },
                        "target": {
                            "type": ["integer", "null"],
                            "description": "number of the target rule, or null",
                        },
                        "because": { "type": "string" },
                    },
                    "required": ["change", "fate", "target", "because"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["changes"],
        "additionalProperties": false,
    })
}

/// What sort of change this is. An addition has no earlier rule behind it,
/// which rules out one of the four fates — a constraint worth holding in the
/// type rather than only in the prompt (§7.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    /// Something done to a rule inherited from the seed.
    Edit,
    /// A rule this canon wrote that the seed never had.
    Addition,
}

/// One thing this canon did to its inherited law, in a form both the model
/// and the renderer can read.
#[derive(Debug, Clone)]
struct Change {
    kind: Kind,
    /// What the person did, in one line, for the prompt.
    did: String,
    /// The command that reproduces it against the new base, with `{}` where
    /// the target commitment id goes. `None` for a change that needs no id.
    command: Option<String>,
    /// A command needing no target id (an addition).
    standalone: Option<String>,
}

fn describe(d: &Divergence, canon: &Canon) -> Vec<Change> {
    let mut out = Vec::new();
    for i in &d.inherited {
        match &i.fate {
            Fate::Superseded { text, .. } => out.push(Change {
                kind: Kind::Edit,
                did: format!("replaced \"{}\" with \"{}\"", i.text, text),
                command: Some(format!(
                    "canon supersede {{}} \"{}\" -m \"<why, in your words>\"",
                    text
                )),
                standalone: None,
            }),
            Fate::Retracted => out.push(Change {
                kind: Kind::Edit,
                did: format!("withdrew \"{}\" with no replacement", i.text),
                command: Some("canon retract {} -m \"<why, in your words>\"".into()),
                standalone: None,
            }),
            Fate::Accepted { rationale } => out.push(Change {
                kind: Kind::Edit,
                did: format!(
                    "chose to carry \"{}\" knowingly against another rule ({rationale})",
                    i.text
                ),
                command: Some("canon accept {} <other> -m \"<what this protects>\"".into()),
                standalone: None,
            }),
            Fate::Untouched | Fate::Absent => {}
        }
    }
    for id in &d.added {
        if let Some(c) = canon.get(id) {
            out.push(Change {
                kind: Kind::Addition,
                did: format!("added \"{}\", which the seed did not have", c.text),
                command: None,
                standalone: Some(format!("canon add \"{}\"", c.text)),
            });
        }
    }
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Carries,
    Already,
    Conflicts,
    Orphaned,
}

impl Verdict {
    fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "carries" => Some(Self::Carries),
            "already" => Some(Self::Already),
            "conflicts" => Some(Self::Conflicts),
            "orphaned" => Some(Self::Orphaned),
            _ => None,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Carries => "CARRIES",
            Self::Already => "ALREADY THERE",
            Self::Conflicts => "CONFLICTS",
            Self::Orphaned => "ORPHANED",
        }
    }
}

struct Landed {
    change: Change,
    verdict: Verdict,
    target: Option<usize>,
    because: String,
}

fn map_changes(
    client: &Client,
    changes: &[Change],
    seed: &Snapshot,
    target: &Snapshot,
) -> Result<Vec<Landed>, ModelError> {
    let mut user = String::from("The rules they started from:\n");
    for c in &seed.commitments {
        user.push_str(&format!("- {}\n", c.text));
    }
    user.push_str("\nThe rules they want to land on:\n");
    for (i, c) in target.commitments.iter().enumerate() {
        user.push_str(&format!("{}. {}\n", i + 1, c.text));
    }
    user.push_str("\nThe changes they made:\n");
    for (i, c) in changes.iter().enumerate() {
        user.push_str(&format!("{}. They {}\n", i + 1, c.did));
    }
    user.push_str("\nFor each change, say whether it carries onto the new rules.");

    let mapped: Mapped = client.complete_json(SYSTEM, &user, "changes", &schema())?;

    let mut out = Vec::new();
    for m in mapped.changes {
        let Some(change) = (m.change >= 1).then(|| changes.get(m.change - 1)).flatten() else {
            eprintln!(
                "warning: dropped a mapping for change {} — only 1..{} exist",
                m.change,
                changes.len()
            );
            continue;
        };
        let Some(mut verdict) = Verdict::parse(&m.fate) else {
            eprintln!(
                "warning: dropped a mapping with an unreadable fate `{}`",
                m.fate
            );
            continue;
        };
        // An added rule cannot be orphaned: there was no earlier rule for it
        // to be about. Corrected here rather than trusted to the prompt — and
        // named, because a silent correction is the thing this codebase most
        // wants not to do.
        if change.kind == Kind::Addition && verdict == Verdict::Orphaned {
            eprintln!(
                "note: change {} adds a rule, so `orphaned` is not a fate it can have — \
                 reading it as `carries`",
                m.change
            );
            verdict = Verdict::Carries;
        }
        // A target number out of range is dropped rather than clamped: a
        // rebase pointing at the wrong rule is worse than one that admits it
        // could not place a change.
        let target_no = m
            .target
            .filter(|n| *n >= 1 && *n <= target.commitments.len());
        // An addition legitimately places nowhere: it is new law, and the
        // whole point is that the target does not have it.
        if change.kind == Kind::Edit && verdict != Verdict::Orphaned && target_no.is_none() {
            eprintln!(
                "warning: `{}` for change {} named no usable target rule — treating it as \
                 unplaced, which you have to resolve by hand",
                m.fate, m.change
            );
        }
        out.push(Landed {
            change: change.clone(),
            verdict,
            target: target_no,
            because: m.because.trim().to_string(),
        });
    }
    Ok(out)
}

pub fn run(args: &[String]) -> i32 {
    let Some(onto) = flag(args, "--onto") else {
        return fail("usage: canon rebase --onto <url>@<generation>");
    };
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let seed = match lineage::load_seed(&dir) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let (url, generation) = match onto.rsplit_once('@') {
        Some((u, g)) if !u.is_empty() && !g.contains('/') && !g.contains(':') => (u, Some(g)),
        _ => (onto, None),
    };
    let (target, _) = match lineage::fetch(&dir, url, generation) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };

    let d = Divergence::compute(&seed, &canon);
    let changes = describe(&d, &canon);
    if changes.is_empty() {
        println!(
            "nothing to rebase — this canon has not changed anything it inherited from {}@{}.",
            seed.lineage, seed.generation
        );
        println!(
            "  `canon upgrade {}` takes the newer generation directly.",
            generation.unwrap_or("<gen>")
        );
        return 0;
    }

    let client = match model::client_for(&dir, has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!(
        "mapping {} local change(s) onto {}@{} on {}",
        changes.len(),
        target.lineage,
        target.generation,
        client.describe()
    );
    let landed = match map_changes(&client, &changes, &seed, &target) {
        Ok(v) => v,
        Err(e) => return model::report(e),
    };

    if has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&payload(&target, &landed)).unwrap_or_default()
        );
    } else {
        print!("{}", render(&seed, &target, &changes, &landed));
    }
    // Exit 1 when something conflicts, so a script can tell a clean rebase
    // from one needing a person. Not 2: the changes are not unaddressed, they
    // are addressed and they collide.
    if landed.iter().any(|l| l.verdict == Verdict::Conflicts) {
        return 1;
    }
    0
}

fn payload(target: &Snapshot, landed: &[Landed]) -> Value {
    json!({
        "onto": { "lineage": target.lineage, "generation": target.generation },
        "changes": landed
            .iter()
            .map(|l| json!({
                "change": l.change.did,
                "fate": l.verdict.label().to_ascii_lowercase(),
                "target": l.target.and_then(|n| target.commitments.get(n - 1)).map(|c| c.id.to_string()),
                "because": l.because,
            }))
            .collect::<Vec<_>>(),
    })
}

fn render(seed: &Snapshot, target: &Snapshot, changes: &[Change], landed: &[Landed]) -> String {
    let carries = landed
        .iter()
        .filter(|l| l.verdict == Verdict::Carries)
        .count();
    let already = landed
        .iter()
        .filter(|l| l.verdict == Verdict::Already)
        .count();
    let conflicts = landed
        .iter()
        .filter(|l| l.verdict == Verdict::Conflicts)
        .count();
    let orphaned = landed
        .iter()
        .filter(|l| l.verdict == Verdict::Orphaned)
        .count();
    // A change the model said nothing about is neither carried nor conflicted
    // — it is unmapped, and reported as such rather than counted as a pass.
    let unmapped = changes.len().saturating_sub(landed.len());

    let mut out = format!(
        "rebase proposal · {}@{} -> {}@{}\n\n",
        seed.lineage, seed.generation, target.lineage, target.generation
    );
    out.push_str(&format!(
        "{carries} of {} of your changes carry. {already} already there, {conflicts} conflict, \
         {orphaned} orphaned",
        changes.len()
    ));
    if unmapped > 0 {
        out.push_str(&format!(", {unmapped} unmapped"));
    }
    out.push_str(".\n");
    out.push_str("Nothing has been written. Run the commands you agree with.\n");

    for l in landed {
        let target_line = l
            .target
            .and_then(|n| target.commitments.get(n - 1))
            .map(|c| format!("    onto {}  \"{}\"\n", c.id, c.text))
            .unwrap_or_default();
        out.push_str(&format!(
            "\n  {}  you {}\n",
            l.verdict.label(),
            l.change.did
        ));
        out.push_str(&target_line);
        if !l.because.is_empty() {
            out.push_str(&format!("    {}\n", l.because));
        }
        match l.verdict {
            Verdict::Carries => {
                if let Some(cmd) = &l.change.standalone {
                    out.push_str(&format!("    {cmd}\n"));
                } else if let (Some(cmd), Some(n)) = (&l.change.command, l.target) {
                    if let Some(c) = target.commitments.get(n - 1) {
                        out.push_str(&format!("    {}\n", cmd.replace("{}", c.id.as_str())));
                    }
                }
            }
            // Deliberately no command. A conflict is a decision, and printing
            // something runnable next to it invites resolving it by reflex.
            Verdict::Conflicts => out.push_str("    decide this one yourself.\n"),
            Verdict::Already => out.push_str("    nothing to do.\n"),
            Verdict::Orphaned => {
                out.push_str("    the rule this was about is gone from the new base.\n")
            }
        }
    }
    out.push_str(
        "\nEvery act you run is authored by you. After a rebase the law is yours to \
                  answer for.\n",
    );
    out
}

#[cfg(test)]
mod tests;
