// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon review` — what is waiting on you, one thing at a time.
//!
//! An agent proposes; a person disposes. Until now the person disposed by
//! copying ids out of `list` into `canon approve`, which is the agent's
//! workflow with a human in the loop rather than a human's. This is the
//! human's: each proposal as a person reads it — the words, who wrote them
//! and when, where they came from, what they replace and why, who has to
//! decide — and one keystroke to answer.
//!
//! **"Waiting on you" is asked of the fold, never re-derived here.** For
//! each proposal the queue folds the canon twice more, once with your
//! approval added and once with a reasoned objection, and it waits on you
//! when either would change where it stands. Joint, threshold, consent and
//! subsidiarity are the fold's rules; a second copy of them here would be a
//! second answer to "who decides", and those drift (§10.6).
//!
//! The same queue feeds `canon ui`, so the terminal and the page cannot
//! disagree about what is waiting.

use std::io::{BufRead, IsTerminal, Write};
use std::path::{Path, PathBuf};

use std::collections::BTreeMap;

use canon_core::{Act, ActId, ActKind, Canon, Log, Pull, Scope, Status, Verdict};
use serde::Serialize;

use crate::cmds::{answer, fail, flag, has, load};
use crate::explain::{one_line, person, refused_by, refused_why};
use crate::store;

/// What a proposal would do if it carried.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A new rule.
    Rule,
    /// A rule replacing one or more others.
    Change,
    /// Taking a rule away, by somebody without the seat to do it alone.
    Retraction,
    /// A change to how a room's rules get made.
    DecisionRule,
}

impl Kind {
    pub fn noun(self) -> &'static str {
        match self {
            Kind::Rule => "new rule",
            Kind::Change => "change to a rule",
            Kind::Retraction => "retraction",
            Kind::DecisionRule => "how a room decides",
        }
    }
}

/// One proposal, as a person reads it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Item {
    pub id: ActId,
    pub kind: Kind,
    /// What would become true. For a retraction, the rule that would go.
    pub text: String,
    pub by: String,
    pub at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Where it was drafted from, `path:start-end`, when it was.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// What it would replace, by id and text.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub replaces: Vec<(ActId, String)>,
    /// The proposer's reason, when they gave one.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub because: String,
    /// What would carry it, in the fold's words.
    pub needs: String,
    /// Whether your approval or objection would change where it stands.
    pub yours: bool,
}

/// Where something stands, compared before and after a hypothetical
/// position. `None` when it is not a proposal-shaped thing at all.
fn standing(st: &Canon, id: &ActId) -> Option<String> {
    if let Some(c) = st.get(id) {
        return Some(format!("{:?}", c.status()));
    }
    if let Some(r) = st.ratifications().iter().find(|r| r.act == *id) {
        return Some(format!("{:?}", r.verdict));
    }
    st.retractions()
        .iter()
        .find(|r| r.act == *id)
        .map(|r| format!("{:?}", r.verdict))
}

/// Where something stands after a decision, as one line a person reads.
pub fn outcome(st: &Canon, id: &ActId) -> String {
    let verdict = |v: &Verdict, done: &str| match v {
        Verdict::Ratified { .. } => done.to_string(),
        Verdict::Proposed { needs, .. } => format!("still waiting — needs {needs}"),
        Verdict::Refused { by, why, .. } => {
            format!(
                "refused{}: {}",
                refused_by(by, None, person),
                refused_why(by, why)
            )
        }
    };
    if let Some(c) = st.get(id) {
        return match &c.status() {
            Status::Active => "in force".into(),
            Status::Proposed { needs } => format!("still waiting — needs {needs}"),
            Status::Refused { by, why, .. } => {
                format!(
                    "refused{}: {}",
                    refused_by(by, None, person),
                    refused_why(by, why)
                )
            }
            Status::Superseded { by } => format!("replaced by {by}"),
            Status::Retracted { .. } => "retracted".into(),
        };
    }
    if let Some(r) = st.ratifications().iter().find(|r| r.act == *id) {
        return verdict(&r.verdict, "in force: this is how the room decides now");
    }
    if let Some(r) = st.retractions().iter().find(|r| r.act == *id) {
        return verdict(&r.verdict, "retracted");
    }
    "no longer in this canon".into()
}

/// Everything proposed and not yet decided, with whether it waits on
/// `actor`. Yours first, oldest first within each.
pub fn queue(log: &Log, st: &Canon, actor: &str, now: i64) -> Vec<Item> {
    let mut items = Vec::new();
    let text_of = |id: &ActId| -> String {
        st.get(id)
            .map(|c| c.text().to_string())
            .or_else(|| st.question(id).map(|q| format!("the question: {}", q.text)))
            .unwrap_or_else(|| "(not in this canon)".into())
    };
    let rationale = |id: &ActId| -> String {
        log.acts()
            .iter()
            .find(|a| a.id == *id)
            .map(|a| match &a.kind {
                ActKind::Supersede { rationale, .. } | ActKind::Retract { rationale, .. } => {
                    rationale.clone()
                }
                _ => String::new(),
            })
            .unwrap_or_default()
    };
    for c in st.commitments() {
        let Status::Proposed { needs } = &c.status() else {
            continue;
        };
        items.push(Item {
            id: c.id().clone(),
            kind: if c.replaces().is_empty() {
                Kind::Rule
            } else {
                Kind::Change
            },
            text: c.text().to_string(),
            by: c.actor().to_string(),
            at: c.asserted_at(),
            scope: st.scope_of(c.id()).map(|s| s.as_str().to_string()),
            source: c.source().map(str::to_string),
            replaces: c
                .replaces()
                .iter()
                .map(|o| (o.clone(), text_of(o)))
                .collect(),
            because: rationale(c.id()),
            needs: needs.clone(),
            yours: false,
        });
    }
    for r in st.retractions() {
        let Verdict::Proposed { needs, .. } = &r.verdict else {
            continue;
        };
        items.push(Item {
            id: r.act.clone(),
            kind: Kind::Retraction,
            text: text_of(&r.target),
            by: r.actor.clone(),
            at: r.at,
            scope: r.scope.as_ref().map(|s| s.as_str().to_string()),
            source: None,
            replaces: vec![(r.target.clone(), text_of(&r.target))],
            because: r.rationale.clone(),
            needs: needs.clone(),
            yours: false,
        });
    }
    for r in st.ratifications() {
        let Verdict::Proposed { needs, .. } = &r.verdict else {
            continue;
        };
        items.push(Item {
            id: r.act.clone(),
            kind: Kind::DecisionRule,
            text: r.text.clone(),
            by: r.actor.clone(),
            at: r.at,
            scope: r.scope.as_ref().map(|s| s.as_str().to_string()),
            source: None,
            replaces: Vec::new(),
            because: String::new(),
            needs: needs.clone(),
            yours: false,
        });
    }

    // The fold, asked. Signatures are set aside first: what does not count
    // already does not, and a hypothetical position cannot be signed.
    let out: std::collections::BTreeSet<&ActId> =
        st.unverified().iter().map(|(id, _)| id).collect();
    let base: Vec<Act> = log
        .acts()
        .iter()
        .filter(|a| !out.contains(&a.id) && !matches!(a.kind, ActKind::Key { .. }))
        .cloned()
        .collect();
    let decided: std::collections::BTreeSet<&str> = st
        .positions()
        .iter()
        .filter(|p| p.by == actor)
        .map(|p| p.about.as_str())
        .collect();
    for item in &mut items {
        if decided.contains(item.id.as_str()) {
            continue;
        }
        let before = standing(st, &item.id);
        item.yours = [Pull::Toward, Pull::Against].into_iter().any(|pull| {
            let mut acts = base.clone();
            acts.push(Act::new(
                ActKind::Position {
                    about: item.id.as_str().into(),
                    citing: None,
                    pull,
                    because: "asked".into(),
                },
                now,
                actor,
            ));
            standing(&Log::from_acts(acts).derive_at(now), &item.id) != before
        });
    }
    items.sort_by(|a, b| b.yours.cmp(&a.yours).then(a.at.cmp(&b.at)));
    items
}

/// Approve or object, and say where it stands afterwards. The one way both
/// faces write, so a decision from the page is the same act as one typed —
/// and the same act `canon approve` and `canon object` write.
pub fn decide(
    d: &Path,
    actor: &str,
    id: &ActId,
    approve: bool,
    because: &str,
) -> Result<String, String> {
    answer(d, actor, id, approve, because)?;
    let st = store::read(d)?.derive_at(store::now());
    Ok(outcome(&st, id))
}

/// The lines of a drafted rule's source, when its file is inside this
/// canon's folder. `path:start-end`, read fresh, at most twelve lines.
///
/// Only inside the folder the canon lives in: a source is a string anybody
/// who can write the log can set, and the page must not become a way to put
/// `~/.ssh/id_ed25519` on somebody's screen.
pub fn passage(d: &Path, source: &str) -> Option<String> {
    let (path, range) = source.rsplit_once(':')?;
    let (start, end) = range.split_once('-')?;
    let (start, end): (usize, usize) = (start.parse().ok()?, end.parse().ok()?);
    let root = d.parent()?.canonicalize().ok()?;
    let file: PathBuf = root.join(path).canonicalize().ok()?;
    if !file.starts_with(&root) || start == 0 || end < start {
        return None;
    }
    let text = std::fs::read_to_string(file).ok()?;
    let lines: Vec<&str> = text
        .lines()
        .skip(start - 1)
        .take((end + 1 - start).min(12))
        .collect();
    (!lines.is_empty()).then(|| lines.join("\n"))
}

/// "2 days ago", the way a person says it.
pub fn ago(then: i64, now: i64) -> String {
    let s = (now - then).max(0);
    match s {
        0..=89 => "just now".into(),
        90..=5399 => format!("{} minutes ago", (s + 30) / 60),
        5400..=129_599 => format!("{} hours ago", (s + 1800) / 3600),
        129_600..=1_209_599 => format!("{} days ago", (s + 43_200) / 86_400),
        _ => format!("on {}", store::ymd(then)),
    }
}

// ── the terminal ────────────────────────────────────────────

/// Where `review` asks and says: the terminal in use, a script in tests.
pub trait Prompt {
    fn say(&mut self, text: &str);
    /// One line typed in answer, trimmed; `None` at the end of input.
    fn ask(&mut self, question: &str) -> Option<String>;
    /// All of `text`, for reading at the person's pace. Why not, when not.
    fn page(&mut self, text: &str) -> Result<(), String>;
}

/// Standard input and output, once [`door`] has found both a terminal.
struct Terminal {
    input: std::io::StdinLock<'static>,
}

impl Prompt for Terminal {
    fn say(&mut self, text: &str) {
        println!("{text}");
    }

    fn ask(&mut self, question: &str) -> Option<String> {
        print!("{question}");
        std::io::stdout().flush().ok()?;
        let mut line = String::new();
        match self.input.read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim().to_string()),
        }
    }

    /// Through `$PAGER`, else `less`: the program and its arguments split on
    /// whitespace and run directly, no shell, so it works where there is
    /// none.
    fn page(&mut self, text: &str) -> Result<(), String> {
        let pager = std::env::var("PAGER")
            .ok()
            .filter(|p| !p.trim().is_empty())
            .unwrap_or_else(|| "less".into());
        let mut words = pager.split_whitespace();
        let program = words.next().unwrap_or("less");
        let mut child = std::process::Command::new(program)
            .args(words)
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("could not run the pager `{pager}` ({e}); set PAGER"))?;
        if let Some(mut input) = child.stdin.take() {
            // Quitting the pager before the end closes the pipe: read enough.
            match input.write_all(text.as_bytes()) {
                Err(e) if e.kind() != std::io::ErrorKind::BrokenPipe => {
                    return Err(format!("writing to `{pager}`: {e}"));
                }
                _ => {}
            }
        }
        let status = child.wait().map_err(|e| format!("`{pager}`: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("the pager `{pager}` exited with {status}"))
        }
    }
}

/// Whether this process may walk somebody through decisions, and if not,
/// why, in one line.
///
/// **A person's verb, never an agent's by accident.** `review` writes
/// approvals and objections as `actor`, so it runs only with a terminal on
/// both stdin and stdout and only for an actor canon counts as a person
/// (`human:`; the fold never counts anyone else's approval). An agent's
/// shell has no terminal, and an agent is told to set `CANON_ACTOR` to
/// `agent:<name>`, so either stops one that runs `canon review` by mistake.
/// It is a speed bump, not a defence: a process can allocate a pty, and
/// whoever can set `CANON_ACTOR` can name themselves a person. What stops
/// somebody deciding as you is `canon identify`. `--json` reads and decides
/// nothing, so it is not behind this door.
pub fn door(terminal: bool, actor: &str, from: &str) -> Result<(), String> {
    if canon_core::Person::of(actor).is_none() {
        return Err(format!(
            "`canon review` decides as a person, and {actor} (from {from}) is not one; \
             `canon review --json` lists what is waiting"
        ));
    }
    if !terminal {
        return Err(
            "`canon review` asks a person at a terminal, and stdin or stdout is not one; \
             `canon review --json` lists what is waiting"
                .into(),
        );
    }
    Ok(())
}

pub fn run(args: &[String]) -> i32 {
    let only = match flag(args, "--scope").map(|raw| Scope::new(raw).ok_or(raw)) {
        None => None,
        Some(Ok(s)) => Some(s),
        Some(Err(raw)) => return fail(format!("`{raw}` is not a scope")),
    };
    if has(args, "--json") {
        let (_, log, st) = match load() {
            Ok(v) => v,
            Err(e) => return fail(e),
        };
        let actor = store::actor();
        let items: Vec<Item> = queue(&log, &st, &actor, store::now())
            .into_iter()
            .filter(|i| {
                only.as_ref()
                    .is_none_or(|o| scope_of(i).is_some_and(|s| o.covers(&s)))
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "actor": actor, "items": items }))
                .unwrap_or_default()
        );
        return 0;
    }
    let (actor, from) = store::actor_from();
    let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    // The door before the canon is even looked for: refused means nothing ran.
    if let Err(why) = door(terminal, &actor, from) {
        return fail(why);
    }
    let d = match crate::cmds::dir() {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    let mut terminal = Terminal {
        input: std::io::stdin().lock(),
    };
    session(&d, &actor, from, true, only.as_ref(), &mut terminal)
}

/// What a session decided. Counted from what was written, not what was
/// typed: an approval that could not be written is `failed`, not approved.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Tally {
    pub approved: usize,
    pub objected: usize,
    pub skipped: usize,
    /// Not answered, because the person quit (or input ended) first.
    pub unreached: usize,
    /// Acts that were asked for and not written.
    pub failed: usize,
}

impl Tally {
    /// The last line a session prints.
    pub fn line(&self) -> String {
        let mut out = format!(
            "approved {}, objected {}, skipped {}",
            self.approved, self.objected, self.skipped
        );
        if self.unreached > 0 {
            out.push_str(&format!(", {} left for later", self.unreached));
        }
        if self.failed > 0 {
            out.push_str(&format!(", {} not written", self.failed));
        }
        out
    }
}

/// `canon review` at a terminal, over the canon in `d`, deciding as
/// `actor`, in `only` and the scopes under it when it is given. The door is checked again here, so a caller that forgot it is
/// refused too. Exits 0 unless an act it was asked to write failed.
pub fn session(
    d: &Path,
    actor: &str,
    from: &str,
    terminal: bool,
    only: Option<&Scope>,
    prompt: &mut dyn Prompt,
) -> i32 {
    if let Err(why) = door(terminal, actor, from) {
        return fail(why);
    }
    let tally = match walk(d, actor, from, only, prompt) {
        Ok(t) => t,
        Err(e) => return fail(e),
    };
    prompt.say(&format!("\n{}", tally.line()));
    i32::from(tally.failed > 0)
}

/// The scope an item is in, when it names one.
fn scope_of(item: &Item) -> Option<Scope> {
    item.scope.as_deref().and_then(Scope::new)
}

/// `scope`, when nobody holds standing over it now: the fold read fresh,
/// because an earlier answer in this session may have changed it.
fn unheld(d: &Path, scope: Option<&Scope>) -> Result<Option<Scope>, String> {
    let Some(scope) = scope else { return Ok(None) };
    let now = store::now();
    let st = store::read(d)?.derive_at(now);
    Ok(st.who_decides(scope, now).is_empty().then(|| scope.clone()))
}

/// What came of offering somebody standing over a scope nobody holds.
enum Stand {
    Took,
    Declined,
    /// Written, and the fold did not apply it. Why.
    Refused(String),
    /// Input ended, or the grant could not be written.
    Quit,
    Failed(String),
}

/// "nobody holds `scope`; take standing?" — and on yes, the grant `canon
/// grant <actor> <scope>` writes, through the same [`crate::govern::give`].
/// Whether it took is the fold's answer, never assumed.
fn stand(d: &Path, actor: &str, scope: &Scope, prompt: &mut dyn Prompt) -> Stand {
    let Some(answer) = prompt.ask(&format!("nobody holds {scope}; take standing? [y/N] › "))
    else {
        return Stand::Quit;
    };
    if !matches!(answer.to_lowercase().as_str(), "y" | "yes") {
        return Stand::Declined;
    }
    let act = match crate::govern::give(d, actor, actor, scope, None, "") {
        Ok(a) => a,
        Err(e) => return Stand::Failed(e),
    };
    match crate::cmds::took(d, &act.id) {
        crate::cmds::Took::Applied | crate::cmds::Took::Open(_) => Stand::Took,
        crate::cmds::Took::Refused(why) | crate::cmds::Took::Proposed(why) => Stand::Refused(why),
    }
}

/// Every proposal waiting on `actor`, a scope at a time, one at a time.
///
/// What waits is the fold's answer ([`queue`]), with one addition: a
/// proposal in a scope nobody holds waits on whoever is willing to take
/// that scope on, so it is shown after asking once whether they will. On
/// yes the fold is asked again, so whether it waits on them now is still
/// its answer.
fn walk(
    d: &Path,
    actor: &str,
    from: &str,
    only: Option<&Scope>,
    prompt: &mut dyn Prompt,
) -> Result<Tally, String> {
    let log = store::read(d)?;
    let now = store::now();
    let st = log.derive_at(now);
    let answered: std::collections::BTreeSet<&str> = st
        .positions()
        .iter()
        .filter(|p| p.by == actor)
        .map(|p| p.about.as_str())
        .collect();
    let mut mine: BTreeMap<Option<String>, Vec<Item>> = BTreeMap::new();
    let mut others = 0;
    for item in queue(&log, &st, actor, now) {
        let scope = scope_of(&item);
        if only.is_some_and(|o| !scope.as_ref().is_some_and(|s| o.covers(s))) {
            continue;
        }
        let open = scope
            .as_ref()
            .is_some_and(|s| st.who_decides(s, now).is_empty())
            && !answered.contains(item.id.as_str());
        if item.yours || open {
            mine.entry(item.scope.clone()).or_default().push(item);
        } else {
            others += 1;
        }
    }
    prompt.say(&whoami(&log, actor, from));
    let mut tally = Tally::default();
    let total: usize = mine.values().map(Vec::len).sum();
    let elsewhere = if others == 0 {
        String::new()
    } else {
        format!(", {others} on someone else")
    };
    if total == 0 {
        prompt.say(&format!("Nothing is waiting on you{elsewhere}."));
        return Ok(tally);
    }
    prompt.say(&format!("{total} waiting on you{elsewhere}."));
    let mut n = 0;
    for (scope, mut items) in mine {
        items.sort_by_key(|i| i.at);
        let label = scope.as_deref().unwrap_or("the canon as a whole");
        prompt.say(&format!("\n━━ {label}: {} ━━", items.len()));
        if let Some(s) = unheld(d, scope.as_deref().and_then(Scope::new).as_ref())? {
            match stand(d, actor, &s, prompt) {
                Stand::Took => {
                    prompt.say(&format!("  you hold {s} now"));
                    // Asked again: the grant is on the record, and what
                    // waits on you is still the fold's call.
                    let log = store::read(d)?;
                    let now = store::now();
                    let fresh = queue(&log, &log.derive_at(now), actor, now);
                    items = items
                        .into_iter()
                        .filter_map(|i| {
                            let now_yours = fresh.iter().find(|f| f.id == i.id && f.yours);
                            if now_yours.is_none() {
                                prompt.say(&format!("  {} does not wait on you even so", i.id));
                                tally.skipped += 1;
                                n += 1;
                            }
                            now_yours.cloned()
                        })
                        .collect();
                }
                Stand::Declined => {
                    prompt.say(&format!("  skipped {} in {s}", items.len()));
                    tally.skipped += items.len();
                    n += items.len();
                    continue;
                }
                Stand::Refused(why) => {
                    prompt.say(&crate::wrap::hang("  not granted: ", &why));
                    tally.failed += 1;
                    tally.skipped += items.len();
                    n += items.len();
                    continue;
                }
                Stand::Failed(why) => {
                    prompt.say(&crate::wrap::hang("  not written: ", &why));
                    tally.failed += 1;
                    tally.skipped += items.len();
                    n += items.len();
                    continue;
                }
                Stand::Quit => {
                    tally.unreached = total - n;
                    return Ok(tally);
                }
            }
        }
        for item in &items {
            n += 1;
            prompt.say(&format!(
                "\n── {n} of {total} ─────────────────────────────"
            ));
            prompt.say(&card(d, item, now));
            if !one(d, actor, item, prompt, &mut tally) {
                // This one was not answered either, so it is left with the rest.
                tally.unreached = total - n + 1;
                return Ok(tally);
            }
        }
    }
    Ok(tally)
}

/// Ask about one proposal until it is answered. False when the person quit
/// or input ended before answering it, so nothing after it is asked.
fn one(d: &Path, actor: &str, item: &Item, prompt: &mut dyn Prompt, tally: &mut Tally) -> bool {
    loop {
        let Some(answer) = prompt.ask("\n  [a]pprove  [o]bject  [s]kip  [v]iew  [q]uit  › ")
        else {
            return false;
        };
        let (approve, why) = match answer.to_lowercase().as_str() {
            "a" | "approve" | "y" => (true, String::new()),
            "o" | "object" => {
                let Some(why) = prompt.ask("  why? (a reason is required; empty goes back)  › ")
                else {
                    return false;
                };
                if why.is_empty() {
                    continue;
                }
                (false, why)
            }
            "s" | "skip" | "" => {
                tally.skipped += 1;
                return true;
            }
            "q" | "quit" => return false,
            "v" | "view" => {
                if let Err(why) = prompt.page(&whole(item)) {
                    // Unread is worse than unpaged: say why, then all of it.
                    prompt.say(&format!("  {why}; here it is in full:\n"));
                    prompt.say(&whole(item));
                }
                continue;
            }
            _ => continue,
        };
        match decide(d, actor, &item.id, approve, &why) {
            Ok(now) => {
                if approve {
                    tally.approved += 1;
                } else {
                    tally.objected += 1;
                }
                prompt.say(&crate::wrap::hang("  → ", &now));
            }
            Err(e) => {
                tally.failed += 1;
                prompt.say(&crate::wrap::hang("  not written: ", &e));
            }
        }
        return true;
    }
}

/// Who is deciding, where that name came from, and whether the record can
/// tell it was them.
pub fn whoami(log: &Log, actor: &str, from: &str) -> String {
    if canon_core::sign::verify(log.acts())
        .bindings
        .get(actor)
        .is_some_and(|b| !b.contested)
    {
        format!("Acting as {actor} (from {from}), identified: what you decide is signed.")
    } else {
        format!(
            "Acting as {actor} (from {from}). Anyone who can write this file could decide \
             as you — `canon identify` makes it so only your key can."
        )
    }
}

/// How much of a proposal's text the card shows before [v]iew. A rule is a
/// sentence; a proposal can be a whole charter (23 KB, the day this was
/// written), and a card that scrolls it past is one nobody reads.
const EXCERPT: usize = 20;

/// `text` as the terminal shows it: each of its own lines wrapped under
/// `lead`, so a document keeps its paragraphs. The first line carries
/// `lead`, the rest are indented to match.
fn rendered(lead: &str, text: &str) -> Vec<String> {
    let pad = " ".repeat(lead.chars().count());
    let mut out = Vec::new();
    for (i, line) in text.trim().lines().enumerate() {
        if line.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let prefix = if i == 0 { lead } else { &pad };
        out.extend(crate::wrap::hang(prefix, line).lines().map(String::from));
    }
    if out.is_empty() {
        out.push(lead.trim_end().to_string());
    }
    out
}

/// All of a proposal, for the pager: what it is and who proposed it, then
/// its text exactly as written.
fn whole(item: &Item) -> String {
    format!(
        "{} · {}{} · proposed by {}\n\n{}\n",
        item.id,
        item.kind.noun(),
        item.scope
            .as_ref()
            .map_or_else(String::new, |s| format!(" in {s}")),
        item.by,
        item.text.trim_end()
    )
}

/// One proposal as the terminal shows it.
fn card(d: &Path, item: &Item, now: i64) -> String {
    let mut out = format!(
        "  {}{} · proposed by {} {}\n\n",
        item.kind.noun(),
        item.scope
            .as_ref()
            .map_or_else(String::new, |s| format!(" in {s}")),
        person(&item.by),
        ago(item.at, now)
    );
    let lead = if item.kind == Kind::Retraction {
        "  take away: "
    } else {
        "  "
    };
    let lines = rendered(lead, &item.text);
    out.push_str(&lines[..lines.len().min(EXCERPT)].join("\n"));
    out.push('\n');
    if lines.len() > EXCERPT {
        out.push_str(&format!(
            "  … the first {EXCERPT} of {} lines; [v]iew reads it all\n",
            lines.len()
        ));
    } else if lines.len() > 1 {
        out.push_str(&format!("  ({} lines)\n", lines.len()));
    }
    if item.kind != Kind::Retraction {
        for (old, text) in &item.replaces {
            out.push('\n');
            out.push_str(&crate::wrap::hang(
                &format!("  replaces {old}  "),
                &one_line(text),
            ));
        }
    }
    if !item.because.is_empty() {
        out.push('\n');
        out.push_str(&crate::wrap::hang("  because  ", &item.because));
    }
    if let Some(src) = &item.source {
        out.push_str(&format!("\n  from     {src}"));
        if let Some(p) = passage(d, src) {
            for line in p.lines() {
                out.push_str(&format!("\n           │ {line}"));
            }
        }
    }
    out.push('\n');
    out.push_str(&crate::wrap::hang("  needs    ", &item.needs));
    out.push_str(&format!("\n  id       {}", item.id));
    out
}

#[cfg(test)]
pub(crate) mod tests;
