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
