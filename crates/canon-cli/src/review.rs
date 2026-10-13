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

use canon_core::{Act, ActId, ActKind, Canon, Log, Pull, Status, Verdict};
use serde::Serialize;

use crate::cmds::{answer, fail, has, load};
use crate::explain::{one_line, person};
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
        return Some(format!("{:?}", c.status));
    }
    if let Some(r) = st.ratifications.iter().find(|r| r.act == *id) {
        return Some(format!("{:?}", r.verdict));
    }
    st.retractions
        .iter()
        .find(|r| r.act == *id)
        .map(|r| format!("{:?}", r.verdict))
}

/// Where something stands after a decision, as one line a person reads.
pub fn outcome(st: &Canon, id: &ActId) -> String {
    let verdict = |v: &Verdict, done: &str| match v {
        Verdict::Ratified { .. } => done.to_string(),
        Verdict::Proposed { needs } => format!("still waiting — needs {needs}"),
        Verdict::Refused { by, why, .. } => format!("refused by {}: {why}", person(by)),
    };
    if let Some(c) = st.get(id) {
        return match &c.status {
            Status::Active => "in force".into(),
            Status::Proposed { needs } => format!("still waiting — needs {needs}"),
            Status::Refused { by, why, .. } => format!("refused by {}: {why}", person(by)),
            Status::Superseded { by } => format!("replaced by {by}"),
            Status::Retracted { .. } => "retracted".into(),
        };
    }
    if let Some(r) = st.ratifications.iter().find(|r| r.act == *id) {
        return verdict(&r.verdict, "in force: this is how the room decides now");
    }
    if let Some(r) = st.retractions.iter().find(|r| r.act == *id) {
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
            .map(|c| c.text.clone())
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
    for c in &st.commitments {
        let Status::Proposed { needs } = &c.status else {
            continue;
        };
        items.push(Item {
            id: c.id.clone(),
            kind: if c.replaces.is_empty() {
                Kind::Rule
            } else {
                Kind::Change
            },
            text: c.text.clone(),
            by: c.actor.clone(),
            at: c.asserted_at,
            scope: st.scope_of(&c.id).map(|s| s.as_str().to_string()),
            source: c.source.clone(),
            replaces: c.replaces.iter().map(|o| (o.clone(), text_of(o))).collect(),
            because: rationale(&c.id),
            needs: needs.clone(),
            yours: false,
        });
    }
    for r in &st.retractions {
        let Verdict::Proposed { needs } = &r.verdict else {
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
    for r in &st.ratifications {
        let Verdict::Proposed { needs } = &r.verdict else {
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
    let out: std::collections::BTreeSet<&ActId> = st.unverified.iter().map(|(id, _)| id).collect();
    let base: Vec<Act> = log
        .acts()
        .iter()
        .filter(|a| !out.contains(&a.id) && !matches!(a.kind, ActKind::Key { .. }))
        .cloned()
        .collect();
    let decided: std::collections::BTreeSet<&str> = st
        .positions
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

pub fn run(args: &[String]) -> i32 {
    let (d, log, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let actor = store::actor();
    let items = queue(&log, &st, &actor, store::now());
    if has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "actor": actor, "items": items }))
                .unwrap_or_default()
        );
        return 0;
    }
    let (mine, others): (Vec<&Item>, Vec<&Item>) = items.iter().partition(|i| i.yours);
    println!("{}", whoami(&log, &actor));
    if mine.is_empty() {
        println!("Nothing is waiting on you.");
        if !others.is_empty() {
            println!(
                "{} waiting on someone else — `canon list` shows what each needs.",
                others.len()
            );
        }
        return 0;
    }
    println!(
        "{} waiting on you{}.",
        mine.len(),
        if others.is_empty() {
            String::new()
        } else {
            format!(", {} on someone else", others.len())
        }
    );
    if !std::io::stdin().is_terminal() {
        // Somebody piping this, or an agent: the list, and the commands a
        // person would type. Deciding needs the person.
        for item in &mine {
            println!("\n{}", card(&d, item, store::now()));
            println!("  canon approve {}", item.id);
            println!("  canon object {} -m \"<why>\"", item.id);
        }
        return 0;
    }
    let mut input = std::io::stdin().lock();
    let (mut approved, mut objected, mut skipped) = (0, 0, 0);
    let total = mine.len();
    'items: for (n, item) in mine.into_iter().enumerate() {
        println!("\n── {} of {total} ─────────────────────────────", n + 1);
        println!("{}", card(&d, item, store::now()));
        loop {
            let Some(answer) = ask(&mut input, "\n  (a)pprove  (o)bject  (s)kip  (q)uit  › ")
            else {
                break 'items;
            };
            let decision = match answer.to_lowercase().as_str() {
                "a" | "approve" | "y" => Some((true, String::new())),
                "o" | "object" => {
                    let Some(why) = ask(
                        &mut input,
                        "  why? (a reason is required; empty goes back)  › ",
                    ) else {
                        break 'items;
                    };
                    if why.is_empty() {
                        continue;
                    }
                    Some((false, why))
                }
                "s" | "skip" | "" => None,
                "q" | "quit" => break 'items,
                _ => continue,
            };
            match decision {
                None => skipped += 1,
                Some((approve, why)) => match decide(&d, &actor, &item.id, approve, &why) {
                    Ok(now) => {
                        if approve {
                            approved += 1;
                        } else {
                            objected += 1;
                        }
                        println!("{}", crate::wrap::hang("  → ", &now));
                    }
                    Err(e) => println!("{}", crate::wrap::hang("  not written: ", &e)),
                },
            }
            break;
        }
    }
    println!("\napproved {approved}, objected {objected}, skipped {skipped}");
    0
}

fn ask(input: &mut impl BufRead, prompt: &str) -> Option<String> {
    print!("{prompt}");
    std::io::stdout().flush().ok();
    let mut line = String::new();
    match input.read_line(&mut line) {
        Ok(0) | Err(_) => None,
        Ok(_) => Some(line.trim().to_string()),
    }
}

/// Who is deciding, and whether the record can tell it was them.
pub fn whoami(log: &Log, actor: &str) -> String {
    if canon_core::sign::verify(log.acts())
        .bindings
        .get(actor)
        .is_some_and(|b| !b.contested)
    {
        format!("You are {actor}, identified: what you decide is signed.")
    } else {
        format!(
            "You are {actor}. Anyone who can write this file could decide as you — \
             `canon identify` makes it so only your key can."
        )
    }
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
    out.push_str(&crate::wrap::hang(lead, &item.text));
    out.push('\n');
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
