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

/// The scripted length a mock needs to serve one whole comparison.
fn passes_for(n: usize) -> usize {
    schedule(n, LOOKS).len()
}

/// Where `g` sits in `set`, 1-based, as a pass numbers its own list.
fn local(set: &[usize], g: usize) -> usize {
    set.iter().position(|x| *x == g).expect("in this pass") + 1
}

#[test]
fn a_small_list_is_one_comparison() {
    let t = texts(BATCH);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(200, found(&[(1, 2, "clash")]))]);
    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(mock.requests().len(), 1, "no batching below the threshold");
    assert_eq!(
        got.pairs,
        vec![Proposed {
            a: 0,
            b: 1,
            reason: "clash".into()
        }]
    );
    // One pass already holds every pair. Asking for a second look at the same
    // crowd asks the same question again, so the schedule says one.
    assert_eq!(got.schedule.looks, 1);
    assert_eq!(got.schedule.passes, 1);
}

#[test]
fn every_pair_is_examined_by_some_pass() {
    // The correctness property of the whole step. Chunking a list and
    // comparing each chunk alone would never look at a cross-chunk pair, and
    // the planted tensions in a charter are mostly cross-section.
    let n = BATCH * 3 + 1;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(200, found(&[])); passes_for(n)]);
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
    // Each pass is shown a renumbered list. Returning position 2 of some pass
    // must not be read as commitment 2 of the whole canon — that would
    // attribute a tension to the wrong rules, and under a covering schedule
    // no pass begins at the top of the list.
    let n = BATCH * 2;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let sets = schedule(n, LOOKS);
    let mut script = vec![(200, found(&[])); sets.len()];
    script[0] = (200, found(&[(1, 2, "in the first pass")]));
    let mock = Mock::spawn(script);
    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(
        got.pairs,
        vec![Proposed {
            a: sets[0][0],
            b: sets[0][1],
            reason: "in the first pass".into()
        }]
    );
}

#[test]
fn the_same_pair_noticed_in_two_passes_is_one_tension() {
    // A pair is weighed LOOKS times by construction, so the fold is not an
    // edge case here — it is the ordinary path.
    let n = BATCH * 2;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let sets = schedule(n, LOOKS);
    let (a, b) = (sets[0][0], sets[0][1]);
    let both: Vec<usize> = (0..sets.len())
        .filter(|i| sets[*i].contains(&a) && sets[*i].contains(&b))
        .collect();
    assert!(
        both.len() >= 2,
        "the schedule owes this pair {LOOKS} looks: {both:?}"
    );
    let mut script = vec![(200, found(&[])); sets.len()];
    for i in &both {
        let set = &sets[*i];
        script[*i] = (
            200,
            found(&[(local(set, a), local(set, b), "the same clash")]),
        );
    }
    let mock = Mock::spawn(script);
    let got = detect_over(&mock.client(), &refs).unwrap();
    assert_eq!(got.pairs.len(), 1, "{:?}", got.pairs);
    assert_eq!((got.pairs[0].a, got.pairs[0].b), (a, b));
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
    let n = BATCH * 2;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let sets = schedule(n, LOOKS);
    let mut script = vec![(200, found(&[])); sets.len()];
    script[0] = (200, found(&[(1, 2, "the first pass")]));
    script[1] = (503, DEADLINE_503.to_string());
    let mock = Mock::spawn(script);
    let got = detect_over(&mock.client(), &refs).unwrap();

    // The surviving pass's finding is kept and mapped back — the run loses
    // one pass, not itself.
    assert_eq!(got.pairs.len(), 1, "{:?}", got.pairs);
    assert_eq!((got.pairs[0].a, got.pairs[0].b), (sets[0][0], sets[0][1]));
    // And the run says how much of the pair space it actually weighed.
    assert_eq!(got.passes, sets.len());
    assert_eq!(got.unread.len(), 1);
    assert!(got.unread[0].starts_with("pass 2/"), "{:?}", got.unread);
    // The failing input is recoverable from the artifact, not just its
    // ordinal — the positions it held are named.
    assert!(
        got.unread[0].contains(&format!("{:?}", sets[1])),
        "the pass must record which commitments it held: {:?}",
        got.unread
    );
}

#[test]
fn every_pass_failing_is_an_error_not_an_empty_answer() {
    // Zero tensions because zero comparisons ran is not "no tensions found",
    // and a caller that cannot tell those apart will publish the wrong one
    // (§18.3). The error returned is the endpoint's own, not a summary.
    let n = BATCH * 2;
    let t = texts(n);
    let refs: Vec<&str> = t.iter().map(String::as_str).collect();
    let mock = Mock::spawn(vec![(503, DEADLINE_503.to_string()); passes_for(n)]);
    let err = detect_over(&mock.client(), &refs).unwrap_err();
    assert!(
        err.to_string().contains("deadline exceeded"),
        "the endpoint's own words, not ours: {err}"
    );
}

// ── the schedule: a covering design, and what it guarantees ─────────────

fn all_pairs(n: usize) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for a in 0..n {
        for b in a + 1..n {
            out.push((a, b));
        }
    }
    out
}

/// How many times each unordered pair appears across the returned sets.
fn looks(sets: &[Vec<usize>], n: usize) -> std::collections::BTreeMap<(usize, usize), usize> {
    let mut seen = std::collections::BTreeMap::new();
    for p in all_pairs(n) {
        seen.insert(p, 0);
    }
    for set in sets {
        for (i, a) in set.iter().enumerate() {
            for b in &set[i + 1..] {
                *seen.entry((*a.min(b), *a.max(b))).or_insert(0) += 1;
            }
        }
    }
    seen
}

#[test]
fn every_pair_is_weighed_at_least_the_number_of_times_asked_for() {
    // The correctness property, and it is strictly stronger than the one the
    // block-pairwise scheme made. That one promised each pair AT LEAST ONE
    // pass; this promises each pair at least `looks` passes, which is what
    // makes the redundancy a policy rather than an accident of where a rule
    // sat in the document.
    for n in [25, 47, 83, 120] {
        for r in [1, 2, 3] {
            let sets = schedule(n, r);
            let seen = looks(&sets, n);
            let worst = seen.values().min().copied().unwrap_or(0);
            assert!(
                worst >= r,
                "n={n} looks={r}: a pair was weighed {worst} time(s)"
            );
        }
    }
}

#[test]
fn no_pair_is_left_on_a_single_look() {
    // What this replaced, measured on a 289-commitment canon: 96.2% of pairs
    // got ONE look and 3.8% got TWENTY-FIVE, and which side a pair landed on
    // was decided by whether the two rules happened to be adjacent in the
    // document. The FLOOR is the guarantee — nobody is left on one look —
    // and it is the half that was missing.
    //
    // The ceiling is loose and that is the greedy's doing, not the contract's:
    // a block of BATCH covers C(BATCH,2) pairs and the last few blocks
    // re-cover many of them to reach the stragglers. Measured at looks=2:
    // n=47 gives 2..6, n=83 gives 2..12, n=120 gives 2..20, mean about three
    // throughout. A better construction tightens the ceiling and changes
    // nothing above here.
    for n in [47, 83, 120] {
        let seen = looks(&schedule(n, 2), n);
        let worst = *seen.values().min().unwrap();
        let mean = seen.values().sum::<usize>() as f64 / seen.len() as f64;
        assert_eq!(worst, 2, "n={n}: a pair was weighed {worst} time(s)");
        assert!(
            mean < 4.0,
            "n={n}: mean {mean:.2} looks — the greedy is drifting"
        );
    }
}

#[test]
fn a_block_holds_between_two_and_batch_commitments() {
    // A pass over fewer than two compares nothing and would pad the coverage
    // a run reports; a pass over more than BATCH is past the size the recall
    // measurement covers.
    for n in [25, 60, 120] {
        for set in schedule(n, 2) {
            assert!(
                (2..=BATCH).contains(&set.len()),
                "n={n}: block of {}",
                set.len()
            );
            let mut sorted = set.clone();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), set.len(), "a block repeated a commitment");
        }
    }
}

#[test]
fn two_runs_over_one_canon_schedule_identically() {
    // Or the noise floor stops being a property of the model and starts
    // being a property of the schedule.
    for n in [25, 47, 83] {
        assert_eq!(schedule(n, 2), schedule(n, 2), "n={n}");
    }
}

#[test]
fn asking_for_more_looks_costs_more_passes_and_never_fewer() {
    let n = 60;
    let one = schedule(n, 1).len();
    let two = schedule(n, 2).len();
    let three = schedule(n, 3).len();
    assert!(one < two && two < three, "{one} {two} {three}");
}

#[test]
fn a_canon_too_small_to_split_needs_no_schedule() {
    assert!(schedule(1, 2).is_empty());
    assert!(schedule(0, 2).is_empty());
    // And asking for no looks is asking for no comparison.
    assert!(schedule(50, 0).is_empty());
}
