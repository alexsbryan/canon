// SPDX-License-Identifier: AGPL-3.0-or-later
//! The near-duplicate warning at `add`, through the real binary.
//!
//! The group wrote the same guest rule twice — one proposed, one already
//! proposed, each one approval short — and nothing said so. The warning is
//! deterministic and non-blocking: it names the act the new text resembles,
//! says whether it is live or proposed, and how to proceed. It is on stderr,
//! so a pipe keeps its contract.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fresh(name: &str, args: &[&str]) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("canon-similar-bar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let (code, out) = canon_as(&canon, "human:mira", args);
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
fn a_second_guest_rule_names_the_first_and_how_to_settle_it() {
    let dir = fresh("proposed", &["init", "--profile", "house"]);
    canon_as(&dir, "human:mira", &["grant", "human:june", "house"]);
    canon_as(&dir, "human:mira", &["grant", "human:sam", "house"]);
    canon_as(&dir, "human:mira", &["ratification", "set", "majority"]);

    let (code, first) = canon_as(
        &dir,
        "human:june",
        &[
            "add",
            "Guests stay at most three nights, unless the house agrees otherwise in advance.",
        ],
    );
    assert_eq!(code, 0, "{first}");
    let first_id = id_of(&first);

    let (code, second) = canon_as(
        &dir,
        "human:sam",
        &["add", "A guest stays three nights at most."],
    );
    assert_eq!(code, 0, "{second}");
    assert!(
        second.contains(&format!("warning: this looks like {first_id} (proposed)")),
        "the warning did not name the proposal:\n{second}"
    );
    assert!(
        second.contains(&format!("`canon supersede {first_id}`")),
        "the warning did not say how to change it:\n{second}"
    );

    let (_, other) = canon_as(&dir, "human:sam", &["add", "No smoking indoors."]);
    assert!(
        !other.contains("warning: this looks like"),
        "an unrelated rule drew a warning:\n{other}"
    );
}

#[test]
fn a_second_rule_against_a_live_one_says_do_not_add_a_second() {
    let dir = fresh("live", &["init", "--profile", "house", "--open"]);
    let (code, first) = canon_as(
        &dir,
        "human:mira",
        &["add", "Guests stay at most three nights."],
    );
    assert_eq!(code, 0, "{first}");
    let first_id = id_of(&first);

    let (code, second) = canon_as(
        &dir,
        "human:mira",
        &["add", "A guest stays three nights at most."],
    );
    assert_eq!(code, 0, "{second}");
    assert!(
        second.contains(&format!("warning: this looks like {first_id} (live)")),
        "the warning did not name the live rule:\n{second}"
    );
    assert!(
        second.contains("if it is the same, do not add a second"),
        "the warning did not say a duplicate is not the move:\n{second}"
    );
}
