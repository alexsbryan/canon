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
        embed_model: None,
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
            embed_model: None,
        })
        .unwrap();
        assert!(c.is_local(), "{e} should be local (host {})", c.host());
        assert!(c.describe().contains("local"));
    }
    for e in remote {
        let c = Client::new(&Config {
            endpoint: Some(e.into()),
            model: None,
            embed_model: None,
        })
        .unwrap();
        assert!(!c.is_local(), "{e} should be remote (host {})", c.host());
        let err = c.require_local().expect_err("must refuse");
        // Exit 2, not 3: the tool could judge — the user has to say the text
        // may leave the machine.
        assert_eq!(err.exit_code(), 2);
    }
}
