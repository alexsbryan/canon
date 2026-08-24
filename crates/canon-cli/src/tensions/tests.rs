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
    let got = detect_over(&mock.client(), &refs).unwrap().pairs;
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
    let got = detect_over(&mock.client(), &refs).unwrap().pairs;
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
    let got = detect_over(&mock.client(), &refs).unwrap().pairs;
    assert_eq!(got.len(), 1, "{got:?}");
}

#[test]
fn a_position_outside_the_pass_is_dropped_not_mapped() {
    let t = texts(4);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(200, found(&[(1, 99, "nowhere"), (3, 3, "itself")]))]);
    assert!(detect_over(&mock.client(), &refs).unwrap().pairs.is_empty());
}

// ── one refusal must not cost every other comparison ────────

/// What the daemon answered on comparison 20 of 36 of a Des Moines sweep:
/// a 300-second inference deadline inside a schema-constrained decode. It
/// took the whole run with it, twenty passes in and thirty-three minutes
/// deep, and the two runs behind it never started.
const DEADLINE_503: &str = r#"{"error":{"message":"local inference failed: inference deadline exceeded after 300s (2964 tokens generated) — likely pathological JSON-Schema mask state","type":"backend_error"}}"#;

#[test]
fn a_pass_that_produces_no_answer_costs_coverage_not_the_run() {
    // Two full blocks, so all three passes really compare something.
    let t = texts(2 * BATCH);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();

    let mock = Mock::spawn(vec![
        (200, found(&[(1, 2, "first block")])),
        (503, DEADLINE_503.to_string()),
        // Answers are in the PASS's own numbering; this is the second block.
        (200, found(&[(1, 2, "second block")])),
    ]);
    let got = detect_over(&mock.client(), &refs).unwrap();

    // Both surviving passes' findings are kept, mapped back to global
    // positions — the run loses the middle pass, not the outer two.
    assert_eq!(got.pairs.len(), 2, "{:?}", got.pairs);
    assert_eq!((got.pairs[0].a, got.pairs[0].b), (0, 1));
    assert_eq!((got.pairs[1].a, got.pairs[1].b), (BATCH, BATCH + 1));
    // And the run says how much of the pair space it actually weighed.
    assert_eq!(got.passes, 3);
    assert_eq!(got.unread.len(), 1);
    assert!(got.unread[0].starts_with("pass 2/3 "), "{:?}", got.unread);
    // The failing input is recoverable from the artifact, not just its
    // ordinal — a cross pass over both blocks, so every position appears.
    assert!(
        got.unread[0].contains(&format!("{:?}", (0..2 * BATCH).collect::<Vec<_>>())),
        "the pass must record which commitments it held: {:?}",
        got.unread
    );
}

#[test]
fn every_pass_failing_is_an_error_not_an_empty_answer() {
    // Zero tensions because zero comparisons ran is not "no tensions found",
    // and a caller that cannot tell those apart will publish the wrong one
    // (§18.3). The error returned is the endpoint's own, not a summary.
    let t = texts(2 * BATCH);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(503, DEADLINE_503.to_string()); 3]);
    let err = detect_over(&mock.client(), &refs).unwrap_err();
    assert!(
        err.to_string().contains("deadline exceeded"),
        "the endpoint's own words, not ours: {err}"
    );
}

// ── ordering: it moves pairs between passes, never drops one ─────────────

/// Every unordered pair of positions, in the sets `passes_over` returns.
fn covered(order: &[usize]) -> Vec<(usize, usize)> {
    let mut seen: Vec<(usize, usize)> = Vec::new();
    for set in passes_over(order) {
        for (i, a) in set.iter().enumerate() {
            for b in &set[i + 1..] {
                seen.push((*a.min(b), *a.max(b)));
            }
        }
    }
    seen.sort_unstable();
    seen
}

fn all_pairs(n: usize) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            out.push((a, b));
        }
    }
    out
}

#[test]
fn every_pair_is_compared_at_least_once_whatever_the_order() {
    // The invariant the whole ordering change rests on. If reordering could
    // drop a pair it would trade recall for tidiness, silently.
    //
    // At LEAST once, not exactly once: a cross pass over blocks x and y also
    // re-examines every within-x and within-y pair, so those get several
    // looks. `detect_over` folds the repeats. Asserting "exactly" here was my
    // own misreading of the doc comment, and this test is what caught it.
    for n in [13, 24, 25, 47, 83] {
        let listed: Vec<usize> = (0..n).collect();
        let reversed: Vec<usize> = (0..n).rev().collect();
        let rotated: Vec<usize> = (0..n).map(|i| (i * 7 + 5) % n).collect();
        for order in [&listed, &reversed, &rotated] {
            let mut seen = covered(order);
            seen.dedup();
            assert_eq!(seen, all_pairs(n), "n={n}: coverage changed with order");
        }
    }
}

#[test]
fn a_within_block_pair_gets_more_looks_than_one_that_straddles_blocks() {
    // Why ordering pays: which pairs land inside a block decides which ones
    // are read against 66 competitors several times over, and which are read
    // against 276 once.
    let order: Vec<usize> = (0..24).collect();
    let looks = |a: usize, b: usize| {
        passes_over(&order)
            .iter()
            .filter(|set| set.contains(&a) && set.contains(&b))
            .count()
    };
    assert_eq!(
        looks(0, 1),
        2,
        "same block: its self pass and one cross pass"
    );
    assert_eq!(looks(0, 12), 1, "straddling blocks: the cross pass only");
}

#[test]
fn a_pair_of_twins_moves_out_of_a_cross_pass_into_a_self_pass() {
    // The Des Moines failure in miniature: two near-identical rules sitting
    // 12 apart in the document land in different blocks and are weighed
    // against 276 competitors; ordering puts them in one block, against 66.
    let listed: Vec<usize> = (0..24).collect();
    let together: Vec<usize> = std::iter::once(0)
        .chain(std::iter::once(12))
        .chain((1..24).filter(|i| *i != 12))
        .collect();
    let block_of = |order: &[usize], v: usize| order.iter().position(|x| *x == v).unwrap() / BATCH;
    assert_ne!(
        block_of(&listed, 0),
        block_of(&listed, 12),
        "document order splits them"
    );
    assert_eq!(
        block_of(&together, 0),
        block_of(&together, 12),
        "ordering joins them"
    );
    // And the pair is still compared exactly once either way.
    assert!(covered(&listed).contains(&(0, 12)));
    assert!(covered(&together).contains(&(0, 12)));
}

/// Two-dimensional unit vectors at the given angles, as an embeddings reply.
fn embeddings(degrees: &[f64]) -> String {
    let data: Vec<Value> = degrees
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let r = d.to_radians();
            json!({ "index": i, "embedding": [r.cos(), r.sin()] })
        })
        .collect();
    json!({ "data": data }).to_string()
}

#[test]
fn ordering_chains_each_commitment_to_its_nearest_unplaced_neighbour() {
    let mock = Mock::spawn(vec![(200, embeddings(&[0.0, 90.0, 10.0, 80.0]))]);
    let texts = ["a", "b", "c", "d"];
    let order = similarity_order(&mock.embedding_client(), &texts);
    // From 0°: 10° is nearest, then 80°, then 90°.
    assert_eq!(order, vec![0, 2, 3, 1]);
}

#[test]
fn equal_similarity_goes_to_the_lower_position() {
    // Two runs over one document must block identically, or the noise floor
    // stops being a property of the model.
    let mock = Mock::spawn(vec![(200, embeddings(&[0.0, 45.0, 45.0]))]);
    let order = similarity_order(&mock.embedding_client(), &["a", "b", "c"]);
    assert_eq!(order, vec![0, 1, 2]);
}

#[test]
fn no_embedding_model_compares_in_document_order() {
    // Ordering is an improvement, not a dependency: without it the tool does
    // what it always did, and says so.
    let mock = Mock::spawn(Vec::new());
    let order = similarity_order(&mock.client(), &["a", "b", "c"]);
    assert_eq!(order, vec![0, 1, 2]);
    assert!(mock.requests().is_empty(), "and costs no call");
}

#[test]
fn a_short_embeddings_reply_falls_back_rather_than_mispairing() {
    // Lining up 2 vectors against 3 texts would silently order by the wrong
    // vector — worse than not ordering at all.
    let mock = Mock::spawn(vec![(200, embeddings(&[0.0, 90.0]))]);
    let order = similarity_order(&mock.embedding_client(), &["a", "b", "c"]);
    assert_eq!(order, vec![0, 1, 2]);
}

#[test]
fn a_refused_embedding_falls_back_rather_than_failing_the_run() {
    let mock = Mock::spawn(vec![(500, "{\"error\":\"no such model\"}".to_string())]);
    let order = similarity_order(&mock.embedding_client(), &["a", "b", "c"]);
    assert_eq!(order, vec![0, 1, 2]);
}

#[test]
fn embeddings_are_lined_up_by_the_index_the_server_states() {
    // A server free to answer out of order must not silently mis-pair every
    // vector with the wrong commitment.
    let scrambled = json!({ "data": [
        { "index": 2, "embedding": [0.0, 1.0] },
        { "index": 0, "embedding": [1.0, 0.0] },
        { "index": 1, "embedding": [0.9848, 0.1736] },
    ]})
    .to_string();
    let mock = Mock::spawn(vec![(200, scrambled)]);
    let order = similarity_order(&mock.embedding_client(), &["a", "b", "c"]);
    // 0° then 10° then 90°, which is only true if `index` was honoured.
    assert_eq!(order, vec![0, 1, 2]);
}
