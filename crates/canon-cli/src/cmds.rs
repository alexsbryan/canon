// SPDX-License-Identifier: AGPL-3.0-or-later
//! The verbs. Every model-free command is here in full; the three that need an
//! endpoint report that plainly rather than degrading into something that
//! looks like an answer.

use std::path::{Path, PathBuf};

use canon_core::{Act, ActId, ActKind, Canon, Log, Policy as _, Status};

use crate::config::{self, Config, Key, API_KEY_ENV};
use crate::profile::Profile;
use crate::store;

// ── plumbing ────────────────────────────────────────────────

/// Does `flag` take a value, so [`positionals`] does not read the value as one?
///
/// Read from the verb table in [`crate::cli`] rather than kept beside it: a
/// list here drifted from the table (it lacked `--carry` and `--continue` and
/// carried two flags no verb takes), and a flag missing from it makes its
/// VALUE look like a positional, which is how `--scope house.kitchen` becomes
/// a proposal called "house.kitchen".
fn takes_value(flag: &str) -> bool {
    crate::cli::VERBS
        .iter()
        .any(|(_, valued, ..)| valued.contains(&flag))
}

pub fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

/// Every value given for a flag that may be repeated, in order.
pub fn flags<'a>(args: &'a [String], name: &str) -> Vec<&'a str> {
    args.iter()
        .enumerate()
        .filter(|(_, a)| *a == name)
        .filter_map(|(i, _)| args.get(i + 1))
        .map(String::as_str)
        .collect()
}

pub fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

pub fn positionals(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if std::mem::take(&mut skip) {
            continue;
        }
        skip = takes_value(a);
        if !skip && !a.starts_with('-') {
            out.push(a.as_str());
        }
    }
    out
}

pub fn dir() -> Result<PathBuf, String> {
    if let Ok(d) = std::env::var("CANON_DIR") {
        return Ok(PathBuf::from(d));
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    store::locate(&cwd)
        .ok_or_else(|| "no canon here. `canon init` to start one, or set CANON_DIR".to_string())
}

pub fn load() -> Result<(PathBuf, Log, Canon), String> {
    let d = dir()?;
    let log = store::read(&d)?;
    // At the wall clock: a consent window that has run out and standing that
    // has lapsed read as such today, whatever the last act's date was.
    let st = canon_institution::legacy(&log, Some(store::now()));
    Ok((d, log, st))
}

/// Where a commitment stands after a write, in the words `list` uses. A
/// write is not a rule until the scope's ratification rule says so, and the
/// person who typed it should hear that from the same command.
pub fn report_status(d: &Path, id: &ActId) {
    let Ok(log) = store::read(d) else { return };
    let canon = log.derive_at(store::now());
    let Some(c) = canon.get(id) else {
        // A change to how a scope decides is a proposal too, judged under
        // the rule it is changing, and the person who set it hears so here.
        if let Some(r) = canon.ratifications().iter().find(|r| r.act == *id) {
            report_verdict(id, &r.verdict);
        }
        // So is a retraction written without standing.
        if let Some(r) = canon.retractions().iter().find(|r| r.act == *id) {
            report_retraction(r);
        }
        // And a change of policy, where the scope's rule reaches policy.
        if let Some(v) = canon
            .pending_policies()
            .iter()
            .chain(canon.policies())
            .find(|p| p.act == *id)
            .and_then(|p| p.verdict.as_ref())
        {
            report_verdict(id, v);
        }
        return;
    };
    match &c.status() {
        // How it became a rule, in a canon somebody holds — the same line
        // `why` prints. In a founding script this is where "nobody held it
        // before this was written; it was open" shows up, on the act it is
        // about, rather than a second later as a proposal nobody expected.
        Status::Active => match canon.ratify(c, store::now()) {
            canon_core::Verdict::Ratified { how, .. } => {
                println!(
                    "{}",
                    crate::wrap::hang("  in force — ", &crate::explain::person_in(&how))
                )
            }
            _ => println!("  in force"),
        },
        Status::Proposed { needs } => {
            say_proposed("  PROPOSED, not yet a rule — needs ", needs, id)
        }
        Status::Refused { by, why, .. } => say_refused(by, why),
        Status::Superseded { .. } | Status::Retracted { .. } => {}
    }
}

fn say_refused(by: &str, why: &str) {
    println!(
        "{}",
        crate::wrap::hang(
            &format!("  REFUSED{}: ", crate::explain::refused_by(by, None, |b| b)),
            &crate::explain::refused_why(by, why)
        )
    );
}

fn say_proposed(head: &str, needs: &str, id: &ActId) {
    println!("{}", crate::wrap::hang(head, needs));
    println!("  approve it:  canon approve {id}");
    println!("  object:      canon object {id} -m \"<why>\"");
}

/// Where a change to a scope's rule stands, in the words `list` uses.
pub fn report_verdict(id: &ActId, v: &canon_core::Verdict) {
    use canon_core::Verdict;
    match v {
        Verdict::Ratified { since, .. } => println!("  in force since {}", store::ymd(*since)),
        Verdict::Proposed { needs, .. } => say_proposed(
            "  PROPOSED, not yet how this area decides — needs ",
            needs,
            id,
        ),
        Verdict::Refused { by, why, .. } => say_refused(by, why),
    }
}

/// Where a retraction written without standing stands, in the words `list`
/// uses.
pub fn report_retraction(r: &canon_core::Retraction) {
    use canon_core::Verdict;
    match &r.verdict {
        Verdict::Ratified { since, how, .. } => println!(
            "{}",
            crate::wrap::hang(
                &format!("  repealed {} since {} — ", r.target, store::ymd(*since)),
                &crate::explain::person_in(how)
            )
        ),
        Verdict::Proposed { needs, .. } => {
            say_proposed("  PROPOSED, not yet repealed — needs ", needs, &r.act)
        }
        Verdict::Refused { by, why, .. } => say_refused(by, why),
    }
}

/// How the fold judged a governance act, read back after the write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Took {
    /// The write happened, but its outcome could not be read.
    Unknown(String),
    /// Applied, by somebody who held standing over what it touched.
    Applied,
    /// Applied only because nothing covering the scope predated it. The
    /// reason, in the fold's words.
    Open(String),
    /// Written, on the record, and not applied. The reason.
    Refused(String),
    /// Written without standing and put to the scope instead of refused —
    /// a retraction, waiting on a holder. What it needs.
    Proposed(String),
}

/// Did a governance act take, and how? The fold refuses a grant, a policy,
/// a ruling or a retraction by somebody without standing over what it
/// touches, and applies one while a scope is still open; the person who
/// typed it should hear which from the same command rather than find it in
/// `list`.
pub fn took(d: &Path, id: &ActId) -> Took {
    let log = match store::read(d) {
        Ok(log) => log,
        Err(e) => return Took::Unknown(e),
    };
    let canon = log.derive_at(store::now());
    if let Some((_, why)) = canon.ungoverned().iter().find(|(x, _)| x == id) {
        return Took::Refused(why.clone());
    }
    if let Some((_, why)) = canon.bootstrap().iter().find(|(x, _)| x == id) {
        return Took::Open(why.clone());
    }
    if let Some(r) = canon.retractions().iter().find(|r| r.act == *id) {
        if let canon_core::Verdict::Proposed { needs, .. } = &r.verdict {
            return Took::Proposed(needs.clone());
        }
    }
    Took::Applied
}

/// Say what the fold said, in that order.
///
/// **A refusal is the headline, and the only line.** Every handler used to
/// print its success line first and ask the fold second, so a refused grant
/// read `agent:claude holds repo.defaults` and then `NOT APPLIED` — and a
/// script or an agent reading the first line took the wrong lesson. The id
/// stays because `undo` needs it. An act that took while open prints its
/// headline and then says so. Returns the exit code.
pub fn announce(t: &Took, id: &ActId, headline: impl FnOnce()) -> i32 {
    match t {
        Took::Unknown(why) => {
            eprintln!("on the record as {id}; outcome unknown: {why}");
            2
        }
        Took::Refused(why) => {
            println!("{}", crate::wrap::hang("NOT APPLIED: ", why));
            println!("  on the record as {id}; somebody in charge has to do it.");
            1
        }
        Took::Open(why) => {
            headline();
            println!("{}", crate::wrap::hang("  applied while open: ", why));
            0
        }
        // A proposal, the way an add by a non-holder is one: not applied,
        // so no success line, and not refused, so it exits as an add does.
        Took::Proposed(needs) => {
            say_proposed("PROPOSED, not yet applied — needs ", needs, id);
            0
        }
        Took::Applied => {
            headline();
            0
        }
    }
}

/// [`took`] then [`announce`]: the one path a gated write reports through.
pub fn governed(d: &Path, id: &ActId, headline: impl FnOnce()) -> i32 {
    announce(&took(d, id), id, headline)
}

/// `--scope <s>` on a write: the commitment is scoped in the same breath, so
/// the ratification rule of that scope applies from the first fold.
fn scope_arg(args: &[String]) -> Result<Option<canon_core::Scope>, String> {
    flag(args, "--area").map(parse_area).transpose()
}

/// An area by its dotted name, or the refusal every verb gives.
pub fn parse_area(raw: &str) -> Result<canon_core::Scope, String> {
    canon_core::Scope::new(raw).ok_or_else(|| format!("`{raw}` is not an area"))
}

pub fn fail(e: impl std::fmt::Display) -> i32 {
    eprintln!("error: {e}");
    2
}

/// A handler's `?`: the value, or the error out through [`fail`] as the exit
/// code. Each call used to be four lines of `match` saying only that.
macro_rules! or_fail {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return $crate::cmds::fail(e),
        }
    };
}
pub(crate) use or_fail;

/// A value as indented JSON on stdout, which is what every `--json` prints.
pub fn print_json<T: serde::Serialize + ?Sized>(v: &T) {
    println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
}

pub fn write(d: &Path, kind: ActKind) -> Result<Act, String> {
    write_as(d, kind, &store::actor())
}

/// [`write`], for a caller that already knows who is acting — `canon ui`
/// decided whose queue to show when it started, and writes as that person.
pub fn write_as(d: &Path, kind: ActKind, actor: &str) -> Result<Act, String> {
    let act = Act::new(kind, store::now(), actor);
    // Signed if its actor has bound a key, refused if it cannot be. A write
    // that would not count is never appended.
    let act = crate::identify::sign_if_bound(&store::read(d)?, act)?;
    store::append(d, &act)?;
    Ok(act)
}

// ── record ──────────────────────────────────────────────────

/// Shared founding behavior. Publish only after both founding acts are written;
/// a failed setup must not leave an apparently usable, ungoverned ledger.
pub fn initialize(
    base: &Path,
    profile: Profile,
    founder: &str,
    plain: bool,
    open: bool,
) -> Result<bool, String> {
    if store::exists(base) {
        return Err(format!("a canon already exists at {}", base.display()));
    }
    std::fs::create_dir_all(base).map_err(|e| e.to_string())?;
    let staging = base.join(".initializing");
    std::fs::create_dir(&staging).map_err(|e| {
        format!(
            "setup at {} is running or interrupted: {e}",
            staging.display()
        )
    })?;
    let result = (|| {
        std::fs::write(staging.join("profile"), format!("{}\n", profile.as_str()))
            .map_err(|e| e.to_string())?;
        store::create(&staging)?;
        // **The batteries, written as the canon's first act.** A change of
        // policy waits like any other change, and a person's own scope is
        // theirs. `standing` keeps the founding sitting what it always was; the
        // knobs bite once the group picks a rule that waits. The act is v4, so
        // a build from before the knobs cannot open this canon and says so;
        // `--plain` founds one it can, with neither knob, which is how every
        // canon before 0.3 began.
        if !plain {
            let kind = ActKind::Ratification {
                text: "Whoever is in charge decides at once. A change to the rulings waits like \
                   any other change, and a person's own area is theirs."
                    .to_string(),
                rule: canon_core::Ratify::Standing,
                scope: None,
                governs: Some(canon_core::Governs::All),
                personal: Some(true),
            };
            write_as(&staging, kind, founder)?;
        }
        // **The founding grant.** A canon whose founder holds it cannot be
        // taken by the first person to write in it: the founder's word is the
        // house's, and everyone else's is a proposal until they hold a scope.
        // An agent cannot seat itself, so its founding waits for a person;
        // `--open` is the old shared start, and writes there are unguarded.
        let granted = if plain || open || canon_core::Person::of(founder).is_none() {
            false
        } else {
            let scope = parse_area(profile.scope())?;
            let kind = ActKind::Grant {
                holder: founder.to_string(),
                scope,
                horizon: None,
                rationale: format!("founded this canon; {founder} is in charge of it"),
            };
            write_as(&staging, kind, founder)?;
            true
        };
        std::fs::rename(staging.join("profile"), base.join("profile"))
            .map_err(|e| e.to_string())?;
        store::publish(&staging, base)?;
        store::ignore_local(base);
        Ok(granted)
    })();
    let _ = std::fs::remove_dir_all(&staging);
    result
}

pub fn init(args: &[String]) -> i32 {
    let profile = or_fail!(Profile::parse(
        flag(args, "--profile").unwrap_or("personal")
    ));
    let base = match std::env::var("CANON_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => or_fail!(std::env::current_dir()).join(store::DIR),
    };
    let plain = has(args, "--plain");
    let open = has(args, "--open");
    let founder = store::actor();
    let granted = or_fail!(initialize(&base, profile, &founder, plain, open));
    // **The whole tool, in three lines, at the moment somebody needs it.**
    // This is the n+1 explanation: if the next person cannot be handed the
    // thing in two sentences it does not spread, and `canon --help` is
    // forty-seven verbs. Everything past these three is opt-in and says so.
    println!(
        "canon initialised at {} ({})",
        base.display(),
        profile.as_str()
    );
    if !plain {
        // Said on the way in, because it is the one thing init decides for
        // the group, and the group can change it with `ratification set`.
        if granted {
            println!(
                "  {founder} is in charge of this canon ({}): what you write is in force.",
                profile.scope()
            );
            println!(
                "  give the others a say: canon give human:<them> {}",
                profile.scope()
            );
            println!(
                "  sign it: canon identify <your key.pub> — so only your key can decide as you"
            );
        } else if open {
            println!("  nobody is in charge of this canon: it is open, and any write takes.");
            println!("  take charge: canon give {founder} {}", profile.scope());
        }
        println!("  whoever is in charge decides at once; to need everyone: canon ratification set unanimous");
        match store::actor().strip_prefix(canon_core::person::PREFIX) {
            Some(name) => {
                println!("  a change to the rulings waits like a rule does, and human:{name}.… is yours alone")
            }
            None => println!("  a change to the rulings waits like a rule does"),
        }
        if canon_core::Person::of(&store::actor()).is_none() {
            println!(
                "  written by {}, so it waits for a person to approve it",
                store::actor()
            );
            println!(
                "  putting you in charge waits too: canon give {} {}",
                store::actor(),
                profile.scope()
            );
        }
    }
    // Padded to a common column, computed rather than typed: the noun
    // changes width by profile, and three lines that almost line up read as
    // sloppier than three that do not try.
    let rows = [
        (
            format!("canon add \"<a {} you already have>\"", profile.noun()),
            "write one down",
        ),
        (
            "canon check \"<something you want to do>\"".to_string(),
            "may I? It names who to ask",
        ),
        ("canon why <id>".to_string(), "what replaced what, and why"),
    ];
    let width = rows
        .iter()
        .map(|(c, _)| c.chars().count())
        .max()
        .unwrap_or(0);
    println!();
    for (cmd, gloss) in &rows {
        println!("  {cmd:<width$}   {gloss}");
    }
    println!();
    println!("that is the whole thing. `canon help all` when you want the rest —");
    println!("who decides what, how you decide, what has gone stale, drawing lots.");
    println!("Profile or area? `canon help profile` and `canon help areas`.");
    0
}

pub fn add(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(text) = pos.first() else {
        return fail(
            "usage: canon add \"<rule>\" [--area <area>] [-m \"<why>\"] [--cite \"<where>\"]",
        );
    };
    let scope = or_fail!(scope_arg(args));
    let (d, _, st) = or_fail!(load());
    // A near-duplicate is a warning, not a refusal: the write still lands,
    // and the person can retract it. The warning is on stderr, so a pipe
    // and `--json` keep their contract. It comes before the new act so the
    // person sees what it resembles while it is still cheap to stop.
    if let Some(like) = crate::similar::nearest(&st, text) {
        eprintln!(
            "warning: this looks like {} ({}): {}",
            like.id,
            like.status,
            crate::explain::one_line(like.text)
        );
        if like.status == "proposed" {
            eprintln!(
                "  if it is the same decision, approve or object to that one; if it changes it, \
                 `canon amend {}`",
                like.id
            );
        } else {
            eprintln!(
                "  if it changes it, `canon amend {}`; if it is the same, do not add a second",
                like.id
            );
        }
    }
    // Why it is a rule, or where it was written: both are the commitment's
    // source, which `why` shows as "from …". The group's repeated want was
    // to keep the neighbours' complaint or the old chat with the rule,
    // instead of retyping it into the text.
    let source = flag(args, "-m")
        .into_iter()
        .chain(flag(args, "--cite"))
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" — ");
    let source = (!source.is_empty()).then_some(source);
    let act = or_fail!(propose(&d, &store::actor(), text, scope, source.as_deref()));
    println!("{}  {}", act.id, text);
    report_status(&d, &act.id);
    0
}

/// Write a commitment, and its scope when it has one: what `add` writes,
/// for `add` and for the page.
pub fn propose(
    d: &Path,
    actor: &str,
    text: &str,
    scope: Option<canon_core::Scope>,
    source: Option<&str>,
) -> Result<Act, String> {
    let kind = ActKind::Assert {
        text: text.to_string(),
        from: None,
        source: source.map(str::to_string),
    };
    write_placed(d, actor, kind, scope)
}

/// Write `kind`, and its area in the same breath when it has one.
fn write_placed(
    d: &Path,
    actor: &str,
    kind: ActKind,
    scope: Option<canon_core::Scope>,
) -> Result<Act, String> {
    let act = write_as(d, kind, actor)?;
    if let Some(s) = scope {
        write_as(
            d,
            ActKind::Scoped {
                commitment: act.id.clone(),
                scope: s,
            },
            actor,
        )
        .map_err(|e| format!("{e}; {} was recorded, but its room was not", act.id))?;
    }
    Ok(act)
}

/// Replace `old` with `text`, keeping the reason: what `supersede` writes,
/// for `supersede` and for the page. The replacement lives where the old
/// rule lived unless `scope` says otherwise, so the same people who
/// ratified the rule ratify its replacement.
pub fn replace(
    d: &Path,
    actor: &str,
    old: &ActId,
    text: &str,
    why: &str,
    scope: Option<canon_core::Scope>,
) -> Result<Act, String> {
    let kind = ActKind::Supersede {
        text: text.to_string(),
        old: vec![old.clone()],
        rationale: why.to_string(),
    };
    write_placed(d, actor, kind, scope)
}

/// Approve a proposal: a position toward it, by name. Whether that makes it
/// a rule is the scope's ratification rule's call, and the answer prints.
/// With `--about`, support an action instead.
pub fn approve(args: &[String]) -> i32 {
    weigh(args, true)
}

/// Object to a proposal. The reason is required: under consent one reasoned
/// objection refuses, and a bare no is not a reason. With `--about`, object
/// to an action instead.
pub fn object(args: &[String]) -> i32 {
    weigh(args, false)
}

/// `approve` and `object`, which differ only in which way they pull.
///
/// **`--about` is a flag, never a fallback.** A mistyped proposal id is
/// still refused, rather than quietly becoming a position on a subject
/// nobody named: with it, the words are the subject a live `check` counts
/// positions under; without it, they are a proposal's id.
fn weigh(args: &[String], approve: bool) -> i32 {
    let (verb, why, done) = match approve {
        true => ("approve", "[-m \"<why>\"]", "approved by"),
        false => ("object", "-m \"<why>\"", "objected to by"),
    };
    let pos = positionals(args);
    let because = flag(args, "-m").unwrap_or("");
    // One or the other, said: an id beside `--about` is two answers to
    // "what is this about", and `--citing` cites for a subject only.
    let needle = match (pos.first(), flag(args, "--about")) {
        (None, Some(about)) => return crate::govern::take(args, about, approve, because),
        (Some(n), None) if flag(args, "--citing").is_none() => *n,
        _ => {
            return fail(format!(
                "usage: canon {verb} <proposal-id> {why}\n       \
                 canon {verb} --about \"<subject>\" {why} [--citing <id>]"
            ))
        }
    };
    let (d, log, st) = or_fail!(load());
    let target = match crate::explain::resolve_proposal(&st, needle) {
        Ok(i) => i,
        Err(e) => {
            return crate::govern::answer_request(&d, (&log, &st), needle, approve, because)
                .unwrap_or_else(|| fail(e))
        }
    };
    let act = or_fail!(answer(
        &d,
        &store::actor(),
        target.as_str(),
        None,
        approve,
        because
    ));
    println!("{}  {done} {}", target, act.actor);
    report_status(&d, &target);
    0
}

/// Approve or object to `about` as `actor`: the position act, written.
///
/// **The one place either is written.** `approve`, `object`, `position`,
/// `canon review` and the page all come here, so "what an approval is" and
/// "an objection needs a reason" have one answer between them rather than
/// five. `about` is a proposal's id or an action's subject.
pub fn answer(
    d: &Path,
    actor: &str,
    about: &str,
    citing: Option<ActId>,
    approve: bool,
    because: &str,
) -> Result<Act, String> {
    let because = because.trim();
    if !approve && because.is_empty() {
        return Err(
            "an objection needs a reason (-m \"<why>\") — one with no reason does not count".into(),
        );
    }
    write_as(
        d,
        ActKind::Position {
            about: about.into(),
            citing,
            pull: if approve {
                canon_core::Pull::Toward
            } else {
                canon_core::Pull::Against
            },
            because: if because.is_empty() {
                "approved".into()
            } else {
                because.into()
            },
        },
        actor,
    )
}

pub fn list(args: &[String]) -> i32 {
    let (d, log, st) = or_fail!(load());
    if has(args, "--json") {
        // The whole derived state, not just the live rows: `list` is the
        // read model, and a caller that wants only the live ones filters on
        // status. Emitting an array of live commitments made the human
        // output's own hint — "canon list --json for detail" about carried
        // contradictions — untrue, because the conflicts were not in it.
        print_json(&crate::render::with_scope_and_rank(&st));
        return 0;
    }
    let profile = Profile::load(&d).unwrap_or_default();
    if has(args, "--markdown") {
        let doc = crate::render::markdown(&st, &crate::lineage::name_of(&d), profile);
        return match (flag(args, "--out"), has(args, "--check")) {
            (None, false) => {
                print!("{doc}");
                0
            }
            (None, true) => fail(
                "`--check` compares against a file — `canon list --markdown --out <file> --check`",
            ),
            (Some(out), true) => match std::fs::read_to_string(out) {
                Ok(had) if had == doc => {
                    println!("{out} is current");
                    0
                }
                // Stale is exit 1, the way a conflict is: a finding about the
                // file, not a usage error.
                Ok(_) => {
                    eprintln!("{out} is stale — `canon list --markdown --out {out}` renders it");
                    1
                }
                Err(e) => fail(format!("{out}: {e}")),
            },
            (Some(out), false) => match std::fs::write(out, &doc) {
                Ok(()) => {
                    println!("wrote {out}");
                    0
                }
                Err(e) => fail(format!("{out}: {e}")),
            },
        };
    }
    let live: Vec<_> = st.active().collect();
    if live.is_empty() && st.proposed().next().is_none() {
        println!(
            "no {} yet. `canon add \"<the first one>\"` to start.",
            profile.nouns()
        );
        crate::govern::unapplied(&log, &st);
        return 0;
    }
    for c in &live {
        println!("{}", crate::wrap::hang(&format!("{}  ", c.id()), c.text()));
    }
    println!("\n{} live", profile.count(live.len()));
    // Proposals are shown with the rules and apart from them: written, by
    // name, and not yet in force. What each one is waiting for is the line
    // that tells the room who has to act.
    let proposed: Vec<_> = st.proposed().collect();
    let retractions: Vec<_> = st
        .retractions()
        .iter()
        .filter(|r| matches!(r.verdict, canon_core::Verdict::Proposed { .. }))
        .collect();
    if !proposed.is_empty() || !retractions.is_empty() {
        println!(
            "\n{} proposed, not yet in force:",
            proposed.len() + retractions.len()
        );
        for c in &proposed {
            println!("{}", crate::wrap::hang(&format!("{}  ", c.id()), c.text()));
            if let Status::Proposed { needs } = &c.status() {
                println!("{}", crate::wrap::hang("                  needs ", needs));
            }
        }
        for r in &retractions {
            let what = format!(
                "retract {}{}",
                crate::explain::named(&st, &r.target),
                if r.rationale.trim().is_empty() {
                    String::new()
                } else {
                    format!(" — {}", r.rationale)
                }
            );
            println!("{}", crate::wrap::hang(&format!("{}  ", r.act), &what));
            if let canon_core::Verdict::Proposed { needs, .. } = &r.verdict {
                println!("{}", crate::wrap::hang("                  needs ", needs));
            }
        }
    }
    let refused = st
        .commitments()
        .iter()
        .filter(|c| matches!(c.status(), Status::Refused { .. }))
        .count();
    if refused > 0 {
        println!("{refused} refused — `canon why <id>` says why");
    }
    crate::govern::unapplied(&log, &st);
    let carried = st.tolerated().count();
    if carried > 0 {
        println!("{carried} contradiction(s) carried knowingly — `canon list --json` for detail");
    }
    // A hole in the record is louder than a missing feature.
    if !st.dangling().is_empty() {
        eprintln!(
            "\nwarning: {} act(s) point at a rule that is not in this log:",
            st.dangling().len()
        );
        for (act, missing) in st.dangling() {
            eprintln!("  {act} -> {missing}");
        }
    }
    // Absence of attribution is reported, never defaulted — counted by
    // kind, because thirty ids on one line is reported and not readable.
    // The ids are in `--json`, and `canon log` names each one's actor.
    if !st.unattended().is_empty() {
        eprintln!(
            "\nwarning: {} ruling(s) were not made by a person: {}",
            st.unattended().len(),
            by_kind(&log, st.unattended())
        );
        eprintln!("  `canon log` shows them; `canon list --json` has their ids under `unattended`");
    }
    // A forgery is louder than an unsigned agent: somebody wrote as
    // somebody else.
    if !st.unverified().is_empty() {
        eprintln!(
            "\nwarning: {} act(s) under a name that has bound a key were not signed by it, and \
             do not count: {}",
            st.unverified().len(),
            by_kind(
                &log,
                &st.unverified()
                    .iter()
                    .map(|(id, _)| id.clone())
                    .collect::<Vec<_>>()
            )
        );
        eprintln!("  `canon log` says why for each; `canon identify` says who you are");
    }
    if let Some(note) = carried_note(&st) {
        eprintln!("\n{note}");
    }
    if let Some(note) = store::disorder(&d) {
        eprintln!("\n{note}");
    }
    0
}

/// `2 grants, 1 ratification rule` — acts counted by what they are, most
/// first.
fn by_kind(log: &Log, ids: &[ActId]) -> String {
    let mut counts: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for act in log.acts().iter().filter(|a| ids.contains(&a.id)) {
        *counts.entry(kind_noun(&act.kind)).or_default() += 1;
    }
    let mut counts: Vec<(&str, usize)> = counts.into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    counts
        .iter()
        .map(|(noun, n)| format!("{n} {noun}{}", if *n == 1 { "" } else { "s" }))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What an act is, as a noun a person would count.
fn kind_noun(kind: &ActKind) -> &'static str {
    match kind {
        ActKind::Assert { .. } => "assertion",
        ActKind::Supersede { .. } => "amendment",
        ActKind::Retract { .. } => "repeal",
        ActKind::Accept { .. } | ActKind::Dismiss { .. } => "conflict ruling",
        ActKind::Revert { .. } => "undo",
        ActKind::Question { .. } => "question",
        ActKind::Grant { .. } => "grant",
        ActKind::Withdraw { .. } => "withdrawal",
        ActKind::Scoped { .. } => "area placement",
        ActKind::Position { .. } => "position",
        ActKind::Adopt { .. } => "adoption",
        ActKind::Policy { .. } => "rulings choice",
        ActKind::Ratification { .. } => "way rules pass",
        ActKind::Allot { .. } => "allotment",
        ActKind::Allocation { .. } => "allocation rule",
        ActKind::Decided { .. } => "decision",
        ActKind::Silence { .. } => "silence",
        ActKind::DrawCommit { .. } => "draw",
        ActKind::DrawSecret { .. } | ActKind::DrawReveal { .. } => "draw secret",
        ActKind::Horizon { .. } => "review date",
        ActKind::Rank { .. } => "rank",
        ActKind::Key { .. } => "key binding",
        ActKind::Annotation { .. } => "annotation",
    }
}

pub fn why(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(needle) = pos.first() else {
        return fail("usage: canon why <id>");
    };
    let (_, log, st) = or_fail!(load());
    let id = or_fail!(crate::explain::resolve_why(&st, &log, needle));
    let e = or_fail!(crate::explain::explain_at(&log, &st, &id, store::now()));
    if has(args, "--json") {
        let view = st
            .question(&id)
            .map(|_| crate::questions::view(&log, &st, store::now(), &id));
        let record = st
            .get(&id)
            .and_then(|c| serde_json::to_value(c).ok())
            .or_else(|| st.question(&id).and_then(|q| serde_json::to_value(q).ok()))
            .or_else(|| {
                st.silences()
                    .iter()
                    .find(|s| s.act == id)
                    .and_then(|s| serde_json::to_value(s).ok())
            })
            .unwrap_or(serde_json::Value::Null);
        print_json(&serde_json::json!({
            "id": id.to_string(),
            "headline": e.headline,
            "lines": e.lines,
            "record": record,
            "question_view": view,
        }));
    } else {
        print!("{}", e.render("  "));
    }
    0
}

pub fn supersede(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon amend <id> \"<new text>\" -m \"<reason>\" [--area <area>]");
    }
    let (d, _, st) = or_fail!(load());
    let old = or_fail!(crate::explain::resolve_any(&st, pos[0]));
    // The replacement lives where the old one lived unless told otherwise,
    // so the same people who ratified the rule ratify its replacement.
    let scope = match scope_arg(args) {
        Ok(Some(s)) => Some(s),
        Ok(None) => st.scope_of(&old).cloned(),
        Err(e) => return fail(e),
    };
    let act = or_fail!(replace(
        &d,
        &store::actor(),
        &old,
        pos[1],
        flag(args, "-m").unwrap_or_default(),
        scope,
    ));
    println!("{}  {}", act.id, pos[1]);
    println!("  replaces {old}");
    report_status(&d, &act.id);
    0
}

pub fn retract(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(needle) = pos.first() else {
        return fail("usage: canon repeal <id> -m \"<reason>\"");
    };
    let (d, _, st) = or_fail!(load());
    let target = or_fail!(crate::explain::resolve_any(&st, needle));
    let act = or_fail!(write(
        &d,
        ActKind::Retract {
            target: target.clone(),
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ));
    governed(&d, &act.id, || println!("repealed {target}"))
}

pub fn accept(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon accept <a> <b> -m \"<reason>\"");
    }
    // The rationale is required here and nowhere else: an accepted
    // contradiction must say what it protects.
    let Some(rationale) = flag(args, "-m").filter(|r| !r.trim().is_empty()) else {
        return fail("accept requires -m \"<reason>\" — a tolerated contradiction must say why");
    };
    let (d, _, st) = or_fail!(load());
    let a = or_fail!(crate::explain::resolve(&st, pos[0]));
    let b = or_fail!(crate::explain::resolve(&st, pos[1]));
    let act = or_fail!(write(
        &d,
        ActKind::Accept {
            a: a.clone(),
            b: b.clone(),
            rationale: rationale.to_string(),
            revisit: flag(args, "--revisit").map(str::to_string),
        },
    ));
    governed(&d, &act.id, || {
        println!("carrying both, knowingly:");
        println!("  {}", crate::explain::named(&st, &a));
        println!("  {}", crate::explain::named(&st, &b));
        println!("  {rationale}");
        if let Some(r) = flag(args, "--revisit") {
            println!("  look again by {r}");
        }
    })
}

pub fn dismiss(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon dismiss <a> <b> [-m \"<reason>\"]");
    }
    let (d, _, st) = or_fail!(load());
    let a = or_fail!(crate::explain::resolve(&st, pos[0]));
    let b = or_fail!(crate::explain::resolve(&st, pos[1]));
    let act = or_fail!(write(
        &d,
        ActKind::Dismiss {
            a: a.clone(),
            b: b.clone(),
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ));
    governed(&d, &act.id, || {
        println!("dismissed: {a} and {b} are not in conflict")
    })
}

/// The one act whose id starts with `needle`, or which of the two ways it
/// failed to be one.
pub fn act_matching(log: &Log, needle: &str, hint: &str) -> Result<ActId, String> {
    let hits: Vec<&Act> = (log.acts().iter())
        .filter(|a| a.id.as_str().starts_with(needle))
        .collect();
    match hits.len() {
        1 => Ok(hits[0].id.clone()),
        0 => Err(format!("no act matching `{needle}`{hint}")),
        n => Err(format!("`{needle}` matches {n} acts — use more characters")),
    }
}

pub fn undo(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(target) = pos.first() else {
        return fail("usage: canon undo <act-id> -m \"<reason>\"");
    };
    let (d, log, _) = or_fail!(load());
    let act_id = or_fail!(act_matching(&log, target, " — `canon log` to see them"));
    match write(
        &d,
        ActKind::Revert {
            targets: vec![act_id.clone()],
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        // Deleting somebody else's act takes standing over it, the same as
        // retracting one. The fold says which before anything claims an
        // effect.
        Ok(a) => governed(&d, &a.id, || {
            println!("reverted {act_id} (itself revertible)")
        }),
        Err(e) => fail(e),
    }
}

/// What this build carried without interpreting, for any surface whose answer
/// could have been different had it understood them.
///
/// Required, not decorative (§18.3, and the §4.3 deviation `PRIMITIVES.md`
/// records): carrying an unknown governance move is what keeps the format
/// extensible, and carrying it *silently* would let a canon answer as though
/// it had read everything when it had not.
pub fn carried_note(canon: &Canon) -> Option<String> {
    // A reply to a question is carried by the core and read by `why` and
    // `open`, so it is not among what this build could not read.
    let unread = |k: &&str| *k != crate::questions::OP;
    let mut kinds: Vec<&str> = (canon.carried().iter().map(|(_, k)| k.as_str()))
        .filter(unread)
        .collect();
    let n = kinds.len();
    kinds.sort_unstable();
    kinds.dedup();
    (n > 0).then(|| {
        format!(
        "  {n} act(s) carried but not interpreted by this build ({}) — `canon log` shows them.\n  \
         They had no effect on the answer above. A newer canon may read them.",
        kinds.join(", ")
    )
    })
}

pub fn log(args: &[String]) -> i32 {
    let (d, log, st) = or_fail!(load());
    if has(args, "--json") {
        print!("{}", log.render());
        return 0;
    }
    // The log is the one text output that speaks the record's own names,
    // so it says once how they map onto the ones the rest of canon uses.
    println!("the record's own words: supersede is amend, retract is repeal, grant is give,");
    println!("scoped is area, policy is rulings — `canon help <word>` for any\n");
    for act in log.acts() {
        let what = match &act.kind {
            ActKind::Assert { text, .. } => format!("assert     {text}"),
            ActKind::Supersede { text, .. } => format!("supersede  {text}"),
            ActKind::Retract { target, .. } => format!("retract    {target}"),
            ActKind::Accept { a, b, .. } => format!("accept     {a} / {b}"),
            ActKind::Dismiss { a, b, .. } => format!("dismiss    {a} / {b}"),
            ActKind::Revert { targets, .. } => format!("revert     {}", targets.len()),
            ActKind::Question { text, .. } => format!("question   {text}"),
            ActKind::Grant {
                holder,
                scope,
                horizon,
                ..
            } => match horizon {
                Some(h) => format!("grant      {holder} over {scope} until {}", store::ymd(*h)),
                None => format!("grant      {holder} over {scope}"),
            },
            ActKind::Withdraw { holder, scope, .. } => {
                format!("withdraw   {holder} from {scope}")
            }
            ActKind::Scoped { commitment, scope } => {
                format!("scoped     {commitment} -> {scope}")
            }
            ActKind::Position {
                about,
                pull,
                citing,
                ..
            } => {
                // "position toward" never said which way to a reader who
                // had not met `Pull` (xenia row 17); the verb that wrote it does.
                let way = match pull {
                    canon_core::Pull::Toward => "approves",
                    canon_core::Pull::Against => "objects to",
                };
                match citing {
                    Some(id) => format!("position   {way} {about} (citing {id})"),
                    None => format!("position   {way} {about}"),
                }
            }
            ActKind::Adopt {
                lineage,
                generation,
                ..
            } => {
                format!("adopt      {lineage}@{generation}")
            }
            ActKind::Policy { rule, scope, .. } => match scope {
                Some(sc) => format!("policy     {} over {sc}", rule.name()),
                None => format!("policy     {}", rule.name()),
            },
            ActKind::Ratification {
                text, rule, scope, ..
            } => match scope {
                Some(sc) => format!("ratify     {} over {sc} — {text}", rule.name()),
                None => format!("ratify     {} — {text}", rule.name()),
            },
            ActKind::Allot {
                unit, units, scope, ..
            } => {
                format!("allot      {} {unit}(s) in {scope}", units.len())
            }
            ActKind::Allocation { rule, scope, .. } => {
                format!("allocation {} over {scope}", rule.name())
            }
            ActKind::Decided {
                about, authority, ..
            } => format!("decided    {about} -> {authority}"),
            ActKind::Silence { about, .. } => format!("silence    \"{about}\""),
            ActKind::DrawCommit {
                scope,
                count,
                after_ts,
                ..
            } => format!(
                "draw       {count} from {scope} after {}",
                store::ymd(*after_ts)
            ),
            ActKind::DrawSecret { commit, .. } => format!("sealed     for {commit}"),
            ActKind::DrawReveal { commit, .. } => format!("opened     for {commit}"),
            ActKind::Horizon { target, at, .. } => {
                format!("horizon    {target} by {}", store::ymd(*at))
            }
            ActKind::Rank { commitment, rank } => {
                format!("rank       {commitment} is a {rank}")
            }
            ActKind::Key { keys, prior, .. } => {
                let fps: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        canon_core::Key::parse(k)
                            .map(|k| k.fingerprint())
                            .unwrap_or_else(|e| format!("(unreadable: {e})"))
                    })
                    .collect();
                match prior.len() {
                    0 => format!("key        {}", fps.join(", ")),
                    n => format!("key        {}, vouching for {n}", fps.join(", ")),
                }
            }
            // Shown, and shown as unread. The whole point of carrying an
            // annotation this build does not understand is that nothing is
            // lost; rendering it as though it were nothing would lose it
            // exactly where a person would look for it.
            ActKind::Annotation { kind, body } if kind == crate::questions::OP => {
                let at = |v: Option<&serde_json::Value>| {
                    v.and_then(|v| v.as_str()).unwrap_or("?").to_string()
                };
                let q = body.get("question").and_then(|q| q.get("id"));
                format!("reply      to {}: {}", at(q), at(body.get("text")))
            }
            ActKind::Annotation { kind, body } => {
                let fields: Vec<String> = body.keys().map(String::clone).collect();
                format!(
                    "{kind:<10} (carried, not interpreted: {})",
                    fields.join(", ")
                )
            }
        };
        println!(
            "{}  {}  {}  {}",
            act.id,
            store::ymd(act.ts_unix),
            act.actor,
            what
        );
        // A governance act the fold refused is on the record and changed
        // nothing; `list` counts them and sends the reader here, so here
        // has to say which.
        if let Some((_, why)) = st.ungoverned().iter().find(|(x, _)| *x == act.id) {
            println!("{}", crate::wrap::hang("    NOT APPLIED: ", why));
        }
        // Under a name that has bound a key, and not signed by it: carried,
        // and the fold never saw it.
        if let Some((_, why)) = st.unverified().iter().find(|(x, _)| *x == act.id) {
            println!("{}", crate::wrap::hang("    DOES NOT COUNT: ", why));
        }
        // A retraction written without standing is neither applied nor
        // refused when written; say which it has since become.
        if let Some(r) = st.retractions().iter().find(|r| r.act == act.id) {
            let said = match &r.verdict {
                canon_core::Verdict::Proposed { needs, .. } => format!("PROPOSED: needs {needs}"),
                canon_core::Verdict::Ratified { since, how, .. } => {
                    format!("applied {}: {how}", store::ymd(*since))
                }
                canon_core::Verdict::Refused { by, why, .. } => format!(
                    "REFUSED{}: {}",
                    crate::explain::refused_by(by, None, |b| b),
                    crate::explain::refused_why(by, why)
                ),
            };
            println!("{}", crate::wrap::hang("    ", &said));
        }
    }
    println!("\n{} acts", log.len());
    if let Some(note) = carried_note(&st) {
        println!("{note}");
    }
    if let Some(note) = store::disorder(&d) {
        eprintln!("\n{note}");
    }
    0
}

/// Record something the canon does not cover.
///
/// No model, and no ceremony: noticing a gap should cost one line. It is
/// answered by superseding it with a commitment, and withdrawn by retracting
/// it — the acts that already mean those things.
pub fn question(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(text) = pos.first() else {
        return fail("usage: canon question \"<what the canon does not cover>\"");
    };
    let d = or_fail!(dir());
    let act = or_fail!(write(
        &d,
        ActKind::Question {
            text: (*text).to_string(),
            proposal: flag(args, "--from-proposal").map(str::to_string),
        },
    ));
    println!("{}  ? {}", act.id, text);
    println!(
        "  answer it:  canon amend {} \"<the rule>\" -m \"<reason>\"",
        act.id
    );
    0
}

/// Say what you know about a question. It is kept beside the question and
/// makes no rule; closing the question is `repeal`, a separate act.
pub fn respond(args: &[String]) -> i32 {
    let pos = positionals(args);
    let [needle, text] = pos[..] else {
        return fail("usage: canon respond <question-id> \"<what you know>\" [--attribution \"<who said it>\"] [--source \"<where from>\"] [--remaining \"<what is still open>\"] [--corrects <reply-id>]");
    };
    let (d, log, st) = or_fail!(load());
    let q = or_fail!(crate::explain::resolve_question(&st, needle));
    let own = |f| flag(args, f).map(str::to_string);
    let said = crate::questions::Said {
        text: text.to_string(),
        attribution: own("--attribution"),
        source: own("--source"),
        remaining: own("--remaining"),
        ..Default::default()
    };
    let kind = or_fail!(crate::questions::reply(
        &log,
        &st,
        &store::actor(),
        &q,
        said,
        flag(args, "--corrects")
    ));
    let act = or_fail!(write(&d, kind));
    // The record as it reads now decides whether the reply counts.
    let (_, log, st) = or_fail!(load());
    let view = crate::questions::view(&log, &st, store::now(), &q);
    if !view.replies.iter().any(|r| r.id == act.id) {
        return fail(format!(
            "{} was written and does not count — `canon why {q}` says why",
            act.id
        ));
    }
    println!("{}  reply to {q}", act.id);
    println!("  kept beside the question; it makes no rule and closes nothing");
    println!("  read it:  canon why {q}");
    0
}

/// What the canon does not cover. `--all` adds the questions that were
/// answered by a rule or closed; a closed one with something new said about
/// it is listed either way.
pub fn open(args: &[String]) -> i32 {
    let (_, log, st) = or_fail!(load());
    let open = crate::questions::listed(&log, &st, store::now(), has(args, "--all"));
    if has(args, "--json") {
        // Each question as it always rendered, in its field order, plus how
        // many seconded it and what was said back.
        #[derive(serde::Serialize)]
        struct Row<'a> {
            #[serde(flatten)]
            question: &'a canon_core::Question,
            seconds: usize,
            question_view: &'a crate::questions::View,
        }
        let rows: Vec<Row> = open
            .iter()
            .map(|(q, v)| Row {
                question: q,
                seconds: st.seconds(&q.id),
                question_view: v,
            })
            .collect();
        print_json(&rows);
        return 0;
    }
    if open.is_empty() {
        println!("no open questions. `canon question \"<...>\"` records one.");
        return 0;
    }
    for (q, v) in &open {
        println!("{}  ? {}", q.id, q.text);
        if let Some(p) = &q.proposal {
            println!("      from: \"{p}\"");
        }
        let n = st.seconds(&q.id);
        if n > 0 {
            println!("      {n} second(s)");
        }
        for note in crate::questions::notes(q, v) {
            println!("      {note}");
        }
    }
    let live = st.open().count();
    match open.len() - live {
        0 => println!("\n{live} open"),
        more => println!("\n{live} open, {more} more shown"),
    }
    0
}

// ── configuration ───────────────────────────────────────────

pub fn config(args: &[String]) -> i32 {
    let pos = positionals(args);
    let d = or_fail!(dir());
    match pos.first().copied() {
        Some("set") => {
            if pos.len() < 3 {
                return fail("usage: canon config set <key> <value>");
            }
            let key = or_fail!(Key::parse(pos[1]));
            match Config::write(&d, key, pos[2]) {
                Ok(()) => {
                    println!("{} = {}", key.as_str(), pos[2]);
                    0
                }
                Err(e) => fail(e),
            }
        }
        Some("get") => {
            if pos.len() < 2 {
                return fail("usage: canon config get <key>");
            }
            let key = or_fail!(Key::parse(pos[1]));
            let cfg = or_fail!(Config::load(&d));
            match cfg.get(key) {
                Some(v) => {
                    println!("{v}");
                    0
                }
                // Unset is not empty-string. Exit 2 so a script can tell the
                // difference without parsing stdout.
                None => {
                    eprintln!("{} is not set", key.as_str());
                    2
                }
            }
        }
        Some("show") | None => {
            let cfg = or_fail!(Config::load(&d));
            let rendered = cfg.render();
            if rendered.is_empty() {
                println!(
                    "nothing configured. `canon config set endpoint http://localhost:8080/v1`"
                );
            } else {
                print!("{rendered}");
            }
            // Whether a key is set is the first question when a hosted
            // endpoint answers 401; the key itself is never printed. A
            // comment, so the output still reads as a config file.
            if config::api_key().is_some() {
                println!("# {API_KEY_ENV} is set (not shown)");
            }
            0
        }
        Some(other) => fail(format!(
            "unknown config command `{other}` — expected set, get, or show"
        )),
    }
}
