// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon-core` — the acts, their content-addressed ids, and the fold that
//! derives current state from them.
//!
//! **This crate does no IO and makes no network calls, and that is enforced by
//! its dependency list rather than by discipline.** It parses a `&str` and
//! folds it. The CLI owns the filesystem; a model, when one is involved at all,
//! is reached from higher layers.
//!
//! Three verbs in the CLI need a model (`check`, `tensions`, `draft`).
//! Everything else is this crate.

pub mod act;
pub mod fold;
pub mod id;
pub mod log;

pub use act::{Act, ActKind, FORMAT_VERSION};
pub use fold::{derive, Ancestry, Canon, Commitment, Conflict, Disposition, Status};
pub use id::{ActId, ID_PREFIX};
pub use log::{Log, ParseError};

#[cfg(test)]
mod tests;
