// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tension-detection tests. The block-pairwise shape is the load-bearing
//! part: it exists because a single comparison over sixty commitments found
//! 1 of 11 planted tensions where blocks of twelve found 5, and the property
//! that makes it correct is that NO PAIR GOES UNEXAMINED.

use serde_json::json;

use super::*;
use crate::testing::{completion, Mock};

fn found(items: &[(usize, usize, &str)]) -> String {
    completion(
        &json!({
            "tensions": items
                .iter()
                .map(|(a, b, r)| json!({ "a": a, "b": b, "reason": r }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

fn texts(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("commitment {i}")).collect()
}

#[test]
fn a_small_list_is_one_comparison() {
    let t = texts(BATCH);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(200, found(&[(1, 2, "clash")]))]);
    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(mock.requests().len(), 1, "no batching below the threshold");
    assert_eq!(
        got,
        vec![Proposed {
            a: 0,
            b: 1,
            reason: "clash".into()
        }]
    );
}

#[test]
fn every_pair_is_examined_by_some_pass() {
    // The correctness property of block-pairwise comparison. Chunking a list
    // and comparing each chunk alone would never look at a cross-chunk pair,
    // and the planted tensions in a charter are mostly cross-section.
    let n = BATCH * 3 + 1;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let blocks = n.div_ceil(BATCH);
    let passes = blocks * (blocks + 1) / 2;
    let mock = Mock::spawn(vec![(200, found(&[])); passes]);
    detect_over(&mock.client(), &refs).unwrap();

    let mut seen = vec![vec![false; n]; n];
    for req in mock.requests() {
        let user = req["messages"][1]["content"].as_str().unwrap().to_string();
        // Recover which commitments this pass was actually shown.
        let offered: Vec<usize> = user
            .lines()
            .filter_map(|l| l.split_once(". "))
            .filter_map(|(_, text)| text.strip_prefix("commitment "))
            .filter_map(|d| d.trim().parse::<usize>().ok())
            .collect();
        for i in &offered {
            for j in &offered {
                seen[*i][*j] = true;
            }
        }
    }
    for (i, row) in seen.iter().enumerate() {
        for (j, examined) in row.iter().enumerate().skip(i + 1) {
            assert!(examined, "pair ({i}, {j}) was never examined");
        }
    }
}

#[test]
fn a_pass_answers_in_its_own_numbering_and_is_mapped_back() {
    // Each pass is shown a renumbered list. Returning position 2 of pass
    // (block1, block2) must not be read as commitment 2 of the whole canon —
    // that would attribute every cross-block tension to the wrong rules.
    let n = BATCH * 2;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let passes = 3;
    let mut script = vec![(200, found(&[])); passes];
    // Passes run (0,0), (0,1), (1,1). The SECOND is the cross-block one; its
    // positions 1 and BATCH+1 are global commitments 0 and BATCH.
    script[1] = (200, found(&[(1, BATCH + 1, "across the blocks")]));
    let mock = Mock::spawn(script);
    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(
        got,
        vec![Proposed {
            a: 0,
            b: BATCH,
            reason: "across the blocks".into()
        }]
    );
}

#[test]
fn the_same_pair_noticed_twice_is_one_tension() {
    let n = BATCH + 1;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let blocks = n.div_ceil(BATCH);
    let passes = blocks * (blocks + 1) / 2;
    // Every pass reports the same first-block pair.
    let mock = Mock::spawn(vec![(200, found(&[(1, 2, "clash")])); passes]);
    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(got.len(), 1, "{got:?}");
}

#[test]
fn a_position_outside_the_pass_is_dropped_not_mapped() {
    let t = texts(4);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(200, found(&[(1, 99, "nowhere"), (3, 3, "itself")]))]);
    assert!(detect_over(&mock.client(), &refs).unwrap().is_empty());
}

// ── the focused pass ────────────────────────────────────────

const LATE: &str = "Quiet hours run from 11:00 PM to 7:00 AM every night.";
const EARLY: &str = "Quiet hours begin at 10:00 PM from Sunday through Thursday.";

fn with_conflict_at_both_ends(n: usize) -> Vec<String> {
    let mut t = texts(n);
    t[0] = LATE.to_string();
    t[n - 1] = EARLY.to_string();
    t
}

#[test]
fn a_pair_the_blocks_missed_gets_one_comparison_of_its_own() {
    // The two halves sit in different blocks, so block-pairwise DOES compare
    // them — buried among twenty-three unrelated rules, which is the burial
    // the batch-size measurement was about. Here every block pass notices
    // nothing and the focused pass, holding only the flagged pair, does.
    let n = BATCH + 2;
    let t = with_conflict_at_both_ends(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let blocks = n.div_ceil(BATCH);
    let passes = blocks * (blocks + 1) / 2;

    let mut responses = vec![(200, found(&[])); passes];
    responses.push((200, found(&[(1, 2, "the hour moved")])));
    let mock = Mock::spawn(responses);

    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(
        mock.requests().len(),
        passes + 1,
        "the flagged pair is worth exactly one more call"
    );
    let focused = mock.requests()[passes]["messages"][1]["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(focused.contains("11:00 PM"), "{focused}");
    assert!(focused.contains("10:00 PM"), "{focused}");
    assert_eq!(
        got,
        vec![Proposed {
            a: 0,
            b: n - 1,
            reason: "the hour moved".into()
        }]
    );
}

#[test]
fn a_pair_the_blocks_already_found_is_not_reported_twice() {
    let n = BATCH + 2;
    let t = with_conflict_at_both_ends(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let blocks = n.div_ceil(BATCH);
    let passes = blocks * (blocks + 1) / 2;

    // The pass holding both blocks reports the pair; the focused pass reports
    // it again in its own numbering.
    let mut responses = vec![(200, found(&[])); passes];
    responses[1] = (200, found(&[(1, BATCH + 2, "the hour moved")]));
    responses.push((200, found(&[(1, 2, "the hour moved, again")])));
    let mock = Mock::spawn(responses);

    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(got.len(), 1, "one pair is one tension however often seen");
    assert_eq!(got[0].reason, "the hour moved", "first reason wins");
}

#[test]
fn rules_that_state_no_measures_buy_no_extra_calls() {
    // The pass is free when there is nothing mechanical to notice, which is
    // what keeps it from taxing every canon for one document's shape.
    let n = BATCH + 2;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let blocks = n.div_ceil(BATCH);
    let passes = blocks * (blocks + 1) / 2;
    let mock = Mock::spawn(vec![(200, found(&[])); passes]);
    assert!(detect_over(&mock.client(), &refs).unwrap().is_empty());
    assert_eq!(mock.requests().len(), passes);
}
