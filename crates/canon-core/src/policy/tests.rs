// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::act::{Act, ActKind};
use crate::log::Log;
use crate::standing::Position;

fn canon_with(texts: &[&str]) -> (Canon, Vec<ActId>) {
    let acts: Vec<Act> = texts
        .iter()
        .enumerate()
        .map(|(i, t)| {
            Act::new(
                ActKind::Assert {
                    text: (*t).into(),
                    from: None,
                    source: None,
                },
                100 + i as i64,
                "human:alex",
            )
        })
        .collect();
    let ids = acts.iter().map(|a| a.id.clone()).collect();
    (Log::from_acts(acts).derive(), ids)
}

fn standing(canon: &Canon, positions: Vec<Position>) -> Standing {
    Standing::cited(canon, "p", positions).0
}

// ── the §10.6 guard ─────────────────────────────────────────

/// **Captured before the policy layer existed, asserted after.**
///
/// This is the table the plan required: every shape of evidence the old
/// inline rule could see, with the answer it gave. `Rule::Default` must
/// reproduce it exactly, and `Standing::outcome()` must agree with
/// `Rule::Default` — not approximately, not usually.
///
/// The failure it prevents is specific and quiet: two implementations of one
/// rule that agree the day they are written. Nothing goes red when they part;
/// the tool simply starts answering two ways depending on which caller asked.
#[test]
fn default_reproduces_the_rule_that_shipped_before_policy_was_configurable() {
    let (canon, ids) = canon_with(&["a", "b", "c"]);
    let toward = |i: usize| Position::of(ids[i].clone(), Pull::Toward, "helps");
    let against = |i: usize| Position::of(ids[i].clone(), Pull::Against, "hurts");
    let vote = |who: &str, pull| Position::by(who, pull, "said so");

    // (positions, the outcome the inline rule gave)
    let table: Vec<(Vec<Position>, Outcome)> = vec![
        (vec![], Outcome::Unaddressed),
        (vec![toward(0)], Outcome::Supported),
        (vec![toward(0), toward(1)], Outcome::Supported),
        (vec![against(0)], Outcome::Conflicts),
        (vec![against(0), against(1)], Outcome::Conflicts),
        (vec![toward(0), against(1)], Outcome::Conflicts),
        (vec![against(0), toward(1), toward(2)], Outcome::Conflicts),
        (vec![vote("human:dana", Pull::Toward)], Outcome::Supported),
        (vec![vote("human:dana", Pull::Against)], Outcome::Conflicts),
        (
            vec![toward(0), vote("human:dana", Pull::Against)],
            Outcome::Conflicts,
        ),
    ];

    for (positions, expected) in table {
        let s = standing(&canon, positions);
        assert_eq!(
            s.outcome(),
            expected,
            "the captured rule changed for {:?}",
            s.positions
        );
        let d = Rule::Default.decide(&s, &Attributes::default(), &canon);
        assert_eq!(
            d.outcome, expected,
            "Rule::Default diverged from the captured rule for {:?}",
            s.positions
        );
        assert_eq!(
            d.outcome,
            s.outcome(),
            "two deciders, two answers — the §10.6 failure"
        );
    }
}

#[test]
fn the_shipped_default_makes_adjudication_a_human_act() {
    let (canon, ids) = canon_with(&["a"]);
    let attrs = Attributes::default();
    let supported = standing(
        &canon,
        vec![Position::of(ids[0].clone(), Pull::Toward, "y")],
    );
    let conflicts = standing(
        &canon,
        vec![Position::of(ids[0].clone(), Pull::Against, "n")],
    );
    let silent = standing(&canon, vec![]);

    assert_eq!(
        Rule::Default.decide(&supported, &attrs, &canon).authority,
        Authority::Act
    );
    // Neither a conflict nor a silence authorizes anything by itself.
    assert_eq!(
        Rule::Default.decide(&conflicts, &attrs, &canon).authority,
        Authority::AskOne
    );
    assert_eq!(
        Rule::Default.decide(&silent, &attrs, &canon).authority,
        Authority::AskOne
    );
}

// ── the named policies ──────────────────────────────────────

#[test]
fn consent_blocks_on_one_reasoned_objection_and_ignores_the_count() {
    let (canon, ids) = canon_with(&["a"]);
    let attrs = Attributes::default();
    let one = standing(
        &canon,
        vec![
            Position::by("human:dana", Pull::Against, "school run"),
            Position::by("human:sam", Pull::Toward, "fine by me"),
            Position::by("human:rae", Pull::Toward, "fine by me"),
        ],
    );
    let d = Rule::Consent.decide(&one, &attrs, &canon);
    assert_eq!(d.outcome, Outcome::Conflicts);
    assert_eq!(d.authority, Authority::Refuse, "one objection is enough");

    // And the same evidence under a threshold of two is not a conflict at
    // all — which is the whole point of policy being configuration.
    let d = Rule::Threshold { against: 2 }.decide(&one, &attrs, &canon);
    assert_eq!(d.outcome, Outcome::Supported);
    assert_eq!(d.authority, Authority::Act);

    let quiet = standing(
        &canon,
        vec![Position::of(ids[0].clone(), Pull::Toward, "serves it")],
    );
    assert_eq!(
        Rule::Consent.decide(&quiet, &attrs, &canon).authority,
        Authority::Act,
        "silence is consent"
    );
}

#[test]
fn consent_does_not_read_an_empty_canon_as_agreement() {
    // Nobody objecting is not the same as nobody having looked. The outcome
    // stays honest — nothing bears on it — and the authority says so.
    let (canon, _) = canon_with(&["a"]);
    let d = Rule::Consent.decide(&standing(&canon, vec![]), &Attributes::default(), &canon);
    assert_eq!(d.outcome, Outcome::Unaddressed);
    assert_eq!(d.authority, Authority::ActAndNotify);
}

#[test]
fn a_supermajority_counts_people_and_not_commitments() {
    let (canon, ids) = canon_with(&["a", "b"]);
    let attrs = Attributes::default();
    let two_thirds = Rule::Supermajority {
        numerator: 2,
        denominator: 3,
    };

    // Four rules pulling toward it are not four votes.
    let rules_only = standing(
        &canon,
        vec![
            Position::of(ids[0].clone(), Pull::Toward, "serves it"),
            Position::of(ids[1].clone(), Pull::Toward, "serves it"),
        ],
    );
    let d = two_thirds.decide(&rules_only, &attrs, &canon);
    assert_eq!(d.outcome, Outcome::Unaddressed, "nobody voted");

    let votes = |toward: usize, against: usize| {
        let mut v = Vec::new();
        for i in 0..toward {
            v.push(Position::by(format!("human:y{i}"), Pull::Toward, "yes"));
        }
        for i in 0..against {
            v.push(Position::by(format!("human:n{i}"), Pull::Against, "no"));
        }
        standing(&canon, v)
    };
    assert_eq!(
        two_thirds.decide(&votes(2, 1), &attrs, &canon).outcome,
        Outcome::Supported,
        "2/3 is exactly the bar and clears it"
    );
    assert_eq!(
        two_thirds.decide(&votes(3, 2), &attrs, &canon).outcome,
        Outcome::Conflicts,
        "3/5 does not"
    );
}

#[test]
fn subsidiarity_routes_to_the_deepest_holder_and_refuses_an_unheld_scope() {
    let kitchen = Scope::new("house.kitchen").unwrap();
    let acts = vec![
        Act::new(
            ActKind::Grant {
                holder: "human:alex".into(),
                scope: Scope::new("house").unwrap(),
                horizon: None,
                rationale: String::new(),
            },
            100,
            "human:alex",
        ),
        Act::new(
            ActKind::Grant {
                holder: "human:dana".into(),
                scope: kitchen.clone(),
                horizon: None,
                rationale: String::new(),
            },
            101,
            "human:alex",
        ),
    ];
    let canon = Log::from_acts(acts).derive();
    let s = standing(&canon, vec![]);

    let dana = Attributes::about("new rota")
        .by("human:dana")
        .in_scope(kitchen.clone())
        .at(200);
    // Dana holds the kitchen; alex only holds the house above it.
    let d = Rule::Subsidiarity.decide(&s, &dana, &canon);
    assert_eq!(
        d.authority,
        Authority::AskOne,
        "and nothing bears on it yet"
    );
    assert!(d.because.contains("including you"), "{}", d.because);

    let alex = dana.clone().by("human:alex");
    let d = Rule::Subsidiarity.decide(&s, &alex, &canon);
    assert!(
        d.because.contains("not you"),
        "the house grant does not beat the kitchen grant: {}",
        d.because
    );

    // A scope UNDER the house is still the house's, which is nesting doing
    // its job — the house grant covers `house.garage` and alex holds it.
    let garage = Attributes::about("x")
        .by("human:alex")
        .in_scope(Scope::new("house.garage").unwrap())
        .at(200);
    assert!(
        Rule::Subsidiarity
            .decide(&s, &garage, &canon)
            .because
            .contains("including you"),
        "a grant on `house` reaches `house.garage`"
    );

    // A boundary nobody holds refuses rather than defaulting to whoever asked.
    let garage = Attributes::about("x")
        .by("human:alex")
        .in_scope(Scope::new("allotment.shed").unwrap())
        .at(200);
    let d = Rule::Subsidiarity.decide(&s, &garage, &canon);
    assert_eq!(d.authority, Authority::Refuse);
    assert!(d.because.contains("nobody holds standing"), "{}", d.because);
}

#[test]
fn a_lapsed_grant_stops_routing_to_that_person() {
    let kitchen = Scope::new("house.kitchen").unwrap();
    let canon = Log::from_acts(vec![Act::new(
        ActKind::Grant {
            holder: "human:dana".into(),
            scope: kitchen.clone(),
            horizon: Some(150),
            rationale: String::new(),
        },
        100,
        "human:alex",
    )])
    .derive();
    let s = standing(&canon, vec![]);
    let before = Attributes::about("x")
        .by("human:dana")
        .in_scope(kitchen)
        .at(140);
    let after = before.clone().at(160);
    assert_ne!(
        Rule::Subsidiarity.decide(&s, &before, &canon).authority,
        Authority::Refuse
    );
    assert_eq!(
        Rule::Subsidiarity.decide(&s, &after, &canon).authority,
        Authority::Refuse,
        "term limits are a horizon, and they have to actually bite"
    );
}

// ── the modifiers ───────────────────────────────────────────

#[test]
fn the_ladder_climbs_on_decisions_and_starts_at_the_bottom() {
    let mut acts = Vec::new();
    let ladder = vec![Authority::AskOne, Authority::AskPanel, Authority::Refuse];
    let rule = Rule::Graduated {
        ladder: ladder.clone(),
        base: Box::new(Rule::Default),
    };
    let attrs = Attributes::about("quiet hours");

    for (i, expected) in ladder.iter().enumerate() {
        let canon = Log::from_acts(acts.clone()).derive();
        let s = standing(&canon, vec![]);
        assert_eq!(
            rule.decide(&s, &attrs, &canon).authority,
            *expected,
            "{i} prior decision(s) should be rung {}",
            i + 1
        );
        acts.push(Act::new(
            ActKind::Decided {
                about: "quiet hours".into(),
                outcome: Outcome::Conflicts,
                authority: Authority::AskOne,
                rationale: "asked once".into(),
            },
            200 + i as i64,
            "human:alex",
        ));
    }
    // And it saturates rather than running off the end.
    let canon = Log::from_acts(acts).derive();
    let s = standing(&canon, vec![]);
    assert_eq!(rule.decide(&s, &attrs, &canon).authority, Authority::Refuse);
}

#[test]
fn the_ladder_is_per_subject_and_a_community_that_never_decided_has_none() {
    let canon = Log::from_acts(vec![Act::new(
        ActKind::Decided {
            about: "quiet hours".into(),
            outcome: Outcome::Conflicts,
            authority: Authority::AskOne,
            rationale: String::new(),
        },
        200,
        "human:alex",
    )])
    .derive();
    let s = standing(&canon, vec![]);
    let rule = Rule::Graduated {
        ladder: vec![Authority::AskOne, Authority::Refuse],
        base: Box::new(Rule::Default),
    };
    assert_eq!(
        rule.decide(&s, &Attributes::about("bike storage"), &canon)
            .authority,
        Authority::AskOne,
        "a different subject starts at the bottom"
    );
    assert_eq!(canon.prior_decisions("quiet hours").len(), 1);
    assert_eq!(canon.prior_decisions("bike storage").len(), 0);
}

#[test]
fn entrenchment_raises_the_bar_for_a_principle_and_leaves_a_convention_alone() {
    let (canon, ids) = canon_with(&["never merge without review", "tabs, not spaces"]);
    let acts = vec![Act::new(
        ActKind::Rank {
            commitment: ids[0].clone(),
            rank: "principle".into(),
        },
        200,
        "human:alex",
    )];
    let mut all: Vec<Act> = Vec::new();
    all.extend(canon.commitments.iter().map(|c| {
        Act::new(
            ActKind::Assert {
                text: c.text.clone(),
                from: None,
                source: None,
            },
            c.asserted_at,
            c.actor.clone(),
        )
    }));
    all.extend(acts);
    let canon = Log::from_acts(all).derive();
    let rule = Rule::Entrenched {
        protected: vec!["principle".into()],
        base: Box::new(Rule::Consent),
    };
    let s = standing(&canon, vec![]);

    let d = rule.decide(&s, &Attributes::about("x").amending(ids[0].clone()), &canon);
    assert_eq!(d.authority, Authority::AskPanel);
    assert!(d.because.contains("entrenched"), "{}", d.because);

    let d = rule.decide(&s, &Attributes::about("x").amending(ids[1].clone()), &canon);
    assert_eq!(
        d.authority,
        Authority::ActAndNotify,
        "an unranked convention is decided under the base rule"
    );
}

#[test]
fn what_cannot_be_undone_is_not_decided_by_silence() {
    // The planted-sabotage shape: a proposal no commitment supports, whose
    // effect cannot be reversed. It lands in the one outcome that cannot
    // authorize anything, and the policy says so out loud.
    let (canon, ids) = canon_with(&["a"]);
    let rule = Rule::Cautious {
        base: Box::new(Rule::Consent),
    };
    let silent = standing(&canon, vec![]);

    let d = rule.decide(&silent, &Attributes::about("x").reversible(false), &canon);
    assert_eq!(d.outcome, Outcome::Unaddressed);
    assert_eq!(d.authority, Authority::Refuse);

    let d = rule.decide(&silent, &Attributes::about("x").reversible(true), &canon);
    assert_eq!(
        d.authority,
        Authority::ActAndNotify,
        "reversible is unchanged"
    );

    // Unknown is not "yes". Nobody said, so the cautious rule does not fire —
    // and the base rule's answer is what stands, unmodified.
    let d = rule.decide(&silent, &Attributes::about("x"), &canon);
    assert_eq!(d.authority, Authority::ActAndNotify);

    let supported = standing(
        &canon,
        vec![Position::of(ids[0].clone(), Pull::Toward, "serves it")],
    );
    let d = rule.decide(
        &supported,
        &Attributes::about("x").reversible(false),
        &canon,
    );
    assert_eq!(
        d.authority,
        Authority::ActAndNotify,
        "irreversible and supported still gets a notice"
    );
}

#[test]
fn a_modifier_can_only_make_an_answer_stricter() {
    // The one-way property the whole ladder rests on. If a wrapper could
    // soften what it wraps, "entrenched" would be a way to weaken a rule
    // rather than a way to protect one.
    assert_eq!(Authority::Act.raise(Authority::Refuse), Authority::Refuse);
    assert_eq!(Authority::Refuse.raise(Authority::Act), Authority::Refuse);
    assert!(Authority::Act < Authority::ActAndNotify);
    assert!(Authority::ActAndNotify < Authority::AskOne);
    assert!(Authority::AskOne < Authority::AskPanel);
    assert!(Authority::AskPanel < Authority::Refuse);
}

// ── policy in the ledger ────────────────────────────────────

#[test]
fn the_policy_is_in_the_canon_and_the_deepest_one_governs() {
    let house = Scope::new("house").unwrap();
    let kitchen = Scope::new("house.kitchen").unwrap();
    let canon = Log::from_acts(vec![
        Act::new(
            ActKind::Policy {
                text: "We decide by consent: silence is consent, one reasoned objection blocks."
                    .into(),
                rule: Rule::Consent,
                scope: None,
            },
            100,
            "human:alex",
        ),
        Act::new(
            ActKind::Policy {
                text: "Whoever is cooking decides the kitchen.".into(),
                rule: Rule::Subsidiarity,
                scope: Some(kitchen.clone()),
            },
            101,
            "human:alex",
        ),
    ])
    .derive();

    assert_eq!(canon.policy_for(None), &Rule::Consent);
    assert_eq!(canon.policy_for(Some(&house)), &Rule::Consent);
    assert_eq!(canon.policy_for(Some(&kitchen)), &Rule::Subsidiarity);
    assert_eq!(
        canon.policy_for(Some(&Scope::new("house.kitchen.rota").unwrap())),
        &Rule::Subsidiarity,
        "and it covers what nests under it"
    );
    // And it reads as prose, because a rule nobody can read is not contestable.
    assert!(canon
        .policy_act(Some(&kitchen))
        .is_some_and(|p| p.text.contains("cooking")));
}

#[test]
fn adopting_a_policy_replaces_the_one_for_that_scope_rather_than_stacking() {
    let canon = Log::from_acts(vec![
        Act::new(
            ActKind::Policy {
                text: "consent".into(),
                rule: Rule::Consent,
                scope: None,
            },
            100,
            "human:alex",
        ),
        Act::new(
            ActKind::Policy {
                text: "two objections".into(),
                rule: Rule::Threshold { against: 2 },
                scope: None,
            },
            200,
            "human:alex",
        ),
    ])
    .derive();
    assert_eq!(canon.policies.len(), 1, "one scope, one answer");
    assert_eq!(canon.policy_for(None), &Rule::Threshold { against: 2 });
}

#[test]
fn a_canon_that_adopted_nothing_decides_by_what_shipped() {
    let (canon, _) = canon_with(&["a"]);
    assert_eq!(canon.policy_for(None), &Rule::Default);
}

#[test]
fn a_policy_round_trips_through_the_log_with_its_rule_intact() {
    let act = Act::new(
        ActKind::Policy {
            text: "principles need the group".into(),
            rule: Rule::Entrenched {
                protected: vec!["principle".into()],
                base: Box::new(Rule::Graduated {
                    ladder: vec![Authority::AskOne, Authority::Refuse],
                    base: Box::new(Rule::Consent),
                }),
            },
            scope: None,
        },
        100,
        "human:alex",
    );
    let line = serde_json::to_string(&act).unwrap();
    let back: Act = serde_json::from_str(&line).unwrap();
    assert_eq!(back, act, "a nested rule survives the wire");
    assert_eq!(back.id, act.id, "and does not change its id");
    let ActKind::Policy { rule, .. } = &back.kind else {
        panic!("not a policy")
    };
    assert_eq!(rule.name(), "entrenched/graduated/consent");
}

#[test]
fn adopting_a_policy_is_an_adjudication_and_an_agent_doing_it_is_reported() {
    let canon = Log::from_acts(vec![Act::new(
        ActKind::Policy {
            text: "I have decided how you decide".into(),
            rule: Rule::Consent,
            scope: None,
        },
        100,
        "agent:claude",
    )])
    .derive();
    assert_eq!(
        canon.unattended.len(),
        1,
        "an agent rewriting the decision rule is exactly what `unattended` is for"
    );
}
