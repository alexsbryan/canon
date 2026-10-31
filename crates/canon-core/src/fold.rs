// SPDX-License-Identifier: AGPL-3.0-or-later
//! The fold: current state as a pure function of the acts.
//!
//! No IO, no inference, no stored mutable state. Everything a renderer shows —
//! what is live, what replaced what and why, which contradictions are carried
//! knowingly — comes from replaying the log.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Serialize};

use crate::act::{Act, ActKind};
use crate::id::ActId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Status {
    Active,
    Superseded {
        by: ActId,
    },
    Retracted {
        at: i64,
    },
    /// Written, and not yet a rule: the scope's ratification rule has not
    /// been met. See [`crate::ratify`]. `needs` says what would meet it.
    Proposed {
        needs: String,
    },
    /// Refused by a holder of the scope. `why` quotes them.
    Refused {
        at: i64,
        by: String,
        why: String,
    },
}

/// A commitment as it stands now.
///
/// Only the fold makes one, and outside this crate it is read, never
/// written, so whether it is in force is the fold's answer and nobody
/// else's:
///
/// ```compile_fail,E0616
/// fn force(c: &mut canon_core::Commitment) {
///     c.status = canon_core::Status::Active;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Commitment {
    pub(crate) id: ActId,
    pub(crate) text: String,
    pub(crate) status: Status,
    pub(crate) asserted_at: i64,
    pub(crate) actor: String,
    /// Commitments this one replaced, when it arrived via `Supersede`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) replaces: Vec<ActId>,
    /// Upstream provenance, when inherited from an adopted seed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) from: Option<ActId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,
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
    /// Who wrote the act. For an actor-sourced position this is the same
    /// person the source names; for a cited one it is who did the citing,
    /// which is a different fact and the one a voice record wants.
    pub by: String,
    pub at: i64,
    /// The act that recorded it, so it can be reverted like anything else.
    pub act: ActId,
}

/// Something left unwritten on purpose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Silence {
    pub about: String,
    pub rationale: String,
    pub at: i64,
    pub actor: String,
    pub act: ActId,
}

/// A policy this canon adopted, and the prose it was adopted as.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// Where the change stands, when the scope's ratification rule reaches
    /// policy (`governs: all`). Absent for a policy that applied on its
    /// writer's standing, which is every policy written before v4.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<crate::ratify::Verdict>,
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
    /// The commitments the decision turned on, as the `decided` act named
    /// them. What a ladder keyed on the cited rule counts by.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citing: Vec<ActId>,
}

/// What a graduated ladder treats as one subject when it counts prior
/// decisions. See [`crate::policy::LadderKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyed<'a> {
    /// Decisions whose `about` is exactly this text.
    About(&'a str),
    /// Decisions that cited any of these commitments.
    Cited(&'a [ActId]),
}

/// A retraction written by someone without standing over what it retracts.
///
/// Not applied when written, and not refused either: it waits, like an add by
/// a non-holder, for a person who could have retracted the commitment
/// themselves to approve it. Their approval is them adopting it, so it takes
/// effect from then; their reasoned objection first refuses it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Retraction {
    /// The `retract` act.
    pub act: ActId,
    /// What it would retract.
    pub target: ActId,
    /// The target's scope: the holders of it are who decide.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<crate::scope::Scope>,
    pub actor: String,
    pub at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    pub verdict: crate::ratify::Verdict,
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
///
/// [`crate::Log::derive`] is the only way to get one. Outside this crate
/// it has no constructor and its fields are read through methods of the
/// same name, so no front end can build a canon or edit what one says:
///
/// ```compile_fail,E0599
/// let forged = canon_core::Canon::default();
/// ```
///
/// ```compile_fail,E0616
/// let mut canon = canon_core::Log::default().derive();
/// canon.commitments.clear();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Canon {
    pub(crate) commitments: Vec<Commitment>,
    /// What the canon does not cover. Answering one is superseding it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) questions: Vec<Question>,
    /// Conflicts someone has ruled on. Only `Tolerated` and `Dismissed`
    /// appear here — see [`Disposition::Open`].
    pub(crate) conflicts: Vec<Conflict>,
    pub(crate) ancestry: Option<Ancestry>,
    /// Adjudications not authored by a person. Reported, never hidden —
    /// absence of attribution is surfaced rather than defaulted.
    pub(crate) unattended: Vec<ActId>,
    /// `(act, missing target)` — an act referencing a commitment that is not
    /// in this log. A truncated or hand-edited file, or a snapshot adopted
    /// without its history. Surfaced rather than silently ignored: a
    /// supersession whose target is absent is a hole in the record, not a
    /// no-op.
    pub(crate) dangling: Vec<(ActId, ActId)>,
    /// Who holds standing, over what. Ostrom's first principle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) grants: Vec<crate::scope::Grant>,
    /// Which scope a commitment belongs to. Last write wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) scopes: Vec<(ActId, crate::scope::Scope)>,
    /// Positions people and commitments have taken, by what they are about.
    ///
    /// Recorded here rather than resolved into an outcome: turning positions
    /// into a verdict is policy's job, and the fold has no policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) positions: Vec<Stated>,
    /// The policies this canon decides under, by scope.
    ///
    /// In the ledger rather than beside it — see [`crate::ActKind::Policy`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) policies: Vec<Adopted>,
    /// Changes of policy still waiting on the group, or refused by it. Only
    /// a scope whose ratification rule `governs: all` has any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) pending_policies: Vec<Adopted>,
    /// What the group has decided, in order. Decisions, never observations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) rulings: Vec<Ruling>,
    /// What this canon leaves unwritten on purpose. Last word per subject.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) silences: Vec<Silence>,
    /// Draws announced.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) draws: Vec<crate::draw::Committed>,
    /// Digests published before a draw's boundary. First per (draw, actor).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) sealed: Vec<crate::draw::Sealed>,
    /// Secrets published after it. First per (draw, actor).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) opened: Vec<crate::draw::Opened>,
    /// Dates attached to acts. Last write per target wins, so a horizon can
    /// be moved as well as set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) horizons: Vec<crate::horizon::Horizon>,
    /// Which commitments are ranked, and as what. Last write wins.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) ranks: Vec<(ActId, String)>,
    /// How proposals become rules, by scope. See [`crate::ratify`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) ratifications: Vec<crate::ratify::AdoptedRatify>,
    /// The pools this canon holds, by scope. See [`crate::allot`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) allotments: Vec<crate::allot::Allotment>,
    /// How each pool is shared out.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) allocations: Vec<crate::allot::AdoptedAllocation>,
    /// Governance acts written by somebody without standing to write them —
    /// a grant, a policy, a ratification rule — and therefore not applied.
    /// Surfaced, never silently dropped: the act is in the log, the house
    /// should see that it was tried.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) ungoverned: Vec<(ActId, String)>,
    /// Governance acts that applied only because no grant covering their
    /// scope predated them — the canon, or that corner of it, was still
    /// open. The other half of `ungoverned`: a refusal says who lacked
    /// standing, and this says nobody had it yet. A founding script writes
    /// its grants and its first rules in one second, and every one of those
    /// rules took this way; a reader that cannot say so reads a notebook as
    /// a governed canon.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) bootstrap: Vec<(ActId, String)>,
    /// Retractions of somebody else's commitment by someone without standing
    /// over it, put to its scope as a proposal rather than refused. A
    /// ratified one has taken effect; the rest wait. See [`Retraction`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) retractions: Vec<Retraction>,
    /// Annotations this build carried without interpreting, by op.
    ///
    /// The §4.3 mitigation, and it is required rather than a courtesy.
    /// Carrying an unknown governance move is what keeps the format
    /// extensible; carrying it SILENTLY would mean a canon answers as though
    /// it had read everything when it had not. Every surface whose answer
    /// could have been affected reports this rather than rendering a shorter
    /// answer with no note (§18.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) carried: Vec<(ActId, String)>,
    /// Acts under a name that has bound a key, which no bound key signed and
    /// the name never vouched for. Carried and not applied, with why — the
    /// fold never saw them. See [`crate::sign`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) unverified: Vec<(ActId, String)>,
    /// When personal scopes were honoured. Derived, never rendered: it is a
    /// reading of `ratifications`, carried so standing can ask it mid-fold.
    #[serde(skip)]
    pub(crate) personal: crate::ratify::Personal,
}

/// Read access to what the fold derived. Each field is `pub(crate)`, so
/// these are all a front end has: it can read a canon and cannot write one.
impl Canon {
    pub fn commitments(&self) -> &[Commitment] {
        &self.commitments
    }
    pub fn questions(&self) -> &[Question] {
        &self.questions
    }
    pub fn conflicts(&self) -> &[Conflict] {
        &self.conflicts
    }
    pub fn ancestry(&self) -> &Option<Ancestry> {
        &self.ancestry
    }
    pub fn unattended(&self) -> &[ActId] {
        &self.unattended
    }
    pub fn dangling(&self) -> &[(ActId, ActId)] {
        &self.dangling
    }
    pub fn grants(&self) -> &[crate::scope::Grant] {
        &self.grants
    }
    pub fn scopes(&self) -> &[(ActId, crate::scope::Scope)] {
        &self.scopes
    }
    pub fn positions(&self) -> &[Stated] {
        &self.positions
    }
    pub fn policies(&self) -> &[Adopted] {
        &self.policies
    }
    pub fn pending_policies(&self) -> &[Adopted] {
        &self.pending_policies
    }
    pub fn rulings(&self) -> &[Ruling] {
        &self.rulings
    }
    pub fn silences(&self) -> &[Silence] {
        &self.silences
    }
    pub fn draws(&self) -> &[crate::draw::Committed] {
        &self.draws
    }
    pub fn sealed(&self) -> &[crate::draw::Sealed] {
        &self.sealed
    }
    pub fn opened(&self) -> &[crate::draw::Opened] {
        &self.opened
    }
    pub fn horizons(&self) -> &[crate::horizon::Horizon] {
        &self.horizons
    }
    pub fn ranks(&self) -> &[(ActId, String)] {
        &self.ranks
    }
    pub fn ratifications(&self) -> &[crate::ratify::AdoptedRatify] {
        &self.ratifications
    }
    pub fn allotments(&self) -> &[crate::allot::Allotment] {
        &self.allotments
    }
    pub fn allocations(&self) -> &[crate::allot::AdoptedAllocation] {
        &self.allocations
    }
    pub fn ungoverned(&self) -> &[(ActId, String)] {
        &self.ungoverned
    }
    pub fn bootstrap(&self) -> &[(ActId, String)] {
        &self.bootstrap
    }
    pub fn retractions(&self) -> &[Retraction] {
        &self.retractions
    }
    pub fn carried(&self) -> &[(ActId, String)] {
        &self.carried
    }
    pub fn unverified(&self) -> &[(ActId, String)] {
        &self.unverified
    }
}

impl Canon {
    /// Nothing derived yet: where every fold starts. Not `Default`, because a
    /// trait impl would be public and hand any caller an empty canon to show.
    pub(crate) fn empty() -> Self {
        Self {
            commitments: Vec::new(),
            questions: Vec::new(),
            conflicts: Vec::new(),
            ancestry: None,
            unattended: Vec::new(),
            dangling: Vec::new(),
            grants: Vec::new(),
            scopes: Vec::new(),
            positions: Vec::new(),
            policies: Vec::new(),
            pending_policies: Vec::new(),
            rulings: Vec::new(),
            silences: Vec::new(),
            draws: Vec::new(),
            sealed: Vec::new(),
            opened: Vec::new(),
            horizons: Vec::new(),
            ranks: Vec::new(),
            ratifications: Vec::new(),
            allotments: Vec::new(),
            allocations: Vec::new(),
            ungoverned: Vec::new(),
            bootstrap: Vec::new(),
            retractions: Vec::new(),
            carried: Vec::new(),
            unverified: Vec::new(),
            personal: crate::ratify::Personal::default(),
        }
    }
}

/// Read access to a commitment. The fields are `pub(crate)`.
impl Commitment {
    pub fn id(&self) -> &ActId {
        &self.id
    }
    pub fn text(&self) -> &str {
        &self.text
    }
    /// In force, proposed, refused, superseded or retracted: the fold's
    /// answer.
    pub fn status(&self) -> &Status {
        &self.status
    }
    pub fn asserted_at(&self) -> i64 {
        self.asserted_at
    }
    pub fn actor(&self) -> &str {
        &self.actor
    }
    pub fn replaces(&self) -> &[ActId] {
        &self.replaces
    }
    pub fn from(&self) -> Option<&ActId> {
        self.from.as_ref()
    }
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }
}

impl Canon {
    pub fn active(&self) -> impl Iterator<Item = &Commitment> {
        self.commitments
            .iter()
            .filter(|c| matches!(c.status, Status::Active))
    }

    /// Written and waiting on the scope's ratification rule.
    pub fn proposed(&self) -> impl Iterator<Item = &Commitment> {
        self.commitments
            .iter()
            .filter(|c| matches!(c.status, Status::Proposed { .. }))
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
            .filter(|g| g.held_at(now) && g.scope.covers(scope) && self.counts(g, Some(scope), now))
            .collect();
        // Deepest first, then by actor so two runs render identically.
        found.sort_by(|a, b| {
            b.scope
                .depth()
                .cmp(&a.scope.depth())
                .then_with(|| a.actor.cmp(&b.actor))
        });
        // One row per PERSON, at the narrowest standing they hold over this.
        // Somebody who holds both `house` and `house.kitchen` is one decider,
        // not two, and listing them twice makes the answer to "who decides
        // this?" read as a longer group than the house actually has.
        let mut seen = BTreeSet::new();
        found.retain(|g| seen.insert(g.actor.clone()));
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

    /// The decisions a graduated ladder counts: those on the same subject,
    /// by `about` text or by a shared cited commitment.
    ///
    /// With `repair`, a decision that went FOR the proposal closes the
    /// episode: only the conflicts after the last one count, so a house that
    /// sorted a problem out does not meet it again on the top rung. Without
    /// it, every decision on the subject counts, and `About` without repair
    /// is exactly [`Self::prior_decisions`]. Still decisions, never
    /// observations — the same line `prior_decisions` holds.
    pub fn ladder(&self, key: Keyed<'_>, repair: bool) -> Vec<&Ruling> {
        let keyed: Vec<&Ruling> = self
            .rulings
            .iter()
            .filter(|r| match key {
                Keyed::About(s) => r.about == s,
                Keyed::Cited(ids) => r.citing.iter().any(|c| ids.contains(c)),
            })
            .collect();
        if !repair {
            return keyed;
        }
        // Rulings are pushed in fold order, which is time order.
        let since = keyed
            .iter()
            .rposition(|r| r.outcome == crate::standing::Outcome::Supported)
            .map_or(0, |i| i + 1);
        keyed[since..]
            .iter()
            .copied()
            .filter(|r| r.outcome == crate::standing::Outcome::Conflicts)
            .collect()
    }

    /// How many people seconded this question. Read by `open` and nothing
    /// else: a second gates attention, never an outcome.
    /// Anything that is not a question has none: weight on a proposal is a
    /// vote, and an unsigned vote is reported, never counted as a second.
    pub fn seconds(&self, q: &ActId) -> usize {
        if self.question(q).is_none() {
            return 0;
        }
        self.positions
            .iter()
            .filter(|s| {
                s.about == q.as_str()
                    && s.position.actor().is_some()
                    && is_second(&s.by, s.position.pull)
            })
            .count()
    }

    /// What each person last said about this subject, in the order they said
    /// it: what a live check adds to what the canon says.
    ///
    /// **People only.** An agent's position is advice to a person, never a
    /// member's objection, so it never reaches a decision. Matched exactly,
    /// as [`Self::silence_about`] and [`Self::prior_decisions`] match; a
    /// later word replaces an earlier one, because a person who changed
    /// their mind holds one position, not two.
    pub fn said_about(&self, about: &str) -> Vec<crate::standing::Position> {
        let mut said: Vec<&Stated> = Vec::new();
        for (_, s) in self.people_on(about) {
            said.retain(|l| l.by != s.by);
            said.push(s);
        }
        said.into_iter().map(|s| s.position.clone()).collect()
    }

    /// Every position a person wrote about `about`, in fold order. The one
    /// reading of "a person said so" that ratification and a live check share.
    pub(crate) fn people_on<'a: 'b, 'b>(
        &'a self,
        about: &'b str,
    ) -> impl Iterator<Item = (crate::Person<'a>, &'a Stated)> + 'b {
        self.positions
            .iter()
            .filter(move |s| s.about == about)
            .filter_map(|s| Some((crate::Person::of(&s.by)?, s)))
    }

    /// Is this subject unwritten on purpose?
    ///
    /// **Exact subject match, and no fuzzy matching ever.** A silence that
    /// spread by resemblance would quietly cover things nobody chose to leave
    /// unwritten, which is the opposite of what it is for. `canon check
    /// --about "<subject>"` is how a proposal is checked against one.
    pub fn silence_about(&self, about: &str) -> Option<&Silence> {
        self.silences.iter().find(|s| s.about == about)
    }

    /// What somebody raised, and what became of it.
    ///
    /// **Hirschman's loyalty mechanism, made answerable.** Voice is only
    /// rational if it works, and whether it has worked for YOU is exactly the
    /// thing a person cannot verify by memory and will not ask about aloud.
    /// One query: the questions you asked and whether any were answered, the
    /// positions you took, and the decisions you made.
    ///
    /// It reads the canon, not a file on a person. Everything here is
    /// something the actor themselves put in the log.
    pub fn voice_of(&self, actor: &str) -> Voice<'_> {
        Voice {
            asked: self.questions.iter().filter(|q| q.actor == actor).collect(),
            positions: self.positions.iter().filter(|p| p.by == actor).collect(),
            decided: self.rulings.iter().filter(|r| r.actor == actor).collect(),
            silences: self.silences.iter().filter(|s| s.actor == actor).collect(),
        }
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

/// What one person put into the canon, and what came of it.
///
/// Borrowed rather than owned so this costs a filter and not a clone of the
/// log. It is a view of the record, not a record of its own — there is no
/// second store here that could drift from the first, and nothing in it was
/// observed about anybody.
#[derive(Debug)]
pub struct Voice<'a> {
    pub asked: Vec<&'a Question>,
    pub positions: Vec<&'a Stated>,
    pub decided: Vec<&'a Ruling>,
    pub silences: Vec<&'a Silence>,
}

impl Voice<'_> {
    /// Questions of theirs that a commitment answered.
    pub fn answered(&self) -> usize {
        self.asked
            .iter()
            .filter(|q| matches!(q.status, Status::Superseded { .. }))
            .count()
    }

    /// Questions still open. The number that says whether voice is working.
    pub fn open(&self) -> usize {
        self.asked
            .iter()
            .filter(|q| matches!(q.status, Status::Active))
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.asked.is_empty()
            && self.positions.is_empty()
            && self.decided.is_empty()
            && self.silences.is_empty()
    }
}

/// Derive current state from the acts.
///
/// Acts are folded in the order given, which is why this is not public:
/// outside this crate the only way in is [`crate::Log`], whose constructors
/// sort by `(ts_unix, id)` and dedupe by id, so how lines interleaved during
/// a merge cannot reach the fold.
pub(crate) fn derive(acts: &[Act]) -> Canon {
    checked(acts, None)
}

/// Set aside what a bound key did not sign, then fold the rest.
///
/// The one place signatures bite (see [`crate::sign`]). It runs before every
/// pass, so a forgery under a bound name never reaches a gate — not even the
/// ones that do not ask whether an actor is a person — and none of the fold's
/// rules had to learn about keys. `derive`'s clock is the latest act that
/// counts: a forged line dated a century ahead must not fold the canon at a
/// moment when every consent window has run out.
fn checked(acts: &[Act], now: Option<i64>) -> Canon {
    let verified = crate::sign::verify(acts);
    let kept: alloc::borrow::Cow<'_, [Act]> = if verified.unverified.is_empty() {
        alloc::borrow::Cow::Borrowed(acts)
    } else {
        let out: BTreeSet<&ActId> = verified.unverified.iter().map(|(id, _)| id).collect();
        alloc::borrow::Cow::Owned(
            acts.iter()
                .filter(|a| !out.contains(&a.id))
                .cloned()
                .collect(),
        )
    };
    let now = now.unwrap_or_else(|| kept.iter().map(|a| a.ts_unix).max().unwrap_or(0));
    // Personal scopes are switched on by a ratification act, which is known
    // to be ratified only once a fold has run; standing needs the answer
    // before that. So fold once, read when the knob was honoured, and fold
    // again with that timeline. One refold is enough because, while the
    // knob is declared, no canon-wide verdict reads a personal grant (see
    // `Canon::counts`), so the second fold judges the knob exactly as the
    // first did. A log that never declares it folds once, as before.
    let declared = kept.iter().any(|a| {
        matches!(
            a.kind,
            ActKind::Ratification {
                scope: None,
                personal: Some(true),
                ..
            }
        )
    });
    let mut seed = crate::ratify::Personal {
        declared,
        on: Vec::new(),
    };
    let mut canon = fold(&kept, now, &seed);
    if declared {
        seed = crate::ratify::Personal::from_ratified(true, &canon.ratifications);
        if !seed.on.is_empty() {
            canon = fold(&kept, now, &seed);
        }
    }
    canon.unverified = verified.unverified;
    canon
}

/// The one gate on a governance act, and its bookkeeping.
///
/// Every act that takes standing — a grant, a policy, a ruling, a retraction
/// of somebody else's — asks this and nothing else. (A retraction asks
/// [`Canon::seat`] first, and without standing is put to its scope as a
/// [`Retraction`] rather than refused.) The answer is recorded
/// as it is given: an act by somebody without standing lands in `ungoverned`
/// with `refused` as the reason and is not applied; an act that took only
/// because nothing covering the scope predated it lands in `bootstrap` and
/// is applied. Nine gates each writing their own `if !may_govern` was nine
/// places for the second kind of answer to go unrecorded, which is how it
/// went unrecorded (§10.6).
fn gate(
    canon: &mut Canon,
    act: &Act,
    scope: Option<&crate::scope::Scope>,
    refused: impl FnOnce() -> String,
) -> bool {
    match canon.seat(&act.actor, scope, act.ts_unix) {
        crate::ratify::Seat::Held => true,
        crate::ratify::Seat::Open => {
            canon.bootstrap.push((
                act.id.clone(),
                format!(
                    "no grant over {} predates this act, so it was open",
                    crate::ratify::where_(scope)
                ),
            ));
            true
        }
        crate::ratify::Seat::Lacking => {
            canon.ungoverned.push((act.id.clone(), refused()));
            false
        }
    }
}

/// May this act's actor rule on this pair? Standing over either side's scope
/// — or, for unscoped commitments, standing in the canon at all. Records the
/// attempt in `ungoverned` when not, so the answer is also the bookkeeping.
fn rules_over(canon: &mut Canon, act: &Act, a: &ActId, b: &ActId) -> bool {
    let scope = canon.scope_of(a).or_else(|| canon.scope_of(b)).cloned();
    gate(canon, act, scope.as_ref(), || {
        format!(
            "{} ruled on {a} and {b} without standing over {}",
            act.actor,
            scope
                .as_ref()
                .map_or("them".to_string(), ToString::to_string)
        )
    })
}

/// A `grant` act applied, or recorded in `ungoverned` when its actor could
/// not have written it.
///
/// Lifted out of the fold because the revert gate needs the same answer to
/// the same question one pass earlier, and two spellings of "who holds this"
/// is one too many.
fn apply_grant(
    canon: &mut Canon,
    act: &Act,
    holder: &str,
    scope: &crate::scope::Scope,
    horizon: Option<i64>,
) {
    // Granting standing over a scope takes standing over it or over the
    // scope above. The first grant in an ungoverned canon is the bootstrap
    // and is open.
    if !gate(canon, act, Some(scope), || {
        format!(
            "{} put {holder} in charge of {scope} without being in charge of it",
            act.actor
        )
    }) {
        return;
    }
    // Re-granting the same actor the same scope CLOSES the old one rather
    // than stacking on it: two grants live at one instant would make "when
    // does this lapse" have two answers. Closing rather than deleting is what
    // keeps "who held this in March" answerable — and what stops a re-grant
    // today from changing a pool that was frozen in March.
    for g in canon
        .grants
        .iter_mut()
        .filter(|g| g.actor == *holder && g.scope == *scope && g.withdrawn_at.is_none())
    {
        g.withdrawn_at = Some(act.ts_unix);
    }
    canon.grants.push(crate::scope::Grant {
        actor: holder.to_string(),
        scope: scope.clone(),
        horizon,
        granted_at: act.ts_unix,
        withdrawn_at: None,
        act: act.id.clone(),
    });
}

/// A `withdraw` act applied, or recorded in `ungoverned`. See [`apply_grant`].
fn apply_withdraw(canon: &mut Canon, act: &Act, holder: &str, scope: &crate::scope::Scope) {
    // Stepping back yourself is always yours to do. Standing somebody else
    // down takes standing over the scope.
    if act.actor != *holder
        && !gate(canon, act, Some(scope), || {
            format!(
                "{} took {holder} off {scope} without being in charge of it",
                act.actor
            )
        })
    {
        return;
    }
    // Removes grants AT or BELOW the named scope. Carving a hole out of a
    // broader grant is deliberately not expressible: stepping back from
    // `house.kitchen` while holding `house` would need a negative grant, and a
    // permission system with both grants and denials is one where nobody can
    // answer "may they?" by looking. Re-grant narrower instead.
    for g in canon
        .grants
        .iter_mut()
        .filter(|g| g.actor == *holder && scope.covers(&g.scope) && g.withdrawn_at.is_none())
    {
        g.withdrawn_at = Some(act.ts_unix);
    }
}

/// The standing established by the acts before `i`, ignoring anything an
/// earlier in-seat revert has already tomb-stoned.
///
/// Refolded per revert rather than carried alongside the main fold. Reverts
/// are rare — `undo` is a thing people do a handful of times, not a thing
/// that accumulates — so this costs nothing in a canon that has none, and the
/// alternative is a second running copy of the grant table that has to stay
/// in step with the first.
fn governance_before(
    acts: &[Act],
    i: usize,
    tombstoned: &BTreeSet<&str>,
    seed: &crate::ratify::Personal,
) -> Canon {
    let mut gov = seeded(acts, seed);
    for act in acts[..i]
        .iter()
        .filter(|a| !tombstoned.contains(a.id.as_str()))
    {
        match &act.kind {
            ActKind::Grant {
                holder,
                scope,
                horizon,
                ..
            } => apply_grant(&mut gov, act, holder, scope, *horizon),
            ActKind::Withdraw { holder, scope, .. } => apply_withdraw(&mut gov, act, holder, scope),
            ActKind::Scoped { commitment, scope } => {
                gov.scopes.retain(|(id, _)| id != commitment);
                gov.scopes.push((commitment.clone(), scope.clone()));
            }
            _ => {}
        }
    }
    gov
}

/// May this actor tomb-stone that act?
///
/// Reverting your own is always yours, exactly as with `retract` and
/// `withdraw`. Reverting somebody else's takes standing over whatever it
/// touched — the scope a governance act named, or the scope of the commitment
/// it introduced, or the canon itself when it named neither.
fn may_revert(gov: &Canon, acts: &[Act], act: &Act, target: &ActId) -> bool {
    let Some(t) = acts.iter().find(|a| a.id == *target) else {
        // A target this log does not carry is reported as dangling. Judging
        // it at the canon level is the conservative reading: an id nobody can
        // resolve is not a licence to delete it.
        return gov.may_govern(&act.actor, None, act.ts_unix);
    };
    if t.actor == act.actor {
        return true;
    }
    let scope = match &t.kind {
        ActKind::Grant { scope, .. }
        | ActKind::Withdraw { scope, .. }
        | ActKind::Scoped { scope, .. }
        | ActKind::Allot { scope, .. }
        | ActKind::Allocation { scope, .. } => Some(scope.clone()),
        ActKind::Policy { scope, .. } | ActKind::Ratification { scope, .. } => scope.clone(),
        ActKind::Accept { a, b, .. } | ActKind::Dismiss { a, b, .. } => {
            gov.scope_of(a).or_else(|| gov.scope_of(b)).cloned()
        }
        _ => gov.scope_of(&t.id).cloned(),
    };
    gov.may_govern(&act.actor, scope.as_ref(), act.ts_unix)
}

/// Is this act its author placing their own commitment or question in a
/// scope?
///
/// **That is part of proposing it, not a ruling.** `add --scope` writes the
/// assert and the scope as two acts, and the scope says whose approval the
/// proposal waits for. Counting the second act as an agent adjudicating made
/// a canon whose every agent proposal a person had approved warn that 27
/// adjudications had no person behind them. Scoping somebody else's is still
/// one.
fn scopes_own(
    act: &Act,
    by_id: &BTreeMap<ActId, Commitment>,
    questions: &BTreeMap<ActId, Question>,
) -> bool {
    let ActKind::Scoped { commitment, .. } = &act.kind else {
        return false;
    };
    by_id
        .get(commitment)
        .map(|c| &c.actor)
        .or_else(|| questions.get(commitment).map(|q| &q.actor))
        .is_some_and(|author| *author == act.actor)
}

/// Is this act a second: weight added to a question, with no name on it?
///
/// Exempt from attribution because it decides nothing. A question is not a
/// proposal, so no ratification or policy reads a position on one; a second
/// only moves the count `open` shows. Every other position stays an
/// adjudication — an agent's stance on a question, or an anonymous one on a
/// proposal, is still reported unattended.
fn seconds_a_question(act: &Act, questions: &BTreeMap<ActId, Question>) -> bool {
    match &act.kind {
        ActKind::Position {
            about,
            citing: None,
            pull,
            ..
        } => is_second(&act.actor, *pull) && questions.keys().any(|q| q.as_str() == about.as_str()),
        _ => false,
    }
}

/// The shape of a second, apart from what it is about. One definition, read
/// by the attribution exemption and by [`Canon::seconds`].
fn is_second(actor: &str, pull: crate::standing::Pull) -> bool {
    actor == crate::act::ANONYMOUS && pull == crate::standing::Pull::Toward
}

/// Derive current state as of `now`. See [`crate::Log::derive_at`].
pub(crate) fn derive_at(acts: &[Act], now: i64) -> Canon {
    checked(acts, Some(now))
}

/// An empty canon that already knows when personal scopes were honoured,
/// with each owner holding their own root for as long as they were.
///
/// The owner's standing is a grant like any other so every reading of
/// standing sees it without learning about personal scopes, and its `act`
/// is the ratification that conferred it, so `why` points somewhere real.
/// Only roots the log names get one: a person's corner exists once somebody
/// writes in it.
fn seeded(acts: &[Act], seed: &crate::ratify::Personal) -> Canon {
    let mut canon = Canon {
        personal: seed.clone(),
        ..Canon::empty()
    };
    if seed.on.is_empty() {
        return canon;
    }
    let mut roots: BTreeSet<crate::scope::Scope> = BTreeSet::new();
    for act in acts {
        let scope = match &act.kind {
            ActKind::Scoped { scope, .. }
            | ActKind::Grant { scope, .. }
            | ActKind::Withdraw { scope, .. }
            | ActKind::Allot { scope, .. }
            | ActKind::Allocation { scope, .. } => Some(scope),
            ActKind::Policy { scope, .. } | ActKind::Ratification { scope, .. } => scope.as_ref(),
            _ => None,
        };
        if let Some(root) = scope
            .and_then(crate::ratify::personal_owner)
            .and_then(crate::scope::Scope::new)
        {
            roots.insert(root);
        }
    }
    for (since, until, act) in &seed.on {
        for root in &roots {
            canon.grants.push(crate::scope::Grant {
                actor: root.as_str().to_string(),
                scope: root.clone(),
                horizon: None,
                granted_at: *since,
                withdrawn_at: *until,
                act: act.clone(),
            });
        }
    }
    canon
}

fn fold(acts: &[Act], now: i64, seed: &crate::ratify::Personal) -> Canon {
    let n = acts.len();

    // Pass 1a — standing to revert.
    //
    // A tomb-stone is as much a governance move as the act it covers. Gating
    // who may WRITE a grant while leaving who may DELETE one open is not a
    // gate: a stranger who reverts every grant leaves a canon nobody holds,
    // which is the bootstrap state, which is open — and the constitutional
    // level has a back door wide enough to walk a house through. So a revert
    // is judged like a `retract`: your own is always yours, somebody else's
    // takes standing over what it touched.
    //
    // Judged against the standing that stood WHEN THE REVERT WAS WRITTEN,
    // which is the clock rule every other gate here uses and the one
    // `ratification_for_at` applies to rules. A canon with no earlier grant
    // is ungoverned and open, so every log written before this rule existed
    // folds exactly as it did.
    let mut refused: Vec<(ActId, String)> = Vec::new();
    let mut tombstoned: BTreeSet<&str> = BTreeSet::new();
    let mut in_seat = vec![true; n];
    for (i, act) in acts.iter().enumerate() {
        let ActKind::Revert { targets, .. } = &act.kind else {
            continue;
        };
        let gov = governance_before(acts, i, &tombstoned, seed);
        // All or nothing. Half a revert would be an act whose effect nobody
        // could state, and the actor asked for one thing.
        match targets.iter().find(|t| !may_revert(&gov, acts, act, t)) {
            Some(t) => {
                in_seat[i] = false;
                refused.push((
                    act.id.clone(),
                    format!("{} reverted {t} without standing over it", act.actor),
                ));
            }
            None => tombstoned.extend(targets.iter().map(ActId::as_str)),
        }
    }

    // Pass 1b — liveness.
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
    for (j, act) in acts.iter().enumerate().filter(|(j, _)| in_seat[*j]) {
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
    let mut canon = Canon {
        ungoverned: refused,
        ..seeded(acts, seed)
    };
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
    let mut pending_rules: Vec<crate::ratify::AdoptedRatify> = Vec::new();
    let mut written_policies: Vec<Adopted> = Vec::new();
    let mut pending_retractions: Vec<Retraction> = Vec::new();
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
                // Binding your own key decides nothing about anybody else.
                | ActKind::Key { .. }
        ) && !scopes_own(act, &by_id, &questions)
            && !seconds_a_question(act, &questions);
        if adjudication && !act.is_human() {
            canon.unattended.push(act.id.clone());
        }

        match &act.kind {
            // A question is answered by superseding it with a commitment and
            // withdrawn by retracting it: the same two acts, meaning the same
            // two things, rather than a second vocabulary for questions.
            // A supersession retires its targets only once the NEW commitment
            // is ratified, so its effect is applied in the ratification pass
            // below. Here only the dangling check, which needs no verdict.
            ActKind::Supersede { old, .. } => {
                for o in old {
                    if !by_id.contains_key(o) && !questions.contains_key(o) {
                        canon.dangling.push((act.id.clone(), o.clone()));
                    }
                }
            }
            ActKind::Retract { target, rationale } => {
                // Withdrawing your own write is yours to do. Withdrawing
                // somebody else's takes standing over it.
                let own = by_id.get(target).is_some_and(|c| c.actor == act.actor)
                    || questions.get(target).is_some_and(|q| q.actor == act.actor);
                let scope = canon.scope_of(target).cloned();
                // Without it, the retraction is put to the scope rather than
                // refused, the way an add by a non-holder is: an agent that
                // may propose a rule may propose taking one away, and the
                // holder approves the act on the record instead of retyping
                // it and its reason.
                if !own
                    && canon.seat(&act.actor, scope.as_ref(), act.ts_unix)
                        == crate::ratify::Seat::Lacking
                {
                    // A proposal, not a ruling: the person who approves it
                    // is the one adjudicating.
                    canon.unattended.retain(|x| *x != act.id);
                    pending_retractions.push(Retraction {
                        act: act.id.clone(),
                        target: target.clone(),
                        scope: None,
                        actor: act.actor.clone(),
                        at: act.ts_unix,
                        rationale: rationale.clone(),
                        verdict: crate::ratify::Verdict::Proposed {
                            needs: String::new(),
                        },
                    });
                    continue;
                }
                if !own
                    && !gate(&mut canon, act, scope.as_ref(), || {
                        format!("{} retracted {target} without standing over it", act.actor)
                    })
                {
                    continue;
                }
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
                // A ruling on a pair takes standing over the pair. An agent
                // with a kitchen seat ruling on hall rules is outside its
                // standing: the act is kept, flagged, and not applied.
                if !rules_over(&mut canon, act, a, b) {
                    continue;
                }
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
            ActKind::Dismiss { a, b, rationale } => {
                if !rules_over(&mut canon, act, a, b) {
                    continue;
                }
                canon.conflicts.push(Conflict {
                    a: a.clone(),
                    b: b.clone(),
                    disposition: Disposition::Dismissed {
                        rationale: rationale.clone(),
                    },
                    at: act.ts_unix,
                })
            }
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
            } => apply_grant(&mut canon, act, holder, scope, *horizon),
            ActKind::Withdraw {
                holder: actor,
                scope,
                ..
            } => apply_withdraw(&mut canon, act, actor, scope),
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
                    by: act.actor.clone(),
                    at: act.ts_unix,
                    act: act.id.clone(),
                });
            }
            ActKind::Policy { text, rule, scope } => {
                if !gate(&mut canon, act, scope.as_ref(), || {
                    format!(
                        "{} set the rulings for {} without being in charge of it",
                        act.actor,
                        crate::ratify::where_(scope.as_ref())
                    )
                }) {
                    continue;
                }
                // Judged in pass 4b, once every position is folded: under
                // `governs: all` a change of policy is a proposal like any
                // other, and nothing in this pass reads policy, so waiting
                // costs nothing here.
                written_policies.push(Adopted {
                    scope: scope.clone(),
                    rule: rule.clone(),
                    text: text.clone(),
                    at: act.ts_unix,
                    actor: act.actor.clone(),
                    act: act.id.clone(),
                    verdict: None,
                });
            }
            ActKind::Ratification {
                text,
                rule,
                scope,
                governs,
                personal,
            } => {
                // Changing how a scope makes rules is decided one level up:
                // by standing over the scope, which includes the scope above.
                if !gate(&mut canon, act, scope.as_ref(), || {
                    format!(
                        "{} set how {} makes rules without being in charge of it",
                        act.actor,
                        crate::ratify::where_(scope.as_ref())
                    )
                }) {
                    continue;
                }
                // Kept, not replaced: a commitment is judged under the rule
                // in force when it was written, so the history has to stay.
                // Not applied yet either: the change is itself a proposal,
                // judged in pass 4a once every position is folded. The
                // placeholder verdict never escapes this function.
                pending_rules.push(crate::ratify::AdoptedRatify {
                    scope: scope.clone(),
                    rule: rule.clone(),
                    text: text.clone(),
                    at: act.ts_unix,
                    actor: act.actor.clone(),
                    act: act.id.clone(),
                    verdict: crate::ratify::Verdict::Proposed {
                        needs: String::new(),
                    },
                    under: crate::ratify::Ratify::Standing,
                    governs: *governs,
                    personal: *personal,
                });
            }
            ActKind::Allot {
                text,
                unit,
                units,
                scope,
            } => {
                // Saying what a commons HAS is a governance act: it draws the
                // boundary Ostrom's first principle is about, one level down
                // from who holds it.
                if !gate(&mut canon, act, Some(scope), || {
                    format!("{} allotted {scope}, not in charge of it", act.actor)
                }) {
                    continue;
                }
                canon.allotments.retain(|a| a.scope != *scope);
                canon.allotments.push(crate::allot::Allotment {
                    scope: scope.clone(),
                    unit: unit.clone(),
                    units: units.clone(),
                    text: text.clone(),
                    at: act.ts_unix,
                    actor: act.actor.clone(),
                    act: act.id.clone(),
                });
            }
            ActKind::Allocation { text, rule, scope } => {
                if !gate(&mut canon, act, Some(scope), || {
                    format!("{} set how {scope} goes round, not in charge", act.actor)
                }) {
                    continue;
                }
                // Kept rather than replaced, like a ratification rule: the
                // period is counted from adoption, so which turn it is stays
                // answerable for a rule that has since been superseded.
                canon.allocations.push(crate::allot::AdoptedAllocation {
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
                citing,
            } => {
                // A decision names no scope, so it takes standing in the
                // canon at all: somebody the house has said is in.
                if !gate(&mut canon, act, None, || {
                    format!("{} decided \"{about}\" without standing here", act.actor)
                }) {
                    continue;
                }
                // A citation of something this log does not carry is a hole
                // in the record, reported like any other. The ruling keeps
                // it: the decision still said what it turned on.
                for id in citing {
                    if !by_id.contains_key(id) {
                        canon.dangling.push((act.id.clone(), id.clone()));
                    }
                }
                canon.rulings.push(Ruling {
                    about: about.clone(),
                    outcome: *outcome,
                    authority: *authority,
                    rationale: rationale.clone(),
                    at: act.ts_unix,
                    actor: act.actor.clone(),
                    act: act.id.clone(),
                    citing: citing.clone(),
                })
            }
            ActKind::Silence { about, rationale } => {
                canon.silences.retain(|s| s.about != *about);
                canon.silences.push(Silence {
                    about: about.clone(),
                    rationale: rationale.clone(),
                    at: act.ts_unix,
                    actor: act.actor.clone(),
                    act: act.id.clone(),
                });
            }
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
            // Read before the fold began, by `crate::sign::verify`: a key
            // act decides which of its actor's acts reach this loop, and
            // nothing inside it.
            ActKind::Key { .. } => {}
            ActKind::Annotation { kind, .. } => {
                canon.carried.push((act.id.clone(), kind.clone()));
            }
            ActKind::Assert { .. } | ActKind::Revert { .. } | ActKind::Question { .. } => {}
        }
    }

    // Pass 3b — retractions written without standing, now that every
    // position about them is folded. One that a person with standing
    // approved takes effect from that approval, before pass 4 judges what
    // is left.
    for mut r in pending_retractions {
        // The target's scope once everything is folded, the way a
        // commitment is judged in pass 4: an `add --scope` writes its
        // scope in the same second as the assert, and a retraction in that
        // second can sort ahead of it.
        r.scope = canon.scope_of(&r.target).cloned();
        r.verdict = canon.judge_retraction(
            &crate::ratify::Proposal {
                id: &r.act,
                scope: r.scope.as_ref(),
                at: r.at,
                actor: &r.actor,
            },
            now,
        );
        if let crate::ratify::Verdict::Ratified { since, .. } = r.verdict {
            match (by_id.get_mut(&r.target), questions.get_mut(&r.target)) {
                (Some(c), _) => c.status = Status::Retracted { at: since },
                (None, Some(q)) => q.status = Status::Retracted { at: since },
                (None, None) => canon.dangling.push((r.act.clone(), r.target.clone())),
            }
        }
        canon.retractions.push(r);
    }

    // Pass 4a — the rules of rules, in the order they were written.
    //
    // A change to how a scope decides is judged under the rule that scope
    // had when the change was written, and only a ratified change decides
    // anything after it. Each entry is judged against exactly the earlier,
    // already-judged entries, so a change never governs itself, and pushed
    // only once its verdict is known.
    for mut r in pending_rules {
        let p = crate::ratify::Proposal {
            id: &r.act,
            scope: r.scope.as_ref(),
            at: r.at,
            actor: &r.actor,
        };
        let under = canon.ratification_for_at(p.scope, p.at).clone();
        r.verdict = canon.ratify_under(&under, &p, now);
        r.under = under;
        canon.ratifications.push(r);
    }

    // Pass 4b — policies, now that the rules of rules are known.
    //
    // A policy applies on its writer's standing unless the ratification rule
    // its scope had when it was written reaches policy (`governs: all`) and
    // is something other than `standing`; then it is judged like a change
    // of ratification rule and applies only from the moment it was ratified.
    //
    // One policy per scope: two live policies over one scope would make
    // "what do we decide by" have two answers, the duplicated decider §10.6
    // names. The one in force is the latest to TAKE EFFECT, which for a
    // policy that waited is its ratified moment, not its writing. The
    // winners are kept in the order they were written, which is the order
    // the retain-then-push this replaced produced, so a canon without the
    // knob renders exactly as it did.
    let mut in_force: Vec<(usize, i64, Adopted)> = Vec::new();
    for (i, mut pol) in written_policies.into_iter().enumerate() {
        let p = crate::ratify::Proposal {
            id: &pol.act,
            scope: pol.scope.as_ref(),
            at: pol.at,
            actor: &pol.actor,
        };
        let under = canon.ratification_for_at(p.scope, p.at).clone();
        let waits = canon.governs_at(p.scope, p.at) == crate::ratify::Governs::All
            && under != crate::ratify::Ratify::Standing;
        if !waits {
            let at = pol.at;
            in_force.push((i, at, pol));
            continue;
        }
        let verdict = canon.ratify_under(&under, &p, now);
        match verdict {
            crate::ratify::Verdict::Ratified { since, .. } => {
                pol.verdict = Some(verdict);
                in_force.push((i, since, pol));
            }
            _ => {
                pol.verdict = Some(verdict);
                canon.pending_policies.push(pol);
            }
        }
    }
    let mut winners: Vec<(usize, i64, Adopted)> = Vec::new();
    for (i, effective, pol) in in_force {
        match winners.iter_mut().find(|(_, _, w)| w.scope == pol.scope) {
            Some(w) if effective >= w.1 => *w = (i, effective, pol),
            Some(_) => {}
            None => winners.push((i, effective, pol)),
        }
    }
    // Order of the last write, as retain-then-push left it.
    winners.sort_by_key(|(i, _, _)| *i);
    canon.policies = winners.into_iter().map(|(_, _, p)| p).collect();

    // Pass 4 — ratification, in time order.
    //
    // Every introduced commitment is a proposal until the ratification rule
    // of its scope says otherwise. The verdict reads grants, scopes and
    // positions, all folded above, and the clock. A supersession's effect —
    // retiring what it replaces — lands here, only once the replacement is
    // a rule; a proposal to replace a rule leaves the rule standing.
    let supersedes: BTreeMap<&ActId, &Act> = acts
        .iter()
        .filter(|a| matches!(a.kind, ActKind::Supersede { .. }))
        .map(|a| (&a.id, a))
        .collect();
    for id in &order {
        let Some(c) = by_id.get(id) else { continue };
        if !matches!(c.status, Status::Active) {
            continue;
        }
        let verdict = canon.ratify(c, now);
        match verdict {
            crate::ratify::Verdict::Ratified { .. } => {
                if let Some(ActKind::Supersede { old, .. }) = supersedes.get(id).map(|a| &a.kind) {
                    for o in old {
                        match (by_id.get_mut(o), questions.get_mut(o)) {
                            (Some(prev), _) => prev.status = Status::Superseded { by: id.clone() },
                            (None, Some(q)) => q.status = Status::Superseded { by: id.clone() },
                            (None, None) => {}
                        }
                    }
                }
            }
            crate::ratify::Verdict::Proposed { needs } => {
                if let Some(c) = by_id.get_mut(id) {
                    c.status = Status::Proposed { needs };
                }
            }
            crate::ratify::Verdict::Refused { at, by, why } => {
                if let Some(c) = by_id.get_mut(id) {
                    c.status = Status::Refused { at, by, why };
                }
            }
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
