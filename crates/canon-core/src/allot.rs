// SPDX-License-Identifier: AGPL-3.0-or-later
//! Appropriation — who may take what, and when.
//!
//! The third pure function. [`crate::ratify`] answers *is this a rule yet*,
//! [`crate::policy`] answers *may I act*, and neither answers the question
//! every commons Ostrom studied actually turns on: **whose turn is it.**
//!
//! **The classical commons did not meter.** A Valencian huerta takes what it
//! needs while its turn lasts and never measures the water; Alanya draws its
//! named fishing sites in September and rotates one site a day; Törbel caps a
//! household at the cattle it can winter on its own land. All three allocate
//! *turns and positions*, not quantities. Metering is the modern shape — a
//! laser, a build farm — and it is the special case, not the core.
//!
//! That has one large consequence, and it is the same one the draw already
//! reached: **the schedule is a query, not an act.** Given the pool, the
//! holders, the rule and the clock, whose turn it is is a pure function that
//! every reader computes identically and nobody performs. A rotation costs no
//! per-turn bookkeeping at all, which is what makes it something a community
//! can adopt rather than something a community has to staff.
//!
//! It also all but dissolves the surveillance problem. These institutions
//! monitor by mutual visibility — you can see whose boat is on your site —
//! not by ledger. Nothing here records what anybody did.

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
    /// `step` is how far it moves and **its sign is the direction** — Alanya
    /// rotates east from September and west from January, which is one rule
    /// and a sign, not two rules.
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
                    step.abs()
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

/// One unit, and who holds it now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Award {
    pub unit: String,
    pub actor: String,
}

/// Who holds what, at a moment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Schedule {
    pub scope: Scope,
    pub unit: String,
    /// Which turn of the rotation this is, counted from adoption.
    pub period: i64,
    pub rule: String,
    pub awards: Vec<Award>,
    /// Holders this period has no unit for. Reported rather than dropped: a
    /// pool too small for its community is a finding about the commons.
    pub idle: Vec<String>,
    /// Units nobody holds, for the same reason.
    pub free: Vec<String>,
}

/// Why a schedule could not be computed. Refused rather than defaulted — a
/// pool that answers when it should not is worse than one that says it cannot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PoolError {
    NoAllotment,
    NoRule,
    NobodyHolds,
    Draw(String),
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAllotment => write!(f, "nothing is allotted here — `canon allot <scope> …`"),
            Self::NoRule => write!(
                f,
                "there is a pool here and no rule for sharing it — `canon allocation set …`"
            ),
            Self::NobodyHolds => write!(
                f,
                "nobody holds standing over this scope, so there is nobody to take a turn"
            ),
            Self::Draw(e) => write!(f, "the draw this rotation orders by is not settled: {e}"),
        }
    }
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

    /// Who holds which unit at a moment.
    ///
    /// Pure: the pool, the grants held then, the rule, and the clock. The
    /// eligible are **whoever holds standing covering the scope**, which is
    /// Ostrom's first principle doing a second job — boundaries decide who
    /// may appropriate, not only who may decide.
    pub fn pool_at(&self, scope: &Scope, at: i64) -> Result<Schedule, PoolError> {
        let allotment = self.allotment_for(scope).ok_or(PoolError::NoAllotment)?;
        let adopted = self.allocation_for(scope).ok_or(PoolError::NoRule)?;

        let mut holders: Vec<String> = self
            .grants
            .iter()
            .filter(|g| g.held_at(at) && g.scope.covers(scope))
            .map(|g| g.actor.clone())
            .collect();
        holders.sort();
        holders.dedup();
        if holders.is_empty() {
            return Err(PoolError::NobodyHolds);
        }

        let Allocation::Rotation { order, step, per } = &adopted.rule;
        let eligible = holders.clone();
        let holders = self.ordered(order, holders)?;
        let units = &allotment.units;
        let (n, m) = (holders.len() as i64, units.len() as i64);
        // Periods are counted from the adoption day, SNAPPED DOWN to a whole
        // multiple of `per`. Counting from the adoption *instant* is exactly
        // right and completely useless: a rule adopted at two in the
        // afternoon would turn over at two every afternoon, and somebody
        // asking whose turn it is on Thursday would be told Wednesday's
        // answer for most of the day. Snapping makes a day a day. It is
        // arithmetic from the unix epoch, so it needs no timezone, no leap
        // rule and no agreement about when a week starts.
        let period = if *per > 0 {
            let epoch = adopted.at - adopted.at.rem_euclid(*per);
            (at - epoch).div_euclid(*per)
        } else {
            0
        };

        let mut awards = Vec::new();
        if m > 0 {
            if n <= m {
                // Everyone gets one, and everyone advances along the list.
                for (j, actor) in holders.iter().enumerate() {
                    let k = (j as i64 + step * period).rem_euclid(m) as usize;
                    awards.push(Award {
                        unit: units[k].clone(),
                        actor: actor.clone(),
                    });
                }
            } else {
                // More holders than units: the units go to a window of
                // holders that moves, so a turn comes round to everybody
                // rather than to the first few forever.
                for (k, unit) in units.iter().enumerate() {
                    let j = (k as i64 + step * period).rem_euclid(n) as usize;
                    awards.push(Award {
                        unit: unit.clone(),
                        actor: holders[j].clone(),
                    });
                }
            }
        }
        awards.sort_by(|a, b| a.unit.cmp(&b.unit));

        let taken: Vec<&str> = awards.iter().map(|a| a.actor.as_str()).collect();
        let idle: Vec<String> = eligible
            .iter()
            .filter(|h| !taken.contains(&h.as_str()))
            .cloned()
            .collect();
        let held: Vec<&str> = awards.iter().map(|a| a.unit.as_str()).collect();
        let free: Vec<String> = units
            .iter()
            .filter(|u| !held.contains(&u.as_str()))
            .cloned()
            .collect();

        Ok(Schedule {
            scope: scope.clone(),
            unit: allotment.unit.clone(),
            period,
            rule: adopted.rule.name(),
            awards,
            idle,
            free,
        })
    }

    fn ordered(&self, order: &Order, holders: Vec<String>) -> Result<Vec<String>, PoolError> {
        match order {
            Order::Holders => Ok(holders),
            // A written order is the list of who takes turns — the huerta's
            // order down the canal, the articles' list of households. Two
            // rules, and both matter:
            //
            // A name that does not hold the scope is dropped. A fixed order
            // is a claim about sequence, never a second way to grant
            // somebody standing.
            //
            // A holder the order does not name takes no turn — which is how
            // a community keeps its monitor out of the rotation without
            // taking away its seat. They are not silently gone: the schedule
            // reports them as idle, because a holder nobody rostered is
            // exactly the disappearance this project exists to prevent.
            Order::Given { actors } => {
                let mut out: Vec<String> = Vec::new();
                for a in actors {
                    if holders.contains(a) && !out.contains(a) {
                        out.push(a.clone());
                    }
                }
                Ok(out)
            }
            Order::FromDraw { commit } => {
                let drawn = self
                    .draw(commit)
                    .map_err(|e| PoolError::Draw(e.to_string()))?;
                let mut out = holders;
                crate::draw::shuffle(&drawn.seed, &mut out);
                Ok(out)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::act::{Act, ActKind};
    use crate::log::Log;

    const DAY: i64 = 86_400;
    const START: i64 = 1_764_547_200; // a midnight, so periods line up with days

    /// The eleven named spots, west to east along the coast.
    fn sites() -> Vec<String> {
        [
            "kizilburun",
            "incekum",
            "karaburun",
            "mahmutlar",
            "konakli",
            "payallar",
            "turkler",
            "okurcalar",
            "avsallar",
            "demirtas",
            "kargicak",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
    }

    fn boats() -> Vec<String> {
        [
            "human:kemal",
            "human:ayla",
            "human:bora",
            "human:cemre",
            "human:deniz",
            "human:ege",
            "human:fikret",
            "human:gul",
            "human:halim",
            "human:irem",
            "human:jale",
        ]
        .iter()
        .map(|s| (*s).to_string())
        .collect()
    }

    fn scope(s: &str) -> Scope {
        Scope::new(s).expect("scope")
    }

    /// The fishery, its boats, its sites, and a rule for sharing them.
    fn fishery(order: Order, step: i64, holders: &[String], units: Vec<String>) -> Canon {
        let mut acts: Vec<Act> = holders
            .iter()
            .map(|h| {
                Act::new(
                    ActKind::Grant {
                        holder: h.clone(),
                        scope: scope("fishery.sites"),
                        horizon: None,
                        rationale: String::new(),
                    },
                    START,
                    "human:kemal",
                )
            })
            .collect();
        acts.push(Act::new(
            ActKind::Allot {
                text: "The written list of spots.".into(),
                unit: "site".into(),
                units,
                scope: scope("fishery.sites"),
            },
            START + DAY,
            "human:kemal",
        ));
        acts.push(Act::new(
            ActKind::Allocation {
                text: "Each boat moves one site east a day.".into(),
                rule: Allocation::Rotation {
                    order,
                    step,
                    per: DAY,
                },
                scope: scope("fishery.sites"),
            },
            START + DAY,
            "human:kemal",
        ));
        Log::from_acts(acts).derive_at(START + DAY)
    }

    fn site_of(canon: &Canon, at: i64, actor: &str) -> Option<String> {
        canon
            .pool_at(&scope("fishery.sites"), at)
            .expect("a schedule")
            .awards
            .into_iter()
            .find(|a| a.actor == actor)
            .map(|a| a.unit)
    }

    #[test]
    fn alanya_rotates_every_boat_one_site_east_a_day() {
        // Ostrom 1990, ch. 1: a hundred boats, a written list of named spots,
        // a draw each September, and from then each boat moves one site along
        // each day. This is the whole appropriation rule of a fishery that has
        // worked since the seventies, and it costs no per-turn act at all.
        let canon = fishery(Order::Given { actors: boats() }, 1, &boats(), sites());
        let day = |n: i64| START + DAY + n * DAY;

        assert_eq!(
            site_of(&canon, day(0), "human:kemal").as_deref(),
            Some("kizilburun")
        );
        assert_eq!(
            site_of(&canon, day(1), "human:kemal").as_deref(),
            Some("incekum")
        );
        assert_eq!(
            site_of(&canon, day(2), "human:kemal").as_deref(),
            Some("karaburun")
        );

        // Everybody is somewhere, nobody is in two places, and no site is
        // worked twice — which is the property that makes cheating visible to
        // your neighbour and is why the fishery needs no enforcer.
        for n in 0..13 {
            let s = canon
                .pool_at(&scope("fishery.sites"), day(n))
                .expect("schedule");
            assert_eq!(s.awards.len(), 11, "day {n}");
            let mut actors: Vec<&str> = s.awards.iter().map(|a| a.actor.as_str()).collect();
            actors.sort_unstable();
            actors.dedup();
            assert_eq!(actors.len(), 11, "day {n}: somebody is in two places");
            assert!(s.idle.is_empty() && s.free.is_empty(), "day {n}");
        }

        // Over one full turn of the wheel every boat works every site exactly
        // once. That is the fairness claim, and it is a property of the rule
        // rather than of anybody's good behaviour.
        let mut visited: Vec<String> = (0..11)
            .map(|n| site_of(&canon, day(n), "human:ayla").expect("a site"))
            .collect();
        visited.sort();
        visited.dedup();
        assert_eq!(visited.len(), 11, "ayla did not see every site");
        assert_eq!(
            site_of(&canon, day(11), "human:ayla"),
            site_of(&canon, day(0), "human:ayla"),
            "the wheel comes round"
        );
    }

    #[test]
    fn the_sign_of_the_step_is_the_direction() {
        // Alanya rotates east from September and west from January. One rule
        // and a sign, not two rules.
        let east = fishery(Order::Given { actors: boats() }, 1, &boats(), sites());
        let west = fishery(Order::Given { actors: boats() }, -1, &boats(), sites());
        let day1 = START + DAY + DAY;
        assert_eq!(
            site_of(&east, day1, "human:kemal").as_deref(),
            Some("incekum")
        );
        assert_eq!(
            site_of(&west, day1, "human:kemal").as_deref(),
            Some("kargicak")
        );
    }

    #[test]
    fn a_holder_the_roster_does_not_name_takes_no_turn_and_is_reported() {
        // The monitor holds the sites so it can watch them, and a watcher is
        // not an appropriator. Leaving it off the roster must not make it
        // vanish from the answer — a holder nobody rostered is exactly the
        // disappearance this project exists to prevent.
        let mut holders = boats();
        holders.push("agent:logwatch".into());
        let canon = fishery(Order::Given { actors: boats() }, 1, &holders, sites());
        let s = canon
            .pool_at(&scope("fishery.sites"), START + DAY)
            .expect("a schedule");
        assert_eq!(s.awards.len(), 11);
        assert_eq!(s.idle, vec!["agent:logwatch".to_string()]);
    }

    #[test]
    fn more_boats_than_sites_rotates_who_fishes_at_all() {
        // A pool too small for its community is the commons problem itself.
        // The turn has to come round to everybody rather than to the first
        // few forever, and the ones left out this turn are named.
        let canon = fishery(
            Order::Given { actors: boats() },
            1,
            &boats(),
            sites()[..4].to_vec(),
        );
        let day = |n: i64| START + DAY + n * DAY;
        let mut ever: Vec<String> = Vec::new();
        for n in 0..11 {
            let s = canon
                .pool_at(&scope("fishery.sites"), day(n))
                .expect("schedule");
            assert_eq!(s.awards.len(), 4, "day {n}");
            assert_eq!(s.idle.len(), 7, "day {n}: the rest are named, not dropped");
            for a in s.awards {
                if !ever.contains(&a.actor) {
                    ever.push(a.actor);
                }
            }
        }
        assert_eq!(ever.len(), 11, "over eleven days everyone gets a turn");
    }

    #[test]
    fn a_pool_with_no_rule_refuses_rather_than_guessing() {
        let acts = vec![
            Act::new(
                ActKind::Grant {
                    holder: "human:kemal".into(),
                    scope: scope("fishery.sites"),
                    horizon: None,
                    rationale: String::new(),
                },
                START,
                "human:kemal",
            ),
            Act::new(
                ActKind::Allot {
                    text: "The list.".into(),
                    unit: "site".into(),
                    units: sites(),
                    scope: scope("fishery.sites"),
                },
                START + DAY,
                "human:kemal",
            ),
        ];
        let canon = Log::from_acts(acts).derive_at(START + DAY);
        assert_eq!(
            canon.pool_at(&scope("fishery.sites"), START + DAY),
            Err(PoolError::NoRule)
        );
        assert_eq!(
            canon.pool_at(&scope("harbour"), START + DAY),
            Err(PoolError::NoAllotment)
        );
    }

    #[test]
    fn allotting_a_scope_you_do_not_hold_is_recorded_and_not_applied() {
        let acts = vec![
            Act::new(
                ActKind::Grant {
                    holder: "human:kemal".into(),
                    scope: scope("fishery.sites"),
                    horizon: None,
                    rationale: String::new(),
                },
                START,
                "human:kemal",
            ),
            Act::new(
                ActKind::Allot {
                    text: "mine now".into(),
                    unit: "site".into(),
                    units: sites(),
                    scope: scope("fishery.sites"),
                },
                START + DAY,
                "human:stranger",
            ),
        ];
        let canon = Log::from_acts(acts).derive_at(START + DAY);
        assert!(canon.allotments.is_empty(), "it did not take");
        assert_eq!(canon.ungoverned.len(), 1);
        assert!(canon.ungoverned[0].1.contains("allotted"));
    }
}
