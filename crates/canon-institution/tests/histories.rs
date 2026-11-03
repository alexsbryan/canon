//! Product-owned acceptance cases: no xenia process, browser, Python or model.
use canon_core::{ActId, Grant, Log};
use serde_json::{json, Value};

fn observe(state: &Value, h: &Value, at: i64, wanted: &Value) -> Value {
    let rule = |id: &Value| {
        state["commitments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == *id)
    };
    let status = |id: &Value| rule(id).map_or(json!("absent"), |c| c["status"]["status"].clone());
    let mut found = json!({});
    for key in wanted.as_object().unwrap().keys() {
        found[key] = match key.as_str() {
            "status" => status(&h["target"]),
            "original_status" => status(&h["original"]),
            "scope" => rule(&h["target"]).map_or(Value::Null, |c| c["scope"].clone()),
            "sam_holds" => json!(state["grants"].as_array().unwrap().iter().any(|g| {
                let grant: Grant = serde_json::from_value(g.clone()).unwrap();
                grant.actor == "human:Sam"
                    && grant.scope.as_str() == "courtyard"
                    && grant.held_at(at)
            })),
            other => panic!("unassessed fixture expectation: {other}"),
        };
    }
    found
}

#[test]
fn ten_original_histories_preserve_native_results_and_satisfy_the_optional_interpretation() {
    let cases: Value = serde_json::from_str(include_str!("fixtures/cases.json")).unwrap();
    let histories: Vec<Value> =
        serde_json::from_str(include_str!("fixtures/histories.json")).unwrap();
    assert_eq!(histories.len(), 10);
    let mut unchanged_controls = 0;
    for h in histories {
        let label = format!("{}/{}", h["experiment"], h["case"]);
        let basis = h["basis"].as_str().unwrap();
        let source = format!("{}{}", basis, h["appended"].as_str().unwrap());
        assert_eq!(
            canon_core::id::digest_hex(basis.as_bytes()),
            h["before_sha256"],
            "{label}"
        );
        assert_eq!(
            canon_core::id::digest_hex(source.as_bytes()),
            h["after_sha256"],
            "{label}"
        );
        let log = Log::parse(&source).unwrap();
        let at = log.acts().iter().map(|a| a.ts_unix).max().unwrap();
        let expected = &cases[h["experiment"].as_str().unwrap()]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == h["case"])
            .unwrap()["expect"];
        let native = log.derive_at(at);
        let mut native_state = serde_json::to_value(&native).unwrap();
        for c in native_state["commitments"].as_array_mut().unwrap() {
            c["scope"] =
                serde_json::to_value(native.scope_of(&ActId::from_raw(c["id"].as_str().unwrap())))
                    .unwrap();
        }
        assert_eq!(
            observe(&native_state, &h, at, expected),
            h["native_observed"],
            "native {label}"
        );
        let interpreted = canon_institution::project(basis, &source, at).unwrap();
        assert_eq!(
            observe(&interpreted["state"], &h, at, expected),
            *expected,
            "institution {label}"
        );
        unchanged_controls += usize::from(h["native_observed"] == *expected);
    }
    assert_eq!(unchanged_controls, 6);
}
