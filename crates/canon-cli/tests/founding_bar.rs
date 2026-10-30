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
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
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

    for verb in ["approve", "list", "retract", "adopt", "who", "draft"] {
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
        flat(&out).contains("holds this canon by holding arch or procedure"),
        "{out}"
    );

    let (code, out) = canon_as(&dir, "human:alex", &["who"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("held by whoever holds arch, procedure"),
        "{out}"
    );
    assert!(out.contains("human:alex  over arch"), "{out}");

    let (_, out) = canon_as(&dir, "human:alex", &["who", "default"]);
    assert!(
        out.contains("nothing in this canon is in `default`"),
        "{out}"
    );
    assert!(
        out.contains("the scopes in use are arch, procedure"),
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
    assert!(out.contains("nobody holds procedure yet"), "{out}");
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
        out.contains("reason it was retracted: their tooling, not ours"),
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
        out.contains("1 adjudication(s) were not authored by a person: 1 grant"),
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
    assert!(out.contains("human:june holds this canon (house)"), "{out}");
    assert!(out.contains("grant the others"), "{out}");
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (code, out) = canon_as(
        &canon,
        "human:june",
        &["add", "Quiet hours are 10pm to 7am."],
    );
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("in force — june holds this canon"), "{out}");
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
    assert!(out.contains("nothing holds this canon"), "{out}");
    let (_, who) = canon_as(&shared, "human:june", &["who"]);
    assert!(who.contains("nobody holds this canon"), "{who}");
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
        who.contains("governance act(s) written without standing"),
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
        log.contains("ratify     standing — A holder's word"),
        "the log hides the ratification's policy: {log}"
    );
    let id = |line: &str| {
        line.split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    };
    let rid = log
        .lines()
        .find(|l| l.contains("ratify"))
        .map(id)
        .unwrap_or_default();
    let (code, why) = canon_as(&canon, "human:june", &["why", &rid]);
    assert_eq!(code, 0, "{why}");
    assert!(why.contains("ratification"), "{why}");
    assert!(why.contains("A holder's word"), "{why}");
    let gid = log
        .lines()
        .find(|l| l.contains("grant"))
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
    assert!(out.contains("no longer holds house.kitchen"), "{out}");
    assert!(out.contains("recorded without your name"), "{out}");
    let log = canon_as(&dir, "human:bo", &["log", "--json"]).1;
    let asked = log
        .lines()
        .find(|l| l.contains("Who waters the plants?"))
        .unwrap_or_else(|| panic!("no question in:\n{log}"));
    assert!(asked.contains(r#""actor":"anonymous""#), "{asked}");
}
