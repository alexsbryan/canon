// SPDX-License-Identifier: AGPL-3.0-or-later
//! A question, what was said back to it, and who closed it, through the real
//! binary.
//!
//! The source tasks are two: an explanation that was owed and partly given
//! (Gal Oya's omitted design answers; Törbel's reconstruction of an
//! irrigation order), and an organizer who leaves before it is settled. The
//! actors, clocks and words here are synthetic. What is checked is that a
//! reply is found beside the same question by whoever comes next, that it
//! never becomes a rule, and that closing says who closed it and nothing
//! about whether anybody was satisfied.
//!
//! Records are written by hand where the second matters, and each reply's
//! body is built here rather than by the code under test, so the wire is
//! pinned from outside it.

use std::path::{Path, PathBuf};
use std::process::Command;

use canon_core::{sign, Act, ActId, ActKind, Log, Pull, Scope};
use serde_json::{json, Value};

fn canon_as(dir: &Path, actor: &str, args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(args)
        .env("CANON_DIR", dir)
        .env("CANON_ACTOR", actor)
        .output()
        .expect("canon runs");
    let said = [out.stdout, out.stderr].concat();
    (
        out.status.code().unwrap_or(-1),
        // `why` wraps for a terminal; the claims here are about words.
        String::from_utf8_lossy(&said)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" "),
    )
}

fn record(name: &str, acts: Vec<Act>) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("canon-questions-bar-{name}-{}", std::process::id()))
        .join(".canon");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("acts.jsonl"), Log::from_acts(acts).render()).unwrap();
    dir
}

fn ledger(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("acts.jsonl")).unwrap()
}

fn last_id(dir: &Path) -> String {
    let line = ledger(dir).lines().last().unwrap().to_string();
    serde_json::from_str::<Value>(&line).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string()
}

fn why(dir: &Path, q: &ActId) -> String {
    canon_as(dir, "human:reader", &["why", q.as_str()]).1
}

fn view(dir: &Path, q: &ActId) -> Value {
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(["why", q.as_str(), "--json"])
        .env("CANON_DIR", dir)
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

fn grant(holder: &str, ts: i64) -> Act {
    let kind = ActKind::Grant {
        holder: holder.into(),
        scope: Scope::new("canal").unwrap(),
        horizon: None,
        rationale: String::new(),
    };
    Act::new(kind, ts, "human:sam")
}

fn ask(ts: i64, actor: &str) -> Act {
    let kind = ActKind::Question {
        text: "Why is the inlet label absent from the drawing?".into(),
        proposal: None,
    };
    Act::new(kind, ts, actor)
}

fn scoped(q: &ActId, ts: i64) -> Act {
    let kind = ActKind::Scoped {
        commitment: q.clone(),
        scope: Scope::new("canal").unwrap(),
    };
    Act::new(kind, ts, "human:sam")
}

fn close(q: &ActId, ts: i64, actor: &str, why: &str) -> Act {
    let kind = ActKind::Retract {
        target: q.clone(),
        rationale: why.into(),
    };
    Act::new(kind, ts, actor)
}

fn undo(target: &ActId, ts: i64, actor: &str) -> Act {
    let kind = ActKind::Revert {
        targets: vec![target.clone()],
        rationale: "resume".into(),
    };
    Act::new(kind, ts, actor)
}

/// A reply as the wire has it, with `more` laid over the required fields.
fn reply(q: &Act, ts: i64, actor: &str, text: &str, more: Value) -> Act {
    let mut body = json!({
        "schema": "canon-question-response/1",
        "question": {"id": q.id, "sha256": sign::digest(q)},
        "text": text,
    });
    for (k, v) in more.as_object().unwrap() {
        body[k] = v.clone();
    }
    let Value::Object(body) = body else {
        unreachable!()
    };
    let kind = ActKind::Annotation {
        kind: "question_response".into(),
        body,
    };
    Act::new(kind, ts, actor)
}

fn pin(act: &Act) -> Value {
    json!({"id": act.id, "sha256": sign::digest(act)})
}

/// Sam and Theo are in charge of the canal. Ola asked, in the canal's area.
fn canal() -> (Vec<Act>, Act) {
    let q = ask(100, "human:ola");
    let acts = vec![
        grant("human:sam", 10),
        grant("human:theo", 20),
        q.clone(),
        scoped(&q.id, 100),
    ];
    (acts, q)
}

#[test]
fn a_reply_is_kept_beside_the_question_and_makes_nothing_else() {
    let (mut acts, q) = canal();
    let rule = Act::new(
        ActKind::Assert {
            text: "Water is taken in the order of the ditch.".into(),
            from: None,
            source: None,
        },
        50,
        "human:sam",
    );
    acts.push(rule);
    let dir = record("makes-nothing", acts);
    // Everything the canon holds, less the list of what the core carried
    // unread, which is where a reply is honestly reported.
    let rules = |d: &Path| {
        let (_, out) = canon_as(d, "human:reader", &["list", "--json"]);
        let mut all: Value = serde_json::from_str(&out).unwrap();
        all.as_object_mut().unwrap().remove("carried");
        all
    };
    let who = |d: &Path| canon_as(d, "human:reader", &["who", "--json"]).1;
    let (before, standing) = (rules(&dir), who(&dir));

    // The clerk's own account, with nothing but its words.
    let (c, out) = canon_as(
        &dir,
        "human:clerk",
        &["respond", q.id.as_str(), "The copy omitted that label."],
    );
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("makes no rule and closes nothing"), "{out}");
    // Someone else's words, passed on, with what is still owed.
    let (c, out) = canon_as(
        &dir,
        "human:clerk",
        &[
            "respond",
            q.id.as_str(),
            "The label was left off when the drawing was copied.",
            "--attribution",
            "Engineer, as reported by the meeting clerk",
            "--source=meeting-note:section-2",
            "--remaining",
            "Check the intended inlet location with affected farmers",
        ],
    );
    assert_eq!(c, 0, "{out}");

    // What is written: the body the design names, and no more.
    let line: Value = serde_json::from_str(ledger(&dir).lines().last().unwrap()).unwrap();
    assert_eq!(line["op"], "question_response");
    assert_eq!(line["schema"], "canon-question-response/1");
    assert_eq!(line["question"], pin(&q));
    assert_eq!(
        line["actor"], "human:clerk",
        "the recorder, not the engineer"
    );
    assert!(line.get("corrects").is_none() && line.get("status").is_none());

    // No rule, no standing, nothing to approve: the same canon as before.
    assert_eq!(rules(&dir), before);
    assert_eq!(who(&dir), standing);
    let canon = Log::parse(&ledger(&dir)).unwrap().derive();
    assert_eq!(canon.active().count(), 1);
    assert_eq!(canon.proposed().count(), 0);
    assert!(canon.retractions().is_empty() && canon.rulings().is_empty());
    assert_eq!(
        canon.carried().len(),
        2,
        "carried by the core, unread by it"
    );
    assert!(canon.open().any(|x| x.id == q.id), "a reply closes nothing");

    // Read back by somebody who was not there.
    let read = why(&dir, &q.id);
    for said in [
        "open — the canon does not cover this",
        "The copy omitted that label.",
        "said by: not separately stated",
        "source: not supplied",
        "still open: not assessed in this reply",
        "said by: Engineer, as reported by the meeting clerk",
        "source: meeting-note:section-2",
        "still open: Check the intended inlet location with affected farmers",
    ] {
        assert!(read.contains(said), "missing `{said}` in: {read}");
    }
    assert!(!read.contains("answered by"), "{read}");
    // And the list a successor starts from points at it.
    let (_, open) = canon_as(&dir, "human:new", &["open"]);
    assert!(
        open.contains("2 reply(ies)") && open.contains("1 open"),
        "{open}"
    );
    // The note about what this build could not read does not count replies.
    let (_, listed) = canon_as(&dir, "human:new", &["list"]);
    assert!(!listed.contains("carried but not interpreted"), "{listed}");
}

#[test]
fn a_replacement_reads_and_adds_to_the_same_question_and_closes_only_with_the_right() {
    // Ola asked and has gone. Nothing records her leaving, and nothing needs to.
    let (mut acts, q) = canal();
    acts.push(reply(
        &q,
        150,
        "human:ola",
        "The DCO said the copy was wrong.",
        json!({}),
    ));
    let dir = record("turnover", acts);

    // The replacement has no seat. Their account is kept all the same.
    let (c, out) = canon_as(
        &dir,
        "human:new",
        &[
            "respond",
            q.id.as_str(),
            "Farmers at the tail end say it is still unlabelled.",
        ],
    );
    assert_eq!(c, 0, "{out}");
    // Their closing is a proposal, said as one, and the question stays open.
    let (c, out) = canon_as(
        &dir,
        "human:new",
        &["repeal", q.id.as_str(), "-m", "handing over"],
    );
    assert_eq!(c, 0, "{out}");
    assert!(!out.contains("repealed"), "not reported applied: {out}");
    let read = why(&dir, &q.id);
    assert!(
        read.contains("open — the canon does not cover this"),
        "{read}"
    );
    assert!(!read.contains("closed by"), "{read}");
    assert!(
        read.contains("still unlabelled") && read.contains("The DCO said"),
        "{read}"
    );

    // Theo is in charge of the canal: he closes it without Ola, as himself.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let reason = "Stop tracking in this notebook; follow-up remains with the DCO";
    let (c, out) = canon_as(&dir, "human:theo", &["repeal", q.id.as_str(), "-m", reason]);
    assert_eq!(c, 0, "{out}");
    assert!(out.contains("repealed"), "{out}");
    let read = why(&dir, &q.id);
    assert!(
        read.contains(&format!(
            "closed by theo, withdrawn from the active notebook: {reason}"
        )),
        "{read}"
    );
    assert!(
        read.contains("it does not say the question was answered"),
        "{read}"
    );
    assert!(
        read.contains("still unlabelled"),
        "the replies outlive the closing: {read}"
    );
    // Gone from the open list, and one flag away.
    let (_, open) = canon_as(&dir, "human:new", &["open"]);
    assert!(open.contains("no open questions"), "{open}");
    let (_, all) = canon_as(&dir, "human:new", &["open", "--all"]);
    assert!(
        all.contains("closed: tracking stopped") && all.contains("2 reply(ies)"),
        "{all}"
    );

    // Undoing the closing, by the one who closed, opens it again.
    let closing = last_id(&dir);
    let (c, out) = canon_as(
        &dir,
        "human:theo",
        &["undo", &closing, "-m", "Resume tracking"],
    );
    assert_eq!(c, 0, "{out}");
    assert!(why(&dir, &q.id).contains("open — the canon does not cover this"));
}

#[test]
fn an_anonymous_question_is_never_given_an_author() {
    // What `withdraw --ask` leaves: a question by `anonymous`.
    let q = ask(100, canon_core::ANONYMOUS);
    let acts = vec![grant("human:sam", 10), q.clone(), scoped(&q.id, 100)];
    let dir = record("anonymous", acts);
    let (c, out) = canon_as(
        &dir,
        "human:new",
        &["respond", q.id.as_str(), "Nobody has replied yet."],
    );
    assert_eq!(c, 0, "{out}");
    let (c, out) = canon_as(
        &dir,
        "human:new",
        &["repeal", q.id.as_str(), "-m", "mine now"],
    );
    assert_eq!(c, 0, "{out}");
    let read = why(&dir, &q.id);
    assert!(
        read.contains("by anonymous") && read.contains("open —"),
        "{read}"
    );
    std::thread::sleep(std::time::Duration::from_millis(1100));
    let (c, out) = canon_as(
        &dir,
        "human:sam",
        &["repeal", q.id.as_str(), "-m", "no longer needed"],
    );
    assert_eq!(c, 0, "{out}");
    assert!(
        why(&dir, &q.id).contains("closed by sam"),
        "{}",
        why(&dir, &q.id)
    );
}

#[test]
fn a_correction_is_the_same_writers_and_a_lost_one_revives_nothing() {
    let (mut acts, q) = canal();
    let first = reply(
        &q,
        200,
        "human:clerk",
        "The copy omitted the label.",
        json!({}),
    );
    let fixed = reply(
        &q,
        200,
        "human:clerk",
        "The copy omitted the label and the scale.",
        json!({"corrects": pin(&first)}),
    );
    // Another person's account stands beside it; it corrects nobody.
    let other = reply(
        &q,
        300,
        "human:ola",
        "No: the original never had one.",
        json!({}),
    );
    let hijack = reply(
        &q,
        300,
        "human:ola",
        "Ignore the clerk.",
        json!({"corrects": pin(&fixed)}),
    );
    acts.extend([first.clone(), fixed.clone(), other, hijack.clone()]);
    let dir = record("corrections", acts.clone());

    let v = view(&dir, &q.id);
    let replies = v["question_view"]["replies"].as_array().unwrap();
    assert_eq!(replies.len(), 4, "every account is kept");
    let by = |id: &ActId| replies.iter().find(|r| r["id"] == json!(id)).unwrap();
    // Same second, either order: the link is by id and digest.
    assert_eq!(by(&first.id)["corrected_by"], json!(fixed.id));
    assert!(
        by(&fixed.id).get("corrected_by").is_none(),
        "nobody else corrects it"
    );
    let read = why(&dir, &q.id);
    assert!(
        read.contains(&format!("corrected by {}; kept as history", fixed.id)),
        "{read}"
    );
    assert!(
        read.contains(&format!("{} corrects nothing", hijack.id)),
        "{read}"
    );
    assert!(read.contains("No: the original never had one."), "{read}");

    // The clerk undoes the correction. The first account is not handed the
    // word back in silence: the chain is flagged.
    acts.push(undo(&fixed.id, 400, "human:clerk"));
    let dir = record("corrections-undone", acts);
    let read = why(&dir, &q.id);
    assert!(
        read.contains(&format!("not counted: {} was undone", fixed.id)),
        "{read}"
    );
    assert!(
        read.contains(&format!(
            "needs attention: {} corrected {}, and no longer counts",
            fixed.id, first.id
        )),
        "{read}"
    );
    assert!(!read.contains("kept as history"), "{read}");
    let (_, open) = canon_as(&dir, "human:new", &["open"]);
    assert!(open.contains("needs attention"), "{open}");
}

#[test]
fn a_reply_that_cannot_be_trusted_is_named_and_not_counted() {
    let (mut acts, q) = canal();
    // The same id over different words is a different question.
    let wrong = reply(
        &q,
        200,
        "human:a",
        "wrong digest",
        json!({"question": {"id": q.id, "sha256": "0".repeat(64)}}),
    );
    let ahead = reply(
        &q,
        200,
        "human:b",
        "a later form",
        json!({"schema": "canon-question-response/2"}),
    );
    let extra = reply(
        &q,
        200,
        "human:c",
        "a field nobody defined",
        json!({"status": "resolved"}),
    );
    let blank = reply(&q, 200, "human:d", "  ", json!({}));
    let early = reply(&q, 50, "human:e", "before it was asked", json!({}));
    let gone = reply(
        &q,
        300,
        "human:f",
        "corrects a reply that is not here",
        json!({"corrects": {"id": "can-0123456789ab", "sha256": "1".repeat(64)}}),
    );
    let good = reply(&q, 300, "human:g", "the one that counts", json!({}));
    // Dated after any clock this test runs under.
    let future = reply(
        &q,
        4_102_444_800,
        "human:h",
        "from the year 2100",
        json!({}),
    );
    acts.extend([
        wrong.clone(),
        ahead.clone(),
        extra.clone(),
        blank.clone(),
        early.clone(),
        gone.clone(),
        good,
        future,
    ]);
    let dir = record("trust", acts);

    let v = view(&dir, &q.id);
    let ids: Vec<&str> = (v["question_view"]["replies"].as_array().unwrap().iter())
        .map(|r| r["text"].as_str().unwrap())
        .collect();
    assert_eq!(
        ids,
        ["corrects a reply that is not here", "the one that counts"]
    );
    let read = why(&dir, &q.id);
    for (act, why_not) in [
        (&wrong, "pins a different question"),
        (&ahead, "is in a form this build does not read"),
        (&extra, "is in a form this build does not read"),
        (&blank, "is in a form this build does not read"),
        (&early, "is dated before the question"),
    ] {
        assert!(
            read.contains(&format!("not counted: {} {why_not}", act.id)),
            "{why_not}: {read}"
        );
    }
    assert!(
        read.contains(&format!(
            "needs attention: {} corrects can-0123456789ab, which is not available",
            gone.id
        )),
        "{read}"
    );
    assert!(
        !read.contains("2100"),
        "held back at this reader's clock: {read}"
    );
    // The raw record is untouched by being read, and the core still reads it.
    assert_eq!(
        Log::parse(&ledger(&dir)).unwrap().derive().carried().len(),
        8
    );
}

#[test]
fn a_reply_the_writer_would_refuse_is_never_appended() {
    let (mut acts, q) = canal();
    let theirs = reply(&q, 200, "human:ola", "Ola's account.", json!({}));
    acts.push(theirs.clone());
    let dir = record("refusals", acts);
    let before = ledger(&dir);
    let id = q.id.as_str();
    for refused in [
        &["respond", id][..],
        &["respond", id, "  "],
        &["respond", "can-ffffffffffff", "to nothing"],
        &["respond", id, "words", "--source", " "],
        &["respond", id, "words", "--remaining="],
        // A correction is to your own reply, and to one that exists.
        &["respond", id, "words", "--corrects", theirs.id.as_str()],
        &["respond", id, "words", "--corrects", "can-ffffffffffff"],
        &["respond", id, "words", "--close"],
    ] {
        let (c, out) = canon_as(&dir, "human:clerk", refused);
        assert_ne!(c, 0, "{refused:?}: {out}");
    }
    assert_eq!(ledger(&dir), before);
}

#[test]
fn the_reason_shown_is_the_withdrawal_in_force_or_every_one_that_fits() {
    let (mut acts, q) = canal();
    let first = close(&q.id, 200, "human:theo", "stop tracking");
    let second = close(&q.id, 300, "human:sam", "duplicate");
    acts.extend([first, second.clone(), undo(&second.id, 400, "human:sam")]);
    let dir = record("overlap", acts);
    let read = why(&dir, &q.id);
    assert!(
        read.contains("closed by theo, withdrawn from the active notebook: stop tracking"),
        "{read}"
    );
    assert!(
        !read.contains("duplicate"),
        "the newest line is not the one in force: {read}"
    );

    // Two in one second: both shown, neither named.
    let (mut acts, q) = canal();
    acts.push(close(&q.id, 200, "human:theo", "stop tracking"));
    acts.push(close(&q.id, 200, "human:sam", "duplicate"));
    let dir = record("same-second", acts);
    let read = why(&dir, &q.id);
    assert!(
        read.contains("2 withdrawals fit and the record does not say which is in force"),
        "{read}"
    );
    assert!(
        read.contains("theo: stop tracking") && read.contains("sam: duplicate"),
        "{read}"
    );
    assert!(!read.contains("closed by"), "{read}");
}

#[test]
fn an_old_question_keeps_its_meaning_and_gains_its_reason() {
    // Probe B, first mapping: a reply written as `amend`. It is a rule, it
    // was a rule, and no reading of it as a fact is offered.
    let (mut acts, q) = canal();
    let answer = Act::new(
        ActKind::Supersede {
            text: "The copy omitted that label.".into(),
            old: vec![q.id.clone()],
            rationale: "reply".into(),
        },
        200,
        "human:sam",
    );
    acts.extend([
        answer.clone(),
        reply(&q, 300, "human:clerk", "Said again as a fact.", json!({})),
    ]);
    let dir = record("old-amend", acts);
    let v = view(&dir, &q.id);
    assert_eq!(
        v["record"]["status"],
        json!({"status": "superseded", "by": answer.id})
    );
    let (_, rules) = canon_as(&dir, "human:reader", &["list"]);
    assert!(
        rules.contains("The copy omitted that label."),
        "still a rule: {rules}"
    );
    let (_, all) = canon_as(&dir, "human:reader", &["open", "--all"]);
    assert!(
        all.contains(&format!("answered by the rule {}", answer.id)),
        "{all}"
    );

    // Probe B, second mapping: `repeal Q -m REPLY`, written before replies
    // existed. Its status is what it was; its reason can now be read.
    let (mut acts, q) = canal();
    acts.push(close(
        &q.id,
        200,
        "human:theo",
        "The copy omitted that label.",
    ));
    let dir = record("old-repeal", acts.clone());
    let v = view(&dir, &q.id);
    assert_eq!(
        v["record"]["status"],
        json!({"status": "retracted", "at": 200})
    );
    let read = why(&dir, &q.id);
    assert!(
        read.contains("withdrawn 1970-01-01"),
        "the old line stays: {read}"
    );
    assert!(
        read.contains(
            "closed by theo, withdrawn from the active notebook: The copy omitted that label."
        ),
        "{read}"
    );
    let (_, open) = canon_as(&dir, "human:reader", &["open"]);
    assert!(open.contains("no open questions"), "{open}");

    // Something said after it closed flags it, and changes nothing else.
    acts.push(reply(
        &q,
        300,
        "human:ola",
        "The second copy has no label either.",
        json!({}),
    ));
    let dir = record("old-repeal-news", acts);
    let v = view(&dir, &q.id);
    assert_eq!(
        v["record"]["status"],
        json!({"status": "retracted", "at": 200}),
        "not reopened"
    );
    let (_, open) = canon_as(&dir, "human:reader", &["open"]);
    assert!(
        open.contains("needs attention: new information since it was closed"),
        "{open}"
    );
    assert!(open.contains("0 open, 1 more shown"), "{open}");
}

#[test]
fn approving_a_proposed_closing_is_what_closes_it() {
    let (mut acts, q) = canal();
    let put = close(&q.id, 200, "human:new", "I think this is done");
    let yes = ActKind::Position {
        about: put.id.to_string(),
        citing: None,
        pull: Pull::Toward,
        because: "yes".into(),
    };
    acts.extend([put, Act::new(yes, 300, "human:theo")]);
    let dir = record("approved", acts);
    let read = why(&dir, &q.id);
    assert!(
        read.contains("closed by new, withdrawn from the active notebook: I think this is done"),
        "{read}"
    );
}
