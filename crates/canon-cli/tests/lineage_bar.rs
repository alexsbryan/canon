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
    let mut child = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", "human:sam")
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
