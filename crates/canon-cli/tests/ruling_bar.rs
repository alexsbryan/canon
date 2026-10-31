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

/// The `canon_check` MCP tool, called once over stdio. Its text and whether
/// it was an error.
fn mcp_check(dir: &Path, arguments: Value, bearings: &'static str) -> (String, bool) {
    let call = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": "canon_check", "arguments": arguments },
    });
    let (_, out) = canon_at(
        dir,
        "agent:helper",
        &["mcp"],
        Some(&endpoint(bearings)),
        &format!("{call}\n"),
    );
    let line = out.lines().find(|l| l.starts_with('{')).expect("a reply");
    let reply: Value = serde_json::from_str(line).unwrap();
    let result = &reply["result"];
    (
        result["content"][0]["text"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        result["isError"].as_bool().unwrap_or(false),
    )
}

fn id_of(out: &str) -> String {
    out.split_whitespace()
        .find(|w| w.starts_with("can-"))
        .unwrap_or_else(|| panic!("no id in: {out}"))
        .to_string()
}

/// The acts as `canon log --json` prints them, one per line.
fn acts(dir: &Path) -> Vec<Value> {
    let (code, out) = canon_as(dir, "human:alex", &["log", "--json"]);
    assert_eq!(code, 0, "{out}");
    out.lines()
        .filter(|l| l.starts_with('{'))
        .map(|l| serde_json::from_str(l).expect("one act per line"))
        .collect()
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
const SHED: &str = r#"[{"commitment":1,"pull":"toward","because":"the shed has room"}]"#;

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

// ── 2. the MCP tool asks the room's own rule ───────────────────────────

#[test]
fn the_mcp_check_takes_a_scope_and_a_subject_and_decides_by_the_rooms_rule() {
    let dir = fresh("room", false);
    let (code, out) = canon_as(&dir, "human:alex", &["add", "Bikes live in the shed."]);
    assert_eq!(code, 0, "{out}");
    let args = [
        "policy",
        "set",
        "consent",
        "--scope",
        "house.kitchen",
        "-m",
        "we consent",
    ];
    let (code, out) = canon_as(&dir, "human:alex", &args);
    assert_eq!(code, 0, "{out}");

    // The call shape every agent already uses: the canon-wide rule.
    let proposal = serde_json::json!({ "proposal": "a bike rack by the fridge" });
    let (text, error) = mcp_check(&dir, proposal, SHED);
    assert!(!error, "{text}");
    assert!(text.contains("default:"), "{text}");

    // In the kitchen, the kitchen's rule.
    let kitchen = serde_json::json!({
        "proposal": "a bike rack by the fridge",
        "scope": "house.kitchen",
    });
    let (text, error) = mcp_check(&dir, kitchen.clone(), SHED);
    assert!(!error, "{text}");
    assert!(text.contains("consent: no objection"), "{text}");

    // And what people said about the subject, named apart from the words.
    let args = [
        "position",
        "bikes indoors",
        "--against",
        "-m",
        "the floor is wet",
    ];
    let (code, out) = canon_as(&dir, "human:dana", &args);
    assert_eq!(code, 0, "{out}");
    let (text, _) = mcp_check(&dir, kitchen, SHED);
    assert!(!text.contains("human:dana"), "a different subject: {text}");
    let about = serde_json::json!({
        "proposal": "a bike rack by the fridge",
        "scope": "house.kitchen",
        "about": "bikes indoors",
    });
    let (text, _) = mcp_check(&dir, about, SHED);
    assert!(text.contains("consent: 1 reasoned objection"), "{text}");
    assert!(text.contains("human:dana"), "{text}");

    // A scope that is not one is refused, not read as the canon as a whole.
    let bad = serde_json::json!({ "proposal": "a bike rack", "scope": "house..kitchen" });
    let (text, error) = mcp_check(&dir, bad, SHED);
    assert!(error, "{text}");
    assert!(text.contains("is not a scope"), "{text}");
}

// ── 3. every "ask" names who to ask ────────────────────────────────────

#[test]
fn an_ask_names_the_holders_of_the_narrowest_scope_and_a_go_ahead_says_so() {
    let dir = fresh("whom", false);
    let (code, out) = canon_as(&dir, "human:alex", &["add", "Bikes live in the shed."]);
    assert_eq!(code, 0, "{out}");

    // Nothing in the canon bears on it: ask whoever holds the canon. Said
    // in a fresh house that never adopted a rule, and without its jargon.
    let (code, out) = check(&dir, "human:sam", &["bikes in the hall"], NOTHING);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("\nask human:alex first"), "{out}");
    assert!(!out.contains("standing"), "{out}");
    let (_, out) = check(&dir, "human:sam", &["bikes in the hall", "--json"], NOTHING);
    assert_eq!(
        json_of(&out)["ask"],
        serde_json::json!(["human:alex"]),
        "{out}"
    );
    let bare = serde_json::json!({ "proposal": "bikes in the hall" });
    let (text, _) = mcp_check(&dir, bare, NOTHING);
    assert!(
        text.contains("ask human:alex first"),
        "an agent is told too: {text}"
    );

    // In the kitchen, the kitchen's holders, not the house's.
    for who in ["human:dana", "human:sam"] {
        let (code, out) = canon_as(&dir, "human:alex", &["grant", who, "house.kitchen"]);
        assert_eq!(code, 0, "{out}");
    }
    let kitchen = ["a rack by the fridge", "--scope", "house.kitchen"];
    let (_, out) = check(&dir, "human:ola", &kitchen, NOTHING);
    assert!(
        out.contains("ask one of human:dana, human:sam first"),
        "{out}"
    );
    let mut json = kitchen.to_vec();
    json.push("--json");
    let (_, out) = check(&dir, "human:ola", &json, NOTHING);
    let v = json_of(&out);
    assert_eq!(
        v["ask"],
        serde_json::json!(["human:dana", "human:sam"]),
        "{v}"
    );

    // Nothing to ask: go ahead, and no `ask` to read.
    let (code, out) = check(&dir, "human:ola", &kitchen, SHED);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("\ngo ahead"), "{out}");
    let (_, out) = check(&dir, "human:ola", &json, SHED);
    assert!(json_of(&out).get("ask").is_none(), "{out}");

    // Nobody holds anything: any person can answer.
    let open = fresh("whom-open", true);
    let (code, out) = canon_as(&open, "human:alex", &["add", "Bikes live in the shed."]);
    assert_eq!(code, 0, "{out}");
    let (_, out) = check(&open, "human:sam", &["bikes in the hall"], NOTHING);
    assert!(out.contains("anyone can answer"), "{out}");
    let (_, out) = check(
        &open,
        "human:sam",
        &["bikes in the hall", "--json"],
        NOTHING,
    );
    assert_eq!(json_of(&out)["ask"], serde_json::json!([]), "{out}");
}

// ── 4. object and approve, about an action ─────────────────────────────

#[test]
fn object_and_approve_about_an_action_write_positions_a_check_counts() {
    let dir = fresh("verbs", false);
    let (code, out) = canon_as(&dir, "human:alex", &["add", "Bikes live in the shed."]);
    assert_eq!(code, 0, "{out}");
    let rule = id_of(&out);
    let subject = "bikes in the hall";

    // An objection still needs its reason.
    let (code, out) = canon_as(&dir, "human:dana", &["object", "--about", subject]);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("needs a reason"), "{out}");

    let args = [
        "object",
        "--about",
        subject,
        "--citing",
        &rule,
        "-m",
        "it narrows the hall",
    ];
    let (code, out) = canon_as(&dir, "human:dana", &args);
    assert_eq!(code, 0, "{out}");
    let (code, out) = canon_as(&dir, "human:sam", &["approve", "--about", subject]);
    assert_eq!(code, 0, "{out}");
    let log = acts(&dir);
    // Written in the same second, so found by who wrote them, not by order.
    let by = |who: &str| {
        let mine = log
            .iter()
            .filter(|a| a["about"] == subject && a["actor"] == who);
        mine.cloned().collect::<Vec<Value>>()
    };
    let (dana, sam) = (by("human:dana"), by("human:sam"));
    assert_eq!((dana.len(), sam.len()), (1, 1), "{log:?}");
    assert_eq!(dana[0]["op"], "position");
    assert_eq!(dana[0]["pull"], "against");
    assert_eq!(dana[0]["citing"], rule.as_str());
    assert_eq!(dana[0]["because"], "it narrows the hall");
    assert_eq!(sam[0]["pull"], "toward");

    // And a check about the action hears both.
    let (code, out) = check(&dir, "human:ola", &[subject, "--json"], NOTHING);
    assert_eq!(code, 1, "{out}");
    assert_eq!(
        json_of(&out)["positions"].as_array().unwrap().len(),
        2,
        "{out}"
    );

    // A mistyped proposal id is refused, never read as a subject.
    let before = acts(&dir).len();
    let (code, out) = canon_as(&dir, "human:dana", &["object", "can-0000dead", "-m", "no"]);
    assert_ne!(code, 0, "{out}");
    let (code, out) = canon_as(&dir, "human:dana", &["approve", "bikes in the hall"]);
    assert_ne!(code, 0, "{out}");
    // Nor is an id beside `--about`, or a citation on a proposal.
    let both = ["object", "can-0000dead", "--about", subject, "-m", "no"];
    let (code, out) = canon_as(&dir, "human:dana", &both);
    assert_ne!(code, 0, "{out}");
    assert!(out.contains("usage: canon object"), "{out}");
    let cited = ["approve", "can-0000dead", "--citing", &rule];
    let (code, out) = canon_as(&dir, "human:dana", &cited);
    assert_ne!(code, 0, "{out}");
    assert_eq!(acts(&dir).len(), before, "a refusal wrote something");

    // `position` keeps working for whoever already types it, and says
    // what it is called now; the long help no longer offers it.
    tick();
    let old = ["position", subject, "--toward", "-m", "a rack fixes it"];
    let (code, out) = canon_as(&dir, "human:dana", &old);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("`canon position` is `canon object --about` / `canon approve --about` now"),
        "{out}"
    );
    assert_eq!(acts(&dir).len(), before + 1);
    let (_, help) = canon_as(&dir, "human:dana", &["help", "all"]);
    assert!(!help.contains("  position "), "{help}");
    assert!(help.contains("--about"), "{help}");
}
