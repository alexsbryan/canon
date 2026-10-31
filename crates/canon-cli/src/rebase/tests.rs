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
                3,
                "conflicts",
                Some(1),
                "the new base sets quiet hours every night",
            ),
            (
                4,
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
    assert!(!conflict_block.contains("canon amend"), "{conflict_block}");
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
        mapping(&[(3, "carries", Some(1), "same rule, reworded")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    let text = render(&seed, &t, &changes, &landed);
    assert!(
        text.contains("canon amend can-tg0000000001 \"Quiet hours run 10pm-7am on weeknights.\""),
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
        mapping(&[(3, "carries", Some(99), "somewhere")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    assert_eq!(landed.len(), 1);
    assert!(
        landed[0].target.is_none(),
        "an out-of-range target is not clamped"
    );
    // With no target it cannot print a command, so nothing runnable is offered.
    let text = render(&seed, &t, &changes, &landed);
    assert!(!text.contains("canon amend can-"), "{text}");
}

#[test]
fn a_change_the_model_said_nothing_about_is_reported_unmapped_not_counted_as_passing() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(200, mapping(&[(3, "carries", Some(1), "ok")]))]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    let text = render(&seed, &t, &changes, &landed);
    assert!(text.contains("1 unmapped"), "{text}");
}

#[test]
fn an_unreadable_fate_is_dropped_rather_than_guessed() {
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(200, mapping(&[(3, "probably fine", Some(1), "hm")]))]);
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
        mapping(&[(4, "orphaned", None, "the target has no movie night rule")]),
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
        mapping(&[(3, "orphaned", None, "the rule is gone from the new base")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    assert_eq!(landed[0].verdict, Verdict::Orphaned);
}

// ── shaped like the real move: a fork of commonwealth-ai onto a family canon
//
// The family canon is itself a paste-adoption of the same lineage, so its
// rules carry the text and the upstream ids the fork inherited, under ids of
// its own. On 2026-10-23 the model judged retractions of rules the family
// canon holds word for word "orphaned", and applying that verdict drops the
// retraction.

const PRE_PUSH: &str = "A feature that has not passed the pre-push gate is not done.";
const TWELVE: &str = "Hold all twelve principles.";
const ATLAS: &str = "The work atlas gives a view of what other agents are doing.";
const GUARD: &str = "A guard ships with its remedy.";
const MINE: &str = "Ersilia measures and recommends; a person enacts.";

fn asserted(text: &str, from: Option<&str>, ts: i64) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: from.map(canon_core::ActId::from_raw),
            source: None,
        },
        ts,
        "human:sam",
    )
}

fn retracted(target: &Act, ts: i64) -> Act {
    Act::new(
        ActKind::Retract {
            target: target.id.clone(),
            rationale: "not ours".into(),
        },
        ts,
        "human:sam",
    )
}

/// The fork's seed, the fork, and the family canon's log and snapshot.
fn family_case() -> (Snapshot, Canon, Snapshot, Canon) {
    let seed = snap(
        "commonwealth-ai",
        &[
            ("can-cw0000000001", PRE_PUSH),
            ("can-cw0000000002", TWELVE),
            ("can-cw0000000003", ATLAS),
            ("can-cw0000000004", "Commit freely; never push."),
        ],
    );
    let f1 = asserted(PRE_PUSH, Some("can-cw0000000001"), 100);
    let f2 = asserted(TWELVE, Some("can-cw0000000002"), 101);
    let f3 = asserted(ATLAS, Some("can-cw0000000003"), 102);
    let f4 = asserted("Commit freely; never push.", Some("can-cw0000000004"), 103);
    let guard = asserted(GUARD, None, 104);
    let mine = asserted(MINE, None, 105);
    let fork = Log::from_acts(vec![
        retracted(&f1, 200),
        retracted(&f2, 201),
        retracted(&f3, 202),
        f1,
        f2,
        f3,
        f4,
        guard,
        mine,
    ])
    .derive();

    let t1 = asserted(PRE_PUSH, Some("can-cw0000000001"), 300);
    let t2 = asserted(TWELVE, Some("can-cw0000000002"), 301);
    let t3 = asserted(ATLAS, Some("can-cw0000000003"), 302);
    let t4 = asserted("Commit freely; never push.", Some("can-cw0000000004"), 303);
    let t5 = asserted(GUARD, None, 304);
    let family = Log::from_acts(vec![retracted(&t3, 400), t1, t2, t3, t4, t5]).derive();
    let target = Snapshot::of(&family, "cw-family-canon", "house", 0);
    (seed, fork, target, family)
}

fn landed_for<'a>(landed: &'a [Landed], needle: &str) -> &'a Landed {
    landed
        .iter()
        .find(|l| l.change.did.contains(needle))
        .unwrap_or_else(|| panic!("no verdict for {needle}"))
}

#[test]
fn a_change_the_new_base_settles_word_for_word_is_decided_in_code_not_by_the_model() {
    let (seed, fork, target, family) = family_case();
    let changes = describe(&Divergence::compute(&seed, &fork), &fork);
    assert_eq!(changes.len(), 5, "{changes:#?}");
    // What the model said on 2026-10-23, for every change it was shown.
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[
            (1, "orphaned", None, "the new rules have no such rule"),
            (2, "orphaned", None, "the new rules have no such rule"),
            (3, "orphaned", None, "the new rules have no such rule"),
            (4, "carries", None, "new law"),
            (5, "carries", None, "new law"),
        ]),
    )]);
    let base = Base {
        snapshot: &target,
        log: Some(&family),
    };
    let landed = land(|| Ok(mock.client()), &changes, &seed, &base).unwrap();
    let at = |text: &str| target.commitments.iter().position(|c| c.text == text);

    // A retraction of a rule the new base holds verbatim carries onto it.
    for rule in [PRE_PUSH, TWELVE] {
        let l = landed_for(&landed, rule);
        assert_eq!(l.verdict, Verdict::Carries, "{rule}");
        assert_eq!(l.target, at(rule).map(|i| i + 1), "{rule}");
    }
    // One the new base withdrew too is already there.
    assert_eq!(landed_for(&landed, ATLAS).verdict, Verdict::Already);
    // An addition the new base holds verbatim is already there.
    let g = landed_for(&landed, GUARD);
    assert_eq!(g.verdict, Verdict::Already);
    assert_eq!(g.target, at(GUARD).map(|i| i + 1));

    // And the model was shown only what code could not decide.
    let asked: Vec<String> = mock
        .requests()
        .iter()
        .map(|r| r["messages"][1]["content"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(asked.len(), 1, "{asked:#?}");
    let shown = asked[0].split("The changes they made:").nth(1).unwrap();
    for rule in [PRE_PUSH, TWELVE, ATLAS, GUARD] {
        assert!(!shown.contains(rule), "the model was asked about {rule}");
    }
    assert!(shown.contains(MINE), "{shown}");
}

#[test]
fn nothing_left_for_the_model_means_no_model_is_needed() {
    // Every change settled in code: a canon with no endpoint configured can
    // still be rebased, and the client is never asked for.
    let (seed, fork, target, family) = family_case();
    let changes: Vec<Change> = describe(&Divergence::compute(&seed, &fork), &fork)
        .into_iter()
        .filter(|c| !c.did.contains(MINE))
        .collect();
    let base = Base {
        snapshot: &target,
        log: Some(&family),
    };
    let landed = land(
        || Err(ModelError::Config("no endpoint configured".into())),
        &changes,
        &seed,
        &base,
    )
    .unwrap();
    assert_eq!(landed.len(), changes.len());
}

#[test]
fn the_rules_and_the_changes_are_numbered_apart_so_neither_reads_as_the_other() {
    // The 2026-10-23 runs cited "New rule 43 is the store-is-the-trace-of-
    // record rule" and printed target rule 43, "You are a Senior Architect":
    // both lists were numbered from 1, and for an addition, which has no
    // target rule, the model gave its own change number as the target.
    let (seed, canon) = seed_and_canon();
    let t = target();
    let changes = describe(&Divergence::compute(&seed, &canon), &canon);
    let mock = Mock::spawn(vec![(200, mapping(&[]))]);
    map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    let user = mock.requests()[0]["messages"][1]["content"]
        .as_str()
        .unwrap()
        .to_string();
    let numbers = |section: &str| -> Vec<usize> {
        section
            .lines()
            .filter_map(|l| l.split_once(". "))
            .filter_map(|(n, _)| n.trim().parse::<usize>().ok())
            .collect()
    };
    let (rules, made) = user.split_once("The changes they made:").unwrap();
    let rules = numbers(
        rules
            .split("The rules they want to land on:")
            .nth(1)
            .unwrap(),
    );
    let made = numbers(made);
    assert_eq!(rules.len(), t.commitments.len());
    assert_eq!(made.len(), changes.len());
    assert!(
        made.iter().all(|n| !rules.contains(n)),
        "rules {rules:?} and changes {made:?} share numbers"
    );

    // A reply that names an addition's own number as its target is not
    // read as a target rule.
    let own = made[1];
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[(own, "carries", Some(own), "new rule is the movie rule")]),
    )]);
    let landed = map_changes(&mock.client(), &changes, &seed, &t).unwrap();
    assert_eq!(landed.len(), 1);
    assert_eq!(landed[0].change.kind, Kind::Addition);
    assert_eq!(landed[0].target, None);
}

#[test]
fn a_replacement_of_a_rule_the_new_base_withdrew_carries_as_this_canons_own() {
    // zoracite replaced "It must be specific and actionable by someone who
    // did not see the incident." with its own words; cw-family-canon adopted
    // with `--except`, so its copy of the original arrived retracted. The
    // model called the change orphaned, which drops the replacement. The
    // base withdrew the original itself, so the replacement is this canon's
    // own rule on the new base, and that is a lookup.
    const OLD: &str = "It must be specific and actionable by someone who did not see the incident.";
    const NEW: &str = "A principle must be specific, and actionable by someone who did not see it.";
    let seed = snap(
        "commonwealth-ai",
        &[
            ("can-cw0000000010", OLD),
            ("can-cw0000000004", "Commit freely; never push."),
        ],
    );
    let old = asserted(OLD, Some("can-cw0000000010"), 100);
    let keep = asserted("Commit freely; never push.", Some("can-cw0000000004"), 101);
    let sup = Act::new(
        ActKind::Supersede {
            text: NEW.into(),
            old: vec![old.id.clone()],
            rationale: "ours".into(),
        },
        200,
        "human:sam",
    );
    let fork = Log::from_acts(vec![old, keep, sup]).derive();

    let t_old = asserted(OLD, Some("can-cw0000000010"), 300);
    let t_keep = asserted("Commit freely; never push.", Some("can-cw0000000004"), 301);
    let family = Log::from_acts(vec![retracted(&t_old, 300), t_old, t_keep]).derive();
    let target = Snapshot::of(&family, "cw-family-canon", "house", 0);

    let changes = describe(&Divergence::compute(&seed, &fork), &fork);
    assert_eq!(changes.len(), 1, "{changes:#?}");
    let mock = Mock::spawn(vec![(
        200,
        mapping(&[(2, "orphaned", None, "the new set has no such rule")]),
    )]);
    let base = Base {
        snapshot: &target,
        log: Some(&family),
    };
    let landed = land(|| Ok(mock.client()), &changes, &seed, &base).unwrap();
    assert_eq!(landed.len(), 1);
    assert_eq!(landed[0].verdict, Verdict::Carries);
    assert!(mock.requests().is_empty(), "the model was asked");
    let text = render(&seed, &target, &changes, &landed);
    assert!(text.contains(&format!("canon add \"{NEW}\"")), "{text}");
}
