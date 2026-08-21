// SPDX-License-Identifier: AGPL-3.0-or-later
//! Rebase tests. The invariants here are about what the proposal REFUSES to
//! offer: a conflict never comes with a runnable command, and a mapping that
//! points nowhere is dropped rather than aimed at a neighbouring rule.

use canon_core::{Act, ActKind, Log, SnapshotCommitment};
use serde_json::json;

use super::*;
use crate::testing::{completion, Mock};

fn snap(lineage: &str, generation_of: &[(&str, &str)]) -> Snapshot {
    let commitments: Vec<SnapshotCommitment> = generation_of
        .iter()
        .map(|(id, text)| SnapshotCommitment {
            id: canon_core::ActId::from_raw(*id),
            text: (*text).to_string(),
        })
        .collect();
    Snapshot {
        lineage: lineage.into(),
        generation: canon_core::lineage::generation_of(&commitments),
        profile: "house".into(),
        at: 0,
        commitments,
    }
}

fn seed_and_canon() -> (Snapshot, Canon) {
    // The seed, and a canon that inherited it and then changed two things.
    let up_quiet = canon_core::ActId::from_raw("can-up0000000001");
    let up_guest = canon_core::ActId::from_raw("can-up0000000002");
    let seed = snap(
        "house-5",
        &[
            ("can-up0000000001", "Quiet hours run 11pm-7am."),
            ("can-up0000000002", "A guest may stay 2 nights in any 7."),
        ],
    );
    let iq = Act::new(
        ActKind::Assert {
            text: "Quiet hours run 11pm-7am.".into(),
            from: Some(up_quiet),
            source: None,
        },
        100,
        "human:priya",
    );
    let ig = Act::new(
        ActKind::Assert {
            text: "A guest may stay 2 nights in any 7.".into(),
            from: Some(up_guest),
            source: None,
        },
        101,
        "human:priya",
    );
    let mine = Act::new(
        ActKind::Assert {
            text: "Thursday is movie night until midnight.".into(),
            from: None,
            source: None,
        },
        102,
        "human:priya",
    );
    let sup = Act::new(
        ActKind::Supersede {
            text: "Quiet hours run 10pm-7am on weeknights.".into(),
            old: vec![iq.id.clone()],
            rationale: "night shifts".into(),
        },
        200,
        "human:priya",
    );
    (seed, Log::from_acts(vec![iq, ig, mine, sup]).derive())
}

fn target() -> Snapshot {
    snap(
        "house-5",
        &[
            ("can-tg0000000001", "Quiet hours run 11pm-7am, every night."),
            ("can-tg0000000002", "Bikes live on the porch."),
        ],
    )
}

fn mapping(items: &[(usize, &str, Option<usize>, &str)]) -> String {
    completion(
        &json!({
            "changes": items
                .iter()
                .map(|(c, f, t, b)| json!({ "change": c, "fate": f, "target": t, "because": b }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

#[test]
fn a_divergence_becomes_one_change_per_thing_the_holder_actually_did() {
    let (seed, canon) = seed_and_canon();
    let d = Divergence::compute(&seed, &canon);
    let changes = describe(&d, &canon);
    assert_eq!(changes.len(), 2, "{changes:#?}");
    assert!(changes[0].did.contains("replaced"));
    assert!(changes[1].did.contains("added"));
    // The addition needs no target id, so it carries a runnable command on
    // its own; the supersession's command has a hole for one.
    assert!(changes[1].standalone.is_some());
    assert!(changes[0].command.as_ref().unwrap().contains("{}"));
}

#[test]
fn a_conflict_is_marked_and_gets_no_command() {
    // The invariant. A runnable line next to a conflict invites resolving it
    // by reflex, which is the one thing a rebase must not encourage.
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[
            (
                1,
                "conflicts",
                Some(1),
                "the new base sets quiet hours every night",
            ),
            (
                2,
                "carries",
                Some(2),
                "nothing in the new base covers movie night",
            ),
        ]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    let text = render(&seed, &t, &changes, &landed);

    let conflict_block = text
        .split("\n\n")
        .find(|b| b.contains("CONFLICTS"))
        .expect("a conflict block");
    assert!(
        !conflict_block.contains("canon supersede"),
        "{conflict_block}"
    );
    assert!(conflict_block.contains("decide this one yourself"));
    assert!(text.contains("1 of 2 of your changes carry"));
    assert!(text.contains("Nothing has been written"));
}

#[test]
fn a_carried_change_names_the_exact_command_against_the_new_base() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[(1, "carries", Some(1), "same rule, reworded")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    let text = render(&seed, &t, &changes, &landed);
    assert!(
        text.contains(
            "canon supersede can-tg0000000001 \"Quiet hours run 10pm-7am on weeknights.\""
        ),
        "{text}"
    );
}

#[test]
fn a_mapping_pointing_at_a_rule_that_does_not_exist_is_dropped_not_aimed_nearby() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[(1, "carries", Some(99), "somewhere")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    assert_eq!(landed.len(), 1);
    assert!(
        landed[0].target.is_none(),
        "an out-of-range target is not clamped"
    );
    // With no target it cannot print a command, so nothing runnable is offered.
    let text = render(&seed, &t, &changes, &landed);
    assert!(!text.contains("canon supersede can-"), "{text}");
}

#[test]
fn a_change_the_model_said_nothing_about_is_reported_unmapped_not_counted_as_passing() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(200, mapping(&[(1, "carries", Some(1), "ok")]))]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    let text = render(&seed, &t, &changes, &landed);
    assert!(text.contains("1 unmapped"), "{text}");
}

#[test]
fn an_unreadable_fate_is_dropped_rather_than_guessed() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(200, mapping(&[(1, "probably fine", Some(1), "hm")]))]);
    assert!(map_changes(&mock.client(), &changes, &seed, &t)
        .unwrap()
        .is_empty());
}

#[test]
fn an_added_rule_can_never_come_back_orphaned() {
    // Structural, not prompt-trusted: an addition had no earlier rule for a
    // change to be "about", so `orphaned` is not a fate it can have. Observed
    // for real against a local endpoint, which read the fate definitions
    // literally and orphaned a rule the holder had just written.
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    assert_eq!(changes[1].kind, Kind::Addition);
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[(2, "orphaned", None, "the target has no movie night rule")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    assert_eq!(landed[0].verdict, Verdict::Carries);
    // And it comes out with a command, because a rule you wrote carries by
    // default onto a base that does not mention it.
    let text = render(&seed, &t, &changes, &landed);
    assert!(
        text.contains("canon add \"Thursday is movie night until midnight.\""),
        "{text}"
    );
}

#[test]
fn an_edit_can_still_be_orphaned() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    assert_eq!(changes[0].kind, Kind::Edit);
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[(1, "orphaned", None, "the rule is gone from the new base")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    assert_eq!(landed[0].verdict, Verdict::Orphaned);
}
