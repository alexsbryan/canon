// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lineages through the real binary: a canon adopted by URL, one adopted by
//! paste, and one moved onto a new base, each the way a person runs it.
//!
//! The lineages here are git repositories on disk, made by the binary itself
//! and tagged by hand, so a generation is what `adopt <url>@<gen>` reads in
//! use: a tag on a clone.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("canon-lineage-bar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Run one verb against the canon at `dir`, with `stdin` piped in when given.
fn canon(dir: &Path, args: &[&str], stdin: Option<&str>) -> (i32, String) {
    canon_by(dir, "human:sam", args, stdin)
}

/// [`canon`], as `actor`. Acts are content-addressed, so two canons written
/// by one person in one second mint the same ids; a second writer keeps the
/// two canons' ids apart, as they are in use.
fn canon_by(dir: &Path, actor: &str, args: &[&str], stdin: Option<&str>) -> (i32, String) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", actor)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("canon runs");
    if let Some(s) = stdin {
        child.stdin.take().unwrap().write_all(s.as_bytes()).unwrap();
    }
    drop(child.stdin.take());
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

fn ok(dir: &Path, args: &[&str]) -> String {
    let (code, out) = canon(dir, args, None);
    assert_eq!(code, 0, "canon {args:?}\n{out}");
    out
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t.invalid")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

/// A lineage repository at `root` whose canon holds `rules`, named `name`.
fn lineage(root: &Path, name: &str, rules: &[&str]) -> PathBuf {
    std::fs::create_dir_all(root).unwrap();
    let c = root.join(".canon");
    ok(&c, &["init", "--profile", "house"]);
    std::fs::write(c.join("name"), name).unwrap();
    for r in rules {
        ok(&c, &["add", r]);
    }
    git(root, &["init", "--quiet"]);
    git(root, &["add", "."]);
    git(root, &["commit", "--quiet", "-m", "rules"]);
    c
}

fn tag(root: &Path, t: &str) {
    git(root, &["add", "."]);
    let _ = Command::new("git")
        .args(["commit", "--quiet", "-m", t])
        .current_dir(root)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t.invalid")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t.invalid")
        .output();
    git(root, &["tag", t]);
}

#[test]
fn a_cloned_lineage_never_lands_in_the_adopters_commit() {
    // `adopt <url>` clones into `.canon/lineages/`, and `git add .canon`
    // then committed a nested clone. A canon founded before `init` listed
    // the cache must be covered too, so this one is given the old list.
    let base = scratch("ignored");
    let up = base.join("up");
    lineage(&up, "house", &["Quiet hours run 11pm-7am."]);
    tag(&up, "g1");

    let home = base.join("home");
    std::fs::create_dir_all(&home).unwrap();
    git(&home, &["init", "--quiet"]);
    let c = home.join(".canon");
    ok(&c, &["init", "--profile", "house"]);
    std::fs::write(c.join(".gitignore"), "seen\ndraft-runs/\nconfig\n").unwrap();
    ok(&c, &["adopt", &format!("{}@g1", up.display())]);

    git(&home, &["add", ".canon"]);
    let staged = git(&home, &["ls-files", "--cached"]);
    assert!(
        !staged.contains("lineages"),
        "the lineage cache was staged:\n{staged}"
    );
    assert!(staged.contains(".canon/upstream/seed.json"), "{staged}");
}

/// The id on a write's first line, `can-…`.
fn id_of(out: &str) -> String {
    out.split_whitespace()
        .find(|w| w.starts_with("can-"))
        .unwrap_or_else(|| panic!("no id in: {out}"))
        .trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
        .to_string()
}

/// A canon at `dir`, named `name`, holding `rules`; the ids of the rules.
fn founded(dir: &Path, name: &str, rules: &[&str]) -> Vec<String> {
    ok(dir, &["init", "--profile", "house"]);
    std::fs::write(dir.join("name"), name).unwrap();
    rules.iter().map(|r| id_of(&ok(dir, &["add", r]))).collect()
}

#[test]
fn a_pasted_fork_takes_its_upstreams_next_generation_by_paste() {
    // Ersilia forked commonwealth-ai with `share | adopt --paste`, and a day
    // later `upgrade` refused: a pasted block has no upstream to fetch, and
    // the advice was to adopt into a new canon, discarding the fork's own
    // record (LOAD_TEST_COMMONWEALTH.md entry 32).
    let base = scratch("paste-upgrade");
    let up = base.join("up");
    let ids = founded(
        &up,
        "house",
        &["Quiet hours run 11pm-7am.", "Bikes live inside."],
    );
    let first = ok(&up, &["share"]);

    let fork = base.join("fork");
    ok(&fork, &["init", "--profile", "house"]);
    let (code, adopted) = canon(&fork, &["adopt", "--paste"], Some(&first));
    assert_eq!(code, 0, "{adopted}");
    assert!(
        adopted.contains("canon upgrade --paste"),
        "adopt --paste does not say how such a fork takes a later generation:\n{adopted}"
    );
    ok(&fork, &["add", "Thursday is movie night."]);

    // Upstream moves on: a new rule, and one withdrawn.
    ok(&up, &["add", "Guests sign the book."]);
    ok(&up, &["retract", &ids[1], "-m", "the shed is built"]);
    let second = ok(&up, &["share"]);

    let (code, out) = canon(&fork, &["upgrade", "--paste"], Some(&second));
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("2 change(s) applied"), "{out}");
    let live = ok(&fork, &["list"]);
    assert!(live.contains("Guests sign the book."), "{live}");
    assert!(!live.contains("Bikes live inside."), "{live}");
    assert!(live.contains("Thursday is movie night."), "{live}");
    let diff = ok(&fork, &["diff", "--upstream"]);
    assert!(diff.contains("ADDED (1) · UNTOUCHED (2)"), "{diff}");

    // A block from another lineage is refused, and writes nothing.
    let other = base.join("other");
    founded(&other, "elsewhere", &["Dogs welcome."]);
    let before = ok(&fork, &["log"]);
    let (code, out) = canon(
        &fork,
        &["upgrade", "--paste"],
        Some(&ok(&other, &["share"])),
    );
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("elsewhere"), "{out}");
    assert_eq!(ok(&fork, &["log"]), before);
}

// ── moving an existing canon onto a new base ────────────────
//
// Shaped like 2026-10-23: ersilia is a pasted fork of commonwealth-ai with
// its own retractions and additions, and cw-family-canon is a second
// pasted fork of commonwealth-ai, with rules of its own excepted and one
// rule added, published by git. The operator wants ersilia to descend from
// the family canon and keep its own record.

const PRE_PUSH: &str = "A feature that has not passed the pre-push gate is not done.";
const TWELVE: &str = "Hold all twelve principles.";
const ATLAS: &str = "The work atlas shows what other agents are doing.";
const PUSH: &str = "Commit freely; never push without the operator.";
const DOCTOR: &str = "When code intel looks dead, run the doctor first.";
const HOUSE: &str = "A new house rule, ratified after the forks.";
const MINE: &str = "Ersilia measures and recommends; a person enacts.";

/// A local endpoint answering every rebase reading with `fate` for each
/// change it is shown, numbered as the prompt numbers them.
fn endpoint(fate: &'static str) -> String {
    use std::io::Read;
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for sock in listener.incoming() {
            let Ok(mut sock) = sock else { return };
            let mut buf = Vec::new();
            let mut tmp = [0u8; 8192];
            let body = loop {
                if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&buf[..pos]).to_ascii_lowercase();
                    let len: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0);
                    while buf.len() < pos + 4 + len {
                        match sock.read(&mut tmp) {
                            Ok(0) | Err(_) => break,
                            Ok(n) => buf.extend_from_slice(&tmp[..n]),
                        }
                    }
                    break String::from_utf8_lossy(&buf[pos + 4..]).to_string();
                }
                match sock.read(&mut tmp) {
                    Ok(0) | Err(_) => break String::new(),
                    Ok(n) => buf.extend_from_slice(&tmp[..n]),
                }
            };
            let req: serde_json::Value = serde_json::from_str(&body).unwrap_or_default();
            let user = req["messages"][1]["content"].as_str().unwrap_or("");
            let made = user.split("The changes they made:").nth(1).unwrap_or("");
            let changes: Vec<serde_json::Value> = made
                .lines()
                .filter_map(|l| l.split_once(". They "))
                .filter_map(|(n, _)| n.trim().parse::<usize>().ok())
                .map(|n| {
                    serde_json::json!({ "change": n, "fate": fate, "target": null,
                                        "because": "the reading" })
                })
                .collect();
            let content = serde_json::json!({ "changes": changes }).to_string();
            let out = serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": content } }]
            })
            .to_string();
            let _ = sock.write_all(
                format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                     connection: close\r\n\r\n{out}",
                    out.len()
                )
                .as_bytes(),
            );
        }
    });
    format!("http://127.0.0.1:{port}/v1")
}

fn canon_at(dir: &Path, endpoint: Option<&str>, args: &[&str]) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_canon"));
    cmd.args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", "human:sam")
        .env_remove("CANON_ENDPOINT")
        .env_remove("CANON_MODEL");
    if let Some(e) = endpoint {
        cmd.env("CANON_ENDPOINT", e);
    }
    let out = cmd.output().expect("canon runs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// The block line ids of `rules`, as `--except` takes them.
fn block_ids(block: &str, rules: &[&str]) -> String {
    rules
        .iter()
        .map(|r| {
            let line = block
                .lines()
                .find(|l| l.starts_with(r))
                .unwrap_or_else(|| panic!("{r} not in the block"));
            line.rsplit_once(" (")
                .unwrap()
                .1
                .trim_end_matches(')')
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// commonwealth-ai's block, ersilia forked from it, and the family canon
/// published at tag `g1`. Returns ersilia's canon dir and the family repo.
fn the_family(base: &Path) -> (PathBuf, PathBuf) {
    let cw = base.join("cw/.canon");
    founded(
        &cw,
        "commonwealth-ai",
        &[PRE_PUSH, TWELVE, ATLAS, PUSH, DOCTOR],
    );
    let block = ok(&cw, &["share"]);

    let ersilia = base.join("ersilia/.canon");
    ok(&ersilia, &["init", "--profile", "house"]);
    let (code, out) = canon(
        &ersilia,
        &[
            "adopt",
            "--paste",
            "--except",
            &block_ids(&block, &[PRE_PUSH, TWELVE, ATLAS]),
            "-m",
            "commonwealth tooling",
        ],
        Some(&block),
    );
    assert_eq!(code, 0, "{out}");
    ok(&ersilia, &["add", MINE]);
    ok(&ersilia, &["add", HOUSE]);

    let fam = base.join("family");
    let fam_canon = fam.join(".canon");
    let ola = |args: &[&str], stdin: Option<&str>| canon_by(&fam_canon, "human:ola", args, stdin);
    let (code, out) = ola(&["init", "--profile", "house"], None);
    assert_eq!(code, 0, "{out}");
    std::fs::write(fam_canon.join("name"), "cw-family-canon").unwrap();
    let (code, out) = ola(
        &[
            "adopt",
            "--paste",
            "--except",
            &block_ids(&block, &[ATLAS, DOCTOR]),
            "-m",
            "not for the family",
        ],
        Some(&block),
    );
    assert_eq!(code, 0, "{out}");
    let (code, out) = ola(&["add", HOUSE], None);
    assert_eq!(code, 0, "{out}");
    git(&fam, &["init", "--quiet"]);
    tag(&fam, "g1");
    (ersilia, fam)
}

#[test]
fn a_fork_moves_onto_a_family_canon_and_keeps_its_own_record() {
    let base = scratch("move");
    let (ersilia, fam) = the_family(&base);
    let url = format!("{}@g1", fam.display());
    let before = ok(&ersilia, &["log", "--json"]);

    let e = endpoint("carries");
    let (code, out) = canon_at(&ersilia, Some(&e), &["adopt", &url, "--rebase"]);
    assert_eq!(code, 0, "{out}");

    // Append-only: every act the fork held is still there, unchanged.
    let after = ok(&ersilia, &["log", "--json"]);
    let now: Vec<&str> = after.lines().collect();
    for act in before.lines() {
        assert!(now.contains(&act), "an act was lost or changed: {act}");
    }

    // It now descends from the family canon, and diffs against it: its two
    // retractions of rules the family holds word for word carry, the rule
    // the family withdrew too is gone from the diff, its local copy of the
    // family's new rule is that rule, and the rule the family dropped and
    // ersilia never touched follows the family out.
    let diff = ok(&ersilia, &["diff", "--upstream"]);
    assert!(diff.starts_with("adopted  cw-family-canon@"), "{diff}");
    assert!(
        diff.contains("SUPERSEDED (0) · RETRACTED (2) · ACCEPTED (0) · ADDED (1) · UNTOUCHED (2)"),
        "{diff}"
    );
    assert!(!diff.contains("NOT HERE"), "{diff}");
    let live = ok(&ersilia, &["list"]);
    assert!(!live.contains(DOCTOR), "{live}");
    assert!(live.contains(MINE) && live.contains(PUSH), "{live}");
    assert_eq!(live.matches(HOUSE).count(), 1, "{live}");

    // And it takes the family's next generation by `upgrade`.
    let (code, out) = canon_by(
        &fam.join(".canon"),
        "human:ola",
        &["add", "Dogs are welcome."],
        None,
    );
    assert_eq!(code, 0, "{out}");
    tag(&fam, "g2");
    let (code, out) = canon_at(&ersilia, None, &["upgrade", "g2"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("1 change(s) applied"), "{out}");
    let diff = ok(&ersilia, &["diff", "--upstream"]);
    assert!(diff.contains("ADDED (1) · UNTOUCHED (3)"), "{diff}");
}

#[test]
fn a_move_the_rebase_would_mark_as_a_conflict_is_refused_and_writes_nothing() {
    let base = scratch("move-conflict");
    let (ersilia, fam) = the_family(&base);
    let url = format!("{}@g1", fam.display());
    let log = ok(&ersilia, &["log", "--json"]);
    let seed = std::fs::read_to_string(ersilia.join("upstream/seed.json")).unwrap();

    let e = endpoint("conflicts");
    let (code, out) = canon_at(&ersilia, Some(&e), &["adopt", &url, "--rebase"]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains(MINE), "the refusal names the conflict:\n{out}");

    // No endpoint at all: the rebase cannot be judged, so neither is the move.
    let (code, out) = canon_at(&ersilia, None, &["adopt", &url, "--rebase"]);
    assert_ne!(code, 0, "{out}");

    assert_eq!(ok(&ersilia, &["log", "--json"]), log);
    assert_eq!(
        std::fs::read_to_string(ersilia.join("upstream/seed.json")).unwrap(),
        seed
    );
}
