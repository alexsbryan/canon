// SPDX-License-Identifier: AGPL-3.0-or-later
//! Endpoint-client tests against a mock server.
//!
//! Every rung of the structured-output ladder is exercised here, including
//! the two that must NOT happen: a non-schema 4xx must not burn a second
//! call, and prose must never be parsed. The mock records request bodies, so
//! the assertions are about what actually went over the wire rather than
//! about what the code intended to send.

use serde::Deserialize;
use serde_json::{json, Value};

use super::*;
use crate::testing::{completion, Mock};

#[derive(Debug, Deserialize, PartialEq, Eq)]
struct Pairs {
    pairs: Vec<String>,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": { "pairs": { "type": "array", "items": { "type": "string" } } },
        "required": ["pairs"],
    })
}

fn ask(client: &Client) -> Result<Pairs, ModelError> {
    client.complete_json("system", "user", "pairs", &schema())
}

#[test]
fn a_schema_answer_comes_back_as_typed_data_in_one_call() {
    let mock = Mock::spawn(vec![(200, completion(r#"{"pairs":["a","b"]}"#))]);
    let got = ask(&mock.client()).expect("answer");
    assert_eq!(got.pairs, vec!["a".to_string(), "b".to_string()]);

    let reqs = mock.requests();
    assert_eq!(reqs.len(), 1, "the happy path must not retry");
    assert_eq!(reqs[0]["response_format"]["type"], "json_schema");
    assert_eq!(reqs[0]["temperature"], 0.0, "adjudication is pinned");
}

#[test]
fn a_schema_refusal_degrades_exactly_once_and_states_the_schema_in_the_prompt() {
    let mock = Mock::spawn(vec![
        (
            400,
            json!({ "error": { "message": "response_format.json_schema is not supported by this server" } })
                .to_string(),
        ),
        (200, completion(r#"{"pairs":["a"]}"#)),
    ]);
    let got = ask(&mock.client()).expect("answer after degrading");
    assert_eq!(got.pairs, vec!["a".to_string()]);

    let reqs = mock.requests();
    assert_eq!(reqs.len(), 2, "the ladder has exactly two rungs");
    assert_eq!(reqs[0]["response_format"]["type"], "json_schema");
    assert_eq!(reqs[1]["response_format"]["type"], "json_object");
    // Dropping to json_object drops the endpoint's enforcement, so the schema
    // has to travel in the prompt instead or the reply is unconstrained.
    let user = reqs[1]["messages"][1]["content"].as_str().unwrap();
    assert!(
        user.contains("\"pairs\""),
        "rung 2 must state the schema: {user}"
    );
}

#[test]
fn a_refusal_that_is_not_about_the_schema_does_not_burn_a_second_call() {
    // A 400 for an unknown model must be reported as an unknown model. Climbing
    // down the ladder here would spend a second call and name the wrong cause.
    let mock = Mock::spawn(vec![(
        400,
        json!({ "error": { "message": "model `nope` not found" } }).to_string(),
    )]);
    let err = ask(&mock.client()).expect_err("must not succeed");
    assert!(matches!(err, ModelError::Refused { status: 400, .. }));
    assert_eq!(mock.requests().len(), 1);
    assert_eq!(err.exit_code(), 3);
}

#[test]
fn prose_is_never_parsed_as_a_fallback() {
    // The whole point of exit 3. An endpoint that answers in English has not
    // answered; scraping a verdict out of a paragraph is the failure §18.3
    // exists to prevent.
    let mock = Mock::spawn(vec![(
        200,
        completion("There are no tensions between these commitments, in my view."),
    )]);
    let err = ask(&mock.client()).expect_err("prose is not an answer");
    assert!(matches!(err, ModelError::Malformed { .. }), "{err:?}");
    assert_eq!(err.exit_code(), 3);
    assert!(err.to_string().contains("There are no tensions"));
}

#[test]
fn a_declining_model_is_a_refusal_not_a_malformed_reply() {
    let mock = Mock::spawn(vec![(
        200,
        json!({ "choices": [{ "message": { "refusal": "I can't help with that." } }] }).to_string(),
    )]);
    let err = ask(&mock.client()).expect_err("a decline is not an answer");
    assert!(
        matches!(&err, ModelError::Refused { detail, .. } if detail.contains("I can't help")),
        "{err:?}"
    );
}

#[test]
fn a_fenced_object_decodes_because_the_fence_is_transport_not_prose() {
    let mock = Mock::spawn(vec![(200, completion("```json\n{\"pairs\":[\"x\"]}\n```"))]);
    assert_eq!(ask(&mock.client()).unwrap().pairs, vec!["x".to_string()]);
}

#[test]
fn an_unreachable_endpoint_is_a_transport_error_naming_the_url() {
    // Port 1 on loopback: nothing listens, and the connect fails fast.
    let client = Client::new(&Config {
        endpoint: Some("http://127.0.0.1:1/v1".into()),
        model: None,
        extract_model: None,
    })
    .unwrap();
    let err = ask(&client).expect_err("nothing is listening");
    assert!(matches!(err, ModelError::Transport { .. }), "{err:?}");
    assert!(err.to_string().contains("127.0.0.1:1"));
}

#[test]
fn a_missing_endpoint_is_reported_not_guessed_at() {
    let err = Client::new(&Config::default()).expect_err("no endpoint");
    assert!(matches!(err, ModelError::NoEndpoint));
    assert_eq!(err.exit_code(), 3);
}

#[test]
fn locality_is_decided_conservatively() {
    let local = [
        "http://localhost:8080/v1",
        "http://127.0.0.1:9741/v1",
        "http://127.1.2.3/v1",
        "http://[::1]:8080/v1",
        "http://dev.localhost:8080/v1",
    ];
    let remote = [
        "https://api.example.com/v1",
        "https://localhost.example.com/v1",
        "http://user@10.0.0.4:8080/v1",
        "http://127.0.0.1.example.com/v1",
    ];
    for e in local {
        let c = Client::new(&Config {
            endpoint: Some(e.into()),
            model: None,
            extract_model: None,
        })
        .unwrap();
        assert!(c.is_local(), "{e} should be local (host {})", c.host());
        assert!(c.describe().contains("local"));
    }
    for e in remote {
        let c = Client::new(&Config {
            endpoint: Some(e.into()),
            model: None,
            extract_model: None,
        })
        .unwrap();
        assert!(!c.is_local(), "{e} should be remote (host {})", c.host());
        let err = c.require_local().expect_err("must refuse");
        // Exit 2, not 3: the tool could judge — the user has to say the text
        // may leave the machine.
        assert_eq!(err.exit_code(), 2);
    }
}

// ── a reasoning wrapper is a transport shell, not an answer ──

#[test]
fn a_reply_that_closes_a_thought_after_answering_still_decodes() {
    // Measured, not imagined: a Qwen3-family 4B on this endpoint extracted
    // all 24 passages of the Maple House fixture and then killed the run at
    // the reduce step, three times identically, with the right JSON followed
    // by a stray `</think>`. The server had consumed the opening tag.
    let v = decode("{\n  \"rules\": [{\"n\": 1, \"same_as\": 2}]\n}\n</think>").unwrap();
    assert_eq!(v["rules"][0]["n"], 1);
}

#[test]
fn a_reply_that_thinks_before_answering_still_decodes() {
    let v = decode("<think>the second restates the first</think>\n{\"rules\":[]}").unwrap();
    assert!(v["rules"].as_array().unwrap().is_empty());
}

#[test]
fn the_other_channel_names_decode_too() {
    for tag in ["thinking", "reasoning"] {
        let raw = format!("<{tag}>weighing it up</{tag}>{{\"ok\":true}}");
        assert_eq!(decode(&raw).unwrap()["ok"], true, "{tag}");
    }
}

#[test]
fn a_fenced_answer_inside_a_thought_decodes() {
    let v = decode("<think>x</think>\n```json\n{\"ok\":true}\n```").unwrap();
    assert_eq!(v["ok"], true);
}

#[test]
fn prose_with_no_json_in_it_is_still_malformed() {
    // The wrapper is unwrapped; the reply is not hunted through. A reply
    // with no JSON anywhere must fail loudly and carry its raw text, or a
    // broken endpoint reads as an empty answer (§18.3).
    let e = decode("<think>I am not going to answer that</think> sorry").unwrap_err();
    assert!(matches!(e, ModelError::Malformed { .. }), "{e:?}");
    let e = decode("no json here at all").unwrap_err();
    assert!(matches!(e, ModelError::Malformed { .. }), "{e:?}");
}

// ── a position a model answered with ────────────────────────

#[test]
fn a_sentinel_position_is_refused_and_not_fatal() {
    // Measured on this endpoint: a Qwen3-family 4B answers `same_as: -1` for
    // a rule that duplicates nothing. As a `usize` field that was a
    // deserialization error which killed the whole reduce call and threw
    // away twenty-six good candidates — a partial answer reported as no
    // answer, which is the failure §18.3 names.
    #[derive(Debug, serde::Deserialize)]
    struct Reply {
        n: Pos,
        same_as: Pos,
    }
    let r: Reply = serde_json::from_str(r#"{"n":2,"same_as":-1}"#).unwrap();
    assert_eq!(r.n.get(), Some(2));
    assert_eq!(r.same_as.get(), None, "-1 is not a position");
    // And a warning about it can still say what the model actually answered.
    assert_eq!(r.same_as.to_string(), "-1");
}

#[test]
fn an_omitted_position_is_not_the_first_one() {
    // Zero is deliberately not a position: markers are 1-based, so a field
    // the answer left out is refused by the same range check that refuses
    // one it got wrong — never read as "the first sentence".
    #[derive(Debug, serde::Deserialize)]
    struct Reply {
        #[serde(default)]
        first: Pos,
    }
    let r: Reply = serde_json::from_str("{}").unwrap();
    assert_eq!(r.first.get(), Some(0));
}

// ── the tape: record once, replay for nothing ───────────────
//
// The measurement this exists for: three arms on the maple-house bar cost
// about three hours of 27B time on 2026-08-24, and every one re-paid
// extraction — 24 of ~36 calls — for a change that acted after it.

#[test]
fn a_recording_keeps_what_the_server_said_in_order() {
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "pairs": ["a"] }).to_string())),
        (200, completion(&json!({ "pairs": ["b"] }).to_string())),
    ]);
    let client = mock.client().recording();
    for _ in 0..2 {
        ask(&client).unwrap();
    }
    let tape = client.tape();
    assert_eq!(tape.len(), 2);
    assert!(tape.iter().all(|e| e.path == "chat/completions"));
    assert!(tape[0].raw.contains("\\\"a\\\"") || tape[0].raw.contains("a"));
    assert!(tape[1].raw.contains("b"));
}

#[test]
fn a_replay_answers_from_the_tape_and_never_reaches_the_endpoint() {
    // The endpoint is deliberately somewhere nothing listens. A replay that
    // reached the network would fail here rather than pass quietly.
    let mock = Mock::spawn(vec![(
        200,
        completion(&json!({ "pairs": ["only"] }).to_string()),
    )]);
    let live = mock.client().recording();
    ask(&live).unwrap();

    let replayed = Client::replaying("http://127.0.0.1:1/v1", "primary", live.tape());
    let got = ask(&replayed).unwrap();
    assert_eq!(got.pairs, vec!["only".to_string()]);
}

#[test]
fn a_leg_shares_the_runs_tape_rather_than_starting_its_own() {
    // Caught while building this: `with_model` handed the extract leg a client
    // with no tape, so the 24 extraction calls — the expensive two thirds of a
    // run — were silently absent from every recording.
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "pairs": ["leg"] }).to_string())),
        (200, completion(&json!({ "pairs": ["base"] }).to_string())),
    ]);
    let base = mock.client().recording();
    let leg = base.with_model("fast");
    ask(&leg).unwrap();
    ask(&base).unwrap();
    assert_eq!(base.tape().len(), 2, "the leg's call is on the run's tape");
    assert_eq!(
        leg.model(),
        "fast",
        "and it still went to the leg's own slot"
    );
}

#[test]
fn a_tape_that_runs_out_refuses_rather_than_inventing_a_reply() {
    // A build that makes MORE calls than were recorded has changed the call
    // sequence, and replay cannot judge that. Answering anything here would be
    // a number about neither build (§18.3).
    let replayed = Client::replaying("http://127.0.0.1:1/v1", "primary", vec![]);
    let err = ask(&replayed).expect_err("an empty tape cannot answer");
    assert_eq!(err.exit_code(), 3, "cannot judge, not an ordinary error");
    assert!(format!("{err}").contains("exhausted"), "{err}");
}

#[test]
fn a_tape_whose_calls_went_elsewhere_refuses() {
    let tape = vec![TapeEntry {
        path: "embeddings".into(),
        stage: "embeddings".into(),
        raw: completion(&json!({ "pairs": [] }).to_string()),
        status: 200,
    }];
    let replayed = Client::replaying("http://127.0.0.1:1/v1", "primary", tape);
    let err =
        ask(&replayed).expect_err("a completion must not be answered from an embeddings call");
    assert!(format!("{err}").contains("out of step"), "{err}");
}

#[test]
fn a_tape_cut_at_a_stage_plays_above_it_and_goes_live_from_it() {
    // The loop this exists for: the comparison stage is 10 of ~36 calls, so an
    // arm on it should cost the 10 and not the 36. Everything above `tensions`
    // comes off the tape; `tensions` itself is a real call to the mock.
    let mock = Mock::spawn(vec![(
        200,
        completion(&json!({ "pairs": ["live"] }).to_string()),
    )]);
    let tape = vec![TapeEntry {
        path: "chat/completions".into(),
        stage: "commitments".into(),
        raw: completion(&json!({ "pairs": ["taped"] }).to_string()),
        status: 200,
    }];
    let client = mock.client().playing(tape, Some("tensions".into()));

    let above: Pairs = client
        .complete_json("s", "u", "commitments", &schema())
        .unwrap();
    assert_eq!(
        above.pairs,
        vec!["taped".to_string()],
        "above the cut: the tape answers"
    );

    let at: Pairs = client
        .complete_json("s", "u", "tensions", &schema())
        .unwrap();
    assert_eq!(
        at.pairs,
        vec!["live".to_string()],
        "at the cut: the endpoint answers"
    );
    assert_eq!(
        mock.requests().len(),
        1,
        "and only the live stage cost a call"
    );
}

#[test]
fn a_stage_label_that_disagrees_with_the_recording_refuses() {
    let tape = vec![TapeEntry {
        path: "chat/completions".into(),
        stage: "groups".into(),
        raw: completion(&json!({ "pairs": [] }).to_string()),
        status: 200,
    }];
    let replayed = Client::replaying("http://127.0.0.1:1/v1", "primary", tape);
    let err = replayed
        .complete_json::<Pairs>("s", "u", "commitments", &schema())
        .expect_err("an extraction must not be answered from a dedupe call");
    assert!(format!("{err}").contains("out of step"), "{err}");
}

// ── backpressure ────────────────────────────────────────────

#[test]
fn a_busy_host_is_told_apart_from_one_that_declined() {
    // The whole decision: waiting helps a host that is over capacity and
    // cannot help one that has refused the request itself. Retrying the
    // second burns a call to learn what it already knows (§18.3).
    assert!(is_backpressure(429, ""), "rate limited, unambiguously");
    assert!(
        is_backpressure(
            503,
            r#"{"error":"host busy: ~7000 ms predicted wait at queue position 3"}"#
        ),
        "the queue is full and the request never ran"
    );
    assert!(
        is_backpressure(500, r#"{"error":"host busy: ~4000 ms predicted wait"}"#),
        "a 500 that names the queue is still backpressure"
    );
    assert!(!is_backpressure(400, "bad request"));
    assert!(!is_backpressure(404, "no such model"));
    // The one that makes the status alone useless. This daemon answers 503
    // for a request that RAN and blew its deadline as well as for a full
    // queue, and retrying the former spends another 300s on a request
    // already known to be too expensive.
    assert!(
        !is_backpressure(
            503,
            r#"{"error":{"message":"local inference failed: inference deadline exceeded after 300s","type":"backend_error"}}"#
        ),
        "a blown deadline is not the host asking us to come back"
    );
    // The one that matters: a 200 carrying a model refusal. `post` maps a
    // declining model to Refused{status: 200}, and no amount of waiting
    // changes its mind — retrying it would double every refusal.
    assert!(
        !is_backpressure(200, "the model declined: host busy is not a thing I do"),
        "a model declining is not the host asking us to come back"
    );
}

#[test]
fn backoff_grows_then_caps_then_gives_up() {
    use std::time::Duration;
    let secs = |a: usize, w: u64| backoff(a, Duration::from_secs(w)).map(|d| d.as_secs());
    assert_eq!(
        secs(1, 0),
        Some(2),
        "starts small — a spike may clear at once"
    );
    assert_eq!(secs(2, 2), Some(4));
    assert_eq!(secs(3, 6), Some(8));
    assert_eq!(secs(4, 14), Some(16));
    assert_eq!(
        secs(5, 30),
        Some(30),
        "capped, so it polls rather than sleeps"
    );
    assert_eq!(
        secs(9, 30),
        Some(30),
        "and stays capped however long it runs"
    );
    // The budget is what stops a retry becoming a hang.
    assert_eq!(secs(6, 300), None, "budget spent");
    assert_eq!(
        secs(6, 290),
        Some(10),
        "the last wait is trimmed to what is left"
    );
    assert_eq!(
        secs(0, 0),
        None,
        "attempt 0 is the original call, not a retry"
    );
}

#[test]
fn a_busy_host_is_waited_out_rather_than_counted_as_a_failure() {
    // Observed 2026-08-26: the daemon shed load for about eight minutes
    // during a founding sweep's comparison stage. Nothing waited, so 588 of
    // 690 passes came back refused inside eight minutes and the run reported
    // 15% coverage. One retry is the difference between a slow run and a
    // meaningless one.
    let busy = r#"{"error":"host busy: ~7000 ms predicted wait at queue position 3","reason":"local_queue"}"#;
    let mock = Mock::spawn(vec![
        (503, busy.to_string()),
        (200, completion(r#"{"pairs":["a"]}"#)),
    ]);
    let got = ask(&mock.client()).expect("the retry answers");
    assert_eq!(got.pairs, vec!["a".to_string()]);
    assert_eq!(
        mock.requests().len(),
        2,
        "the call was made twice: once refused, once answered"
    );
}

#[test]
fn a_refusal_that_is_not_backpressure_is_never_retried() {
    // The ladder above already spends a second call on a schema downgrade.
    // Retrying a plain refusal on top of that would double every failure in
    // the run for nothing.
    let mock = Mock::spawn(vec![(404, r#"{"error":"no such model"}"#.to_string())]);
    let err = ask(&mock.client()).expect_err("must not succeed");
    assert!(
        matches!(err, ModelError::Refused { status: 404, .. }),
        "{err:?}"
    );
    assert_eq!(mock.requests().len(), 1, "exactly one call");
}

#[test]
fn the_tape_records_a_refusal_so_a_replay_stays_in_step() {
    // The founding run of 2026-08-26 lost chunk 1 to a backend 503. Because
    // only successes were taped, replaying it fed chunk 2's reply to chunk 1
    // and every later passage read the reply meant for its predecessor: 256
    // candidates instead of 342, 88 of them dropped for citing a passage
    // they had nothing to do with. Exit 0 throughout.
    let mock = Mock::spawn(vec![
        (
            503,
            r#"{"error":{"message":"MTP process(verify) failed"}}"#.to_string(),
        ),
        (200, completion(r#"{"pairs":["second"]}"#)),
    ]);
    let client = mock.client().recording();
    ask(&client).expect_err("the first call is refused");
    ask(&client).expect("the second answers");

    let tape = client.tape();
    assert_eq!(
        tape.len(),
        2,
        "BOTH calls are on the tape, not just the one that worked"
    );
    assert_eq!(tape[0].status, 503);
    assert_eq!(tape[1].status, 200);

    // And replaying it reproduces the refusal in the same position, so the
    // second call still gets the second reply.
    let replayed = Client::replaying("http://x/v1", "primary", tape);
    let err = ask(&replayed).expect_err("the recorded refusal replays as one");
    assert!(
        matches!(err, ModelError::Refused { status: 503, .. }),
        "{err:?}"
    );
    assert_eq!(
        ask(&replayed).expect("still in step").pairs,
        vec!["second".to_string()]
    );
}
