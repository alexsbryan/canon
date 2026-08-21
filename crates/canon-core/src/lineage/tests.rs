// SPDX-License-Identifier: AGPL-3.0-or-later
//! Snapshot round-trip and divergence arithmetic. Both pure — no IO in this
//! crate, and no model anywhere near a lineage.

use super::*;
use crate::act::ActKind;
use crate::{Act, Log};

fn asserted(text: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: None,
            source: None,
        },
        ts,
        "human:dana",
    )
}

fn inherited(text: &str, from: &ActId, ts: i64) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: Some(from.clone()),
            source: None,
        },
        ts,
        "human:priya",
    )
}

fn upstream() -> Snapshot {
    let canon = Log::from_acts(vec![
        asserted("Quiet hours run 11pm-7am.", 100),
        asserted("A guest may stay 2 nights in any 7.", 110),
        asserted("The kitchen is cleaned by whoever used it.", 120),
    ])
    .derive();
    Snapshot::of(&canon, "house-5-consensus", "house", 1_787_000_000)
}

#[test]
fn a_snapshot_round_trips_through_the_block_a_person_pastes() {
    let snap = upstream();
    let block = snap.render("2026-08-21");
    let back = Snapshot::parse(&block).unwrap();
    assert_eq!(back.lineage, "house-5-consensus");
    assert_eq!(back.profile, "house");
    assert_eq!(back.generation, snap.generation);
    assert_eq!(back.commitments, snap.commitments);
}

#[test]
fn the_block_survives_the_chat_thread_around_it() {
    // It arrives with "here you go" above it and a reply below.
    let block = upstream().render("2026-08-21");
    let pasted = format!("dana: here you go!\n\n{block}\nsam: thanks");
    assert_eq!(Snapshot::parse(&pasted).unwrap().commitments.len(), 3);
}

#[test]
fn a_block_edited_after_it_was_shared_is_refused() {
    // Someone "fixes" a rule in the thread before pasting it on. The
    // generation no longer matches the commitments, and adopting it would
    // record it as though the sender had published it.
    let block = upstream().render("2026-08-21");
    let tampered = block.replace("11pm-7am", "midnight-7am");
    let err = Snapshot::parse(&tampered).expect_err("must refuse");
    assert!(err.contains("edited after it was shared"), "{err}");
}

#[test]
fn the_generation_does_not_depend_on_the_order_the_rules_are_in() {
    // Two people holding the same rules are on the same generation, whatever
    // order their files landed in.
    let mut a = upstream();
    let mut b = a.clone();
    b.commitments.reverse();
    assert_eq!(generation_of(&a.commitments), generation_of(&b.commitments));
    a.commitments.pop();
    assert_ne!(generation_of(&a.commitments), generation_of(&b.commitments));
}

#[test]
fn a_line_with_no_id_is_refused_rather_than_adopted_as_a_rule() {
    let block = "--- canon x · house · snapshot 2026-08-21\nsomething someone typed\n--- 1 live\n";
    assert!(Snapshot::parse(block).is_err());
}

#[test]
fn divergence_is_arithmetic_over_two_logs() {
    let seed = upstream();
    let (q, g, k) = (
        seed.commitments[0].id.clone(),
        seed.commitments[1].id.clone(),
        seed.commitments[2].id.clone(),
    );

    // The adopter takes all three, then rewrites quiet hours, retracts the
    // guest rule, adds one of their own, and carries a contradiction.
    let iq = inherited("Quiet hours run 11pm-7am.", &q, 200);
    let ig = inherited("A guest may stay 2 nights in any 7.", &g, 201);
    let ik = inherited("The kitchen is cleaned by whoever used it.", &k, 202);
    let mine = asserted("Thursday is movie night until midnight.", 203);
    let acts = vec![
        iq.clone(),
        ig.clone(),
        ik.clone(),
        mine.clone(),
        Act::new(
            ActKind::Supersede {
                text: "Quiet hours run 10pm-7am on weeknights.".into(),
                old: vec![iq.id.clone()],
                rationale: "night shifts".into(),
            },
            300,
            "human:priya",
        ),
        Act::new(
            ActKind::Retract {
                target: ig.id.clone(),
                rationale: "we do not host".into(),
            },
            310,
            "human:priya",
        ),
        Act::new(
            ActKind::Accept {
                a: ik.id.clone(),
                b: mine.id.clone(),
                rationale: "the cook gets the night off".into(),
                revisit: None,
            },
            320,
            "human:priya",
        ),
    ];
    let canon = Log::from_acts(acts).derive();
    let d = Divergence::compute(&seed, &canon);

    assert_eq!(d.lineage, "house-5-consensus");
    assert_eq!(d.count(|f| matches!(f, Fate::Superseded { .. })), 1);
    assert_eq!(d.count(|f| matches!(f, Fate::Retracted)), 1);
    assert_eq!(d.count(|f| matches!(f, Fate::Accepted { .. })), 1);
    assert_eq!(d.count(|f| matches!(f, Fate::Untouched)), 0);
    assert_eq!(d.added.len(), 1, "only the locally authored rule is added");

    let sup = d
        .inherited
        .iter()
        .find(|i| i.upstream == q)
        .expect("the quiet-hours line");
    assert!(matches!(&sup.fate, Fate::Superseded { text, .. } if text.contains("10pm")));
}

#[test]
fn a_seed_commitment_that_never_landed_is_reported_not_ignored() {
    // A paste that lost a line, or an adoption someone edited. Silence here
    // would read as "untouched", which is the opposite of what happened.
    let seed = upstream();
    let canon = Log::from_acts(vec![inherited(
        "Quiet hours run 11pm-7am.",
        &seed.commitments[0].id,
        200,
    )])
    .derive();
    let d = Divergence::compute(&seed, &canon);
    assert_eq!(d.count(|f| matches!(f, Fate::Never)), 2);
    assert_eq!(d.count(|f| matches!(f, Fate::Untouched)), 1);
}

#[test]
fn divergence_links_by_provenance_not_by_text() {
    // A canon that arrived by paste has no git history, and an adopter who
    // reworded a rule in place still holds THAT rule. Text matching would
    // call it a different one.
    let seed = upstream();
    let canon = Log::from_acts(vec![inherited(
        "Quiet hours: 11pm to 7am, every night.",
        &seed.commitments[0].id,
        200,
    )])
    .derive();
    let d = Divergence::compute(&seed, &canon);
    assert!(matches!(d.inherited[0].fate, Fate::Untouched));
    assert!(
        d.added.is_empty(),
        "a reworded inherited rule is not an addition"
    );
}
