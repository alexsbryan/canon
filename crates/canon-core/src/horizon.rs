// SPDX-License-Identifier: AGPL-3.0-or-later
//! A date attached to an act. What has gone past its date is read outside
//! this crate.

use alloc::string::String;
use serde::{Deserialize, Serialize};

use crate::id::ActId;

/// A date attached to an act after the fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Horizon {
    pub target: ActId,
    pub at: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
    pub set_at: i64,
    pub act: ActId,
}
