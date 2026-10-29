// SPDX-License-Identifier: AGPL-3.0-or-later
//! The invariants the rest of the design leans on. Each test names the
//! property rather than the mechanism.

use crate::*;
use alloc::{
    string::{String, ToString},
    vec::Vec,
};

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
            .any(|c| matches!(c.disposition, Disposition::Open { .. })),
        "derive() must not invent an Open conflict"
    );
}

fn ask(text: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Question {
            text: text.into(),
            proposal: None,
        },
        ts,
        "human:sam",
    )
}

#[test]
fn an_open_question_is_what_the_canon_does_not_cover() {
    let st = Log::from_acts(vec![
        assert_c("quiet hours at 11", 100),
        ask("do quiet hours apply to the backyard?", 200),
    ])
    .derive();
    assert_eq!(st.open().count(), 1);
    assert_eq!(st.active().count(), 1, "a question is not a commitment");
}

#[test]
fn a_question_is_answered_by_superseding_it_with_a_commitment() {
    // No new act kind for answering: supersede already means "this replaces
    // that", which is exactly what answering a question is.
    let q = ask("do quiet hours apply to the backyard?", 100);
    let answer = Act::new(
        ActKind::Supersede {
            text: "Quiet hours apply to the backyard at the same times.".into(),
            old: vec![q.id.clone()],
            rationale: "house meeting 2026-02-24".into(),
        },
        200,
        "human:priya",
    );
    let st = Log::from_acts(vec![q.clone(), answer.clone()]).derive();

    assert_eq!(st.open().count(), 0, "answered questions are not open");
    assert_eq!(
        st.question(&q.id).map(|x| x.status.clone()),
        Some(Status::Superseded {
            by: answer.id.clone()
        })
    );
    // And the answer itself is a live commitment.
    assert!(st.active().any(|c| c.id == answer.id));
    assert!(st.dangling.is_empty(), "{:?}", st.dangling);
}

#[test]
fn a_question_is_withdrawn_by_retracting_it() {
    let q = ask("should we have a pet policy?", 100);
    let st = Log::from_acts(vec![
        q.clone(),
        Act::new(
            ActKind::Retract {
                target: q.id.clone(),
                rationale: "nobody wants a pet".into(),
            },
            200,
            "human:sam",
        ),
    ])
    .derive();
    assert_eq!(st.open().count(), 0);
    assert!(matches!(
        st.question(&q.id).map(|x| &x.status),
        Some(Status::Retracted { .. })
    ));
    assert!(st.dangling.is_empty(), "{:?}", st.dangling);
}

#[test]
fn asking_is_not_adjudication_and_is_not_flagged_unattended() {
    // An agent that notices a gap should be able to say so. Recording a
    // question decides nothing, so it is not held to the human-authored bar
    // that `accept` and `dismiss` are.
    let st = Log::from_acts(vec![Act::new(
        ActKind::Question {
            text: "does this cover contractors?".into(),
            proposal: Some("hire a contractor for the roof".into()),
        },
        100,
        "agent:claude-code",
    )])
    .derive();
    assert!(st.unattended.is_empty());
    assert_eq!(st.open().count(), 1);
    assert_eq!(
        st.questions[0].proposal.as_deref(),
        Some("hire a contractor for the roof")
    );
}

#[test]
fn superseding_a_question_that_is_not_in_the_log_is_still_reported() {
    let st = Log::from_acts(vec![Act::new(
        ActKind::Supersede {
            text: "an answer to nothing".into(),
            old: vec![ActId::from_raw("can-000000000000")],
            rationale: String::new(),
        },
        100,
        "human:sam",
    )])
    .derive();
    assert_eq!(st.dangling.len(), 1);
}

// ── v2: the op namespace splits, and the halves have opposite rules ──────

/// One raw JSONL line, so these tests exercise the reader rather than a
/// constructor that could not produce the shape in the first place.
fn line(op: &str, extra: &str) -> String {
    format!(
        r#"{{"id":"can-000000000000","v":2,"ts_unix":100,"actor":"human:sam","op":"{op}"{extra}}}"#
    )
}

#[test]
fn an_unknown_structural_op_is_refused_not_carried() {
    // There is no such thing as a structural op we do not know: the list is
    // closed. But a MALFORMED known one must not slip through as an
    // annotation, which is the way this guard would fail quietly.
    let bad = line("retract", "");
    let err = Log::parse(&bad).expect_err("a retract with no target is not readable");
    assert!(
        matches!(err, ParseError::Malformed { .. }),
        "a malformed structural op refuses the line, it does not become an annotation: {err:?}"
    );
}

#[test]
fn an_unknown_annotation_is_carried_and_round_trips() {
    // The governance move this build has never heard of. Before v2 this was a
    // parse error, which made every new move a breaking change.
    let raw = line(
        "second",
        r#","question":"can-abc","actor_ref":"human:dana""#,
    );
    let log = Log::parse(&raw).expect("an unknown annotation is readable");
    assert_eq!(log.len(), 1);
    let ActKind::Annotation { kind, body } = &log.acts()[0].kind else {
        panic!("expected an Annotation, got {:?}", log.acts()[0].kind);
    };
    assert_eq!(kind, "second");
    assert_eq!(
        body.get("question").and_then(|v| v.as_str()),
        Some("can-abc")
    );
    assert!(
        !body.contains_key("op"),
        "`op` is the kind, not part of the body"
    );

    // Round-trips: nothing is lost by reading a log we do not fully understand.
    let again = Log::parse(&log.render()).expect("re-readable");
    assert_eq!(again.acts()[0].kind, log.acts()[0].kind);
    assert_eq!(again.acts()[0].id, log.acts()[0].id, "the id survives");
}

#[test]
fn a_carried_annotation_changes_nothing_and_is_reported() {
    // "Not interpreted" has to mean it cannot reach the fold at all — that is
    // what makes carrying an unknown move safe rather than a hole.
    let known = Log::from_acts(vec![assert_c("quiet hours at 11", 100)]);
    let raw = format!(
        "{}\n{}",
        known.render().trim(),
        line("sanction", r#","who":"human:dana","step":2"#)
    );
    let mixed = Log::parse(&raw).expect("readable");

    let a = known.derive();
    let b = mixed.derive();
    assert_eq!(
        a.commitments, b.commitments,
        "an unread act changes nothing"
    );
    assert_eq!(a.conflicts, b.conflicts);
    assert!(a.carried.is_empty());

    // And it is reported rather than silently dropped (§18.3, §4.3).
    assert_eq!(b.carried.len(), 1, "carried is not the same as ignored");
    assert_eq!(b.carried[0].1, "sanction");
    assert!(
        b.unattended.is_empty(),
        "an annotation we did not interpret is not an adjudication"
    );
}

#[test]
fn a_version_beyond_this_build_is_still_refused() {
    // v2 widened what ops mean, not what versions mean. v3 is signing, v4
    // the governance knobs, and the next one is refused the same way.
    let ahead = line("assert", r#","text":"x""#).replace(r#""v":2"#, r#""v":5"#);
    assert!(matches!(
        Log::parse(&ahead),
        Err(ParseError::UnknownVersion { found: 5, .. })
    ));
}

// ── positions: a vote is an act, and the actor is the act's own ──────────

#[test]
fn a_position_citing_nothing_is_the_actors_own() {
    // The whole point of the second source kind. Dana objecting is not a
    // commitment bearing on the proposal; it is a person with a reason.
    let act = Act::new(
        ActKind::Position {
            about: "move standup to 8am".into(),
            citing: None,
            pull: Pull::Against,
            because: "school run until 8:30".into(),
        },
        100,
        "human:dana",
    );
    let canon = Log::from_acts(vec![act.clone()]).derive();
    assert_eq!(canon.positions.len(), 1);
    let stated = &canon.positions[0];
    assert_eq!(stated.about, "move standup to 8am");
    assert_eq!(
        stated.position.actor(),
        Some("human:dana"),
        "the act's actor IS the source — nothing else says who"
    );
    assert!(stated.position.commitment().is_none());
    assert_eq!(stated.act, act.id, "revertible like any other act");
}

#[test]
fn a_position_citing_a_commitment_carries_the_citation() {
    let rule = assert_c("mornings are protected", 100);
    let cited = Act::new(
        ActKind::Position {
            about: "move standup to 8am".into(),
            citing: Some(rule.id.clone()),
            pull: Pull::Against,
            because: "8am is inside mornings".into(),
        },
        200,
        "agent:canon",
    );
    let canon = Log::from_acts(vec![rule.clone(), cited]).derive();
    assert_eq!(canon.positions[0].position.commitment(), Some(&rule.id));
    assert!(
        canon.positions[0].position.actor().is_none(),
        "a cited position rests on the rule, not on whoever noticed it"
    );
}

#[test]
fn an_agent_may_cite_a_rule_but_may_not_hold_an_opinion() {
    // The line PRIMITIVES.md draws, falling out of the type rather than
    // being remembered. Citing a commitment is a READING and is what an
    // agent is for. Taking your own position is a STANCE — and under a
    // consent policy one reasoned objection blocks, so an agent that may
    // object may veto.
    let rule = assert_c("mornings are protected", 100);
    let reading = Act::new(
        ActKind::Position {
            about: "p".into(),
            citing: Some(rule.id.clone()),
            pull: Pull::Against,
            because: "reads against it".into(),
        },
        200,
        "agent:canon",
    );
    let stance = Act::new(
        ActKind::Position {
            about: "p".into(),
            citing: None,
            pull: Pull::Against,
            because: "I do not like it".into(),
        },
        300,
        "agent:canon",
    );

    let cited_only = Log::from_acts(vec![rule.clone(), reading.clone()]).derive();
    assert!(
        cited_only.unattended.is_empty(),
        "an agent citing a rule is reading, not ruling"
    );

    let with_stance = Log::from_acts(vec![rule, reading, stance.clone()]).derive();
    assert_eq!(
        with_stance.unattended,
        vec![stance.id],
        "an agent's own position is an adjudication and is surfaced"
    );

    // And the same stance from a person is not flagged at all.
    let human = Act::new(
        ActKind::Position {
            about: "p".into(),
            citing: None,
            pull: Pull::Against,
            because: "school run".into(),
        },
        400,
        "human:dana",
    );
    let by_person = Log::from_acts(vec![human]).derive();
    assert!(by_person.unattended.is_empty());
}

#[test]
fn a_reverted_position_leaves_no_trace_in_the_fold() {
    let p = Act::new(
        ActKind::Position {
            about: "p".into(),
            citing: None,
            pull: Pull::Toward,
            because: "fine by me".into(),
        },
        100,
        "human:dana",
    );
    let undo = Act::new(
        ActKind::Revert {
            targets: vec![p.id.clone()],
            rationale: "wrong proposal".into(),
        },
        200,
        "human:dana",
    );
    let canon = Log::from_acts(vec![p, undo]).derive();
    assert!(canon.positions.is_empty(), "revert tombstones its effects");
}

// ── scope: who holds standing, over what (Ostrom #1) ────────────────────

fn scope(s: &str) -> Scope {
    Scope::new(s).expect("valid scope")
}

fn grant(actor: &str, s: &str, horizon: Option<i64>, ts: i64) -> Act {
    Act::new(
        ActKind::Grant {
            holder: actor.into(),
            scope: scope(s),
            horizon,
            rationale: String::new(),
        },
        ts,
        "human:sam",
    )
}

#[test]
fn who_decides_answers_without_anyone_having_to_be_asked() {
    // The Freeman floor. If finding out who decides needs knowing whom to
    // ask, that person is the gatekeeper.
    let canon = Log::from_acts(vec![
        grant("human:dana", "house.kitchen", None, 100),
        grant("human:sam", "house", None, 100),
    ])
    .derive();

    let who = canon.who_decides(&scope("house.kitchen"), 150);
    assert_eq!(who.len(), 2, "both the specific and the general cover it");
    assert_eq!(
        who[0].actor, "human:dana",
        "deepest first — that ordering IS subsidiarity"
    );

    // And the house-wide grant does not reach a scope it never covered.
    let elsewhere = canon.who_decides(&scope("network"), 150);
    assert!(elsewhere.is_empty());
}

#[test]
fn standing_is_held_not_remembered() {
    // Rotation is the default shape, so a lapsed grant stops deciding. The
    // grant is not deleted — it happened — but it no longer answers.
    let canon = Log::from_acts(vec![grant("human:dana", "house.kitchen", Some(200), 100)]).derive();
    assert!(canon.standing_of("human:dana", &scope("house.kitchen"), 150));
    assert!(!canon.standing_of("human:dana", &scope("house.kitchen"), 300));
    assert_eq!(
        canon.grants.len(),
        1,
        "the fact is kept, the authority is not"
    );
}

#[test]
fn stepping_back_from_a_scope_removes_what_it_covers() {
    // Withdrawal read as a first-class move: the pre-exit signal, without
    // demanding a confrontation from someone already disengaging.
    let out = Act::new(
        ActKind::Withdraw {
            holder: "human:dana".into(),
            scope: scope("house"),
            rationale: "moving out in spring".into(),
        },
        300,
        "human:dana",
    );
    let canon = Log::from_acts(vec![
        grant("human:dana", "house.kitchen", None, 100),
        grant("human:dana", "house.garden", None, 100),
        grant("human:sam", "house.kitchen", None, 100),
        out,
    ])
    .derive();

    assert!(!canon.standing_of("human:dana", &scope("house.kitchen"), 400));
    assert!(!canon.standing_of("human:dana", &scope("house.garden"), 400));
    assert!(
        canon.standing_of("human:sam", &scope("house.kitchen"), 400),
        "and takes nobody else's standing with it"
    );
}

#[test]
fn re_granting_closes_the_old_grant_rather_than_stacking_on_it() {
    // Two grants LIVE AT ONE INSTANT would make "when does this lapse" have
    // two answers. The old one is closed, not deleted: standing is an as-of
    // question, and a renewal today must not rewrite who held it in March.
    // Sam founds the house first: in a governed canon, granting takes standing.
    let canon = Log::from_acts(vec![
        grant("human:sam", "house", None, 50),
        grant("human:dana", "house.kitchen", Some(200), 100),
        grant("human:dana", "house.kitchen", Some(900), 300),
    ])
    .derive();
    let dana: Vec<_> = canon
        .grants
        .iter()
        .filter(|g| g.actor == "human:dana")
        .collect();
    assert_eq!(dana.len(), 2, "both are facts that happened");
    let live: Vec<_> = dana.iter().filter(|g| g.held_at(500)).collect();
    assert_eq!(live.len(), 1, "but only one is held at any instant");
    assert_eq!(live[0].horizon, Some(900), "the renewal wins");
    assert!(canon.standing_of("human:dana", &scope("house.kitchen"), 500));
    // And the old term is still answerable about its own window.
    assert!(dana[0].held_at(150));
    assert!(!dana[0].held_at(500));
}

#[test]
fn a_commitment_can_be_scoped_and_rescoped() {
    let rule = assert_c("wipe the stovetop after cooking", 100);
    let first = Act::new(
        ActKind::Scoped {
            commitment: rule.id.clone(),
            scope: scope("house"),
        },
        200,
        "human:sam",
    );
    let corrected = Act::new(
        ActKind::Scoped {
            commitment: rule.id.clone(),
            scope: scope("house.kitchen"),
        },
        300,
        "human:sam",
    );
    let canon = Log::from_acts(vec![rule.clone(), first, corrected]).derive();
    assert_eq!(canon.scope_of(&rule.id), Some(&scope("house.kitchen")));
    assert_eq!(
        canon.scopes.len(),
        1,
        "last write wins, it does not accumulate"
    );
}

#[test]
fn granting_standing_is_an_adjudication_and_a_machine_one_is_surfaced() {
    // An agent that can grant itself standing has escalated its own
    // authority. It is not refused here — the fold has no policy — but it is
    // never invisible.
    let by_agent = Act::new(
        ActKind::Grant {
            holder: "agent:canon".into(),
            scope: scope("house"),
            horizon: None,
            rationale: "convenient".into(),
        },
        100,
        "agent:canon",
    );
    let canon = Log::from_acts(vec![by_agent.clone()]).derive();
    assert_eq!(canon.unattended, vec![by_agent.id]);
}

// ── the wire, exhaustively ──────────────────────────────────

/// One instance of every act kind this build understands.
///
/// Kept beside the exhaustiveness assertion below, which is what makes it a
/// gate rather than a list somebody remembers to update.
fn every_kind() -> Vec<ActKind> {
    let id = ActId::from_raw("can-000000000001");
    let scope = crate::scope::Scope::new("house.kitchen").unwrap();
    vec![
        ActKind::Assert {
            text: "a".into(),
            from: Some(id.clone()),
            source: Some("notes.md".into()),
        },
        ActKind::Supersede {
            text: "b".into(),
            old: vec![id.clone()],
            rationale: "because".into(),
        },
        ActKind::Retract {
            target: id.clone(),
            rationale: "because".into(),
        },
        ActKind::Revert {
            targets: vec![id.clone()],
            rationale: "because".into(),
        },
        ActKind::Accept {
            a: id.clone(),
            b: id.clone(),
            rationale: "protects".into(),
            revisit: Some("2026-12-31".into()),
        },
        ActKind::Dismiss {
            a: id.clone(),
            b: id.clone(),
            rationale: String::new(),
        },
        ActKind::Question {
            text: "?".into(),
            proposal: Some("p".into()),
        },
        ActKind::Adopt {
            lineage: "l".into(),
            generation: "g1".into(),
            source: Some("https://example.org".into()),
        },
        ActKind::Position {
            about: "p".into(),
            citing: Some(id.clone()),
            pull: crate::standing::Pull::Against,
            because: "why".into(),
        },
        ActKind::Grant {
            holder: "human:dana".into(),
            scope: scope.clone(),
            horizon: Some(200),
            rationale: "rota".into(),
        },
        ActKind::Withdraw {
            holder: "human:dana".into(),
            scope: scope.clone(),
            rationale: "moving out".into(),
        },
        ActKind::Scoped {
            commitment: id.clone(),
            scope: scope.clone(),
        },
        ActKind::Policy {
            text: "consent".into(),
            rule: crate::policy::Rule::Consent,
            scope: Some(scope),
        },
        ActKind::Decided {
            about: "quiet hours".into(),
            outcome: crate::standing::Outcome::Conflicts,
            authority: crate::policy::Authority::AskOne,
            rationale: "asked once".into(),
            citing: Vec::new(),
        },
        ActKind::Horizon {
            target: id.clone(),
            at: 200,
            rationale: "trial period".into(),
        },
        ActKind::Silence {
            about: "who cooks on a wednesday".into(),
            rationale: "it works and writing it down would break it".into(),
        },
        ActKind::DrawCommit {
            scope: crate::scope::Scope::new("house").unwrap(),
            count: 3,
            after_ts: 1_000,
            rationale: "kitchen panel".into(),
        },
        ActKind::DrawSecret {
            commit: id.clone(),
            digest: crate::id::digest_hex(b"s"),
        },
        ActKind::DrawReveal {
            commit: id.clone(),
            secret: "s".into(),
        },
        ActKind::Key {
            keys: vec![
                "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIlizSIN3DQV78VPNjVvvwjgf+P5HqbPYCYu3BOMdjAE"
                    .into(),
            ],
            prior: vec![crate::id::digest_hex(b"p")],
        },
        ActKind::Rank {
            commitment: id,
            rank: "principle".into(),
        },
        ActKind::Allot {
            text: "the sites".into(),
            unit: "site".into(),
            units: vec!["kizilburun".into(), "incekum".into()],
            scope: crate::scope::Scope::new("fishery.sites").unwrap(),
        },
        ActKind::Allocation {
            text: "one east a day".into(),
            rule: crate::allot::Allocation::Rotation {
                order: crate::allot::Order::FromDraw {
                    commit: ActId::from_raw("can-000000000001"),
                },
                step: 1,
                per: 86_400,
            },
            scope: crate::scope::Scope::new("fishery.sites").unwrap(),
        },
        ActKind::Ratification {
            text: "the cooks decide together".into(),
            rule: crate::ratify::Ratify::Joint {
                holders: vec!["human:dana".into(), "human:sam".into()],
            },
            scope: Some(Scope::new("house.kitchen").unwrap()),
            governs: None,
            personal: None,
        },
        ActKind::Annotation {
            kind: "from-the-future".into(),
            body: serde_json::Map::new(),
        },
    ]
}

/// The op each kind writes. No wildcard, so a new variant does not compile
/// until it is named here.
fn op_of(kind: &ActKind) -> &'static str {
    match kind {
        ActKind::Assert { .. } => "assert",
        ActKind::Supersede { .. } => "supersede",
        ActKind::Retract { .. } => "retract",
        ActKind::Revert { .. } => "revert",
        ActKind::Accept { .. } => "accept",
        ActKind::Dismiss { .. } => "dismiss",
        ActKind::Question { .. } => "question",
        ActKind::Adopt { .. } => "adopt",
        ActKind::Position { .. } => "position",
        ActKind::Grant { .. } => "grant",
        ActKind::Withdraw { .. } => "withdraw",
        ActKind::Scoped { .. } => "scoped",
        ActKind::Policy { .. } => "policy",
        ActKind::Decided { .. } => "decided",
        ActKind::Horizon { .. } => "horizon",
        ActKind::Silence { .. } => "silence",
        ActKind::DrawCommit { .. } => "draw_commit",
        ActKind::DrawSecret { .. } => "draw_secret",
        ActKind::DrawReveal { .. } => "draw_reveal",
        ActKind::Rank { .. } => "rank",
        ActKind::Ratification { .. } => "ratification",
        ActKind::Allot { .. } => "allot",
        ActKind::Allocation { .. } => "allocation",
        ActKind::Key { .. } => "key",
        ActKind::Annotation { kind, .. } => {
            assert_eq!(kind, "from-the-future");
            "(carried)"
        }
    }
}

#[test]
fn every_act_kind_survives_the_wire_with_its_id_intact() {
    // **The failure this catches, which nothing else did.** `grant` carried a
    // body field named `actor`, and the body is FLATTENED into the same JSON
    // object as the envelope — which already has an `actor`, the person doing
    // the granting. Every grant written to disk produced a line with two
    // `actor` keys, and the next read of that canon died with "duplicate
    // field `actor`". A canon that could be written once and never reopened.
    //
    // Every test that built acts in memory passed straight through it,
    // because the collision only exists on the wire. This is the gate that
    // was missing, and it was found by running the CLI rather than the suite.
    for kind in every_kind() {
        let op = op_of(&kind);
        let act = Act::new(kind, 100, "human:alex");
        let line = serde_json::to_string(&act).expect("serializes");
        let back: Act = serde_json::from_str(&line)
            .unwrap_or_else(|e| panic!("`{op}` does not survive the wire: {e}\n  {line}"));
        assert_eq!(back, act, "`{op}` came back different");
        assert_eq!(back.id, act.id, "`{op}` changed its id on the way back");

        // And through the log, which is the path an actual file takes.
        let log = Log::parse(&format!("{line}\n"))
            .unwrap_or_else(|e| panic!("`{op}` does not parse as a log line: {e}"));
        assert_eq!(log.len(), 1, "`{op}`");
        assert_eq!(
            log.render(),
            format!("{line}\n"),
            "`{op}` re-renders differently"
        );
    }
}

#[test]
fn the_round_trip_covers_every_op_this_build_knows() {
    // What makes the test above a gate. `KNOWN_ANNOTATIONS` and `STRUCTURAL`
    // are the two lists a new act kind must join to be read strictly, so
    // deriving the expected count from them means adding an op fails here
    // until it also has a round-trip instance.
    use crate::act::{KNOWN_ANNOTATIONS, STRUCTURAL};
    let mut ops: Vec<&str> = every_kind().iter().map(op_of).collect();
    ops.sort_unstable();
    ops.dedup();
    assert_eq!(
        ops.len(),
        STRUCTURAL.len() + KNOWN_ANNOTATIONS.len() + 1,
        "an op was added without a round-trip instance: {ops:?}"
    );
    for op in STRUCTURAL.iter().chain(KNOWN_ANNOTATIONS.iter()) {
        assert!(ops.contains(op), "`{op}` has no round-trip instance");
    }
}

// ── horizons ────────────────────────────────────────────────

#[test]
fn one_query_answers_a_term_limit_a_trial_period_and_a_revisit_date() {
    // The generalization claim, asserted as one call returning three kinds.
    // If these needed three queries they would be three mechanisms, and
    // `PRIMITIVES.md` would be wrong about Primitive 8.
    //
    // Real epoch seconds throughout, because one of the three dates lives in
    // the format as a STRING (`accept.revisit`) and the point is that both
    // spellings compare through one calendar.
    let jan = crate::date::parse_ymd("2026-01-01").unwrap();
    let feb = crate::date::parse_ymd("2026-02-01").unwrap();
    let jun = crate::date::parse_ymd("2026-06-01").unwrap();
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
                scope: crate::scope::Scope::new("house.kitchen").unwrap(),
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
    assert!(
        matches!(due[0].what, crate::horizon::Due::Standing { .. }),
        "february"
    );
    assert!(
        matches!(due[1].what, crate::horizon::Due::Revisit { .. }),
        "march"
    );
    assert!(
        matches!(due[2].what, crate::horizon::Due::Horizon { .. }),
        "june"
    );
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
    assert_eq!(canon.horizons.len(), 1, "one date per target, not two");
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
            scope: crate::scope::Scope::new("house").unwrap(),
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

#[test]
fn a_deliberate_silence_is_a_third_state_and_not_a_gap() {
    // The métis floor. A tool whose only two states are "written" and
    // "missing" reads every unwritten norm as a gap and every gap as an
    // invitation to legislate — which is precisely how making a place legible
    // destroys what it was running on.
    let canon = Log::from_acts(vec![Act::new(
        ActKind::Silence {
            about: "who cooks on a wednesday".into(),
            rationale: "it works, and writing it down would turn it into a rota".into(),
        },
        100,
        "human:alex",
    )])
    .derive();
    let s = canon
        .silence_about("who cooks on a wednesday")
        .expect("recorded");
    assert!(s.rationale.contains("rota"));
    // And it never spreads by resemblance.
    assert!(canon.silence_about("who cooks").is_none());
    assert!(canon
        .silence_about("who cooks on a wednesday evening")
        .is_none());
}

#[test]
fn the_last_word_on_a_subject_is_the_silence_that_stands() {
    let canon = Log::from_acts(vec![
        Act::new(
            ActKind::Silence {
                about: "guests".into(),
                rationale: "first".into(),
            },
            100,
            "human:alex",
        ),
        Act::new(
            ActKind::Silence {
                about: "guests".into(),
                rationale: "we have talked about it since".into(),
            },
            200,
            "human:sam",
        ),
    ])
    .derive();
    assert_eq!(canon.silences.len(), 1);
    assert_eq!(canon.silence_about("guests").unwrap().actor, "human:sam");
}

#[test]
fn a_voice_record_says_whether_raising_things_has_ever_gone_anywhere() {
    // Hirschman: voice is only rational if it works, and whether it has
    // worked FOR YOU is exactly what a person cannot verify from memory and
    // will not ask about aloud.
    let asked = Act::new(
        ActKind::Question {
            text: "is it ok to run the machine after midnight?".into(),
            proposal: None,
        },
        100,
        "human:dana",
    );
    let ignored = Act::new(
        ActKind::Question {
            text: "what about the bins?".into(),
            proposal: None,
        },
        101,
        "human:dana",
    );
    let canon = Log::from_acts(vec![
        asked.clone(),
        ignored,
        Act::new(
            ActKind::Supersede {
                text: "The machine is off after 11.".into(),
                old: vec![asked.id.clone()],
                rationale: "answered at the meeting".into(),
            },
            200,
            "human:alex",
        ),
        Act::new(
            ActKind::Position {
                about: "bike storage".into(),
                citing: None,
                pull: crate::standing::Pull::Against,
                because: "no room in the hall".into(),
            },
            300,
            "human:dana",
        ),
    ])
    .derive();

    let dana = canon.voice_of("human:dana");
    assert_eq!(dana.asked.len(), 2);
    assert_eq!(dana.answered(), 1, "one of her questions became a rule");
    assert_eq!(dana.open(), 1, "and one has gone nowhere");
    assert_eq!(dana.positions.len(), 1);
    assert!(dana.decided.is_empty(), "she has never adjudicated");

    // It is a view of the record, not a file on a person: somebody who has
    // put nothing in has nothing in it.
    assert!(canon.voice_of("human:nobody").is_empty());
}

#[test]
fn a_voice_record_credits_the_citer_and_not_only_the_cited() {
    let a = Act::new(
        ActKind::Assert {
            text: "Mornings are protected.".into(),
            from: None,
            source: None,
        },
        100,
        "human:alex",
    );
    let canon = Log::from_acts(vec![
        a.clone(),
        Act::new(
            ActKind::Position {
                about: "8am standup".into(),
                citing: Some(a.id.clone()),
                pull: crate::standing::Pull::Against,
                because: "8am is inside mornings".into(),
            },
            200,
            "agent:claude",
        ),
    ])
    .derive();
    // The position's SOURCE is the commitment; the person who did the citing
    // is a different fact, and it is the one a voice record needs.
    assert_eq!(canon.voice_of("agent:claude").positions.len(), 1);
    assert_eq!(canon.voice_of("human:alex").positions.len(), 0);
}

#[test]
fn holding_a_scope_and_its_parent_makes_you_one_decider_and_not_two() {
    // "Who decides this?" is a question about PEOPLE. Somebody granted the
    // house and then the kitchen appeared twice, which reads as a larger
    // group than the house has — and quorum is counted off this list.
    let canon = Log::from_acts(vec![
        grant("human:sam", "house", None, 100),
        grant("human:dana", "house", None, 101),
        grant("human:dana", "house.kitchen", None, 102),
    ])
    .derive();
    let who = canon.who_decides(&scope("house.kitchen"), 200);
    assert_eq!(who.len(), 2);
    assert_eq!(who[0].actor, "human:dana");
    assert_eq!(
        who[0].scope.as_str(),
        "house.kitchen",
        "and at the narrowest standing they hold, which is what subsidiarity routes on"
    );
    assert!(canon.standing_of("human:sam", &scope("house.kitchen"), 200));
}

// ── ratification: how a proposal becomes a rule ─────────────

fn assert_at(text: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: None,
            source: None,
        },
        ts,
        actor,
    )
}

fn scoped_at(commitment: &ActId, s: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Scoped {
            commitment: commitment.clone(),
            scope: scope(s),
        },
        ts,
        "human:sam",
    )
}

fn ratification_at(rule: &str, s: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Ratification {
            text: String::new(),
            rule: crate::ratify::Ratify::parse(rule).expect("a rule"),
            scope: Some(scope(s)),
            governs: None,
            personal: None,
        },
        ts,
        actor,
    )
}

fn approve_at(target: &ActId, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Position {
            about: target.to_string(),
            citing: None,
            pull: Pull::Toward,
            because: "yes".into(),
        },
        ts,
        actor,
    )
}

fn object_at(target: &ActId, ts: i64, actor: &str, why: &str) -> Act {
    Act::new(
        ActKind::Position {
            about: target.to_string(),
            citing: None,
            pull: Pull::Against,
            because: why.into(),
        },
        ts,
        actor,
    )
}

/// Sam founds the house; Dana and Sam hold the kitchen; Theo holds the house.
fn governed_house() -> Vec<Act> {
    vec![
        grant("human:sam", "house", None, 10),
        grant("human:theo", "house", None, 20),
        grant("human:dana", "house.kitchen", None, 20),
        grant("human:sam", "house.kitchen", None, 20),
    ]
}

#[test]
fn an_ungoverned_canon_is_a_notebook_and_every_assert_is_a_rule() {
    // No grants, no gate. This is every canon that existed before
    // ratification, and it must keep folding the same way.
    let canon =
        Log::from_acts(vec![assert_at("Quiet after eleven.", 100, "human:anyone")]).derive();
    assert_eq!(canon.active().count(), 1);
    assert_eq!(canon.proposed().count(), 0);
}

#[test]
fn under_standing_a_holder_writes_and_a_non_holder_proposes() {
    let mut acts = governed_house();
    let by_dana = assert_at("Wipe the stovetop.", 100, "human:dana");
    // Theo holds the house, which covers the kitchen — but the kitchen's
    // rules are the kitchen's holders' to write. Wider standing proposes.
    let by_theo = assert_at("Label your food.", 100, "human:theo");
    acts.push(scoped_at(&by_dana.id, "house.kitchen", 100));
    acts.push(scoped_at(&by_theo.id, "house.kitchen", 100));
    acts.push(by_dana.clone());
    acts.push(by_theo.clone());
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(
        matches!(canon.get(&by_dana.id).unwrap().status, Status::Active),
        "a cook wrote it"
    );
    let Status::Proposed { needs } = &canon.get(&by_theo.id).unwrap().status else {
        panic!("theo does not hold the kitchen, so his line is a proposal")
    };
    assert!(needs.contains("holds house.kitchen"), "{needs}");

    // One holder's approval is enough under `standing`.
    acts.push(approve_at(&by_theo.id, 200, "human:sam"));
    let canon = Log::from_acts(acts).derive();
    assert!(matches!(
        canon.get(&by_theo.id).unwrap().status,
        Status::Active
    ));
}

#[test]
fn joint_ratification_needs_every_named_holder_and_one_objection_refuses() {
    let mut acts = governed_house();
    acts.push(ratification_at(
        "joint:human:dana,human:sam",
        "house.kitchen",
        30,
        "human:dana",
    ));
    let pan = assert_at("Wash your own pan before you sit down.", 100, "human:theo");
    acts.push(scoped_at(&pan.id, "house.kitchen", 100));
    acts.push(pan.clone());
    acts.push(approve_at(&pan.id, 200, "human:dana"));
    let canon = Log::from_acts(acts.clone()).derive();
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("one of two is not both")
    };
    assert_eq!(needs, "approval from human:sam");

    let mut carried = acts.clone();
    carried.push(approve_at(&pan.id, 300, "human:sam"));
    let canon = Log::from_acts(carried).derive();
    assert!(
        matches!(canon.get(&pan.id).unwrap().status, Status::Active),
        "both said yes"
    );

    let mut refused = acts;
    refused.push(object_at(
        &pan.id,
        300,
        "human:sam",
        "the sink is too small for that at dinner",
    ));
    let canon = Log::from_acts(refused).derive();
    let Status::Refused { by, why, .. } = &canon.get(&pan.id).unwrap().status else {
        panic!("a named holder objecting refuses it")
    };
    assert_eq!(by, "human:sam");
    assert!(why.contains("sink"));
}

#[test]
fn approvals_from_people_without_standing_and_from_agents_do_not_count() {
    let mut acts = governed_house();
    acts.push(ratification_at(
        "threshold:2/1",
        "house.kitchen",
        30,
        "human:sam",
    ));
    let p = assert_at("The dishwasher runs only when full.", 100, "agent:helper");
    acts.push(scoped_at(&p.id, "house.kitchen", 100));
    acts.push(p.clone());
    // The agent approving itself, a house member without kitchen standing,
    // and a stranger: none of these are kitchen holders' word.
    acts.push(approve_at(&p.id, 200, "agent:helper"));
    acts.push(approve_at(&p.id, 200, "human:theo"));
    acts.push(approve_at(&p.id, 200, "human:stranger"));
    let canon = Log::from_acts(acts.clone()).derive();
    let Status::Proposed { needs } = &canon.get(&p.id).unwrap().status else {
        panic!("nobody who counts has spoken")
    };
    assert!(needs.starts_with("2 more approval"), "{needs}");

    acts.push(approve_at(&p.id, 300, "human:dana"));
    acts.push(approve_at(&p.id, 300, "human:sam"));
    let canon = Log::from_acts(acts).derive();
    assert!(
        matches!(canon.get(&p.id).unwrap().status, Status::Active),
        "the agent proposed; people disposed"
    );
}

#[test]
fn an_agent_with_standing_still_cannot_mint_its_own_rule() {
    // Ostrom's fourth principle: the monitor is answerable to the people, not
    // the other way round. Standing lets an agent propose and object. It does
    // not let it write a rule that nobody approved.
    let mut acts = governed_house();
    acts.push(grant("agent:helper", "house.kitchen", Some(10_000), 20));
    let p = assert_at("Run the extractor fan when frying.", 100, "agent:helper");
    acts.push(scoped_at(&p.id, "house.kitchen", 100));
    acts.push(p.clone());
    let canon = Log::from_acts(acts).derive();
    let Status::Proposed { needs } = &canon.get(&p.id).unwrap().status else {
        panic!("an agent's assert is a proposal even where it holds standing")
    };
    assert!(needs.contains("not a person"), "{needs}");
}

#[test]
fn consent_ratifies_by_the_clock_and_a_reasoned_objection_stops_it() {
    let mut acts = governed_house();
    acts.push(ratification_at(
        "consent:7d",
        "house.kitchen",
        30,
        "human:sam",
    ));
    let p = assert_at("No phones at the table.", 1_000, "human:theo");
    acts.push(scoped_at(&p.id, "house.kitchen", 1_000));
    acts.push(p.clone());
    let day = 86_400;
    let early = Log::from_acts(acts.clone()).derive_at(1_000 + 3 * day);
    assert!(matches!(
        early.get(&p.id).unwrap().status,
        Status::Proposed { .. }
    ));
    let late = Log::from_acts(acts.clone()).derive_at(1_000 + 8 * day);
    assert!(
        matches!(late.get(&p.id).unwrap().status, Status::Active),
        "silence for a week is consent"
    );

    // An objection with no reason is not an objection.
    acts.push(object_at(&p.id, 1_000 + day, "human:dana", "   "));
    let canon = Log::from_acts(acts.clone()).derive_at(1_000 + 8 * day);
    assert!(matches!(canon.get(&p.id).unwrap().status, Status::Active));
    acts.push(object_at(
        &p.id,
        1_000 + 2 * day,
        "human:dana",
        "we take calls from the night shift at dinner",
    ));
    let canon = Log::from_acts(acts).derive_at(1_000 + 8 * day);
    assert!(matches!(
        canon.get(&p.id).unwrap().status,
        Status::Refused { .. }
    ));
}

#[test]
fn a_proposed_supersession_leaves_the_old_rule_standing() {
    let mut acts = governed_house();
    let old = assert_at("Quiet after eleven.", 100, "human:sam");
    acts.push(old.clone());
    let replacement = Act::new(
        ActKind::Supersede {
            text: "Quiet after ten.".into(),
            old: vec![old.id.clone()],
            rationale: "work nights".into(),
        },
        200,
        "human:stranger",
    );
    acts.push(replacement.clone());
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(
        matches!(canon.get(&old.id).unwrap().status, Status::Active),
        "not replaced yet"
    );
    assert!(matches!(
        canon.get(&replacement.id).unwrap().status,
        Status::Proposed { .. }
    ));

    acts.push(approve_at(&replacement.id, 300, "human:theo"));
    let canon = Log::from_acts(acts).derive();
    assert!(matches!(
        canon.get(&old.id).unwrap().status,
        Status::Superseded { .. }
    ));
    assert!(matches!(
        canon.get(&replacement.id).unwrap().status,
        Status::Active
    ));
}

#[test]
fn governance_acts_without_standing_are_recorded_and_not_applied() {
    let mut acts = governed_house();
    // A stranger grants themselves the kitchen and sets its rule to suit.
    acts.push(Act::new(
        ActKind::Grant {
            holder: "human:stranger".into(),
            scope: scope("house.kitchen"),
            horizon: None,
            rationale: String::new(),
        },
        100,
        "human:stranger",
    ));
    acts.push(ratification_at(
        "standing",
        "house.kitchen",
        110,
        "human:stranger",
    ));
    let canon = Log::from_acts(acts).derive();
    assert!(!canon.standing_of("human:stranger", &scope("house.kitchen"), 500));
    assert!(canon.ratifications.is_empty(), "the rule did not change");
    assert_eq!(
        canon.ungoverned.len(),
        2,
        "and both attempts are on the record: {:?}",
        canon.ungoverned
    );
}

#[test]
fn simultaneous_founding_grants_do_not_lock_the_founder_out() {
    // Twelve grants in one sitting share a second; the log orders them by
    // id. Whichever sorts first must not be the only one that took.
    let acts: Vec<Act> = ["human:a", "human:b", "human:c", "human:d"]
        .iter()
        .map(|who| grant(who, "house", None, 100))
        .collect();
    let canon = Log::from_acts(acts).derive();
    assert_eq!(canon.who_decides(&scope("house"), 200).len(), 4);
    assert!(canon.ungoverned.is_empty());
}

#[test]
fn an_act_that_took_while_open_is_recorded_as_such() {
    // A founding script: the founder's grant and an agent's self-grant share
    // a second. Both take — nothing predates them — and the record says WHY
    // each took, because a founder reading "applied" cannot otherwise tell a
    // seat from an open door. A second later the door is shut: a stranger's
    // grant is refused, and the founder's own is held, not open.
    let stranger = Act::new(
        ActKind::Grant {
            holder: "human:b".into(),
            scope: scope("house"),
            horizon: None,
            rationale: String::new(),
        },
        200,
        "human:stranger",
    );
    let founder_later = grant("human:c", "house", None, 300);
    let canon = Log::from_acts(vec![
        grant("human:sam", "house", None, 100),
        Act::new(
            ActKind::Grant {
                holder: "agent:helper".into(),
                scope: scope("house.kitchen"),
                horizon: None,
                rationale: String::new(),
            },
            100,
            "agent:helper",
        ),
        stranger.clone(),
        founder_later.clone(),
    ])
    .derive();

    assert_eq!(canon.bootstrap.len(), 2, "{:?}", canon.bootstrap);
    assert!(canon
        .bootstrap
        .iter()
        .all(|(_, why)| why.contains("predates this act")));
    assert_eq!(canon.ungoverned.len(), 1);
    assert_eq!(canon.ungoverned[0].0, stranger.id);
    assert!(
        !canon
            .bootstrap
            .iter()
            .any(|(id, _)| *id == founder_later.id),
        "a grant by a seated holder is held, not open"
    );
    assert_eq!(canon.who_decides(&scope("house"), 400).len(), 2);

    // The typed answer, directly.
    use crate::Seat;
    let house = scope("house");
    assert_eq!(canon.seat("human:sam", Some(&house), 100), Seat::Open);
    assert_eq!(canon.seat("human:sam", Some(&house), 150), Seat::Held);
    assert_eq!(
        canon.seat("human:stranger", Some(&house), 150),
        Seat::Lacking
    );
    // A corner nobody holds is open to somebody the canon has seated, and
    // not to a stranger.
    let attic = scope("attic");
    assert_eq!(canon.seat("agent:helper", Some(&attic), 150), Seat::Open);
    assert_eq!(
        canon.seat("human:stranger", Some(&attic), 150),
        Seat::Lacking
    );
}

#[test]
fn a_ruling_takes_standing_over_what_it_touches() {
    // The helper holds the kitchen. It dismisses a pair of HALL rules — outside
    // its seat. The act is on the record and flagged; the pair stays open;
    // Dana, who holds the house, carries it knowingly and that applies.
    let mut acts = governed_house();
    acts.push(grant("agent:helper", "house.kitchen", Some(100_000), 20));
    let hall = assert_at(
        "The hall stays clear enough for a stroller.",
        100,
        "human:sam",
    );
    let bikes = assert_at("Bikes live in the hall.", 100, "human:sam");
    acts.push(hall.clone());
    acts.push(bikes.clone());
    acts.push(Act::new(
        ActKind::Dismiss {
            a: hall.id.clone(),
            b: bikes.id.clone(),
            rationale: "these do not really conflict".into(),
        },
        200,
        "agent:helper",
    ));
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(
        canon.conflicts.is_empty(),
        "the helper's ruling did not take"
    );
    assert_eq!(canon.ungoverned.len(), 1);
    assert!(canon.ungoverned[0].1.contains("agent:helper ruled"));
    assert!(
        canon.unattended.len() == 1,
        "and it is still flagged as an agent's adjudication"
    );

    acts.push(Act::new(
        ActKind::Accept {
            a: hall.id.clone(),
            b: bikes.id.clone(),
            rationale: "nowhere else for the bikes".into(),
            revisit: None,
        },
        300,
        "human:theo",
    ));
    let canon = Log::from_acts(acts).derive();
    assert_eq!(
        canon.tolerated().count(),
        1,
        "a house holder's ruling applies"
    );
}

#[test]
fn you_may_withdraw_your_own_proposal_but_not_somebody_elses_rule() {
    let mut acts = governed_house();
    let mine = assert_at("Shoes off at the door.", 100, "human:stranger");
    let theirs = assert_at("Recycling out on Sunday night.", 100, "human:sam");
    acts.push(mine.clone());
    acts.push(theirs.clone());
    acts.push(Act::new(
        ActKind::Retract {
            target: mine.id.clone(),
            rationale: "never mind".into(),
        },
        200,
        "human:stranger",
    ));
    acts.push(Act::new(
        ActKind::Retract {
            target: theirs.id.clone(),
            rationale: "I disagree".into(),
        },
        200,
        "human:stranger",
    ));
    let canon = Log::from_acts(acts).derive();
    assert!(matches!(
        canon.get(&mine.id).unwrap().status,
        Status::Retracted { .. }
    ));
    assert!(matches!(
        canon.get(&theirs.id).unwrap().status,
        Status::Active
    ));
    // Not refused: put to the house as a proposal, and waiting on a holder.
    assert!(canon.ungoverned.is_empty());
    assert_eq!(canon.retractions.len(), 1);
    assert!(matches!(
        canon.retractions[0].verdict,
        crate::ratify::Verdict::Proposed { .. }
    ));
}

#[test]
fn a_retraction_without_standing_waits_for_someone_who_could_have_written_it() {
    // The agent may propose a rule; it may propose taking one away, and the
    // holder approves the act on the record rather than retyping it.
    let mut acts = governed_house();
    let rule = assert_at("Recycling out on Sunday night.", 100, "human:sam");
    let scoped = scoped_at(&rule.id, "house.kitchen", 100);
    let retract = Act::new(
        ActKind::Retract {
            target: rule.id.clone(),
            rationale: "duplicates the bins rule".into(),
        },
        200,
        "agent:claude",
    );
    acts.extend([rule.clone(), scoped, retract.clone()]);

    let waiting = Log::from_acts(acts.clone()).derive();
    assert!(matches!(
        waiting.get(&rule.id).unwrap().status,
        Status::Active
    ));
    let crate::ratify::Verdict::Proposed { needs } = &waiting.retractions[0].verdict else {
        panic!("a proposal until a holder speaks")
    };
    assert!(needs.contains("house.kitchen"), "{needs}");
    // A proposal, not an adjudication: the person who approves it is the
    // one ruling.
    assert!(!waiting.unattended.contains(&retract.id));

    // Another agent, or a person with no seat, agreeing changes nothing.
    let mut others = acts.clone();
    others.push(approve_at(&retract.id, 300, "agent:helper"));
    others.push(approve_at(&retract.id, 300, "human:stranger"));
    let still = Log::from_acts(others).derive();
    assert!(matches!(
        still.get(&rule.id).unwrap().status,
        Status::Active
    ));

    // Theo holds the house, which covers the kitchen: he could have
    // retracted it himself, so his approval is him doing it, from then.
    let mut approved = acts.clone();
    approved.push(approve_at(&retract.id, 300, "human:theo"));
    let done = Log::from_acts(approved).derive();
    assert!(matches!(
        done.get(&rule.id).unwrap().status,
        Status::Retracted { at: 300 }
    ));

    // A holder's reasoned objection first refuses it, and a later approval
    // does not undo that.
    let mut refused = acts;
    refused.push(object_at(
        &retract.id,
        300,
        "human:dana",
        "we still need it",
    ));
    refused.push(approve_at(&retract.id, 400, "human:theo"));
    let kept = Log::from_acts(refused).derive();
    assert!(matches!(kept.get(&rule.id).unwrap().status, Status::Active));
    assert!(matches!(
        kept.retractions[0].verdict,
        crate::ratify::Verdict::Refused { .. }
    ));
}

#[test]
fn an_agents_write_in_a_scope_nobody_holds_waits_for_a_person() {
    // Sam holds the house; nobody holds `garden`. A person writing there
    // is the bootstrap and takes; an agent's write is a proposal, because
    // agents never mint, and with nobody holding the scope any person's
    // word decides it.
    let mut acts = vec![grant("human:sam", "house", None, 10)];
    let by_agent = assert_at("Water the beds at dawn.", 100, "agent:claude");
    let by_person = assert_at("Compost goes in the left bin.", 100, "human:priya");
    acts.push(scoped_at(&by_agent.id, "garden", 100));
    acts.push(scoped_at(&by_person.id, "garden", 100));
    acts.extend([by_agent.clone(), by_person.clone()]);

    let canon = Log::from_acts(acts.clone()).derive();
    assert!(matches!(
        canon.get(&by_person.id).unwrap().status,
        Status::Active
    ));
    let Status::Proposed { needs } = &canon.get(&by_agent.id).unwrap().status else {
        panic!("an agent's write in an open scope is not law")
    };
    assert!(needs.contains("nobody holds garden"), "{needs}");

    // An agent's approval is not a person's.
    let mut agents = acts.clone();
    agents.push(approve_at(&by_agent.id, 200, "agent:helper"));
    let still = Log::from_acts(agents).derive();
    assert!(matches!(
        still.get(&by_agent.id).unwrap().status,
        Status::Proposed { .. }
    ));

    let mut approved = acts.clone();
    approved.push(approve_at(&by_agent.id, 200, "human:priya"));
    let done = Log::from_acts(approved).derive();
    assert!(matches!(
        done.get(&by_agent.id).unwrap().status,
        Status::Active
    ));

    // The same in a canon nobody has seated at all.
    let fresh = Log::from_acts(vec![by_agent.clone()]).derive();
    assert!(matches!(
        fresh.get(&by_agent.id).unwrap().status,
        Status::Proposed { .. }
    ));
}

#[test]
fn scoping_your_own_proposal_is_part_of_proposing_it() {
    // `add --scope` writes the assert and its scope as two acts. The scope
    // is the proposal's, not a ruling, so an agent's proposals a person has
    // approved leave nothing unattended — and scoping somebody else's rule
    // still is a ruling.
    let mut acts = vec![grant("human:sam", "house", None, 10)];
    let theirs = assert_at("Lights off when you leave.", 100, "human:sam");
    let mine = assert_at("Wipe the counter.", 100, "agent:claude");
    let own = Act::new(
        ActKind::Scoped {
            commitment: mine.id.clone(),
            scope: scope("house"),
        },
        100,
        "agent:claude",
    );
    let moved = Act::new(
        ActKind::Scoped {
            commitment: theirs.id.clone(),
            scope: scope("house"),
        },
        100,
        "agent:claude",
    );
    acts.extend([theirs, mine, own.clone(), moved.clone()]);
    let canon = Log::from_acts(acts).derive();
    assert!(!canon.unattended.contains(&own.id));
    assert!(canon.unattended.contains(&moved.id));
}

#[test]
fn the_canon_is_held_by_its_widest_held_scopes_and_says_so() {
    // There is no scope called the canon. An unscoped proposal waits on
    // whoever holds the widest scopes anyone holds, and its needs-line
    // names them so the reader knows what to grant.
    let acts = vec![
        grant("human:sam", "arch", None, 10),
        grant("human:sam", "procedure", None, 10),
        grant("human:dana", "arch.store", None, 10),
        assert_at("Prefer a test to a comment.", 100, "agent:claude"),
    ];
    let canon = Log::from_acts(acts).derive();
    let held: Vec<String> = canon
        .canon_scopes(100)
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(held, ["arch", "procedure"]);
    let Status::Proposed { needs } = &canon.proposed().next().unwrap().status else {
        unreachable!()
    };
    assert!(
        needs.contains("holds this canon by holding arch or procedure"),
        "{needs}"
    );
}

#[test]
fn a_stranger_cannot_tombstone_the_grants_that_shut_them_out() {
    // The back door under the constitutional level, found by walking the
    // three tiers with the CLI. Gating who may WRITE a grant while leaving
    // who may DELETE one open is not a gate: revert every grant and the canon
    // is ungoverned, and an ungoverned canon is open, so the refused
    // self-grant applies after all and the house belongs to whoever asked
    // last.
    let mut acts = governed_house();
    let seize = Act::new(
        ActKind::Grant {
            holder: "human:stranger".into(),
            scope: scope("house"),
            horizon: None,
            rationale: String::new(),
        },
        100,
        "human:stranger",
    );
    acts.push(seize.clone());
    for g in governed_house() {
        acts.push(Act::new(
            ActKind::Revert {
                targets: vec![g.id.clone()],
                rationale: "clearing the way".into(),
            },
            300,
            "human:stranger",
        ));
    }

    let canon = Log::from_acts(acts).derive();
    let holders = canon.who_decides(&scope("house"), 400);
    assert_eq!(holders.len(), 2, "the house still holds itself");
    assert!(holders.iter().all(|g| g.actor != "human:stranger"));
    assert_eq!(
        canon.ungoverned.len(),
        5,
        "the grant it could not write, and the four it could not delete"
    );
    assert!(canon
        .ungoverned
        .iter()
        .any(|(_, why)| why.contains("reverted")));
}

#[test]
fn your_own_revert_is_yours_the_house_may_undo_the_kitchen_and_not_the_reverse() {
    let dana = ratification_at(
        "joint:human:dana,human:sam",
        "house.kitchen",
        100,
        "human:dana",
    );

    // Hers to withdraw, like any other act she wrote.
    let mut acts = governed_house();
    acts.push(dana.clone());
    acts.push(Act::new(
        ActKind::Revert {
            targets: vec![dana.id.clone()],
            rationale: "too soon".into(),
        },
        200,
        "human:dana",
    ));
    let canon = Log::from_acts(acts).derive();
    assert!(canon.ratifications.is_empty());
    assert!(canon.ungoverned.is_empty());

    // Theo holds the house, which covers the kitchen: the level above may
    // undo how the level below decided to make rules.
    let mut acts = governed_house();
    acts.push(dana.clone());
    acts.push(Act::new(
        ActKind::Revert {
            targets: vec![dana.id.clone()],
            rationale: "the house takes this one back".into(),
        },
        200,
        "human:theo",
    ));
    let canon = Log::from_acts(acts).derive();
    assert!(canon.ratifications.is_empty());
    assert!(canon.ungoverned.is_empty());

    // And not the reverse. Dana holds only the kitchen.
    let mut acts = governed_house();
    let house_rule = ratification_at("threshold:2/1", "house", 100, "human:sam");
    acts.push(house_rule.clone());
    acts.push(Act::new(
        ActKind::Revert {
            targets: vec![house_rule.id.clone()],
            rationale: "I would rather it were consent".into(),
        },
        200,
        "human:dana",
    ));
    let canon = Log::from_acts(acts).derive();
    assert_eq!(
        canon.ratifications.len(),
        1,
        "the kitchen does not undo the house"
    );
    assert_eq!(canon.ungoverned.len(), 1);
}

// ── the rule of rules is itself a proposal ──────────────────

const DAY: i64 = 86_400;

fn verdict_of<'a>(canon: &'a Canon, act: &ActId) -> &'a crate::ratify::Verdict {
    &canon
        .ratifications
        .iter()
        .find(|r| r.act == *act)
        .expect("the change is on the record")
        .verdict
}

#[test]
fn a_house_holder_changing_the_kitchens_rule_proposes_and_one_cook_carries_it() {
    use crate::ratify::{Ratify, Verdict};
    // Theo holds the house, which covers the kitchen, so he MAY write the
    // kitchen's rule — and under the kitchen's own rule (standing) he is
    // not a cook, so what he wrote is a proposal until a cook says so.
    let mut acts = governed_house();
    let change = ratification_at(
        "joint:human:dana,human:sam",
        "house.kitchen",
        100,
        "human:theo",
    );
    acts.push(change.clone());
    let canon = Log::from_acts(acts.clone()).derive();
    let Verdict::Proposed { needs } = verdict_of(&canon, &change.id) else {
        panic!("a house holder proposes the kitchen's rule; the cooks ratify it")
    };
    assert!(needs.contains("holds house.kitchen"), "{needs}");
    assert_eq!(
        canon.ratification_for(Some(&scope("house.kitchen"))),
        &Ratify::Standing,
        "until then the kitchen decides the way it did"
    );
    assert!(canon.ungoverned.is_empty(), "this is not out of seat");

    // A pan rule written meanwhile is judged under standing, not joint.
    let pan = assert_at("Wash your own pan.", 150, "human:theo");
    acts.push(scoped_at(&pan.id, "house.kitchen", 150));
    acts.push(pan.clone());
    acts.push(approve_at(&change.id, 200, "human:dana"));
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(
        matches!(
            verdict_of(&canon, &change.id),
            Verdict::Ratified { since: 200, .. }
        ),
        "one cook's approval carries it, and it is in force from then"
    );
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("theo's pan rule is a proposal")
    };
    assert!(
        needs.contains("one person who holds"),
        "{needs}: standing, not joint"
    );

    // A pan rule written after the change needs both cooks.
    let later = assert_at("Dry it too.", 250, "human:theo");
    acts.push(scoped_at(&later.id, "house.kitchen", 250));
    acts.push(later.clone());
    let canon = Log::from_acts(acts).derive();
    let Status::Proposed { needs } = &canon.get(&later.id).unwrap().status else {
        panic!("still a proposal")
    };
    assert_eq!(needs, "approval from human:dana, human:sam");
}

#[test]
fn a_rule_change_is_judged_under_the_rule_it_is_changing() {
    use crate::ratify::{Ratify, Verdict};
    let mut acts = governed_house();
    let joint = ratification_at(
        "joint:human:dana,human:sam",
        "house.kitchen",
        30,
        "human:dana",
    );
    acts.push(joint.clone());
    // Sam holds the kitchen and may write its rule. Lowering the bar back
    // to standing is judged under joint: Dana has to agree.
    let lower = ratification_at("standing", "house.kitchen", 100, "human:sam");
    acts.push(lower.clone());
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(
        matches!(
            verdict_of(&canon, &joint.id),
            Verdict::Ratified { since: 30, .. }
        ),
        "a cook's change under standing lands at once, as it always did"
    );
    let Verdict::Proposed { needs } = verdict_of(&canon, &lower.id) else {
        panic!("nobody lowers the bar for their own corner alone")
    };
    assert_eq!(needs, "approval from human:dana");
    assert_eq!(
        canon.ratification_for(Some(&scope("house.kitchen"))).name(),
        "joint:human:dana,human:sam"
    );

    acts.push(approve_at(&lower.id, 200, "human:dana"));
    let canon = Log::from_acts(acts).derive();
    assert_eq!(
        canon.ratification_for(Some(&scope("house.kitchen"))),
        &Ratify::Standing
    );
}

#[test]
fn a_cook_objecting_refuses_a_rule_change_and_the_old_rule_stands() {
    use crate::ratify::Verdict;
    let mut acts = governed_house();
    acts.push(ratification_at(
        "joint:human:dana,human:sam",
        "house.kitchen",
        30,
        "human:dana",
    ));
    let lower = ratification_at("standing", "house.kitchen", 100, "human:sam");
    acts.push(lower.clone());
    acts.push(object_at(
        &lower.id,
        150,
        "human:dana",
        "we agreed both of us, always",
    ));
    let canon = Log::from_acts(acts).derive();
    let Verdict::Refused { by, why, .. } = verdict_of(&canon, &lower.id) else {
        panic!("one named cook objecting refuses it")
    };
    assert_eq!(by, "human:dana");
    assert!(why.contains("both of us"));
    assert_eq!(
        canon.ratification_for(Some(&scope("house.kitchen"))).name(),
        "joint:human:dana,human:sam"
    );
}

#[test]
fn a_consent_rule_change_governs_only_what_is_written_after_its_window_closes() {
    use crate::ratify::Verdict;
    let mut acts = governed_house();
    acts.push(ratification_at(
        "consent:7d",
        "house.kitchen",
        30,
        "human:dana",
    ));
    // Sam proposes going back to standing. Under consent it takes a week.
    let back = ratification_at("standing", "house.kitchen", 1_000, "human:sam");
    acts.push(back.clone());
    // Theo writes a rule three days in: consent still governs it, and the
    // window on his own rule starts from when he wrote it.
    let early = assert_at("Label your food.", 1_000 + 3 * DAY, "human:theo");
    acts.push(scoped_at(&early.id, "house.kitchen", 1_000 + 3 * DAY));
    acts.push(early.clone());
    // And one eight days in: standing by then, so it needs a cook.
    let late = assert_at("Rinse the sink.", 1_000 + 8 * DAY, "human:theo");
    acts.push(scoped_at(&late.id, "house.kitchen", 1_000 + 8 * DAY));
    acts.push(late.clone());

    let canon = Log::from_acts(acts.clone()).derive_at(1_000 + 3 * DAY + 1);
    assert!(matches!(
        verdict_of(&canon, &back.id),
        Verdict::Proposed { .. }
    ));

    let canon = Log::from_acts(acts).derive_at(1_000 + 8 * DAY + 1);
    let Verdict::Ratified { since, .. } = verdict_of(&canon, &back.id) else {
        panic!("a week of silence carries it")
    };
    assert_eq!(
        *since,
        1_000 + 7 * DAY,
        "in force from the end of the window"
    );
    let Status::Proposed { needs } = &canon.get(&early.id).unwrap().status else {
        panic!("written under consent, its own window is still open")
    };
    assert!(needs.starts_with("no objection"), "{needs}");
    let Status::Proposed { needs } = &canon.get(&late.id).unwrap().status else {
        panic!("written under standing, by a non-cook")
    };
    assert!(needs.contains("one person who holds"), "{needs}");
}

#[test]
fn an_objection_after_a_rule_is_settled_does_not_unmake_it() {
    let mut acts = governed_house();
    acts.push(ratification_at(
        "threshold:1/1",
        "house.kitchen",
        30,
        "human:dana",
    ));
    let pan = assert_at("Wash your own pan.", 100, "human:theo");
    acts.push(scoped_at(&pan.id, "house.kitchen", 100));
    acts.push(pan.clone());
    acts.push(approve_at(&pan.id, 200, "human:sam"));
    // Dana objects a hundred seconds too late. On the record; nothing moves.
    let mut late = acts.clone();
    late.push(object_at(&pan.id, 300, "human:dana", "I never liked it"));
    let canon = Log::from_acts(late).derive();
    assert!(
        matches!(canon.get(&pan.id).unwrap().status, Status::Active),
        "first word wins: otherwise any holder unmakes any settled rule at any time"
    );
    // The other order still refuses.
    let mut early = acts;
    early.push(object_at(&pan.id, 150, "human:dana", "I never liked it"));
    let canon = Log::from_acts(early).derive();
    assert!(matches!(
        canon.get(&pan.id).unwrap().status,
        Status::Refused { .. }
    ));
}

#[test]
fn twice_by_turnover_waits_for_someone_who_did_not_hold_the_kitchen_at_the_first_approval() {
    let mut acts = governed_house();
    acts.push(ratification_at(
        "twice:turnover:standing",
        "house.kitchen",
        30,
        "human:dana",
    ));
    let pan = assert_at("Wash your own pan.", 100, "human:theo");
    acts.push(scoped_at(&pan.id, "house.kitchen", 100));
    acts.push(pan.clone());
    let canon = Log::from_acts(acts.clone()).derive();
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("first vote not yet cast")
    };
    assert!(needs.starts_with("first of two:"), "{needs}");

    acts.push(approve_at(&pan.id, 200, "human:dana"));
    let canon = Log::from_acts(acts.clone()).derive();
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("carried once; now it waits for the kitchen to change hands")
    };
    assert!(needs.contains("did not hold house.kitchen"), "{needs}");

    // Dana approving again, before anyone new arrives, counts for nothing.
    acts.push(approve_at(&pan.id, 300, "human:dana"));
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(matches!(
        canon.get(&pan.id).unwrap().status,
        Status::Proposed { .. }
    ));

    // Rae joins the kitchen. That is the election.
    acts.push(grant("human:rae", "house.kitchen", None, 400));
    let canon = Log::from_acts(acts.clone()).derive_at(500);
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("the second vote is open, and nothing said before it counts")
    };
    assert!(needs.starts_with("a second time, since"), "{needs}");

    // Any cook's approval after that carries it — Rae's lever was to object.
    acts.push(approve_at(&pan.id, 600, "human:sam"));
    let canon = Log::from_acts(acts).derive_at(700);
    assert!(matches!(canon.get(&pan.id).unwrap().status, Status::Active));
    let v = canon.ratify(canon.get(&pan.id).unwrap(), 700);
    let crate::ratify::Verdict::Ratified { since, how } = v else {
        panic!()
    };
    assert_eq!(since, 600);
    assert!(how.contains("human:rae joined"), "{how}");
}

#[test]
fn renewing_your_own_standing_is_not_turnover_and_neither_is_a_lapse_nobody_fills() {
    let mut acts = vec![
        grant("human:sam", "house", None, 10),
        grant("human:dana", "house.kitchen", Some(1_000), 20),
        grant("human:sam", "house.kitchen", None, 20),
    ];
    acts.push(ratification_at(
        "twice:turnover:standing",
        "house.kitchen",
        30,
        "human:dana",
    ));
    let pan = assert_at("Wash your own pan.", 100, "human:theo");
    acts.push(scoped_at(&pan.id, "house.kitchen", 100));
    acts.push(pan.clone());
    acts.push(approve_at(&pan.id, 200, "human:dana"));
    // Sam renews himself; Dana's standing lapses at 1000 and nobody takes
    // her seat; Dana is granted the kitchen again at 1200. None of these
    // is somebody who did not hold the kitchen at the first vote.
    acts.push(grant("human:sam", "house.kitchen", None, 500));
    acts.push(grant("human:dana", "house.kitchen", None, 1_200));
    acts.push(approve_at(&pan.id, 1_300, "human:sam"));
    let canon = Log::from_acts(acts.clone()).derive_at(1_400);
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("the same coalition cannot elect itself")
    };
    assert!(needs.contains("did not hold house.kitchen"), "{needs}");

    // Somebody who last held it years ago and returns is new to THIS vote.
    acts.push(grant("human:theo", "house.kitchen", None, 1_500));
    acts.push(approve_at(&pan.id, 1_600, "human:sam"));
    let canon = Log::from_acts(acts).derive_at(1_700);
    assert!(matches!(canon.get(&pan.id).unwrap().status, Status::Active));
}

#[test]
fn twice_by_days_counts_only_approvals_written_after_the_boundary_and_no_implicit_one() {
    let mut acts = governed_house();
    acts.push(ratification_at(
        "twice:7d:standing",
        "house.kitchen",
        30,
        "human:dana",
    ));
    // Dana holds the kitchen: her own write is the first vote, at once.
    let pan = assert_at("Wash your own pan.", 100, "human:dana");
    acts.push(scoped_at(&pan.id, "house.kitchen", 100));
    acts.push(pan.clone());
    let canon = Log::from_acts(acts.clone()).derive_at(100 + 3 * DAY);
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("carried once; the week has not passed")
    };
    assert!(needs.starts_with("a second approval after"), "{needs}");

    // An approval inside the week does not count, and neither does having
    // written it: the second vote is cast, not implied.
    acts.push(approve_at(&pan.id, 100 + 3 * DAY, "human:dana"));
    let canon = Log::from_acts(acts.clone()).derive_at(100 + 8 * DAY);
    let Status::Proposed { needs } = &canon.get(&pan.id).unwrap().status else {
        panic!("still one vote short")
    };
    assert!(needs.starts_with("a second time, since"), "{needs}");

    acts.push(approve_at(&pan.id, 100 + 8 * DAY, "human:dana"));
    let canon = Log::from_acts(acts).derive_at(100 + 9 * DAY);
    assert!(matches!(canon.get(&pan.id).unwrap().status, Status::Active));
}

// ── seconds, citing, and the v4 line (format v4) ────────────

fn second_at(q: &ActId, ts: i64, actor: &str, pull: Pull) -> Act {
    Act::new(
        ActKind::Position {
            about: q.to_string(),
            citing: None,
            pull,
            because: "seconded".into(),
        },
        ts,
        actor,
    )
}

#[test]
fn a_second_on_a_question_is_not_unattended() {
    // Weight with no name on a question decides nothing, so reporting it as
    // an adjudication nobody owns would bury the real ones.
    let q = ask("should the sink get a second rack?", 100);
    let canon = Log::from_acts(vec![
        q.clone(),
        second_at(&q.id, 200, ANONYMOUS, Pull::Toward),
        second_at(&q.id, 300, ANONYMOUS, Pull::Toward),
    ])
    .derive();
    assert!(canon.unattended.is_empty(), "{:?}", canon.unattended);
    assert_eq!(canon.seconds(&q.id), 2);
}

#[test]
fn an_anonymous_position_on_a_proposal_is_still_unattended() {
    // On a commitment a position is a vote, and an unsigned vote is exactly
    // what the attribution check exists to name.
    let c = assert_c("pans out of the sink overnight", 100);
    let q = ask("a second rack?", 100);
    let on_rule = second_at(&c.id, 200, ANONYMOUS, Pull::Toward);
    // Against is an objection, not a second, even on a question.
    let against = second_at(&q.id, 200, ANONYMOUS, Pull::Against);
    let canon =
        Log::from_acts(vec![c.clone(), q.clone(), on_rule.clone(), against.clone()]).derive();
    assert!(canon.unattended.contains(&on_rule.id));
    assert!(canon.unattended.contains(&against.id));
    assert_eq!(canon.seconds(&c.id), 0, "a commitment has no seconds");
    assert_eq!(canon.seconds(&q.id), 0);
}

#[test]
fn an_agent_position_on_a_question_is_still_unattended() {
    // Only `anonymous` seconds. An agent's stance on a question is a stance.
    let q = ask("a second rack?", 100);
    let by_agent = second_at(&q.id, 200, "agent:helper", Pull::Toward);
    let canon = Log::from_acts(vec![q.clone(), by_agent.clone()]).derive();
    assert_eq!(canon.unattended, vec![by_agent.id]);
    assert_eq!(canon.seconds(&q.id), 0);
}

#[test]
fn a_second_never_counts_toward_ratification() {
    // Theo holds the house, not the kitchen, so his kitchen rule waits for a
    // cook. Anonymous weight — on the rule itself, or on a question about
    // it — never stands in for one.
    let mut acts = governed_house();
    let pan = assert_at("Wash your own pan.", 100, "human:theo");
    acts.push(pan.clone());
    acts.push(scoped_at(&pan.id, "house.kitchen", 100));
    let q = ask("should we make the pan rule?", 110);
    acts.push(q.clone());
    for i in 0..5 {
        acts.push(second_at(&q.id, 120 + i, ANONYMOUS, Pull::Toward));
        acts.push(second_at(&pan.id, 120 + i, ANONYMOUS, Pull::Toward));
    }
    let canon = Log::from_acts(acts.clone()).derive();
    assert_eq!(canon.seconds(&q.id), 5);
    assert!(
        matches!(canon.get(&pan.id).unwrap().status, Status::Proposed { .. }),
        "{:?}",
        canon.get(&pan.id).unwrap().status
    );
    acts.push(approve_at(&pan.id, 200, "human:dana"));
    let canon = Log::from_acts(acts).derive();
    assert!(matches!(canon.get(&pan.id).unwrap().status, Status::Active));
}

#[test]
fn decided_citing_reaches_the_ruling() {
    let c = assert_c("pans out of the sink overnight", 100);
    let missing = ActId::from_raw("can-000000000bad");
    let d = Act::new(
        ActKind::Decided {
            about: "pans".into(),
            outcome: crate::standing::Outcome::Conflicts,
            authority: crate::policy::Authority::AskOne,
            rationale: String::new(),
            citing: vec![c.id.clone(), missing.clone()],
        },
        200,
        "human:sam",
    );
    let canon = Log::from_acts(vec![c.clone(), d.clone()]).derive();
    assert_eq!(canon.rulings[0].citing, vec![c.id.clone(), missing.clone()]);
    assert_eq!(
        canon
            .ladder(Keyed::Cited(core::slice::from_ref(&c.id)), false)
            .len(),
        1
    );
    assert_eq!(
        canon.dangling,
        vec![(d.id, missing)],
        "a citation of nothing in this log is a hole, reported"
    );
}

#[test]
fn new_fields_write_v4_and_absent_fields_do_not() {
    let decided = |citing: Vec<ActId>| {
        Act::new(
            ActKind::Decided {
                about: "pans".into(),
                outcome: crate::standing::Outcome::Conflicts,
                authority: crate::policy::Authority::AskOne,
                rationale: String::new(),
                citing,
            },
            200,
            "human:sam",
        )
    };
    let graduated = |key, repair| ActKind::Policy {
        text: String::new(),
        rule: crate::policy::Rule::Graduated {
            ladder: vec![crate::policy::Authority::AskOne],
            base: alloc::boxed::Box::new(crate::policy::Rule::Default),
            key,
            repair,
        },
        scope: None,
    };
    let ratification = |governs, personal| ActKind::Ratification {
        text: String::new(),
        rule: crate::ratify::Ratify::Standing,
        scope: None,
        governs,
        personal,
    };
    let v = |kind: ActKind| Act::new(kind, 100, "human:sam").v;

    assert_eq!(decided(Vec::new()).v, UNSIGNED_VERSION);
    assert_eq!(decided(vec![ActId::from_raw("can-000000000001")]).v, 4);
    assert_eq!(v(graduated(None, None)), UNSIGNED_VERSION);
    assert_eq!(v(graduated(Some(crate::policy::LadderKey::Cited), None)), 4);
    assert_eq!(v(graduated(None, Some(false))), 4, "set, even to false");
    assert_eq!(v(ratification(None, None)), UNSIGNED_VERSION);
    assert_eq!(v(ratification(Some(crate::ratify::Governs::All), None)), 4);
    assert_eq!(v(ratification(None, Some(true))), 4);
    // A knob nested under another modifier still needs a v4 reader.
    assert_eq!(
        v(ActKind::Policy {
            text: String::new(),
            rule: crate::policy::Rule::Cautious {
                base: alloc::boxed::Box::new(crate::policy::Rule::Graduated {
                    ladder: vec![crate::policy::Authority::AskOne],
                    base: alloc::boxed::Box::new(crate::policy::Rule::Default),
                    key: None,
                    repair: Some(true),
                }),
            },
            scope: None,
        }),
        4
    );

    // Signing lifts a v2 body to v3 and no further; a v4 body stays v4.
    let mut plain = decided(Vec::new());
    crate::sign::prepare(&mut plain, &[]);
    assert_eq!(plain.v, SIGNED_VERSION);
    let mut knob = decided(vec![ActId::from_raw("can-000000000001")]);
    crate::sign::prepare(&mut knob, &[]);
    assert_eq!(knob.v, FORMAT_VERSION);

    // And a v4 line reads back as written.
    let line = serde_json::to_string(&knob).unwrap();
    assert!(line.contains(r#""v":4"#), "{line}");
    assert!(line.contains(r#""citing":["can-000000000001"]"#), "{line}");
    let back = Log::parse(&line).expect("a v4 line is readable");
    assert_eq!(back.acts()[0], knob);
}

// ── shares: a house counted as it stands when it is asked ─────────────

fn granted_by(holder: &str, s: &str, ts: i64, by: &str) -> Act {
    Act::new(
        ActKind::Grant {
            holder: holder.into(),
            scope: scope(s),
            horizon: None,
            rationale: String::new(),
        },
        ts,
        by,
    )
}

fn moves_out(holder: &str, s: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Withdraw {
            holder: holder.into(),
            scope: scope(s),
            rationale: "moving out".into(),
        },
        ts,
        holder,
    )
}

/// Ana founds the house with these people, all in one sitting, under `rule`.
fn a_house_of(people: &[&str], rule: &str) -> Vec<Act> {
    let mut acts: Vec<Act> = people
        .iter()
        .map(|p| granted_by(p, "house", 10, "human:ana"))
        .collect();
    acts.push(ratification_at(rule, "house", 20, "human:ana"));
    acts
}

/// A rule proposed into the house.
fn proposed(text: &str, ts: i64, actor: &str) -> (Act, Act) {
    let rule = assert_at(text, ts, actor);
    let into = scoped_at(&rule.id, "house", ts);
    (rule, into)
}

#[test]
fn a_unanimous_house_that_shrinks_still_decides_and_can_change_its_rule() {
    // Four agreed that everything takes all of them. Dee moves out. A count
    // of the house as it was would wait on Dee for ever, and the rule of
    // rules with it: nothing could pass, not even a change of how things pass.
    let mut acts = a_house_of(
        &["human:ana", "human:bo", "human:cy", "human:dee"],
        "unanimous",
    );
    acts.push(moves_out("human:dee", "house", 100));
    let (quiet, into) = proposed("Quiet after ten.", 200, "human:ana");
    acts.extend([quiet.clone(), into]);
    acts.push(approve_at(&quiet.id, 300, "human:bo"));
    acts.push(approve_at(&quiet.id, 300, "human:cy"));
    let looser = ratification_at("majority", "house", 400, "human:bo");
    acts.push(looser.clone());
    acts.push(approve_at(&looser.id, 500, "human:ana"));
    acts.push(approve_at(&looser.id, 500, "human:cy"));
    let canon = Log::from_acts(acts).derive();
    assert!(
        matches!(canon.get(&quiet.id).unwrap().status, Status::Active),
        "the three who live here agreed: {:?}",
        canon.get(&quiet.id).unwrap().status
    );
    let crate::ratify::Verdict::Ratified { how, .. } = verdict_of(&canon, &looser.id) else {
        panic!("the house as it is changed its own rule")
    };
    assert!(how.contains("all 3 people"), "{how}");
    assert_eq!(
        canon.ratification_for(Some(&scope("house"))).name(),
        "majority"
    );
}

#[test]
fn a_majority_is_of_the_house_when_the_proposal_was_written() {
    // Three chose "most of us". Two more move in; a proposal written after
    // takes three of the five, and two is not most of them.
    let mut acts = a_house_of(&["human:ana", "human:bo", "human:cy"], "majority");
    acts.push(granted_by("human:eve", "house", 50, "human:ana"));
    acts.push(granted_by("human:fay", "house", 50, "human:ana"));
    let (bikes, into) = proposed("Bikes in the shed.", 100, "human:bo");
    acts.extend([bikes.clone(), into]);
    acts.push(approve_at(&bikes.id, 200, "human:ana"));
    let canon = Log::from_acts(acts.clone()).derive();
    let Status::Proposed { needs } = &canon.get(&bikes.id).unwrap().status else {
        panic!(
            "two of five is not most: {:?}",
            canon.get(&bikes.id).unwrap().status
        )
    };
    assert_eq!(
        needs,
        "1 more approval(s) from any of human:cy, human:eve, human:fay — of the 5 people who \
         held house when it was proposed"
    );
    acts.push(approve_at(&bikes.id, 300, "human:eve"));
    let canon = Log::from_acts(acts).derive();
    assert!(
        matches!(canon.get(&bikes.id).unwrap().status, Status::Active),
        "three of five"
    );
}

#[test]
fn under_unanimous_one_reasoned_objection_refuses() {
    let mut acts = a_house_of(&["human:ana", "human:bo", "human:cy"], "unanimous");
    let (pets, into) = proposed("A cat may live here.", 100, "human:ana");
    acts.extend([pets.clone(), into]);
    acts.push(approve_at(&pets.id, 200, "human:bo"));
    acts.push(object_at(&pets.id, 250, "human:cy", "I'm allergic"));
    acts.push(approve_at(&pets.id, 300, "human:cy"));
    let canon = Log::from_acts(acts).derive();
    let Status::Refused { by, why, at } = &canon.get(&pets.id).unwrap().status else {
        panic!("one objection refuses it, and a yes after it does not unmake that")
    };
    assert_eq!(
        (by.as_str(), why.as_str(), *at),
        ("human:cy", "I'm allergic", 250)
    );
}

#[test]
fn under_unanimous_someone_moving_out_before_they_approve_refuses_it() {
    // Waiting on someone who has gone is the deadlock this rule exists to
    // avoid: refused, said, and free to be proposed again to the house as it is.
    let mut acts = a_house_of(&["human:ana", "human:bo", "human:cy"], "unanimous");
    let (paint, into) = proposed("Paint the hall green.", 100, "human:ana");
    acts.extend([paint.clone(), into]);
    acts.push(approve_at(&paint.id, 150, "human:bo"));
    acts.push(moves_out("human:cy", "house", 200));
    let canon = Log::from_acts(acts).derive();
    let Status::Refused { by, why, at } = &canon.get(&paint.id).unwrap().status else {
        panic!(
            "cy is gone, so everyone can no longer agree: {:?}",
            canon.get(&paint.id).unwrap().status
        )
    };
    assert_eq!((by.as_str(), *at), ("", 200));
    assert!(
        why.contains("human:cy") && why.contains("no longer holds house"),
        "{why}"
    );
}

#[test]
fn somebody_who_joins_later_does_not_count_toward_what_was_already_asked() {
    let mut acts = a_house_of(&["human:ana", "human:bo", "human:cy"], "majority");
    let (rota, into) = proposed("A cleaning rota.", 100, "human:ana");
    acts.extend([rota.clone(), into]);
    acts.push(granted_by("human:eve", "house", 150, "human:ana"));
    acts.push(approve_at(&rota.id, 200, "human:eve"));
    let canon = Log::from_acts(acts.clone()).derive();
    let Status::Proposed { needs } = &canon.get(&rota.id).unwrap().status else {
        panic!(
            "eve was not here when it was asked: {:?}",
            canon.get(&rota.id).unwrap().status
        )
    };
    assert!(
        needs.starts_with("1 more approval(s) from human:bo or human:cy"),
        "{needs}"
    );
}

#[test]
fn a_share_rule_is_a_rule_kind_and_an_older_reader_refuses_it_by_name() {
    // SPEC 6: a new rule kind is announced in the list, not versioned. Its
    // line is written at the version its envelope needs, and a reader that
    // does not know the kind refuses the line and says which, rather than
    // fold the house under a rule of rules it cannot read.
    let unanimous = ratification_at("unanimous", "house", 20, "human:ana");
    assert_eq!(unanimous.v, UNSIGNED_VERSION);
    let line = serde_json::to_string(&unanimous).unwrap();
    assert!(line.contains(r#""rule":{"rule":"unanimous"}"#), "{line}");
    assert_eq!(Log::parse(&line).expect("readable").acts()[0], unanimous);
    let unknown = line.replace(r#""rule":"unanimous""#, r#""rule":"plurality""#);
    let Err(ParseError::Malformed { detail, .. }) = Log::parse(&unknown) else {
        panic!("an unknown rule is refused, not carried")
    };
    assert!(detail.contains("plurality"), "{detail}");
}

// ── share: any bar, counted as majority is ────────────────────────────

/// A position against with no reason given.
fn against_at(target: &ActId, ts: i64, actor: &str) -> Act {
    object_at(target, ts, actor, "")
}

fn status_of<'a>(canon: &'a Canon, id: &ActId) -> &'a Status {
    &canon.get(id).expect("on the record").status
}

#[test]
fn a_rule_0_4_0_wrote_reads_as_it_did() {
    // Minted by the released canon 0.4.0: `canon ratification set majority
    // --scope house`, and `unanimous` over house.kitchen. Houses have
    // adopted these since that release, so they read as the same rule, the
    // same id and the same bytes for ever.
    for (line, rule) in [
        (
            r#"{"id":"can-6efb9b0f5d07","v":4,"ts_unix":1792269653,"actor":"human:ana","op":"ratification","text":"more than half of us","rule":{"rule":"majority"},"scope":"house","governs":"all"}"#,
            crate::ratify::Ratify::Majority,
        ),
        (
            r#"{"id":"can-a633dcbfe43e","v":4,"ts_unix":1792269653,"actor":"human:ana","op":"ratification","text":"all of us","rule":{"rule":"unanimous"},"scope":"house.kitchen","governs":"all"}"#,
            crate::ratify::Ratify::Unanimous,
        ),
    ] {
        let log = Log::parse(line).expect("0.4.0's line reads");
        let act = &log.acts()[0];
        let ActKind::Ratification { rule: read, .. } = &act.kind else {
            panic!("a ratification act: {act:?}")
        };
        assert_eq!(read, &rule);
        assert_eq!(log.render().trim_end(), line, "byte for byte");
        assert_eq!(
            Act::new(act.kind.clone(), act.ts_unix, &act.actor).id,
            act.id
        );
    }
}

#[test]
fn a_share_is_written_as_its_bar_and_a_bar_no_house_could_use_is_refused() {
    let sixty = ratification_at("share:60%", "house", 20, "human:ana");
    assert_eq!(sixty.v, UNSIGNED_VERSION);
    let line = serde_json::to_string(&sixty).unwrap();
    assert!(
        line.contains(r#""rule":{"rule":"share","numerator":3,"denominator":5,"more_than":false}"#),
        "{line}"
    );
    assert_eq!(Log::parse(&line).expect("readable").acts()[0], sixty);
    let over =
        serde_json::to_string(&ratification_at("share:>2/3", "house", 20, "human:ana")).unwrap();
    assert!(
        over.contains(r#""rule":{"rule":"share","numerator":2,"denominator":3,"more_than":true}"#),
        "{over}"
    );

    // The same bar written another way reads as written, so the line keeps
    // the bytes it was signed as, and takes the same people.
    let tenths = line.replace(
        r#""numerator":3,"denominator":5"#,
        r#""numerator":6,"denominator":10"#,
    );
    let log = Log::parse(&tenths).expect("6/10 is a share");
    assert_eq!(log.render().trim_end(), tenths);
    let ActKind::Ratification { rule, .. } = &log.acts()[0].kind else {
        panic!("a ratification act")
    };
    assert_eq!(rule.name(), "share:60%");
    assert_eq!(rule.bar().map(|b| b.needed(30)), Some(18));

    // A bar nobody could reach, or nobody needs to, refuses the line as a
    // malformed known op, saying why, rather than fold under it.
    for (from, to, says) in [
        (r#""denominator":5"#, r#""denominator":0"#, "can't be 0"),
        (
            r#""numerator":3"#,
            r#""numerator":0"#,
            "before anyone approved",
        ),
        (r#""numerator":3"#, r#""numerator":7"#, "more than everyone"),
        (
            r#""numerator":3,"denominator":5,"more_than":false"#,
            r#""numerator":1,"denominator":1,"more_than":true"#,
            "more than everyone",
        ),
        (r#","more_than":false"#, "", "more_than"),
    ] {
        let bad = line.replace(from, to);
        assert_ne!(bad, line, "{from}");
        let Err(ParseError::Malformed { detail, .. }) = Log::parse(&bad) else {
            panic!("refused: {bad}")
        };
        assert!(detail.contains(says), "{detail}");
    }
}

#[test]
fn five_people_under_sixty_percent_carry_at_three_and_refuse_as_a_house() {
    let five = [
        "human:ana",
        "human:bo",
        "human:cy",
        "human:dee",
        "human:eve",
    ];
    let mut acts = a_house_of(&five, "share:60%");
    // Bo's own write is his yes; three of five is 60%.
    let (shed, into) = proposed("Bikes in the shed.", 100, "human:bo");
    acts.extend([shed.clone(), into]);
    acts.push(approve_at(&shed.id, 200, "human:cy"));
    let canon = Log::from_acts(acts.clone()).derive();
    assert_eq!(
        status_of(&canon, &shed.id),
        &Status::Proposed {
            needs: "1 more approval(s) from any of human:ana, human:dee, human:eve — of the 5 \
                    people who held house when it was proposed"
                .into()
        }
    );
    acts.push(approve_at(&shed.id, 300, "human:dee"));

    // Three can no longer approve: two said why, one just said no.
    let (quiet, into) = proposed("Quiet after nine.", 100, "human:bo");
    acts.extend([quiet.clone(), into]);
    acts.push(object_at(
        &quiet.id,
        200,
        "human:cy",
        "Late cooks need the evening.",
    ));
    acts.push(object_at(&quiet.id, 210, "human:dee", "Not in summer."));
    acts.push(against_at(&quiet.id, 220, "human:eve"));

    // Two said no, and Eve moving out tips it.
    let (paint, into) = proposed("Paint the hall green.", 100, "human:bo");
    acts.extend([paint.clone(), into]);
    acts.push(object_at(&paint.id, 200, "human:cy", "Too dark."));
    acts.push(object_at(&paint.id, 210, "human:dee", "Not in summer."));
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(
        matches!(status_of(&canon, &paint.id), Status::Proposed { .. }),
        "three of the five can still approve: {:?}",
        status_of(&canon, &paint.id)
    );
    acts.push(moves_out("human:eve", "house", 400));

    let canon = Log::from_acts(acts).derive();
    assert_eq!(status_of(&canon, &shed.id), &Status::Active);
    assert_eq!(
        canon.ratify(canon.get(&shed.id).unwrap(), i64::MAX),
        crate::ratify::Verdict::Ratified {
            since: 300,
            how: "3 of the 5 people who held house when it was proposed approved".into()
        }
    );
    assert_eq!(
        status_of(&canon, &quiet.id),
        &Status::Refused {
            at: 220,
            by: String::new(),
            why: "3 of the 5 people who held house when it was proposed said no or no longer \
                  hold it, so 60% can no longer approve it — human:cy: Late cooks need the \
                  evening.; human:dee: Not in summer."
                .into()
        }
    );
    assert_eq!(
        status_of(&canon, &paint.id),
        &Status::Refused {
            at: 400,
            by: String::new(),
            why: "3 of the 5 people who held house when it was proposed said no or no longer \
                  hold it, so 60% can no longer approve it — human:cy: Too dark.; human:dee: \
                  Not in summer."
                .into()
        }
    );
}

#[test]
fn thirty_people_under_sixty_percent_carry_at_eighteen() {
    let names: Vec<String> = (1..=30).map(|i| format!("human:p{i:02}")).collect();
    let thirty: Vec<&str> = names.iter().map(String::as_str).collect();
    // P01 founds this house, as Ana founds the others.
    let mut acts: Vec<Act> = thirty
        .iter()
        .map(|p| granted_by(p, "house", 10, "human:p01"))
        .collect();
    acts.push(ratification_at("share:60%", "house", 20, "human:p01"));

    let (carry, into) = proposed("A shared tool shelf.", 100, "human:p01");
    acts.extend([carry.clone(), into]);
    for p in &thirty[1..17] {
        acts.push(approve_at(&carry.id, 200, p));
    }
    let canon = Log::from_acts(acts.clone()).derive();
    assert_eq!(
        status_of(&canon, &carry.id),
        &Status::Proposed {
            needs: "1 more approval(s) from any of human:p18, human:p19, human:p20 and 10 \
                    other(s) — of the 30 people who held house when it was proposed"
                .into()
        },
        "seventeen of thirty is not 60%"
    );
    acts.push(approve_at(&carry.id, 300, "human:p18"));

    // Twelve say no and eighteen can still carry it; the thirteenth refuses.
    let (late, into) = proposed("Breakfast ends at nine.", 100, "human:p01");
    acts.extend([late.clone(), into]);
    acts.push(object_at(
        &late.id,
        200,
        "human:p02",
        "Late cooks need the morning.",
    ));
    for p in &thirty[2..13] {
        acts.push(against_at(&late.id, 200, p));
    }
    let canon = Log::from_acts(acts.clone()).derive();
    assert!(matches!(
        status_of(&canon, &late.id),
        Status::Proposed { .. }
    ));
    acts.push(object_at(&late.id, 300, "human:p14", "Not in summer."));

    // Twelve say no, and the thirtieth moving out tips it.
    let (pool, into) = proposed("A paddling pool in the yard.", 100, "human:p01");
    acts.extend([pool.clone(), into]);
    acts.push(object_at(&pool.id, 200, "human:p02", "Not in summer."));
    for p in &thirty[2..13] {
        acts.push(against_at(&pool.id, 200, p));
    }
    acts.push(moves_out("human:p30", "house", 400));

    let canon = Log::from_acts(acts).derive();
    assert_eq!(status_of(&canon, &carry.id), &Status::Active);
    assert_eq!(
        status_of(&canon, &late.id),
        &Status::Refused {
            at: 300,
            by: String::new(),
            why: "13 of the 30 people who held house when it was proposed said no or no longer \
                  hold it, so 60% can no longer approve it — human:p02: Late cooks need the \
                  morning.; human:p14: Not in summer."
                .into()
        }
    );
    assert_eq!(
        status_of(&canon, &pool.id),
        &Status::Refused {
            at: 400,
            by: String::new(),
            why: "13 of the 30 people who held house when it was proposed said no or no longer \
                  hold it, so 60% can no longer approve it — human:p02: Not in summer."
                .into()
        }
    );
}

#[test]
fn under_majority_nobody_refuses_alone() {
    // 0.4.0 credited this refusal to Cy, whose no came last: "refused by
    // human:cy: Too loud." Two of three said no, and both are named.
    let mut acts = a_house_of(&["human:ana", "human:bo", "human:cy"], "majority");
    let (band, into) = proposed("Band practice on Sundays.", 100, "human:ana");
    acts.extend([band.clone(), into]);
    acts.push(object_at(
        &band.id,
        200,
        "human:bo",
        "Sunday is for sleeping.",
    ));
    acts.push(object_at(&band.id, 210, "human:cy", "Too loud."));
    let canon = Log::from_acts(acts).derive();
    assert_eq!(
        status_of(&canon, &band.id),
        &Status::Refused {
            at: 210,
            by: String::new(),
            why: "2 of the 3 people who held house when it was proposed said no or no longer \
                  hold it, so more than half can no longer approve it — human:bo: Sunday is \
                  for sleeping.; human:cy: Too loud."
                .into()
        }
    );
}

#[test]
fn only_a_bar_of_everyone_waits_for_a_reason() {
    // A majority of two takes both of them, as everyone does; it is still
    // a majority, and a no without a reason counts, as it did in 0.4.0.
    for (rule, refuses) in [
        ("majority", true),
        ("share:100%", false),
        ("unanimous", false),
    ] {
        let mut acts = a_house_of(&["human:ana", "human:bo"], rule);
        let (gate, into) = proposed("Lock the gate at night.", 100, "human:ana");
        acts.extend([gate.clone(), into]);
        acts.push(against_at(&gate.id, 200, "human:bo"));
        let canon = Log::from_acts(acts.clone()).derive();
        match status_of(&canon, &gate.id) {
            Status::Refused { at, by, why } if refuses => {
                assert_eq!((*at, by.as_str()), (200, ""));
                assert_eq!(
                    why,
                    "1 of the 2 people who held house when it was proposed said no or no \
                     longer holds it, so more than half can no longer approve it"
                );
            }
            Status::Proposed { .. } if !refuses => {}
            other => panic!("{rule}: {other:?}"),
        }
        if !refuses {
            // Everyone: one reason decides, and it is that person's.
            acts.push(object_at(&gate.id, 300, "human:bo", "The night shift."));
            let canon = Log::from_acts(acts).derive();
            assert_eq!(
                status_of(&canon, &gate.id),
                &Status::Refused {
                    at: 300,
                    by: "human:bo".into(),
                    why: "The night shift.".into()
                },
                "{rule}"
            );
        }
    }
}

#[test]
fn a_share_of_the_whole_canon_names_the_scope_its_people_held() {
    let mut acts: Vec<Act> = ["human:ana", "human:bo", "human:cy"]
        .iter()
        .map(|p| granted_by(p, "house", 10, "human:ana"))
        .collect();
    acts.push(Act::new(
        ActKind::Ratification {
            text: String::new(),
            rule: crate::ratify::Ratify::parse("share:2/3").unwrap(),
            scope: None,
            governs: None,
            personal: None,
        },
        20,
        "human:ana",
    ));
    // An import keeps no scope; the canon-wide rule decides it.
    let kept = assert_at("Shoes off at the door.", 100, "human:ana");
    acts.push(kept.clone());
    acts.push(against_at(&kept.id, 200, "human:bo"));
    acts.push(object_at(
        &kept.id,
        210,
        "human:cy",
        "Not with these floors.",
    ));
    let canon = Log::from_acts(acts).derive();
    assert_eq!(
        status_of(&canon, &kept.id),
        &Status::Refused {
            at: 210,
            by: String::new(),
            why: "2 of the 3 people who held house when it was proposed said no or no longer \
                  hold it, so 2/3 can no longer approve it — human:cy: Not with these floors."
                .into()
        },
        "two-thirds of three is two; one no leaves two who can, and a second no ends it"
    );
}

#[test]
fn twice_under_a_share_counts_the_house_at_each_vote() {
    // 60% of the five who held house at the first vote, then 60% of the six
    // who held it when the second opened: four of them.
    let five = [
        "human:ana",
        "human:bo",
        "human:cy",
        "human:dee",
        "human:eve",
    ];
    let mut acts = a_house_of(&five, "twice:1d:share:60%");
    let (oven, into) = proposed("Clean the oven monthly.", 100, "human:ana");
    acts.extend([oven.clone(), into]);
    acts.push(approve_at(&oven.id, 200, "human:bo"));
    acts.push(approve_at(&oven.id, 200, "human:cy"));
    acts.push(granted_by("human:fay", "house", 50_000, "human:ana"));
    for p in ["human:bo", "human:cy", "human:fay"] {
        acts.push(approve_at(&oven.id, 90_000, p));
    }
    let canon = Log::from_acts(acts.clone()).derive_at(100_000);
    let Status::Proposed { needs } = status_of(&canon, &oven.id) else {
        panic!("three of six is not 60%: {:?}", status_of(&canon, &oven.id))
    };
    assert!(
        needs.contains("1 more approval(s) from any of human:ana, human:dee, human:eve"),
        "{needs}"
    );
    assert!(
        needs.ends_with("of the 6 people who held house when it was proposed"),
        "{needs}"
    );
    acts.push(approve_at(&oven.id, 90_100, "human:dee"));
    let canon = Log::from_acts(acts).derive_at(100_000);
    let crate::ratify::Verdict::Ratified { since, how } =
        canon.ratify(canon.get(&oven.id).unwrap(), 100_000)
    else {
        panic!("four of six carried the second vote")
    };
    assert_eq!(since, 90_100);
    assert!(
        how.starts_with("3 of the 5 people") && how.contains("4 of the 6 people"),
        "{how}"
    );
}
