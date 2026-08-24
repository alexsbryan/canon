// SPDX-License-Identifier: AGPL-3.0-or-later
//! The acts, and the envelope that carries them.
//!
//! Internally tagged on `"op"` so every line is self-describing and each
//! variant carries exactly its own fields — illegal field combinations are
//! unrepresentable.
//!
//! **The op namespace is split, and the two halves have opposite rules.**
//!
//! [`STRUCTURAL`] ops change what is LIVE — something enters, replaces,
//! leaves, or is undone. An unknown or malformed structural op REFUSES the
//! line. A peer silently dropping your retraction is a correctness failure,
//! not a compatibility inconvenience.
//!
//! Everything else is an **annotation**: a typed statement about a commitment
//! or a pair of them. An annotation this build does not recognise is CARRIED
//! and not interpreted, as [`ActKind::Annotation`]. Refusing it instead would
//! make every governance move a community invents — a vote, a scope grant, a
//! trial period — a breaking change to the format, which is the thing
//! `PRIMITIVES.md` exists to prevent.
//!
//! Carried is not the same as ignored. An unread annotation has no effect on
//! the fold, and every surface that could have been affected by one says how
//! many it is carrying rather than quietly rendering a shorter answer.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::id::ActId;

/// Line format version. A reader that does not understand a declared version
/// REFUSES the line rather than misinterpreting it (see [`crate::Log::parse`]).
///
/// **v2** split the op namespace (above). v1 readers refuse a v2 line, which
/// is correct: a v1 reader would reject a `position` or a `grant` outright,
/// and folding a governance log while dropping its governance is worse than
/// declining to read it.
pub const FORMAT_VERSION: u32 = 2;

/// The ops that change what is live. Unknown or malformed here refuses the
/// line; see the module note.
pub const STRUCTURAL: [&str; 4] = ["assert", "supersede", "retract", "revert"];

/// The annotations this build understands. Anything outside these two lists
/// is carried as [`ActKind::Annotation`].
pub const KNOWN_ANNOTATIONS: [&str; 12] = [
    "accept", "dismiss", "question", "adopt", "position", "grant", "withdraw", "scoped", "policy",
    "decided", "rank", "horizon",
];

/// The acts. A commitment is *introduced* by `Assert` or `Supersede`; its id
/// is the id of the act that introduced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", remote = "Self")]
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
    /// Someone takes a position on a proposal.
    ///
    /// A vote, an objection, a second, a delegation's effect — all one shape.
    /// **The actor is the act's own `actor`, never a field here**: two places
    /// naming who did something is exactly the duplicated decider that
    /// diverges quietly (§10.6).
    ///
    /// `citing` present means the position rests on a commitment the canon
    /// holds; absent means it is the actor's own. Both are positions, and the
    /// difference is what lets one policy count votes while another weighs
    /// what the canon already says.
    Position {
        /// What this is a position on: a proposal key, or a question's id.
        about: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        citing: Option<ActId>,
        pull: crate::standing::Pull,
        because: String,
    },
    /// Somebody is given standing over a scope.
    ///
    /// Ostrom's first principle in one act. Modelled as an annotation rather
    /// than a structural op because it does not change which commitments are
    /// live — it changes who may decide about them — and because that keeps it
    /// citable, contestable and revertible like anything else.
    Grant {
        /// **`holder`, not `actor`.** The envelope already has an `actor` —
        /// the person doing the granting — and the body is flattened into the
        /// same JSON object, so a body field of that name produces a line with
        /// two `actor` keys that no reader can parse back. The two are
        /// genuinely different people, which is why this act needs a field at
        /// all where `position` does not.
        holder: String,
        scope: crate::scope::Scope,
        /// When it lapses. Absent is standing with no end, which a community
        /// may choose and should have to.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        horizon: Option<i64>,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// Somebody steps back from a scope, or is stood down from it.
    ///
    /// The same act serves both, and that is deliberate: withdrawal read as a
    /// first-class move is the pre-exit signal. People leave a house in stages
    /// — stop hosting, stop cooking, stop coming — and those stages are exits
    /// from SCOPES. Recording them makes the signal legible without demanding
    /// a confrontation from someone already disengaging.
    Withdraw {
        holder: String,
        scope: crate::scope::Scope,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// A commitment belongs to a scope.
    ///
    /// An annotation rather than a field on `assert`, so the structural ops
    /// stay closed and unchanged and a commitment can be scoped — or rescoped
    /// — after the fact, which is what actually happens.
    Scoped {
        commitment: ActId,
        scope: crate::scope::Scope,
    },
    /// The policy this canon decides under.
    ///
    /// **Policy lives in the ledger, and that is the whole design.** Defaults
    /// are extraordinarily sticky and most adopters never change them, so
    /// whatever ships as default *is* the governance for nearly everyone —
    /// calling it loosely held describes our intentions rather than the
    /// outcome. The mitigation is recursive and costs nothing, because the
    /// machinery already exists: put the policy in the canon. Then how a
    /// community governs is subject to `check`, to tension detection, to
    /// `supersede` with a rationale, and to a visible diff against the lineage
    /// it was forked from. A default you can run `canon why` against is
    /// genuinely loosely held. One living in a TOML file is not.
    ///
    /// `text` and `rule` say the same thing twice on purpose (§7.6): the prose
    /// renders and is citable, the typed rule is what code reads. Asking a
    /// resolver to read governance rules would be a prompt imperative relied
    /// on for correctness.
    Policy {
        text: String,
        rule: crate::policy::Rule,
        /// Which scope it governs. Absent is the whole canon.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        scope: Option<crate::scope::Scope>,
    },
    /// Somebody decided something. The rung a graduated ladder counts.
    ///
    /// **A decision, never an observation.** Ostrom's fifth principle needs to
    /// know this is the third occurrence, and counting occurrences by person
    /// is precisely the surveillance file this project forbids. The resolution
    /// is a real distinction and not a compromise: an adjudication is a thing
    /// the group did, attributed to whoever did it, and it belongs in the
    /// record. What a person was seen doing does not. There is no act here
    /// that records the second kind, and adding one would be the defect.
    Decided {
        about: String,
        outcome: crate::standing::Outcome,
        authority: crate::policy::Authority,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// Look at this again by then.
    ///
    /// **One act for five technologies.** A term limit, a sunset clause, a
    /// trial period, a revisit date and a rotation are the same shape: a date
    /// attached to a decision, and a query for what is past it. Modelled as
    /// an annotation on a TARGET rather than as a field on each act, for the
    /// same reason `scoped` is: the structural ops stay closed, and a date can
    /// be attached — or moved — after the fact, which is what actually
    /// happens.
    ///
    /// `at` is Unix seconds, not a date string. `Accept.revisit` is a string
    /// because it shipped that way and the format does not rewrite history;
    /// the staleness query reads both through one calendar.
    Horizon {
        target: ActId,
        at: i64,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// A commitment is of some rank — a principle rather than a convention.
    ///
    /// Open text, deliberately: which ranks exist and what they mean is a
    /// community's vocabulary, not ours (§2.4/§4). Policy reads it; nothing
    /// here interprets it.
    Rank { commitment: ActId, rank: String },
    /// An annotation this build does not interpret.
    ///
    /// Carried verbatim so a log written by a community with governance moves
    /// we have never heard of still round-trips byte-for-byte, and so nothing
    /// is lost by reading it. It has no effect on the fold — that is what
    /// "not interpreted" means, and it is why carrying is safe: an unknown
    /// annotation cannot bypass a gate it cannot reach.
    ///
    /// `body` is a `serde_json::Map`, which is a `BTreeMap` in this build
    /// (`preserve_order` is off), so key order is sorted and two machines
    /// re-render an adopted log to identical bytes.
    #[serde(skip)]
    Annotation {
        kind: String,
        body: serde_json::Map<String, serde_json::Value>,
    },
}

/// Serialize by hand so [`ActKind::Annotation`] can write itself back out as
/// the op it arrived as, rather than as a variant name nobody sent.
impl Serialize for ActKind {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Annotation { kind, body } => {
                let mut out = serde_json::Map::with_capacity(body.len() + 1);
                out.insert("op".into(), serde_json::Value::String(kind.clone()));
                for (k, v) in body {
                    out.insert(k.clone(), v.clone());
                }
                out.serialize(serializer)
            }
            known => Self::serialize(known, serializer),
        }
    }
}

/// The namespace split, enforced at the one place a line becomes an act.
impl<'de> Deserialize<'de> for ActKind {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(deserializer)?;
        let op = value
            .get("op")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::missing_field("op"))?
            .to_string();

        // A known op is read strictly. We understand it, so a malformed body
        // is a defect in the writer and not a version we are behind.
        if STRUCTURAL.contains(&op.as_str()) || KNOWN_ANNOTATIONS.contains(&op.as_str()) {
            return Self::deserialize(value).map_err(D::Error::custom);
        }
        // An unknown STRUCTURAL op cannot exist — the list is closed — so
        // anything left is an annotation from a build ahead of this one.
        let serde_json::Value::Object(mut body) = value else {
            return Err(D::Error::custom("an act must be a JSON object"));
        };
        body.remove("op");
        Ok(Self::Annotation { kind: op, body })
    }
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
