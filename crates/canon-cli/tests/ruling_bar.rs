// SPDX-License-Identifier: AGPL-3.0-or-later
//! A live ruling, through the real binary: `check` and the MCP tool hear
//! what people said about an action, name who to ask, and end with the
//! command that records the answer; `object`, `approve` and `decide` say it
//! in the verbs a house already knows.
//!
//! The model is a local listener that answers every request with one fixed
//! reading, so what changes between runs is the record and nothing else.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

fn fresh(name: &str, open: bool) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("canon-ruling-bar-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let canon = dir.join(".canon");
    let mut init = vec!["init", "--profile", "house"];
    if open {
        init.push("--open");
    }
    let (code, out) = canon_as(&canon, "human:alex", &init);
    assert_eq!(code, 0, "{out}");
    canon
}

/// Run one verb as one actor against `endpoint`, with `stdin` piped in.
/// Stdout, stderr and the exit code.
fn canon_at(
    dir: &Path,
    actor: &str,
    args: &[&str],
    endpoint: Option<&str>,
    stdin: &str,
) -> (i32, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_canon"));
    cmd.args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", actor)
        .env_remove("CANON_ENDPOINT")
        .env_remove("CANON_MODEL")
        .current_dir(dir.parent().unwrap())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(e) = endpoint {
        cmd.env("CANON_ENDPOINT", e);
    }
    let mut child = cmd.spawn().expect("canon runs");
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
    canon_at(dir, actor, args, None, "")
}

/// `canon check` as `actor`, the model answering with `bearings`.
fn check(dir: &Path, actor: &str, args: &[&str], bearings: &'static str) -> (i32, String) {
    let mut all = vec!["check"];
    all.extend_from_slice(args);
    canon_at(dir, actor, &all, Some(&endpoint(bearings)), "")
}

/// The JSON a check printed, without the stderr line naming the endpoint.
fn json_of(out: &str) -> Value {
    let start = out.find('{').unwrap_or_else(|| panic!("no JSON in: {out}"));
    let end = out.rfind('}').unwrap();
    serde_json::from_str(&out[start..=end]).unwrap_or_else(|e| panic!("{e}: {out}"))
}

/// Output with its wrapping undone, for a phrase the terminal may break.
fn flat(out: &str) -> String {
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn tick() {
    std::thread::sleep(std::time::Duration::from_millis(1100));
}

/// A model that reads every proposal as `bearings`, the JSON list a model
/// returns, e.g. `[]` or `[{"commitment":1,"pull":"toward","because":"…"}]`.
fn endpoint(bearings: &'static str) -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for sock in listener.incoming() {
            let Ok(mut sock) = sock else { return };
            let mut buf = Vec::new();
            let mut tmp = [0u8; 8192];
            loop {
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
                    break;
                }
                match sock.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => buf.extend_from_slice(&tmp[..n]),
                }
            }
            let content = format!("{{\"bearings\":{bearings}}}");
            let body = serde_json::json!({
                "choices": [{ "message": { "role": "assistant", "content": content } }]
            })
            .to_string();
            let _ = write!(
                sock,
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\
                 connection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    format!("http://127.0.0.1:{port}/v1")
}

const NOTHING: &str = "[]";

// ── 1. a live check hears what people said ─────────────────────────────

#[test]
fn a_live_check_counts_what_each_person_last_said_and_not_what_an_agent_said() {
    let dir = fresh("heard", false);
    let (code, out) = canon_as(&dir, "human:alex", &["add", "Bikes live in the shed."]);
    assert_eq!(code, 0, "{out}");

    // Nothing bears on it and nobody has said anything: a gap.
    let (code, out) = check(&dir, "human:sam", &["bikes in the hall"], NOTHING);
    assert_eq!(code, 2, "{out}");

    // An agent's word is advice to a person, never a member's objection.
    let subject = "bikes in the hall";
    let args = ["position", subject, "--against", "-m", "I object"];
    let (code, out) = canon_as(&dir, "agent:helper", &args);
    assert_eq!(code, 0, "{out}");
    let (code, out) = check(&dir, "human:sam", &[subject], NOTHING);
    assert_eq!(code, 2, "an agent's position counted: {out}");
    assert!(!out.contains("agent:helper"), "{out}");

    // A person's is counted, by name and with their reason.
    let args = [
        "position",
        subject,
        "--against",
        "-m",
        "it narrows the hall",
    ];
    let (code, out) = canon_as(&dir, "human:dana", &args);
    assert_eq!(code, 0, "{out}");
    let (code, out) = check(&dir, "human:sam", &[subject], NOTHING);
    assert_eq!(code, 1, "{out}");
    assert!(out.contains("human:dana  objects"), "{out}");
    assert!(flat(&out).contains("it narrows the hall"), "{out}");

    // `--about` names the subject when the proposal is worded differently.
    let (code, out) = check(
        &dir,
        "human:sam",
        &["leave two bikes by the door", "--about", subject, "--json"],
        NOTHING,
    );
    assert_eq!(code, 1, "{out}");
    let v = json_of(&out);
    assert_eq!(v["positions"][0]["source"]["actor"], "human:dana", "{v}");

    // A person who changed their mind holds one position: the latest.
    tick();
    let args = ["position", subject, "--toward", "-m", "a rack fixes it"];
    let (code, out) = canon_as(&dir, "human:dana", &args);
    assert_eq!(code, 0, "{out}");
    let (code, out) = check(&dir, "human:sam", &[subject, "--json"], NOTHING);
    assert_eq!(code, 0, "{out}");
    let v = json_of(&out);
    assert_eq!(v["outcome"], "supported", "{v}");
    assert_eq!(v["positions"].as_array().unwrap().len(), 1, "{v}");
}
