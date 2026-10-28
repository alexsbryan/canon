// SPDX-License-Identifier: AGPL-3.0-or-later
//! The v4 verbs through the real binary: `second`, the seconds `open` shows,
//! the ladder `policy set --graduated` writes, and `decide --citing`.
//!
//! Each test reads the line the verb appended, because the claim is about the
//! record: who it names, what it carries, and which version a reader needs.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn fresh(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("canon-knobs-bar-{name}-{}", std::process::id()));
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

/// The last line in the ledger, parsed.
fn last_line(dir: &Path) -> Value {
    let raw = std::fs::read_to_string(dir.join("acts.jsonl")).expect("the ledger");
    let line = raw.lines().rfind(|l| !l.trim().is_empty()).expect("a line");
    serde_json::from_str(line).expect("a JSON line")
}

fn asked(dir: &Path) -> String {
    let (c, out) = canon_as(
        dir,
        "human:ola",
        &["question", "Should the sink get a second drying rack?"],
    );
    assert_eq!(c, 0, "{out}");
    id_of(&out)
}

#[test]
fn second_refuses_an_agent() {
    let dir = fresh("agent");
    let q = asked(&dir);
    let (c, out) = canon_as(&dir, "agent:helper", &["second", &q]);
    assert_ne!(c, 0, "{out}");
    assert!(out.contains("seconding is a person's"), "{out}");
    assert_eq!(last_line(&dir)["op"], "question", "nothing was appended");
}

#[test]
fn second_refuses_a_non_question() {
    let dir = fresh("non-question");
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &["add", "Pans out of the sink overnight."],
    );
    assert_eq!(c, 0, "{out}");
    let rule = id_of(&out);
    // Weight on a commitment would be an unsigned vote.
    let (c, out) = canon_as(&dir, "human:dana", &["second", &rule]);
    assert_ne!(c, 0, "{out}");
    assert!(out.contains("no question matching"), "{out}");
    assert_eq!(last_line(&dir)["op"], "assert", "nothing was appended");
}

#[test]
fn second_writes_anonymous_toward_seconded() {
    let dir = fresh("writes");
    let q = asked(&dir);
    let (c, out) = canon_as(&dir, "human:dana", &["second", &q]);
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("1 second(s)"), "{out}");
    assert!(
        out.contains("thin"),
        "the anonymity is said to be thin: {out}"
    );
    let line = last_line(&dir);
    assert_eq!(line["actor"], "anonymous", "{line}");
    assert_eq!(line["op"], "position");
    assert_eq!(line["about"], q.as_str());
    assert_eq!(line["pull"], "toward");
    assert_eq!(line["because"], "seconded");
    assert!(line.get("citing").is_none(), "{line}");
    assert_eq!(line["v"], 2, "a second sets no knob: {line}");
    assert!(
        !line.to_string().contains("dana"),
        "the record carries no name: {line}"
    );
}

#[test]
fn open_shows_seconds() {
    let dir = fresh("open");
    let q = asked(&dir);
    let (_, before) = canon_as(&dir, "human:sam", &["open"]);
    assert!(
        !before.contains("second(s)"),
        "no seconds, no line: {before}"
    );
    let (c, out) = canon_as(&dir, "human:dana", &["second", &q]);
    assert_eq!(c, 0, "{out}");
    let (_, text) = canon_as(&dir, "human:sam", &["open"]);
    assert!(text.contains("1 second(s)"), "{text}");
    let (_, json) = canon_as(&dir, "human:sam", &["open", "--json"]);
    let v: Value = serde_json::from_str(&json).expect("open --json is JSON");
    assert_eq!(v[0]["id"], q.as_str(), "{v}");
    assert_eq!(v[0]["seconds"], 1, "{v}");
}

#[test]
fn policy_set_graduated_defaults_to_cited_and_repair() {
    let dir = fresh("graduated");
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "policy",
            "set",
            "default",
            "--graduated",
            "ask-one,ask-panel,refuse",
        ],
    );
    assert_eq!(c, 0, "{out}");
    let line = last_line(&dir);
    assert_eq!(line["rule"]["key"], "cited", "{line}");
    assert_eq!(line["rule"]["repair"], true, "{line}");
    assert_eq!(line["v"], 4, "{line}");

    // Both opt-outs write the body a ladder had before the knobs, at v2.
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "policy",
            "set",
            "default",
            "--graduated",
            "ask-one,refuse",
            "--by-about",
            "--no-repair",
        ],
    );
    assert_eq!(c, 0, "{out}");
    let line = last_line(&dir);
    assert_eq!(
        line["rule"].to_string(),
        r#"{"base":{"rule":"default"},"ladder":["ask-one","refuse"],"rule":"graduated"}"#,
        "{line}"
    );
    assert_eq!(line["v"], 2, "{line}");
}

#[test]
fn decide_citing_writes_v4() {
    let dir = fresh("decide");
    let (_, out) = canon_as(
        &dir,
        "human:sam",
        &["add", "Pans out of the sink overnight."],
    );
    let rule = id_of(&out);
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &["policy", "set", "default", "--graduated", "ask-one,refuse"],
    );
    assert_eq!(c, 0, "{out}");

    // Citing nothing under a ladder keyed on the rule cited: said, once.
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "decide",
            "pans",
            "--outcome",
            "conflicts",
            "--authority",
            "ask-one",
        ],
    );
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("will not count on that ladder"), "{out}");
    assert_eq!(last_line(&dir)["v"], 2);

    let prefix = &rule[..8];
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "decide",
            "pans again",
            "--outcome",
            "conflicts",
            "--authority",
            "ask-one",
            "--citing",
            prefix,
        ],
    );
    assert_eq!(c, 0, "{out}");
    assert!(!out.contains("will not count"), "{out}");
    let line = last_line(&dir);
    assert_eq!(line["v"], 4, "{line}");
    assert_eq!(
        line["citing"],
        serde_json::json!([rule]),
        "a prefix resolves to the full id"
    );

    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "decide",
            "x",
            "--outcome",
            "conflicts",
            "--authority",
            "ask-one",
            "--citing",
            "can-nothing",
        ],
    );
    assert_ne!(c, 0, "an unknown citation is refused, not written: {out}");
}

// ── governs and personal, through `ratification set` and `init` ──────────

#[test]
fn ratification_set_writes_the_batteries_and_carries_them_forward() {
    let dir = fresh("ratify-knobs");
    let (c, out) = canon_as(&dir, "human:sam", &["grant", "human:sam", "house"]);
    assert_eq!(c, 0, "{out}");
    let (c, out) = canon_as(&dir, "human:sam", &["ratification", "set", "consent:7d"]);
    assert_eq!(c, 0, "{out}");
    let line = last_line(&dir);
    assert_eq!(line["governs"], "all", "{line}");
    assert_eq!(line["personal"], true, "{line}");
    assert_eq!(line["v"], 4, "{line}");

    // A kitchen rule written later without the flag keeps what the house
    // lives under, so it cannot reopen the loophole one level down.
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "ratification",
            "set",
            "standing",
            "--scope",
            "house.kitchen",
        ],
    );
    assert_eq!(c, 0, "{out}");
    let line = last_line(&dir);
    assert_eq!(line["governs"], "all", "{line}");
    assert!(line.get("personal").is_none(), "canon-wide only: {line}");
}

#[test]
fn personal_cannot_be_set_for_one_scope() {
    let dir = fresh("personal-scoped");
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &[
            "ratification",
            "set",
            "standing",
            "--scope",
            "house",
            "--personal",
            "on",
        ],
    );
    assert_ne!(c, 0, "{out}");
    assert!(out.contains("canon-wide"), "{out}");
}

#[test]
fn a_policy_set_under_governs_all_says_it_is_proposed_and_can_be_objected_to() {
    // A second apart each time: acts in one second cannot govern each other,
    // so the rule must be in force before the policy it is to catch.
    let tick = || std::thread::sleep(std::time::Duration::from_millis(1100));
    let dir = fresh("policy-waits");
    for who in ["human:sam", "human:dana"] {
        let (c, out) = canon_as(&dir, "human:sam", &["grant", who, "house"]);
        assert_eq!(c, 0, "{out}");
    }
    tick();
    let (c, out) = canon_as(&dir, "human:sam", &["ratification", "set", "consent:7d"]);
    assert_eq!(c, 0, "{out}");
    tick();
    let (c, out) = canon_as(&dir, "human:dana", &["policy", "set", "subsidiarity"]);
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("PROPOSED"), "{out}");
    let id = id_of(&out);
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &["object", &id, "-m", "we decide the house together"],
    );
    assert_eq!(c, 0, "{out}");
    let (c, out) = canon_as(&dir, "human:sam", &["policy", "show"]);
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("REFUSED by human:sam"), "{out}");
}

#[test]
fn init_founds_the_canon_with_the_knobs_on_and_plain_without() {
    let dir = std::env::temp_dir().join(format!("canon-knobs-bar-init-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (c, out) = canon_as(&canon, "human:sam", &["init", "--profile", "house"]);
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("is yours alone"), "said on the way in: {out}");
    assert!(
        out.contains("human:sam.… is yours alone"),
        "the real personal scope, never a placeholder: {out}"
    );
    let line = last_line(&canon);
    assert_eq!(line["op"], "ratification", "{line}");
    assert_eq!(line["governs"], "all", "{line}");
    assert_eq!(line["personal"], true, "{line}");
    assert_eq!(line["v"], 4, "{line}");

    // `--plain` founds a canon a build from before the knobs can read.
    let plain = dir.join("plain");
    let (c, out) = canon_as(
        &plain,
        "human:sam",
        &["init", "--profile", "house", "--plain"],
    );
    assert_eq!(c, 0, "{out}");
    let raw = std::fs::read_to_string(plain.join("acts.jsonl")).unwrap();
    assert!(raw.trim().is_empty(), "{raw}");

    // An agent may found one, and is told its founding act waits for a person.
    let agent = dir.join("agent");
    let (c, out) = canon_as(&agent, "agent:helper", &["init", "--profile", "house"]);
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("waits for a person"), "{out}");
}
