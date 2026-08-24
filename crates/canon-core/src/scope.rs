// SPDX-License-Identifier: AGPL-3.0-or-later
//! Who holds standing, over what.
//!
//! Ostrom's **first** design principle, from the study of common-pool-resource
//! institutions that lasted centuries: clearly defined boundaries — who holds
//! rights, and over which resource. Not her eighth. Systems without it do not
//! endure, and every richer policy this library can carry (subsidiarity,
//! sortition, cohort ratification, scoped authority, delegation) is unstateable
//! without it.
//!
//! It is also the floor under Freeman's problem. Informal power runs on private
//! knowledge of the process — who decides, and how you would find out. A group
//! where "who decides this?" can only be answered by asking the right person
//! has made that person the gatekeeper. [`crate::Canon::who_decides`] exists so
//! the answer is a query.
//!
//! **A scope is a dotted path, and nesting is Ostrom's eighth principle for
//! free.** `house` covers `house.kitchen`; a grant at the top covers everything
//! under it, and a policy that prefers the deepest grant is subsidiarity —
//! decisions at the lowest competent level — with no extra machinery.

use serde::{Deserialize, Serialize};

use crate::id::ActId;

/// A dotted path naming what someone holds standing over.
///
/// Validated on construction rather than checked at each use: an empty segment
/// would make [`Scope::covers`] answer wrongly, and a scope that cannot be
/// trusted to compare is worse than no scope at all.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Scope(String);

impl Scope {
    /// `house.kitchen` — segments separated by dots, none of them empty.
    ///
    /// Returns `None` rather than repairing the input. A silently-fixed
    /// `house..kitchen` is a scope nobody wrote and everybody would then have
    /// to reason about.
    pub fn new(path: &str) -> Option<Self> {
        let path = path.trim();
        if path.is_empty() || path.split('.').any(|seg| seg.trim().is_empty()) {
            return None;
        }
        Some(Self(path.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Does this scope contain `other`?
    ///
    /// **The boundary must be a dot.** `house` covers `house.kitchen` and
    /// itself, and does NOT cover `household` — a bare `starts_with` is the
    /// classic prefix trap, and here it would hand someone authority over a
    /// scope that merely spells similarly.
    pub fn covers(&self, other: &Scope) -> bool {
        if self.0 == other.0 {
            return true;
        }
        other
            .0
            .strip_prefix(&self.0)
            .is_some_and(|rest| rest.starts_with('.'))
    }

    /// How deep this sits. Subsidiarity prefers the largest.
    pub fn depth(&self) -> usize {
        self.0.split('.').count()
    }

    /// The scope one level up, if any.
    pub fn parent(&self) -> Option<Self> {
        self.0.rsplit_once('.').map(|(head, _)| Self(head.into()))
    }
}

impl TryFrom<String> for Scope {
    type Error = String;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::new(&s).ok_or_else(|| format!("`{s}` is not a scope: dotted path, no empty segments"))
    }
}

impl From<Scope> for String {
    fn from(s: Scope) -> Self {
        s.0
    }
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Standing somebody holds, over some scope, possibly until some date.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Grant {
    pub actor: String,
    pub scope: Scope,
    /// When it lapses. `None` is standing with no end — which a community may
    /// want and should have to choose, because rotation is the default shape
    /// that keeps incumbency from being the default outcome.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<i64>,
    pub granted_at: i64,
    /// When it was given up, stood down, or replaced by a re-grant.
    ///
    /// **Kept rather than deleted, and that is what makes standing an AS-OF
    /// question.** A live-list-only model cannot answer "who held the kitchen
    /// in March", and worse, it lets a withdrawal today silently change a
    /// pool that was supposed to be frozen in March — which is the pool-churn
    /// attack the draw's threat model names, arriving from the other
    /// direction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawn_at: Option<i64>,
    /// The act that granted it, so it can be reverted like anything else.
    pub act: ActId,
}

impl Grant {
    /// Has this lapsed by `now`?
    ///
    /// An expired grant is not deleted — it is a fact that happened, and the
    /// staleness query wants to say "this lapsed and nobody renewed it".
    pub fn lapsed(&self, now: i64) -> bool {
        self.horizon.is_some_and(|h| now > h)
    }

    /// Was this standing actually held at that moment?
    ///
    /// Three ways it is not: it had not been given yet, it had lapsed, or it
    /// had been given up. All three are dates, so all three are answerable
    /// about any moment rather than only about now.
    pub fn held_at(&self, now: i64) -> bool {
        self.granted_at <= now && !self.lapsed(now) && self.withdrawn_at.is_none_or(|w| w > now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scope_with_an_empty_segment_is_refused_not_repaired() {
        assert!(Scope::new("house..kitchen").is_none());
        assert!(Scope::new("").is_none());
        assert!(Scope::new(".").is_none());
        assert!(Scope::new("house.").is_none());
        assert!(Scope::new(".house").is_none());
        assert!(Scope::new("house.kitchen").is_some());
    }

    #[test]
    fn a_scope_covers_what_nests_under_it_and_not_what_merely_spells_like_it() {
        let house = Scope::new("house").unwrap();
        let kitchen = Scope::new("house.kitchen").unwrap();
        let household = Scope::new("household").unwrap();

        assert!(house.covers(&house), "and itself");
        assert!(house.covers(&kitchen));
        assert!(
            !house.covers(&household),
            "the prefix trap: `household` is not inside `house`"
        );
        assert!(!kitchen.covers(&house), "and never upward");
    }

    #[test]
    fn depth_orders_the_specific_above_the_general() {
        // Which is all subsidiarity needs: prefer the deepest grant that
        // covers the question.
        assert_eq!(Scope::new("house").unwrap().depth(), 1);
        assert_eq!(Scope::new("house.kitchen.rota").unwrap().depth(), 3);
        assert_eq!(
            Scope::new("house.kitchen").unwrap().parent(),
            Scope::new("house")
        );
        assert_eq!(Scope::new("house").unwrap().parent(), None);
    }

    #[test]
    fn a_scope_round_trips_through_its_string() {
        let s = Scope::new("house.kitchen").unwrap();
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(json, "\"house.kitchen\"");
        assert_eq!(serde_json::from_str::<Scope>(&json).unwrap(), s);
        // And a malformed one is refused at the edge, not carried inward.
        assert!(serde_json::from_str::<Scope>("\"house..kitchen\"").is_err());
    }

    #[test]
    fn a_grant_lapses_only_after_its_horizon() {
        let g = Grant {
            actor: "human:dana".into(),
            scope: Scope::new("house.kitchen").unwrap(),
            horizon: Some(200),
            granted_at: 100,
            withdrawn_at: None,
            act: ActId::from_raw("can-000000000000"),
        };
        assert!(!g.lapsed(199));
        assert!(!g.lapsed(200), "on the day is still standing");
        assert!(g.lapsed(201));

        let forever = Grant { horizon: None, ..g };
        assert!(!forever.lapsed(i64::MAX));
    }

    #[test]
    fn standing_is_an_as_of_question_and_not_only_a_now_question() {
        let g = Grant {
            actor: "human:dana".into(),
            scope: Scope::new("house").unwrap(),
            horizon: None,
            granted_at: 100,
            withdrawn_at: Some(500),
            act: ActId::from_raw("can-000000000000"),
        };
        assert!(!g.held_at(99), "before it was given");
        assert!(g.held_at(100), "on the day it was given");
        assert!(g.held_at(499));
        assert!(!g.held_at(500), "the moment it was given up");
        assert!(!g.held_at(10_000));
    }
}
