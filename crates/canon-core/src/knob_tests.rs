// SPDX-License-Identifier: AGPL-3.0-or-later
//! The two v4 knobs on a ratification rule: `governs` (a change of policy
//! waits like any other change) and `personal` (a person's own scope is
//! theirs). Each is checked in both directions — what it changes when set,
//! and that a canon without it folds as it always did.

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::{
    act::{Act, ActKind},
    id::ActId,
    log::Log,
    policy::Rule,
    ratify::{Governs, Ratify, Seat, Verdict},
    scope::Scope,
    standing::Pull,
};

const DAY: i64 = 86_400;

fn scope(s: &str) -> Scope {
    Scope::new(s).expect("a scope")
}

fn grant(holder: &str, s: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Grant {
            holder: holder.into(),
            scope: scope(s),
            horizon: None,
            rationale: String::new(),
        },
        ts,
        "human:sam",
    )
}

fn ratification(
    rule: &str,
    governs: Option<Governs>,
    personal: Option<bool>,
    ts: i64,
    actor: &str,
) -> Act {
    Act::new(
        ActKind::Ratification {
            text: String::new(),
            rule: Ratify::parse(rule).expect("a rule"),
            scope: None,
            governs,
            personal,
        },
        ts,
        actor,
    )
}

fn policy(rule: Rule, s: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Policy {
            text: String::new(),
            rule,
            scope: Some(scope(s)),
        },
        ts,
        actor,
    )
}

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

fn scoped(c: &ActId, s: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Scoped {
            commitment: c.clone(),
            scope: scope(s),
        },
        ts,
        actor,
    )
}

fn position(about: &ActId, pull: Pull, ts: i64, actor: &str, why: &str) -> Act {
    Act::new(
        ActKind::Position {
            about: about.to_string(),
            citing: None,
            pull,
            because: why.into(),
        },
        ts,
        actor,
    )
}

/// Sam, Dana and Theo hold the house, granted before anything else.
fn house() -> Vec<Act> {
    vec![
        grant("human:sam", "house", 10),
        grant("human:dana", "house", 10),
        grant("human:theo", "house", 10),
    ]
}

// ── governs ──────────────────────────────────────────────────────────────

#[test]
fn a_policy_waits_under_governs_all_and_applies_from_its_ratified_moment() {
    let mut acts = house();
    acts.push(ratification(
        "consent:7d",
        Some(Governs::All),
        None,
        100,
        "human:sam",
    ));
    let change = policy(Rule::Subsidiarity, "house.kitchen", 200, "human:dana");
    let id = change.id.clone();
    acts.push(change);
    let log = Log::from_acts(acts);

    let during = log.derive_at(200 + DAY);
    assert!(during.policies.is_empty(), "one holder cannot set it alone");
    assert!(matches!(
        during.pending_policies[0].verdict,
        Some(Verdict::Proposed { .. })
    ));
    assert_eq!(
        *during.policy_for(Some(&scope("house.kitchen"))),
        Rule::Default
    );

    let after = log.derive_at(200 + 8 * DAY);
    assert_eq!(after.policies.len(), 1);
    assert_eq!(after.policies[0].act, id);
    assert!(matches!(
        after.policies[0].verdict,
        Some(Verdict::Ratified { since, .. }) if since == 200 + 7 * DAY
    ));
}

#[test]
fn one_reasoned_objection_refuses_a_waiting_policy() {
    let mut acts = house();
    acts.push(ratification(
        "consent:7d",
        Some(Governs::All),
        None,
        100,
        "human:sam",
    ));
    let change = policy(Rule::Subsidiarity, "house.kitchen", 200, "human:dana");
    let id = change.id.clone();
    acts.push(change);
    acts.push(position(
        &id,
        Pull::Against,
        300,
        "human:theo",
        "the kitchen is everyone's",
    ));
    let canon = Log::from_acts(acts).derive_at(200 + 30 * DAY);
    assert!(canon.policies.is_empty());
    assert!(matches!(
        &canon.pending_policies[0].verdict,
        Some(Verdict::Refused { by, .. }) if by == "human:theo"
    ));
}

#[test]
fn without_governs_a_policy_applies_on_its_writers_standing_as_before() {
    // The canon every build before v4 wrote: a consent rule for commitments,
    // and a policy that takes effect the moment a holder sets it.
    let mut acts = house();
    acts.push(ratification("consent:7d", None, None, 100, "human:sam"));
    acts.push(policy(
        Rule::Subsidiarity,
        "house.kitchen",
        200,
        "human:dana",
    ));
    let canon = Log::from_acts(acts).derive_at(200 + DAY);
    assert_eq!(canon.policies.len(), 1);
    assert_eq!(canon.policies[0].verdict, None);
    assert!(canon.pending_policies.is_empty());
}

#[test]
fn governs_all_under_standing_changes_nothing() {
    // `standing` is a holder's word taking effect at once, for policy as for
    // anything else, so the knob only bites where the group chose more.
    let mut acts = house();
    acts.push(ratification(
        "standing",
        Some(Governs::All),
        None,
        100,
        "human:sam",
    ));
    acts.push(policy(
        Rule::Subsidiarity,
        "house.kitchen",
        200,
        "human:dana",
    ));
    let canon = Log::from_acts(acts).derive_at(201);
    assert_eq!(canon.policies.len(), 1);
    assert!(canon.pending_policies.is_empty());
}

// ── personal ─────────────────────────────────────────────────────────────

/// The house, with personal scopes honoured from t=100 under `standing`.
fn personal_house() -> Vec<Act> {
    let mut acts = house();
    acts.push(ratification("standing", None, Some(true), 100, "human:sam"));
    acts
}

#[test]
fn a_personal_scope_is_its_owners_alone_and_never_open() {
    let mut acts = personal_house();
    let glasses = assert_at("Ask before using my glasses.", 200, "human:dana");
    acts.push(scoped(&glasses.id, "human:dana.glasses", 200, "human:dana"));
    acts.push(glasses);
    let canon = Log::from_acts(acts).derive_at(300);
    let s = scope("human:dana.glasses");
    assert_eq!(canon.seat("human:dana", Some(&s), 300), Seat::Held);
    assert_eq!(canon.seat("human:sam", Some(&s), 300), Seat::Lacking);
    assert_eq!(canon.seat("human:stranger", Some(&s), 300), Seat::Lacking);
    let who: Vec<&str> = canon
        .who_decides(&s, 300)
        .iter()
        .map(|g| g.actor.as_str())
        .collect();
    assert_eq!(who, ["human:dana"]);
}

#[test]
fn a_housemate_cannot_govern_somebody_elses_corner() {
    let mut acts = personal_house();
    acts.push(grant("human:sam", "human:dana.glasses", 200));
    acts.push(policy(
        Rule::Consent,
        "human:dana.glasses",
        200,
        "human:sam",
    ));
    let canon = Log::from_acts(acts).derive_at(300);
    assert_eq!(canon.ungoverned.len(), 2, "{:?}", canon.ungoverned);
    assert!(canon.policies.is_empty());
}

#[test]
fn personal_scopes_are_ordinary_scopes_without_the_knob() {
    // A canon that never turned the knob on: `human:dana.glasses` is a root
    // like any other, and somebody seated in the house may claim it.
    let mut acts = house();
    acts.push(grant("human:sam", "human:dana.glasses", 200));
    let canon = Log::from_acts(acts).derive_at(300);
    let s = scope("human:dana.glasses");
    assert_eq!(canon.seat("human:sam", Some(&s), 300), Seat::Held);
    assert_eq!(canon.seat("human:dana", Some(&s), 300), Seat::Lacking);
}

#[test]
fn an_owners_write_is_in_force_even_where_the_house_decides_by_consent() {
    let mut acts = house();
    acts.push(ratification(
        "consent:7d",
        Some(Governs::All),
        Some(true),
        100,
        "human:sam",
    ));
    // The knob itself waited a week under the rule it sits on.
    let at = 100 + 8 * DAY;
    let mine = assert_at("Ask before using my glasses.", at, "human:dana");
    let theirs = assert_at("Dana's glasses are for parties.", at, "human:sam");
    acts.push(scoped(&mine.id, "human:dana.glasses", at, "human:dana"));
    acts.push(scoped(&theirs.id, "human:dana.glasses", at, "human:sam"));
    let (mine_id, theirs_id) = (mine.id.clone(), theirs.id.clone());
    acts.push(mine);
    acts.push(theirs);
    let (mine, theirs) = (mine_id, theirs_id);
    let canon = Log::from_acts(acts).derive_at(at + DAY);
    assert!(matches!(
        canon.get(&mine).map(|c| &c.status),
        Some(crate::fold::Status::Active)
    ));
    assert!(
        matches!(
            canon.get(&theirs).map(|c| &c.status),
            Some(crate::fold::Status::Proposed { .. })
        ),
        "anyone else's write in her scope waits for her"
    );
}

#[test]
fn personal_roots_never_hold_the_canon() {
    let mut acts = personal_house();
    let note = assert_at("Mine.", 200, "human:dana");
    acts.push(scoped(&note.id, "human:dana", 200, "human:dana"));
    acts.push(note);
    let canon = Log::from_acts(acts).derive_at(300);
    let scopes: Vec<&str> = canon.canon_scopes(300).iter().map(|s| s.as_str()).collect();
    assert_eq!(scopes, ["house"]);
}

#[test]
fn the_knob_counts_from_its_ratified_moment_and_not_before() {
    // The first rule a canon adopts is judged under `standing` and takes at
    // once; the knob then arrives under consent and waits its week.
    let mut acts = house();
    acts.push(ratification("consent:7d", None, None, 100, "human:sam"));
    acts.push(ratification(
        "consent:7d",
        None,
        Some(true),
        200,
        "human:sam",
    ));
    // During the week the knob waits, the corner is an ordinary root.
    acts.push(grant("human:sam", "human:dana.glasses", 300));
    // After it, a grant into it by anyone but Dana is refused.
    acts.push(grant("human:theo", "human:dana.glasses", 200 + 8 * DAY));
    let canon = Log::from_acts(acts).derive_at(200 + 9 * DAY);
    assert_eq!(canon.ungoverned.len(), 1, "{:?}", canon.ungoverned);
    assert!(canon.ungoverned[0].1.contains("human:theo"));
}
