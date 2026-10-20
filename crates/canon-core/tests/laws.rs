// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated checks for docs/CONTRACT.md. Inputs shrink with the failing case;
//! proptest records regression seeds and accepts PROPTEST_RNG_SEED for replay.
//! These are distributional checks, not proofs or held-out measurements.

use canon_core::standing::Pull;
use canon_core::{
    Act, ActKind, Attributes, Bar, Between, Log, Policy, Position, Proposal, Ratify, Rule, Scope,
    Standing, Status, Verdict,
};
use proptest::prelude::*;

/// References always point to earlier generated acts. Equal timestamps, agent
/// proposals, stale horizons, and references to replaced rules are intentional.
fn histories() -> impl Strategy<Value = Vec<Act>> {
    proptest::collection::vec((0u8..8, any::<usize>(), 0i64..3, any::<bool>()), 1..25).prop_map(
        |steps| {
            let mut acts: Vec<Act> = Vec::new();
            let mut rules: Vec<canon_core::ActId> = Vec::new();
            let mut at = 100;
            for (i, (op, target, tick, human)) in steps.into_iter().enumerate() {
                at += tick;
                let actor = if human { "human:sam" } else { "agent:reader" };
                let kind = if rules.is_empty() || op == 0 {
                    ActKind::Assert {
                        text: format!("agreement {i}"),
                        from: None,
                        source: None,
                    }
                } else {
                    let rule = rules[target % rules.len()].clone();
                    match op {
                        1 => ActKind::Supersede {
                            text: format!("replacement {i}"),
                            old: vec![rule],
                            rationale: "changed circumstances".into(),
                        },
                        2 => ActKind::Retract {
                            target: rule,
                            rationale: "no longer needed".into(),
                        },
                        3 => ActKind::Revert {
                            targets: vec![acts[target % acts.len()].id.clone()],
                            rationale: "correct the record".into(),
                        },
                        4 => ActKind::Question {
                            text: format!("question {i}"),
                            proposal: None,
                        },
                        5 => ActKind::Silence {
                            about: format!("subject {i}"),
                            rationale: "leave room for judgement".into(),
                        },
                        6 => ActKind::Horizon {
                            target: rule,
                            at: at + tick,
                            rationale: "review it".into(),
                        },
                        _ => ActKind::Position {
                            about: rule.as_str().into(),
                            citing: None,
                            pull: if human { Pull::Toward } else { Pull::Against },
                            because: "a stated reason".into(),
                        },
                    }
                };
                let is_rule = matches!(kind, ActKind::Assert { .. } | ActKind::Supersede { .. });
                let act = Act::new(kind, at, actor);
                if is_rule {
                    rules.push(act.id.clone());
                }
                acts.push(act);
            }
            acts
        },
    )
}

#[test]
fn generated_laws_are_indexed_by_the_contract() {
    // Function references make a stale/renamed check a compile error; the row
    // check makes removing its guarantee from the index an explicit change.
    let checks: [(&str, fn()); 5] = [
        ("replay_deterministic", replay_deterministic),
        ("arrival_order_independent", arrival_order_independent),
        ("merge_union_laws", merge_union_laws),
        ("agents_cannot_ratify", agents_cannot_ratify),
        ("share_bars_are_their_rules", share_bars_are_their_rules),
    ];
    let contract = include_str!("../../../docs/CONTRACT.md");
    for (name, _) in checks {
        assert!(
            contract.contains(&format!("| `{name}` |")),
            "generated law {name} is missing from docs/CONTRACT.md"
        );
    }
}

/// The moments a house's people come, go and speak, around a proposal written
/// at 100: before it, in its second, after it, and either side of the day a
/// `twice` waits.
const SHARE_TIMES: [i64; 9] = [10, 100, 101, 150, 200, 86_600, 86_700, 90_000, 172_900];

/// A house of up to eight people around one proposal at 100. Each is granted
/// `house` (now and then `garden`, another top-level scope) at a generated
/// moment, some with a term that lapses, some moving out; holders, a
/// stranger and an agent take positions for and against, with and without a
/// reason, often in the same second. The proposer is a holder or the agent,
/// and the proposal is put in `house` or left for the whole canon.
fn share_histories() -> impl Strategy<Value = (Vec<Act>, Act, bool)> {
    let person = (
        0usize..9,
        proptest::option::of(0usize..9),
        proptest::option::of(0usize..9),
        0u8..10,
    );
    let said = (0usize..10, 0usize..9, any::<bool>(), any::<bool>());
    (
        proptest::collection::vec(person, 1..9),
        proptest::collection::vec(said, 0..16),
        0usize..9,
        any::<bool>(),
    )
        .prop_map(|(people, positions, proposer, scoped)| {
            let house = Scope::new("house").unwrap();
            let garden = Scope::new("garden").unwrap();
            let name = |i: usize| format!("human:p{i}");
            let mut acts = Vec::new();
            for (i, (granted, lapses, leaves, seat)) in people.iter().enumerate() {
                let scope = if *seat == 0 { &garden } else { &house };
                acts.push(Act::new(
                    ActKind::Grant {
                        holder: name(i),
                        scope: scope.clone(),
                        horizon: lapses.map(|t| SHARE_TIMES[t]),
                        rationale: String::new(),
                    },
                    SHARE_TIMES[*granted],
                    "human:founder",
                ));
                if let Some(t) = leaves {
                    acts.push(Act::new(
                        ActKind::Withdraw {
                            holder: name(i),
                            scope: scope.clone(),
                            rationale: "moving out".into(),
                        },
                        SHARE_TIMES[*t],
                        name(i),
                    ));
                }
            }
            let speaker = |i: usize| match i {
                i if i < people.len() => name(i),
                8 => "agent:reader".into(),
                _ => "human:stranger".into(),
            };
            let writer = speaker(if proposer < people.len() { proposer } else { 8 });
            let proposal = Act::new(
                ActKind::Assert {
                    text: "a proposed agreement".into(),
                    from: None,
                    source: None,
                },
                100,
                &writer,
            );
            if scoped {
                acts.push(Act::new(
                    ActKind::Scoped {
                        commitment: proposal.id.clone(),
                        scope: house,
                    },
                    100,
                    &writer,
                ));
            }
            for (who, at, toward, reasoned) in positions {
                acts.push(Act::new(
                    ActKind::Position {
                        about: proposal.id.as_str().into(),
                        citing: None,
                        pull: if toward { Pull::Toward } else { Pull::Against },
                        because: if reasoned {
                            format!("{} has a reason", speaker(who))
                        } else {
                            String::new()
                        },
                    },
                    SHARE_TIMES[at],
                    speaker(who),
                ));
            }
            acts.push(proposal.clone());
            (acts, proposal, scoped)
        })
}

fn union(a: &Log, b: &Log) -> Log {
    Log::from_acts(a.acts().iter().chain(b.acts()).cloned().collect())
}

fn grant(holder: &str, scope: &Scope) -> Act {
    Act::new(
        ActKind::Grant {
            holder: holder.into(),
            scope: scope.clone(),
            horizon: None,
            rationale: "founding seats".into(),
        },
        100,
        "human:founder",
    )
}

fn vote(id: &canon_core::ActId, actor: &str, at: i64, pull: Pull) -> Act {
    Act::new(
        ActKind::Position {
            about: id.as_str().into(),
            citing: None,
            pull,
            because: "a reason for this position".into(),
        },
        at,
        actor,
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn replay_deterministic(acts in histories(), clock in 0i64..200, pulls in any::<u64>()) {
        let log = Log::from_acts(acts);
        let now = 100 + clock;
        let restored = Log::parse(&log.render()).unwrap();
        let state = log.derive_at(now);
        let again = restored.derive_at(now);
        prop_assert_eq!(&state, &again);
        prop_assert_eq!(log.render(), restored.render());
        prop_assert_eq!(state.overdue(now), again.overdue(now));

        let evidence: Vec<_> = state.commitments().iter().enumerate().map(|(i, c)| {
            Position::of(c.id().clone(), if pulls.rotate_right(i as u32) & 1 == 0 {
                Pull::Toward
            } else {
                Pull::Against
            }, "generated evidence")
        }).collect();
        let (standing, refused) = Standing::cited(&state, "proposal", evidence);
        prop_assert!(refused.is_empty());
        let attrs = Attributes { now, ..Attributes::about("proposal") };
        for policy in [Rule::Default, Rule::Consent, Rule::Threshold { against: 2 }, Rule::Subsidiarity] {
            prop_assert_eq!(policy.decide(&standing, &attrs, &state),
                            policy.decide(&standing, &attrs, &again));
        }
    }

    #[test]
    fn arrival_order_independent(
        acts in histories(),
        keys in proptest::collection::vec(any::<u64>(), 1..40),
        clock in 0i64..200,
    ) {
        let expected = Log::from_acts(acts.clone());
        let mut shuffled: Vec<_> = acts.into_iter().enumerate().collect();
        shuffled.sort_by_key(|(i, _)| (keys[i % keys.len()], *i));
        let actual = Log::from_acts(shuffled.into_iter().map(|(_, act)| act).collect());
        prop_assert_eq!(expected.derive_at(100 + clock), actual.derive_at(100 + clock));
        prop_assert_eq!(expected.render(), actual.render());
    }

    #[test]
    fn merge_union_laws(
        acts in histories(), other in histories(),
        membership in proptest::collection::vec(any::<bool>(), 1..40),
        clock in 0i64..200,
    ) {
        // Every act reaches at least one branch. Some reach both; one branch
        // also receives exact duplicates. A third history exercises associativity.
        let left = Log::from_acts(acts.iter().enumerate()
            .filter(|(i, _)| membership[i % membership.len()] || i % 3 == 0)
            .flat_map(|(_, a)| [a.clone(), a.clone()]).collect());
        let right = Log::from_acts(acts.iter().enumerate()
            .filter(|(i, _)| !membership[i % membership.len()] || i % 3 == 0)
            .map(|(_, a)| a.clone()).collect());
        let third = Log::from_acts(other);
        let expected = Log::from_acts(acts);
        let merged = union(&left, &right);
        prop_assert_eq!(merged.render(), expected.render());
        prop_assert_eq!(merged.derive_at(100 + clock), expected.derive_at(100 + clock));
        prop_assert_eq!(merged.render(), union(&right, &left).render());
        prop_assert_eq!(merged.render(), union(&merged, &merged).render());
        let first = union(&union(&left, &right), &third);
        let second = union(&left, &union(&right, &third));
        prop_assert_eq!(first.render(), second.render());
        prop_assert_eq!(first.derive_at(100 + clock), second.derive_at(100 + clock));
    }

    #[test]
    fn agents_cannot_ratify(
        suffix in "[a-z]{1,8}",
        ballots in proptest::collection::vec(any::<bool>(), 1..9),
        days in 1u32..4,
    ) {
        let scope = Scope::new("house.kitchen").unwrap();
        let proposer = format!("agent:{suffix}");
        let act = Act::new(ActKind::Assert {
            text: "a proposed agreement".into(), from: None, source: None,
        }, 200, &proposer);
        let proposal = Proposal { id: &act.id, scope: Some(&scope), at: 200, actor: &proposer };
        let now = 200 + i64::from(days) * 86_400 + 20;

        for held in [false, true] {
            let mut base = vec![act.clone(), Act::new(ActKind::Scoped {
                commitment: act.id.clone(), scope: scope.clone(),
            }, 200, &proposer)];
            if held {
                base.extend([grant("human:dana", &scope), grant("human:sam", &scope),
                             grant(&proposer, &scope)]);
            }
            let mut noisy = base.clone();
            for (i, toward) in ballots.iter().enumerate() {
                let actor = format!("agent:{suffix}-{i}");
                if held { noisy.push(grant(&actor, &scope)); }
                noisy.push(vote(&act.id, &actor, 201 + i as i64,
                    if *toward { Pull::Toward } else { Pull::Against }));
            }
            // Self-approval must not count either, even if the agent holds a seat.
            noisy.push(vote(&act.id, &proposer, 210, Pull::Toward));
            let quiet = Log::from_acts(base.clone()).derive_at(now);
            let loud = Log::from_acts(noisy.clone()).derive_at(now);
            prop_assert!(matches!(loud.get(&act.id).unwrap().status(), Status::Proposed { .. }),
                         "an agent's write remains proposed under standing");
            let mut rules = vec![Ratify::Standing];
            if held {
                rules.extend([
                    Ratify::Joint { holders: vec!["human:dana".into(), "human:sam".into()] },
                    Ratify::Threshold { approve: 2, block: 1 },
                    Ratify::Majority,
                    Ratify::Unanimous,
                    Ratify::Share(Bar::new(3, 5, false).unwrap()),
                    Ratify::Consent { days },
                    Ratify::Twice { each: Box::new(Ratify::Threshold { approve: 1, block: 1 }),
                                    between: Between::Days { days } },
                ]);
            }
            for rule in rules {
                prop_assert_eq!(quiet.ratify_under(&rule, &proposal, now),
                                loud.ratify_under(&rule, &proposal, now));
                let mut human = noisy.clone();
                if matches!(rule, Ratify::Consent { .. }) {
                    // Consent can carry by time alone: that is the group's rule,
                    // not an agent's approval. A timely human objection blocks it.
                    prop_assert!(matches!(loud.ratify_under(&rule, &proposal, now), Verdict::Ratified { .. }),
                                 "consent carries after its waiting period");
                    human.push(vote(&act.id, "human:dana", 201, Pull::Against));
                    prop_assert!(matches!(Log::from_acts(human).derive_at(now)
                        .ratify_under(&rule, &proposal, now), Verdict::Refused { .. }),
                        "a timely human objection refuses consent");
                } else {
                    prop_assert!(matches!(loud.ratify_under(&rule, &proposal, now), Verdict::Proposed { .. }),
                                 "agents cannot supply the missing approvals");
                    human.extend([vote(&act.id, "human:dana", 201, Pull::Toward),
                                  vote(&act.id, "human:sam", 202, Pull::Toward),
                                  vote(&act.id, "human:dana", now - 1, Pull::Toward)]);
                    prop_assert!(matches!(Log::from_acts(human).derive_at(now)
                        .ratify_under(&rule, &proposal, now), Verdict::Ratified { .. }),
                        "human holders can still carry the proposal");
                }
            }
        }
    }

    #[test]
    fn share_bars_are_their_rules(history in share_histories()) {
        // `majority` is the share more than half and `unanimous` the share
        // of everyone. Held as a share — as `Bar` builds it, or as a record
        // writes it another way — each is judged as the rule it equals:
        // the same verdict, moment and words, alone and under twice, and
        // the same fold for a house that adopts either.
        let (acts, proposal, scoped) = history;
        let now = 200_000;
        let house = Scope::new("house").unwrap();
        let canon = Log::from_acts(acts.clone()).derive_at(now);
        let p = Proposal {
            id: &proposal.id,
            scope: scoped.then_some(&house),
            at: 100,
            actor: &proposal.actor,
        };
        let written = |line: &str| serde_json::from_str::<Ratify>(line).unwrap();
        let pairs = [
            (Ratify::Majority, Ratify::Share(Bar::MAJORITY)),
            (Ratify::Majority, written(r#"{"rule":"share","numerator":2,"denominator":4,"more_than":true}"#)),
            (Ratify::Unanimous, Ratify::Share(Bar::EVERYONE)),
            (Ratify::Unanimous, written(r#"{"rule":"share","numerator":3,"denominator":3,"more_than":false}"#)),
        ];
        let adopted = |rule: Ratify| {
            let mut acts = acts.clone();
            acts.push(grant("human:founder", &house));
            acts.push(Act::new(ActKind::Ratification {
                text: String::new(), rule, scope: None, governs: None, personal: None,
            }, 50, "human:founder"));
            let canon = Log::from_acts(acts).derive_at(now);
            let c = canon.get(&proposal.id).unwrap();
            (c.status().clone(), canon.ratify(c, now))
        };
        for (named, share) in pairs {
            prop_assert_eq!(named.name(), share.name());
            for between in [None, Some(Between::Days { days: 1 }), Some(Between::Turnover)] {
                let under = |rule: &Ratify| match &between {
                    None => rule.clone(),
                    Some(b) => Ratify::Twice { each: Box::new(rule.clone()), between: b.clone() },
                };
                prop_assert_eq!(canon.ratify_under(&under(&named), &p, now),
                                canon.ratify_under(&under(&share), &p, now));
            }
            prop_assert_eq!(adopted(named), adopted(share));
        }
    }
}
