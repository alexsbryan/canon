// SPDX-License-Identifier: AGPL-3.0-or-later
//! The fold: current state as a pure function of the acts.
//!
//! No IO, no inference, no stored mutable state. Everything a renderer shows —
//! what is live, what replaced what and why, which contradictions are carried
//! knowingly — comes from replaying the log.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::act::{Act, ActKind};
use crate::id::ActId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    Active,
    Superseded { by: ActId },
    Retracted { at: i64 },
}

/// A commitment as it stands now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub id: ActId,
    pub text: String,
    pub status: Status,
    pub asserted_at: i64,
    pub actor: String,
    /// Commitments this one replaced, when it arrived via `Supersede`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<ActId>,
    /// Upstream provenance, when inherited from an adopted seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from: Option<ActId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// A contradiction the holder carries knowingly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tolerated {
    pub a: ActId,
    pub b: ActId,
    pub rationale: String,
    pub revisit: Option<String>,
    pub at: i64,
}

/// Where this canon came from, if it was adopted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ancestry {
    pub lineage: String,
    pub generation: String,
    pub source: Option<String>,
    pub at: i64,
}

/// The derived read-model.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub commitments: Vec<Commitment>,
    pub tolerated: Vec<Tolerated>,
    /// Pairs explicitly judged not-a-conflict. Renderers use this to stop
    /// re-surfacing noise a person already dismissed.
    pub dismissed: Vec<(ActId, ActId)>,
    pub ancestry: Option<Ancestry>,
    /// Adjudications not authored by a person. Reported, never hidden —
    /// absence of attribution is surfaced rather than defaulted.
    pub unattended: Vec<ActId>,
    /// `(act, missing target)` — an act referencing a commitment that is not
    /// in this log. A truncated or hand-edited file, or a snapshot adopted
    /// without its history. Surfaced rather than silently ignored: a
    /// supersession whose target is absent is a hole in the record, not a
    /// no-op.
    pub dangling: Vec<(ActId, ActId)>,
}

impl State {
    pub fn active(&self) -> impl Iterator<Item = &Commitment> {
        self.commitments
            .iter()
            .filter(|c| matches!(c.status, Status::Active))
    }

    pub fn get(&self, id: &ActId) -> Option<&Commitment> {
        self.commitments.iter().find(|c| &c.id == id)
    }

    /// Is this pair already settled — carried knowingly, or dismissed?
    pub fn is_settled(&self, a: &ActId, b: &ActId) -> bool {
        let hit = |x: &ActId, y: &ActId| (x == a && y == b) || (x == b && y == a);
        self.tolerated.iter().any(|t| hit(&t.a, &t.b))
            || self.dismissed.iter().any(|(x, y)| hit(x, y))
    }
}

/// Derive current state from the acts.
///
/// Acts are folded in the order given; [`crate::Log`] sorts by `(ts_unix, id)`
/// on parse, so the result is identical regardless of how lines interleaved
/// during a merge.
pub fn derive(acts: &[Act]) -> State {
    let n = acts.len();

    // Pass 1 — liveness.
    //
    // An act is dead iff some LIVE `Revert` targets it; a `Revert` cancelled
    // by another live `Revert` has no effect, which is revert-of-revert
    // re-applying the originals.
    //
    // This is resolved by reference rather than by position. A backward walk
    // over the sorted acts looks correct and is not: acts routinely share a
    // second, and the id tiebreak can place a `Revert` ahead of the very act
    // it cancels. The recursion is well-founded because an act can only
    // reference an id that already existed when it was written.
    let mut reverters: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (j, act) in acts.iter().enumerate() {
        if let ActKind::Revert { targets, .. } = &act.kind {
            for t in targets {
                reverters.entry(t.as_str()).or_default().push(j);
            }
        }
    }

    #[derive(Clone, Copy, PartialEq)]
    enum Mark {
        Unknown,
        Walking,
        Known(bool),
    }
    let mut mark = vec![Mark::Unknown; n];

    fn resolve(
        i: usize,
        acts: &[Act],
        reverters: &BTreeMap<&str, Vec<usize>>,
        mark: &mut Vec<Mark>,
    ) -> bool {
        match mark[i] {
            Mark::Known(v) => return v,
            // A cycle would mean an act referencing an id minted after it,
            // which content-addressing makes impossible. Fail live rather
            // than looping.
            Mark::Walking => return true,
            Mark::Unknown => {}
        }
        mark[i] = Mark::Walking;
        let mut alive = true;
        if let Some(js) = reverters.get(acts[i].id.as_str()) {
            for &j in js {
                if resolve(j, acts, reverters, mark) {
                    alive = false;
                    break;
                }
            }
        }
        mark[i] = Mark::Known(alive);
        alive
    }

    let live: Vec<bool> = (0..n)
        .map(|i| resolve(i, acts, &reverters, &mut mark))
        .collect();

    // Pass 2 — introduce every commitment BEFORE applying any status change.
    //
    // Acts are sorted by `(ts_unix, id)`, which is deterministic but NOT
    // causal: several acts routinely share a second, and an id-tiebreak can
    // place a supersession ahead of the very commitment it retires. Splitting
    // introduction from effect makes the fold independent of order within a
    // timestamp, which is also what keeps a merged log folding identically to
    // an unmerged one.
    let mut order: Vec<ActId> = Vec::new();
    let mut by_id: BTreeMap<ActId, Commitment> = BTreeMap::new();
    let mut state = State::default();
    let live_acts = || {
        acts.iter()
            .enumerate()
            .filter(|(i, _)| live[*i])
            .map(|(_, a)| a)
    };

    for act in live_acts() {
        let (text, replaces, from, source) = match &act.kind {
            ActKind::Assert { text, from, source } => {
                (text, Vec::new(), from.clone(), source.clone())
            }
            ActKind::Supersede { text, old, .. } => (text, old.clone(), None, None),
            _ => continue,
        };
        order.push(act.id.clone());
        by_id.insert(
            act.id.clone(),
            Commitment {
                id: act.id.clone(),
                text: text.clone(),
                status: Status::Active,
                asserted_at: act.ts_unix,
                actor: act.actor.clone(),
                replaces,
                from,
                source,
            },
        );
    }

    // Pass 3 — effects, in time order, so a later act wins over an earlier one.
    for act in live_acts() {
        // Attribution: everything except asserting and adopting is an
        // adjudication, and adjudications are expected to be human.
        let adjudication = !matches!(act.kind, ActKind::Assert { .. } | ActKind::Adopt { .. });
        if adjudication && !act.is_human() {
            state.unattended.push(act.id.clone());
        }

        match &act.kind {
            ActKind::Supersede { old, .. } => {
                for o in old {
                    match by_id.get_mut(o) {
                        Some(c) => c.status = Status::Superseded { by: act.id.clone() },
                        None => state.dangling.push((act.id.clone(), o.clone())),
                    }
                }
            }
            ActKind::Retract { target, .. } => match by_id.get_mut(target) {
                Some(c) => c.status = Status::Retracted { at: act.ts_unix },
                None => state.dangling.push((act.id.clone(), target.clone())),
            },
            ActKind::Accept {
                a,
                b,
                rationale,
                revisit,
            } => {
                for side in [a, b] {
                    if !by_id.contains_key(side) {
                        state.dangling.push((act.id.clone(), side.clone()));
                    }
                }
                state.tolerated.push(Tolerated {
                    a: a.clone(),
                    b: b.clone(),
                    rationale: rationale.clone(),
                    revisit: revisit.clone(),
                    at: act.ts_unix,
                });
            }
            ActKind::Dismiss { a, b, .. } => state.dismissed.push((a.clone(), b.clone())),
            ActKind::Adopt {
                lineage,
                generation,
                source,
            } => {
                state.ancestry = Some(Ancestry {
                    lineage: lineage.clone(),
                    generation: generation.clone(),
                    source: source.clone(),
                    at: act.ts_unix,
                })
            }
            ActKind::Assert { .. } | ActKind::Revert { .. } => {}
        }
    }

    state.commitments = order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect();
    state
}
