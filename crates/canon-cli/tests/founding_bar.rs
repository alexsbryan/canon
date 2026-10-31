// SPDX-License-Identifier: AGPL-3.0-or-later
//! Founding a canon by adoption, through the real binary.
//!
//! Ersilia's canon was the first founded by somebody other than canon's
//! author: forked from Commonwealth's with `share | adopt --paste`, 17 of
//! the inherited rules left behind, an agent filing proposals and the
//! operator ratifying, and the result rendered to a file for agents that
//! run without the CLI. Each test here is one place that founding needed a
//! script, a scratch directory or a guess (LOAD_TEST_COMMONWEALTH.md,
//! entries 25–31).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod jargon;

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("canon-founding-bar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn fresh(name: &str) -> PathBuf {
    let canon = scratch(name).join(".canon");
    let (code, out) = canon_as(
        &canon,
        "human:alex",
        &["init", "--profile", "code", "--open"],
    );
    assert_eq!(code, 0, "{out}");
    canon
}

/// Run one verb as one actor, with `stdin` piped in. Stdout, stderr and the
/// exit code.
fn canon_in(dir: &Path, actor: &str, args: &[&str], stdin: &str) -> (i32, String) {
    let (code, out, err) = canon_apart(dir, actor, args, stdin);
    (code, format!("{out}{err}"))
}

/// The exit code, stdout and stderr, kept apart for a test that tells them
/// apart.
fn canon_apart(dir: &Path, actor: &str, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", actor)
        .current_dir(dir.parent().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("canon runs");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn canon_as(dir: &Path, actor: &str, args: &[&str]) -> (i32, String) {
    canon_in(dir, actor, args, "")
}

fn id_of(out: &str) -> String {
    out.split_whitespace()
        .find(|w| w.starts_with("can-"))
        .unwrap_or_else(|| panic!("no id in: {out}"))
        .to_string()
}

/// Output with its wrapping undone, for a phrase the terminal may break.
fn flat(out: &str) -> String {
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn tick() {
    std::thread::sleep(std::time::Duration::from_millis(1100));
}

#[test]
fn every_verb_says_what_it_does_when_asked() {
    // Entry 25: `canon init --help` refused, so learning what init writes
    // meant initialising a scratch directory. No canon is needed to ask.
    let nowhere = scratch("help").join(".canon");
    let (code, out) = canon_as(&nowhere, "human:alex", &["init", "--help"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("init [--profile personal|code|house]"),
        "{out}"
    );
    assert!(out.contains(".canon/acts.jsonl"), "{out}");
    assert!(
        out.contains("flags: --open  --plain  --profile <value>"),
        "{out}"
    );
    assert!(!nowhere.exists(), "asking wrote nothing");

    let (_, same) = canon_as(&nowhere, "human:alex", &["help", "init"]);
    assert_eq!(same, out);

    for verb in ["approve", "list", "repeal", "adopt", "who", "draft"] {
        let (code, out) = canon_as(&nowhere, "human:alex", &[verb, "--help"]);
        assert_eq!(code, 0, "`canon {verb} --help`: {out}");
        assert!(
            out.lines()
                .next()
                .unwrap_or_default()
                .starts_with(&format!("  {verb}")),
            "`canon {verb} --help` led with something else:\n{out}"
        );
        assert!(out.contains("canon help all"), "{out}");
    }
}

#[test]
fn the_canon_as_a_whole_is_named_by_the_scopes_that_hold_it() {
    // Entry 27: an unscoped proposal needed "one person who holds this
    // canon", and the only thing on screen that looked like a way to hold
    // it was `canon who default`'s "grant it" — a guess made into a scope.
    let dir = fresh("who");
    canon_as(&dir, "human:alex", &["grant", "human:alex", "arch"]);
    canon_as(&dir, "human:alex", &["grant", "human:alex", "procedure"]);
    tick();

    let (code, out) = canon_as(
        &dir,
        "agent:claude",
        &["add", "Prefer a test to a comment."],
    );
    assert_eq!(code, 0, "{out}");
    assert!(
        flat(&out).contains("in charge of this canon (its areas arch or procedure)"),
        "{out}"
    );

    let (code, out) = canon_as(&dir, "human:alex", &["who"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("whoever is in charge of arch, procedure"),
        "{out}"
    );
    assert!(out.contains("human:alex  in charge of arch"), "{out}");

    let (_, out) = canon_as(&dir, "human:alex", &["who", "default"]);
    assert!(
        out.contains("nothing in this canon is in `default`"),
        "{out}"
    );
    assert!(
        out.contains("the areas in use are arch, procedure"),
        "{out}"
    );
}

#[test]
fn an_agents_first_write_in_a_fresh_canon_is_a_proposal() {
    // Entry 26: the first probe rule an agent wrote while founding became
    // law before any person had written anything.
    let dir = fresh("open");
    let (code, out) = canon_as(
        &dir,
        "agent:claude",
        &["add", "Probe rule.", "--scope", "procedure"],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("PROPOSED"), "{out}");
    assert!(
        out.contains("nobody is in charge of procedure yet"),
        "{out}"
    );
    let id = id_of(&out);
    let (_, out) = canon_as(&dir, "human:alex", &["approve", &id]);
    assert!(out.contains("in force"), "{out}");
}

#[test]
fn adoption_can_leave_rules_behind_and_name_them_by_upstream_id() {
    // Entry 28: adoption was all or nothing, and the inherited rules had
    // local ids only `list --json` could map back to the pasted block's.
    let up = fresh("upstream");
    for rule in [
        "Cite or abstain.",
        "Run svrn doctor before a release.",
        "Name things for what they are.",
    ] {
        canon_as(&up, "human:maintainer", &["add", rule]);
    }
    let (code, block) = canon_as(&up, "human:maintainer", &["share"]);
    assert_eq!(code, 0, "{block}");
    let upstream_id = |text: &str| -> String {
        let line = block.lines().find(|l| l.starts_with(text)).unwrap();
        line.rsplit('(')
            .next()
            .unwrap()
            .trim_end_matches(')')
            .to_string()
    };
    let tooling = upstream_id("Run svrn doctor");
    let cite = upstream_id("Cite or abstain.");

    // A typo refuses the whole adoption rather than half-writing it.
    let down = fresh("downstream");
    let (code, out) = canon_in(
        &down,
        "human:alex",
        &["adopt", "--paste", "--except", "can-nothere"],
        &block,
    );
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("nothing was adopted"), "{out}");
    let (_, log) = canon_as(&down, "human:alex", &["log"]);
    // Only the founding act `init` writes: the refused adoption added none.
    assert!(log.contains("1 acts"), "{log}");

    let (code, out) = canon_in(
        &down,
        "human:alex",
        &[
            "adopt",
            "--paste",
            "--except",
            &tooling,
            "-m",
            "their tooling, not ours",
        ],
        &block,
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 retracted as it arrived"), "{out}");
    let (_, out) = canon_as(&down, "human:alex", &["diff", "--upstream"]);
    assert!(out.contains("RETRACTED (1)"), "{out}");
    let (_, out) = canon_as(&down, "human:alex", &["why", &tooling]);
    assert!(
        out.contains("reason it was repealed: their tooling, not ours"),
        "{out}"
    );

    // Later, by the id the block printed.
    let (code, out) = canon_as(&down, "human:alex", &["why", &cite]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains(&format!("inherited from upstream {cite}")),
        "{out}"
    );
    let (code, out) = canon_as(&down, "human:alex", &["retract", &cite, "-m", "a fragment"]);
    assert_eq!(code, 0, "{out}");
    let (_, out) = canon_as(&down, "human:alex", &["list"]);
    assert!(out.contains("1 principle live"), "{out}");
}

#[test]
fn a_ratified_founding_warns_by_kind_not_by_thirty_ids() {
    // Entry 30: after every proposal was approved, `list` still warned that
    // 31 adjudications had no person behind them, with 31 ids on one line.
    // 27 were the agent scoping its own proposals.
    let dir = fresh("warning");
    canon_as(&dir, "agent:claude", &["grant", "human:alex", "arch"]);
    tick();
    let mut ids = Vec::new();
    for rule in ["One store.", "Same inputs, same bytes."] {
        let (_, out) = canon_as(&dir, "agent:claude", &["add", rule, "--scope", "arch"]);
        assert!(out.contains("PROPOSED"), "{out}");
        ids.push(id_of(&out));
    }
    for id in &ids {
        canon_as(&dir, "human:alex", &["approve", id]);
    }
    let (_, out) = canon_as(&dir, "human:alex", &["list"]);
    assert!(
        out.contains("1 ruling(s) were not made by a person: 1 grant"),
        "{out}"
    );
    for id in &ids {
        assert_eq!(
            out.matches(id.as_str()).count(),
            1,
            "{id} listed once:\n{out}"
        );
    }
}

#[test]
fn the_canon_renders_to_a_file_and_says_when_the_file_is_stale() {
    // Entry 31: every canon read by agents outside the CLI was going to
    // write its own `jq` over the log.
    let dir = fresh("render");
    canon_as(&dir, "human:alex", &["grant", "human:alex", "arch"]);
    tick();
    let (_, out) = canon_as(
        &dir,
        "human:alex",
        &["add", "One store.", "--scope", "arch"],
    );
    let id = id_of(&out);
    canon_as(&dir, "human:alex", &["rank", &id, "principle"]);

    let (_, json) = canon_as(&dir, "human:alex", &["list", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&json).unwrap();
    let c = &v["commitments"][0];
    assert_eq!(c["scope"], "arch", "{json}");
    assert_eq!(c["rank"], "principle", "{json}");

    let file = dir.parent().unwrap().join("PRINCIPLES.md");
    let file = file.to_str().unwrap();
    let (code, out) = canon_as(&dir, "human:alex", &["list", "--markdown", "--out", file]);
    assert_eq!(code, 0, "{out}");
    let doc = std::fs::read_to_string(file).unwrap();
    assert!(doc.contains("## arch"), "{doc}");
    assert!(
        doc.contains(&format!("- **principle:** One store. `{id}`")),
        "{doc}"
    );
    let (code, out) = canon_as(&dir, "human:alex", &["list", "--markdown"]);
    assert_eq!((code, out), (0, doc));

    let (code, out) = canon_as(
        &dir,
        "human:alex",
        &["list", "--markdown", "--out", file, "--check"],
    );
    assert_eq!(code, 0, "{out}");
    canon_as(
        &dir,
        "human:alex",
        &["add", "Same inputs, same bytes.", "--scope", "arch"],
    );
    let (code, out) = canon_as(
        &dir,
        "human:alex",
        &["list", "--markdown", "--out", file, "--check"],
    );
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("is stale"), "{out}");

    let (code, _) = canon_as(&dir, "human:alex", &["list", "--markdown", "--check"]);
    assert_eq!(code, 2, "--check without a file is a usage error");
}

#[test]
fn init_holds_the_canon_for_its_founder_so_writes_are_not_open() {
    // The group's finding: a canon that nobody holds lets the first person
    // to write become the house. `init` now seats its founder, so the
    // founder's write is the house's and everyone else's is a proposal.
    let dir = std::env::temp_dir().join(format!("canon-founded-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:june", &["init", "--profile", "house"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("human:june is in charge of this canon (house)"),
        "{out}"
    );
    assert!(out.contains("give the others"), "{out}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (code, out) = canon_as(
        &canon,
        "human:june",
        &["add", "Quiet hours are 10pm to 7am."],
    );
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("in force — june is in charge of this canon"),
        "{out}"
    );
    let (code, out) = canon_as(&canon, "human:sam", &["add", "Guests stay three nights."]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("PROPOSED"), "{out}");

    // The old shared start is explicit.
    let shared = dir.join("shared");
    let (code, out) = canon_as(
        &shared,
        "human:june",
        &["init", "--profile", "house", "--open"],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("nobody is in charge of this canon"), "{out}");
    let (_, who) = canon_as(&shared, "human:june", &["who"]);
    assert!(who.contains("nobody is in charge of this canon"), "{who}");
}

#[test]
fn a_bare_name_is_refused_before_it_is_written_and_who_names_the_refused_acts() {
    let dir = std::env::temp_dir().join(format!("canon-bare-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:june", &["init", "--profile", "house"]);
    assert_eq!(code, 0, "{out}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (code, out) = canon_as(&canon, "human:june", &["grant", "Mira", "house"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("human:Mira"), "{out}");
    let raw = std::fs::read_to_string(canon.join("acts.jsonl")).unwrap();
    assert!(!raw.contains("\"holder\":\"Mira\""), "{raw}");
    // A self-grant without holding is written and refused; `who` says so.
    let (_, out) = canon_as(&canon, "human:sam", &["grant", "human:sam", "house"]);
    assert!(out.contains("NOT APPLIED"), "{out}");
    let (_, who) = canon_as(&canon, "human:june", &["who"]);
    assert!(
        who.contains("change(s) to who decides came from someone not in charge"),
        "{who}"
    );
}

#[test]
fn add_keeps_why_it_is_a_rule_and_where_it_came_from() {
    // Sam's two asks in the guests and quiet scenes: the reason at the
    // first writing, and the scattered source with it, never retyped into
    // the rule's text.
    let dir = std::env::temp_dir().join(format!("canon-add-source-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:june", &["init", "--profile", "house"]);
    assert_eq!(code, 0, "{out}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (code, out) = canon_as(
        &canon,
        "human:june",
        &[
            "add",
            "Guests stay three nights.",
            "-m",
            "Sam's cousin stayed two weeks.",
            "--cite",
            "the house chat, 2025-06",
        ],
    );
    assert_eq!(code, 0, "{out}");
    let id = out.split_whitespace().next().unwrap_or_default();
    let (code, why) = canon_as(&canon, "human:june", &["why", id]);
    assert_eq!(code, 0, "{why}");
    assert!(why.contains("Sam's cousin stayed two weeks."), "{why}");
    assert!(why.contains("the house chat, 2025-06"), "{why}");
}

#[test]
fn help_topics_explain_profiles_and_scopes_and_an_unknown_topic_says_so() {
    let nowhere = scratch("topics").join(".canon");
    let (code, out) = canon_as(&nowhere, "human:alex", &["help", "profile"]);
    assert_eq!(code, 0, "{out}");
    for word in ["personal", "code", "house"] {
        assert!(out.contains(word), "{out}");
    }
    let (code, out) = canon_as(&nowhere, "human:alex", &["help", "scopes"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("house.quiet"), "{out}");
    let (code, out) = canon_as(&nowhere, "human:alex", &["help", "nonsense"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("no such topic"), "{out}");
    assert!(!nowhere.exists(), "asking wrote nothing");
}

#[test]
fn a_house_is_told_how_its_rules_are_made_and_a_replaced_rule_says_so() {
    // xenia's coliving house (canon 53f64f3): init never named the rule it
    // founded, so the three argued for days over `policy set` and
    // `ratification set`; and once Sam set unanimous, `ratification show`
    // still called standing "in force since" beside the one deciding.
    let dir = std::env::temp_dir().join(format!("canon-how-made-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:june", &["init", "--profile", "house"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("canon ratification set unanimous"), "{out}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (code, out) = canon_as(&canon, "human:june", &["ratification", "set", "unanimous"]);
    assert_eq!(code, 0, "{out}");
    let uid = out
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .to_string();
    let (code, show) = canon_as(&canon, "human:june", &["ratification", "show"]);
    assert_eq!(code, 0, "{show}");
    assert!(
        show.contains(&format!("replaced: {uid} decides now")),
        "{show}"
    );
    assert_eq!(show.matches("in force since").count(), 1, "{show}");
}

#[test]
fn why_explains_a_governance_act_and_the_log_names_a_ratification() {
    // The June's setup: a ratification id from `log` answered "no record
    // matching", and the log's "ratify majority" hid which policy changed.
    let dir = std::env::temp_dir().join(format!("canon-why-act-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:june", &["init", "--profile", "house"]);
    assert_eq!(code, 0, "{out}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (_, log) = canon_as(&canon, "human:june", &["log"]);
    assert!(
        log.contains("ratify     standing — Whoever is in charge decides"),
        "the log hides the ratification's policy: {log}"
    );
    let id = |line: &str| {
        line.split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    };
    // The log speaks the record's own names, and says so once, in text only.
    assert!(
        log.starts_with("the record's own words: supersede is amend"),
        "{log}"
    );
    let (_, json) = canon_as(&canon, "human:june", &["log", "--json"]);
    assert!(!json.contains("the record's own words"), "{json}");
    let rid = log
        .lines()
        .find(|l| l.starts_with("can-") && l.contains("ratify"))
        .map(id)
        .unwrap_or_default();
    let (code, why) = canon_as(&canon, "human:june", &["why", &rid]);
    assert_eq!(code, 0, "{why}");
    assert!(why.contains("ratification"), "{why}");
    assert!(why.contains("Whoever is in charge decides"), "{why}");
    let gid = log
        .lines()
        .find(|l| l.starts_with("can-") && l.contains("grant"))
        .map(id)
        .unwrap_or_default();
    let (code, why) = canon_as(&canon, "human:june", &["why", &gid]);
    assert_eq!(code, 0, "{why}");
    assert!(why.contains("holder: human:june"), "{why}");
    assert!(why.contains("founded this canon"), "{why}");
}

#[test]
fn voice_lists_standing_as_the_house_holds_it() {
    // `voice` read grants from the raw log: a grant refused for want of
    // standing was listed as held, and a withdrawal over a wider scope did
    // not end the narrower grant under it.
    let dir = fresh("voice");
    canon_as(&dir, "human:alex", &["grant", "human:alex", "house"]);
    tick();
    canon_as(&dir, "human:alex", &["grant", "human:bo", "house.kitchen"]);
    tick();
    let (_, out) = canon_as(&dir, "human:mal", &["grant", "human:mal", "house.garden"]);
    assert!(out.contains("NOT APPLIED"), "{out}");
    tick();
    canon_as(&dir, "human:alex", &["withdraw", "human:bo", "house"]);

    let (code, out) = canon_as(&dir, "human:alex", &["voice", "human:bo"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("over house.kitchen, granted by alex"), "{out}");
    assert!(out.contains(", ended "), "{out}");
    assert!(!out.contains("no end date"), "{out}");

    let (_, out) = canon_as(&dir, "human:alex", &["voice", "human:mal"]);
    assert!(out.contains("has not put anything in this canon"), "{out}");
}

#[test]
fn a_leaver_leaves_their_question_without_their_name_and_only_their_own() {
    // `canon leave` was a verb of its own for a withdrawal and a question;
    // it is `withdraw <you> <scope> --ask` now.
    let dir = fresh("ask");
    canon_as(&dir, "human:alex", &["grant", "human:alex", "house"]);
    tick();
    canon_as(&dir, "human:alex", &["grant", "human:bo", "house.kitchen"]);
    tick();
    let before = canon_as(&dir, "human:bo", &["log", "--json"]).1;
    let (code, out) = canon_as(
        &dir,
        "human:bo",
        &["withdraw", "human:alex", "house", "--ask", "Why?"],
    );
    assert_ne!(code, 0, "{out}");
    assert_eq!(canon_as(&dir, "human:bo", &["log", "--json"]).1, before);

    let (code, out) = canon_as(
        &dir,
        "human:bo",
        &[
            "withdraw",
            "human:bo",
            "house.kitchen",
            "--ask",
            "Who waters the plants?",
        ],
    );
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("no longer in charge of house.kitchen"),
        "{out}"
    );
    assert!(out.contains("recorded without your name"), "{out}");
    let log = canon_as(&dir, "human:bo", &["log", "--json"]).1;
    let asked = log
        .lines()
        .find(|l| l.contains("Who waters the plants?"))
        .unwrap_or_else(|| panic!("no question in:\n{log}"));
    assert!(asked.contains(r#""actor":"anonymous""#), "{asked}");
}

#[test]
fn a_verb_nobody_declared_is_refused_with_the_short_help() {
    // Dispatch reads cli::VERBS; a name outside it reaches no handler.
    let dir = fresh("unknown");
    let (code, out) = canon_as(&dir, "human:alex", &["frobnicate"]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("unknown command `frobnicate`"), "{out}");
    assert!(out.contains("canon help all"), "{out}");
    let (code, out) = canon_as(&dir, "human:alex", &["--version"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.starts_with("canon "), "{out}");
}

/// **An old name is the new one, and a pipe hears nothing about it.** Every
/// renamed verb and flag, run under both spellings on twin canons: the same
/// exit code and stdout, ids and digits aside, and no rename note on stderr,
/// because stderr here is a pipe, as it is for an agent, a hook or the Slack
/// bot. That the note does reach a terminal is `cli::tests`' to show: a test
/// cannot be one.
#[test]
fn every_old_spelling_runs_the_new_verb_and_says_nothing_to_a_pipe() {
    let plain = |s: &str| {
        s.split_whitespace()
            .map(|w| {
                if w.contains("can-") {
                    "<id>".into()
                } else {
                    w.replace(|c: char| c.is_ascii_digit(), "#")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let twins: &[(&[&str], &[&str])] = &[
        (
            &[
                "supersede",
                "<id>",
                "Dishes by morning.",
                "-m",
                "late shifts",
            ],
            &["amend", "<id>", "Dishes by morning.", "-m", "late shifts"],
        ),
        (
            &["retract", "<id>", "-m", "nobody cooks"],
            &["repeal", "<id>", "-m", "nobody cooks"],
        ),
        (
            &["grant", "human:sam", "house.kitchen"],
            &["give", "human:sam", "house.kitchen"],
        ),
        (&["policy", "show"], &["rulings", "show"]),
        (
            &["horizon", "<id>", "2030-01-01"],
            &["revisit", "<id>", "2030-01-01"],
        ),
        (&["tensions"], &["conflicts"]),
        (
            &["scope", "<id>", "house.kitchen"],
            &["area", "<id>", "house.kitchen"],
        ),
        (
            &["add", "Quiet after ten.", "--scope", "house"],
            &["add", "Quiet after ten.", "--area", "house"],
        ),
        (
            &[
                "give",
                "human:sam",
                "house.kitchen",
                "--horizon",
                "2030-01-01",
            ],
            &[
                "give",
                "human:sam",
                "house.kitchen",
                "--until",
                "2030-01-01",
            ],
        ),
        (
            &["rulings", "set", "consent", "--entrench", "principle"],
            &["rulings", "set", "consent", "--constitutional", "principle"],
        ),
        (
            &["rulings", "set", "default", "--graduated", "ask-one,refuse"],
            &["rulings", "set", "default", "--escalate", "ask-one,refuse"],
        ),
        (
            &["replay", "--policy", "consent", "--brief"],
            &["replay", "--rulings", "consent", "--brief"],
        ),
        (
            &["replay", "--ratification", "unanimous", "--brief"],
            &["replay", "--passes", "unanimous", "--brief"],
        ),
        (
            &["replay", "--entrench", "principle", "--brief"],
            &["replay", "--constitutional", "principle", "--brief"],
        ),
        (
            &["replay", "--graduated", "ask-one,refuse", "--brief"],
            &["replay", "--escalate", "ask-one,refuse", "--brief"],
        ),
    ];
    // One house with one rule, copied for each run, so every twin starts
    // from the same bytes.
    let base = scratch("alias-base").join(".canon");
    canon_as(&base, "human:alex", &["init", "--profile", "house"]);
    tick();
    let id = id_of(&canon_as(&base, "human:alex", &["add", "Dishes the same day."]).1);
    tick();
    for (n, (old, new)) in twins.iter().enumerate() {
        let mut said = Vec::new();
        for (twin, typed) in [old, new].into_iter().enumerate() {
            let dir = scratch(&format!("alias-{n}-{twin}")).join(".canon");
            std::fs::create_dir_all(&dir).unwrap();
            for file in std::fs::read_dir(&base).unwrap().flatten() {
                if file.path().is_file() {
                    std::fs::copy(file.path(), dir.join(file.file_name())).unwrap();
                }
            }
            let args: Vec<&str> = typed
                .iter()
                .map(|a| if *a == "<id>" { id.as_str() } else { a })
                .collect();
            let (code, out, err) = canon_apart(&dir, "human:alex", &args, "");
            assert!(
                !err.contains("is now"),
                "`{}` told a pipe:\n{err}",
                typed.join(" ")
            );
            said.push((code, plain(&out)));
        }
        assert_eq!(
            said[0],
            said[1],
            "`{}` and `{}` part",
            old.join(" "),
            new.join(" ")
        );
    }
}

/// **A student can read a whole session.** A house of two, run through the
/// verbs a household meets in its first week, and not one word of the
/// record's own vocabulary on stdout or stderr: what canon says to a person
/// is in the words the person would use.
#[test]
fn a_first_week_in_a_house_reads_in_plain_words() {
    let dir = scratch("first-week").join(".canon");
    let mut said = String::new();
    let mut run = |actor: &str, args: &[&str]| {
        let (_, out, err) = canon_apart(&dir, actor, args, "");
        said.push_str(&format!("$ canon {}\n{out}{err}", args.join(" ")));
        if ![
            "",
            "who",
            "list",
            "why",
            "open",
            "overdue",
            "ratification",
            "rulings",
        ]
        .contains(args.first().unwrap_or(&""))
        {
            tick();
        }
        out
    };
    run("human:alex", &["init", "--profile", "house"]);
    run("human:alex", &["give", "human:sam", "house.kitchen"]);
    let rule = id_of(&run(
        "human:alex",
        &["add", "Dishes are washed the same day."],
    ));
    let quiet = id_of(&run(
        "human:sam",
        &["add", "Quiet after ten.", "--area", "house"],
    ));
    let guests = id_of(&run(
        "human:sam",
        &["add", "Guests leave by midnight.", "--area", "house"],
    ));
    run("human:alex", &["approve", &quiet, "-m", "fair"]);
    run("human:alex", &["object", &guests, "-m", "too strict"]);
    for args in [
        &[][..],
        &["who"],
        &["who", "house.kitchen"],
        &["list"],
        &["why", &rule],
        &["why", &quiet],
        &["open"],
        &["overdue"],
        &["ratification", "show"],
        &["rulings", "show"],
    ] {
        run("human:alex", args);
    }
    run(
        "human:sam",
        &["withdraw", "human:sam", "house.kitchen", "-m", "moving out"],
    );
    run("human:alex", &["check", "Paint the kitchen red."]);
    run("human:sam", &[]);
    let found = jargon::found(&said);
    assert!(found.is_empty(), "{}\n\n{said}", found.join("\n"));
}

/// **A bare `canon` in a canon is where you are and what waits on you.** A
/// house of two: Sam, in charge of the kitchen only, proposes a house rule,
/// and one rule is past its review date. Alex, in charge of the house, sees
/// the proposal as his to answer; Sam sees where he stands and that it is
/// not his.
#[test]
fn a_bare_canon_says_what_is_waiting_for_whoever_runs_it() {
    let dir = scratch("bare-house").join(".canon");
    canon_as(&dir, "human:alex", &["init", "--profile", "house"]);
    tick();
    canon_as(&dir, "human:alex", &["give", "human:sam", "house.kitchen"]);
    tick();
    let (_, out) = canon_as(
        &dir,
        "human:alex",
        &["add", "Dishes are washed the same day."],
    );
    let dishes = id_of(&out);
    tick();
    canon_as(&dir, "human:alex", &["revisit", &dishes, "2020-01-01"]);
    tick();
    canon_as(
        &dir,
        "human:sam",
        &["add", "Quiet after ten.", "--area", "house"],
    );
    let (code, out) = canon_as(&dir, "human:alex", &[]);
    assert_eq!(code, 0, "{out}");
    let page = flat(&out);
    assert!(page.contains("this house's rules: 1 in force"), "{out}");
    assert!(page.contains("You are in charge of house."), "{out}");
    let waiting = page
        .split("waiting for your yes or no:")
        .nth(1)
        .unwrap_or_else(|| panic!("no waiting section:\n{out}"));
    assert!(waiting.contains("Quiet after ten."), "{out}");
    assert!(waiting.contains("canon review"), "{out}");
    assert!(page.contains("past its review date: 1"), "{out}");
    assert!(page.contains("canon overdue"), "{out}");
    assert!(
        out.trim_end()
            .ends_with("`canon help` for everything else."),
        "{out}"
    );

    let (code, out) = canon_as(&dir, "human:sam", &[]);
    assert_eq!(code, 0, "{out}");
    assert!(
        flat(&out).contains("You are in charge of house.kitchen."),
        "{out}"
    );
    assert!(!out.contains("waiting for your yes or no"), "{out}");
    assert!(!out.contains("Quiet after ten."), "{out}");
}

#[test]
fn a_bare_canon_in_an_empty_canon_says_nothing_waits_and_how_to_begin() {
    let dir = fresh("bare-empty");
    let (code, out) = canon_as(&dir, "human:alex", &[]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("nothing is waiting for you"), "{out}");
    assert!(out.contains("canon add \"<a principle>\""), "{out}");
    assert!(out.contains("`canon help` for everything else."), "{out}");
}

/// Outside any canon, the short help still, with the way to start one, and
/// still exit 2: nothing was done.
#[test]
fn a_bare_canon_outside_any_canon_is_the_short_help_and_how_to_start() {
    let dir = scratch("bare-outside");
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .env_remove("CANON_DIR")
        .env("HOME", &dir)
        .current_dir(&dir)
        .output()
        .expect("canon runs");
    let said = String::from_utf8_lossy(&out.stdout);
    assert_eq!(out.status.code(), Some(2), "{said}");
    assert!(said.contains("canon help all"), "{said}");
    assert!(said.contains("`canon init` starts one here"), "{said}");
}
