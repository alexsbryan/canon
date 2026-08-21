// SPDX-License-Identifier: AGPL-3.0-or-later
//! How a proposal stands against the canon.
//!
//! A pure contract type with no model dependency, so `--json`, the CLI
//! renderers and the MCP tool all render the same object rather than three
//! near-copies that drift.
//!
//! **A standing must carry citations.** `Bearing` has no constructor that
//! omits the commitment it names, so "this conflicts with your principles"
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

/// Which way a commitment pulls on a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Pull {
    Toward,
    Against,
}

/// One commitment's bearing on a proposal, and why.
///
/// The `because` is not decoration: a bearing whose reason a person cannot
/// check is an assertion, and this whole tool exists to replace assertions
/// with citations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bearing {
    pub commitment: ActId,
    pub pull: Pull,
    pub because: String,
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

/// How a proposal stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Standing {
    pub proposal: String,
    pub bearings: Vec<Bearing>,
}

impl Standing {
    /// Build a standing, keeping only bearings that cite a commitment this
    /// canon actually has and that say why.
    ///
    /// Returns `(standing, refused)`. Absence is reported, never defaulted:
    /// the caller prints what was refused rather than quietly rendering a
    /// shorter answer (§18.3).
    pub fn cited(
        canon: &Canon,
        proposal: impl Into<String>,
        bearings: Vec<Bearing>,
    ) -> (Self, Vec<Bearing>) {
        let (kept, refused): (Vec<_>, Vec<_>) = bearings
            .into_iter()
            .partition(|b| canon.get(&b.commitment).is_some() && !b.because.trim().is_empty());
        (
            Self {
                proposal: proposal.into(),
                bearings: kept,
            },
            refused,
        )
    }

    pub fn outcome(&self) -> Outcome {
        if self.bearings.is_empty() {
            Outcome::Unaddressed
        } else if self.against().next().is_some() {
            Outcome::Conflicts
        } else {
            Outcome::Supported
        }
    }

    pub fn against(&self) -> impl Iterator<Item = &Bearing> {
        self.bearings.iter().filter(|b| b.pull == Pull::Against)
    }

    pub fn toward(&self) -> impl Iterator<Item = &Bearing> {
        self.bearings.iter().filter(|b| b.pull == Pull::Toward)
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

    fn bearing(id: &ActId, pull: Pull, because: &str) -> Bearing {
        Bearing {
            commitment: id.clone(),
            pull,
            because: because.into(),
        }
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
        assert_eq!(standing.bearings.len(), 1);
        assert_eq!(refused.len(), 1);
    }

    #[test]
    fn a_bearing_with_no_reason_is_refused() {
        let (canon, ids) = canon_with(&["Mornings are protected."]);
        let (standing, refused) =
            Standing::cited(&canon, "p", vec![bearing(&ids[0], Pull::Against, "  ")]);
        assert!(standing.bearings.is_empty());
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
}
