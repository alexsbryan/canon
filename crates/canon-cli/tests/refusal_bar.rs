// SPDX-License-Identifier: AGPL-3.0-or-later
//! What a gated write says, in what order, through the real binary.
//!
//! Every governance verb used to print its success line and only then ask
//! the fold whether the act had applied, so a refused grant read
//! `agent:claude holds repo.defaults` before it read `NOT APPLIED`. An agent
//! or a script reading the headline took the wrong lesson. This pins the
//! order for every gated verb in one table, so a verb added later cannot
//! skip it — and pins the other half: an act that took only because the
//! scope was still open says so on the line that reports it.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("canon-refusal-bar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:setup", &["init", "--profile", "house"]);
    assert_eq!(code, 0, "{out}");
    canon
}

/// Run one verb as one actor. Stdout and stderr, and the exit code.
fn canon_as(dir: &Path, actor: &str, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", actor)
        .output()
        .expect("canon runs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// The id on a write's first line, `can-…`.
fn id_of(out: &str) -> String {
    out.split_whitespace()
        .find(|w| w.starts_with("can-"))
        .unwrap_or_else(|| panic!("no id in: {out}"))
        .trim_end_matches(';')
        .to_string()
}

#[test]
fn a_refused_act_leads_with_not_applied_on_every_gated_verb() {
    let dir = fresh("refused");
    // Sam founds the house and writes two rules; a second passes so the
    // gate is shut when Ola, who holds nothing, tries each verb.
    let (c, out) = canon_as(&dir, "human:sam", &["grant", "human:sam", "house"]);
    assert_eq!(c, 0, "{out}");
    let (_, a) = canon_as(
        &dir,
        "human:sam",
        &["add", "Quiet after eleven.", "--scope", "house"],
    );
    let (_, b) = canon_as(
        &dir,
        "human:sam",
        &["add", "Bikes in the hall.", "--scope", "house"],
    );
    let (a, b) = (id_of(&a), id_of(&b));
    let (_, g) = canon_as(&dir, "human:sam", &["log"]);
    let grant_id = g
        .lines()
        .find(|l| l.contains("grant"))
        .map(|l| l.split_whitespace().next().unwrap().to_string())
        .expect("the grant is in the log");
    std::thread::sleep(std::time::Duration::from_millis(1100));

    let verbs: Vec<Vec<&str>> = vec![
        vec!["grant", "human:ola", "house"],
        vec!["withdraw", "human:sam", "house"],
        vec!["policy", "set", "consent", "--scope", "house"],
        vec!["ratification", "set", "consent:7d", "--scope", "house"],
        vec!["retract", &a, "-m", "no"],
        vec!["accept", &a, &b, "-m", "both"],
        vec!["dismiss", &a, &b],
        vec![
            "decide",
            "the hall",
            "--outcome",
            "supported",
            "--authority",
            "act",
        ],
        vec!["undo", &grant_id, "-m", "no"],
        vec!["allot", "house", "--named", "drill,ladder"],
        vec![
            "allocation",
            "set",
            "rotation",
            "--scope",
            "house",
            "--order",
            "holders",
            "--per",
            "7",
        ],
    ];
    for v in &verbs {
        let (code, out) = canon_as(&dir, "human:ola", v);
        let first = out.lines().next().unwrap_or_default();
        assert_eq!(code, 1, "`canon {}` exit: {out}", v.join(" "));
        assert!(
            first.starts_with("NOT APPLIED: "),
            "`canon {}` led with `{first}`, not with NOT APPLIED:\n{out}",
            v.join(" ")
        );
        assert!(
            out.contains("on the record as can-"),
            "`canon {}` did not name the act:\n{out}",
            v.join(" ")
        );
    }
}

#[test]
fn an_act_that_took_while_open_says_so() {
    let dir = fresh("open");
    // The first grant in any canon takes because nothing predates it.
    let (code, out) = canon_as(&dir, "human:sam", &["grant", "human:sam", "house"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("human:sam holds house"), "{out}");
    assert!(
        out.contains("applied while open: no grant over house predates this act"),
        "{out}"
    );
    assert!(out.contains("granted by human:sam"), "{out}");

    // A rule written right after says how it came to be in force rather
    // than a bare "in force" — the founding-script case, on the act itself.
    let (code, out) = canon_as(
        &dir,
        "human:sam",
        &["add", "Quiet after eleven.", "--scope", "house"],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("in force — "), "{out}");
    assert!(
        out.contains("nobody held house before this was written; it was open")
            || out.contains("holds house"),
        "{out}"
    );
}

#[test]
fn a_grant_names_its_granter_and_says_when_it_leaves_you_out() {
    let dir = fresh("granter");
    // The git-name case: the person acts as one label and seats another.
    let (code, out) = canon_as(&dir, "human:Alex Bryan", &["grant", "human:alex", "house"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("granted by human:Alex Bryan"), "{out}");
    assert!(
        out.contains("human:Alex Bryan does not hold house"),
        "{out}"
    );
    assert!(out.contains("waiting on human:alex"), "{out}");

    // Granting yourself says no such thing.
    let dir = fresh("self");
    let (_, out) = canon_as(&dir, "human:sam", &["grant", "human:sam", "house"]);
    assert!(!out.contains("does not hold"), "{out}");
}

#[test]
fn who_names_both_rules_by_name() {
    let dir = fresh("who");
    canon_as(&dir, "human:sam", &["grant", "human:sam", "house"]);
    let (code, out) = canon_as(
        &dir,
        "human:sam",
        &["ratification", "set", "consent:7d", "--scope", "house"],
    );
    assert_eq!(code, 0, "{out}");
    let (code, out) = canon_as(&dir, "human:sam", &["who", "house.kitchen"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("rules made under: consent:7d — set "), "{out}");
    assert!(out.contains("proposals judged under: default"), "{out}");
    assert!(!out.contains("decided under:"), "{out}");
}
