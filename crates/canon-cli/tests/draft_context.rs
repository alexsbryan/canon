// SPDX-License-Identifier: AGPL-3.0-or-later
//! The optional context view uses the recorded passage and keeps the same
//! candidate on screen; looking at evidence is not an act.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[test]
fn a_reviewer_can_open_context_and_then_quit_without_writing_an_act() {
    let dir = std::env::temp_dir().join(format!("canon-draft-context-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let canon = dir.join(".canon");
    std::fs::create_dir_all(&dir).unwrap();
    let binary = env!("CARGO_BIN_EXE_canon");
    let init = Command::new(binary)
        .args(["init", "--profile", "house"])
        .env("CANON_DIR", &canon)
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    std::fs::create_dir_all(canon.join("draft-runs")).unwrap();
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/maple-house/runs/demo-tape/run.json");
    std::fs::copy(source, canon.join("draft-runs/100.json")).unwrap();
    let before = std::fs::read(canon.join("acts.jsonl")).unwrap_or_default();

    let mut child = Command::new(binary)
        .args(["draft", "--resume", "100.json"])
        .env("CANON_DIR", &canon)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"c\nq\n").unwrap();
    let output = child.wait_with_output().unwrap();
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{text}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        text.contains("recorded passage — maple-house.md:3-8"),
        "{text}"
    );
    assert!(
        text.contains("section title, for context only: Maple House Charter"),
        "{text}"
    );
    assert!(
        text.contains("Any single")
            && text.contains("guest may stay no more than two consecutive nights"),
        "{text}"
    );
    assert_eq!(
        text.matches("[c]ontext").count(),
        2,
        "same candidate is still open: {text}"
    );
    assert_eq!(
        std::fs::read(canon.join("acts.jsonl")).unwrap_or_default(),
        before
    );
}
