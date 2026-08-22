// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantify tests. The load-bearing one is that a unit nobody wrote down
//! still compares correctly — that is the whole reason this replaces a list.

use serde_json::json;

use super::*;
use crate::testing::{completion, Mock};

fn q(value: &str, unit: &str, of: &str, canonical: &str) -> Quantity {
    Quantity {
        value: value.into(),
        unit: unit.into(),
        of: of.into(),
        canonical: canonical.into(),
    }
}

/// One scripted reading: `(rule number, [(value, unit, of, canonical)])`.
type Reading<'a> = (usize, &'a [(&'a str, &'a str, &'a str, &'a str)]);

fn answer(rules: &[Reading]) -> String {
    completion(
        &json!({
            "rules": rules.iter().map(|(n, qs)| json!({
                "n": n,
                "quantities": qs.iter()
                    .map(|(v, u, o, c)| json!({ "value": v, "unit": u, "of": o, "canonical": c }))
                    .collect::<Vec<_>>(),
            })).collect::<Vec<_>>()
        })
        .to_string(),
    )
}

#[test]
fn a_unit_nobody_listed_still_separates_two_limits() {
    // The failure this module exists for. `measure.rs` knew minutes, hours,
    // days, gallons and dollars, so 85 dBA and 85 dBC both read as stating
    // no measure at all and five planted supersessions were folded away.
    // Nothing here knows what a decibel is.
    assert!(differs_by_quantity(
        &[q("85", "dBA", "sound level", "85 dBA")],
        &[q("85", "dBC", "sound level", "85 dBC")]
    ));
    // And the next document's units, which no list would have had either.
    assert!(differs_by_quantity(
        &[q("30", "lux", "corridor lighting", "30 lux")],
        &[q("50", "lux", "corridor lighting", "50 lux")]
    ));
    assert!(differs_by_quantity(
        &[q("2", "acre-feet", "annual draw", "2 acre-foot")],
        &[q("3", "acre-feet", "annual draw", "3 acre-foot")]
    ));
}

#[test]
fn the_same_limit_worded_differently_is_not_a_difference() {
    assert!(!differs_by_quantity(
        &[q("85", "dBC", "sound level", "85 dBC")],
        &[q("85", "DBC", "Sound Level", "85 dBC")]
    ));
}

#[test]
fn two_numbers_about_different_things_do_not_disagree() {
    // "no more than 2 guests" against "no more than 2 nights" is the reason
    // a quantity carries what it measures. No stopword list involved.
    assert!(!differs_by_quantity(
        &[q("2", "", "guests per room", "2")],
        &[q("2", "nights", "length of stay", "2 night")]
    ));
}

#[test]
fn a_rule_stating_no_quantity_can_still_be_a_duplicate() {
    assert!(!differs_by_quantity(
        &[],
        &[q("85", "dBA", "sound level", "85 dBA")]
    ));
    assert!(!differs_by_quantity(
        &[q("85", "dBA", "sound level", "85 dBA")],
        &[]
    ));
}

#[test]
fn independent_rules_are_read_in_batches() {
    let rules: Vec<String> = (0..BATCH + 3).map(|i| format!("rule {i}")).collect();
    let refs: Vec<&str> = rules.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![
        (
            200,
            answer(&[(1, &[("85", "dBA", "sound level", "85 dBA")])]),
        ),
        (
            200,
            answer(&[(3, &[("10:00 PM", "", "quiet hours start", "22:00")])]),
        ),
    ]);
    let got = quantify(&mock.client(), &refs).unwrap();
    assert_eq!(mock.requests().len(), 2, "one pass per block of {BATCH}");
    assert_eq!(got.len(), refs.len());
    assert_eq!(got[0], vec![q("85", "dBA", "sound level", "85 dBA")]);
    assert_eq!(
        got[BATCH + 2],
        vec![q("10:00 PM", "", "quiet hours start", "22:00")]
    );
    // Everything unanswered is empty, never another rule's answer.
    assert!(got[1].is_empty());
}

#[test]
fn an_answer_naming_a_rule_that_was_not_offered_is_dropped_not_misfiled() {
    let mock = Mock::spawn(vec![(
        200,
        answer(&[(99, &[("85", "dBA", "sound level", "85 dBA")])]),
    )]);
    let got = quantify(&mock.client(), &["one rule"]).unwrap();
    assert_eq!(got, vec![Vec::<Quantity>::new()]);
}

#[test]
fn the_reading_prompt_forbids_normalising_the_units_away() {
    let mock = Mock::spawn(vec![(200, answer(&[(1, &[])]))]);
    quantify(&mock.client(), &["a rule"]).unwrap();
    let sent = mock.requests();
    let system = sent[0]["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("as written"), "{system}");
    assert!(system.contains("part of the unit"), "{system}");
}
