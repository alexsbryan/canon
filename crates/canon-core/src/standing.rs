// SPDX-License-Identifier: AGPL-3.0-or-later
//! How a proposal stands against the canon.
//!
//! A pure contract type with no model dependency, so `--json`, the CLI
//! renderers and the MCP tool all render the same object rather than three
//! near-copies that drift.
//!
//! **A standing must carry citations.** `Position` has no constructor that
//! omits the source it comes from, so "this conflicts with your principles"
//! with nothing to point at is unrepresentable rather than discouraged. That
//! is the whole difference between agent reasoning that is *citable* and
//! agent reasoning that is *plausible* — which is what distinguishes "the
//! agent misread the rule" from "the rule is wrong", a correction from an
//! amendment.
//!
//! The name is `Standing` and not `Verdict` deliberately. A survey of the
//! wider family found ten separate definitions of `Verdict` across eleven
//! crates: as a bare enum it is a word everything reaches for and nothing
//! agrees on. `Standing` says the same thing — how a proposal stands — and
//! is not already taken.

use serde::{Deserialize, Serialize};

use crate::fold::Canon;
use crate::id::ActId;

/// Which way something pulls on a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pull {
    Toward,
    Against,
}

/// Where a position comes from.
///
/// **Two source kinds, and modelling only the first is what made every voting
/// technology look like it needed new mechanism.** A commitment bears on a
/// proposal because of what the canon already holds. An actor bears on it
/// because they are a person with standing who said so. Majority, quorum,
/// consent, delegation, seconds and per-actor budgets are all the second kind,
/// and they become policy the moment the type admits them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    /// A commitment this canon holds. Checked against it by [`Standing::cited`].
    Commitment(ActId),
    /// A person or agent, by the same `actor` string an act carries.
    Actor(String),
}

/// One position on a proposal, and why.
///
/// The `because` is not decoration: a position whose reason a person cannot
/// check is an assertion, and this whole tool exists to replace assertions
/// with citations. Required from BOTH source kinds — sociocratic practice only
/// obliges an objection to argue itself, but being stricter costs nothing here
/// and keeps one rule instead of two.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub source: Source,
    pub pull: Pull,
    pub because: String,
}

impl Position {
    /// A commitment the canon holds, bearing on a proposal.
    pub fn of(commitment: ActId, pull: Pull, because: impl Into<String>) -> Self {
        Self {
            source: Source::Commitment(commitment),
            pull,
            because: because.into(),
        }
    }

    /// A person or agent taking a position.
    pub fn by(actor: impl Into<String>, pull: Pull, because: impl Into<String>) -> Self {
        Self {
            source: Source::Actor(actor.into()),
            pull,
            because: because.into(),
        }
    }

    /// The commitment this cites, when it cites one.
    pub fn commitment(&self) -> Option<&ActId> {
        match &self.source {
            Source::Commitment(id) => Some(id),
            Source::Actor(_) => None,
        }
    }

    /// The actor who took it, when a person took it.
    pub fn actor(&self) -> Option<&str> {
        match &self.source {
            Source::Actor(a) => Some(a),
            Source::Commitment(_) => None,
        }
    }
}

/// The shape of the answer, before any profile decides how to say it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// Commitments bear on it and none pull against.
    Supported,
    /// At least one commitment pulls against.
    Conflicts,
    /// No commitment bears on it at all. Not an approval.
    Unaddressed,
}

impl Outcome {
    /// How this reads to a person.
    ///
    /// Was a private free function in `policy.rs` with one caller. It is here
    /// and public because more than one surface now has to say what an
    /// outcome IS — `check`, and the counterfactual `replay` prints — and two
    /// copies of three clauses is how the same verdict ends up worded two
    /// ways in one tool.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Supported => "commitments bear on it and none pull against",
            Self::Conflicts => "at least one commitment pulls against",
            Self::Unaddressed => "nothing bears on it",
        }
    }
}

/// How a proposal stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub proposal: String,
    pub positions: Vec<Position>,
}

impl Standing {
    /// Build a standing, keeping only positions that name something real and
    /// say why: a commitment this canon actually holds, or an actor.
    ///
    /// Returns `(standing, refused)`. Absence is reported, never defaulted:
    /// the caller prints what was refused rather than quietly rendering a
    /// shorter answer (§18.3).
    pub fn cited(
        canon: &Canon,
        proposal: impl Into<String>,
        positions: Vec<Position>,
    ) -> (Self, Vec<Position>) {
        let (kept, refused): (Vec<_>, Vec<_>) = positions.into_iter().partition(|p| {
            let names_something_real = match &p.source {
                // The citation filter, unchanged: a commitment nobody holds
                // cannot license anything.
                Source::Commitment(id) => canon.get(id).is_some(),
                // An actor is real if they said so. WHETHER THEY MAY is a
                // question about standing, which is scope's job and not this
                // filter's — see `Canon::standing_of`.
                Source::Actor(a) => !a.trim().is_empty(),
            };
            names_something_real && !p.because.trim().is_empty()
        });
        (
            Self {
                proposal: proposal.into(),
                positions: kept,
            },
            refused,
        )
    }

    /// How this stands under the shipped default rule.
    ///
    /// **Delegates; it does not re-implement.** The rule lives in
    /// [`crate::policy::default_outcome`] and this is one of its two callers,
    /// the other being `Rule::Default`. Two copies of a three-line rule agree
    /// today, diverge in a month, and produce a plausible answer with nothing
    /// red anywhere — which is the failure §10.6 exists to prevent, and the
    /// main risk this whole policy layer carried.
    ///
    /// A canon that has adopted a policy asks the policy, not this. Callers
    /// that want the configured answer go through
    /// [`crate::Canon::policy_for`].
    pub fn outcome(&self) -> Outcome {
        crate::policy::default_outcome(self)
    }

    pub fn against(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter().filter(|p| p.pull == Pull::Against)
    }

    pub fn toward(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter().filter(|p| p.pull == Pull::Toward)
    }

    /// Positions a commitment takes. What `check` renders as citations.
    pub fn cited_commitments(&self) -> impl Iterator<Item = &Position> {
        self.positions.iter().filter(|p| p.commitment().is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::act::ActKind;
    use crate::{Act, Log};

    fn canon_with(texts: &[&str]) -> (Canon, Vec<ActId>) {
        let acts: Vec<Act> = texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Act::new(
                    ActKind::Assert {
                        text: (*t).into(),
                        from: None,
                        source: None,
                    },
                    100 + i as i64,
                    "human:alex",
                )
            })
            .collect();
        let ids = acts.iter().map(|a| a.id.clone()).collect();
        (Log::from_acts(acts).derive(), ids)
    }

    fn bearing(id: &ActId, pull: Pull, because: &str) -> Position {
        Position::of(id.clone(), pull, because)
    }

    #[test]
    fn a_bearing_citing_a_commitment_this_canon_does_not_have_is_refused() {
        // The failure this prevents: a model names a plausible id, and the
        // renderer prints a conflict against a rule nobody ever wrote.
        let (canon, ids) = canon_with(&["Mornings are protected."]);
        let (standing, refused) = Standing::cited(
            &canon,
            "take the 8am rotation",
            vec![
                bearing(&ids[0], Pull::Against, "the rotation starts at 8"),
                bearing(
                    &ActId::from_raw("can-000000000000"),
                    Pull::Toward,
                    "invented",
                ),
            ],
        );
        assert_eq!(standing.positions.len(), 1);
        assert_eq!(refused.len(), 1);
    }

    #[test]
    fn a_bearing_with_no_reason_is_refused() {
        let (canon, ids) = canon_with(&["Mornings are protected."]);
        let (standing, refused) =
            Standing::cited(&canon, "p", vec![bearing(&ids[0], Pull::Against, "  ")]);
        assert!(standing.positions.is_empty());
        assert_eq!(refused.len(), 1);
        // And with nothing cited, the outcome is UNADDRESSED, not supported.
        // Silence is not approval.
        assert_eq!(standing.outcome(), Outcome::Unaddressed);
    }

    #[test]
    fn one_bearing_against_is_enough_to_conflict() {
        let (canon, ids) = canon_with(&["a", "b"]);
        let (standing, _) = Standing::cited(
            &canon,
            "p",
            vec![
                bearing(&ids[0], Pull::Toward, "helps"),
                bearing(&ids[1], Pull::Against, "hurts"),
            ],
        );
        assert_eq!(standing.outcome(), Outcome::Conflicts);
        assert_eq!(standing.against().count(), 1);
        assert_eq!(standing.toward().count(), 1);
    }

    #[test]
    fn nothing_bearing_on_it_is_unaddressed_and_never_supported() {
        let (canon, _) = canon_with(&["a"]);
        let (standing, _) = Standing::cited(&canon, "something else entirely", vec![]);
        assert_eq!(standing.outcome(), Outcome::Unaddressed);
    }
    #[test]
    fn an_actor_may_take_a_position_without_citing_a_commitment() {
        // The change that turns majority, quorum, consent, delegation and
        // per-actor budgets from mechanism into policy. A vote is not a
        // commitment bearing on a proposal; it is a person saying so.
        let (canon, ids) = canon_with(&["Mornings are protected."]);
        let (standing, refused) = Standing::cited(
            &canon,
            "move the standup to 8am",
            vec![
                Position::of(ids[0].clone(), Pull::Against, "8am is inside mornings"),
                Position::by("human:dana", Pull::Against, "I have school run until 8:30"),
                Position::by("human:sam", Pull::Toward, "works for me"),
            ],
        );
        assert!(refused.is_empty(), "an actor names something real");
        assert_eq!(standing.positions.len(), 3);
        assert_eq!(standing.against().count(), 2);
        assert_eq!(
            standing.cited_commitments().count(),
            1,
            "only one cites a rule"
        );
        assert_eq!(
            standing.positions[1].actor(),
            Some("human:dana"),
            "the source survives the filter"
        );
    }

    #[test]
    fn a_position_from_nobody_is_refused_like_a_commitment_nobody_holds() {
        // Both source kinds must name something real, or the filter has a
        // hole shaped exactly like the one it exists to close.
        let (canon, _) = canon_with(&["Mornings are protected."]);
        let (standing, refused) = Standing::cited(
            &canon,
            "p",
            vec![Position::by("   ", Pull::Against, "anonymous veto")],
        );
        assert!(standing.positions.is_empty());
        assert_eq!(refused.len(), 1);
        assert_eq!(standing.outcome(), Outcome::Unaddressed);
    }

    #[test]
    fn an_actor_position_still_has_to_say_why() {
        let (canon, _) = canon_with(&["Mornings are protected."]);
        let (standing, refused) = Standing::cited(
            &canon,
            "p",
            vec![Position::by("human:dana", Pull::Against, " ")],
        );
        assert!(standing.positions.is_empty(), "a bare no is an assertion");
        assert_eq!(refused.len(), 1);
    }
}
