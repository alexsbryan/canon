// SPDX-License-Identifier: AGPL-3.0-or-later
//! The kitchen bar — the governance knobs of format v4, run as a house would
//! meet them.
//!
//! `fixtures/fernwood-commons/kitchen.jsonl` is a second scenario over the
//! same acts as the Ostrom bar: somebody's own corner, a change to how the
//! kitchen decides, the same argument in new words, a milk rotation, and a
//! question with weight and no names. Its expectations were written before
//! the code that has to meet them, in `kitchen.expected.json`.
//!
//! **Not every expectation is met by this change.** These steps depend on the
//! ratification knobs (`governs`, `personal`) and fail until those are
//! folded:
//!
//! - `kitchen-phyllas-rule-in-force-on-her-write`
//! - `kitchen-a-housemate-cannot-govern-phyllas-corner`
//! - `kitchen-policy-change-is-proposed`
//! - `kitchen-policy-change-refused-by-one-objection`
//! - `kitchen-still-decided-by-subsidiarity`
//! - `kitchen-seconds-move-no-ratification`
//!
//! Everything else — the ladder keyed on the cited rule, the repair, the
//! fallback to `about`, the pool, the seconds — is met now.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

fn fernwood() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
        .join("fernwood-commons")
}

/// Run `canon replay` over fernwood with these extra arguments. Whether it
/// passed, and everything it said.
fn replay(extra: &[&Path]) -> (bool, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_canon"));
    cmd.arg("replay").arg(fernwood());
    let flags = ["--scenario", "--expect"];
    for (flag, path) in flags.iter().zip(extra) {
        cmd.arg(flag).arg(path);
    }
    let out = cmd.output().expect("canon replay runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

/// The names a scenario gives its steps. A step without a name is called by
/// its kind, which no expectation can tell apart, so those are left out.
fn named_steps(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .filter_map(|l| {
            let v: Value = serde_json::from_str(l).expect("scenario line is JSON");
            v["name"].as_str().map(str::to_string)
        })
        .collect()
}

#[test]
fn kitchen_scenario_meets_its_expectations() {
    let dir = fernwood();
    let (ok, text) = replay(&[
        &dir.join("kitchen.jsonl"),
        &dir.join("kitchen.expected.json"),
    ]);
    assert!(
        ok,
        "the kitchen scenario did not meet its expectations:\n{text}"
    );
    assert!(text.contains("all as expected"), "{text}");
}

#[test]
fn every_kitchen_step_is_asserted() {
    // A named step nobody asserts is a sentence, not a test. The other
    // direction — an expectation naming no step — is refused by `--expect`
    // itself, so a typo there fails the replay above.
    let dir = fernwood();
    let names = named_steps(&dir.join("kitchen.jsonl"));
    let raw = std::fs::read_to_string(dir.join("kitchen.expected.json")).expect("expectations");
    let Value::Object(expected) = serde_json::from_str(&raw).expect("expectations are JSON") else {
        panic!("kitchen.expected.json is not an object")
    };
    let unique: BTreeSet<&String> = names.iter().collect();
    assert_eq!(unique.len(), names.len(), "two steps share a name");
    let unasserted: Vec<&String> = names
        .iter()
        .filter(|n| {
            expected
                .get(*n)
                .and_then(Value::as_object)
                .is_none_or(|o| o.is_empty())
        })
        .collect();
    assert!(
        unasserted.is_empty(),
        "named and never asserted: {unasserted:?}"
    );
}

#[test]
fn the_fernwood_bar_is_unchanged() {
    // The kitchen sits beside the Ostrom scenario and must not move it: the
    // same acts, the same scenario.jsonl, the same expected.json.
    let (ok, text) = replay(&[]);
    assert!(ok, "fernwood no longer replays as expected:\n{text}");
    assert!(text.contains("all as expected"), "{text}");
}
