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
    // v2 widened what ops mean, not what versions mean.
    let ahead = line("assert", r#","text":"x""#).replace(r#""v":2"#, r#""v":3"#);
    assert!(matches!(
        Log::parse(&ahead),
        Err(ParseError::UnknownVersion { found: 3, .. })
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
    let canon = Log::from_acts(vec![
        grant("human:dana", "house.kitchen", Some(200), 100),
        grant("human:dana", "house.kitchen", Some(900), 300),
    ])
    .derive();
    assert_eq!(canon.grants.len(), 2, "both are facts that happened");
    let live: Vec<_> = canon.grants.iter().filter(|g| g.held_at(500)).collect();
    assert_eq!(live.len(), 1, "but only one is held at any instant");
    assert_eq!(live[0].horizon, Some(900), "the renewal wins");
    assert!(canon.standing_of("human:dana", &scope("house.kitchen"), 500));
    // And the old term is still answerable about its own window.
    assert!(canon.grants[0].held_at(150));
    assert!(!canon.grants[0].held_at(500));
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
        ActKind::Rank {
            commitment: id,
            rank: "principle".into(),
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
