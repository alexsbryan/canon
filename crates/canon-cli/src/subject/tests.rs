// SPDX-License-Identifier: AGPL-3.0-or-later
//! Subject tests. The load-bearing ones are that a group gets its own call,
//! and that anything the model leaves unsaid refuses a fold rather than
//! allowing one.

use serde_json::json;

use super::*;
use crate::testing::{completion, Mock};

/// One scripted partition: `(rule number, the rule it governs the same thing as)`.
fn answer(rules: &[(usize, usize)]) -> String {
    completion(
        &json!({
            "rules": rules
                .iter()
                .map(|(n, same)| json!({ "n": n, "same_as": same }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

#[test]
fn rules_governing_one_thing_share_a_representative() {
    let mock = Mock::spawn(vec![(200, answer(&[(1, 1), (2, 1)]))]);
    let out = same_thing(&mock.client(), &[vec!["r1", "r2"]]).unwrap();
    assert_eq!(out, vec![vec![0, 0]], "both represent rule 1");
}

#[test]
fn rules_governing_different_things_keep_their_own() {
    // The Des Moines failure: same limit, same wording, different permit.
    let mock = Mock::spawn(vec![(200, answer(&[(1, 1), (2, 2)]))]);
    let out = same_thing(&mock.client(), &[vec!["type B", "type C"]]).unwrap();
    assert_eq!(out, vec![vec![0, 1]]);
}

#[test]
fn each_group_gets_its_own_call() {
    // `same_as` is a position within the call, so two groups in one call lets
    // a member of the first be named by a member of the second.
    let mock = Mock::spawn(vec![
        (200, answer(&[(1, 1), (2, 1)])),
        (200, answer(&[(1, 1), (2, 2)])),
    ]);
    let out = same_thing(&mock.client(), &[vec!["a1", "a2"], vec!["b1", "b2"]]).unwrap();
    assert_eq!(out, vec![vec![0, 0], vec![0, 1]]);
    assert_eq!(mock.requests().len(), 2);
}

#[test]
fn a_rule_the_model_skipped_represents_only_itself() {
    let mock = Mock::spawn(vec![(200, answer(&[(1, 1)]))]);
    let out = same_thing(&mock.client(), &[vec!["r1", "r2"]]).unwrap();
    assert_eq!(out, vec![vec![0, 1]], "silence refuses the fold");
}

#[test]
fn a_representative_past_the_end_is_refused_not_wrapped() {
    let mock = Mock::spawn(vec![(200, answer(&[(1, 1), (2, 9)]))]);
    let out = same_thing(&mock.client(), &[vec!["r1", "r2"]]).unwrap();
    assert_eq!(out, vec![vec![0, 1]]);
}

#[test]
fn a_representative_naming_a_later_rule_is_refused() {
    // `same_as` is the SMALLEST such number, so a forward reference is an
    // answer to a different question than the one asked.
    let mock = Mock::spawn(vec![(200, answer(&[(1, 2), (2, 1)]))]);
    let out = same_thing(&mock.client(), &[vec!["r1", "r2"]]).unwrap();
    assert_eq!(out[0][0], 0, "rule 1 cannot represent rule 2");
}

#[test]
fn a_position_that_was_never_offered_is_dropped() {
    let mock = Mock::spawn(vec![(200, answer(&[(0, 1), (9, 1), (2, 1)]))]);
    let out = same_thing(&mock.client(), &[vec!["r1", "r2"]]).unwrap();
    assert_eq!(out, vec![vec![0, 0]]);
}

#[test]
fn a_group_too_small_to_fold_costs_no_call() {
    let mock = Mock::spawn(Vec::new());
    let out = same_thing(&mock.client(), &[vec!["only"], vec![]]).unwrap();
    assert_eq!(out, vec![vec![0], vec![]]);
    assert!(mock.requests().is_empty());
}

#[test]
fn no_groups_means_no_call_at_all() {
    let mock = Mock::spawn(Vec::new());
    assert!(same_thing(&mock.client(), &[]).unwrap().is_empty());
}
