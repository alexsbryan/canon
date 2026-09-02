// SPDX-License-Identifier: AGPL-3.0-or-later
//! Policy — a pure function from evidence to an outcome and an authority.
//!
//! Everything in this file is *policy*, in the sense `PRIMITIVES.md` fixes:
//! a question with a defensible range of answers, which a community answers
//! and this library does not. How many objections make a conflict, who may
//! decide what, what happens when nothing bears on a proposal at all. The
//! mechanism — that the record cannot be quietly rewritten, that a
//! justification names something real, that absence is reported rather than
//! defaulted — lives elsewhere and is not configurable.
//!
//! **Two outputs, not one, and the split is load-bearing.** [`Outcome`] is how
//! a proposal stands against the canon: a fact about the evidence. [`Authority`]
//! is what you may then do about it: a decision the community made in advance.
//! Collapsing them forces a policy that wants to say "you may proceed, but tell
//! people" to lie about the outcome instead. Keeping them apart is also what
//! lets `Unaddressed` stay honest — nothing bears on this — while one community
//! treats that as a stop and another treats it as consent.
//!
//! **The authority is a ladder, not a boolean.** Ostrom's consistent finding
//! across long-enduring commons is that both zero enforcement and harsh
//! first-strike enforcement fail, and that what endures is mild-first
//! escalation. So the range is act, act-and-notify, ask one, ask a panel,
//! refuse — ordered, so a policy can *raise* another policy's answer without
//! knowing what it said.
//!
//! **[`Rule`] is an enum and not a struct per policy.** The set this library
//! ships is closed and has to serialize, because a policy lives in the ledger
//! rather than in a config file (§2.1). [`Policy`] stays a trait so a caller
//! can add one without forking, but nothing in this crate needs that yet and
//! four near-empty structs implementing it would be four names for what is one
//! decision each.
//!
//! **There is exactly one implementation of the default rule** — the free
//! function [`default_outcome`]. `Standing::outcome()` calls it and so does
//! `Rule::Default`. A second copy of that three-line rule is the failure §10.6
//! names: two deciders that agree today, diverge in a month, and produce a
//! plausible answer with nothing red anywhere.

use serde::{Deserialize, Serialize};

use crate::fold::Canon;
use crate::id::ActId;
use crate::scope::Scope;
use crate::standing::{Outcome, Pull, Source, Standing};

/// What the actor may do about it, mildest first.
///
/// **Ordered, and the ordering is used.** `Ord` is derived from declaration
/// order so [`Authority::raise`] is `max`: a policy that wraps another can
/// make its answer stricter without a table of every pair. Nothing may make an
/// answer *milder* — the escalation is one-way by construction, which is the
/// same reason `Withdraw` cannot carve a hole out of a broader grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
// Kebab, so the wire spelling and the spelling a person types at the CLI are
// the same five words. `ask_one` on disk and `ask-one` in a command would be
// one value with two names, which is where a mapping table quietly appears.
#[serde(rename_all = "kebab-case")]
pub enum Authority {
    /// Go ahead. The canon already decided this.
    Act,
    /// Go ahead, and say that you did.
    ActAndNotify,
    /// One person with standing has to agree first.
    AskOne,
    /// This needs the group.
    AskPanel,
    /// Not under this policy.
    Refuse,
}

impl Authority {
    /// The stricter of the two. Never the milder — see the type's note.
    pub fn raise(self, floor: Self) -> Self {
        self.max(floor)
    }

    /// The one spelling, used by the CLI to read a rung and to print one.
    /// Two vocabularies for five values is how `ask-one` and `AskOne` end up
    /// in the same log.
    pub fn parse(raw: &str) -> Option<Self> {
        Some(match raw.trim() {
            "act" => Self::Act,
            "notify" | "act-and-notify" => Self::ActAndNotify,
            "ask-one" => Self::AskOne,
            "ask-panel" => Self::AskPanel,
            "refuse" => Self::Refuse,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Act => "act",
            Self::ActAndNotify => "act-and-notify",
            Self::AskOne => "ask-one",
            Self::AskPanel => "ask-panel",
            Self::Refuse => "refuse",
        }
    }

    /// What this rung means, in the words a person already sees.
    ///
    /// These five phrasings shipped in `canon check` and are what a house has
    /// read on screen since the verb existed. They are here rather than there
    /// because the counterfactual has to say the same five things, and a
    /// second table would be a sixth vocabulary for five values.
    pub fn prose(self) -> &'static str {
        match self {
            Self::Act => "act",
            Self::ActAndNotify => "act, and say that you did",
            Self::AskOne => "ask one person with standing",
            Self::AskPanel => "ask the group",
            Self::Refuse => "not under this policy",
        }
    }
}

impl std::fmt::Display for Authority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// What is being proposed, beyond its words.
///
/// **The library never learns what a door is.** Buchanan and Tullock's result
/// is that the optimal decision rule minimizes external costs plus decision
/// costs, and that the balance differs by decision type — so "irreversible and
/// unaddressed means refuse" has to be expressible without this crate
/// classifying effects. Classification is the caller's job; these fields are
/// the interface between them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attributes {
    /// What this is about — the key positions and prior decisions are filed
    /// under. The same string a `position` act carries.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub about: String,
    /// Who proposes to act.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,
    /// Which scope it falls in, when the caller knows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    /// Can it be undone? `None` means nobody said, which is not the same as
    /// "yes" and is never treated as one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversible: Option<bool>,
    /// The commitment this would amend, when it amends one. Entrenchment
    /// reads it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amends: Option<ActId>,
    /// The clock, passed in. Nothing here reads a system clock: a replay that
    /// depends on when it ran is not a replay (§12.4, and the same discipline
    /// that keeps ids reproducible).
    #[serde(default)]
    pub now: i64,
}

impl Attributes {
    /// What is being proposed, and nothing else known about it.
    pub fn about(about: impl Into<String>) -> Self {
        Self {
            about: about.into(),
            ..Self::default()
        }
    }

    pub fn by(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    pub fn in_scope(mut self, scope: Scope) -> Self {
        self.scope = Some(scope);
        self
    }

    pub fn reversible(mut self, yes: bool) -> Self {
        self.reversible = Some(yes);
        self
    }

    pub fn amending(mut self, id: ActId) -> Self {
        self.amends = Some(id);
        self
    }

    pub fn at(mut self, now: i64) -> Self {
        self.now = now;
        self
    }
}

/// What a policy answered, and which rule answered it.
///
/// The `because` is not decoration and it is not the same field a position
/// carries. A bar that asserts only the outcome passes when the right answer
/// arrives for the wrong reason, and a policy chain — entrenchment over
/// consent over a graduated ladder — has several ways to reach the same
/// verdict. Naming the rule that fired is what makes the answer checkable
/// (§18.1, and principle 1: a decision invisible at debug is not finished).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    pub outcome: Outcome,
    pub authority: Authority,
    /// Which rule decided, and on what.
    pub because: String,
}

/// A policy: evidence in, an outcome and an authority out.
///
/// Pure. No clock, no IO, no model — `now` arrives in [`Attributes`]. That is
/// what makes a governance replay possible at all, and it is the single most
/// important fact about this layer.
pub trait Policy {
    /// A stable name, for rendering and for the bar.
    fn name(&self) -> String;

    fn decide(&self, standing: &Standing, attrs: &Attributes, canon: &Canon) -> Decision;
}

/// Today's rule, in one place.
///
/// Supported when commitments bear on it and none pull against; conflicts when
/// any do; unaddressed when none bear at all. **Unaddressed is not approval**
/// — a proposal engineered to cause harm is, almost by construction, one no
/// commitment supports, so it lands in the one outcome that cannot authorize
/// anything. What a community *does* about that is policy, and several rules
/// below answer it differently. What it *is* is this.
pub fn default_outcome(standing: &Standing) -> Outcome {
    if standing.positions.is_empty() {
        Outcome::Unaddressed
    } else if standing.against().next().is_some() {
        Outcome::Conflicts
    } else {
        Outcome::Supported
    }
}

/// The policies this library ships, as data.
///
/// Serializable because a policy **lives in the ledger** — as a `policy` act
/// carrying both prose and these fields. Not a config file beside the canon:
/// a default nobody can run `canon why` against is loosely held in intention
/// only. In the ledger it is subject to `check`, to tension detection, to
/// `supersede` with a rationale, and to a visible diff against the lineage it
/// was forked from.
///
/// The prose and the fields say the same thing twice on purpose (§7.6). The
/// prose is for people and for `why`; the fields are what code reads. Asking a
/// model to parse governance rules would be a prompt imperative relied on for
/// correctness, which is the one thing structure exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Rule {
    /// What shipped before policy was configurable. Adjudication asks a
    /// person: neither a conflict nor a silence authorizes anything by itself.
    Default,
    /// Sociocratic consent. One reasoned objection blocks; silence is not an
    /// objection. Nothing bearing on it at all is not silence *about* it —
    /// it is nobody having looked — so it proceeds only with notice.
    Consent,
    /// It takes this many objections to be a conflict. Below the line, the
    /// objections are recorded and the proposal stands.
    Threshold { against: usize },
    /// Counts what PEOPLE said, not what commitments say: supported when the
    /// share pulling toward it clears `numerator/denominator`.
    Supermajority { numerator: u32, denominator: u32 },
    /// Decide at the lowest competent level. Routes by the deepest live grant
    /// covering the proposal's scope; an actor with no standing there cannot
    /// decide it, and a scope nobody holds refuses rather than defaulting to
    /// whoever asked.
    Subsidiarity,
    /// Ostrom's fifth principle: mild first, escalating.
    ///
    /// **Counts prior DECISIONS, never prior observations.** "The house asked
    /// Dana to stop doing X" is an adjudication, attributed to whoever decided
    /// it, and belongs in the record. "Dana ran the washing machine at 1am" is
    /// an observation about a person and does not. A community that has never
    /// decided anything has no ladder to climb, which is correct.
    Graduated {
        ladder: Vec<Authority>,
        base: Box<Rule>,
    },
    /// Amending something ranked as protected costs more than amending a
    /// convention. A constitution harder to change than a statute.
    Entrenched {
        protected: Vec<String>,
        base: Box<Rule>,
    },
    /// What cannot be undone is not decided by silence.
    Cautious { base: Box<Rule> },
}

impl Rule {
    /// How this rule reads to a person, when nobody wrote it in their own
    /// words.
    ///
    /// A rendering of the typed fields, never a substitute for them: the
    /// prose on a `policy` act is what the community said, and this is what
    /// the library says when they said nothing. `canon policy set -m` is how
    /// you replace it, and the two are never blended.
    pub fn prose(&self) -> String {
        match self {
            Self::Default => "Commitments decide. A conflict or a gap is settled by a person, \
                              never by silence."
                .into(),
            Self::Consent => "We decide by consent: one reasoned objection blocks, and silence \
                              is not an objection."
                .into(),
            Self::Threshold { against } => {
                format!("It takes {against} reasoned objection(s) to stop a proposal.")
            }
            Self::Supermajority {
                numerator,
                denominator,
            } => format!(
                "A proposal carries when {numerator} in {denominator} of those who \
                 voted are for it."
            ),
            Self::Subsidiarity => {
                "Whoever holds the narrowest standing over a thing decides it.".into()
            }
            Self::Graduated { ladder, base } => format!(
                "{} Repeat decisions about the same thing escalate: {}.",
                base.prose(),
                ladder
                    .iter()
                    .map(|a| a.as_str())
                    .collect::<Vec<_>>()
                    .join(", then ")
            ),
            Self::Entrenched { protected, base } => format!(
                "{} Amending a {} takes the group.",
                base.prose(),
                protected.join(" or a ")
            ),
            Self::Cautious { base } => format!(
                "{} What cannot be undone is not decided by silence.",
                base.prose()
            ),
        }
    }

    fn base(&self) -> Option<&Rule> {
        match self {
            Self::Graduated { base, .. }
            | Self::Entrenched { base, .. }
            | Self::Cautious { base } => Some(base),
            _ => None,
        }
    }
}

impl Policy for Rule {
    fn name(&self) -> String {
        let head = match self {
            Self::Default => "default",
            Self::Consent => "consent",
            Self::Threshold { .. } => "threshold",
            Self::Supermajority { .. } => "supermajority",
            Self::Subsidiarity => "subsidiarity",
            Self::Graduated { .. } => "graduated",
            Self::Entrenched { .. } => "entrenched",
            Self::Cautious { .. } => "cautious",
        };
        match self.base() {
            Some(b) => format!("{head}/{}", b.name()),
            None => head.to_string(),
        }
    }

    fn decide(&self, standing: &Standing, attrs: &Attributes, canon: &Canon) -> Decision {
        match self {
            Self::Default => {
                let outcome = default_outcome(standing);
                Decision {
                    outcome,
                    // The shipped default: adjudication is a human act. A
                    // conflict needs someone to amend or to carry it
                    // knowingly; a silence needs someone to write a rule or
                    // record the gap. Neither happens by itself.
                    authority: match outcome {
                        Outcome::Supported => Authority::Act,
                        Outcome::Conflicts | Outcome::Unaddressed => Authority::AskOne,
                    },
                    because: format!("default: {}", outcome.describe()),
                }
            }
            Self::Consent => {
                let objections = standing.against().count();
                if objections > 0 {
                    Decision {
                        outcome: Outcome::Conflicts,
                        authority: Authority::Refuse,
                        because: format!(
                            "consent: {objections} reasoned objection(s); one is enough"
                        ),
                    }
                } else if standing.positions.is_empty() {
                    Decision {
                        outcome: Outcome::Unaddressed,
                        authority: Authority::ActAndNotify,
                        because: "consent: nothing bears on it and nobody objected, \
                                  but nobody looked either"
                            .into(),
                    }
                } else {
                    Decision {
                        outcome: Outcome::Supported,
                        authority: Authority::Act,
                        because: "consent: no objection".into(),
                    }
                }
            }
            Self::Threshold { against } => {
                let objections = standing.against().count();
                let (outcome, authority) = if *against > 0 && objections >= *against {
                    (Outcome::Conflicts, Authority::AskPanel)
                } else if standing.positions.is_empty() {
                    (Outcome::Unaddressed, Authority::AskOne)
                } else {
                    (Outcome::Supported, Authority::Act)
                };
                Decision {
                    outcome,
                    authority,
                    because: format!("threshold: {objections} against, {against} needed"),
                }
            }
            Self::Supermajority {
                numerator,
                denominator,
            } => {
                // People, not commitments. A supermajority of rules is not a
                // thing anyone means.
                let voters = || {
                    standing
                        .positions
                        .iter()
                        .filter(|p| matches!(p.source, Source::Actor(_)))
                };
                let toward = voters().filter(|p| p.pull == Pull::Toward).count();
                let total = voters().count();
                if total == 0 {
                    return Decision {
                        outcome: Outcome::Unaddressed,
                        authority: Authority::AskOne,
                        because: "supermajority: nobody voted".into(),
                    };
                }
                let clears =
                    (toward as u64) * (*denominator as u64) >= (total as u64) * (*numerator as u64);
                Decision {
                    outcome: if clears {
                        Outcome::Supported
                    } else {
                        Outcome::Conflicts
                    },
                    authority: if clears {
                        Authority::Act
                    } else {
                        Authority::Refuse
                    },
                    because: format!(
                        "supermajority: {toward}/{total} toward, {numerator}/{denominator} needed"
                    ),
                }
            }
            Self::Subsidiarity => {
                let outcome = default_outcome(standing);
                let Some(scope) = &attrs.scope else {
                    return Decision {
                        outcome,
                        authority: Authority::AskOne,
                        because: "subsidiarity: no scope on the proposal, so nothing to route to"
                            .into(),
                    };
                };
                let deciders = canon.who_decides(scope, attrs.now);
                let Some(deepest) = deciders.first().map(|g| g.scope.depth()) else {
                    // A boundary nobody holds. Refusing is Ostrom's first
                    // principle stated as behaviour: an undefined boundary is
                    // not an open one, and defaulting to whoever asked is
                    // exactly how informal power gets in.
                    return Decision {
                        outcome,
                        authority: Authority::Refuse,
                        because: format!("subsidiarity: nobody holds standing over `{scope}`"),
                    };
                };
                let holders: Vec<&str> = deciders
                    .iter()
                    .filter(|g| g.scope.depth() == deepest)
                    .map(|g| g.actor.as_str())
                    .collect();
                let mine = attrs.actor.as_deref().is_some_and(|a| holders.contains(&a));
                let authority = match (mine, outcome) {
                    (true, Outcome::Supported) => Authority::Act,
                    (true, _) => Authority::AskOne,
                    (false, _) => Authority::AskOne,
                };
                Decision {
                    outcome,
                    authority,
                    because: format!(
                        "subsidiarity: `{scope}` is held by {} — {}",
                        holders.join(", "),
                        if mine { "including you" } else { "not you" }
                    ),
                }
            }
            Self::Graduated { ladder, base } => {
                let under = base.decide(standing, attrs, canon);
                if ladder.is_empty() {
                    return under;
                }
                let prior = canon.prior_decisions(&attrs.about).len();
                let rung = ladder[prior.min(ladder.len() - 1)];
                Decision {
                    authority: under.authority.raise(rung),
                    because: format!(
                        "graduated: {prior} prior decision(s) about `{}` -> rung {} of {} ({})",
                        attrs.about,
                        prior.min(ladder.len() - 1) + 1,
                        ladder.len(),
                        under.because
                    ),
                    outcome: under.outcome,
                }
            }
            Self::Entrenched { protected, base } => {
                let under = base.decide(standing, attrs, canon);
                let rank = attrs.amends.as_ref().and_then(|id| canon.rank_of(id));
                let Some(rank) = rank.filter(|r| protected.iter().any(|p| p == r)) else {
                    return under;
                };
                Decision {
                    authority: under.authority.raise(Authority::AskPanel),
                    because: format!("entrenched: amends a `{rank}` ({})", under.because),
                    outcome: under.outcome,
                }
            }
            Self::Cautious { base } => {
                let under = base.decide(standing, attrs, canon);
                if attrs.reversible != Some(false) {
                    return under;
                }
                let floor = match under.outcome {
                    Outcome::Supported => Authority::ActAndNotify,
                    // What cannot be undone and nothing supports is the one
                    // combination this rule exists for.
                    Outcome::Conflicts | Outcome::Unaddressed => Authority::Refuse,
                };
                Decision {
                    authority: under.authority.raise(floor),
                    because: format!("cautious: irreversible ({})", under.because),
                    outcome: under.outcome,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
