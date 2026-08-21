// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lineages: the snapshot format, and how a canon has diverged from its seed.
//!
//! Both are pure. A snapshot renders to and parses from a block of text, and
//! a divergence is arithmetic over two sets of commitments — **no model, and
//! no network**. Aggregated across many canons the divergence is convergent
//! divergence — *forty houses independently superseded Article 9 in the same
//! direction, so Article 9 is wrong* — and the thing that makes that evidence
//! rather than opinion is that it is counting, not judgement.
//!
//! **A snapshot is not a log.** It carries derived current state and drops
//! supersession history, rationales, and the reasoning behind tolerated
//! contradictions — the parts that name incidents and people. Enough to
//! **adopt**, not enough to **audit**, which is the right trade for a block
//! of text pasted into a chat thread.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::fold::{Canon, Disposition, Status};
use crate::id::ActId;

/// One commitment as it travels: its text, and the id it had upstream.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotCommitment {
    pub id: ActId,
    pub text: String,
}

/// What is live in a canon at one moment, in a form that can be pasted into a
/// chat thread and read back out.
///
/// This is also the three-way base stored at `.canon/upstream/seed.json`
/// after adopting: the same noun for what you send, what you receive, and
/// what you later diff against, rather than three near-identical records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub lineage: String,
    /// Content-addressed generation. Two people who share the same rules
    /// produce the same generation, which is what makes "are we on the same
    /// version?" answerable without a registry.
    pub generation: String,
    pub profile: String,
    /// When the snapshot was taken. Unix seconds.
    pub at: i64,
    pub commitments: Vec<SnapshotCommitment>,
}

/// Length of the generation digest. Six hex characters is enough to tell two
/// versions of a house's rules apart and short enough to read aloud.
const GENERATION: usize = 6;

impl Snapshot {
    /// Take a snapshot of what is live now.
    pub fn of(
        canon: &Canon,
        lineage: impl Into<String>,
        profile: impl Into<String>,
        at: i64,
    ) -> Self {
        let commitments: Vec<SnapshotCommitment> = canon
            .active()
            .map(|c| SnapshotCommitment {
                id: c.id.clone(),
                text: c.text.clone(),
            })
            .collect();
        Self {
            lineage: lineage.into(),
            generation: generation_of(&commitments),
            profile: profile.into(),
            at,
            commitments,
        }
    }

    /// Render the pasteable block.
    ///
    /// Readable by a person scrolling a thread, parseable back by the tool.
    /// No attachment, no link, no auth, nothing to rot.
    pub fn render(&self, date: &str) -> String {
        let mut out = format!(
            "--- canon {} · {} · snapshot {} · {}\n",
            self.lineage, self.profile, date, self.generation
        );
        for c in &self.commitments {
            out.push_str(&format!("{}  ({})\n", c.text, c.id));
        }
        out.push_str(&format!(
            "--- {} live · adopt: canon adopt --paste\n",
            self.commitments.len()
        ));
        out
    }

    /// Parse a pasted block back.
    ///
    /// Tolerates leading and trailing chatter, because it arrives out of a
    /// chat thread with a "here you go" above it. It does NOT tolerate a
    /// generation that does not match the commitments: a block someone edited
    /// by hand in the thread is refused rather than adopted as though it were
    /// what the sender published.
    pub fn parse(block: &str) -> Result<Self, String> {
        let lines: Vec<&str> = block.lines().collect();
        let start = lines
            .iter()
            .position(|l| l.trim_start().starts_with("--- canon "))
            .ok_or("no canon snapshot here — the block starts `--- canon <name> · ...`")?;
        let end = lines
            .iter()
            .enumerate()
            .skip(start + 1)
            .find(|(_, l)| l.trim_start().starts_with("---"))
            .map(|(i, _)| i)
            .ok_or("the snapshot has no closing `--- N live` line")?;

        let header = lines[start]
            .trim_start()
            .trim_start_matches("--- canon ")
            .trim();
        let fields: Vec<&str> = header.split('·').map(str::trim).collect();
        let lineage = fields.first().copied().unwrap_or("").to_string();
        if lineage.is_empty() {
            return Err("the snapshot names no lineage".into());
        }
        let profile = fields.get(1).copied().unwrap_or("personal").to_string();
        let declared = fields.get(3).copied().map(str::to_string);

        let mut commitments = Vec::new();
        for line in &lines[start + 1..end] {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // `<text>  (<id>)` — the id is last so the text can contain
            // anything, including parentheses.
            let open = line
                .rfind(" (")
                .ok_or_else(|| format!("no id on `{line}` — expected `<text>  (<id>)`"))?;
            let id = line[open + 2..].trim_end_matches(')').trim();
            if !line.ends_with(')') || id.is_empty() {
                return Err(format!("no id on `{line}` — expected `<text>  (<id>)`"));
            }
            commitments.push(SnapshotCommitment {
                id: ActId::from_raw(id),
                text: line[..open].trim().to_string(),
            });
        }
        if commitments.is_empty() {
            return Err("the snapshot carries no commitments".into());
        }
        let generation = generation_of(&commitments);
        if let Some(d) = declared.filter(|d| !d.is_empty()) {
            if d != generation {
                return Err(format!(
                    "this block says generation {d} but its commitments hash to {generation} — \
                     it was edited after it was shared. Ask for it again rather than adopting it."
                ));
            }
        }
        Ok(Self {
            lineage,
            generation,
            profile,
            at: 0,
            commitments,
        })
    }
}

/// The generation of a set of commitments: a digest over `(id, text)` pairs.
///
/// Order-independent, because two people holding the same rules in a
/// different order are on the same generation.
///
/// **The text is in the digest and has to be.** Hashing only the ids looks
/// sufficient — ids are already content hashes — but the ids in a pasted
/// block are just characters someone can retype. A block whose text was
/// edited in the thread would carry the ids it arrived with and hash
/// identically, which is exactly the tampering the generation exists to
/// catch.
pub fn generation_of(commitments: &[SnapshotCommitment]) -> String {
    let mut pairs: Vec<String> = commitments
        .iter()
        .map(|c| format!("{}\u{1f}{}", c.id, c.text))
        .collect();
    pairs.sort_unstable();
    let digest = Sha256::digest(pairs.join("|").as_bytes());
    digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>()
        .chars()
        .take(GENERATION)
        .collect()
}

/// How one inherited commitment fared.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "fate", rename_all = "snake_case")]
pub enum Fate {
    /// Still exactly as it arrived.
    Untouched,
    /// Replaced locally, and by what.
    Superseded { by: ActId, text: String },
    /// Withdrawn locally with no replacement.
    Retracted,
    /// Carried knowingly against another commitment.
    Accepted { rationale: String },
    /// In the seed and not in this canon. Several causes, and the fold
    /// cannot tell them apart: a line lost in a paste, an adoption that
    /// skipped it, or an `upgrade` that held it back because this canon had
    /// already changed the rule it replaced. Reported as the fact it is,
    /// without a story about how it happened.
    Absent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Inherited {
    /// The upstream id, which is what a lineage maintainer recognises.
    pub upstream: ActId,
    pub text: String,
    pub fate: Fate,
}

/// How a canon has diverged from the seed it adopted.
///
/// Pure computation over two logs. This is the strategically important
/// command: aggregated across adopters it says which articles a tradition
/// keeps having to rewrite.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Divergence {
    pub lineage: String,
    pub generation: String,
    pub inherited: Vec<Inherited>,
    /// Live commitments that came from nowhere upstream.
    pub added: Vec<ActId>,
}

impl Divergence {
    pub fn compute(seed: &Snapshot, canon: &Canon) -> Self {
        let mut inherited = Vec::new();
        for s in &seed.commitments {
            // The link is `from`, recorded in the act, NOT position or text
            // matching: a canon that arrived by paste has no git history, and
            // text matching would call an edited rule a different rule.
            let local = canon
                .commitments
                .iter()
                .find(|c| c.from.as_ref() == Some(&s.id));
            let fate = match local {
                None => Fate::Absent,
                Some(c) => match &c.status {
                    Status::Retracted { .. } => Fate::Retracted,
                    Status::Superseded { by } => Fate::Superseded {
                        by: by.clone(),
                        text: canon.get(by).map(|n| n.text.clone()).unwrap_or_default(),
                    },
                    Status::Active => canon
                        .tolerated()
                        .find(|x| x.a == c.id || x.b == c.id)
                        .and_then(|x| match &x.disposition {
                            Disposition::Tolerated { rationale, .. } => Some(Fate::Accepted {
                                rationale: rationale.clone(),
                            }),
                            _ => None,
                        })
                        .unwrap_or(Fate::Untouched),
                },
            };
            inherited.push(Inherited {
                upstream: s.id.clone(),
                text: s.text.clone(),
                fate,
            });
        }
        // A commitment is ADDED only if nothing in its ancestry came from the
        // seed. A supersession of an inherited rule mints a commitment with
        // no `from` of its own, and counting that as an addition would report
        // every rewrite twice — once as a supersession and once as new law
        // the adopter invented.
        let mut descends: std::collections::BTreeSet<ActId> = canon
            .commitments
            .iter()
            .filter(|c| {
                c.from
                    .as_ref()
                    .is_some_and(|f| seed.commitments.iter().any(|s| &s.id == f))
            })
            .map(|c| c.id.clone())
            .collect();
        // Transitive: superseding a supersession is still not an addition.
        loop {
            let grown: Vec<ActId> = canon
                .commitments
                .iter()
                .filter(|c| !descends.contains(&c.id))
                .filter(|c| c.replaces.iter().any(|r| descends.contains(r)))
                .map(|c| c.id.clone())
                .collect();
            if grown.is_empty() {
                break;
            }
            descends.extend(grown);
        }
        let added = canon
            .active()
            .filter(|c| !descends.contains(&c.id))
            .map(|c| c.id.clone())
            .collect();
        Self {
            lineage: seed.lineage.clone(),
            generation: seed.generation.clone(),
            inherited,
            added,
        }
    }

    pub fn count(&self, f: impl Fn(&Fate) -> bool) -> usize {
        self.inherited.iter().filter(|i| f(&i.fate)).count()
    }
}

#[cfg(test)]
mod tests;
