// SPDX-License-Identifier: AGPL-3.0-or-later
//! Characterization tests: each pins what the fold does today for a behaviour
//! a code review reported and nobody had demonstrated. They assert what was
//! observed, not what ought to happen; a change that moves one is a change to
//! how records read.

use canon_core::id::digest_hex;
use canon_core::standing::Pull;
use canon_core::{
    Act, ActKind, Allocation, Attributes, Canon, Log, Order, Outcome, Policy, Position, Ratify,
    Rule, Scope, Standing, Status, Verdict,
};
use canon_readers::{DrawError, Draws, Pools};

const ALEX: &str = "human:alex";
const BO: &str = "human:bo";
const CY: &str = "human:cy";
const HELPER: &str = "agent:helper";

fn scope(path: &str) -> Scope {
    Scope::new(path).expect("a scope")
}

fn rule(text: &str, at: i64, by: &str) -> Act {
    let (text, from, source) = (text.into(), None, None);
    Act::new(ActKind::Assert { text, from, source }, at, by)
}

fn replace(old: &Act, text: &str, why: &str, at: i64) -> Act {
    let (text, old, rationale) = (text.into(), vec![old.id.clone()], why.into());
    let kind = ActKind::Supersede {
        text,
        old,
        rationale,
    };
    Act::new(kind, at, ALEX)
}

fn retract(target: &Act, why: &str, at: i64, by: &str) -> Act {
    let (target, rationale) = (target.id.clone(), why.into());
    Act::new(ActKind::Retract { target, rationale }, at, by)
}

fn grant(holder: &str, over: &str, at: i64, by: &str) -> Act {
    let kind = ActKind::Grant {
        holder: holder.into(),
        scope: scope(over),
        horizon: None,
        rationale: String::new(),
    };
    Act::new(kind, at, by)
}

fn scoped(commitment: &Act, into: &str, at: i64, by: &str) -> Act {
    let (commitment, scope) = (commitment.id.clone(), scope(into));
    Act::new(ActKind::Scoped { commitment, scope }, at, by)
}

fn say(about: &str, pull: Pull, at: i64, by: &str) -> Act {
    let kind = ActKind::Position {
        about: about.into(),
        citing: None,
        pull,
        because: "said so".into(),
    };
    Act::new(kind, at, by)
}

fn approve(proposal: &Act, at: i64, by: &str) -> Act {
    say(proposal.id.as_str(), Pull::Toward, at, by)
}

fn status(canon: &Canon, of: &Act) -> Status {
    canon.get(&of.id).expect("the commitment").status().clone()
}

/// Two spellings of one act that differ only in an inert note: the first
/// sorts before `fixed` by id, the second after it.
fn either_side(fixed: &Act, make: impl Fn(String) -> Act) -> (Act, Act) {
    let (mut before, mut after) = (None, None);
    for n in 0..64 {
        let act = make(format!("note {n}"));
        if act.id < fixed.id {
            before.get_or_insert(act);
        } else {
            after.get_or_insert(act);
        }
    }
    (before.expect("one before"), after.expect("one after"))
}

fn with(base: &[Act], more: &[&Act]) -> Canon {
    let acts = base.iter().chain(more.iter().copied()).cloned().collect();
    Log::from_acts(acts).derive()
}

#[test]
fn a_withdrawal_and_an_act_that_needs_the_withdrawn_standing_in_one_second_read_by_id_order() {
    // Reported: within one second acts fold in id order, and it changes the
    // outcome. Observed: it does. Bo is stood down in the same second Bo puts
    // Cy in charge; when the withdrawal's id sorts first Bo's grant is refused
    // and Cy holds nothing, and when it sorts second the grant applies and Cy
    // holds the house. Only the withdrawal's rationale text differs.
    let base = [grant(ALEX, "house", 1, ALEX), grant(BO, "house", 1, ALEX)];
    let seats_cy = grant(CY, "house", 10, BO);
    let (first, second) = either_side(&seats_cy, |rationale| {
        let (holder, scope) = (BO.into(), scope("house"));
        let kind = ActKind::Withdraw {
            holder,
            scope,
            rationale,
        };
        Act::new(kind, 10, ALEX)
    });

    let withdrawn_first = with(&base, &[&seats_cy, &first]);
    assert!(!withdrawn_first.standing_of(CY, &scope("house"), 10));
    assert_eq!(withdrawn_first.ungoverned().len(), 1);
    assert_eq!(withdrawn_first.ungoverned()[0].0, seats_cy.id);

    let withdrawn_second = with(&base, &[&seats_cy, &second]);
    assert!(withdrawn_second.standing_of(CY, &scope("house"), 10));
    assert!(withdrawn_second.ungoverned().is_empty());

    // Bo is out either way; only what Bo's last act did differs.
    for canon in [&withdrawn_first, &withdrawn_second] {
        assert!(!canon.standing_of(BO, &scope("house"), 10));
    }
}

#[test]
fn a_scoping_and_a_retraction_in_one_second_read_by_id_order() {
    // Reported: a `scoped` act and a `retract` of the same commitment by
    // someone whose standing depends on that scope, written in one second,
    // read either way round. Observed: they do. Bo holds only the kitchen;
    // when the scoping sorts first Bo's retraction applies, and when the
    // retraction sorts first it is judged against the whole canon, where Bo
    // lacks a seat, so it waits as a proposal and the rule stays in force.
    let base = [
        grant(ALEX, "house", 1, ALEX),
        grant(BO, "house.kitchen", 1, ALEX),
        grant(CY, "house.kitchen", 1, ALEX),
    ];
    let knives = rule("Knives are washed by hand.", 5, CY);
    let placed = scoped(&knives, "house.kitchen", 10, CY);
    let (first, second) = either_side(&placed, |why| retract(&knives, &why, 10, BO));

    let scoped_first = with(&base, &[&knives, &placed, &second]);
    assert_eq!(status(&scoped_first, &knives), Status::Retracted { at: 10 });
    assert!(scoped_first.retractions().is_empty());

    let retracted_first = with(&base, &[&knives, &placed, &first]);
    assert_eq!(status(&retracted_first, &knives), Status::Active);
    assert_eq!(retracted_first.retractions().len(), 1);
    let waiting = &retracted_first.retractions()[0];
    assert_eq!(waiting.act, first.id);
    assert!(matches!(waiting.verdict, Verdict::Proposed { .. }));
}

#[test]
fn a_replacement_beats_a_retraction_whichever_was_written_later() {
    // Reported: precedence is by pass, not by time. Observed: it is. A rule
    // that is both retracted and replaced reads as superseded whether the
    // retraction came after the replacement or before it, because
    // supersession is applied in the ratification pass, after every
    // retraction. The later retraction leaves no trace in the status.
    let old = rule("Quiet after ten.", 1, ALEX);
    let new = |at| replace(&old, "Quiet after eleven.", "summer", at);
    let superseded = |by: &Act| Status::Superseded { by: by.id.clone() };

    let (early, late) = (new(5), new(9));
    let retracted_later = with(&[], &[&old, &early, &retract(&old, "dropped", 9, ALEX)]);
    assert_eq!(status(&retracted_later, &old), superseded(&early));
    assert_eq!(status(&retracted_later, &early), Status::Active);

    let replaced_later = with(&[], &[&old, &retract(&old, "dropped", 5, ALEX), &late]);
    assert_eq!(status(&replaced_later, &old), superseded(&late));
    assert_eq!(status(&replaced_later, &late), Status::Active);
}

#[test]
fn a_chain_of_replacements_in_one_second_can_leave_the_first_rule_in_force() {
    // Reported: with C replacing B replacing A, the judging order decides
    // whether A ends up active. Observed: it does. B and C share a second;
    // when C's id sorts after B's the chain reads A superseded, B superseded,
    // C active. When C sorts first it retires B before B is judged, B is then
    // skipped, and A is never retired: A and C are both in force.
    let a = rule("Quiet after ten.", 1, ALEX);
    let b = replace(&a, "Quiet after eleven.", "summer", 5);
    let (first, second) = either_side(&b, |why| replace(&b, "Quiet after midnight.", &why, 5));
    let superseded = |by: &Act| Status::Superseded { by: by.id.clone() };

    let c_after_b = with(&[], &[&a, &b, &second]);
    assert_eq!(status(&c_after_b, &a), superseded(&b));
    assert_eq!(status(&c_after_b, &b), superseded(&second));
    assert_eq!(c_after_b.active().count(), 1);

    let c_before_b = with(&[], &[&a, &b, &first]);
    assert_eq!(status(&c_before_b, &a), Status::Active);
    assert_eq!(status(&c_before_b, &b), superseded(&first));
    assert_eq!(status(&c_before_b, &first), Status::Active);
    assert_eq!(c_before_b.active().count(), 2);
}

#[test]
fn acts_dated_after_now_are_folded_and_a_later_approval_ratifies_at_now() {
    // Reported: `derive_at(now)` still applies acts dated after `now`.
    // Observed: it does. Folded at 50, an agent's proposal is in force on an
    // approval dated 100, with a verdict that says it was ratified at 100; a
    // rule dated 200 is in force; and a grant dated 300 is in `grants()`,
    // though `standing_of` at 50 does not count it.
    let base = [grant(ALEX, "house", 1, ALEX)];
    let drafted = rule("Bins go out on Thursday.", 10, HELPER);
    let placed = scoped(&drafted, "house", 10, HELPER);
    let approval = approve(&drafted, 100, ALEX);
    let future_rule = rule("Guests are announced.", 200, ALEX);
    let future_grant = grant(BO, "house", 300, ALEX);
    let at_50 = |more: &[&Act]| {
        let acts = base.iter().chain(more.iter().copied()).cloned().collect();
        Log::from_acts(acts).derive_at(50)
    };

    let unapproved = at_50(&[&drafted, &placed]);
    assert!(matches!(
        status(&unapproved, &drafted),
        Status::Proposed { .. }
    ));

    let canon = at_50(&[&drafted, &placed, &approval, &future_rule, &future_grant]);
    assert_eq!(status(&canon, &drafted), Status::Active);
    let verdict = canon.ratify(canon.get(&drafted.id).expect("the proposal"), 50);
    assert!(matches!(verdict, Verdict::Ratified { since: 100, .. }));
    assert_eq!(status(&canon, &future_rule), Status::Active);
    assert!(canon.grants().iter().any(|g| g.actor == BO));
    assert!(!canon.standing_of(BO, &scope("house"), 50));
    assert!(canon.standing_of(BO, &scope("house"), 300));
}

#[test]
fn a_supermajority_counts_an_agents_position_when_a_caller_hands_it_one() {
    // Reported: `Rule::Supermajority` counts positions without asking whether
    // the actor is a person. Observed: it does, both ways. `decide` counts
    // every position whose source is an actor, and `Standing::cited` keeps
    // any non-empty actor, so an agent's no sinks a vote and two agents' yes
    // carry one. `Canon::said_about` drops agents, so a caller that builds
    // its standing from that never hands `decide` an agent's position.
    let empty = Log::from_acts(Vec::new()).derive();
    let (numerator, denominator) = (2, 3);
    let two_thirds = Rule::Supermajority {
        numerator,
        denominator,
    };
    let decide = |positions: Vec<Position>| {
        let (standing, refused) = Standing::cited(&empty, "paint the shed", positions);
        assert!(refused.is_empty(), "every position reached the rule");
        two_thirds.decide(&standing, &Attributes::default(), &empty)
    };

    let sunk = decide(vec![
        Position::by(ALEX, Pull::Toward, "yes"),
        Position::by(HELPER, Pull::Against, "no"),
    ]);
    assert_eq!(sunk.outcome, Outcome::Conflicts);
    assert_eq!(sunk.because, "supermajority: 1/2 toward, 2/3 needed");

    let carried = decide(vec![
        Position::by(ALEX, Pull::Against, "no"),
        Position::by(HELPER, Pull::Toward, "yes"),
        Position::by("agent:second", Pull::Toward, "yes"),
    ]);
    assert_eq!(carried.outcome, Outcome::Supported);
    assert_eq!(carried.because, "supermajority: 2/3 toward, 2/3 needed");

    let recorded = with(
        &[],
        &[
            &say("paint the shed", Pull::Toward, 1, ALEX),
            &say("paint the shed", Pull::Against, 2, HELPER),
        ],
    );
    assert_eq!(recorded.positions().len(), 2);
    let said = recorded.said_about("paint the shed");
    assert_eq!(said.len(), 1);
    assert_eq!(said[0].actor(), Some(ALEX));
}

#[test]
fn a_joint_rule_never_counts_a_named_person_who_holds_only_the_wider_area() {
    // Reported: `joint` names people, but only the narrowest holders' words
    // are tallied. Observed: so. Alex holds the house, Bo the kitchen, and
    // the kitchen's rule names both; Alex's approval of a kitchen proposal is
    // not counted and the proposal waits on Alex for good. The same approval
    // counts once Alex is also granted the kitchen itself.
    let kitchen = scope("house.kitchen");
    let joint = Ratify::Joint {
        holders: vec![ALEX.into(), BO.into()],
    };
    let adopted = ActKind::Ratification {
        text: "Alex and Bo both sign kitchen rules.".into(),
        rule: joint.clone(),
        scope: Some(kitchen.clone()),
        governs: None,
        personal: None,
    };
    let base = [
        grant(ALEX, "house", 1, ALEX),
        Act::new(adopted, 2, ALEX),
        grant(BO, "house.kitchen", 3, ALEX),
    ];
    let knives = rule("Knives are washed by hand.", 10, BO);
    let placed = scoped(&knives, "house.kitchen", 10, BO);
    let approval = approve(&knives, 11, ALEX);

    let wider_only = with(&base, &[&knives, &placed, &approval]);
    assert_eq!(wider_only.ratification_for(Some(&kitchen)), &joint);
    assert_eq!(wider_only.positions().len(), 1, "the approval is recorded");
    let Status::Proposed { needs } = status(&wider_only, &knives) else {
        panic!("the proposal waits");
    };
    assert_eq!(needs, "approval from human:alex");

    let also_kitchen = grant(ALEX, "house.kitchen", 3, ALEX);
    let narrow_too = with(&base, &[&also_kitchen, &knives, &placed, &approval]);
    assert_eq!(status(&narrow_too, &knives), Status::Active);
}

#[test]
fn a_pool_and_a_draw_disagree_about_who_holds_a_personal_area() {
    // Reported: `pool_at` filters grants directly while `draw` asks
    // `who_decides`, so the two can name different people. Observed: they do
    // once personal areas are switched on. Alex puts Bo and Cy in charge of
    // Alex's own tools; `pool_at` gives all three a turn, while `who_decides`
    // and the draw over the same area see Alex alone.
    let tools = scope("human:alex.tools");
    let knob = ActKind::Ratification {
        text: "Each person's own area is theirs.".into(),
        rule: Ratify::Standing,
        scope: None,
        governs: None,
        personal: Some(true),
    };
    let allot = ActKind::Allot {
        text: "Three tools.".into(),
        unit: "tool".into(),
        units: vec!["drill".into(), "plane".into(), "saw".into()],
        scope: tools.clone(),
    };
    let (order, step, per) = (Order::Holders, 1, 86_400);
    let share = ActKind::Allocation {
        text: "They go round daily.".into(),
        rule: Allocation::Rotation { order, step, per },
        scope: tools.clone(),
    };
    let announce = ActKind::DrawCommit {
        scope: tools.clone(),
        count: 1,
        after_ts: 20,
        rationale: String::new(),
    };
    let announced = Act::new(announce, 8, ALEX);
    let commit = announced.id.clone();
    let seal = ActKind::DrawSecret {
        commit: commit.clone(),
        digest: digest_hex(b"alex's secret"),
    };
    let open = ActKind::DrawReveal {
        commit: commit.clone(),
        secret: "alex's secret".into(),
    };
    let canon = Log::from_acts(vec![
        grant(ALEX, "house", 1, ALEX),
        Act::new(knob, 2, ALEX),
        grant(BO, "human:alex.tools", 5, ALEX),
        grant(CY, "human:alex.tools", 5, ALEX),
        Act::new(allot, 6, ALEX),
        Act::new(share, 7, ALEX),
        announced,
        Act::new(seal, 9, ALEX),
        Act::new(open, 21, ALEX),
    ])
    .derive_at(30);
    assert!(canon.ungoverned().is_empty(), "every act above applied");

    let schedule = canon.pool_at(&tools, 20).expect("a schedule");
    let mut turns: Vec<&str> = schedule.awards.iter().map(|a| a.actor.as_str()).collect();
    turns.sort_unstable();
    assert_eq!(turns, [ALEX, BO, CY]);

    let deciders: Vec<&str> = canon
        .who_decides(&tools, 20)
        .iter()
        .map(|g| g.actor.as_str())
        .collect();
    assert_eq!(deciders, [ALEX]);
    let (pool, count) = (1, 1);
    assert_eq!(
        canon.draw(&commit),
        Err(DrawError::PoolTooSmall { pool, count })
    );
}
