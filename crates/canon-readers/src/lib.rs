// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon-readers` — views computed from a derived record: whose turn it is,
//! who a draw seats, what has gone past its date, and how a canon has moved
//! from the one it was adopted from.
//!
//! Nothing here is read by the fold or by any ratification rule, and nothing
//! here can change what is in force. `canon-core` holds the acts and the
//! types they embed; these are readings of them, and carry no kernel claim.

#![no_std]
#![forbid(unsafe_code)]

#[macro_use]
extern crate alloc;

#[cfg(test)]
extern crate std;

pub mod allot;
pub mod draw;
pub mod horizon;
pub mod lineage;

pub use allot::{Award, PoolError, Pools, Schedule};
pub use draw::{DrawError, Drawn, Draws};
pub use horizon::{Due, Overdue, Reviews};
pub use lineage::{Divergence, Fate, Inherited, Snapshot, SnapshotCommitment};
