// SPDX-License-Identifier: AGPL-3.0-or-later
//! The governance verbs — who decides, under what rule, and what was decided.
//!
//! All model-free. Every command here reads or appends an act and folds; none
//! of them touches an endpoint, which is what makes a governance replay
//! possible and is the single most important fact about this layer.

use canon_core::{Act, ActKind, Authority, Outcome, Policy as _, Rule, Scope};

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
                    crate::cmds::report_governed(&d, &act.id)
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

// ── canon ratification ──────────────────────────────────────

/// How a proposal in a scope becomes a rule. See `canon_core::ratify`.
///
/// Setting it is gated in the fold by standing over the scope — the level
/// above decides how a level makes rules — so a `set` by somebody without
/// standing is written, not applied, and says so.
pub fn ratification(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("set") => {
            let Some(raw) = pos.get(1) else {
                return fail(
                    "usage: canon ratification set <standing|joint:a,b|threshold:n/m|consent:Nd> \
                     [--scope s] [-m \"<how it reads>\"]",
                );
            };
            let Some(rule) = canon_core::Ratify::parse(raw) else {
                return fail(format!(
                    "`{raw}` is not a ratification rule — standing, joint:human:a,human:b, \
                     threshold:2/1, or consent:7d"
                ));
            };
            let scope = match scope_from(args) {
                Ok(s) => s,
                Err(e) => return fail(e),
            };
            let (d, _, _) = match load() {
                Ok(v) => v,
                Err(e) => return fail(e),
            };
            let own = flag(args, "-m").filter(|m| !m.trim().is_empty());
            let text = own.map_or_else(|| rule.prose(), str::to_string);
            match write(
                &d,
                ActKind::Ratification {
                    text: text.clone(),
                    rule: rule.clone(),
                    scope: scope.clone(),
                },
            ) {
                Ok(act) => {
                    println!("{}  {}", act.id, rule.name());
                    println!("  {text}");
                    match &scope {
                        Some(s) => println!("  how rules are made in {s} and everything under it"),
                        None => println!("  how rules are made in this canon"),
                    }
                    crate::cmds::report_governed(&d, &act.id)
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
                    serde_json::to_string_pretty(&canon.ratifications).unwrap_or_default()
                );
                return 0;
            }
            if canon.ratifications.is_empty() {
                println!("no ratification rule adopted. proposals become rules by what shipped:");
                println!("  {}", canon_core::Ratify::Standing.name());
                println!("  {}", canon_core::Ratify::Standing.prose());
                println!(
                    "\n  adopt one:  canon ratification set joint:human:a,human:b --scope house.kitchen"
                );
                return 0;
            }
            for r in &canon.ratifications {
                match &r.scope {
                    Some(s) => println!("{}  {}  over {s}", r.act, r.rule.name()),
                    None => println!("{}  {}  (whole canon)", r.act, r.rule.name()),
                }
                println!("  {}", r.text);
                println!("  adopted {} by {}", store::ymd(r.at), r.actor);
            }
            0
        }
        Some(other) => fail(format!(
            "unknown ratification command `{other}` — expected set or show"
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
            crate::cmds::report_governed(&d, &act.id)
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
        Ok(act) => {
            match horizon {
                Some(h) => println!("{} holds {scope} until {}", pos[0], store::ymd(h)),
                None => println!(
                    "{} holds {scope} with no end — `canon overdue` will never mention it",
                    pos[0]
                ),
            }
            crate::cmds::report_governed(&d, &act.id)
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
    let (d, _, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
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
    let (_, log, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let v = canon.voice_of(&who);
    // Standing this actor was given, and rulings they made on pairs. Neither
    // is something the actor "put in" — a grant is somebody else's act — but
    // both are the answer to "what is this member's record", and for an agent
    // that question is the whole reason to ask.
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let mut held: Vec<String> = Vec::new();
    let mut rulings: Vec<String> = Vec::new();
    for act in log.acts().iter() {
        match &act.kind {
            canon_core::ActKind::Grant { holder, scope, horizon, .. } if *holder == who => {
                let withdrawn = log.acts().iter().any(|w| {
                    matches!(&w.kind, canon_core::ActKind::Withdraw { holder: h, scope: s, .. }
                        if *h == who && s == scope && w.ts_unix >= act.ts_unix)
                });
                let end = match (withdrawn, horizon) {
                    (true, _) => ", since withdrawn".to_string(),
                    (false, Some(h)) if *h < now => format!(" until {}, lapsed", store::ymd(*h)),
                    (false, Some(h)) => format!(" until {}", store::ymd(*h)),
                    (false, None) => ", no end date".to_string(),
                };
                held.push(format!(
                    "  over {scope}, granted by {} on {}{end}",
                    act.actor,
                    store::ymd(act.ts_unix)
                ));
            }
            canon_core::ActKind::Dismiss { a, b, rationale } if act.actor == who => {
                let outside = canon.ungoverned.iter().any(|(x, _)| x == &act.id);
                rulings.push(format!(
                    "  said {a} and {b} do not conflict, {}{}{}",
                    store::ymd(act.ts_unix),
                    if rationale.is_empty() { String::new() } else { format!(" — {rationale}") },
                    if outside {
                        "\n    not applied: outside their standing".to_string()
                    } else {
                        overruled(&log, &canon, a, b, act.ts_unix)
                    }
                ));
            }
            canon_core::ActKind::Accept { a, b, rationale, .. } if act.actor == who => {
                rulings.push(format!(
                    "  carried {a} against {b}, {} — {rationale}{}",
                    store::ymd(act.ts_unix),
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
        println!("\nheld standing {} time(s):", held.len());
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
            println!("  {way} \"{}\" — {}", p.about, p.position.because);
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
fn overruled(log: &canon_core::Log, canon: &canon_core::Canon, a: &canon_core::ActId, b: &canon_core::ActId, at: i64) -> String {
    // A pair can carry two dispositions in the fold — the dismissal and the
    // later acceptance — so the current one is the latest, not the first.
    let Some(c) = canon.conflicts.iter().filter(|c| c.is_pair(a, b)).max_by_key(|c| c.at) else {
        return String::new();
    };
    if c.at <= at {
        return String::new();
    }
    let later = log
        .acts()
        .iter()
        .filter(|x| x.ts_unix > at)
        .filter(|x| match &x.kind {
            canon_core::ActKind::Accept { a: p, b: q, .. }
            | canon_core::ActKind::Dismiss { a: p, b: q, .. } => c.is_pair(p, q),
            _ => false,
        })
        .last();
    match (later, &c.disposition) {
        (Some(x), canon_core::Disposition::Tolerated { .. }) => {
            format!("\n    overruled by {}, {}: carried knowingly", x.actor, store::ymd(x.ts_unix))
        }
        (Some(x), canon_core::Disposition::Dismissed { .. }) => {
            format!("\n    overruled by {}, {}: not a conflict", x.actor, store::ymd(x.ts_unix))
        }
        _ => String::new(),
    }
}

// ── canon leave ─────────────────────────────────────────────

/// Step out of a scope, and leave the question behind.
///
/// **The thing a leaver knows is the thing nobody else will ever learn.** By
/// the time somebody is going, raising it costs them a confrontation they have
/// no reason to accept — so it goes unsaid, and the group loses its single
/// best-informed critique at the exact moment it is offered for free.
///
/// This writes two acts: the withdrawal, which is the pre-exit signal read as
/// a first-class move rather than as an absence, and an unattributed question.
///
/// **The unattribution is thin and this says so.** The question carries no
/// name, but it lands in the log beside a withdrawal that does, in the same
/// second. That is worth having anyway — a norm of asking on the way out is
/// worth more than the anonymity — but a tool that implied it was untraceable
/// would be lying about something that could cost somebody.
pub fn leave(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(raw) = pos.first() else {
        return fail("usage: canon leave <scope> [-m \"<the question you never raised>\"]");
    };
    let Some(scope) = Scope::new(raw) else {
        return fail(format!("`{raw}` is not a scope"));
    };
    let (d, _, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let me = store::actor();
    if let Err(e) = write(
        &d,
        ActKind::Withdraw {
            holder: me.clone(),
            scope: scope.clone(),
            rationale: flag(args, "--why").unwrap_or_default().to_string(),
        },
    ) {
        return fail(e);
    }
    println!("{me} no longer holds {scope}");
    let Some(question) = flag(args, "-m").filter(|q| !q.trim().is_empty()) else {
        println!("\n  the thing you know and nobody else will learn:");
        println!("    canon leave {scope} -m \"<it>\"   (recorded without your name)");
        return 0;
    };
    // Written by `anonymous`, so the record carries no name. `Question` is
    // exempt from the attribution check by design — asking is not
    // adjudicating — so this does not show up as an unattended decision.
    let act = Act::new(
        ActKind::Question {
            text: question.to_string(),
            proposal: None,
        },
        store::now(),
        "anonymous",
    );
    match store::append(&d, &act) {
        Ok(()) => {
            println!("{}  ? {question}", act.id);
            println!("  recorded without your name.");
            println!(
                "  it sits next to your withdrawal in the log, so treat the anonymity as thin."
            );
            0
        }
        Err(e) => fail(e),
    }
}
