// SPDX-License-Identifier: AGPL-3.0-or-later
//! What waits on whom. The fold is asked; these pin that the asking gets
//! the fold's answer for the shapes people actually meet.

use super::*;
use canon_core::Scope;
use std::path::{Path, PathBuf};

/// A house: alex holds it, sam holds the kitchen, and an agent has
/// proposed a rule for each. Acts a second apart so grants predate what
/// they govern.
pub(crate) fn house() -> (Log, ActId, ActId) {
    let grant = |holder: &str, scope: &str, ts: i64| {
        Act::new(
            ActKind::Grant {
                holder: holder.into(),
                scope: Scope::new(scope).unwrap(),
                horizon: None,
                rationale: String::new(),
            },
            ts,
            "human:alex",
        )
    };
    let propose = |text: &str, scope: &str, ts: i64| {
        let a = Act::new(
            ActKind::Assert {
                text: text.into(),
                from: None,
                source: None,
            },
            ts,
            "agent:euphemia",
        );
        let s = Act::new(
            ActKind::Scoped {
                commitment: a.id.clone(),
                scope: Scope::new(scope).unwrap(),
            },
            ts,
            "agent:euphemia",
        );
        (a, s)
    };
    let (quiet, q_scope) = propose("Quiet hours are 10pm to 7am.", "house", 200);
    let (pans, p_scope) = propose("Pans are washed the same evening.", "house.kitchen", 200);
    let log = Log::from_acts(vec![
        grant("human:alex", "house", 100),
        grant("human:sam", "house.kitchen", 100),
        quiet.clone(),
        q_scope,
        pans.clone(),
        p_scope,
    ]);
    (log, quiet.id, pans.id)
}

fn waiting(log: &Log, actor: &str) -> Vec<ActId> {
    let st = log.derive_at(300);
    queue(log, &st, actor, 300)
        .into_iter()
        .filter(|i| i.yours)
        .map(|i| i.id)
        .collect()
}

#[test]
fn a_proposal_waits_on_whoever_holds_its_room_and_nobody_else() {
    let (log, quiet, pans) = house();
    assert_eq!(waiting(&log, "human:alex"), vec![quiet.clone()]);
    // The kitchen is sam's: the narrowest holder decides, not the house.
    assert_eq!(waiting(&log, "human:sam"), vec![pans]);
    assert!(waiting(&log, "human:dana").is_empty(), "no seat, no say");
    assert!(
        waiting(&log, "agent:euphemia").is_empty(),
        "agents are never asked"
    );
}

#[test]
fn once_you_have_answered_it_is_not_waiting_on_you() {
    let (log, quiet, _) = house();
    let approve = Act::new(
        ActKind::Position {
            about: quiet.as_str().into(),
            citing: None,
            pull: Pull::Toward,
            because: "approved".into(),
        },
        250,
        "human:alex",
    );
    let log = Log::from_acts([log.acts().to_vec(), vec![approve]].concat());
    assert!(waiting(&log, "human:alex").is_empty());
    assert_eq!(outcome(&log.derive_at(300), &quiet), "in force");
}

#[test]
fn a_proposal_reads_as_what_it_would_change() {
    let (log, quiet, _) = house();
    let st = log.derive_at(300);
    let item = queue(&log, &st, "human:alex", 300)
        .into_iter()
        .find(|i| i.id == quiet)
        .unwrap();
    assert_eq!(item.kind, Kind::Rule);
    assert_eq!(item.text, "Quiet hours are 10pm to 7am.");
    assert_eq!(item.scope.as_deref(), Some("house"));
    assert_eq!(item.by, "agent:euphemia");
    assert!(item.needs.contains("holds house"), "{}", item.needs);
}

#[test]
fn a_source_is_quoted_only_from_inside_the_canons_folder() {
    let root = std::env::temp_dir().join(format!("canon-review-passage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".canon")).unwrap();
    std::fs::create_dir_all(root.join("notes")).unwrap();
    std::fs::write(root.join("notes/meeting.md"), "one\ntwo\nthree\n").unwrap();
    let outside = std::env::temp_dir().join(format!("canon-review-secret-{}", std::process::id()));
    std::fs::write(&outside, "do not show this\n").unwrap();
    let d = root.join(".canon");

    assert_eq!(
        passage(&d, "notes/meeting.md:2-3").as_deref(),
        Some("two\nthree")
    );
    assert_eq!(passage(&d, &format!("{}:1-1", outside.display())), None);
    let up = format!("../canon-review-secret-{}:1-1", std::process::id());
    assert_eq!(passage(&d, &up), None, "a path that climbs out");
    assert_eq!(
        passage(&d, "notes/meeting.md:3-2"),
        None,
        "a backwards range"
    );
    assert_eq!(passage(&d, "notes/meeting.md"), None, "no range, no quote");
}

#[test]
fn ago_says_it_the_way_a_person_does() {
    assert_eq!(ago(1000, 1010), "just now");
    assert_eq!(ago(0, 600), "10 minutes ago");
    assert_eq!(ago(0, 3 * 3600), "3 hours ago");
    assert_eq!(ago(0, 2 * 86_400), "2 days ago");
    assert!(ago(0, 30 * 86_400).starts_with("on "));
}

// ── the walk, at a scripted terminal ────────────────────────

/// A terminal that answers from a script and keeps what it was told.
pub(crate) struct Script {
    answers: std::collections::VecDeque<String>,
    pub said: Vec<String>,
    pub asked: Vec<String>,
}

impl Script {
    pub(crate) fn new(answers: &[&str]) -> Self {
        Self {
            answers: answers.iter().map(|a| a.to_string()).collect(),
            said: Vec::new(),
            asked: Vec::new(),
        }
    }

    fn heard(&self) -> String {
        self.said.join("\n")
    }
}

impl Prompt for Script {
    fn say(&mut self, text: &str) {
        self.said.push(text.to_string());
    }

    fn ask(&mut self, question: &str) -> Option<String> {
        self.asked.push(question.to_string());
        self.answers.pop_front()
    }
}

/// A canon in a temp dir: alex holds `house`, and an agent has proposed
/// `texts` in it, an hour ago, a second apart. The `.canon` dir and the
/// proposals' ids, oldest first.
fn scratch(name: &str, texts: &[&str]) -> (PathBuf, Vec<ActId>) {
    let scoped: Vec<(&str, &str)> = texts.iter().map(|t| ("house", *t)).collect();
    scratch_in(name, &scoped)
}

/// [`scratch`], each proposal in the scope named beside it.
fn scratch_in(name: &str, proposals: &[(&str, &str)]) -> (PathBuf, Vec<ActId>) {
    let root =
        std::env::temp_dir().join(format!("canon-review-walk-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let d = root.join(".canon");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join(store::FILE), "").unwrap();
    let then = store::now() - 3600;
    store::append(
        &d,
        &Act::new(
            ActKind::Grant {
                holder: "human:alex".into(),
                scope: Scope::new("house").unwrap(),
                horizon: None,
                rationale: String::new(),
            },
            then,
            "human:alex",
        ),
    )
    .unwrap();
    let mut ids = Vec::new();
    for (i, (scope, text)) in proposals.iter().enumerate() {
        let at = then + 10 + i as i64;
        let a = Act::new(
            ActKind::Assert {
                text: (*text).into(),
                from: None,
                source: None,
            },
            at,
            "agent:euphemia",
        );
        let s = Act::new(
            ActKind::Scoped {
                commitment: a.id.clone(),
                scope: Scope::new(scope).unwrap(),
            },
            at,
            "agent:euphemia",
        );
        store::append(&d, &a).unwrap();
        store::append(&d, &s).unwrap();
        ids.push(a.id);
    }
    (d, ids)
}

/// Every position in the canon: who, about what, which way, and why.
fn positions(d: &Path) -> Vec<(String, String, Pull, String)> {
    store::read(d)
        .unwrap()
        .acts()
        .iter()
        .filter_map(|a| match &a.kind {
            ActKind::Position {
                about,
                pull,
                because,
                ..
            } => Some((a.actor.clone(), about.clone(), *pull, because.clone())),
            _ => None,
        })
        .collect()
}

fn status(d: &Path, id: &ActId) -> String {
    outcome(&store::read(d).unwrap().derive_at(store::now()), id)
}

fn alex(d: &Path, answers: &[&str]) -> (i32, Script) {
    let mut script = Script::new(answers);
    let code = session(d, "human:alex", "CANON_ACTOR", true, None, &mut script);
    (code, script)
}

#[test]
fn approving_writes_the_approval_and_the_rule_is_in_force() {
    let (d, ids) = scratch("approve", &["Quiet hours are 10pm to 7am."]);
    let (code, script) = alex(&d, &["a"]);
    assert_eq!(code, 0, "{}", script.heard());
    assert_eq!(
        positions(&d),
        vec![(
            "human:alex".into(),
            ids[0].to_string(),
            Pull::Toward,
            "approved".into()
        )]
    );
    assert_eq!(status(&d, &ids[0]), "in force");
    assert!(
        script
            .heard()
            .contains("Acting as human:alex (from CANON_ACTOR)"),
        "{}",
        script.heard()
    );
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 1, objected 0, skipped 0"
    );
}

#[test]
fn objecting_asks_for_the_reason_and_will_not_take_a_bare_no() {
    let (d, ids) = scratch("object", &["Pans are washed the same evening."]);
    // An empty reason goes back to the question; nothing is written for it.
    let (code, script) = alex(&d, &["o", "", "o", "We cook late on Fridays."]);
    assert_eq!(code, 0, "{}", script.heard());
    assert_eq!(
        positions(&d),
        vec![(
            "human:alex".into(),
            ids[0].to_string(),
            Pull::Against,
            "We cook late on Fridays.".into()
        )]
    );
    assert!(
        status(&d, &ids[0]).starts_with("refused by"),
        "{}",
        status(&d, &ids[0])
    );
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 0, objected 1, skipped 0"
    );
}

#[test]
fn skipping_writes_nothing_and_leaves_it_waiting() {
    let (d, ids) = scratch("skip", &["Bikes live in the shed."]);
    let (code, script) = alex(&d, &["s"]);
    assert_eq!(code, 0);
    assert!(positions(&d).is_empty());
    assert!(status(&d, &ids[0]).starts_with("still waiting"));
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 0, objected 0, skipped 1"
    );
}

#[test]
fn quitting_midway_keeps_what_was_decided_and_asks_nothing_more() {
    let (d, ids) = scratch(
        "quit",
        &[
            "Quiet after ten.",
            "Bins out on Tuesday.",
            "Guests stay three nights.",
        ],
    );
    let (code, script) = alex(&d, &["a", "q", "a"]);
    assert_eq!(code, 0);
    assert_eq!(positions(&d).len(), 1, "only the first was answered");
    assert_eq!(positions(&d)[0].1, ids[0].to_string());
    assert_eq!(script.asked.len(), 2, "nothing is asked after quit");
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 1, objected 0, skipped 0, 2 left for later"
    );
}

#[test]
fn an_agent_is_refused_before_anything_is_shown_or_written() {
    let (d, _) = scratch("agent", &["Quiet after ten."]);
    let before = std::fs::read_to_string(d.join(store::FILE)).unwrap();
    let mut script = Script::new(&["a"]);
    let code = session(&d, "agent:euphemia", "CANON_ACTOR", true, None, &mut script);
    assert_eq!(code, 2);
    assert!(script.said.is_empty() && script.asked.is_empty());
    assert_eq!(
        std::fs::read_to_string(d.join(store::FILE)).unwrap(),
        before
    );
    let why = door(true, "agent:euphemia", "CANON_ACTOR").unwrap_err();
    assert!(
        why.contains("agent:euphemia") && why.contains("CANON_ACTOR"),
        "{why}"
    );
}

#[test]
fn without_a_terminal_it_is_refused_before_anything_is_shown_or_written() {
    let (d, _) = scratch("tty", &["Quiet after ten."]);
    let before = std::fs::read_to_string(d.join(store::FILE)).unwrap();
    let mut script = Script::new(&["a"]);
    let code = session(&d, "human:alex", "git user.name", false, None, &mut script);
    assert_eq!(code, 2);
    assert!(script.said.is_empty() && script.asked.is_empty());
    assert_eq!(
        std::fs::read_to_string(d.join(store::FILE)).unwrap(),
        before
    );
    assert!(door(false, "human:alex", "git user.name")
        .unwrap_err()
        .contains("terminal"));
}

#[cfg(unix)]
#[test]
fn an_act_that_cannot_be_written_is_counted_and_exits_one() {
    use std::os::unix::fs::PermissionsExt as _;
    let (d, _) = scratch("readonly", &["Quiet after ten."]);
    let file = d.join(store::FILE);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o444)).unwrap();
    let (code, script) = alex(&d, &["a"]);
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
    // Root writes through 0444; there the act lands and this proves nothing.
    if positions(&d).is_empty() {
        assert_eq!(code, 1, "{}", script.heard());
        assert!(script.heard().contains("not written"), "{}", script.heard());
        assert_eq!(
            script.said.last().unwrap().trim(),
            "approved 0, objected 0, skipped 0, 1 not written"
        );
    }
}

/// Every grant in the canon: the act's kind and who wrote it.
fn grants(d: &Path) -> Vec<(ActKind, String)> {
    store::read(d)
        .unwrap()
        .acts()
        .iter()
        .filter(|a| matches!(a.kind, ActKind::Grant { .. }))
        .map(|a| (a.kind.clone(), a.actor.clone()))
        .collect()
}

#[test]
fn taking_standing_over_a_scope_nobody_holds_is_the_grant_and_then_the_question() {
    let (d, ids) = scratch_in("take", &[("garden", "Compost goes in the left bin.")]);
    let (code, script) = alex(&d, &["y", "a"]);
    assert_eq!(code, 0, "{}", script.heard());
    assert_eq!(
        script.asked[0],
        "nobody holds garden; take standing? [y/N] › "
    );
    // The act `canon grant human:alex garden` writes, by alex.
    assert_eq!(
        grants(&d).last().unwrap(),
        &(
            ActKind::Grant {
                holder: "human:alex".into(),
                scope: Scope::new("garden").unwrap(),
                horizon: None,
                rationale: String::new(),
            },
            "human:alex".to_string()
        )
    );
    let st = store::read(&d).unwrap().derive_at(store::now());
    let garden = Scope::new("garden").unwrap();
    assert!(st
        .who_decides(&garden, store::now())
        .iter()
        .any(|g| g.actor == "human:alex"));
    assert_eq!(positions(&d).len(), 1);
    assert_eq!(status(&d, &ids[0]), "in force");
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 1, objected 0, skipped 0"
    );
}

#[test]
fn declining_standing_skips_that_scope_and_writes_nothing() {
    let (d, ids) = scratch_in(
        "decline",
        &[
            ("garden", "Compost goes in the left bin."),
            ("garden", "Hoses coiled."),
        ],
    );
    let (code, script) = alex(&d, &["n", "a"]);
    assert_eq!(code, 0, "{}", script.heard());
    assert_eq!(grants(&d).len(), 1, "only the founding grant");
    assert!(positions(&d).is_empty());
    assert_eq!(script.asked.len(), 1, "nothing in garden is asked about");
    assert!(status(&d, &ids[0]).starts_with("still waiting"));
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 0, objected 0, skipped 2"
    );
}

#[test]
fn scopes_are_walked_one_at_a_time_and_scope_narrows_the_walk() {
    let proposals = [
        ("house", "Quiet after ten."),
        ("garden", "Compost goes in the left bin."),
        ("house", "Bins out on Tuesday."),
    ];
    // Grouped by scope: garden (asked first, declined), then both in house.
    let (d, ids) = scratch_in("grouped", &proposals);
    let (_, script) = alex(&d, &["n", "a", "a"]);
    let mut decided: Vec<String> = positions(&d).into_iter().map(|p| p.1).collect();
    decided.sort();
    let mut house = vec![ids[0].to_string(), ids[2].to_string()];
    house.sort();
    assert_eq!(decided, house);
    // Shown in that order: the garden heading, then house's two, oldest first.
    let heard = script.heard();
    let at = |s: &str| {
        heard
            .find(s)
            .unwrap_or_else(|| panic!("{s} not in {heard}"))
    };
    assert!(at("garden: 1") < at("house: 2"));
    assert!(at("house: 2") < at(ids[0].as_str()));
    assert!(at(ids[0].as_str()) < at(ids[2].as_str()));

    // `--scope house`: garden is never mentioned.
    let (d, _) = scratch_in("only", &proposals);
    let mut script = Script::new(&["s", "s"]);
    let house = Scope::new("house").unwrap();
    let code = session(
        &d,
        "human:alex",
        "CANON_ACTOR",
        true,
        Some(&house),
        &mut script,
    );
    assert_eq!(code, 0);
    assert!(!script.heard().contains("garden"), "{}", script.heard());
    assert!(script.asked.iter().all(|q| !q.contains("standing")));
    assert_eq!(
        script.said.last().unwrap().trim(),
        "approved 0, objected 0, skipped 2"
    );
}
