// SPDX-License-Identifier: AGPL-3.0-or-later
//! Content-addressed act ids.
//!
//! An id is derived from `(prefix, ts_unix, actor, body)` where `body` is the
//! act's JSON in field-declaration order. Two consequences the rest of the
//! design leans on:
//!
//! 1. **Merge is exact.** The same act arriving from two machines produces the
//!    same id, so a union-and-dedupe merge is correct rather than heuristic.
//! 2. **Replay is stable.** Ids do not depend on position in the log, so
//!    appending, sorting, and merging never renumber anything.
//!
//! Two byte-identical acts by the same actor in the same second collide by
//! design; appending happens in real time, so it does not arise in practice.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The id prefix for canon acts. Distinct from the Commonwealth governance
/// oplog's `gov` — same envelope, different act vocabulary. See `SPEC.md`.
pub const ID_PREFIX: &str = "can";

/// Length of the hex digest kept in an id. 12 hex chars = 48 bits; ample for
/// a log a person or a household will ever write, and short enough to read
/// aloud and type.
const SHORT: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActId(String);

impl ActId {
    pub fn from_raw(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Derive the id for an act body. `body` must be the act's serialized
    /// JSON — callers go through [`crate::Act::new`] rather than this.
    pub fn derive(ts_unix: i64, actor: &str, body: &str) -> Self {
        let input = format!("{ID_PREFIX}|{ts_unix}|{actor}|{body}");
        let digest = Sha256::digest(input.as_bytes());
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        Self(format!("{ID_PREFIX}-{}", &hex[..SHORT]))
    }
}

/// The full hex digest of arbitrary bytes.
///
/// Exposed because this crate already owns content addressing, and the
/// alternative — a second hasher elsewhere — is two implementations of one
/// idea (§10.6). The draw's commit-reveal check and its seed both go through
/// here, so "what does this hash to" has exactly one answer in the tree.
pub fn digest_hex(input: &[u8]) -> String {
    Sha256::digest(input)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A short content digest of arbitrary text.
pub fn short_digest(input: &str) -> String {
    digest_hex(input.as_bytes()).chars().take(8).collect()
}

impl std::fmt::Display for ActId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
