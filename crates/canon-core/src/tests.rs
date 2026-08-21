// SPDX-License-Identifier: AGPL-3.0-or-later
//! The invariants the rest of the design leans on. Each test names the
//! property rather than the mechanism.

use crate::*;

fn assert_c(text: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: None,
            source: None,
        },
        ts,
        "human:sam",
    )
}

#[test]
fn ids_are_content_addressed_and_stable_across_runs() {
    let a = assert_c("mornings are protected", 1000);
    let b = assert_c("mornings are protected", 1000);
    assert_eq!(
        a.id, b.id,
        "same content, same actor, same second => same id"
    );

    let c = assert_c("mornings are not protected", 1000);
    assert_ne!(a.id, c.id, "different content => different id");
    assert!(a.id.as_str().starts_with("can-"));
}

#[test]
fn merge_of_two_branches_folds_identically_to_either_order() {
    let x = assert_c("quiet hours at 11", 100);
    let y = assert_c("guests two nights", 200);

    // Two machines append independently; a union merge interleaves them
    // arbitrarily. Both orderings must fold to the same state.
    let one = Log::from_acts(vec![x.clone(), y.clone()]);
    let two = Log::from_acts(vec![y, x]);
    assert_eq!(one.derive(), two.derive());
    assert_eq!(one.render(), two.render());
}

#[test]
fn duplicate_acts_collapse_so_union_merge_is_safe() {
    let a = assert_c("one rule", 100);
    let log = Log::from_acts(vec![a.clone(), a.clone(), a]);
    assert_eq!(
        log.len(),
        1,
        "same act from both sides of a merge is one line"
    );
}

#[test]
fn supersede_retires_the_old_and_records_what_replaced_it() {
    let old = assert_c("quiet hours at 11", 100);
    let new = Act::new(
        ActKind::Supersede {
            text: "quiet hours at 10 on weeknights".into(),
            old: vec![old.id.clone()],
            rationale: "house meeting".into(),
        },
        200,
        "human:priya",
    );
    let st = Log::from_acts(vec![old.clone(), new.clone()]).derive();

    assert_eq!(st.active().count(), 1);
    assert_eq!(st.active().next().unwrap().id, new.id);
    assert!(matches!(
        st.get(&old.id).unwrap().status,
        Status::Superseded { ref by } if by == &new.id
    ));
    assert_eq!(st.get(&new.id).unwrap().replaces, vec![old.id]);
}

#[test]
fn revert_tombstones_an_act_and_its_effects() {
    let old = assert_c("quiet hours at 11", 100);
    let sup = Act::new(
        ActKind::Supersede {
            text: "quiet hours at 10".into(),
            old: vec![old.id.clone()],
            rationale: String::new(),
        },
        200,
        "human:priya",
    );
    let rev = Act::new(
        ActKind::Revert {
            targets: vec![sup.id.clone()],
            rationale: "wrong article".into(),
        },
        300,
        "human:priya",
    );

    let st = Log::from_acts(vec![old.clone(), sup, rev]).derive();
    assert_eq!(st.active().count(), 1, "the supersession never happened");
    assert_eq!(
        st.active().next().unwrap().id,
        old.id,
        "the original is live again"
    );
}

#[test]
fn reverting_a_revert_reapplies_the_original() {
    let old = assert_c("quiet hours at 11", 100);
    let sup = Act::new(
        ActKind::Supersede {
            text: "quiet hours at 10".into(),
            old: vec![old.id.clone()],
            rationale: String::new(),
        },
        200,
        "human:priya",
    );
    let rev = Act::new(
        ActKind::Revert {
            targets: vec![sup.id.clone()],
            rationale: String::new(),
        },
        300,
        "human:priya",
    );
    let unrev = Act::new(
        ActKind::Revert {
            targets: vec![rev.id.clone()],
            rationale: "actually it was right".into(),
        },
        400,
        "human:priya",
    );

    let st = Log::from_acts(vec![old.clone(), sup.clone(), rev, unrev]).derive();
    assert_eq!(
        st.active().next().unwrap().id,
        sup.id,
        "supersession is back in force"
    );
    assert!(matches!(
        st.get(&old.id).unwrap().status,
        Status::Superseded { .. }
    ));
}

#[test]
fn a_tolerated_contradiction_is_first_class_and_carries_its_reason() {
    let a = assert_c("mornings are protected", 100);
    let b = assert_c("be someone the team can rely on", 110);
    let acc = Act::new(
        ActKind::Accept {
            a: a.id.clone(),
            b: b.id.clone(),
            rationale: "reliability is how I earn the autonomy, for now".into(),
            revisit: Some("2026-12-01".into()),
        },
        200,
        "human:sam",
    );

    let st = Log::from_acts(vec![a.clone(), b.clone(), acc]).derive();
    assert_eq!(st.active().count(), 2, "neither side is eliminated");
    assert_eq!(st.tolerated().count(), 1);
    assert!(st.is_settled(&a.id, &b.id));
    assert!(st.is_settled(&b.id, &a.id), "settlement is symmetric");
    assert!(matches!(
        st.conflicts[0].disposition,
        Disposition::Tolerated { ref revisit, .. } if revisit.is_some()
    ));
}

#[test]
fn adjudication_by_a_non_human_actor_is_reported_not_hidden() {
    let a = assert_c("one", 100);
    let b = assert_c("two", 110);
    let by_agent = Act::new(
        ActKind::Accept {
            a: a.id.clone(),
            b: b.id.clone(),
            rationale: "seems fine".into(),
            revisit: None,
        },
        200,
        "agent:helper",
    );

    let st = Log::from_acts(vec![a, b, by_agent.clone()]).derive();
    assert_eq!(st.unattended, vec![by_agent.id], "the fold surfaces it");
}

#[test]
fn asserting_is_machine_work_and_is_not_flagged() {
    let drafted = Act::new(
        ActKind::Assert {
            text: "mornings are protected".into(),
            from: None,
            source: Some("journal/2026-03-14.md".into()),
        },
        100,
        "draft",
    );
    let st = Log::from_acts(vec![drafted]).derive();
    assert!(
        st.unattended.is_empty(),
        "extraction is expected to be machine work"
    );
}

#[test]
fn ancestry_lives_in_the_log_so_it_survives_a_file_that_travels_alone() {
    let adopt = Act::new(
        ActKind::Adopt {
            lineage: "house-12-consensus".into(),
            generation: "v3".into(),
            source: Some("https://codeberg.org/commons/house-12-consensus".into()),
        },
        50,
        "human:dana",
    );
    let st = Log::from_acts(vec![adopt, assert_c("local rule", 100)]).derive();
    let anc = st.ancestry.expect("ancestry recorded");
    assert_eq!(anc.lineage, "house-12-consensus");
    assert_eq!(anc.generation, "v3");
}

#[test]
fn a_future_format_version_is_refused_rather_than_partially_read() {
    let line = r#"{"id":"can-deadbeef0000","v":9999,"ts_unix":1,"actor":"human:x","op":"assert","text":"hi"}"#;
    match Log::parse(line) {
        Err(ParseError::UnknownVersion { found, .. }) => assert_eq!(found, 9999),
        other => panic!("expected refusal, got {other:?}"),
    }
}

#[test]
fn roundtrip_through_jsonl_preserves_every_act() {
    let acts = vec![
        assert_c("one", 100),
        assert_c("two", 200),
        Act::new(
            ActKind::Dismiss {
                a: ActId::from_raw("can-aaaaaaaaaaaa"),
                b: ActId::from_raw("can-bbbbbbbbbbbb"),
                rationale: "different topics".into(),
            },
            300,
            "human:sam",
        ),
    ];
    let rendered = Log::from_acts(acts.clone()).render();
    let reparsed = Log::parse(&rendered).expect("parses");
    assert_eq!(reparsed.acts(), Log::from_acts(acts).acts());
}

// ── regressions found by the first smoke test ───────────────

#[test]
fn supersede_works_when_every_act_shares_one_second() {
    // Scripted use puts many acts in the same second, and the (ts, id)
    // tiebreak can sort a supersession AHEAD of the commitment it retires.
    // The fold introduces every commitment before applying any effect, so
    // this must hold regardless of how the ids happen to compare.
    let old = assert_c("quiet hours at 11", 1787341438);
    let sup = Act::new(
        ActKind::Supersede {
            text: "quiet hours at 10 on weeknights".into(),
            old: vec![old.id.clone()],
            rationale: "house meeting".into(),
        },
        1787341438, // same second
        "human:alex",
    );
    let st = Log::from_acts(vec![old.clone(), sup.clone()]).derive();

    assert_eq!(
        st.active().count(),
        1,
        "the retired commitment must not stay live"
    );
    assert_eq!(st.active().next().unwrap().id, sup.id);
    assert!(
        st.dangling.is_empty(),
        "the target was present, so nothing dangles"
    );
}

#[test]
fn same_second_ordering_does_not_change_the_result() {
    let old = assert_c("original", 500);
    let sup = Act::new(
        ActKind::Supersede {
            text: "replacement".into(),
            old: vec![old.id.clone()],
            rationale: String::new(),
        },
        500,
        "human:alex",
    );
    let forward = Log::from_acts(vec![old.clone(), sup.clone()]).derive();
    let reversed = Log::from_acts(vec![sup, old]).derive();
    assert_eq!(forward, reversed);
}

#[test]
fn an_act_referencing_a_missing_commitment_is_reported_not_ignored() {
    // A truncated log, a hand-edited file, or a snapshot adopted without its
    // history. Silently doing nothing would leave a hole nobody could see.
    let ghost = ActId::from_raw("can-000000000000");
    let sup = Act::new(
        ActKind::Supersede {
            text: "replacement".into(),
            old: vec![ghost.clone()],
            rationale: String::new(),
        },
        100,
        "human:alex",
    );
    let st = Log::from_acts(vec![sup.clone()]).derive();
    assert_eq!(st.dangling, vec![(sup.id, ghost)]);
}

#[test]
fn retracting_a_missing_commitment_is_also_reported() {
    let ghost = ActId::from_raw("can-000000000000");
    let ret = Act::new(
        ActKind::Retract {
            target: ghost.clone(),
            rationale: String::new(),
        },
        100,
        "human:alex",
    );
    let st = Log::from_acts(vec![ret.clone()]).derive();
    assert_eq!(st.dangling, vec![(ret.id, ghost)]);
}

#[test]
fn revert_works_when_it_shares_a_second_with_its_target() {
    // The second instance of the same-second defect, found by the smoke test
    // after the first was fixed: a backward walk over sorted acts placed the
    // Revert BEFORE the act it cancelled, so the cancellation never applied
    // and `undo` silently did nothing. Liveness now resolves by reference.
    let ts = 1787341438;
    let old = assert_c("quiet hours at 11", ts);
    let sup = Act::new(
        ActKind::Supersede {
            text: "quiet hours at 10".into(),
            old: vec![old.id.clone()],
            rationale: String::new(),
        },
        ts,
        "human:alex",
    );
    let rev = Act::new(
        ActKind::Revert {
            targets: vec![sup.id.clone()],
            rationale: "minutes were wrong".into(),
        },
        ts, // same second as everything else
        "human:alex",
    );

    let st = Log::from_acts(vec![old.clone(), sup, rev]).derive();
    assert_eq!(st.active().count(), 1, "the supersession must be undone");
    assert_eq!(
        st.active().next().unwrap().id,
        old.id,
        "the original is live again"
    );
}

#[test]
fn revert_of_revert_holds_at_one_timestamp_too() {
    let ts = 500;
    let old = assert_c("original", ts);
    let sup = Act::new(
        ActKind::Supersede {
            text: "replacement".into(),
            old: vec![old.id.clone()],
            rationale: String::new(),
        },
        ts,
        "human:alex",
    );
    let rev = Act::new(
        ActKind::Revert {
            targets: vec![sup.id.clone()],
            rationale: String::new(),
        },
        ts,
        "human:alex",
    );
    let unrev = Act::new(
        ActKind::Revert {
            targets: vec![rev.id.clone()],
            rationale: String::new(),
        },
        ts,
        "human:alex",
    );
    let st = Log::from_acts(vec![old.clone(), sup.clone(), rev, unrev]).derive();
    assert_eq!(
        st.active().next().unwrap().id,
        sup.id,
        "supersession back in force"
    );
}

#[test]
fn liveness_is_independent_of_the_order_acts_arrive_in() {
    let ts = 900;
    let a = assert_c("one", ts);
    let sup = Act::new(
        ActKind::Supersede {
            text: "two".into(),
            old: vec![a.id.clone()],
            rationale: String::new(),
        },
        ts,
        "human:alex",
    );
    let rev = Act::new(
        ActKind::Revert {
            targets: vec![sup.id.clone()],
            rationale: String::new(),
        },
        ts,
        "human:alex",
    );
    let forward = Log::from_acts(vec![a.clone(), sup.clone(), rev.clone()]).derive();
    let shuffled = Log::from_acts(vec![rev, a, sup]).derive();
    assert_eq!(forward, shuffled);
}

// ── the Conflict/Disposition merge ──────────────────────────────

#[test]
fn a_dismissal_carries_its_reason_into_the_read_model() {
    // The capability the merge unlocks. While `dismissed` was a bare
    // `(ActId, ActId)` tuple, the reason someone gave for calling a pair
    // detector noise was written to the log and then thrown away by the
    // fold — so no renderer could show it back.
    let a = assert_c("guests may stay two nights", 100);
    let b = assert_c("guests park in the second bay", 110);
    let dis = Act::new(
        ActKind::Dismiss {
            a: a.id.clone(),
            b: b.id.clone(),
            rationale: "different topics — parking is not staying".into(),
        },
        200,
        "human:sam",
    );

    let st = Log::from_acts(vec![a.clone(), b.clone(), dis]).derive();
    assert_eq!(st.conflicts.len(), 1);
    assert!(matches!(
        st.conflicts[0].disposition,
        Disposition::Dismissed { ref rationale } if rationale.contains("parking")
    ));
    assert!(
        st.is_settled(&a.id, &b.id),
        "a dismissal settles the pair too"
    );
    assert_eq!(st.tolerated().count(), 0, "dismissed is not tolerated");
}

#[test]
fn tolerated_and_dismissed_are_one_noun_and_do_not_collide() {
    let a = assert_c("one", 100);
    let b = assert_c("two", 110);
    let c = assert_c("three", 120);
    let tol = Act::new(
        ActKind::Accept {
            a: a.id.clone(),
            b: b.id.clone(),
            rationale: "both matter".into(),
            revisit: None,
        },
        200,
        "human:sam",
    );
    let dis = Act::new(
        ActKind::Dismiss {
            a: a.id.clone(),
            b: c.id.clone(),
            rationale: String::new(),
        },
        210,
        "human:sam",
    );

    let st = Log::from_acts(vec![a.clone(), b.clone(), c.clone(), tol, dis]).derive();
    assert_eq!(st.conflicts.len(), 2, "both dispositions live in one list");
    assert_eq!(st.tolerated().count(), 1);
    assert!(st.is_settled(&a.id, &b.id));
    assert!(st.is_settled(&a.id, &c.id));
    assert!(
        !st.is_settled(&b.id, &c.id),
        "an unruled pair is not settled"
    );
}

#[test]
fn the_fold_never_mints_an_open_disposition() {
    // `Open` describes a pair some surface proposed and nobody ruled on,
    // which by definition left no act. Only `canon tensions` mints it.
    let a = assert_c("one", 100);
    let b = assert_c("two", 110);
    let acc = Act::new(
        ActKind::Accept {
            a: a.id.clone(),
            b: b.id.clone(),
            rationale: "kept".into(),
            revisit: None,
        },
        200,
        "human:sam",
    );
    let st = Log::from_acts(vec![a, b, acc]).derive();
    assert!(
        !st.conflicts
            .iter()
            .any(|c| c.disposition == Disposition::Open),
        "derive() must not invent an Open conflict"
    );
}
