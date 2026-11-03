// SPDX-License-Identifier: AGPL-3.0-or-later
//! Optional institutional interpretation. History and time are explicit inputs.
//! Typed temporal v2 and the compatible fixed-basis v1 reader; see DESIGN.md.
#![no_std]
#![forbid(unsafe_code)]

#[macro_use]
extern crate alloc;
#[cfg(test)]
extern crate std;

mod admission;
mod decision;
pub use admission::project;
pub use decision::AdmissionState;
mod temporal;
pub use temporal::{interpret, Arrangements, Decision, Institution};

/// Compatibility adapter: `None` preserves the last-verified-act clock used by
/// existing record readers; `Some` uses the caller's explicit current clock.
pub fn legacy(log: &canon_core::Log, at: Option<i64>) -> canon_core::Canon {
    at.map_or_else(|| log.derive(), |at| log.derive_at(at))
}
