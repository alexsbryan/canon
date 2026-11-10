// SPDX-License-Identifier: AGPL-3.0-or-later
//! A pool and its sharing rule, as the record holds them. Whose turn it is
//! is read outside this crate, from these.

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use serde::{Deserialize, Serialize};

use crate::fold::Canon;
use crate::id::ActId;
use crate::scope::Scope;

/// A pool of units belonging to a scope: named sites, headgates, slots.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allotment {
    pub scope: Scope,
    /// What the community calls one of them — `site`, `turn`, `slot`. A noun,
    /// which is vocabulary; nothing in the fold reads it.
    pub unit: String,
    /// The units themselves, in the order the community wrote them. **Named
    /// rather than counted**, because the order carries meaning a count
    /// cannot: `gate-1 … gate-11` runs down a canal, and the sites at Alanya
    /// run along a coast. A plain count is written out as `1 … n`.
    pub units: Vec<String>,
    pub text: String,
    pub at: i64,
    pub actor: String,
    pub act: ActId,
}

/// Where the order of holders comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "order", rename_all = "snake_case")]
pub enum Order {
    /// Whoever holds the scope, sorted. The default, and it needs no act.
    Holders,
    /// A fixed order somebody wrote down — position on the canal, seniority,
    /// the order the articles list. Names not holding the scope are ignored.
    Given { actors: Vec<String> },
    /// Shuffled by a draw's verified seed, so the starting order is one
    /// nobody chose. **The draw's contribution is the seed**, not its seats:
    /// ordering everybody is a different question from selecting a few, and
    /// this asks the second of the same unsteerable number.
    FromDraw { commit: ActId },
}

/// How a scope's units are handed out.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Allocation {
    /// Everyone takes a turn, and the turn moves every `per` seconds.
    ///
    /// `step` is how far it moves and **its sign is the direction**. Which
    /// sign moves a holder along the unit list depends on the counts: with
    /// more holders than units the unit moves along the holders, so the
    /// holder moves back. A new rule starts again from its own first turn.
    Rotation { order: Order, step: i64, per: i64 },
}

impl Allocation {
    pub fn name(&self) -> String {
        match self {
            Self::Rotation { step, per, .. } => format!("rotation:{step}/{}s", per),
        }
    }

    pub fn prose(&self) -> String {
        match self {
            Self::Rotation { step, per, .. } => {
                let days = *per as f64 / 86_400.0;
                let every = if (days - 1.0).abs() < f64::EPSILON {
                    "day".to_string()
                } else if days >= 1.0 {
                    format!("{days:.0} days")
                } else {
                    format!("{per} seconds")
                };
                format!(
                    "Everyone takes a turn; it moves {} place(s) every {every}.",
                    step.unsigned_abs()
                )
            }
        }
    }
}

/// An allocation rule somebody adopted for a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptedAllocation {
    pub scope: Scope,
    pub rule: Allocation,
    pub text: String,
    /// Period zero begins when the rule was adopted. The clock is in the
    /// ledger rather than beside it, so two readers agree on which turn it is.
    pub at: i64,
    pub actor: String,
    pub act: ActId,
}

impl Canon {
    /// The allotment covering a scope: the deepest one that covers it.
    pub fn allotment_for(&self, scope: &Scope) -> Option<&Allotment> {
        self.allotments
            .iter()
            .filter(|a| a.scope.covers(scope))
            .max_by_key(|a| (a.scope.depth(), a.at))
    }

    /// The allocation rule for a scope. Deepest wins, then latest — the same
    /// reading `policy_for` and `ratification_for` already give.
    pub fn allocation_for(&self, scope: &Scope) -> Option<&AdoptedAllocation> {
        self.allocations
            .iter()
            .filter(|a| a.scope.covers(scope))
            .max_by_key(|a| (a.scope.depth(), a.at))
    }
}
