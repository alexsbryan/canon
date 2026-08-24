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

/// Something the canon does not cover yet.
///
/// Shares [`Status`] with [`Commitment`] rather than carrying an enum of its
/// own, because the three states are the same three states: open, answered by
/// a commitment that superseded it, withdrawn. One vocabulary, so `why` and
/// the renderers do not need a second set of branches.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Question {
    pub id: ActId,
    pub text: String,
    /// `Active` is open. `Superseded { by }` is answered by that commitment.
    /// `Retracted` is withdrawn.
    pub status: Status,
    pub asked_at: i64,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal: Option<String>,
}

/// What was decided about a conflict.
///
/// Three states, and the fold mints only the last two: `Open` describes a
/// pair some surface has proposed and nobody has ruled on, which by
/// definition left no act in the log. `canon tensions` mints it; `derive`
/// never does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "disposition", rename_all = "snake_case")]
pub enum Disposition {
    /// Proposed, never ruled on. Not derivable from the log.
    ///
    /// The reason belongs to the DETECTOR, not to a decision: it says why the
    /// pair was flagged, in the words of whatever surfaced it. That is a
    /// different thing from a `rationale`, which is why someone ruled the way
    /// they did — so it gets a different name rather than the same one
    /// carrying two meanings.
    Open { reason: String },
    /// Carried knowingly. The rationale is required — a contradiction you
    /// keep on purpose must say what it protects.
    Tolerated {
        rationale: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revisit: Option<String>,
    },
    /// Judged not a real conflict. Light ceremony: rejecting noise is
    /// routine, so the rationale is optional.
    Dismissed {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
}

/// Two commitments that may not both be honoured, and what was decided.
///
/// One noun for both outcomes. Modelling "carried knowingly" as a struct and
/// "not a conflict" as a bare pair — which the first scaffold did — loses the
/// dismissal's reason, and leaves the third state nowhere to live.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conflict {
    pub a: ActId,
    pub b: ActId,
    pub disposition: Disposition,
    /// When it was dispositioned. Zero for `Open`, which was never recorded.
    #[serde(default)]
    pub at: i64,
}

impl Conflict {
    /// Does this concern the same unordered pair? Conflicts are symmetric:
    /// `(a, b)` and `(b, a)` are one conflict, not two.
    pub fn is_pair(&self, x: &ActId, y: &ActId) -> bool {
        (&self.a == x && &self.b == y) || (&self.a == y && &self.b == x)
    }
}

/// Where this canon came from, if it was adopted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ancestry {
    pub lineage: String,
    pub generation: String,
    pub source: Option<String>,
    pub at: i64,
}

/// What is in force right now — the derived body of norms.
///
/// Named for the essence rather than the mechanism: a fold produces state,
/// but the thing it produces *is* a canon, an authoritative body of norms.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Canon {
    pub commitments: Vec<Commitment>,
    /// What the canon does not cover. Answering one is superseding it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub questions: Vec<Question>,
    /// Conflicts someone has ruled on. Only `Tolerated` and `Dismissed`
    /// appear here — see [`Disposition::Open`].
    pub conflicts: Vec<Conflict>,
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
    /// Annotations this build carried without interpreting, by op.
    ///
    /// The §4.3 mitigation, and it is required rather than a courtesy.
    /// Carrying an unknown governance move is what keeps the format
    /// extensible; carrying it SILENTLY would mean a canon answers as though
    /// it had read everything when it had not. Every surface whose answer
    /// could have been affected reports this rather than rendering a shorter
    /// answer with no note (§18.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried: Vec<(ActId, String)>,
}

impl Canon {
    pub fn active(&self) -> impl Iterator<Item = &Commitment> {
        self.commitments
            .iter()
            .filter(|c| matches!(c.status, Status::Active))
    }

    pub fn get(&self, id: &ActId) -> Option<&Commitment> {
        self.commitments.iter().find(|c| &c.id == id)
    }

    /// Questions nobody has answered or withdrawn. This is `canon open`.
    pub fn open(&self) -> impl Iterator<Item = &Question> {
        self.questions
            .iter()
            .filter(|q| matches!(q.status, Status::Active))
    }

    pub fn question(&self, id: &ActId) -> Option<&Question> {
        self.questions.iter().find(|q| &q.id == id)
    }

    /// Is this pair already ruled on — carried knowingly, or dismissed?
    ///
    /// `tensions` filters through this so a pair someone already settled is
    /// never re-surfaced as news.
    pub fn is_settled(&self, a: &ActId, b: &ActId) -> bool {
        self.conflicts.iter().any(|c| c.is_pair(a, b))
    }

    /// Conflicts carried knowingly, with what they protect.
    pub fn tolerated(&self) -> impl Iterator<Item = &Conflict> {
        self.conflicts
            .iter()
            .filter(|c| matches!(c.disposition, Disposition::Tolerated { .. }))
    }
}

/// Derive current state from the acts.
///
/// Acts are folded in the order given; [`crate::Log`] sorts by `(ts_unix, id)`
/// on parse, so the result is identical regardless of how lines interleaved
/// during a merge.
pub fn derive(acts: &[Act]) -> Canon {
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
    let mut q_order: Vec<ActId> = Vec::new();
    let mut by_id: BTreeMap<ActId, Commitment> = BTreeMap::new();
    let mut questions: BTreeMap<ActId, Question> = BTreeMap::new();
    let mut canon = Canon::default();
    let live_acts = || {
        acts.iter()
            .enumerate()
            .filter(|(i, _)| live[*i])
            .map(|(_, a)| a)
    };

    for act in live_acts() {
        if let ActKind::Question { text, proposal } = &act.kind {
            q_order.push(act.id.clone());
            questions.insert(
                act.id.clone(),
                Question {
                    id: act.id.clone(),
                    text: text.clone(),
                    status: Status::Active,
                    asked_at: act.ts_unix,
                    actor: act.actor.clone(),
                    proposal: proposal.clone(),
                },
            );
            continue;
        }
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
        // An annotation we did not interpret is not an adjudication — we do
        // not know what it is, and calling it one would be an interpretation
        // we just declined to make. It cannot bypass a gate either, because
        // it has no effect on the fold at all.
        let adjudication = !matches!(
            act.kind,
            ActKind::Assert { .. }
                | ActKind::Adopt { .. }
                | ActKind::Question { .. }
                | ActKind::Annotation { .. }
        );
        if adjudication && !act.is_human() {
            canon.unattended.push(act.id.clone());
        }

        match &act.kind {
            // A question is answered by superseding it with a commitment and
            // withdrawn by retracting it: the same two acts, meaning the same
            // two things, rather than a second vocabulary for questions.
            ActKind::Supersede { old, .. } => {
                for o in old {
                    match (by_id.get_mut(o), questions.get_mut(o)) {
                        (Some(c), _) => c.status = Status::Superseded { by: act.id.clone() },
                        (None, Some(q)) => q.status = Status::Superseded { by: act.id.clone() },
                        (None, None) => canon.dangling.push((act.id.clone(), o.clone())),
                    }
                }
            }
            ActKind::Retract { target, .. } => {
                match (by_id.get_mut(target), questions.get_mut(target)) {
                    (Some(c), _) => c.status = Status::Retracted { at: act.ts_unix },
                    (None, Some(q)) => q.status = Status::Retracted { at: act.ts_unix },
                    (None, None) => canon.dangling.push((act.id.clone(), target.clone())),
                }
            }
            ActKind::Accept {
                a,
                b,
                rationale,
                revisit,
            } => {
                for side in [a, b] {
                    if !by_id.contains_key(side) {
                        canon.dangling.push((act.id.clone(), side.clone()));
                    }
                }
                canon.conflicts.push(Conflict {
                    a: a.clone(),
                    b: b.clone(),
                    disposition: Disposition::Tolerated {
                        rationale: rationale.clone(),
                        revisit: revisit.clone(),
                    },
                    at: act.ts_unix,
                });
            }
            ActKind::Dismiss { a, b, rationale } => canon.conflicts.push(Conflict {
                a: a.clone(),
                b: b.clone(),
                disposition: Disposition::Dismissed {
                    rationale: rationale.clone(),
                },
                at: act.ts_unix,
            }),
            ActKind::Adopt {
                lineage,
                generation,
                source,
            } => {
                canon.ancestry = Some(Ancestry {
                    lineage: lineage.clone(),
                    generation: generation.clone(),
                    source: source.clone(),
                    at: act.ts_unix,
                })
            }
            // Recorded, never acted on. This arm IS "not interpreted".
            ActKind::Annotation { kind, .. } => {
                canon.carried.push((act.id.clone(), kind.clone()));
            }
            ActKind::Assert { .. } | ActKind::Revert { .. } | ActKind::Question { .. } => {}
        }
    }

    canon.commitments = order
        .into_iter()
        .filter_map(|id| by_id.remove(&id))
        .collect();
    canon.questions = q_order
        .into_iter()
        .filter_map(|id| questions.remove(&id))
        .collect();
    canon
}
