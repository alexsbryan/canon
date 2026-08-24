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
/// A position somebody took, with when and by whom.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Stated {
    /// What it is a position on.
    pub about: String,
    pub position: crate::standing::Position,
    pub at: i64,
    /// The act that recorded it, so it can be reverted like anything else.
    pub act: ActId,
}

/// A policy this canon adopted, and the prose it was adopted as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Adopted {
    /// Which scope it governs. `None` is the whole canon.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<crate::scope::Scope>,
    pub rule: crate::policy::Rule,
    /// How it reads to a person. Citable by `why` like any other prose.
    pub text: String,
    pub at: i64,
    pub actor: String,
    pub act: ActId,
}

/// Something the group decided. What a graduated ladder counts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ruling {
    pub about: String,
    pub outcome: crate::standing::Outcome,
    pub authority: crate::policy::Authority,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    pub at: i64,
    /// Who decided. An adjudication with no person behind it is reported by
    /// `Canon::unattended`, never hidden.
    pub actor: String,
    pub act: ActId,
}

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
    /// Who holds standing, over what. Ostrom's first principle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub grants: Vec<crate::scope::Grant>,
    /// Which scope a commitment belongs to. Last write wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<(ActId, crate::scope::Scope)>,
    /// Positions people and commitments have taken, by what they are about.
    ///
    /// Recorded here rather than resolved into an outcome: turning positions
    /// into a verdict is policy's job, and the fold has no policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub positions: Vec<Stated>,
    /// The policies this canon decides under, by scope.
    ///
    /// In the ledger rather than beside it — see [`crate::ActKind::Policy`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub policies: Vec<Adopted>,
    /// What the group has decided, in order. Decisions, never observations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rulings: Vec<Ruling>,
    /// Draws announced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub draws: Vec<crate::draw::Committed>,
    /// Digests published before a draw's boundary. First per (draw, actor).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sealed: Vec<crate::draw::Sealed>,
    /// Secrets published after it. First per (draw, actor).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub opened: Vec<crate::draw::Opened>,
    /// Dates attached to acts. Last write per target wins, so a horizon can
    /// be moved as well as set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub horizons: Vec<crate::horizon::Horizon>,
    /// Which commitments are ranked, and as what. Last write wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ranks: Vec<(ActId, String)>,
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

    /// Who may decide about this scope, deepest grant first.
    ///
    /// **Answerable without asking a person, and that is the point.** Informal
    /// power runs on private knowledge of the process; a group where finding
    /// out who decides requires knowing whom to ask has made that person the
    /// gatekeeper. Ordering by depth is subsidiarity: the most specific
    /// standing that covers the question comes first.
    ///
    /// Lapsed grants are excluded — held standing, not remembered standing.
    pub fn who_decides(&self, scope: &crate::scope::Scope, now: i64) -> Vec<&crate::scope::Grant> {
        let mut found: Vec<&crate::scope::Grant> = self
            .grants
            .iter()
            .filter(|g| g.held_at(now) && g.scope.covers(scope))
            .collect();
        // Deepest first, then by actor so two runs render identically.
        found.sort_by(|a, b| {
            b.scope
                .depth()
                .cmp(&a.scope.depth())
                .then_with(|| a.actor.cmp(&b.actor))
        });
        found
    }

    /// Does this actor hold standing over this scope right now?
    pub fn standing_of(&self, actor: &str, scope: &crate::scope::Scope, now: i64) -> bool {
        self.who_decides(scope, now)
            .iter()
            .any(|g| g.actor == actor)
    }

    /// The scope a commitment belongs to, if anyone said.
    pub fn scope_of(&self, commitment: &ActId) -> Option<&crate::scope::Scope> {
        self.scopes
            .iter()
            .find(|(id, _)| id == commitment)
            .map(|(_, s)| s)
    }

    /// The policy that governs this scope: the deepest one that covers it,
    /// else the canon-wide one, else what shipped.
    ///
    /// **Deepest wins, and that is subsidiarity at the policy layer itself.**
    /// A house may decide by consent and its kitchen by whoever is cooking,
    /// and neither has to know about the other.
    pub fn policy_for(&self, scope: Option<&crate::scope::Scope>) -> &crate::policy::Rule {
        static SHIPPED: crate::policy::Rule = crate::policy::Rule::Default;
        let mut best: Option<&Adopted> = None;
        for p in &self.policies {
            let applies = match (&p.scope, scope) {
                (None, _) => true,
                (Some(s), Some(target)) => s.covers(target),
                (Some(_), None) => false,
            };
            if !applies {
                continue;
            }
            let depth = p.scope.as_ref().map_or(0, crate::scope::Scope::depth);
            let beats =
                best.is_none_or(|b| depth > b.scope.as_ref().map_or(0, crate::scope::Scope::depth));
            if beats {
                best = Some(p);
            }
        }
        best.map_or(&SHIPPED, |p| &p.rule)
    }

    /// The policy act governing this scope, for rendering and for `why`.
    pub fn policy_act(&self, scope: Option<&crate::scope::Scope>) -> Option<&Adopted> {
        let rule = self.policy_for(scope);
        self.policies.iter().find(|p| &p.rule == rule)
    }

    /// What has already been decided about this subject.
    ///
    /// **This reads decisions. There is no observation-counting sibling and
    /// there must never be one** — that is the line between a graduated
    /// sanction and a file on a person, and it is held by there being nothing
    /// in the format that records the second kind.
    pub fn prior_decisions(&self, about: &str) -> Vec<&Ruling> {
        self.rulings.iter().filter(|r| r.about == about).collect()
    }

    /// What rank someone gave this commitment, if anyone did.
    pub fn rank_of(&self, commitment: &ActId) -> Option<&str> {
        self.ranks
            .iter()
            .find(|(id, _)| id == commitment)
            .map(|(_, r)| r.as_str())
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
        //
        // A POSITION SPLITS ON ITS SOURCE, and the split is the reason the
        // two source kinds are worth having. Citing a commitment is a
        // READING — "this rule bears on that proposal" — which is exactly
        // what an agent is for, and flagging every agent citation as an
        // unattended adjudication would bury the real ones. Taking your OWN
        // position is a STANCE, and an agent with a stance is adjudicating:
        // under a consent policy one reasoned objection blocks, so an agent
        // that may object may veto. "Agents draft, ask and cite; they do not
        // adjudicate" now falls out of the type instead of being remembered
        // (§7 — structural, not instructed).
        let adjudication = !matches!(
            act.kind,
            ActKind::Assert { .. }
                | ActKind::Adopt { .. }
                | ActKind::Question { .. }
                | ActKind::Annotation { .. }
                | ActKind::Position {
                    citing: Some(_),
                    ..
                }
                // Sealing and opening a secret is PARTICIPATION, not
                // adjudication: it decides nothing and cannot be steered.
                // Announcing a draw is an adjudication and is not exempt.
                | ActKind::DrawSecret { .. }
                | ActKind::DrawReveal { .. }
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
            ActKind::Grant {
                holder,
                scope,
                horizon,
                ..
            } => {
                // Re-granting the same actor the same scope CLOSES the old
                // one rather than stacking on it: two grants live at one
                // instant would make "when does this lapse" have two answers.
                // Closing rather than deleting is what keeps "who held this
                // in March" answerable — and what stops a re-grant today
                // from changing a pool that was frozen in March.
                for g in canon
                    .grants
                    .iter_mut()
                    .filter(|g| g.actor == *holder && g.scope == *scope && g.withdrawn_at.is_none())
                {
                    g.withdrawn_at = Some(act.ts_unix);
                }
                canon.grants.push(crate::scope::Grant {
                    actor: holder.clone(),
                    scope: scope.clone(),
                    horizon: *horizon,
                    granted_at: act.ts_unix,
                    withdrawn_at: None,
                    act: act.id.clone(),
                });
            }
            ActKind::Withdraw {
                holder: actor,
                scope,
                ..
            } => {
                // Removes grants AT or BELOW the named scope. Carving a hole
                // out of a broader grant is deliberately not expressible:
                // stepping back from `house.kitchen` while holding `house`
                // would need a negative grant, and a permission system with
                // both grants and denials is one where nobody can answer "may
                // they?" by looking. Re-grant narrower instead.
                for g in canon.grants.iter_mut().filter(|g| {
                    g.actor == *actor && scope.covers(&g.scope) && g.withdrawn_at.is_none()
                }) {
                    g.withdrawn_at = Some(act.ts_unix);
                }
            }
            ActKind::Scoped { commitment, scope } => {
                canon.scopes.retain(|(id, _)| id != commitment);
                canon.scopes.push((commitment.clone(), scope.clone()));
            }
            ActKind::Position {
                about,
                citing,
                pull,
                because,
            } => {
                // The act's own actor is the source when nothing is cited.
                // One place says who.
                let position = match citing {
                    Some(id) => crate::standing::Position::of(id.clone(), *pull, because),
                    None => crate::standing::Position::by(act.actor.clone(), *pull, because),
                };
                canon.positions.push(Stated {
                    about: about.clone(),
                    position,
                    at: act.ts_unix,
                    act: act.id.clone(),
                });
            }
            ActKind::Policy { text, rule, scope } => {
                // One policy per scope. Two live policies over one scope
                // would make "what do we decide by" have two answers, which
                // is the duplicated decider §10.6 names.
                canon.policies.retain(|p| p.scope != *scope);
                canon.policies.push(Adopted {
                    scope: scope.clone(),
                    rule: rule.clone(),
                    text: text.clone(),
                    at: act.ts_unix,
                    actor: act.actor.clone(),
                    act: act.id.clone(),
                });
            }
            ActKind::Decided {
                about,
                outcome,
                authority,
                rationale,
            } => canon.rulings.push(Ruling {
                about: about.clone(),
                outcome: *outcome,
                authority: *authority,
                rationale: rationale.clone(),
                at: act.ts_unix,
                actor: act.actor.clone(),
                act: act.id.clone(),
            }),
            ActKind::DrawCommit {
                scope,
                count,
                after_ts,
                rationale,
            } => canon.draws.push(crate::draw::Committed {
                act: act.id.clone(),
                scope: scope.clone(),
                count: *count,
                after_ts: *after_ts,
                at: act.ts_unix,
                drawer: act.actor.clone(),
                rationale: rationale.clone(),
            }),
            // FIRST per (draw, actor) wins, for both. A second digest would
            // let somebody publish several and open whichever flatters them;
            // a second opening is meaningless once the first was checked.
            ActKind::DrawSecret { commit, digest } => {
                if !canon
                    .sealed
                    .iter()
                    .any(|s| s.commit == *commit && s.actor == act.actor)
                {
                    canon.sealed.push(crate::draw::Sealed {
                        commit: commit.clone(),
                        actor: act.actor.clone(),
                        digest: digest.clone(),
                        at: act.ts_unix,
                    });
                }
            }
            ActKind::DrawReveal { commit, secret } => {
                if !canon
                    .opened
                    .iter()
                    .any(|o| o.commit == *commit && o.actor == act.actor)
                {
                    canon.opened.push(crate::draw::Opened {
                        commit: commit.clone(),
                        actor: act.actor.clone(),
                        secret: secret.clone(),
                        at: act.ts_unix,
                    });
                }
            }
            ActKind::Horizon {
                target,
                at,
                rationale,
            } => {
                canon.horizons.retain(|h| h.target != *target);
                canon.horizons.push(crate::horizon::Horizon {
                    target: target.clone(),
                    at: *at,
                    rationale: rationale.clone(),
                    set_at: act.ts_unix,
                    act: act.id.clone(),
                });
            }
            ActKind::Rank { commitment, rank } => {
                canon.ranks.retain(|(id, _)| id != commitment);
                canon.ranks.push((commitment.clone(), rank.clone()));
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
