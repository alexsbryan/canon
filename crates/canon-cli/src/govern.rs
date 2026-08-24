// SPDX-License-Identifier: AGPL-3.0-or-later
//! The governance verbs — who decides, under what rule, and what was decided.
//!
//! All model-free. Every command here reads or appends an act and folds; none
//! of them touches an endpoint, which is what makes a governance replay
//! possible and is the single most important fact about this layer.

use canon_core::{ActKind, Authority, Outcome, Policy as _, Rule, Scope};

use crate::cmds::{fail, flag, has, load, positionals, write};
use crate::store;

/// Parse a rule from the command line.
///
/// The base rule is a positional word; the modifiers are flags that WRAP it,
/// in a fixed order, so `--cautious --entrench principle` and the reverse
/// produce the same rule. Two spellings of one policy would be two policies
/// as far as `policy_for` is concerned, and the ledger would carry whichever
/// order somebody typed.
fn rule_from(args: &[String], base: &str) -> Result<Rule, String> {
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
    if let Some(ladder) = flag(args, "--graduated") {
        let rungs: Result<Vec<Authority>, String> = ladder.split(',').map(authority).collect();
        let rungs = rungs?;
        if rungs.is_empty() {
            return Err("--graduated takes rungs, e.g. ask-one,ask-panel,refuse".into());
        }
        rule = Rule::Graduated {
            ladder: rungs,
            base: Box::new(rule),
        };
    }
    if let Some(ranks) = flag(args, "--entrench") {
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
    match flag(args, "--scope") {
        None => Ok(None),
        Some(raw) => Scope::new(raw)
            .map(Some)
            .ok_or_else(|| format!("`{raw}` is not a scope: dotted path, no empty segments")),
    }
}

// ── canon policy ────────────────────────────────────────────

pub fn policy(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("set") => {
            let Some(base) = pos.get(1) else {
                return fail(
                    "usage: canon policy set <default|consent|threshold|supermajority|subsidiarity> \
                     [--objections n] [--of n/d] [--graduated a,b,c] [--entrench rank] [--cautious] \
                     [--scope s] [-m \"<how it reads>\"]",
                );
            };
            let rule = match rule_from(args, base) {
                Ok(r) => r,
                Err(e) => return fail(e),
            };
            let scope = match scope_from(args) {
                Ok(s) => s,
                Err(e) => return fail(e),
            };
            let (d, _, _) = match load() {
                Ok(v) => v,
                Err(e) => return fail(e),
            };
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
                Ok(act) => {
                    println!("{}  {}", act.id, rule.name());
                    println!("  {text}");
                    if own.is_none() {
                        println!("  (our words, not yours — `-m` says it in the house's own)");
                    }
                    match scope {
                        Some(s) => println!("  governs {s} and everything under it"),
                        None => println!("  governs this canon"),
                    }
                    0
                }
                Err(e) => fail(e),
            }
        }
        Some("show") | None => {
            let (_, _, canon) = match load() {
                Ok(v) => v,
                Err(e) => return fail(e),
            };
            if has(args, "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&canon.policies).unwrap_or_default()
                );
                return 0;
            }
            if canon.policies.is_empty() {
                // Absence reported as absence. A canon that adopted nothing
                // is still governed by something, and it should know which.
                println!("no policy adopted. this canon decides by what shipped:");
                println!("  {}", Rule::Default.name());
                println!("  {}", Rule::Default.prose());
                println!("\n  adopt one:  canon policy set consent -m \"<how it reads here>\"");
                return 0;
            }
            for p in &canon.policies {
                match &p.scope {
                    Some(s) => println!("{}  {}  over {s}", p.act, p.rule.name()),
                    None => println!("{}  {}  (whole canon)", p.act, p.rule.name()),
                }
                println!("  {}", p.text);
                println!("  adopted {} by {}", store::ymd(p.at), p.actor);
            }
            println!("\n{} policy act(s)", canon.policies.len());
            0
        }
        Some(other) => fail(format!(
            "unknown policy command `{other}` — expected set or show"
        )),
    }
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
            "usage: canon decide \"<what it was about>\" --outcome <supported|conflicts|unaddressed> \
             --authority <act|notify|ask-one|ask-panel|refuse> -m \"<what was decided>\"",
        );
    };
    let (Some(o), Some(a)) = (flag(args, "--outcome"), flag(args, "--authority")) else {
        return fail(
            "decide requires --outcome and --authority — `canon check --json` prints both, \
             and defaulting either would record a decision nobody made",
        );
    };
    let (outcome, authority) = match (outcome(o), authority(a)) {
        (Ok(o), Ok(a)) => (o, a),
        (Err(e), _) | (_, Err(e)) => return fail(e),
    };
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let prior = canon.prior_decisions(about).len();
    match write(
        &d,
        ActKind::Decided {
            about: (*about).to_string(),
            outcome,
            authority,
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(act) => {
            println!("{act_id}  decided about \"{about}\"", act_id = act.id);
            println!("  {} / {authority}", format!("{outcome:?}").to_lowercase());
            // The rung is shown because the ladder is the point: a decision
            // that silently moves the next one up is a decision nobody saw.
            println!("  this is decision {} about it", prior + 1);
            0
        }
        Err(e) => fail(e),
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
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let id = match crate::explain::resolve(&canon, pos[0]) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
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

/// Who may decide this?
///
/// **Answerable without asking a person, and that is the whole point.**
/// Informal power runs on private knowledge of the process: a group where
/// finding out who decides means knowing whom to ask has made that person the
/// gatekeeper. This is the Freeman floor, and it is one query.
pub fn who(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(raw) = pos.first() else {
        return fail("usage: canon who <scope>   e.g. canon who house.kitchen");
    };
    let Some(scope) = Scope::new(raw) else {
        return fail(format!(
            "`{raw}` is not a scope: dotted path, no empty segments"
        ));
    };
    let (_, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
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
        println!("nobody holds standing over `{scope}`.");
        println!("  grant it:  canon grant <actor> {scope}");
        // Lapsed standing is remembered here even though it is not held, so
        // "it used to be Dana and nobody renewed it" is answerable.
        let lapsed: Vec<&canon_core::Grant> = canon
            .grants
            .iter()
            .filter(|g| g.lapsed(now) && g.scope.covers(&scope))
            .collect();
        if !lapsed.is_empty() {
            println!("\n  {} lapsed grant(s) over it:", lapsed.len());
            for g in lapsed {
                println!(
                    "    {} until {}",
                    g.actor,
                    g.horizon.map(store::ymd).unwrap_or_default()
                );
            }
        }
        return 0;
    }
    for g in &deciders {
        let until = match g.horizon {
            Some(h) => format!(" until {}", store::ymd(h)),
            None => String::new(),
        };
        println!("{}  over {}{until}", g.actor, g.scope);
    }
    println!("\n{} with standing, narrowest first", deciders.len());
    println!("decided under: {}", canon.policy_for(Some(&scope)).name());
    0
}

// ── canon grant / withdraw ──────────────────────────────────

pub fn grant(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon grant <actor> <scope> [--horizon YYYY-MM-DD] [-m \"<why>\"]");
    }
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not a scope", pos[1]));
    };
    let horizon = match flag(args, "--horizon").map(canon_core::date::parse_ymd) {
        None => None,
        Some(Some(ts)) => Some(ts),
        Some(None) => return fail("--horizon takes a date like 2026-12-31"),
    };
    let (d, _, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    match write(
        &d,
        ActKind::Grant {
            holder: pos[0].to_string(),
            scope: scope.clone(),
            horizon,
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(_) => {
            match horizon {
                Some(h) => println!("{} holds {scope} until {}", pos[0], store::ymd(h)),
                None => println!(
                    "{} holds {scope} with no end — `canon overdue` will never mention it",
                    pos[0]
                ),
            }
            0
        }
        Err(e) => fail(e),
    }
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
        return fail("usage: canon withdraw <actor> <scope> [-m \"<why>\"]");
    }
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not a scope", pos[1]));
    };
    let (d, _, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    match write(
        &d,
        ActKind::Withdraw {
            holder: pos[0].to_string(),
            scope: scope.clone(),
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(_) => {
            println!("{} no longer holds {scope}, or anything under it", pos[0]);
            0
        }
        Err(e) => fail(e),
    }
}

/// Put a commitment in a scope.
pub fn scoped(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon scope <id> <scope>");
    }
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let id = match crate::explain::resolve(&canon, pos[0]) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not a scope", pos[1]));
    };
    match write(
        &d,
        ActKind::Scoped {
            commitment: id.clone(),
            scope: scope.clone(),
        },
    ) {
        Ok(_) => {
            println!("{id} belongs to {scope}");
            0
        }
        Err(e) => fail(e),
    }
}

// ── canon position ──────────────────────────────────────────

/// Take a position. A vote, an objection, a second — one shape.
pub fn position(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.is_empty() {
        return fail(
            "usage: canon position \"<about>\" --against|--toward -m \"<why>\" [--citing <id>]",
        );
    }
    let (against, toward) = (has(args, "--against"), has(args, "--toward"));
    let pull = match (against, toward) {
        (true, false) => canon_core::Pull::Against,
        (false, true) => canon_core::Pull::Toward,
        // Refuse rather than pick. A position whose direction we guessed is
        // a position nobody took.
        _ => return fail("say which way: --against or --toward, exactly one"),
    };
    let Some(because) = flag(args, "-m").filter(|b| !b.trim().is_empty()) else {
        return fail(
            "position requires -m \"<why>\" — a bare no is an assertion, and this tool \
             exists to replace assertions with citations",
        );
    };
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let citing = match flag(args, "--citing") {
        None => None,
        Some(needle) => match crate::explain::resolve(&canon, needle) {
            Ok(i) => Some(i),
            Err(e) => return fail(e),
        },
    };
    match write(
        &d,
        ActKind::Position {
            about: pos[0].to_string(),
            citing: citing.clone(),
            pull,
            because: because.to_string(),
        },
    ) {
        Ok(act) => {
            let way = if against { "against" } else { "toward" };
            println!("{}  {way} \"{}\"", act.id, pos[0]);
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
        return fail("usage: canon horizon <act-id> <YYYY-MM-DD> [-m \"<why>\"]");
    }
    let Some(at) = canon_core::date::parse_ymd(pos[1]) else {
        return fail(format!("`{}` is not a date — YYYY-MM-DD", pos[1]));
    };
    let (d, log, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
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
    let (_, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
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
                    .map(|c| c.text.clone())
                    .or_else(|| canon.question(&o.target).map(|q| format!("? {}", q.text)))
                    .unwrap_or_else(|| o.target.to_string());
                println!("{when}  {}  {what}", o.target);
                if !rationale.is_empty() {
                    println!("          {rationale}");
                }
            }
            canon_core::Due::Revisit { other, rationale } => {
                println!("{when}  {}  carried against {other}", o.target);
                println!("          \"{rationale}\"");
            }
            canon_core::Due::Standing { holder, scope } => {
                println!("{when}  {holder} no longer holds {scope}");
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
