// SPDX-License-Identifier: AGPL-3.0-or-later
//! What has gone past its date: one query for a term limit, a trial period
//! and a revisit date, and the clock it is handed.

use canon_core::{Act, ActKind, Log, Scope};
use canon_readers::{Due, Reviews};

#[test]
fn one_query_answers_a_term_limit_a_trial_period_and_a_revisit_date() {
    // The generalization claim, asserted as one call returning three kinds.
    // If these needed three queries they would be three mechanisms, and
    // `PRIMITIVES.md` would be wrong about Primitive 8.
    //
    // Real epoch seconds throughout, because one of the three dates lives in
    // the format as a STRING (`accept.revisit`) and the point is that both
    // spellings compare through one calendar.
    let jan = canon_core::date::parse_ymd("2026-01-01").unwrap();
    let feb = canon_core::date::parse_ymd("2026-02-01").unwrap();
    let jun = canon_core::date::parse_ymd("2026-06-01").unwrap();
    let a = Act::new(
        ActKind::Assert {
            text: "Bikes live in the hall.".into(),
            from: None,
            source: None,
        },
        jan,
        "human:alex",
    );
    let b = Act::new(
        ActKind::Assert {
            text: "The hall stays clear.".into(),
            from: None,
            source: None,
        },
        jan + 1,
        "human:alex",
    );
    let acts = vec![
        a.clone(),
        b.clone(),
        // a trial period on a commitment
        Act::new(
            ActKind::Horizon {
                target: a.id.clone(),
                at: jun,
                rationale: "trial for one winter".into(),
            },
            jan + 2,
            "human:alex",
        ),
        // a contradiction carried with a revisit date, written as a string
        Act::new(
            ActKind::Accept {
                a: a.id.clone(),
                b: b.id.clone(),
                rationale: "the hall is wide enough for now".into(),
                revisit: Some("2026-03-01".into()),
            },
            jan + 3,
            "human:alex",
        ),
        // a term limit
        Act::new(
            ActKind::Grant {
                holder: "human:dana".into(),
                scope: Scope::new("house.kitchen").unwrap(),
                horizon: Some(feb),
                rationale: "one term".into(),
            },
            jan + 4,
            "human:alex",
        ),
    ];
    let canon = Log::from_acts(acts).derive();

    assert!(canon.overdue(jan + 5).is_empty(), "nothing is due yet");

    let due = canon.overdue(jun + 86_400);
    assert_eq!(due.len(), 3, "{due:#?}");
    // Oldest first.
    assert!(due[0].due <= due[1].due && due[1].due <= due[2].due);
    assert!(matches!(due[0].what, Due::Standing { .. }), "february");
    assert!(matches!(due[1].what, Due::Revisit { .. }), "march");
    assert!(matches!(due[2].what, Due::Horizon { .. }), "june");
}

#[test]
fn a_horizon_on_something_already_dealt_with_stops_being_overdue() {
    // Re-surfacing what was settled is how a closure query teaches people to
    // ignore it, which is worse than not having one.
    let a = Act::new(
        ActKind::Assert {
            text: "Trial: no shoes indoors.".into(),
            from: None,
            source: None,
        },
        100,
        "human:alex",
    );
    let horizon = Act::new(
        ActKind::Horizon {
            target: a.id.clone(),
            at: 500,
            rationale: "decide after the trial".into(),
        },
        101,
        "human:alex",
    );
    let canon = Log::from_acts(vec![a.clone(), horizon.clone()]).derive();
    assert_eq!(canon.overdue(1_000).len(), 1);

    let settled = Log::from_acts(vec![
        a.clone(),
        horizon,
        Act::new(
            ActKind::Supersede {
                text: "No shoes indoors.".into(),
                old: vec![a.id.clone()],
                rationale: "the trial went fine".into(),
            },
            600,
            "human:alex",
        ),
    ])
    .derive();
    assert!(
        settled.overdue(1_000).is_empty(),
        "the trial ended in a decision"
    );
}

#[test]
fn a_horizon_can_be_moved_and_the_last_one_wins() {
    let a = Act::new(
        ActKind::Assert {
            text: "x".into(),
            from: None,
            source: None,
        },
        100,
        "human:alex",
    );
    let canon = Log::from_acts(vec![
        a.clone(),
        Act::new(
            ActKind::Horizon {
                target: a.id.clone(),
                at: 500,
                rationale: "first".into(),
            },
            101,
            "human:alex",
        ),
        Act::new(
            ActKind::Horizon {
                target: a.id.clone(),
                at: 5_000,
                rationale: "given another season".into(),
            },
            102,
            "human:alex",
        ),
    ])
    .derive();
    assert_eq!(canon.horizons().len(), 1, "one date per target, not two");
    assert!(canon.overdue(1_000).is_empty(), "the later date governs");
    assert_eq!(canon.overdue(6_000).len(), 1);
}

#[test]
fn a_revisit_date_nobody_can_read_is_reported_and_never_read_as_epoch_zero() {
    // Both wrong answers are bad: dropping it loses a real intention, and
    // treating it as zero makes it permanently overdue, which is how the
    // whole query becomes noise.
    let a = Act::new(
        ActKind::Assert {
            text: "a".into(),
            from: None,
            source: None,
        },
        100,
        "human:alex",
    );
    let b = Act::new(
        ActKind::Assert {
            text: "b".into(),
            from: None,
            source: None,
        },
        101,
        "human:alex",
    );
    let canon = Log::from_acts(vec![
        a.clone(),
        b.clone(),
        Act::new(
            ActKind::Accept {
                a: a.id.clone(),
                b: b.id.clone(),
                rationale: "both matter".into(),
                revisit: Some("in the spring".into()),
            },
            102,
            "human:alex",
        ),
    ])
    .derive();
    assert!(
        canon.overdue(i64::MAX / 2).is_empty(),
        "not overdue, because nobody knows when it is due"
    );
    let unreadable = canon.unreadable_dates();
    assert_eq!(unreadable.len(), 1);
    assert_eq!(unreadable[0].1, "in the spring");
}

#[test]
fn the_staleness_query_takes_a_clock_and_never_reads_one() {
    // Pinned because `canon replay` depends on it completely: an answer that
    // changes with the wall clock is not a replay. This is a compile-time
    // fact — `overdue` has no way to ask what time it is — and the assertion
    // is that two calls at different `now` differ only by `now`.
    let canon = Log::from_acts(vec![Act::new(
        ActKind::Grant {
            holder: "human:dana".into(),
            scope: Scope::new("house").unwrap(),
            horizon: Some(1_000),
            rationale: String::new(),
        },
        100,
        "human:alex",
    )])
    .derive();
    assert!(canon.overdue(999).is_empty());
    assert_eq!(canon.overdue(1_001).len(), 1);
    assert_eq!(canon.overdue(1_001), canon.overdue(1_001), "and it is pure");
}

// ── silence, and the voice record ───────────────────────────
