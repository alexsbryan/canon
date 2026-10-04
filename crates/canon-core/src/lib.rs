// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon-core` — the acts, their content-addressed ids, and the fold that
//! derives current state from them.
//!
//! This crate compiles with `no_std` + `alloc`: history, time and evidence are
//! inputs, rather than things it obtains from the outside world. The gate
//! checks its isolated dependency features as well as the workspace build.
//!
//! The CLI owns IO and model calls (`check`, `tensions`, `draft`, `rebase`).
//! See `docs/CONTRACT.md` for the laws and their assumptions.

#![no_std]
#![forbid(unsafe_code)]

#[macro_use]
extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod act;
pub mod allot;
pub mod date;
pub mod draw;
pub mod fold;
pub mod horizon;
pub mod id;
pub mod lineage;
pub mod log;
pub mod policy;
pub mod ratify;
pub mod scope;
pub mod standing;

pub use act::{Act, ActKind, FORMAT_VERSION};
pub use allot::{AdoptedAllocation, Allocation, Allotment, Award, Order, PoolError, Schedule};
pub use draw::{DrawError, Drawn};
pub use fold::{
    derive, Adopted, Ancestry, Canon, Commitment, Conflict, Disposition, Question, Retraction,
    Ruling, Silence, Stated, Status, Voice,
};
pub use horizon::{Due, Horizon, Overdue};
pub use id::{short_digest, ActId, ID_PREFIX};
pub use lineage::{Divergence, Fate, Inherited, Snapshot, SnapshotCommitment};
pub use log::{Log, ParseError};
pub use policy::{default_outcome, Attributes, Authority, Decision, Policy, Rule};
pub use ratify::{AdoptedRatify, Between, Proposal, Ratify, Seat, Verdict};
pub use scope::{Grant, Scope};
pub use standing::{Outcome, Position, Pull, Source, Standing};

#[cfg(test)]
mod tests;
