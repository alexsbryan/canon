// SPDX-License-Identifier: AGPL-3.0-or-later
//! A draw as the record holds it: the announcement, the sealed secrets and
//! the opened ones. Who is drawn is read outside this crate, from these.

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::fold::Canon;
use crate::id::ActId;
use crate::scope::Scope;

/// A draw somebody announced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committed {
    pub act: ActId,
    pub scope: Scope,
    pub count: usize,
    pub after_ts: i64,
    /// When the announcement was written. The boundary must postdate it.
    pub at: i64,
    pub drawer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

/// A digest somebody published before the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    pub commit: ActId,
    pub actor: String,
    pub digest: String,
    pub at: i64,
}

/// A secret somebody published after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opened {
    pub commit: ActId,
    pub actor: String,
    pub secret: String,
    pub at: i64,
}

impl Canon {
    /// Draws announced but not yet drawable, and why.
    pub fn draws_announced(&self) -> &[Committed] {
        &self.draws
    }
}
