// SPDX-License-Identifier: AGPL-3.0-or-later
//! Who the record names as a person.
//!
//! Ratification counts a person's word and nothing else's. Its counts hold
//! [`Person`]s and this module is the only place one is made, so a new way of
//! counting that forgets to ask whether an actor is a person does not
//! compile. A module of its own rather than a type in `ratify`, because
//! privacy stops at the module: inside `ratify` the field would be open.

use core::fmt;

/// What the record writes before a person's name.
pub const PREFIX: &str = "human:";

/// An actor the record names as a person: `human:<name>`.
///
/// A label, not authentication, until the name binds a key (`crate::sign`).
/// The field is private and [`Person::of`] is the one constructor:
///
/// ```compile_fail,E0603
/// let forged = canon_core::Person("agent:helper");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Person<'a>(&'a str);

impl<'a> Person<'a> {
    /// The actor, if the record names a person.
    pub fn of(actor: &'a str) -> Option<Self> {
        actor.starts_with(PREFIX).then_some(Self(actor))
    }

    /// The actor as written, prefix included.
    pub fn actor(self) -> &'a str {
        self.0
    }
}

impl fmt::Display for Person<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}
