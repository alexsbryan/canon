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
fn independent_pairs_are_read_in_batches() {
    // Pairs are independent of one another, so they batch; a rule and its own
    // citation are not, so they never split (see the pairing test below).
    let texts: Vec<(String, String)> = (0..(BATCH / 2) + 2)
        .map(|i| (format!("rule {i}"), format!("citation {i}")))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
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
    let got = quantify_pairs(&mock.client(), &refs).unwrap();
    assert_eq!(mock.requests().len(), 2, "one call per {} pairs", BATCH / 2);
    assert_eq!(got.len(), refs.len());
    assert_eq!(got[0].0, vec![q("85", "dBA", "sound level", "85 dBA")]);
    // Position 3 of the SECOND call is that block's second rule.
    assert_eq!(
        got[BATCH / 2 + 1].0,
        vec![q("10:00 PM", "", "quiet hours start", "22:00")]
    );
    // Everything unanswered is empty, never another rule's answer.
    assert!(got[0].1.is_empty());
}

#[test]
fn an_answer_naming_a_rule_that_was_not_offered_is_dropped_not_misfiled() {
    let mock = Mock::spawn(vec![(
        200,
        answer(&[(99, &[("85", "dBA", "sound level", "85 dBA")])]),
    )]);
    let got = read_block(&mock.client(), &["one rule"]).unwrap();
    assert_eq!(got, vec![Vec::<Quantity>::new()]);
}

#[test]
fn the_reading_prompt_forbids_normalising_the_units_away() {
    let mock = Mock::spawn(vec![(200, answer(&[(1, &[])]))]);
    read_block(&mock.client(), &["a rule"]).unwrap();
    let sent = mock.requests();
    let system = sent[0]["messages"][0]["content"].as_str().unwrap();
    assert!(system.contains("as written"), "{system}");
    assert!(system.contains("part of the unit"), "{system}");
}

// ── does a citation carry the rule's numbers? ───────────────

#[test]
fn a_number_the_citation_never_states_is_unsupported() {
    let rule = [q("three", "hours", "notice", "3 hour")];
    let cited = [q("three", "days", "notice", "3 day")];
    assert_eq!(unsupported(&rule, &cited).as_deref(), Some("three hours"));
}

#[test]
fn a_citation_may_state_more_than_the_rule_it_supports() {
    // A cited span runs to a sentence boundary, so it routinely carries a
    // clause the rule did not restate. The rule's numbers must be in the
    // citation; the citation's need not be in the rule.
    let rule = [q("11", "PM", "quiet hours start", "23:00")];
    let cited = [
        q("11", "PM", "quiet hours start", "23:00"),
        q("7", "AM", "quiet hours end", "07:00"),
        q("2", "guests", "occupancy", "2 guest"),
    ];
    assert_eq!(unsupported(&rule, &cited), None);
}

#[test]
fn a_rule_stating_no_number_is_supported_by_any_citation() {
    assert_eq!(
        unsupported(&[], &[q("85", "dBA", "sound level", "85 dBA")]),
        None
    );
    assert_eq!(unsupported(&[], &[]), None);
}

#[test]
fn a_citation_stating_no_number_cannot_support_one_that_does() {
    let rule = [q("85", "dBC", "sound level", "85 dBC")];
    assert_eq!(unsupported(&rule, &[]).as_deref(), Some("85 dBC"));
}

#[test]
fn what_a_number_counts_is_not_compared() {
    // "within any seven-day period" reworded as "per week" is a paraphrase,
    // and the two readings will not agree about `of`. Only the measure is
    // checked, so the paraphrase survives and a wrong NUMBER still does not.
    let rule = [q("2", "nights", "length of stay", "2 night")];
    let cited = [q("2", "nights", "consecutive nights per week", "2 night")];
    assert_eq!(unsupported(&rule, &cited), None);
}

#[test]
fn an_answer_with_no_number_in_it_is_not_a_quantity() {
    // Observed on a live smoke run: the reading pass returned an entry with
    // an empty value and unit, and the citation guard refused the rule for
    // stating `` that its citation did not. Two of three candidates died
    // that way — both readings of the Type "F" permit, which is a planted
    // pair. An empty answer states nothing and must not compare as a number.
    let mock = Mock::spawn(vec![(
        200,
        answer(&[(
            1,
            &[
                ("", "", "sound level", ""),
                ("85", "dBA", "sound level", "85 dBA"),
            ],
        )]),
    )]);
    let got = read_block(&mock.client(), &["a rule"]).unwrap();
    assert_eq!(got[0].len(), 1, "the empty entry is dropped: {:?}", got[0]);
    assert_eq!(got[0][0].value, "85");
    // And nothing downstream can be refused for stating it.
    assert_eq!(unsupported(&got[0], &got[0]), None);
}

#[test]
fn a_pair_is_never_split_across_two_calls() {
    // The comparison downstream is between a rule and its own citation, and
    // canonical form is only agreed WITHIN a call — so a pair that landed in
    // two calls would be compared against a normalisation nobody promised.
    // Enough pairs to force several calls, and every one must arrive whole.
    let pairs: Vec<(String, String)> = (0..(BATCH * 3))
        .map(|i| (format!("rule {i}"), format!("citation {i}")))
        .collect();
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    let calls = refs.len().div_ceil(BATCH / 2);
    let mock = Mock::spawn(vec![(200, answer(&[])); calls]);
    let got = quantify_pairs(&mock.client(), &refs).unwrap();
    assert_eq!(got.len(), refs.len(), "every pair comes back");

    for body in mock.requests() {
        let user = body["messages"][1]["content"].as_str().unwrap();
        // Only the numbered lines; the closing instruction says "rule" too.
        let listed: Vec<&str> = user
            .lines()
            .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()))
            .collect();
        let rules = listed.iter().filter(|l| l.contains("rule ")).count();
        let cites = listed.iter().filter(|l| l.contains("citation ")).count();
        assert_eq!(
            rules, cites,
            "a call holds whole pairs only ({rules} rules, {cites} citations):\n{user}"
        );
        // And each citation sits immediately after the rule it belongs to.
        for w in listed.chunks(2) {
            assert!(
                w[0].contains("rule ") && w[1].contains("citation "),
                "{listed:?}"
            );
        }
    }
}
