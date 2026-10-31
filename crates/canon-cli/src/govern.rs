// SPDX-License-Identifier: AGPL-3.0-or-later
//! The governance verbs — who decides, under what rule, and what was decided.
//!
//! All model-free. Every command here reads or appends an act and folds; none
//! of them touches an endpoint, which is what makes a governance replay
//! possible and is the single most important fact about this layer.

use canon_core::{Act, ActKind, Authority, LadderKey, Outcome, Policy as _, Rule, Scope};

use crate::cmds::{fail, flag, has, load, or_fail, positionals, write};
use crate::store;

/// Parse a rule from the command line.
///
/// The base rule is a positional word; the modifiers are flags that WRAP it,
/// in a fixed order, so `--cautious --entrench principle` and the reverse
/// produce the same rule. Two spellings of one policy would be two policies
/// as far as `policy_for` is concerned, and the ledger would carry whichever
/// order somebody typed.
pub(crate) fn rule_from(args: &[String], base: &str) -> Result<Rule, String> {
    let mut rule = match base {
        "default" => Rule::Default,
        "consent" => Rule::Consent,
        "subsidiarity" => Rule::Subsidiarity,
        "threshold" => {
            let n = flag(args, "--objections")
                .ok_or("threshold needs --objections <n>")?
                .parse::<usize>()
                .map_err(|_| "--objections takes a number")?;
            if n == 0 {
                return Err("--objections 0 would make every proposal a conflict".into());
            }
            Rule::Threshold { against: n }
        }
        "supermajority" => {
            let raw = flag(args, "--of").ok_or("supermajority needs --of <n>/<d>, e.g. 2/3")?;
            let (n, d) = raw
                .split_once('/')
                .ok_or("--of takes a fraction like 2/3")?;
            let numerator: u32 = n
                .trim()
                .parse()
                .map_err(|_| "--of takes a fraction like 2/3")?;
            let denominator: u32 = d
                .trim()
                .parse()
                .map_err(|_| "--of takes a fraction like 2/3")?;
            if denominator == 0 || numerator > denominator {
                return Err(format!("`{raw}` is not a share of a whole"));
            }
            Rule::Supermajority {
                numerator,
                denominator,
            }
        }
        other => {
            return Err(format!(
                "unknown rule `{other}` — one of: default, consent, threshold, \
                 supermajority, subsidiarity"
            ))
        }
    };
    // Innermost first, so the wrapping order is a property of the code and
    // not of the argument order.
    if let Some(ladder) = flag(args, "--escalate") {
        let rungs: Result<Vec<Authority>, String> = ladder.split(',').map(authority).collect();
        let rungs = rungs?;
        if rungs.is_empty() {
            return Err(
                "--escalate takes its steps in order, e.g. ask-one,ask-panel,refuse".into(),
            );
        }
        // Keyed on the cited rule and reset by a repair unless asked
        // otherwise: a paraphrase climbing a fresh ladder and a house meeting
        // a solved problem on the top rung are the two ways a ladder keyed on
        // words goes wrong. Each opt-out writes the field absent rather than
        // spelled out, so `--by-about --no-repair` is the body a ladder had
        // before these knobs, at the version it had.
        let key = (!has(args, "--by-about")).then_some(LadderKey::Cited);
        let repair = (!has(args, "--no-repair")).then_some(true);
        rule = Rule::Graduated {
            ladder: rungs,
            base: Box::new(rule),
            key,
            repair,
        };
    }
    if let Some(ranks) = flag(args, "--constitutional") {
        rule = Rule::Entrenched {
            protected: ranks.split(',').map(|r| r.trim().to_string()).collect(),
            base: Box::new(rule),
        };
    }
    if has(args, "--cautious") {
        rule = Rule::Cautious {
            base: Box::new(rule),
        };
    }
    Ok(rule)
}

fn authority(raw: &str) -> Result<Authority, String> {
    Authority::parse(raw).ok_or_else(|| {
        format!("unknown authority `{raw}` — one of: act, notify, ask-one, ask-panel, refuse")
    })
}

fn outcome(raw: &str) -> Result<Outcome, String> {
    match raw.trim() {
        "supported" => Ok(Outcome::Supported),
        "conflicts" => Ok(Outcome::Conflicts),
        "unaddressed" => Ok(Outcome::Unaddressed),
        other => Err(format!(
            "unknown outcome `{other}` — one of: supported, conflicts, unaddressed"
        )),
    }
}

fn scope_from(args: &[String]) -> Result<Option<Scope>, String> {
    match flag(args, "--area") {
        None => Ok(None),
        Some(raw) => Scope::new(raw).map(Some).ok_or_else(|| {
            format!("`{raw}` is not an area: a dotted path like `house.kitchen`, no empty parts")
        }),
    }
}

// ── canon policy ────────────────────────────────────────────

pub fn policy(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("set") => {
            let Some(base) = pos.get(1) else {
                return fail(
                    "usage: canon rulings set <default|consent|threshold|supermajority|subsidiarity> \
                     [--objections n] [--of n/d] [--escalate a,b,c [--by-about] [--no-repair]] \
                     [--constitutional rank] [--cautious] [--area s] [-m \"<how it reads>\"]",
                );
            };
            let rule = or_fail!(rule_from(args, base));
            let scope = or_fail!(scope_from(args));
            let (d, _, _) = or_fail!(load());
            // The community's own words when they wrote any, and a plain
            // rendering of the typed rule when they did not. Never blended,
            // and the difference is visible on the way in.
            let own = flag(args, "-m").filter(|m| !m.trim().is_empty());
            let text = own.map_or_else(|| rule.prose(), str::to_string);
            match write(
                &d,
                ActKind::Policy {
                    text: text.clone(),
                    rule: rule.clone(),
                    scope: scope.clone(),
                },
            ) {
                Ok(act) => crate::cmds::governed(&d, &act.id, || {
                    println!("{}  {}", act.id, rule.name());
                    println!("  {text}");
                    if own.is_none() {
                        println!("  (our words, not yours — `-m` says it in the house's own)");
                    }
                    match scope {
                        Some(s) => println!("  governs {s} and everything under it"),
                        None => println!("  governs this canon"),
                    }
                    // Under `governs: all` this is a proposal, and the person
                    // who typed it hears so here, the way `ratification set`
                    // says it.
                    crate::cmds::report_status(&d, &act.id);
                }),
                Err(e) => fail(e),
            }
        }
        Some("show") | None => {
            let (_, _, canon) = or_fail!(load());
            if has(args, "--json") {
                // The bare array when nothing waits, which is what every
                // canon without the knob has always printed.
                let out = if canon.pending_policies().is_empty() {
                    serde_json::to_string_pretty(&canon.policies())
                } else {
                    serde_json::to_string_pretty(&serde_json::json!({
                        "policies": canon.policies(),
                        "pending": canon.pending_policies(),
                    }))
                };
                println!("{}", out.unwrap_or_default());
                return 0;
            }
            if canon.policies().is_empty() && canon.pending_policies().is_empty() {
                // Absence reported as absence. A canon that adopted nothing
                // is still governed by something, and it should know which.
                println!("no rulings chosen, so actions here are judged as canon shipped:");
                println!("  `{}`: {}", Rule::Default.name(), Rule::Default.prose());
                println!("\n  choose one:  canon rulings set consent -m \"<how it reads here>\"");
                return 0;
            }
            // In force, then waiting: the same lines, and the verb says which.
            let adopted = canon.policies().iter().map(|p| (p, "chosen"));
            for (p, verb) in adopted.chain(canon.pending_policies().iter().map(|p| (p, "set"))) {
                let at = p
                    .scope
                    .as_ref()
                    .map_or("the whole canon".into(), |s| s.to_string());
                println!("{}  `{}`  for {at}", p.act, p.rule.name());
                println!("  {}", p.text);
                println!("  {verb} {} by {}", store::ymd(p.at), p.actor);
                if let Some(v) = &p.verdict {
                    crate::cmds::report_verdict(&p.act, v);
                }
            }
            println!("\n{} rulings chosen", canon.policies().len());
            println!(
                "  change one:  canon rulings set <rule> [--area <area>] -m \"<how it reads>\""
            );
            0
        }
        Some(other) => fail(format!(
            "unknown rulings command `{other}` — expected set or show"
        )),
    }
}

// ── canon ratification ──────────────────────────────────────

/// How a proposal in a scope becomes a rule. See `canon_core::ratify`.
///
/// Setting it is gated in the fold by standing over the scope — the level
/// above decides how a level makes rules — so a `set` by somebody without
/// standing is written, not applied, and says so.
/// The two v4 knobs a new ratification rule is written with.
///
/// **Carried forward, then batteries.** A flag says it outright. Without
/// one, the value the scope already lives under is written again, so
/// `ratification set consent:7d --scope house.kitchen` cannot quietly reopen
/// one level down what the house closed. With nothing to carry, the default
/// is the recommended one: a change of policy waits like any other change,
/// and a person's own scope is theirs. Writing the value explicitly, rather
/// than leaving it absent, is what keeps the meaning in the ledger instead of
/// in whichever build reads it.
fn knobs(
    args: &[String],
    canon: &canon_core::Canon,
    scope: Option<&canon_core::Scope>,
) -> Result<(canon_core::Governs, Option<bool>), String> {
    let now = store::now();
    let in_force = canon.adopted_at(scope, now);
    let governs = match flag(args, "--governs") {
        Some("all") => canon_core::Governs::All,
        // `rules` is the plain word; `commitments` is what it was first called.
        Some("rules" | "commitments") => canon_core::Governs::Commitments,
        Some(other) => {
            return Err(format!(
                "`--governs {other}` — expected `all` (a change to the rulings waits like a \
                 rule) or `rules` (it takes effect because its writer is in charge)"
            ))
        }
        None => in_force
            .and_then(|r| r.governs)
            .unwrap_or(canon_core::Governs::All),
    };
    let personal = match (flag(args, "--personal"), scope) {
        (Some(_), Some(s)) => {
            return Err(format!(
                "`--personal` is canon-wide; it cannot be set for {s}. \
                 Drop --area, or drop --personal"
            ))
        }
        (Some("on"), None) => Some(true),
        (Some("off"), None) => Some(false),
        (Some(other), None) => {
            return Err(format!("`--personal {other}` — expected `on` or `off`"))
        }
        (None, Some(_)) => None,
        (None, None) => Some(
            canon
                .adopted_at(None, now)
                .and_then(|r| r.personal)
                .unwrap_or(true),
        ),
    };
    Ok((governs, personal))
}

/// A ratification rule from what someone typed, or what to type instead.
/// `ratification set` and `replay --ratification` both read rules here.
pub(crate) fn ratify_from(raw: &str) -> Result<canon_core::Ratify, String> {
    raw.parse().map_err(|e| match e {
        canon_core::NotARule::Unknown => format!(
            "`{raw}` is not a ratification rule — standing, joint:human:a,human:b, \
             threshold:2/1 (two approving carry it, one objecting stops it), majority, \
             unanimous, share:60% or share:2/3 (that share of the people who hold the \
             scope), consent:7d, or twice:turnover:consent:7d"
        ),
        e => format!("`{raw}` — {e}"),
    })
}

pub fn ratification(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("set") => {
            let Some(raw) = pos.get(1) else {
                return fail(
                    "usage: canon ratification set \
                     <standing|joint:a,b|threshold:n/m|majority|unanimous|share:N%|share:a/b|\
                     consent:Nd|twice:<turnover|Nd>:<rule>> \
                     [--area s] [--governs all|commitments] [--personal on|off] \
                     [-m \"<how it reads>\"]",
                );
            };
            let rule = or_fail!(ratify_from(raw));
            let scope = or_fail!(scope_from(args));
            let (d, _, canon) = or_fail!(load());
            let (governs, personal) = or_fail!(knobs(args, &canon, scope.as_ref()));
            let own = flag(args, "-m").filter(|m| !m.trim().is_empty());
            let text = own.map_or_else(|| rule.prose(), str::to_string);
            match write(
                &d,
                ActKind::Ratification {
                    text: text.clone(),
                    rule: rule.clone(),
                    scope: scope.clone(),
                    governs: Some(governs),
                    personal,
                },
            ) {
                Ok(act) => crate::cmds::governed(&d, &act.id, || {
                    println!("{}  {}", act.id, rule.name());
                    println!("  {text}");
                    match &scope {
                        Some(s) => println!("  how rules are made in {s} and everything under it"),
                        None => println!("  how rules are made in this canon"),
                    }
                    // The change is judged under the rule it is changing.
                    crate::cmds::report_status(&d, &act.id);
                }),
                Err(e) => fail(e),
            }
        }
        Some("show") | None => {
            let (_, _, canon) = or_fail!(load());
            if has(args, "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&canon.ratifications()).unwrap_or_default()
                );
                return 0;
            }
            let next = "canon ratification set joint:human:a,human:b --area house.kitchen";
            if canon.ratifications().is_empty() {
                println!("no way of passing rules chosen, so rules pass here as canon shipped:");
                let shipped = canon_core::Ratify::Standing;
                println!("  `{}`: {}", shipped.name(), shipped.prose());
                println!("\n  choose one:  {next}");
                return 0;
            }
            let now = store::now();
            for r in canon.ratifications() {
                // The one deciding its scope today, among the history kept.
                let decides = canon.adopted_at(r.scope.as_ref(), now);
                let deciding = decides.is_some_and(|d| d.act == r.act);
                let mark = if deciding { "  (in use now)" } else { "" };
                let at = r
                    .scope
                    .as_ref()
                    .map_or("the whole canon".into(), |s| s.to_string());
                println!(
                    "{}  how rules pass in {at}: `{}`{mark}",
                    r.act,
                    r.rule.name()
                );
                println!("  {}", r.text);
                let (when, under) = (store::ymd(r.at), r.under.name());
                println!(
                    "  set {when} by {}, and passed the way `{under}` says",
                    r.actor
                );
                // A rule that was in force and is not deciding was replaced:
                // "in force since" beside the one deciding read as two in force.
                match (decides, &r.verdict) {
                    (Some(d), canon_core::Verdict::Ratified { .. }) if !deciding => {
                        println!("  replaced: {} decides now", d.act)
                    }
                    _ => crate::cmds::report_verdict(&r.act, &r.verdict),
                }
            }
            println!("\n  change it:  {next}");
            println!("  every rule you can choose:  canon help ratification");
            0
        }
        Some(other) => fail(format!(
            "unknown ratification command `{other}` — expected set or show"
        )),
    }
}

// ── canon allot / allocation / pool ─────────────────────────

/// `1d`, `12h`, `90m`, `3600s` — a duration, not a calendar period.
///
/// A rotation counted in seconds from its own adoption needs no timezone, no
/// leap rule and no agreement about when a week starts. Two readers on two
/// continents compute the same turn.
pub(crate) fn every(raw: &str) -> Result<i64, String> {
    let raw = raw.trim();
    let (n, mult) = match raw.chars().last() {
        Some('d') => (&raw[..raw.len() - 1], 86_400),
        Some('h') => (&raw[..raw.len() - 1], 3_600),
        Some('m') => (&raw[..raw.len() - 1], 60),
        Some('s') => (&raw[..raw.len() - 1], 1),
        _ => (raw, 86_400),
    };
    n.parse::<i64>()
        .map_err(|_| format!("`{raw}` is not a period — try 1d, 12h, 90m"))
        .and_then(|v| {
            if v > 0 {
                Ok(v * mult)
            } else {
                Err("a period has to be longer than nothing".into())
            }
        })
}

pub fn allot(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(raw) = pos.first() else {
        return fail(
            "usage: canon allot <area> --named a,b,c | --count <n> [--unit <noun>] [-m \"<what it is>\"]",
        );
    };
    let Some(scope) = Scope::new(raw) else {
        return fail(format!("`{raw}` is not an area"));
    };
    let unit = flag(args, "--unit").unwrap_or("turn").to_string();
    let units: Vec<String> = match (flag(args, "--named"), flag(args, "--count")) {
        (Some(_), Some(_)) => {
            return fail("--named and --count say the same thing two ways; pick one")
        }
        (Some(list), None) => list
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect(),
        (None, Some(n)) => match n.parse::<usize>() {
            Ok(n) if n > 0 => (1..=n).map(|i| i.to_string()).collect(),
            _ => return fail("--count takes a number above zero"),
        },
        (None, None) => return fail("a pool needs --named a,b,c or --count <n>"),
    };
    if units.is_empty() {
        return fail("--named needs at least one unit");
    }
    let (d, _, _) = or_fail!(load());
    let own = flag(args, "-m").filter(|m| !m.trim().is_empty());
    let text = own.map_or_else(
        || format!("{} {unit}(s) in {scope}.", units.len()),
        str::to_string,
    );
    match write(
        &d,
        ActKind::Allot {
            text: text.clone(),
            unit: unit.clone(),
            units: units.clone(),
            scope: scope.clone(),
        },
    ) {
        Ok(act) => {
            let code = crate::cmds::governed(&d, &act.id, || {
                println!("{}  {} {unit}(s) in {scope}", act.id, units.len());
                println!("  {text}");
                println!("  {}", units.join(", "));
            });
            if code == 0 {
                println!("\n  now say how they are shared:  canon allocation set rotation --area {scope}");
            }
            code
        }
        Err(e) => fail(e),
    }
}

pub fn allocation(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("set") => {
            let base = pos.get(1).copied().unwrap_or("rotation");
            if base != "rotation" {
                return fail(format!(
                    "unknown allocation rule `{base}` — this build ships rotation"
                ));
            }
            let scope = match scope_from(args) {
                Ok(Some(s)) => s,
                Ok(None) => return fail("an allocation rule needs --area <area>"),
                Err(e) => return fail(e),
            };
            // Where the order comes from. The draw's contribution is its
            // SEED, not its seats: ordering everyone is a different question
            // from selecting a few, asked of the same unsteerable number.
            let order = match (flag(args, "--order"), flag(args, "--from-draw")) {
                (Some(_), Some(_)) => {
                    return fail("--order and --from-draw are two answers to one question")
                }
                (Some(list), None) => canon_core::Order::Given {
                    actors: list
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect(),
                },
                // A draw's commit is not a commitment, so it resolves through
                // the draw's own lookup rather than the commitment one.
                (None, Some(id)) => {
                    let (_, _, canon) = or_fail!(load());
                    match crate::draw_cmd::resolve_draw(&canon, id) {
                        Ok(commit) => canon_core::Order::FromDraw { commit },
                        Err(e) => return fail(e),
                    }
                }
                (None, None) => canon_core::Order::Holders,
            };
            let step = match flag(args, "--step").map(str::parse::<i64>) {
                None => 1,
                Some(Ok(n)) => n,
                Some(Err(_)) => {
                    return fail("--step takes a whole number; its sign is the direction")
                }
            };
            let per = or_fail!(every(flag(args, "--per").unwrap_or("1d")));
            let rule = canon_core::Allocation::Rotation { order, step, per };
            let (d, _, _) = or_fail!(load());
            let own = flag(args, "-m").filter(|m| !m.trim().is_empty());
            let text = own.map_or_else(|| rule.prose(), str::to_string);
            match write(
                &d,
                ActKind::Allocation {
                    text: text.clone(),
                    rule: rule.clone(),
                    scope: scope.clone(),
                },
            ) {
                Ok(act) => {
                    let code = crate::cmds::governed(&d, &act.id, || {
                        println!("{}  {}", act.id, rule.name());
                        println!("  {text}");
                        println!("  shares {scope} and everything under it");
                    });
                    if code == 0 {
                        println!("\n  whose turn is it:  canon pool {scope}");
                    }
                    code
                }
                Err(e) => fail(e),
            }
        }
        Some("show") | None => {
            let (_, _, canon) = or_fail!(load());
            if canon.allocations().is_empty() {
                println!("no allocation rule adopted; nothing here is shared out by rule.");
                println!("  adopt one:  canon allocation set rotation --area <area>");
                return 0;
            }
            for a in canon.allocations() {
                println!("{}  {}  over {}", a.act, a.rule.name(), a.scope);
                println!("  {}", a.text);
                println!("  adopted {} by {}", store::ymd(a.at), a.actor);
            }
            0
        }
        Some(other) => fail(format!(
            "unknown allocation command `{other}` — expected set or show"
        )),
    }
}

/// Whose turn it is. The counterpart to `who`: **`who` is authority, `pool`
/// is appropriation.**
pub fn pool(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(raw) = pos.first() else {
        return fail("usage: canon pool <area> [--at YYYY-MM-DD]");
    };
    let Some(scope) = Scope::new(raw) else {
        return fail(format!("`{raw}` is not an area"));
    };
    let (_, _, canon) = or_fail!(load());
    let at = match flag(args, "--at") {
        None => store::now(),
        Some(d) => match canon_core::date::parse_ymd(d) {
            Some(ts) => ts,
            None => return fail(format!("`{d}` is not a date")),
        },
    };
    let schedule = or_fail!(canon.pool_at(&scope, at));
    if has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&schedule).unwrap_or_default()
        );
        return 0;
    }
    println!(
        "{} — turn {} under {}",
        scope, schedule.period, schedule.rule
    );
    println!();
    let width = schedule
        .awards
        .iter()
        .map(|a| a.unit.chars().count())
        .max()
        .unwrap_or(0);
    for a in &schedule.awards {
        println!("  {:<width$}  {}", a.unit, a.actor);
    }
    if !schedule.free.is_empty() {
        println!("\n  free: {}", schedule.free.join(", "));
    }
    // A pool too small for its community is a finding about the commons, not
    // an empty row to leave out.
    if !schedule.idle.is_empty() {
        println!(
            "\n  {} of those in charge have no {} this turn: {}",
            schedule.idle.len(),
            schedule.unit,
            schedule.idle.join(", ")
        );
    }
    0
}

// ── canon decide ────────────────────────────────────────────

/// Record that the group decided something.
///
/// **This records a decision, not an observation, and there is no verb that
/// records the second kind.** Ostrom's graduated sanctions need to know that
/// this is the third time; counting occurrences by person is the surveillance
/// file this project refuses. "The house asked Dana to stop doing X" is an
/// adjudication, attributed to whoever made it. "Dana ran the washing machine
/// at 1am" is not, and the format has nowhere to put it.
pub fn decide(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(about) = pos.first() else {
        return fail(
            "usage: canon decide \"<what it was about>\" allow|deny [--citing <id>,…] \
             -m \"<what was decided>\"\n       \
             canon decide \"<what it was about>\" --outcome <supported|conflicts|unaddressed> \
             --authority <act|notify|ask-one|ask-panel|refuse> [--citing <id>,…] \
             -m \"<what was decided>\"",
        );
    };
    // The short form is a ruling as the person asked says it: allow went
    // FOR the action, which closes a graduated rule's episode; deny went
    // against it, which is the conflict its next rung counts.
    let ruled = match pos.get(1).copied() {
        None => None,
        Some("allow") => Some((Outcome::Supported, Authority::Act)),
        Some("deny") => Some((Outcome::Conflicts, Authority::Refuse)),
        Some(other) => return fail(format!("`{other}` is not a ruling — allow or deny")),
    };
    let (outcome, authority) = match (ruled, flag(args, "--outcome"), flag(args, "--authority")) {
        (Some(r), None, None) => r,
        (None, Some(o), Some(a)) => match (outcome(o), authority(a)) {
            (Ok(o), Ok(a)) => (o, a),
            (Err(e), _) | (_, Err(e)) => return fail(e),
        },
        _ => {
            return fail(
                "decide takes allow|deny, or --outcome and --authority, never both — \
                 defaulting either half would record a decision nobody made",
            )
        }
    };
    let (d, _, canon) = or_fail!(load());
    let mut citing = Vec::new();
    for needle in flag(args, "--citing")
        .into_iter()
        .flat_map(|raw| raw.split(','))
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        match crate::explain::resolve(&canon, needle) {
            Ok(id) if !citing.contains(&id) => citing.push(id),
            Ok(_) => {}
            Err(e) => return fail(e),
        }
    }
    let prior = canon.prior_decisions(about).len();
    // A ladder keyed on the cited rule cannot see a decision that cites
    // nothing. Said once, here, rather than discovered at the next check.
    let missed = citing.is_empty() && canon.policies().iter().any(|p| keys_on_cited(&p.rule));
    match write(
        &d,
        ActKind::Decided {
            about: (*about).to_string(),
            outcome,
            authority,
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
            citing: citing.clone(),
        },
    ) {
        Ok(act) => crate::cmds::governed(&d, &act.id, || {
            println!("{act_id}  decided about \"{about}\"", act_id = act.id);
            println!("  {} / {authority}", format!("{outcome:?}").to_lowercase());
            if !citing.is_empty() {
                let ids: Vec<&str> = citing.iter().map(|i| i.as_str()).collect();
                println!("  citing {}", ids.join(", "));
            }
            // The rung is shown because the ladder is the point: a decision
            // that silently moves the next one up is a decision nobody saw.
            println!("  this is decision {} about it", prior + 1);
            if missed {
                println!(
                    "  a graduated policy here counts by the rule cited; without --citing \
                     this decision will not count on that ladder"
                );
            }
        }),
        Err(e) => fail(e),
    }
}

/// Whether a rule, or a rule it wraps, is a ladder keyed on cited rules.
fn keys_on_cited(rule: &Rule) -> bool {
    match rule {
        Rule::Graduated {
            key: Some(LadderKey::Cited),
            ..
        } => true,
        Rule::Graduated { base, .. } | Rule::Entrenched { base, .. } | Rule::Cautious { base } => {
            keys_on_cited(base)
        }
        _ => false,
    }
}

// ── canon rank ──────────────────────────────────────────────

/// Mark a commitment as a principle rather than a convention.
///
/// The vocabulary is the community's, not ours: `rank` is open text because
/// which ranks exist and what they mean differs by group. A policy reads it;
/// nothing in the library interprets it.
pub fn rank(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon rank <id> <rank>   e.g. canon rank can-abc principle");
    }
    let (d, _, canon) = or_fail!(load());
    let id = or_fail!(crate::explain::resolve(&canon, pos[0]));
    match write(
        &d,
        ActKind::Rank {
            commitment: id.clone(),
            rank: pos[1].to_string(),
        },
    ) {
        Ok(_) => {
            println!("{id} is a {}", pos[1]);
            0
        }
        Err(e) => fail(e),
    }
}

// ── canon who ───────────────────────────────────────────────

/// Every scope a grant or a commitment names, once each.
fn scopes_in_use(canon: &canon_core::Canon) -> Vec<Scope> {
    let mut used: Vec<Scope> = canon
        .grants()
        .iter()
        .map(|g| g.scope.clone())
        .chain(canon.scopes().iter().map(|(_, s)| s.clone()))
        .collect();
    used.sort();
    used.dedup();
    used
}

/// Areas, or anything else, as a reader lists them: `house, repo`.
fn listed<T: ToString>(items: &[T]) -> String {
    let all: Vec<String> = items.iter().map(ToString::to_string).collect();
    all.join(", ")
}

/// ` until 2027-01-01` for a grant with an end, nothing for one without.
fn until(g: &canon_core::Grant) -> String {
    g.horizon
        .map_or(String::new(), |h| format!(" until {}", store::ymd(h)))
}

/// How rules pass in an area and how actions are judged there, each named
/// on its own line, and where to read either in full.
///
/// Two rules, two lines. One unlabelled line read as the ratification rule
/// next to a ratification rule you had just set, and said `default` — which
/// is the policy, and looked like the set had not taken.
fn under(canon: &canon_core::Canon, scope: Option<&Scope>, now: i64) {
    let made = match canon.adopted_at(scope, now) {
        Some(r) => format!(
            "`{}` — set {} by {}{}",
            r.rule.name(),
            store::ymd(r.at),
            r.actor,
            r.scope
                .as_ref()
                .map_or(String::new(), |s| format!(" for {s}"))
        ),
        None => "`standing` (as shipped)".to_string(),
    };
    println!("how rules pass here: {made}");
    // A change still waiting is shown under the rule it would replace, so
    // "I set it and it did not take" has its answer on the same screen.
    for r in canon.ratifications().iter().filter(|r| {
        r.scope
            .as_ref()
            .is_none_or(|s| scope.is_some_and(|at| s.covers(at)))
    }) {
        if let canon_core::Verdict::Proposed { needs, .. } = &r.verdict {
            let head = format!("  a change to `{}` is PROPOSED — needs ", r.rule.name());
            println!("{}", crate::wrap::hang(&head, needs));
        }
    }
    let judged = canon.policy_for(scope).name();
    println!("how actions are judged here: `{judged}`");
    println!("  in full:  canon ratification show   canon rulings show");
}

/// `canon who` with no area: who is in charge of the canon as a whole.
///
/// **The canon has no scope of its own.** An unscoped proposal "needs
/// approval from one person who holds this canon", and there is nothing
/// called that to grant: the holders are whoever holds the widest scopes
/// anyone holds. Asking for it by a guessed name — `canon who default` —
/// used to answer "grant it", and following that made a scope called
/// `default` that meant nothing more than any other.
fn who_holds_the_canon(args: &[String]) -> i32 {
    let (_, _, canon) = or_fail!(load());
    let now = store::now();
    let scopes = canon.canon_scopes(now);
    let mut holders: Vec<&canon_core::Grant> = canon
        .grants()
        .iter()
        .filter(|g| g.held_at(now) && scopes.contains(&&g.scope))
        .collect();
    holders.sort_by(|a, b| a.scope.cmp(&b.scope).then_with(|| a.actor.cmp(&b.actor)));
    if has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&holders).unwrap_or_default()
        );
        return 0;
    }
    if holders.is_empty() {
        println!("nobody is in charge of this canon yet, so it is open.");
        println!("  canon give <actor> <area>   whoever has the widest area is in charge of it");
        return 0;
    }
    println!(
        "in charge of the canon as a whole: whoever is in charge of {}, its widest area{}",
        listed(&scopes),
        if scopes.len() == 1 { "" } else { "s" }
    );
    for g in &holders {
        let bare = if g.actor.contains(':') {
            String::new()
        } else {
            "  (a bare name — not human:".to_string() + &g.actor + ")"
        };
        println!("  {}  in charge of {}{}{bare}", g.actor, g.scope, until(g));
    }
    println!("\nthey say yes or no to proposals made outside any area.");
    under(&canon, None, now);
    // The ledger is careful; the summary above it must say so too.
    if !canon.ungoverned().is_empty() {
        println!(
            "{} change(s) to who decides came from someone not in charge and were not applied — `canon log`",
            canon.ungoverned().len()
        );
    }
    0
}

/// Who may decide this?
///
/// **Answerable without asking a person, and that is the whole point.**
/// Informal power runs on private knowledge of the process: a group where
/// finding out who decides means knowing whom to ask has made that person the
/// gatekeeper. This is the Freeman floor, and it is one query.
pub fn who(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(raw) = pos.first() else {
        return who_holds_the_canon(args);
    };
    let Some(scope) = Scope::new(raw) else {
        return fail(format!(
            "`{raw}` is not an area: a dotted path like `house.kitchen`, no empty parts"
        ));
    };
    let (_, _, canon) = or_fail!(load());
    let now = store::now();
    let deciders = canon.who_decides(&scope, now);
    if has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&deciders).unwrap_or_default()
        );
        return 0;
    }
    if deciders.is_empty() {
        // Not an error, and not an empty list rendered as nothing: an
        // unheld boundary is a finding about the canon.
        println!("nobody is in charge of `{scope}`.");
        println!("  put someone in charge:  canon give <actor> {scope}");
        // A scope typed as a guess at the canon's own name — `default`,
        // `root` — gets this same answer, and the hint above turns the guess
        // into a scope. Say when nothing is in it, and what is.
        let used = scopes_in_use(&canon);
        if !used.iter().any(|s| scope.covers(s) || s.covers(&scope)) {
            println!(
                "  nothing in this canon is in `{scope}`{}",
                if used.is_empty() {
                    String::new()
                } else {
                    format!(" — the areas in use are {}", listed(&used))
                }
            );
            println!("  `canon who` with no area says who is in charge of the whole canon");
        }
        // Lapsed standing is remembered here even though it is not held, so
        // "it used to be Dana and nobody renewed it" is answerable.
        let lapsed: Vec<&canon_core::Grant> = canon
            .grants()
            .iter()
            .filter(|g| g.lapsed(now) && g.scope.covers(&scope))
            .collect();
        if !lapsed.is_empty() {
            println!(
                "\n  {} were in charge of it until a date now past:",
                lapsed.len()
            );
            for g in lapsed {
                println!("    {}{}", g.actor, until(g));
            }
        }
        return 0;
    }
    for g in &deciders {
        println!("{}  in charge of {}{}", g.actor, g.scope, until(g));
    }
    println!("\n{} in charge, closest area first", deciders.len());
    under(&canon, Some(&scope), now);
    0
}

// ── canon grant / withdraw ──────────────────────────────────

/// A bare name is a different person: `Mira` is not `human:Mira`. The
/// group's evening was lost to `grant Mira`; name the mistake before it is
/// written, and give the line to run.
fn bare_actor(name: &str, verb: &str) -> Option<String> {
    if name.contains(':') {
        return None;
    }
    Some(format!(
        "`{name}` is not an actor — actors are `human:<name>` or `agent:<name>`, and `{name}` is a different person from `human:{name}`. Try `canon {verb} human:{name} …`"
    ))
}

pub fn grant(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon give <actor> <area> [--until YYYY-MM-DD] [-m \"<why>\"]");
    }
    if let Some(e) = bare_actor(pos[0], "give") {
        return fail(e);
    }
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not an area", pos[1]));
    };
    let horizon = match flag(args, "--until").map(canon_core::date::parse_ymd) {
        None => None,
        Some(Some(ts)) => Some(ts),
        Some(None) => return fail("--until takes a date like 2026-12-31"),
    };
    let (d, _, _) = or_fail!(load());
    let rationale = flag(args, "-m").unwrap_or_default();
    match give(&d, &store::actor(), pos[0], &scope, horizon, rationale) {
        Ok(act) => {
            let holder = pos[0];
            let code = crate::cmds::governed(&d, &act.id, || {
                match horizon {
                    Some(h) => println!("{holder} is in charge of {scope} until {}", store::ymd(h)),
                    None => println!(
                        "{holder} is in charge of {scope}, with no end date — `canon overdue` will never mention it"
                    ),
                }
                // Who granted is on the line, because it is not always who
                // you think: with `CANON_ACTOR` unset it is the git name.
                println!("  given by {}", act.actor);
                println!("  to take it back:  canon withdraw {holder} {scope} -m \"<why>\"");
            });
            if code != 0 || holder == act.actor {
                return code;
            }
            // A founder who grants somebody else, under a name that is not
            // their own, has seated that person and not themselves — and
            // every act they write from the next second on is a proposal
            // waiting on the holder. Neither the grant nor the next add
            // said so; this does, on the act that caused it.
            let now = store::now();
            let seated = store::read(&d).map(|l| l.derive_at(now)).is_ok_and(|c| {
                c.who_decides(&scope, now)
                    .iter()
                    .any(|g| g.actor == act.actor)
            });
            if !seated {
                println!(
                    "  {} is not in charge of {scope}. From the next second, their writes here are",
                    act.actor
                );
                println!(
                    "  proposals waiting on {holder} — `canon give {} {scope}` if that is wrong.",
                    act.actor
                );
            }
            code
        }
        Err(e) => fail(e),
    }
}

/// `by` grants `holder` standing over `scope`: the grant act, written.
///
/// **The one place a grant is written.** `canon grant` and `canon review`,
/// when a person takes standing over a scope nobody holds, both come here;
/// whether it took is the fold's answer, read with [`crate::cmds::took`].
pub fn give(
    d: &std::path::Path,
    by: &str,
    holder: &str,
    scope: &Scope,
    horizon: Option<i64>,
    rationale: &str,
) -> Result<Act, String> {
    crate::cmds::write_as(
        d,
        ActKind::Grant {
            holder: holder.to_string(),
            scope: scope.clone(),
            horizon,
            rationale: rationale.to_string(),
        },
        by,
    )
}

/// Step back from a scope, or stand someone down from one.
///
/// The same verb serves both, deliberately. People leave a house in stages —
/// stop hosting, stop cooking, stop coming — and those stages are exits from
/// SCOPES. Recording them makes the signal legible without demanding a
/// confrontation from someone who is already disengaging.
pub fn withdraw(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon withdraw <actor> <area> [-m \"<why>\"] [--ask \"<question>\"]");
    }
    if let Some(e) = bare_actor(pos[0], "withdraw") {
        return fail(e);
    }
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not an area", pos[1]));
    };
    let ask = flag(args, "--ask").filter(|q| !q.trim().is_empty());
    if ask.is_some() && pos[0] != store::actor() {
        return fail(format!(
            "--ask leaves your own question as you go; {} is not you, and nothing was written",
            pos[0]
        ));
    }
    let (d, _, _) = or_fail!(load());
    match write(
        &d,
        ActKind::Withdraw {
            holder: pos[0].to_string(),
            scope: scope.clone(),
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        // Standing somebody else down takes standing over the scope, and
        // the fold said so while this printed success anyway.
        Ok(act) => match crate::cmds::governed(&d, &act.id, || {
            println!(
                "{} is no longer in charge of {scope}, or anything under it",
                pos[0]
            )
        }) {
            0 => ask_on_the_way_out(&d, ask),
            refused => refused,
        },
        Err(e) => fail(e),
    }
}

/// The question a leaver never raised, written by `anonymous`.
///
/// **The thing a leaver knows is the thing nobody else will ever learn.** By
/// the time somebody is going, raising it costs a confrontation they have no
/// reason to accept, so `withdraw <you> <scope> --ask` leaves it behind
/// without a name. `Question` is exempt from the attribution check (asking is
/// not adjudicating), so it is no unattended decision.
///
/// **The unattribution is thin and this says so.** It lands beside a
/// withdrawal that carries a name, in the same second; a tool that implied it
/// was untraceable would be lying about something that could cost somebody.
fn ask_on_the_way_out(d: &std::path::Path, ask: Option<&str>) -> i32 {
    let Some(question) = ask else { return 0 };
    let act = Act::new(
        ActKind::Question {
            text: question.to_string(),
            proposal: None,
        },
        store::now(),
        canon_core::ANONYMOUS,
    );
    match store::append(d, &act) {
        Ok(()) => {
            println!("{}  ? {question}", act.id);
            println!("  recorded without your name; it sits next to your withdrawal in the log, so treat the anonymity as thin.");
            0
        }
        Err(e) => fail(e),
    }
}

/// Put a commitment in a scope.
pub fn scoped(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon area <id> <area>");
    }
    let (d, _, canon) = or_fail!(load());
    let id = or_fail!(crate::explain::resolve(&canon, pos[0]));
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not an area", pos[1]));
    };
    match write(
        &d,
        ActKind::Scoped {
            commitment: id.clone(),
            scope: scope.clone(),
        },
    ) {
        Ok(_) => {
            println!("{id} is in {scope}");
            0
        }
        Err(e) => fail(e),
    }
}

// ── canon position ──────────────────────────────────────────

/// Take a position. A vote, an objection, a second — one shape.
///
/// Kept, and kept out of the long help: positions on an action are
/// `canon object --about` and `canon approve --about` now, the verbs a
/// house already knows, and somebody's script still says `position`.
pub fn position(args: &[String]) -> i32 {
    eprintln!("`canon position` is `canon object --about` / `canon approve --about` now");
    let pos = positionals(args);
    if pos.is_empty() {
        return fail(
            "usage: canon position \"<about>\" --against|--toward -m \"<why>\" [--citing <id>]",
        );
    }
    let (against, toward) = (has(args, "--against"), has(args, "--toward"));
    if against == toward {
        // Refuse rather than pick. A position whose direction we guessed is
        // a position nobody took.
        return fail("say which way: --against or --toward, exactly one");
    }
    let Some(because) = flag(args, "-m").filter(|b| !b.trim().is_empty()) else {
        return fail(
            "position requires -m \"<why>\" — a bare no is an assertion, and this tool \
             exists to replace assertions with citations",
        );
    };
    take(args, pos[0], toward, because)
}

/// A position on `about`, citing `--citing` when given: what `position`,
/// `object --about` and `approve --about` write, and what a live check
/// then counts under that subject.
pub fn take(args: &[String], about: &str, toward: bool, because: &str) -> i32 {
    let (d, _, canon) = or_fail!(load());
    let citing = match flag(args, "--citing") {
        None => None,
        Some(needle) => match crate::explain::resolve(&canon, needle) {
            Ok(i) => Some(i),
            Err(e) => return fail(e),
        },
    };
    match crate::cmds::answer(&d, &store::actor(), about, citing.clone(), toward, because) {
        Ok(act) => {
            let way = if toward { "toward" } else { "against" };
            println!("{}  {way} \"{about}\"", act.id);
            if let Some(c) = citing {
                println!("  citing {c}");
            }
            0
        }
        Err(e) => fail(e),
    }
}

// ── canon horizon / canon overdue ───────────────────────────

/// Say when something should be looked at again.
pub fn horizon(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon revisit <act-id> <YYYY-MM-DD> [-m \"<why>\"]");
    }
    let Some(at) = canon_core::date::parse_ymd(pos[1]) else {
        return fail(format!("`{}` is not a date — YYYY-MM-DD", pos[1]));
    };
    let (d, log, _) = or_fail!(load());
    // Any act may carry a horizon, not just a commitment: a grant, an
    // accepted contradiction and a question are all things somebody defers.
    let hits: Vec<&canon_core::Act> = log
        .acts()
        .iter()
        .filter(|a| a.id.as_str().starts_with(pos[0]))
        .collect();
    let target = match hits.len() {
        1 => hits[0].id.clone(),
        0 => return fail(format!("no act matching `{}`", pos[0])),
        n => {
            return fail(format!(
                "`{}` matches {n} acts — use more characters",
                pos[0]
            ))
        }
    };
    match write(
        &d,
        ActKind::Horizon {
            target: target.clone(),
            at,
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(_) => {
            println!("{target} comes back around {}", store::ymd(at));
            println!("  `canon overdue` will surface it after that");
            0
        }
        Err(e) => fail(e),
    }
}

/// What has gone past its date.
///
/// **The closure loop.** Everything else here is additive — you assert, you
/// grant, you accept a contradiction — and a body of commitments nobody ever
/// subtracts from keeps reading as authoritative long after it stopped being
/// true. This is the cheapest defense available, and the difference between
/// deferring something and burying it.
pub fn overdue(args: &[String]) -> i32 {
    let (_, _, canon) = or_fail!(load());
    let now = store::now();
    let due = canon.overdue(now);
    if has(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&due).unwrap_or_default());
        return 0;
    }
    if due.is_empty() {
        println!("nothing overdue.");
    }
    for o in &due {
        let when = store::ymd(o.due);
        match &o.what {
            canon_core::Due::Horizon { rationale } => {
                let what = canon
                    .get(&o.target)
                    .map(|c| c.text().to_string())
                    .or_else(|| canon.question(&o.target).map(|q| format!("? {}", q.text)))
                    .unwrap_or_else(|| o.target.to_string());
                println!("{when}  {}  {what}", o.target);
                if !rationale.is_empty() {
                    println!("          {rationale}");
                }
            }
            canon_core::Due::Revisit { other, rationale } => {
                println!(
                    "{when}  {}\n          carried against {}",
                    crate::explain::named(&canon, &o.target),
                    crate::explain::named(&canon, other)
                );
                println!("          \"{rationale}\"");
            }
            canon_core::Due::Standing { holder, scope } => {
                println!("{when}  {holder} is no longer in charge of {scope}");
                // The Ostrom-4 residue: standing that lapsed and nobody
                // renewed is exactly what a monitor's accountability looks
                // like when it works.
                if canon.who_decides(scope, now).is_empty() {
                    println!("          nobody holds it now — `canon who {scope}`");
                }
            }
        }
    }
    if !due.is_empty() {
        println!("\n{} overdue", due.len());
    }
    // Absence reported, never defaulted: a date nobody can read is a real
    // intention with an unreadable deadline, and it is neither overdue nor
    // absent.
    let unreadable = canon.unreadable_dates();
    if !unreadable.is_empty() {
        eprintln!(
            "\nwarning: {} revisit date(s) are not dates and were not judged:",
            unreadable.len()
        );
        for (id, raw) in &unreadable {
            eprintln!("  {id}  \"{raw}\"");
        }
    }
    if let Some(note) = crate::cmds::carried_note(&canon) {
        eprintln!("\n{note}");
    }
    0
}

// ── canon silence ───────────────────────────────────────────

/// Leave something unwritten, on purpose.
///
/// The third state between "written" and "missing". Without it every
/// unwritten norm reads as a gap and every gap as an invitation to
/// legislate — which is how making a place legible destroys the practical
/// local knowledge it was running on.
pub fn silence(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(about) = pos.first() else {
        return fail(
            "usage: canon silence \"<subject>\" -m \"<what leaving it unwritten protects>\"",
        );
    };
    // Required, like `accept`'s: a silence you keep on purpose must say what
    // it protects, or it cannot be told apart from having forgotten.
    let Some(rationale) = flag(args, "-m").filter(|r| !r.trim().is_empty()) else {
        return fail(
            "silence requires -m \"<reason>\" — an unwritten norm with no reason is \
             indistinguishable from a gap",
        );
    };
    let (d, _, _) = or_fail!(load());
    match write(
        &d,
        ActKind::Silence {
            about: (*about).to_string(),
            rationale: rationale.to_string(),
        },
    ) {
        Ok(act) => {
            println!("{}  unwritten on purpose: \"{about}\"", act.id);
            println!("  {rationale}");
            println!("  `canon check --about \"{about}\"` will say so rather than call it a gap");
            0
        }
        Err(e) => fail(e),
    }
}

// ── canon voice ─────────────────────────────────────────────

/// What somebody raised, and what came of it.
///
/// **Hirschman's loyalty mechanism, made answerable.** Voice is only rational
/// if it works, and whether it has worked for YOU is exactly what a person
/// cannot check from memory and will not ask about out loud. Everything shown
/// is something that person put into the log themselves; nothing here was
/// observed about anybody.
pub fn voice(args: &[String]) -> i32 {
    let pos = positionals(args);
    let who = match pos.first() {
        Some(a) => (*a).to_string(),
        None => store::actor(),
    };
    let (_, log, canon) = or_fail!(load());
    let v = canon.voice_of(&who);
    // Standing this actor was given, and rulings they made on pairs. Neither
    // is something the actor "put in" — a grant is somebody else's act — but
    // both are the answer to "what is this member's record", and for an agent
    // that question is the whole reason to ask. Grants as the fold kept them,
    // so one given without standing, or ended by a withdrawal over a wider
    // scope, reads as the house reads it.
    let now = store::now();
    let held: Vec<String> = canon
        .grants()
        .iter()
        .filter(|g| g.actor == who)
        .map(|g| {
            let end = match (g.withdrawn_at, g.horizon) {
                (Some(w), _) => format!(", ended {}", store::ymd(w)),
                (None, Some(h)) if g.lapsed(now) => format!(" until {}, lapsed", store::ymd(h)),
                (None, Some(h)) => format!(" until {}", store::ymd(h)),
                (None, None) => ", no end date".to_string(),
            };
            let by = log
                .acts()
                .iter()
                .find(|a| a.id == g.act)
                .map_or("", |a| &a.actor);
            format!(
                "  over {}, granted by {} on {}{end}",
                g.scope,
                crate::explain::person(by),
                store::ymd(g.granted_at)
            )
        })
        .collect();
    let mut rulings: Vec<String> = Vec::new();
    for act in log.acts().iter() {
        match &act.kind {
            canon_core::ActKind::Dismiss { a, b, rationale } if act.actor == who => {
                let outside = canon.ungoverned().iter().any(|(x, _)| x == &act.id);
                // The verdict on one line, the two rules named under it, and
                // what became of it under those.
                rulings.push(format!(
                    "  said these two do not conflict, {}{}\n    {}\n    {}{}",
                    store::ymd(act.ts_unix),
                    if rationale.is_empty() {
                        String::new()
                    } else {
                        format!(" — {rationale}")
                    },
                    crate::explain::named(&canon, a),
                    crate::explain::named(&canon, b),
                    if outside {
                        "\n    not applied: outside what they are in charge of".to_string()
                    } else {
                        overruled(&log, &canon, a, b, act.ts_unix)
                    }
                ));
            }
            canon_core::ActKind::Accept {
                a, b, rationale, ..
            } if act.actor == who => {
                rulings.push(format!(
                    "  carried the first against the second, {} — {rationale}\n    {}\n    {}{}",
                    store::ymd(act.ts_unix),
                    crate::explain::named(&canon, a),
                    crate::explain::named(&canon, b),
                    overruled(&log, &canon, a, b, act.ts_unix)
                ));
            }
            _ => {}
        }
    }
    if v.is_empty() && held.is_empty() && rulings.is_empty() {
        println!("{who} has not put anything in this canon.");
        return 0;
    }
    println!("{who}");
    if !held.is_empty() {
        println!("\nin charge {} time(s):", held.len());
        for h in &held {
            println!("{h}");
        }
    }
    if !v.asked.is_empty() {
        println!(
            "\nasked {} question(s), {} answered:",
            v.asked.len(),
            v.answered()
        );
        for q in &v.asked {
            let fate = match &q.status {
                canon_core::Status::Active => "still open".to_string(),
                canon_core::Status::Superseded { by } => format!("answered by {by}"),
                canon_core::Status::Retracted { .. } => "withdrawn".to_string(),
                canon_core::Status::Proposed { .. } | canon_core::Status::Refused { .. } => {
                    "still open".to_string()
                }
            };
            println!("  {}  ? {}  ({fate})", q.id, q.text);
        }
    }
    if !v.positions.is_empty() {
        println!("\ntook {} position(s):", v.positions.len());
        for p in &v.positions {
            let way = match p.position.pull {
                canon_core::Pull::Against => "against",
                canon_core::Pull::Toward => "toward",
            };
            // `about` is a proposal's id when the position ratifies one, and
            // a subject in words when it argues about one. Name the rule.
            let about = match canon_core::ActId::from_raw(p.about.clone()) {
                id if canon.get(&id).is_some() => crate::explain::named(&canon, &id),
                _ => format!("\"{}\"", p.about),
            };
            println!("  {way} {about} — {}", p.position.because);
        }
    }
    if !rulings.is_empty() {
        println!("\nruled on {} pair(s):", rulings.len());
        for r in &rulings {
            println!("{r}");
        }
    }
    if !v.decided.is_empty() {
        println!("\ndecided {} time(s):", v.decided.len());
        for r in &v.decided {
            println!("  \"{}\" -> {}", r.about, r.authority);
        }
    }
    if !v.silences.is_empty() {
        println!("\nleft {} thing(s) unwritten on purpose:", v.silences.len());
        for s in &v.silences {
            println!("  \"{}\" — {}", s.about, s.rationale);
        }
    }
    // The number that answers the question people actually have.
    println!(
        "\n{} of {} question(s) went somewhere",
        v.answered(),
        v.asked.len()
    );
    0
}

/// Was a ruling on this pair later replaced by somebody else's? The current
/// disposition in the canon is the last word; if it was written after this
/// act, name who wrote it. For an agent's record this is the line that
/// matters: not what it said, but whether the house let it stand.
fn overruled(
    log: &canon_core::Log,
    canon: &canon_core::Canon,
    a: &canon_core::ActId,
    b: &canon_core::ActId,
    at: i64,
) -> String {
    // A pair can carry two dispositions in the fold — the dismissal and the
    // later acceptance — so the current one is the latest, not the first.
    let Some(c) = canon
        .conflicts()
        .iter()
        .filter(|c| c.is_pair(a, b))
        .max_by_key(|c| c.at)
    else {
        return String::new();
    };
    if c.at <= at {
        return String::new();
    }
    let later = log.acts().iter().rfind(|x| {
        x.ts_unix > at
            && match &x.kind {
                canon_core::ActKind::Accept { a: p, b: q, .. }
                | canon_core::ActKind::Dismiss { a: p, b: q, .. } => c.is_pair(p, q),
                _ => false,
            }
    });
    match (later, &c.disposition) {
        (Some(x), canon_core::Disposition::Tolerated { .. }) => {
            format!(
                "\n    overruled by {}, {}: carried knowingly",
                x.actor,
                store::ymd(x.ts_unix)
            )
        }
        (Some(x), canon_core::Disposition::Dismissed { .. }) => {
            format!(
                "\n    overruled by {}, {}: not a conflict",
                x.actor,
                store::ymd(x.ts_unix)
            )
        }
        _ => String::new(),
    }
}

// ── canon second ────────────────────────────────────────────

/// Add your weight to a question, without your name.
///
/// **Exit has a pre-signal and voice needs one too.** A question nobody else
/// is seen to share reads as one person's complaint, and the cost of being
/// seen to share it is the same cost that kept the asker quiet. A second says
/// "me too" without saying who, so the group can tell which open questions
/// are widely held.
///
/// **It gates attention, never an outcome.** A second is a `position` toward
/// a question, by `anonymous`. Ratification counts positions on proposals by
/// people who hold the scope, and a question is not a proposal, so nothing a
/// second does can carry or block anything; it only moves a count in `open`.
///
/// **People only.** An agent may ask and cite, but a second is weight, and
/// weight from a process that can run a thousand times is not a second.
pub fn second(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(needle) = pos.first() else {
        return fail("usage: canon second <question-id>");
    };
    let me = store::actor();
    if canon_core::Person::of(&me).is_none() {
        return fail(format!(
            "{me} cannot second a question — agents draft, ask and cite; seconding is a person's"
        ));
    }
    let (d, log, canon) = or_fail!(load());
    let id = or_fail!(crate::explain::resolve_question(&canon, needle));
    // `pick` falls back to an inherited commitment's upstream id, which is
    // right for every other resolver and wrong here: weight on a commitment
    // is a vote.
    let Some(q) = canon.question(&id) else {
        return fail(format!("no question matching `{needle}`"));
    };
    if q.status != canon_core::Status::Active {
        return fail(format!(
            "{id} is no longer open — it has been answered or withdrawn"
        ));
    }
    let act = Act::new(
        ActKind::Position {
            about: id.as_str().to_string(),
            citing: None,
            pull: canon_core::Pull::Toward,
            because: "seconded".into(),
        },
        store::now(),
        canon_core::ANONYMOUS,
    );
    // Two seconds on one question in one second are the same line, and the
    // log keeps one copy of a line. Appending it anyway would report a second
    // that never counts.
    if log.acts().iter().any(|a| a.id == act.id) {
        return fail(format!(
            "someone seconded {id} in the same second — run it again"
        ));
    }
    match store::append(&d, &act) {
        Ok(()) => {
            let n = canon.seconds(&id) + 1;
            println!("{id}  seconded ({n} second(s))");
            println!("  recorded without your name.");
            println!(
                "  its time, and the commit that carries it, can point to you, so treat the \
                 anonymity as thin."
            );
            0
        }
        Err(e) => fail(e),
    }
}
