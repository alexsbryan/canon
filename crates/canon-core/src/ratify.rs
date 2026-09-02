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
//! to `supersede`, and to the counterfactual replay. Changing a scope's
//! ratification rule is decided by whoever holds standing over that scope or
//! the one above it — the constitutional level, one step up — which is what
//! stops anyone from quietly lowering the bar for their own corner.
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
//! **What the default is.** [`Ratify::Standing`]: whoever holds the scope may
//! write into it directly, and a scope nobody holds is open. That is exactly
//! the behaviour every canon had before this module existed, now chosen
//! rather than assumed, and a house can raise it with one act.

use serde::{Deserialize, Serialize};

use crate::fold::{Canon, Commitment};
use crate::scope::Scope;
use crate::standing::Pull;

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
}

impl Ratify {
    pub fn name(&self) -> String {
        match self {
            Self::Standing => "standing".into(),
            Self::Joint { holders } => format!("joint:{}", holders.join(",")),
            Self::Threshold { approve, block } => format!("threshold:{approve}/{block}"),
            Self::Consent { days } => format!("consent:{days}d"),
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
        }
    }

    /// The one spelling, shared by the CLI and the seed dialect:
    /// `standing`, `joint:dana,sam`, `threshold:2/1`, `consent:7d`.
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
    pub at: i64,
    pub actor: String,
    pub act: crate::id::ActId,
}

/// Where a proposal stands under its scope's rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// A rule. `how` says which rule made it one and on whose word.
    Ratified { how: String },
    /// Not yet. `needs` says what would make it one.
    Proposed { needs: String },
    /// Refused by the people the rule names. `why` quotes the objection.
    Refused { at: i64, by: String, why: String },
}

fn is_human(actor: &str) -> bool {
    actor.starts_with("human:")
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
    pub fn ratification_for_at(&self, scope: Option<&Scope>, at: i64) -> &Ratify {
        static SHIPPED: Ratify = Ratify::Standing;
        let mut best: Option<&AdoptedRatify> = None;
        for r in self.ratifications.iter().filter(|r| r.at <= at) {
            let applies = match (&r.scope, scope) {
                (None, _) => true,
                (Some(s), Some(target)) => s.covers(target),
                (Some(_), None) => false,
            };
            if !applies {
                continue;
            }
            let depth = r.scope.as_ref().map_or(0, Scope::depth);
            // Deepest wins; among equals, the latest adopted.
            let beats = best.is_none_or(|b| {
                let bd = b.scope.as_ref().map_or(0, Scope::depth);
                depth > bd || (depth == bd && r.at >= b.at)
            });
            if beats {
                best = Some(r);
            }
        }
        best.map_or(&SHIPPED, |r| &r.rule)
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
                covering.iter().any(|g| g.actor == actor)
                    || (covering.is_empty() && holds_any())
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

    /// Where this commitment stands under the ratification rule of its scope.
    ///
    /// Pure: the commitment, the positions about it, who held standing when,
    /// and the clock. Nothing else.
    pub fn ratify(&self, c: &Commitment, now: i64) -> Verdict {
        let scope = self.scope_of(&c.id).cloned();
        let rule = self.ratification_for_at(scope.as_ref(), c.asserted_at).clone();
        // The people who ratify a scope's rules are the ones who hold it at
        // the NARROWEST level anyone does — the kitchen's holders for a
        // kitchen rule, even though the whole house covers the kitchen.
        // That is subsidiarity, and it is the same reading `check` gives
        // under it: wider standing asks, it does not act. For an unscoped
        // rule the narrowest level is the top one.
        let holder_at = |actor: &str, at: i64| {
            let covering: Vec<&crate::scope::Grant> = self
                .grants
                .iter()
                .filter(|g| g.held_at(at) && scope.as_ref().is_none_or(|s| g.scope.covers(s)))
                .collect();
            let deepest = match &scope {
                Some(_) => covering.iter().map(|g| g.scope.depth()).max(),
                None => Some(1),
            };
            covering
                .iter()
                .any(|g| g.actor == actor && Some(g.scope.depth()) == deepest)
        };
        // Held BEFORE the proposal was written. A house whose first rules and
        // first grants were written in the same sitting has not locked its
        // founders out of their own charter; see `may_govern`.
        let nobody_holds = match &scope {
            Some(s) => !self
                .grants
                .iter()
                .any(|g| g.granted_at < c.asserted_at && g.held_at(c.asserted_at) && g.scope.covers(s)),
            None => !self
                .grants
                .iter()
                .any(|g| g.granted_at < c.asserted_at && g.held_at(c.asserted_at)),
        };
        let where_ = scope.as_ref().map_or_else(|| "this canon".to_string(), ToString::to_string);

        // What people said about it. Only human holders count; the record
        // keeps everyone's word, ratification counts the people the rule
        // names. The proposer's own act is their approval.
        let mut approved: Vec<&str> = Vec::new();
        let mut objection: Option<(i64, &str, &str)> = None;
        let proposer_human = is_human(&c.actor);
        if proposer_human && holder_at(&c.actor, c.asserted_at) {
            approved.push(c.actor.as_str());
        }
        for p in self.positions.iter().filter(|p| p.about == c.id.as_str()) {
            if !is_human(&p.by) || !holder_at(&p.by, p.at) {
                continue;
            }
            match p.position.pull {
                Pull::Toward => {
                    if !approved.contains(&p.by.as_str()) {
                        approved.push(p.by.as_str());
                    }
                }
                Pull::Against if !p.position.because.trim().is_empty() => {
                    if objection.is_none_or(|(at, _, _)| p.at < at) {
                        objection = Some((p.at, p.by.as_str(), p.position.because.as_str()));
                    }
                }
                Pull::Against => {}
            }
        }
        let refused = |(at, by, why): (i64, &str, &str)| Verdict::Refused {
            at,
            by: by.to_string(),
            why: why.to_string(),
        };

        match rule {
            Ratify::Standing => {
                if nobody_holds {
                    return Verdict::Ratified {
                        how: format!("nobody holds {where_}; it is open"),
                    };
                }
                if proposer_human && holder_at(&c.actor, c.asserted_at) {
                    return Verdict::Ratified {
                        how: format!("{} holds {where_}", c.actor),
                    };
                }
                if let Some(o) = objection {
                    return refused(o);
                }
                match approved.first() {
                    Some(who) => Verdict::Ratified {
                        how: format!("approved by {who}, who holds {where_}"),
                    },
                    None => Verdict::Proposed {
                        needs: format!(
                            "approval from one person who holds {where_}{}",
                            if proposer_human { "" } else { " — the proposer is not a person" }
                        ),
                    },
                }
            }
            Ratify::Joint { holders } => {
                if let Some(o) = objection.filter(|(_, by, _)| holders.iter().any(|h| h == by)) {
                    return refused(o);
                }
                let missing: Vec<&String> = holders
                    .iter()
                    .filter(|h| !approved.contains(&h.as_str()))
                    .collect();
                if missing.is_empty() {
                    Verdict::Ratified {
                        how: format!("approved jointly by {}", holders.join(", ")),
                    }
                } else {
                    Verdict::Proposed {
                        needs: format!(
                            "approval from {}",
                            missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
                        ),
                    }
                }
            }
            Ratify::Threshold { approve, block } => {
                let against = self
                    .positions
                    .iter()
                    .filter(|p| {
                        p.about == c.id.as_str()
                            && p.position.pull == Pull::Against
                            && is_human(&p.by)
                            && holder_at(&p.by, p.at)
                    })
                    .count();
                if block > 0 && against >= block {
                    return objection.map_or(
                        Verdict::Refused {
                            at: now,
                            by: String::new(),
                            why: format!("{against} holder(s) objected"),
                        },
                        refused,
                    );
                }
                if approved.len() >= approve {
                    Verdict::Ratified {
                        how: format!("{} of {approve} holder approvals", approved.len()),
                    }
                } else {
                    Verdict::Proposed {
                        needs: format!(
                            "{} more approval(s) from people who hold {where_}",
                            approve - approved.len()
                        ),
                    }
                }
            }
            Ratify::Consent { days } => {
                if let Some(o) = objection {
                    return refused(o);
                }
                let due = c.asserted_at + i64::from(days) * 86_400;
                if now >= due {
                    Verdict::Ratified {
                        how: format!("{days} day(s) passed with no objection from a holder"),
                    }
                } else {
                    Verdict::Proposed {
                        needs: format!(
                            "no objection from a holder before {}",
                            crate::date::ymd(due)
                        ),
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spelling_round_trips() {
        for raw in ["standing", "joint:human:dana,human:sam", "threshold:2/1", "consent:7d"] {
            let r = Ratify::parse(raw).expect(raw);
            assert_eq!(r.name(), raw, "{raw}");
        }
        assert!(Ratify::parse("joint:").is_none());
        assert!(Ratify::parse("threshold:2").is_none());
        assert!(Ratify::parse("unanimity").is_none());
    }
}
