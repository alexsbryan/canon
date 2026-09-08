// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ratification — how a proposal becomes a rule.
//!
//! Ostrom separated three levels of rules. *Operational* rules say what you
//! may do; *collective-choice* rules say how operational rules get made and
//! changed; *constitutional* rules say how the collective-choice rules
//! themselves change. Before this module the canon had the first level and a
//! policy for judging proposals to *act*, and nothing in between: any actor
//! could `assert` a rule over any scope and it was live the moment it was
//! written. Everyone had equal authorship, which is not a governance model,
//! it is a shared notebook.
//!
//! This is the collective-choice level, as a pure function. A commitment
//! written into a scope is a **proposal** until the scope's ratification rule
//! says it is a rule. The rule is chosen per scope and lives in the ledger as
//! a [`crate::ActKind::Ratification`] act, so it is itself subject to `why`,
//! to `supersede`, and to the counterfactual replay.
//!
//! **The rule change is itself a proposal.** Writing a scope's ratification
//! rule takes standing over the scope or the one above it — the
//! constitutional level, one step up — and then the act is judged under the
//! rule in force for that scope when it was written, exactly as a commitment
//! would be. Under `standing` a holder's change lands at once, which is what
//! every canon did before; under `joint`, `threshold`, `consent` or `twice`
//! it waits for the same people the old rule names. The amendment clause
//! governs its own amendment, and nobody lowers the bar for their own corner
//! without clearing the bar that stands. A ratified change takes effect from
//! the moment it was ratified, not from the moment it was written.
//!
//! **Who counts.** Approvals and objections are ordinary `position` acts whose
//! `about` is the proposal's id. Only positions from people who hold standing
//! over the proposal's scope count; anyone may speak, and the record keeps
//! it, but Ostrom's third principle says the people who live under a rule are
//! the ones who change it. Positions from agents never count towards
//! ratification, and an agent never ratifies its own proposal by holding
//! standing: a monitor is answerable to the people, not the other way round.
//! An agent with standing may propose and may object; it cannot mint.
//!
//! **First word wins.** A reasoned objection refuses a proposal only if it
//! lands before the proposal completed. An objection written a year after a
//! rule was carried is on the record and changes nothing — otherwise one
//! holder could unmake any settled rule at any time, and through the rule of
//! rules, unmake everything decided under it.
//!
//! **What the default is.** [`Ratify::Standing`]: whoever holds the scope may
//! write into it directly, and a scope nobody holds is open. That is exactly
//! the behaviour every canon had before this module existed, now chosen
//! rather than assumed, and a house can raise it with one act.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::fold::{Canon, Commitment};
use crate::id::ActId;
use crate::scope::Scope;
use crate::standing::Pull;

const DAY: i64 = 86_400;

/// What has to pass between the two votes of a [`Ratify::Twice`] rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "between", rename_all = "snake_case")]
pub enum Between {
    /// This many days after the first vote completed.
    Days { days: u32 },
    /// Somebody who did not hold the scope at the first vote has been
    /// granted it. The intervening election: a coalition cannot satisfy this
    /// alone except by admitting somebody, on the record, under its own name.
    Turnover,
}

/// How a proposal in a scope becomes a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Ratify {
    /// A holder of the scope may write a rule directly. A non-holder's
    /// proposal takes one holder's approval. A scope nobody holds is open.
    Standing,
    /// Every one of these named people must approve. One of them objecting
    /// refuses it.
    Joint { holders: Vec<String> },
    /// This many holders approving carries it; this many objecting refuses
    /// it. The proposer counts as approving when they hold the scope.
    Threshold { approve: usize, block: usize },
    /// It becomes a rule after this many days unless a holder objects with a
    /// reason. One reasoned objection refuses it. Silence is consent.
    Consent { days: u32 },
    /// Carried twice under `each`, with `between` in the middle. Cheap over a
    /// long horizon and near-impossible over a short one, which is the only
    /// mechanism on the record that raises the cost of capture without
    /// depending on anyone's virtue: intent is unverifiable, so it demands a
    /// duration that only genuine broad support survives.
    Twice { each: Box<Ratify>, between: Between },
}

impl Between {
    fn name(&self) -> String {
        match self {
            Self::Days { days } => format!("{days}d"),
            Self::Turnover => "turnover".into(),
        }
    }
}

impl Ratify {
    pub fn name(&self) -> String {
        match self {
            Self::Standing => "standing".into(),
            Self::Joint { holders } => format!("joint:{}", holders.join(",")),
            Self::Threshold { approve, block } => format!("threshold:{approve}/{block}"),
            Self::Consent { days } => format!("consent:{days}d"),
            Self::Twice { each, between } => format!("twice:{}:{}", between.name(), each.name()),
        }
    }

    /// How it reads to a person who did not write it in their own words.
    pub fn prose(&self) -> String {
        match self {
            Self::Standing => {
                "Whoever holds this scope may write its rules. Anyone else proposes, and one \
                 holder's approval makes it a rule."
                    .into()
            }
            Self::Joint { holders } => format!(
                "A rule here takes the approval of every one of: {}. One of them objecting \
                 refuses it.",
                holders.join(", ")
            ),
            Self::Threshold { approve, block } => format!(
                "A rule here takes {approve} holder(s) approving; {block} objecting refuses it."
            ),
            Self::Consent { days } => format!(
                "A proposal here becomes a rule after {days} day(s) unless a holder objects \
                 with a reason."
            ),
            Self::Twice { each, between } => {
                let gap = match between {
                    Between::Days { days } => format!("{days} day(s)"),
                    Between::Turnover => "someone new being granted the scope".to_string(),
                };
                let mut s = format!(
                    "A rule here is carried twice, with {gap} between the two votes. Each \
                     vote: {}",
                    each.prose()
                );
                if matches!(**each, Self::Joint { .. }) {
                    s.push_str(
                        " Joint names the same people both times, so the second vote adds a \
                         waiting period and no new voices.",
                    );
                }
                s
            }
        }
    }

    /// The one spelling, shared by the CLI and the seed dialect:
    /// `standing`, `joint:dana,sam`, `threshold:2/1`, `consent:7d`,
    /// `twice:turnover:consent:7d`, `twice:90d:threshold:2/1`.
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim();
        if raw == "standing" {
            return Some(Self::Standing);
        }
        if let Some(rest) = raw.strip_prefix("joint:") {
            let holders: Vec<String> = rest
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            return (!holders.is_empty()).then_some(Self::Joint { holders });
        }
        if let Some(rest) = raw.strip_prefix("threshold:") {
            let (a, b) = rest.split_once('/')?;
            return Some(Self::Threshold {
                approve: a.trim().parse().ok()?,
                block: b.trim().parse().ok()?,
            });
        }
        if let Some(rest) = raw.strip_prefix("consent:") {
            let days = rest.trim().strip_suffix('d').unwrap_or(rest.trim());
            return Some(Self::Consent {
                days: days.parse().ok()?,
            });
        }
        if let Some(rest) = raw.strip_prefix("twice:") {
            // The tail is the inner rule, so nested colons are unambiguous.
            let (gap, inner) = rest.split_once(':')?;
            let between = match gap.trim() {
                "turnover" => Between::Turnover,
                d => {
                    let days: u32 = d.strip_suffix('d')?.parse().ok()?;
                    if days == 0 {
                        return None;
                    }
                    Between::Days { days }
                }
            };
            let each = Self::parse(inner)?;
            // Twice over twice reads as nothing a person could explain.
            if matches!(each, Self::Twice { .. }) {
                return None;
            }
            return Some(Self::Twice {
                each: Box::new(each),
                between,
            });
        }
        None
    }
}

/// A ratification rule someone adopted for a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdoptedRatify {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
    pub rule: Ratify,
    pub text: String,
    /// When it was written.
    pub at: i64,
    pub actor: String,
    pub act: ActId,
    /// Where the change itself stands under the rule it is changing. Only a
    /// ratified change decides anything, and only from `since` on.
    pub verdict: Verdict,
    /// The rule it was judged under: what the scope had when this was
    /// written. Kept so `show` and `replay` can say which rule decided,
    /// without asking the question again minus this entry.
    pub under: Ratify,
}

/// Where a proposal stands under its scope's rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// A rule, from `since` on. `how` says which rule made it one and on
    /// whose word.
    Ratified { since: i64, how: String },
    /// Not yet. `needs` says what would make it one.
    Proposed { needs: String },
    /// Refused by the people the rule names. `why` quotes the objection.
    Refused { at: i64, by: String, why: String },
}

/// Something put to a scope: a commitment, or a change to how the scope
/// decides. The verdict reads nothing else about it.
#[derive(Debug, Clone, Copy)]
pub struct Proposal<'a> {
    pub id: &'a ActId,
    pub scope: Option<&'a Scope>,
    /// When it was written.
    pub at: i64,
    pub actor: &'a str,
}

fn is_human(actor: &str) -> bool {
    actor.starts_with("human:")
}

/// The counted word of the people a rule names, in time order.
struct Tally<'a> {
    /// Who approved and when they first did. The proposer's own act, when it
    /// counts, is the earliest entry.
    approved: Vec<(&'a str, i64)>,
    /// Reasoned objections: `(at, by, why)`.
    objections: Vec<(i64, &'a str, &'a str)>,
    /// Every position against, reasoned or not, by time. Threshold's block
    /// counts these.
    against: Vec<(i64, &'a str)>,
}

impl<'a> Tally<'a> {
    fn first_objection(&self) -> Option<(i64, &'a str, &'a str)> {
        self.objections.first().copied()
    }
}

/// What one rule concludes, before deciding which came first.
struct Outcome {
    completed: Option<(i64, String)>,
    refused: Option<(i64, String, String)>,
    needs: String,
}

/// First word wins: refused only if the objection precedes completion.
fn settle(o: Outcome) -> Verdict {
    match (o.completed, o.refused) {
        (Some((done, _)), Some((at, by, why))) if at < done => Verdict::Refused { at, by, why },
        (Some((since, how)), _) => Verdict::Ratified { since, how },
        (None, Some((at, by, why))) => Verdict::Refused { at, by, why },
        (None, None) => Verdict::Proposed { needs: o.needs },
    }
}

fn where_(scope: Option<&Scope>) -> String {
    scope.map_or_else(|| "this canon".to_string(), ToString::to_string)
}

impl Canon {
    /// The ratification rule for a scope now: the deepest one that covers
    /// it, else the canon-wide one, else [`Ratify::Standing`].
    pub fn ratification_for(&self, scope: Option<&Scope>) -> &Ratify {
        self.ratification_for_at(scope, i64::MAX)
    }

    /// The rule that governed a scope at a moment. A rule is judged under
    /// the ratification rule in force WHEN IT WAS WRITTEN: tightening the
    /// kitchen's rule today does not un-ratify what the cooks wrote last
    /// year, and loosening it does not wave through what was still waiting.
    ///
    /// Only a ratified change counts, and only from the moment it was
    /// ratified. Two changes ratified in the same second are ordered by
    /// id, which is deterministic and arbitrary; a house that cares writes
    /// them a second apart.
    pub fn ratification_for_at(&self, scope: Option<&Scope>, at: i64) -> &Ratify {
        self.adopted_at(scope, at).map_or(&SHIPPED, |r| &r.rule)
    }

    /// The adopted rule deciding a scope at a moment, if any was.
    pub fn adopted_at(&self, scope: Option<&Scope>, at: i64) -> Option<&AdoptedRatify> {
        let mut best: Option<(&AdoptedRatify, i64)> = None;
        for r in &self.ratifications {
            let Verdict::Ratified { since, .. } = r.verdict else {
                continue;
            };
            if since > at {
                continue;
            }
            let applies = match (&r.scope, scope) {
                (None, _) => true,
                (Some(s), Some(target)) => s.covers(target),
                (Some(_), None) => false,
            };
            if !applies {
                continue;
            }
            let depth = r.scope.as_ref().map_or(0, Scope::depth);
            // Deepest wins; among equals, the latest in force, then the
            // latest written.
            let beats = best.is_none_or(|(b, bs)| {
                let bd = b.scope.as_ref().map_or(0, Scope::depth);
                depth > bd || (depth == bd && (since > bs || (since == bs && r.at >= b.at)))
            });
            if beats {
                best = Some((r, since));
            }
        }
        best.map(|(r, _)| r)
    }

    /// May this actor change how a scope is governed — grant standing over
    /// it, set its policy, set its ratification rule?
    ///
    /// **The constitutional level, one step up.** Standing over a scope
    /// includes standing over everything above it, so holding `house` is
    /// enough to govern `house.kitchen` and holding only `house.kitchen` is
    /// enough to govern the kitchen but not the house. A canon that has
    /// never granted standing to anyone is ungoverned and open: that is the
    /// bootstrap, and the first grant closes it.
    ///
    /// **Only grants made strictly before `at` count**, for and against.
    /// Acts routinely share a second, and the log orders them within it by
    /// id, which is deterministic and arbitrary. A founder writing twelve
    /// grants in one sitting must not find the first one to sort has locked
    /// the other eleven out. Simultaneous acts cannot govern each other.
    pub fn may_govern(&self, actor: &str, scope: Option<&Scope>, at: i64) -> bool {
        let prior: Vec<&crate::scope::Grant> = self
            .grants
            .iter()
            .filter(|g| g.granted_at < at && g.held_at(at))
            .collect();
        if self.grants.iter().all(|g| g.granted_at >= at) {
            return true;
        }
        let holds_any = || prior.iter().any(|g| g.actor == actor);
        match scope {
            Some(s) => {
                let covering: Vec<&&crate::scope::Grant> =
                    prior.iter().filter(|g| g.scope.covers(s)).collect();
                covering.iter().any(|g| g.actor == actor) || (covering.is_empty() && holds_any())
            }
            // The whole canon: anyone holding a top-level scope, or anyone
            // at all if no top-level scope has been granted.
            None => {
                let top: Vec<&&crate::scope::Grant> =
                    prior.iter().filter(|g| g.scope.depth() == 1).collect();
                if top.is_empty() {
                    holds_any()
                } else {
                    top.iter().any(|g| g.actor == actor)
                }
            }
        }
    }

    /// The people who ratify a scope's rules: those who hold it at the
    /// NARROWEST level anyone does — the kitchen's holders for a kitchen
    /// rule, even though the whole house covers the kitchen. That is
    /// subsidiarity, and it is the same reading `check` gives under it:
    /// wider standing asks, it does not act. For an unscoped rule the
    /// narrowest level is the shallowest anyone holds.
    fn holders_at(&self, scope: Option<&Scope>, at: i64) -> BTreeSet<&str> {
        let covering: Vec<&crate::scope::Grant> = self
            .grants
            .iter()
            .filter(|g| g.held_at(at) && scope.is_none_or(|s| g.scope.covers(s)))
            .collect();
        let level = match scope {
            Some(_) => covering.iter().map(|g| g.scope.depth()).max(),
            None => covering.iter().map(|g| g.scope.depth()).min(),
        };
        covering
            .iter()
            .filter(|g| Some(g.scope.depth()) == level && is_human(&g.actor))
            .map(|g| g.actor.as_str())
            .collect()
    }

    fn holder_at(&self, scope: Option<&Scope>, actor: &str, at: i64) -> bool {
        is_human(actor) && self.holders_at(scope, at).contains(actor)
    }

    /// Held BEFORE the moment. A house whose first rules and first grants
    /// were written in the same sitting has not locked its founders out of
    /// their own charter; see [`Canon::may_govern`].
    fn nobody_holds_before(&self, scope: Option<&Scope>, at: i64) -> bool {
        !self
            .grants
            .iter()
            .any(|g| g.granted_at < at && g.held_at(at) && scope.is_none_or(|s| g.scope.covers(s)))
    }

    /// The earliest moment after `t1` at which someone who did not hold the
    /// scope at `t1` holds it, and who that was.
    fn turnover_after(&self, scope: Option<&Scope>, t1: i64) -> Option<(i64, String)> {
        let then = self.holders_at(scope, t1);
        self.grants
            .iter()
            .filter(|g| {
                g.granted_at > t1
                    && is_human(&g.actor)
                    && scope.is_none_or(|s| g.scope.covers(s))
                    && !then.contains(g.actor.as_str())
                    && self.holder_at(scope, &g.actor, g.granted_at)
            })
            .map(|g| (g.granted_at, g.actor.clone()))
            .min()
    }

    /// What the people the rule names said about it. Only human holders
    /// count; the record keeps everyone's word, ratification counts the
    /// people the rule names. `after` drops everything at or before a
    /// moment; `implicit` counts the proposer's own act as their approval.
    fn tally<'a>(&'a self, p: &Proposal<'a>, after: Option<i64>, implicit: bool) -> Tally<'a> {
        let mut t = Tally {
            approved: Vec::new(),
            objections: Vec::new(),
            against: Vec::new(),
        };
        if implicit && self.holder_at(p.scope, p.actor, p.at) {
            t.approved.push((p.actor, p.at));
        }
        let mut said: Vec<&crate::fold::Stated> = self
            .positions
            .iter()
            .filter(|s| s.about == p.id.as_str() && after.is_none_or(|a| s.at > a))
            .collect();
        said.sort_by_key(|s| s.at);
        for s in said {
            if !self.holder_at(p.scope, &s.by, s.at) {
                continue;
            }
            match s.position.pull {
                Pull::Toward => {
                    if !t.approved.iter().any(|(who, _)| *who == s.by) {
                        t.approved.push((s.by.as_str(), s.at));
                    }
                }
                Pull::Against => {
                    t.against.push((s.at, s.by.as_str()));
                    if !s.position.because.trim().is_empty() {
                        t.objections
                            .push((s.at, s.by.as_str(), s.position.because.as_str()));
                    }
                }
            }
        }
        // By time, so "the n-th approval" means what it says. Stable, so
        // the proposer's own act stays ahead of anything in its second.
        t.approved.sort_by_key(|(_, at)| *at);
        t
    }

    /// One rule, one tally. `from` is when the clock starts for `consent`
    /// and the moment a holder's own write lands under `standing`.
    fn judge_once(&self, rule: &Ratify, p: &Proposal, t: &Tally, from: i64, now: i64) -> Outcome {
        let here = where_(p.scope);
        let objection = |o: Option<(i64, &str, &str)>| {
            o.map(|(at, by, why)| (at, by.to_string(), why.to_string()))
        };
        match rule {
            Ratify::Standing => {
                if self.nobody_holds_before(p.scope, from) {
                    return Outcome {
                        completed: Some((from, format!("nobody holds {here}; it is open"))),
                        refused: None,
                        needs: String::new(),
                    };
                }
                let own = t
                    .approved
                    .first()
                    .filter(|(who, at)| *who == p.actor && *at == p.at);
                let completed = match (own, t.approved.first()) {
                    (Some(_), _) => Some((from, format!("{} holds {here}", p.actor))),
                    (None, Some((who, at))) => {
                        Some((*at, format!("approved by {who}, who holds {here}")))
                    }
                    (None, None) => None,
                };
                Outcome {
                    completed,
                    refused: objection(t.first_objection()),
                    needs: format!(
                        "approval from one person who holds {here}{}",
                        if is_human(p.actor) {
                            ""
                        } else {
                            " — the proposer is not a person"
                        }
                    ),
                }
            }
            Ratify::Joint { holders } => {
                let named = |who: &str| holders.iter().any(|h| h == who);
                let refused = objection(t.objections.iter().find(|(_, by, _)| named(by)).copied());
                let missing: Vec<&str> = holders
                    .iter()
                    .map(String::as_str)
                    .filter(|h| !t.approved.iter().any(|(who, _)| who == h))
                    .collect();
                let completed = if missing.is_empty() {
                    let done = t
                        .approved
                        .iter()
                        .filter(|(who, _)| named(who))
                        .map(|(_, at)| *at)
                        .max()
                        .unwrap_or(from);
                    Some((done, format!("approved jointly by {}", holders.join(", "))))
                } else {
                    None
                };
                Outcome {
                    completed,
                    refused,
                    needs: format!("approval from {}", missing.join(", ")),
                }
            }
            Ratify::Threshold { approve, block } => {
                // Refused the moment the block-th objection landed; the
                // first reasoned one, if any, is the quoted `why`.
                let refused = if *block > 0 && t.against.len() >= *block {
                    let (at, _) = t.against[*block - 1];
                    Some(t.first_objection().map_or(
                        (
                            at,
                            String::new(),
                            format!("{} holder(s) objected", t.against.len()),
                        ),
                        |(_, by, why)| (at, by.to_string(), why.to_string()),
                    ))
                } else {
                    None
                };
                let completed = (t.approved.len() >= *approve).then(|| {
                    let done = if *approve == 0 {
                        from
                    } else {
                        t.approved[*approve - 1].1
                    };
                    (
                        done,
                        format!("{} of {approve} holder approvals", t.approved.len()),
                    )
                });
                Outcome {
                    completed,
                    refused,
                    needs: format!(
                        "{} more approval(s) from people who hold {here}",
                        approve.saturating_sub(t.approved.len())
                    ),
                }
            }
            Ratify::Consent { days } => {
                let due = from + i64::from(*days) * DAY;
                Outcome {
                    completed: (now >= due).then(|| {
                        (
                            due,
                            format!("{days} day(s) passed with no objection from a holder"),
                        )
                    }),
                    refused: objection(t.first_objection()),
                    needs: format!(
                        "no objection from a holder before {}",
                        crate::date::ymd(due)
                    ),
                }
            }
            Ratify::Twice { .. } => unreachable!("twice is judged by judge_twice"),
        }
    }

    fn judge_twice(&self, each: &Ratify, between: &Between, p: &Proposal, now: i64) -> Verdict {
        // First vote: everything said, and the proposer's own act counts.
        let first = self.judge_once(each, p, &self.tally(p, None, true), p.at, now);
        let Some((t1, how1)) = first.completed.clone() else {
            return match settle(first) {
                Verdict::Proposed { needs } => Verdict::Proposed {
                    needs: format!("first of two: {needs}"),
                },
                v => v,
            };
        };
        if let Some((at, by, why)) = first.refused.filter(|(at, _, _)| *at < t1) {
            return Verdict::Refused { at, by, why };
        }
        // The gap. Nothing said before it counts toward the second vote,
        // and an objection anywhere after the first vote still refuses.
        let boundary = match between {
            Between::Days { days } => Some((t1 + i64::from(*days) * DAY, None)),
            Between::Turnover => self
                .turnover_after(p.scope, t1)
                .map(|(b, who)| (b, Some(who))),
        };
        if let Some((at, by, why)) = self.tally(p, Some(t1), false).first_objection() {
            return Verdict::Refused {
                at,
                by: by.to_string(),
                why: why.to_string(),
            };
        }
        // Approvals count from the boundary on. Tallied from there rather
        // than filtered afterwards, so a holder who approved too early and
        // again in time is counted the second time.
        let b = boundary.as_ref().map_or(i64::MAX, |(b, _)| *b);
        let second = self.tally(p, Some(b), false);
        let Some((b, joined)) = boundary else {
            return Verdict::Proposed {
                needs: format!(
                    "someone who did not hold {} on {} to be granted it, then a second approval",
                    where_(p.scope),
                    crate::date::ymd(t1)
                ),
            };
        };
        if now < b {
            return Verdict::Proposed {
                needs: format!("a second approval after {}", crate::date::ymd(b)),
            };
        }
        let outcome = self.judge_once(each, p, &second, b, now);
        let gap = match joined {
            Some(who) => format!("after {who} joined {}", crate::date::ymd(b)),
            None => format!("after {}", crate::date::ymd(b)),
        };
        match settle(outcome) {
            Verdict::Ratified { since, how } => Verdict::Ratified {
                since,
                how: format!("{how1}; and again {gap}: {how}"),
            },
            Verdict::Proposed { needs } => Verdict::Proposed {
                needs: format!("a second time, since {}: {needs}", crate::date::ymd(b)),
            },
            v => v,
        }
    }

    /// Where a proposal stands under a rule you name. What the
    /// counterfactual asks.
    pub fn ratify_under(&self, rule: &Ratify, p: &Proposal, now: i64) -> Verdict {
        match rule {
            Ratify::Twice { each, between } => self.judge_twice(each, between, p, now),
            _ => settle(self.judge_once(rule, p, &self.tally(p, None, true), p.at, now)),
        }
    }

    /// Where a proposal stands under the ratification rule its scope had
    /// when it was written.
    ///
    /// Pure: the proposal, the positions about it, who held standing when,
    /// and the clock. Nothing else.
    pub fn ratify_proposal(&self, p: &Proposal, now: i64) -> Verdict {
        let rule = self.ratification_for_at(p.scope, p.at).clone();
        self.ratify_under(&rule, p, now)
    }

    /// A commitment, as something put to its scope.
    pub fn proposal_of<'a>(&'a self, c: &'a Commitment) -> Proposal<'a> {
        Proposal {
            id: &c.id,
            scope: self.scope_of(&c.id),
            at: c.asserted_at,
            actor: &c.actor,
        }
    }

    /// Where this commitment stands under the ratification rule of its scope.
    pub fn ratify(&self, c: &Commitment, now: i64) -> Verdict {
        self.ratify_proposal(&self.proposal_of(c), now)
    }
}

static SHIPPED: Ratify = Ratify::Standing;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spelling_round_trips() {
        for raw in [
            "standing",
            "joint:human:dana,human:sam",
            "threshold:2/1",
            "consent:7d",
            "twice:turnover:standing",
            "twice:90d:joint:human:dana,human:sam",
            "twice:turnover:consent:14d",
        ] {
            let r = Ratify::parse(raw).expect(raw);
            assert_eq!(r.name(), raw, "{raw}");
        }
        assert!(Ratify::parse("joint:").is_none());
        assert!(Ratify::parse("threshold:2").is_none());
        assert!(Ratify::parse("unanimity").is_none());
        assert!(Ratify::parse("twice:standing").is_none(), "no gap named");
        assert!(
            Ratify::parse("twice:0d:standing").is_none(),
            "no gap at all"
        );
        assert!(
            Ratify::parse("twice:turnover:twice:7d:standing").is_none(),
            "twice over twice is nothing a person could explain"
        );
    }

    #[test]
    fn twice_over_joint_says_it_adds_no_voices() {
        let r = Ratify::parse("twice:30d:joint:human:dana,human:sam").unwrap();
        assert!(r.prose().contains("no new voices"), "{}", r.prose());
        let r = Ratify::parse("twice:turnover:standing").unwrap();
        assert!(!r.prose().contains("no new voices"));
    }
}
