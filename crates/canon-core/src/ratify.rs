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
//! every canon did before; under `joint`, `threshold`, `majority`,
//! `unanimous`, `share`, `consent` or `twice` it waits for the same people
//! the old rule names. The amendment clause
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
//! write into it directly, and a scope nobody holds is open to people. That
//! is the behaviour every canon had before this module existed, now chosen
//! rather than assumed, and a house can raise it with one act. The one
//! exception is an agent's write into a scope nobody holds: it waits for a
//! person's approval, because the open scope is there so founders are not
//! locked out of their own charter, not so a machine's first write is law.

use alloc::{
    boxed::Box,
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};

use serde::{Deserialize, Serialize};

use crate::fold::{Canon, Commitment};
use crate::id::ActId;
use crate::person::Person;
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

/// The share of the people who held a scope that a share rule needs: at
/// least `numerator/denominator` of them, or more than that.
///
/// **A fraction of people, kept exact.** Two-thirds of a house of three is
/// two; 67% of it would be three. So the bar is a fraction, and what it needs
/// of `n` people is integer arithmetic ([`Bar::needed`]).
///
/// **Every bar is one a house can reach, and none is reached by nobody.**
/// The denominator is not 0, the share is not more than everyone, and "at
/// least none of them" is not a bar. [`Bar::new`] refuses the rest, and so
/// does reading one from a record, so judging a proposal never divides by
/// zero and never waits on more people than there are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "BarFields")]
pub struct Bar {
    numerator: u32,
    denominator: u32,
    more_than: bool,
}

/// A bar as a record spells it, before it is known to be one.
#[derive(Deserialize)]
struct BarFields {
    numerator: u32,
    denominator: u32,
    more_than: bool,
}

impl TryFrom<BarFields> for Bar {
    type Error = BadBar;
    fn try_from(f: BarFields) -> Result<Self, BadBar> {
        Self::checked(f.numerator, f.denominator, f.more_than)
    }
}

/// Why a share is not one a house could use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadBar {
    /// Out of 0: there is no whole to take a share of.
    OutOfNothing,
    /// At least none of them: it would carry before anyone approved.
    Nobody,
    /// More than everyone: nothing could ever carry.
    MoreThanEveryone,
}

impl core::fmt::Display for BadBar {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match self {
            Self::OutOfNothing => {
                "a share out of 0 is no share: the number after the slash can't be 0"
            }
            Self::Nobody => "a share of none of them would make a rule before anyone approved it",
            Self::MoreThanEveryone => {
                "that is more than everyone who holds the scope, so nothing could ever carry"
            }
        })
    }
}

/// Why a spelling is not a ratification rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotARule {
    /// No rule is spelled this way.
    Unknown,
    /// `share:` followed by neither a whole percent nor a fraction.
    Share,
    /// A share no house could use.
    Bar(BadBar),
}

impl core::fmt::Display for NotARule {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Unknown => f.write_str("not a ratification rule"),
            // A shell reads `>` as a redirect, so an unquoted `share:>1/2`
            // arrives as `share:` and lands here: say how to type it.
            Self::Share => f.write_str(
                "a share is a whole percent or a fraction of the people who hold the scope: \
                 share:60%, share:2/3, or 'share:>1/2' for more than half (quoted, so the \
                 shell leaves the > alone)",
            ),
            Self::Bar(bad) => bad.fmt(f),
        }
    }
}

impl Bar {
    /// More than half: what `majority` takes.
    pub const MAJORITY: Self = Self {
        numerator: 1,
        denominator: 2,
        more_than: true,
    };
    /// All of them: what `unanimous` takes.
    pub const EVERYONE: Self = Self {
        numerator: 1,
        denominator: 1,
        more_than: false,
    };

    /// At least `numerator/denominator` of the people, or more than that,
    /// in lowest terms. Refused if no house could use it.
    pub fn new(numerator: u32, denominator: u32, more_than: bool) -> Result<Self, BadBar> {
        let (numerator, denominator) = Self::checked(numerator, denominator, more_than)?.lowest();
        Ok(Self {
            numerator,
            denominator,
            more_than,
        })
    }

    /// Refused if no house could use it, and otherwise kept as written. A
    /// bar read from a record keeps the record's numbers, so the act renders
    /// to the bytes it was signed as; 6/10 and 3/5 take the same people.
    fn checked(numerator: u32, denominator: u32, more_than: bool) -> Result<Self, BadBar> {
        if denominator == 0 {
            return Err(BadBar::OutOfNothing);
        }
        if numerator > denominator || (more_than && numerator == denominator) {
            return Err(BadBar::MoreThanEveryone);
        }
        if numerator == 0 && !more_than {
            return Err(BadBar::Nobody);
        }
        Ok(Self {
            numerator,
            denominator,
            more_than,
        })
    }

    pub fn numerator(&self) -> u32 {
        self.numerator
    }

    pub fn denominator(&self) -> u32 {
        self.denominator
    }

    /// More than the fraction, rather than at least it.
    pub fn more_than(&self) -> bool {
        self.more_than
    }

    /// How many of `n` people it takes: ⌈a·n/b⌉ for at least a/b, and
    /// ⌊a·n/b⌋ + 1 for more than a/b. In whole numbers, so there is no
    /// rounding to argue over, and widened so no size of house overflows it.
    /// For any house of at least one it is between 1 and `n`, because
    /// [`Bar::checked`] refused every bar that is not.
    pub fn needed(&self, n: usize) -> usize {
        let (a, b, n) = (
            u128::from(self.numerator),
            u128::from(self.denominator),
            n as u128,
        );
        let needed = if self.more_than {
            a * n / b + 1
        } else {
            (a * n).div_ceil(b)
        };
        needed as usize
    }

    /// Whether it takes everyone: the one bar under which a single person
    /// decides, whatever the size of the house.
    pub fn is_everyone(&self) -> bool {
        !self.more_than && self.numerator == self.denominator
    }

    fn is_majority(&self) -> bool {
        self.more_than && self.lowest() == (1, 2)
    }

    fn lowest(&self) -> (u32, u32) {
        let (mut a, mut b) = (self.numerator, self.denominator);
        while b != 0 {
            (a, b) = (b, a % b);
        }
        // `a` is now the greatest common divisor, at least 1 because the
        // denominator is not 0.
        (self.numerator / a, self.denominator / a)
    }

    /// The share as a person writes it: a percent when it is a whole one —
    /// its denominator divides 100 — and otherwise the fraction, so 3/5 is
    /// `60%` and 2/3 stays `2/3`.
    fn amount(&self) -> String {
        let (a, b) = self.lowest();
        if 100 % b == 0 {
            format!("{}%", a * (100 / b))
        } else {
            format!("{a}/{b}")
        }
    }

    /// What follows `share:` in the spelling.
    fn spelling(&self) -> String {
        let more = if self.more_than { ">" } else { "" };
        format!("{more}{}", self.amount())
    }

    /// What it takes, in a sentence: "more than half", "60%", "more than
    /// 2/3", "everyone".
    pub fn phrase(&self) -> String {
        if self.is_majority() {
            "more than half".into()
        } else if self.is_everyone() {
            "everyone".into()
        } else if self.more_than {
            format!("more than {}", self.amount())
        } else {
            self.amount()
        }
    }

    /// `60%`, `2/3`, `>50%`, `>1/2`: the part after `share:`.
    fn parse(raw: &str) -> Result<Self, NotARule> {
        let raw = raw.trim();
        let (more_than, amount) = match raw.strip_prefix('>') {
            Some(rest) => (true, rest.trim()),
            None => (false, raw),
        };
        let whole = |s: &str| s.trim().parse::<u32>().map_err(|_| NotARule::Share);
        let (numerator, denominator) = match amount.strip_suffix('%') {
            Some(percent) => (whole(percent)?, 100),
            None => {
                let (a, b) = amount.split_once('/').ok_or(NotARule::Share)?;
                (whole(a)?, whole(b)?)
            }
        };
        Self::new(numerator, denominator, more_than).map_err(NotARule::Bar)
    }
}

/// How a proposal in a scope becomes a rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Ratify {
    /// A holder of the scope may write a rule directly. A non-holder's
    /// proposal takes one holder's approval. A scope nobody holds is open to
    /// people; an agent's write there takes any person's approval.
    Standing,
    /// Every one of these named people must approve. One of them objecting
    /// refuses it.
    Joint { holders: Vec<String> },
    /// This many holders approving carries it; this many objecting refuses
    /// it. The proposer counts as approving when they hold the scope.
    Threshold { approve: usize, block: usize },
    /// More than half of the people who hold the scope when it is proposed
    /// approving carries it. Counted against the house as it stood then, so
    /// nobody who joins later moves the count, and a count of the house on
    /// the day the rule was chosen cannot outlive the house: once more than
    /// half can no longer approve — they said no, or no longer hold the
    /// scope — it is refused, and can be proposed again to the house as it
    /// is.
    Majority,
    /// Every person who holds the scope when it is proposed approving
    /// carries it. One of them objecting with a reason refuses it, and so
    /// does one of them no longer holding the scope before they approved:
    /// everyone can no longer agree, so it is refused rather than waiting
    /// for someone who has gone.
    Unanimous,
    /// At least, or more than, a share of the people who hold the scope when
    /// it is proposed approving carries it: `share:60%`, `share:2/3`. Counted
    /// and refused as `majority` is. More than half is spelled `majority`
    /// and all of them `unanimous`, so a record names those two the way
    /// 0.4.0 reads them, and one bar has one spelling.
    Share(Bar),
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
            Self::Majority => "majority".into(),
            Self::Unanimous => "unanimous".into(),
            Self::Share(bar) => match Self::of_bar(*bar) {
                Self::Share(bar) => format!("share:{}", bar.spelling()),
                same => same.name(),
            },
            Self::Consent { days } => format!("consent:{days}d"),
            Self::Twice { each, between } => format!("twice:{}:{}", between.name(), each.name()),
        }
    }

    /// The share of the house a share rule takes; `None` for a rule that
    /// counts something else. The one place `majority` and `unanimous` say
    /// which share they are, so all three are judged by one count.
    pub fn bar(&self) -> Option<Bar> {
        match self {
            Self::Majority => Some(Bar::MAJORITY),
            Self::Unanimous => Some(Bar::EVERYONE),
            Self::Share(bar) => Some(*bar),
            Self::Standing
            | Self::Joint { .. }
            | Self::Threshold { .. }
            | Self::Consent { .. }
            | Self::Twice { .. } => None,
        }
    }

    /// A bar as the rule it is: more than half is `majority` and all of them
    /// is `unanimous`, whichever way it was spelled.
    fn of_bar(bar: Bar) -> Self {
        if bar.is_majority() {
            Self::Majority
        } else if bar.is_everyone() {
            Self::Unanimous
        } else {
            Self::Share(bar)
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
            Self::Majority => "A rule here takes more than half of the people who hold this \
                               scope when it is proposed. It is refused once that can no longer \
                               happen."
                .into(),
            Self::Unanimous => "A rule here takes everyone who holds this scope when it is \
                                proposed. One of them objecting, or no longer holding it before \
                                they approve, refuses it."
                .into(),
            Self::Share(bar) => match Self::of_bar(*bar) {
                Self::Share(bar) => format!(
                    "A rule here takes {} {} of the people who hold this scope when it is \
                     proposed. It is refused once that can no longer happen.",
                    if bar.more_than {
                        "more than"
                    } else {
                        "at least"
                    },
                    bar.amount()
                ),
                same => same.prose(),
            },
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

    /// A rule from its spelling, or `None`. [`Ratify::from_str`] says why
    /// not.
    pub fn parse(raw: &str) -> Option<Self> {
        raw.parse().ok()
    }
}

impl core::str::FromStr for Ratify {
    type Err = NotARule;

    /// The one spelling, shared by the CLI and the seed dialect:
    /// `standing`, `joint:dana,sam`, `threshold:2/1`, `majority`,
    /// `unanimous`, `share:60%`, `share:2/3`, `share:>2/3`, `consent:7d`,
    /// `twice:turnover:consent:7d`, `twice:90d:threshold:2/1`.
    ///
    /// `threshold:2/1` and `share:2/3` both have a slash and mean different
    /// things: two people approving and one objecting, against two-thirds
    /// of the people who hold the scope.
    fn from_str(raw: &str) -> Result<Self, NotARule> {
        use NotARule::Unknown;
        let raw = raw.trim();
        match raw {
            "standing" => return Ok(Self::Standing),
            "majority" => return Ok(Self::Majority),
            "unanimous" => return Ok(Self::Unanimous),
            _ => {}
        }
        if let Some(rest) = raw.strip_prefix("joint:") {
            let holders: Vec<String> = rest
                .split(',')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect();
            return if holders.is_empty() {
                Err(Unknown)
            } else {
                Ok(Self::Joint { holders })
            };
        }
        if let Some(rest) = raw.strip_prefix("threshold:") {
            let (a, b) = rest.split_once('/').ok_or(Unknown)?;
            return Ok(Self::Threshold {
                approve: a.trim().parse().map_err(|_| Unknown)?,
                block: b.trim().parse().map_err(|_| Unknown)?,
            });
        }
        if let Some(rest) = raw.strip_prefix("share:") {
            // One bar, one spelling: more than half and all of them come
            // back as the rules 0.4.0 already reads.
            return Bar::parse(rest).map(Self::of_bar);
        }
        if let Some(rest) = raw.strip_prefix("consent:") {
            let days = rest.trim().strip_suffix('d').unwrap_or(rest.trim());
            return Ok(Self::Consent {
                days: days.parse().map_err(|_| Unknown)?,
            });
        }
        if let Some(rest) = raw.strip_prefix("twice:") {
            // The tail is the inner rule, so nested colons are unambiguous.
            let (gap, inner) = rest.split_once(':').ok_or(Unknown)?;
            let between = match gap.trim() {
                "turnover" => Between::Turnover,
                d => {
                    let days: u32 = d
                        .strip_suffix('d')
                        .and_then(|d| d.parse().ok())
                        .ok_or(Unknown)?;
                    if days == 0 {
                        return Err(Unknown);
                    }
                    Between::Days { days }
                }
            };
            let each: Self = inner.parse()?;
            // Twice over twice reads as nothing a person could explain.
            if matches!(each, Self::Twice { .. }) {
                return Err(Unknown);
            }
            return Ok(Self::Twice {
                each: Box::new(each),
                between,
            });
        }
        Err(Unknown)
    }
}

/// What a scope's ratification rule reaches. See the `governs` field on the
/// `ratification` act.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Governs {
    /// Commitments wait for the rule; a policy applies on its writer's
    /// standing. What every canon did before v4.
    Commitments,
    /// A change of policy waits for the rule too. Without it, one holder in a
    /// house that decides by consent can make it decide by anything.
    All,
}

/// The person a personal scope belongs to: the first segment of the path,
/// when it is a human actor id. `human:phylla.glasses` is Phylla's.
///
/// **The name is the owner.** Identity from essence: nothing has to be
/// granted, recorded or remembered for the house to know whose it is, and a
/// scope named for an agent is never anyone's own — agents draft, they do
/// not hold a corner.
pub fn personal_owner(scope: &Scope) -> Option<&str> {
    let root = scope.as_str().split('.').next()?;
    (root.len() > crate::person::PREFIX.len() && crate::Person::of(root).is_some()).then_some(root)
}

/// When personal scopes were honoured, as the canon-wide ratification rule
/// said. Computed from a first fold and handed to a second, because the
/// knob is itself a proposal: it is known only once ratification has run,
/// and standing is needed before that.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Personal {
    /// The log carries a ratification act that turns the knob on. While it
    /// does, grants over personal roots never count toward the canon as a
    /// whole — in either fold — which is what makes one refold enough: no
    /// canon-wide verdict can depend on the knob's own effects.
    pub(crate) declared: bool,
    /// `[since, until)` and the act whose ratification switched it on.
    pub(crate) on: Vec<(i64, Option<i64>, ActId)>,
}

impl Personal {
    /// The intervals in which the canon-wide rule in force honoured personal
    /// scopes, read off the ratified canon-wide rules in the order they took
    /// effect — the same order [`Canon::adopted_at`] resolves.
    pub(crate) fn from_ratified(declared: bool, ratifications: &[AdoptedRatify]) -> Self {
        let mut wide: Vec<(i64, i64, bool, &ActId)> = ratifications
            .iter()
            .filter(|r| r.scope.is_none())
            .filter_map(|r| match r.verdict {
                Verdict::Ratified { since, .. } => {
                    Some((since, r.at, r.personal == Some(true), &r.act))
                }
                _ => None,
            })
            .collect();
        wide.sort_by_key(|(since, at, _, _)| (*since, *at));
        let mut on: Vec<(i64, Option<i64>, ActId)> = Vec::new();
        for (since, _, honoured, act) in wide {
            let open = on.last().is_some_and(|(_, until, _)| until.is_none());
            match (open, honoured) {
                (false, true) => on.push((since, None, act.clone())),
                (true, false) => {
                    if let Some(last) = on.last_mut() {
                        last.1 = Some(since);
                    }
                }
                _ => {}
            }
        }
        Self { declared, on }
    }

    /// The act that had personal scopes honoured at `at`, if any did.
    pub(crate) fn at(&self, at: i64) -> Option<&ActId> {
        self.on
            .iter()
            .find(|(since, until, _)| *since <= at && until.is_none_or(|u| at < u))
            .map(|(_, _, act)| act)
    }
}

/// A ratification rule someone adopted for a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// What the rule reaches, when the act said. See [`Governs`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub governs: Option<Governs>,
    /// Canon-wide: whether personal scopes are honoured, when the act said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal: Option<bool>,
}

/// Where a proposal stands under its scope's rule.
///
/// Only ratification makes one: each variant is `non_exhaustive`, so
/// outside this crate a verdict can be matched and never built.
///
/// ```compile_fail,E0639
/// let forged = canon_core::Verdict::Ratified { since: 0, how: String::new() };
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum Verdict {
    /// A rule, from `since` on. `how` says which rule made it one and on
    /// whose word.
    #[non_exhaustive]
    Ratified { since: i64, how: String },
    /// Not yet. `needs` says what would make it one.
    #[non_exhaustive]
    Proposed { needs: String },
    /// Refused by the people the rule names. `why` quotes the objection.
    #[non_exhaustive]
    Refused { at: i64, by: String, why: String },
}

/// Where an actor stands toward governing a scope at a moment — the typed
/// answer behind [`Canon::may_govern`].
///
/// Two of the three say yes, and the difference between them is the one a
/// founder needs to see. `Held` is standing granted before the act. `Open`
/// is the bootstrap: no grant covering the scope predates the act, so the
/// act took because nothing yet stood in its way — and the next second, once
/// the grants beside it count, the same act would be a proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seat {
    /// Holds standing covering the scope, granted before the moment.
    Held,
    /// No grant covering the scope predates the moment: it was open.
    Open,
    /// Somebody else holds it, and this actor does not.
    Lacking,
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
    Person::of(actor).is_some()
}

/// The counted word of the people a rule names, in time order. Every entry
/// is a [`Person`], so nothing but a person's word can be counted here.
#[derive(Default)]
struct Tally<'a> {
    /// Who approved and when they first did. The proposer's own act, when it
    /// counts, is the earliest entry.
    approved: Vec<(Person<'a>, i64)>,
    /// Reasoned objections: `(at, by, why)`.
    objections: Vec<(i64, Person<'a>, &'a str)>,
    /// Every position against, reasoned or not, by time. Threshold's block
    /// counts these.
    against: Vec<(i64, Person<'a>)>,
}

impl<'a> Tally<'a> {
    fn first_objection(&self) -> Option<(i64, Person<'a>, &'a str)> {
        self.objections.first().copied()
    }

    /// A position against: always counted against, and an objection only
    /// when it gives a reason.
    fn object(&mut self, at: i64, who: Person<'a>, why: &'a str) {
        self.against.push((at, who));
        if !why.trim().is_empty() {
            self.objections.push((at, who, why));
        }
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

pub(crate) fn where_(scope: Option<&Scope>) -> String {
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

    /// Whether the rule deciding a scope at a moment reaches policy too.
    /// Absent on that rule is [`Governs::Commitments`], what every canon did
    /// before v4.
    pub fn governs_at(&self, scope: Option<&Scope>, at: i64) -> Governs {
        self.adopted_at(scope, at)
            .and_then(|r| r.governs)
            .unwrap_or(Governs::Commitments)
    }

    /// The adopted rule deciding a scope at a moment, if any was.
    pub fn adopted_at(&self, scope: Option<&Scope>, at: i64) -> Option<&AdoptedRatify> {
        // Inside a person's own scope only a rule they set there applies.
        // The house's rule does not reach in, so with none of their own
        // their write is in force on arrival and anyone else's waits for
        // them — which is `standing` with the owner as the one holder.
        let own_root = scope
            .filter(|s| self.owned_at(s, at).is_some())
            .and_then(|s| personal_owner(s));
        let mut best: Option<(&AdoptedRatify, i64)> = None;
        for r in &self.ratifications {
            if let Some(root) = own_root {
                let inside = r
                    .scope
                    .as_ref()
                    .is_some_and(|rs| personal_owner(rs) == Some(root));
                if !inside {
                    continue;
                }
            }
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

    /// The owner of `scope`, when it is a personal scope and personal scopes
    /// were honoured at `at`.
    pub fn owned_at<'s>(&self, scope: &'s Scope, at: i64) -> Option<&'s str> {
        self.personal.at(at)?;
        personal_owner(scope)
    }

    /// Does this grant count toward standing over `scope` at `at`?
    ///
    /// **The one place personal scopes change who holds what**, so every
    /// reading of standing — seat, holders, who decides — gives the same
    /// answer. Inside an honoured personal scope only the owner's grants
    /// count, which includes the one the knob confers. Toward the canon as a
    /// whole, a grant over anybody's personal root never counts once the
    /// knob is declared: somebody's own corner is not a seat in the house.
    pub fn counts(&self, g: &crate::scope::Grant, scope: Option<&Scope>, at: i64) -> bool {
        if let Some(owner) = scope.and_then(|s| self.owned_at(s, at)) {
            return personal_owner(&g.scope) == Some(owner) && g.actor == owner;
        }
        !(self.personal.declared && personal_owner(&g.scope).is_some() && scope.is_none())
    }

    /// May this actor change how a scope is governed — grant standing over
    /// it, set its policy, set its ratification rule? The yes-or-no reading
    /// of [`Canon::seat`].
    pub fn may_govern(&self, actor: &str, scope: Option<&Scope>, at: i64) -> bool {
        self.seat(actor, scope, at) != Seat::Lacking
    }

    /// Where this actor stands toward governing a scope at a moment.
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
    ///
    /// The same rule makes a founding script's grant and the act right after
    /// it simultaneous, so the act takes under [`Seat::Open`] rather than
    /// under the grant — which is correct and was invisible: a bool cannot
    /// say *why* it was true. The fold records every act that took while
    /// open in [`Canon::bootstrap`], and the CLI says so on the line that
    /// reports the act.
    pub fn seat(&self, actor: &str, scope: Option<&Scope>, at: i64) -> Seat {
        // A person's own scope is theirs, whatever anyone holds above it,
        // and it is never open: an unheld corner of somebody else's is not a
        // corner for whoever asks first.
        if let Some(owner) = scope.and_then(|s| self.owned_at(s, at)) {
            return if actor == owner {
                Seat::Held
            } else {
                Seat::Lacking
            };
        }
        let prior: Vec<&crate::scope::Grant> = self
            .grants
            .iter()
            .filter(|g| g.granted_at < at && g.held_at(at) && self.counts(g, scope, at))
            .collect();
        if self
            .grants
            .iter()
            .filter(|g| self.counts(g, None, at))
            .all(|g| g.granted_at >= at)
        {
            return Seat::Open;
        }
        let holds_any = || prior.iter().any(|g| g.actor == actor);
        // A scope nobody holds is open to anyone the canon has seated at all.
        // A stranger to the canon does not get a corner of it for free.
        let open_to = |in_house: bool| if in_house { Seat::Open } else { Seat::Lacking };
        let held_by = |holders: &[&&crate::scope::Grant]| {
            if holders.iter().any(|g| g.actor == actor) {
                Seat::Held
            } else {
                Seat::Lacking
            }
        };
        match scope {
            Some(s) => {
                let covering: Vec<&&crate::scope::Grant> =
                    prior.iter().filter(|g| g.scope.covers(s)).collect();
                if covering.is_empty() {
                    open_to(holds_any())
                } else {
                    held_by(&covering)
                }
            }
            // The whole canon: anyone holding a top-level scope, or anyone
            // at all if no top-level scope has been granted.
            None => {
                let top: Vec<&&crate::scope::Grant> =
                    prior.iter().filter(|g| g.scope.depth() == 1).collect();
                if top.is_empty() {
                    open_to(holds_any())
                } else {
                    held_by(&top)
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
            .filter(|g| {
                g.held_at(at)
                    && scope.is_none_or(|s| g.scope.covers(s))
                    && self.counts(g, scope, at)
            })
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

    /// The scopes that hold the whole canon at a moment: the widest level
    /// anyone holds, which is who [`Canon::holders_at`] counts for an
    /// unscoped proposal. The canon has no scope of its own to grant.
    pub fn canon_scopes(&self, at: i64) -> Vec<&Scope> {
        let held = || {
            self.grants
                .iter()
                .filter(|g| g.held_at(at) && self.counts(g, None, at))
        };
        let Some(level) = held().map(|g| g.scope.depth()).min() else {
            return Vec::new();
        };
        let mut scopes: Vec<&Scope> = held()
            .filter(|g| g.scope.depth() == level)
            .map(|g| &g.scope)
            .collect();
        scopes.sort();
        scopes.dedup();
        scopes
    }

    /// What holding a scope means, said so the reader can act on it. A
    /// scope is its own name; "this canon" is not a scope anyone can be
    /// granted, so it is said as the scopes that hold it.
    fn held_as(&self, scope: Option<&Scope>, at: i64) -> String {
        if scope.is_some() {
            return where_(scope);
        }
        let scopes: Vec<String> = self
            .canon_scopes(at)
            .iter()
            .map(|s| s.to_string())
            .collect();
        match scopes.split_last() {
            None => where_(None),
            Some((only, [])) => format!("this canon by holding {only}"),
            Some((last, rest)) => format!("this canon by holding {} or {last}", rest.join(", ")),
        }
    }

    /// What the people a share counts held, by name: a scope is its own
    /// name, and the whole canon is the scopes that held it at `at` — "the
    /// 30 people who held house", where "who held this canon" names nothing
    /// anyone was granted.
    fn held_then(&self, scope: Option<&Scope>, at: i64) -> String {
        if scope.is_some() {
            return where_(scope);
        }
        let scopes: Vec<String> = self
            .canon_scopes(at)
            .iter()
            .map(|s| s.to_string())
            .collect();
        match scopes.split_last() {
            None => where_(None),
            Some((only, [])) => only.clone(),
            Some((last, rest)) => format!("{} or {last}", rest.join(", ")),
        }
    }

    fn holder_at(&self, scope: Option<&Scope>, actor: &str, at: i64) -> bool {
        self.holding(scope, actor, at).is_some()
    }

    /// The person, if they hold the scope at its narrowest level at `at`.
    fn holding<'a>(&self, scope: Option<&Scope>, actor: &'a str, at: i64) -> Option<Person<'a>> {
        Person::of(actor).filter(|_| self.holders_at(scope, at).contains(actor))
    }

    /// Held BEFORE the moment. A house whose first rules and first grants
    /// were written in the same sitting has not locked its founders out of
    /// their own charter; see [`Canon::may_govern`].
    fn nobody_holds_before(&self, scope: Option<&Scope>, at: i64) -> bool {
        !self.grants.iter().any(|g| {
            g.granted_at < at
                && g.held_at(at)
                && scope.is_none_or(|s| g.scope.covers(s))
                && self.counts(g, scope, at)
        })
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
                    && self.counts(g, scope, g.granted_at)
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
        let mut t = Tally::default();
        if let Some(who) = self.holding(p.scope, p.actor, p.at).filter(|_| implicit) {
            t.approved.push((who, p.at));
        }
        let mut said: Vec<&crate::fold::Stated> = self
            .positions
            .iter()
            .filter(|s| s.about == p.id.as_str() && after.is_none_or(|a| s.at > a))
            .collect();
        said.sort_by_key(|s| s.at);
        for s in said {
            let Some(who) = self.holding(p.scope, &s.by, s.at) else {
                continue;
            };
            match s.position.pull {
                Pull::Toward => {
                    if !t.approved.iter().any(|(w, _)| *w == who) {
                        t.approved.push((who, s.at));
                    }
                }
                Pull::Against => t.object(s.at, who, &s.position.because),
            }
        }
        // By time, so "the n-th approval" means what it says. Stable, so
        // the proposer's own act stays ahead of anything in its second.
        t.approved.sort_by_key(|(_, at)| *at);
        t
    }

    /// What any person said about a proposal, at or after `from`. What
    /// decides an agent's proposal in a scope nobody holds, where there are
    /// no holders to count.
    fn people_said<'a>(&'a self, p: &Proposal, from: i64) -> Tally<'a> {
        let mut said: Vec<(Person<'a>, &crate::fold::Stated)> = self
            .people_on(p.id.as_str())
            .filter(|(_, s)| s.at >= from)
            .collect();
        said.sort_by_key(|(_, s)| s.at);
        let mut t = Tally::default();
        for (who, s) in said {
            match s.position.pull {
                Pull::Toward => t.approved.push((who, s.at)),
                Pull::Against => t.object(s.at, who, &s.position.because),
            }
        }
        t
    }

    /// One rule, one tally. `from` is when the clock starts for `consent`
    /// and the moment a holder's own write lands under `standing`.
    fn judge_once(&self, rule: &Ratify, p: &Proposal, t: &Tally, from: i64, now: i64) -> Outcome {
        if let Some(bar) = rule.bar() {
            return self.judge_share(bar, p, t, from, now);
        }
        let here = where_(p.scope);
        let objection = |o: Option<(i64, Person, &str)>| {
            o.map(|(at, by, why)| (at, by.to_string(), why.to_string()))
        };
        match rule {
            Ratify::Standing => {
                // **Agents never mint, open or not.** An open scope lets a
                // founder write without being locked out of their own
                // charter; it is not a reason for a machine's write to be
                // law before any person has read it. With nobody holding the
                // scope there is nobody narrower to ask, so any person's
                // word decides.
                if !is_human(p.actor) && self.nobody_holds_before(p.scope, from) {
                    let people = self.people_said(p, from);
                    return Outcome {
                        completed: people.approved.first().map(|(who, at)| {
                            (*at, format!("approved by {who}; nobody held {here}"))
                        }),
                        refused: objection(people.first_objection()),
                        needs: format!(
                            "a person's approval — nobody holds {here} yet, and the proposer is \
                             not a person"
                        ),
                    };
                }
                if self.nobody_holds_before(p.scope, from) {
                    return Outcome {
                        // "Held before", not "holds": in a founding script
                        // the scope IS held, one second too late to count.
                        completed: Some((
                            from,
                            format!("nobody held {here} before this was written; it was open"),
                        )),
                        refused: None,
                        needs: String::new(),
                    };
                }
                let own = t
                    .approved
                    .first()
                    .filter(|(who, at)| who.actor() == p.actor && *at == p.at);
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
                        "approval from one person who holds {}{}",
                        self.held_as(p.scope, now),
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
                let refused = objection(
                    t.objections
                        .iter()
                        .find(|(_, by, _)| named(by.actor()))
                        .copied(),
                );
                let missing: Vec<&str> = holders
                    .iter()
                    .map(String::as_str)
                    .filter(|h| !t.approved.iter().any(|(who, _)| who.actor() == *h))
                    .collect();
                let completed = if missing.is_empty() {
                    let done = t
                        .approved
                        .iter()
                        .filter(|(who, _)| named(who.actor()))
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
            Ratify::Majority | Ratify::Unanimous | Ratify::Share(_) => {
                unreachable!("a share of the house is judged by judge_share")
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

    /// `majority`, `unanimous` and `share`: a share of the people who held
    /// the scope when the vote opened, at `from`, counted against them alone.
    /// Somebody who joins later never moves the count, so history stays
    /// still; and the moment the share can no longer be reached — enough of
    /// them said no, or no longer hold the scope — it is refused, so a house
    /// that has shrunk is never left waiting on people who have gone, its
    /// rule of rules included. A fixed count chosen on the day the house was
    /// four reaches neither of those.
    ///
    /// Everything here is read off the bar, never off which rule named it,
    /// so `majority` and the share more than half are one count.
    fn judge_share(&self, bar: Bar, p: &Proposal, t: &Tally, from: i64, now: i64) -> Outcome {
        let here = self.held_then(p.scope, from);
        let voters = self.holders_at(p.scope, from);
        let n = voters.len();
        // Only a bar of everyone lets one person decide at every size of
        // house, so only there does one reasoned objection count against,
        // as under `joint`, and only there is a refusal that person's. Read
        // off the bar and not off the count: a majority of two also takes
        // both, and counts any position against, as it did in 0.4.0.
        let all = bar.is_everyone();
        if n == 0 {
            // Every share of nobody is reached at once, which would make an
            // empty scope's first write law; refusing says why instead.
            return Outcome {
                completed: None,
                refused: Some((
                    from,
                    String::new(),
                    format!("nobody held {here} when it was proposed"),
                )),
                needs: String::new(),
            };
        }
        let needed = bar.needed(n);
        let approved: Vec<(Person, i64)> = t
            .approved
            .iter()
            .filter(|(who, _)| voters.contains(who.actor()))
            .copied()
            .collect();
        let completed = approved.get(needed - 1).map(|(_, at)| {
            let how = if all {
                format!("approved by all {n} people who held {here} when it was proposed")
            } else {
                format!("{needed} of the {n} people who held {here} when it was proposed approved")
            };
            (*at, how)
        });
        // Who can no longer approve, and from when: their first word against
        // (a reasoned objection when the bar is everyone, as under `joint`;
        // any position against below it, as `threshold` counts its block),
        // or the moment their standing ended. An approval first keeps them
        // counted, whatever they said after: first word wins.
        let mut lost: Vec<(i64, &str, Option<&str>)> = Vec::new();
        for &voter in &voters {
            let said = if all {
                t.objections
                    .iter()
                    .find(|(_, by, _)| by.actor() == voter)
                    .map(|(at, _, why)| (*at, Some(*why)))
            } else {
                t.against
                    .iter()
                    .find(|(_, by)| by.actor() == voter)
                    .map(|(at, by)| {
                        let why = t
                            .objections
                            .iter()
                            .find(|(o, b, _)| o == at && b == by)
                            .map(|(_, _, why)| *why);
                        (*at, why)
                    })
            };
            let ended = self
                .holding_ended(p.scope, voter, from, now)
                .map(|at| (at, None));
            let Some((at, why)) = [said, ended]
                .into_iter()
                .flatten()
                .min_by_key(|(at, _)| *at)
            else {
                continue;
            };
            let first_yes = approved.iter().find(|(who, _)| who.actor() == voter);
            if first_yes.is_none_or(|(_, yes)| *yes > at) {
                lost.push((at, voter, why));
            }
        }
        lost.sort();
        let refused = lost
            .iter()
            .enumerate()
            .find(|(i, _)| n - (i + 1) < needed)
            .map(|(_, &(at, voter, why))| {
                if all {
                    // One reasoned objection decides, so it is that
                    // person's refusal.
                    return match why {
                        Some(why) => (at, voter.to_string(), why.to_string()),
                        None => (
                            at,
                            String::new(),
                            format!(
                                "{voter} said no or no longer holds {here}, so everyone who \
                                 held it when it was proposed can no longer approve it"
                            ),
                        ),
                    };
                }
                // Below everyone, nobody refuses alone: it is refused by
                // everyone who could no longer approve it by then, counted,
                // with the reasons they gave. Everyone lost in that second
                // counts, so a tie within it reads the same in any order.
                let gone: Vec<&(i64, &str, Option<&str>)> =
                    lost.iter().take_while(|(when, _, _)| *when <= at).collect();
                let k = gone.len();
                let who = match (n, k) {
                    (1, _) => format!("the one person who held {here} when it was proposed"),
                    _ => format!("{k} of the {n} people who held {here} when it was proposed"),
                };
                let hold = if k == 1 { "holds" } else { "hold" };
                let mut why = format!(
                    "{who} said no or no longer {hold} it, so {} can no longer approve it",
                    bar.phrase()
                );
                let reasons: Vec<String> = gone
                    .iter()
                    .filter_map(|(_, by, said)| said.map(|said| format!("{by}: {}", said.trim())))
                    .collect();
                if !reasons.is_empty() {
                    why.push_str(" — ");
                    why.push_str(&reasons.join("; "));
                }
                (at, String::new(), why)
            });
        let needs = if all {
            let missing: Vec<&str> = voters
                .iter()
                .copied()
                .filter(|v| !approved.iter().any(|(who, _)| who.actor() == *v))
                .collect();
            format!(
                "approval from {}: everyone who held {here} when it was proposed",
                missing.join(", ")
            )
        } else {
            // Who is still able to approve, by name. The count alone left
            // the house unable to tell "Sam is thinking" from "Sam has not
            // seen it": two proposals could both say "one more approval"
            // and the same person could be the one missing from both.
            // Somebody who said no or no longer holds the scope is not a
            // name to chase, so they are left out.
            let lost_now: BTreeSet<&str> = lost.iter().map(|(_, voter, _)| *voter).collect();
            let missing: Vec<&str> = voters
                .iter()
                .copied()
                .filter(|v| !approved.iter().any(|(who, _)| who.actor() == *v))
                .filter(|v| !lost_now.contains(v))
                .collect();
            let count = needed.saturating_sub(approved.len());
            if missing.is_empty() {
                format!(
                    "{} more approval(s) from the {n} people who held {here} when it was proposed",
                    count
                )
            } else {
                // Three names is enough to chase; thirty is a list, not a
                // sentence. The count is still there for the rest.
                let shown = missing.len().min(3);
                let names = missing[..shown].join(if shown == 2 { " or " } else { ", " });
                let who = if missing.len() > shown {
                    format!("any of {names} and {} other(s)", missing.len() - shown)
                } else if shown <= 2 {
                    names
                } else {
                    format!("any of {names}")
                };
                format!(
                    "{} more approval(s) from {who} — of the {n} people who held {here} when it \
                     was proposed",
                    count
                )
            }
        };
        Outcome {
            completed,
            refused,
            needs,
        }
    }

    /// The first moment after `from`, up to `now`, at which `actor` no longer
    /// counts as holding `scope`: a withdrawal, a lapse, or someone granted a
    /// narrower level of it, which moves who counts. Only a moment a grant
    /// begins or ends, or personal scopes are switched, can change that, so
    /// those are the moments read.
    fn holding_ended(
        &self,
        scope: Option<&Scope>,
        actor: &str,
        from: i64,
        now: i64,
    ) -> Option<i64> {
        let mut moments: Vec<i64> = self
            .grants
            .iter()
            .flat_map(|g| {
                [
                    Some(g.granted_at),
                    g.withdrawn_at,
                    g.horizon.map(|h| h.saturating_add(1)),
                ]
            })
            .chain(
                self.personal
                    .on
                    .iter()
                    .flat_map(|(since, until, _)| [Some(*since), *until]),
            )
            .flatten()
            .filter(|at| *at > from && *at <= now)
            .collect();
        moments.sort_unstable();
        moments.dedup();
        moments
            .into_iter()
            .find(|at| !self.holder_at(scope, actor, *at))
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

    /// Where a retraction written without standing stands. See
    /// [`crate::fold::Retraction`].
    ///
    /// **Not the scope's ratification rule.** Retracting somebody else's
    /// commitment directly takes standing over its scope and nothing more,
    /// under any rule, so the people whose word counts here are exactly the
    /// people who could have written it themselves: a person whose seat is
    /// not [`Seat::Lacking`] when they speak. Judging it under `consent`
    /// instead would let a stranger's retraction take effect by waiting.
    pub fn judge_retraction(&self, p: &Proposal, now: i64) -> Verdict {
        let here = where_(p.scope);
        let mut said: Vec<(Person, &crate::fold::Stated)> = self
            .people_on(p.id.as_str())
            .filter(|(_, s)| self.seat(&s.by, p.scope, s.at) != Seat::Lacking)
            .collect();
        said.sort_by_key(|(_, s)| s.at);
        let completed = said
            .iter()
            .find(|(_, s)| s.position.pull == Pull::Toward)
            .map(|(who, s)| (s.at, format!("approved by {who}, who holds {here}")));
        let refused = said
            .iter()
            .find(|(_, s)| {
                s.position.pull == Pull::Against && !s.position.because.trim().is_empty()
            })
            .map(|(who, s)| (s.at, who.to_string(), s.position.because.clone()));
        settle(Outcome {
            completed,
            refused,
            needs: format!(
                "approval from a person who holds {}",
                self.held_as(p.scope, now)
            ),
        })
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
            "majority",
            "unanimous",
            "consent:7d",
            "twice:turnover:standing",
            "twice:7d:majority",
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

    fn bar(raw: &str) -> Bar {
        Ratify::parse(raw)
            .and_then(|r| r.bar())
            .unwrap_or_else(|| panic!("{raw} is a share"))
    }

    #[test]
    fn what_a_share_needs_of_one_to_forty_people() {
        // Worked by hand, and checked against a search for the fewest k of n
        // with k/n reaching the bar in exact fractions; not by the formula
        // under test.
        let table: [(&str, [usize; 40]); 5] = [
            (
                "majority",
                [
                    1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
                    13, 14, 14, 15, 15, 16, 16, 17, 17, 18, 18, 19, 19, 20, 20, 21,
                ],
            ),
            (
                "share:60%",
                [
                    1, 2, 2, 3, 3, 4, 5, 5, 6, 6, 7, 8, 8, 9, 9, 10, 11, 11, 12, 12, 13, 14, 14,
                    15, 15, 16, 17, 17, 18, 18, 19, 20, 20, 21, 21, 22, 23, 23, 24, 24,
                ],
            ),
            (
                "share:2/3",
                [
                    1, 2, 2, 3, 4, 4, 5, 6, 6, 7, 8, 8, 9, 10, 10, 11, 12, 12, 13, 14, 14, 15, 16,
                    16, 17, 18, 18, 19, 20, 20, 21, 22, 22, 23, 24, 24, 25, 26, 26, 27,
                ],
            ),
            (
                "share:3/4",
                [
                    1, 2, 3, 3, 4, 5, 6, 6, 7, 8, 9, 9, 10, 11, 12, 12, 13, 14, 15, 15, 16, 17, 18,
                    18, 19, 20, 21, 21, 22, 23, 24, 24, 25, 26, 27, 27, 28, 29, 30, 30,
                ],
            ),
            (
                "unanimous",
                [
                    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                    23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40,
                ],
            ),
        ];
        for (raw, needs) in table {
            let b = bar(raw);
            for (i, want) in needs.into_iter().enumerate() {
                assert_eq!(b.needed(i + 1), want, "{raw} of {}", i + 1);
            }
        }
        assert_eq!(bar("share:2/3").needed(3), 2, "two-thirds of three is two");
        assert_eq!(bar("share:67%").needed(3), 3, "where 67% of three is three");
        assert_eq!(bar("share:60%").needed(30), 18);
        assert_eq!(bar("share:>1/2").needed(30), 16);
        assert_eq!(bar("share:>60%").needed(5), 4, "more than three of five");
        // The counts 0.4.0 hard-coded, from the general bar.
        for n in 1..=1000 {
            assert_eq!(Bar::MAJORITY.needed(n), n / 2 + 1, "majority of {n}");
            assert_eq!(Bar::EVERYONE.needed(n), n, "everyone of {n}");
        }
        // No house is too big to count: widened, never wrapped.
        assert_eq!(Bar::EVERYONE.needed(usize::MAX), usize::MAX);
        assert_eq!(Bar::MAJORITY.needed(usize::MAX), usize::MAX / 2 + 1);
        let fine = Bar::new(u32::MAX - 1, u32::MAX, false).unwrap();
        assert_eq!(fine.needed(u32::MAX as usize), u32::MAX as usize - 1);
    }

    #[test]
    fn a_share_is_spelled_one_way_and_reads_back_as_itself() {
        for raw in [
            "share:60%",
            "share:2/3",
            "share:>60%",
            "share:>2/3",
            "share:50%",
            "share:75%",
            "share:1/3",
            "share:>0%",
            "twice:30d:share:60%",
            "twice:turnover:share:2/3",
        ] {
            let r = Ratify::parse(raw).expect(raw);
            assert_eq!(r.name(), raw, "{raw}");
            assert_eq!(Ratify::parse(&r.name()), Some(r), "{raw}");
        }
        // Another way to write the same bar is the same rule, and names
        // itself the one way.
        for (raw, is) in [
            ("share:3/5", "share:60%"),
            ("share:6/10", "share:60%"),
            (" share: 60 % ", "share:60%"),
            ("share:4/6", "share:2/3"),
            ("share:1/2", "share:50%"),
            ("share:>3/4", "share:>75%"),
            ("share:>1/2", "majority"),
            ("share:>50%", "majority"),
            ("share:>2/4", "majority"),
            ("share:100%", "unanimous"),
            ("share:1/1", "unanimous"),
            ("share:3/3", "unanimous"),
            ("twice:7d:share:>1/2", "twice:7d:majority"),
        ] {
            let r = Ratify::parse(raw).expect(raw);
            assert_eq!(r.name(), is, "{raw}");
            assert_eq!(Ratify::parse(is), Some(r), "{raw}");
        }
        assert_eq!(Ratify::parse("share:>1/2"), Some(Ratify::Majority));
        assert_eq!(Ratify::parse("share:100%"), Some(Ratify::Unanimous));
        assert_eq!(Ratify::parse("share:1/1"), Some(Ratify::Unanimous));
        // At least half is not more than half.
        assert!(matches!(Ratify::parse("share:50%"), Some(Ratify::Share(_))));
        // A share held directly names itself as the rule it equals.
        assert_eq!(Ratify::Share(Bar::MAJORITY).name(), "majority");
        assert_eq!(Ratify::Share(Bar::EVERYONE).name(), "unanimous");
        assert_eq!(Ratify::Majority.bar(), Some(Bar::MAJORITY));
        assert_eq!(Ratify::Unanimous.bar(), Some(Bar::EVERYONE));
        assert_eq!(Ratify::parse("threshold:2/1").and_then(|r| r.bar()), None);
    }

    #[test]
    fn a_share_nobody_could_reach_or_nobody_needs_to_is_refused_and_says_why() {
        use NotARule::{Bar as Bad, Share as Unread};
        for (raw, why) in [
            ("share:0%", Bad(BadBar::Nobody)),
            ("share:0/3", Bad(BadBar::Nobody)),
            ("share:>100%", Bad(BadBar::MoreThanEveryone)),
            ("share:>1/1", Bad(BadBar::MoreThanEveryone)),
            ("share:>3/3", Bad(BadBar::MoreThanEveryone)),
            ("share:101%", Bad(BadBar::MoreThanEveryone)),
            ("share:4/3", Bad(BadBar::MoreThanEveryone)),
            ("share:3/0", Bad(BadBar::OutOfNothing)),
            ("share:>0/0", Bad(BadBar::OutOfNothing)),
            ("twice:7d:share:0%", Bad(BadBar::Nobody)),
            // What a shell leaves of an unquoted `share:>1/2`.
            ("share:", Unread),
            ("share:>", Unread),
            ("share:sixty", Unread),
            ("share:62.5%", Unread),
            ("share:2/", Unread),
            ("share:-1/2", Unread),
            ("share:2/3/4", Unread),
            ("plurality", NotARule::Unknown),
        ] {
            assert_eq!(raw.parse::<Ratify>(), Err(why), "{raw}");
            assert_eq!(Ratify::parse(raw), None, "{raw}");
        }
        let said = |raw: &str| raw.parse::<Ratify>().unwrap_err().to_string();
        assert!(said("share:0%").contains("before anyone approved"));
        assert!(said("share:>100%").contains("more than everyone"));
        assert!(said("share:3/0").contains("can't be 0"));
        assert!(
            said("share:").contains("'share:>1/2'"),
            "{}",
            said("share:")
        );
        // A count of people approving and blocking is not a share of them.
        assert_eq!(
            Ratify::parse("threshold:2/1"),
            Some(Ratify::Threshold {
                approve: 2,
                block: 1
            })
        );
    }

    #[test]
    fn a_share_reads_as_what_it_takes() {
        assert_eq!(bar("majority").phrase(), "more than half");
        assert_eq!(bar("unanimous").phrase(), "everyone");
        assert_eq!(bar("share:60%").phrase(), "60%");
        assert_eq!(bar("share:>2/3").phrase(), "more than 2/3");
        let prose = Ratify::parse("share:2/3").unwrap().prose();
        assert!(prose.contains("at least 2/3 of the people"), "{prose}");
        let prose = Ratify::parse("share:>60%").unwrap().prose();
        assert!(prose.contains("more than 60% of the people"), "{prose}");
        assert_eq!(
            Ratify::Share(Bar::MAJORITY).prose(),
            Ratify::Majority.prose()
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
