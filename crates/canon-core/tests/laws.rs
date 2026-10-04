// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated checks for docs/CONTRACT.md. Inputs shrink with the failing case;
//! proptest records regression seeds and accepts PROPTEST_RNG_SEED for replay.
//! These are distributional checks, not proofs or held-out measurements.

use canon_core::standing::Pull;
use canon_core::{
    Act, ActKind, Attributes, Between, Log, Policy, Position, Proposal, Ratify, Rule, Scope,
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
    let checks: [(&str, fn()); 4] = [
        ("replay_deterministic", replay_deterministic),
        ("arrival_order_independent", arrival_order_independent),
        ("merge_union_laws", merge_union_laws),
        ("agents_cannot_ratify", agents_cannot_ratify),
    ];
    let contract = include_str!("../../../docs/CONTRACT.md");
    for (name, _) in checks {
        assert!(
            contract.contains(&format!("| `{name}` |")),
            "generated law {name} is missing from docs/CONTRACT.md"
        );
    }
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

        let evidence: Vec<_> = state.commitments.iter().enumerate().map(|(i, c)| {
            Position::of(c.id.clone(), if pulls.rotate_right(i as u32) & 1 == 0 {
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
            prop_assert!(matches!(loud.get(&act.id).unwrap().status, Status::Proposed { .. }),
                         "an agent's write remains proposed under standing");
            let mut rules = vec![Ratify::Standing];
            if held {
                rules.extend([
                    Ratify::Joint { holders: vec!["human:dana".into(), "human:sam".into()] },
                    Ratify::Threshold { approve: 2, block: 1 },
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
}
