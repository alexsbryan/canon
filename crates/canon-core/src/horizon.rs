// SPDX-License-Identifier: AGPL-3.0-or-later
//! Horizons, and the one query that answers what has gone stale.
//!
//! **One generalization pays for five technologies.** Term limits, sunset
//! clauses, trial periods, revisit dates and rotation are the same shape: a
//! date somebody attached to a decision, plus a query for what is past it.
//! That is the strongest evidence in `PRIMITIVES.md` that this is a primitive
//! and not a feature — five names for one mechanism means the mechanism was
//! there all along.
//!
//! Systems like this rot because things accumulate and nothing closes. Every
//! surface here is additive by nature — you assert, you grant, you accept a
//! contradiction — and a store nobody ever subtracts from reads as
//! authoritative long after it stopped being true. A closure query is the
//! cheapest available defense, and it is the whole difference between
//! deferring something and burying it.
//!
//! **The clock is passed in.** Nothing in this crate reads the system time.
//! A replay whose answer depends on when it ran is not a replay, and
//! `canon replay` leans on this completely.

use serde::{Deserialize, Serialize};

use crate::fold::Canon;
use crate::id::ActId;
use crate::scope::Scope;

/// Why something is overdue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Due {
    /// Somebody said they would look at this again by a date.
    Horizon {
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// A contradiction carried knowingly, with a revisit date that has
    /// passed. `Accept.revisit`'s own comment — "a date that has passed is
    /// not noise; it is the signal" — generalized rather than special-cased.
    Revisit {
        other: ActId,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        rationale: String,
    },
    /// Standing that lapsed and nobody renewed. A term limit doing its job,
    /// or an agent's authority quietly expiring — the same event either way.
    Standing { holder: String, scope: Scope },
}

/// Something whose date has passed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Overdue {
    /// What is overdue — the commitment, the accept, or the grant.
    pub target: ActId,
    /// When it came due.
    pub due: i64,
    pub what: Due,
}

impl Canon {
    /// What has gone past its date, oldest first.
    ///
    /// Excludes anything already closed: a commitment that was retracted or
    /// superseded has been dealt with, and re-surfacing it would train people
    /// to ignore the query — which is the failure mode of every staleness
    /// report that ever shipped.
    pub fn overdue(&self, now: i64) -> Vec<Overdue> {
        let mut out = Vec::new();

        for h in &self.horizons {
            if h.at > now {
                continue;
            }
            // A horizon on a commitment that is no longer live is moot: the
            // thing it asked to be revisited has been.
            if self
                .get(&h.target)
                .is_some_and(|c| !matches!(c.status, crate::fold::Status::Active))
            {
                continue;
            }
            if self
                .question(&h.target)
                .is_some_and(|q| !matches!(q.status, crate::fold::Status::Active))
            {
                continue;
            }
            out.push(Overdue {
                target: h.target.clone(),
                due: h.at,
                what: Due::Horizon {
                    rationale: h.rationale.clone(),
                },
            });
        }

        for c in self.tolerated() {
            let crate::fold::Disposition::Tolerated {
                revisit: Some(raw),
                rationale,
            } = &c.disposition
            else {
                continue;
            };
            // Unreadable dates are NOT treated as overdue and NOT treated as
            // absent — they are reported separately by `unreadable_dates`.
            // Reading "spring" as epoch zero would make it permanently
            // overdue, which is how a closure query becomes noise.
            let Some(due) = crate::date::parse_ymd(raw) else {
                continue;
            };
            if due > now {
                continue;
            }
            out.push(Overdue {
                target: c.a.clone(),
                due,
                what: Due::Revisit {
                    other: c.b.clone(),
                    rationale: rationale.clone(),
                },
            });
        }

        for g in &self.grants {
            // Lapsed, and not already closed some other way. Standing that
            // somebody gave up before its horizon was dealt with; surfacing
            // it as overdue would be the query re-raising settled work.
            if !g.lapsed(now) || g.withdrawn_at.is_some() || g.granted_at > now {
                continue;
            }
            out.push(Overdue {
                target: g.act.clone(),
                due: g.horizon.unwrap_or_default(),
                what: Due::Standing {
                    holder: g.actor.clone(),
                    scope: g.scope.clone(),
                },
            });
        }

        // Oldest first: the thing that has been waiting longest is the thing
        // most likely to have quietly stopped being true.
        out.sort_by(|a, b| a.due.cmp(&b.due).then_with(|| a.target.cmp(&b.target)));
        out
    }

    /// Dates written into the log that are not dates.
    ///
    /// Reported rather than defaulted (§18.3). A `revisit` somebody typed as
    /// "spring" is a real intention with an unreadable deadline, and the two
    /// wrong answers are equally bad: silently dropping it loses the
    /// intention, and reading it as epoch zero makes it permanently overdue.
    pub fn unreadable_dates(&self) -> Vec<(ActId, String)> {
        self.conflicts
            .iter()
            .filter_map(|c| match &c.disposition {
                crate::fold::Disposition::Tolerated {
                    revisit: Some(raw), ..
                } if crate::date::parse_ymd(raw).is_none() => Some((c.a.clone(), raw.clone())),
                _ => None,
            })
            .collect()
    }
}

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
