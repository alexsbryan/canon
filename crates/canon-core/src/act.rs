// SPDX-License-Identifier: AGPL-3.0-or-later
//! The acts, and the envelope that carries them.
//!
//! Internally tagged on `"op"` so every line is self-describing and each
//! variant carries exactly its own fields — illegal field combinations are
//! unrepresentable.

use serde::{Deserialize, Serialize};

use crate::id::ActId;

/// Line format version. A reader that does not understand a declared version
/// REFUSES the line rather than misinterpreting it (see [`crate::Log::parse`]).
pub const FORMAT_VERSION: u32 = 1;

/// The acts. A commitment is *introduced* by `Assert` or `Supersede`; its id
/// is the id of the act that introduced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ActKind {
    /// A commitment enters the canon.
    Assert {
        text: String,
        /// Provenance when this commitment came from an adopted seed: the
        /// upstream act it corresponds to. Enables `diff --upstream` on a
        /// file that arrived with no git history.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        from: Option<ActId>,
        /// Where the text was drafted from, when it was extracted rather than
        /// authored. A drafted commitment with no citation is never written.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    /// One or more commitments are replaced by a new one. The new commitment
    /// is this act's id; each old one becomes superseded.
    Supersede {
        text: String,
        old: Vec<ActId>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// A commitment leaves the canon with no replacement.
    Retract {
        target: ActId,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// Two commitments genuinely conflict and the conflict is carried
    /// knowingly. The rationale is REQUIRED: an accepted contradiction must
    /// say what it protects.
    Accept {
        a: ActId,
        b: ActId,
        rationale: String,
        /// When the holder said they would revisit it. A date that has passed
        /// is not noise; it is the signal.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revisit: Option<String>,
    },
    /// Two commitments were flagged as conflicting and are not. Detector
    /// noise, dismissed. Light ceremony by design.
    Dismiss {
        a: ActId,
        b: ActId,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// Tomb-stone prior acts. The fold skips each target AND its effects.
    /// Revertible: reverting a `Revert` re-applies the originals.
    Revert {
        targets: Vec<ActId>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// Something the canon does not cover, recorded so it can be found again.
    ///
    /// Added inside v1 rather than in a later version, because an unknown
    /// `op` is refused rather than skipped (that is the point of the version
    /// rule), which makes every new act kind a breaking change. At 0.0.1 with
    /// nothing deployed the cost is zero; after the first adopter it is a
    /// migration.
    ///
    /// A question is ANSWERED by superseding it with a commitment, and
    /// WITHDRAWN by retracting it. Both acts already exist and already mean
    /// the right thing, so neither needed inventing.
    Question {
        text: String,
        /// The proposal that surfaced it, when `check` found nothing covering
        /// one. Kept verbatim: months later "what was I actually asking?" is
        /// the question that matters.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        proposal: Option<String>,
    },
    /// This canon was forked from a lineage. Recorded as an ACT rather than as
    /// git metadata so ancestry survives a file that travels by paste.
    Adopt {
        lineage: String,
        generation: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
}

/// One line of the log: an act plus its provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Act {
    pub id: ActId,
    /// Format version. Always written.
    pub v: u32,
    /// When the act happened (Unix seconds). NOT when it arrived on this
    /// machine — git records that, and the two differ after an offline append.
    pub ts_unix: i64,
    /// Who performed it. `human:<name>` for anything a person decided.
    pub actor: String,
    #[serde(flatten)]
    pub kind: ActKind,
}

impl Act {
    /// Build an act, deriving its content-addressed id.
    pub fn new(kind: ActKind, ts_unix: i64, actor: impl Into<String>) -> Self {
        let actor = actor.into();
        // serde_json writes fields in declaration order, so the body string —
        // and therefore the id — is deterministic across runs and builds.
        let body = serde_json::to_string(&kind).unwrap_or_default();
        Self {
            id: ActId::derive(ts_unix, &actor, &body),
            v: FORMAT_VERSION,
            ts_unix,
            actor,
            kind,
        }
    }

    /// True when this act was authored by a person.
    ///
    /// Adjudication — everything except `Assert`, `Question` and `Adopt` — is
    /// expected to be human-authored. See [`crate::Canon::unattended`].
    pub fn is_human(&self) -> bool {
        self.actor.starts_with("human:")
    }
}
