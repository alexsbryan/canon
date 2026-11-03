use super::*;
use canon_core::{Governs, Pull, Ratify};

const ADA: &str = "human:ada";
const BEN: &str = "human:ben";
const SCRIBE: &str = "agent:scribe";

fn scope(s: &str) -> Scope {
    Scope::new(s).unwrap()
}
fn act(kind: ActKind, at: i64, actor: &str) -> Act {
    Act::new(kind, at, actor)
}
fn grant(holder: &str, domain: &str) -> Act {
    act(
        ActKind::Grant {
            holder: holder.into(),
            scope: scope(domain),
            horizon: None,
            rationale: String::new(),
        },
        10,
        "human:founder",
    )
}
fn passes(domain: &str, rule: Ratify) -> Act {
    act(
        ActKind::Ratification {
            text: "A different institution's procedure".into(),
            rule,
            scope: Some(scope(domain)),
            governs: Some(Governs::All),
            personal: None,
        },
        20,
        ADA,
    )
}
fn vote(id: &ActId, who: &str, at: i64) -> Act {
    act(
        ActKind::Position {
            about: id.to_string(),
            citing: None,
            pull: Pull::Toward,
            because: "I assent to this change".into(),
        },
        at,
        who,
    )
}
fn established(adopted: bool, rule: Ratify) -> (Log, Act) {
    let target = act(
        ActKind::Assert {
            text: "The observatory shares its telescope fairly.".into(),
            from: None,
            source: None,
        },
        30,
        SCRIBE,
    );
    let mut acts = vec![
        grant(ADA, "observatory"),
        grant(BEN, "observatory"),
        grant(ADA, "archive"),
        passes("observatory", rule),
        passes("archive", Ratify::Standing),
        target.clone(),
        act(
            ActKind::Scoped {
                commitment: target.id.clone(),
                scope: scope("observatory"),
            },
            30,
            SCRIBE,
        ),
    ];
    if adopted {
        acts.extend([vote(&target.id, ADA, 40), vote(&target.id, BEN, 50)]);
    }
    (Log::from_acts(acts), target)
}
fn joint() -> Ratify {
    Ratify::Joint {
        holders: vec![ADA.into(), BEN.into()],
    }
}
fn removal(target: &Act, undo: bool) -> Act {
    act(
        if undo {
            ActKind::Revert {
                targets: vec![target.id.clone()],
                rationale: "changed my mind".into(),
            }
        } else {
            ActKind::Retract {
                target: target.id.clone(),
                rationale: "changed my mind".into(),
            }
        },
        60,
        SCRIBE,
    )
}
fn run(basis: &Log, extra: Vec<Act>, at: i64) -> Value {
    let mut source = basis.acts().to_vec();
    source.extend(extra);
    project(&basis.render(), &Log::from_acts(source).render(), at).unwrap()
}
fn state_status(result: &Value, id: &ActId) -> String {
    result["state"]["commitments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id.as_str())
        .map_or("absent", |c| c["status"]["status"].as_str().unwrap())
        .into()
}

#[test]
fn authorship_cancels_pending_but_does_not_remove_an_adopted_rule_by_either_route() {
    for undo in [false, true] {
        for adopted in [false, true] {
            let (basis, target) = established(adopted, joint());
            let result = run(&basis, vec![removal(&target, undo)], 100);
            assert_eq!(
                result["admissions"][0]["status"],
                if adopted { "proposed" } else { "admitted" }
            );
            assert_eq!(
                state_status(&result, &target.id),
                if adopted {
                    "active"
                } else if undo {
                    "absent"
                } else {
                    "retracted"
                }
            );
        }
    }
}

#[test]
fn all_four_protected_effects_use_assent_and_can_be_admitted_when_people_actually_agree() {
    let (basis, target) = established(true, joint());
    let changes = [
        removal(&target, false),
        removal(&target, true),
        act(
            ActKind::Withdraw {
                holder: BEN.into(),
                scope: scope("observatory"),
                rationale: "new delegate".into(),
            },
            60,
            ADA,
        ),
        act(
            ActKind::Scoped {
                commitment: target.id.clone(),
                scope: scope("archive"),
            },
            60,
            ADA,
        ),
    ];
    for change in changes {
        let denied = run(&basis, vec![change.clone()], 100);
        assert_eq!(denied["admissions"][0]["status"], "proposed");
        let agreed = run(
            &basis,
            vec![
                change.clone(),
                vote(&change.id, ADA, 70),
                vote(&change.id, BEN, 80),
            ],
            100,
        );
        assert_eq!(agreed["admissions"][0]["status"], "admitted");
        assert!(agreed["admitted_log"]
            .as_str()
            .unwrap()
            .contains(change.id.as_str()));
        match change.kind {
            ActKind::Retract { .. } => assert_eq!(state_status(&agreed, &target.id), "retracted"),
            ActKind::Revert { .. } => assert_eq!(state_status(&agreed, &target.id), "absent"),
            ActKind::Withdraw { .. } => assert!(agreed["state"]["grants"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["actor"] == BEN && g["withdrawn_at"] == 60)),
            ActKind::Scoped { .. } => assert!(agreed["state"]["commitments"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c["id"] == target.id.as_str() && c["scope"] == "archive")),
            _ => unreachable!(),
        }
    }
}

#[test]
fn a_transfer_requires_both_the_departing_and_receiving_jurisdictions() {
    let (base, target) = established(true, joint());
    let mut acts: Vec<_> = base.acts().iter().filter(|a| !matches!(&a.kind, ActKind::Ratification { scope: Some(s), .. } if s.as_str() == "archive")).cloned().collect();
    acts.extend([
        grant("human:cy", "archive"),
        passes("archive", Ratify::Unanimous),
    ]);
    let basis = Log::from_acts(acts);
    let change = act(
        ActKind::Scoped {
            commitment: target.id.clone(),
            scope: scope("archive"),
        },
        60,
        ADA,
    );
    let one = run(&basis, vec![change.clone(), vote(&change.id, BEN, 70)], 100);
    assert_eq!(one["admissions"][0]["status"], "proposed");
    assert_eq!(
        one["admissions"][0]["requirements"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let both = run(
        &basis,
        vec![
            change.clone(),
            vote(&change.id, BEN, 70),
            vote(&change.id, "human:cy", 80),
        ],
        100,
    );
    assert_eq!(both["admissions"][0]["status"], "admitted");
}

#[test]
fn a_requested_grant_cannot_supply_its_own_new_voter_for_another_request() {
    let (basis, _) = established(
        true,
        Ratify::Threshold {
            approve: 2,
            block: 1,
        },
    );
    let new_seat = act(
        ActKind::Grant {
            holder: "human:cy".into(),
            scope: scope("observatory"),
            horizon: None,
            rationale: String::new(),
        },
        60,
        ADA,
    );
    let removal = act(
        ActKind::Withdraw {
            holder: BEN.into(),
            scope: scope("observatory"),
            rationale: String::new(),
        },
        60,
        ADA,
    );
    let result = run(
        &basis,
        vec![new_seat, removal.clone(), vote(&removal.id, "human:cy", 70)],
        100,
    );
    assert!(result["admissions"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| a["status"] == "proposed"));
}

#[test]
fn future_approvals_do_not_authorize_a_present_change() {
    let (basis, target) = established(true, joint());
    let change = removal(&target, false);
    let result = run(
        &basis,
        vec![
            change.clone(),
            vote(&change.id, ADA, 101),
            vote(&change.id, BEN, 102),
        ],
        100,
    );
    assert_eq!(result["admissions"][0]["status"], "proposed");
    assert_eq!(state_status(&result, &target.id), "active");
}

#[test]
fn unsupported_effects_and_missing_basis_are_errors_not_partial_success() {
    let (basis, _) = established(true, joint());
    let mut source = basis.acts().to_vec();
    source.push(act(
        ActKind::Silence {
            about: "everything".into(),
            rationale: "unsupported in this experiment".into(),
        },
        60,
        ADA,
    ));
    assert!(project(&basis.render(), &Log::from_acts(source).render(), 100).is_err());
    assert!(project(&basis.render(), "", 100).is_err());
}

#[test]
fn input_arrival_order_does_not_change_the_admission_or_projection() {
    let (basis, target) = established(true, joint());
    let change = removal(&target, false);
    let mut source = basis.acts().to_vec();
    source.extend([
        change.clone(),
        vote(&change.id, ADA, 70),
        vote(&change.id, BEN, 80),
    ]);
    let canonical = Log::from_acts(source.clone()).render();
    source.reverse();
    let reordered = source
        .iter()
        .map(|a| serde_json::to_string(a).unwrap() + "\n")
        .collect::<String>();
    let before = canonical.clone();
    let a = project(&basis.render(), &canonical, 100).unwrap();
    let b = project(&basis.render(), &reordered, 100).unwrap();
    assert_eq!(a["state"], b["state"]);
    assert_eq!(a["admissions"], b["admissions"]);
    assert_eq!(before, canonical);
}

#[test]
fn a_native_fold_limit_is_reported_rather_than_calling_an_unapplied_grant_effective() {
    let (basis, _) = established(true, joint());
    let change = act(
        ActKind::Grant {
            holder: "human:cy".into(),
            scope: scope("observatory"),
            horizon: None,
            rationale: String::new(),
        },
        60,
        SCRIBE,
    );
    let mut source = basis.acts().to_vec();
    source.extend([
        change.clone(),
        vote(&change.id, ADA, 70),
        vote(&change.id, BEN, 80),
    ]);
    let error = project(&basis.render(), &Log::from_acts(source).render(), 100).unwrap_err();
    assert!(error.contains("lower projection seam"));
}

#[test]
fn resigning_does_not_authorize_restoring_your_own_seat_by_undo() {
    let (base, _) = established(true, joint());
    let resignation = act(
        ActKind::Withdraw {
            holder: BEN.into(),
            scope: scope("observatory"),
            rationale: "I resign".into(),
        },
        55,
        BEN,
    );
    let mut acts = base.acts().to_vec();
    acts.push(resignation.clone());
    let basis = Log::from_acts(acts);
    let restore = act(
        ActKind::Revert {
            targets: vec![resignation.id],
            rationale: "I want my seat back".into(),
        },
        60,
        BEN,
    );
    let result = run(&basis, vec![restore], 100);
    assert_eq!(result["admissions"][0]["status"], "proposed");
}

#[test]
fn an_adoption_and_its_cancellation_cannot_be_smuggled_into_one_fixed_basis_round() {
    let (basis, target) = established(false, joint());
    let mut source = basis.acts().to_vec();
    source.extend([
        vote(&target.id, ADA, 60),
        vote(&target.id, BEN, 61),
        act(
            ActKind::Retract {
                target: target.id,
                rationale: "still mine?".into(),
            },
            62,
            SCRIBE,
        ),
    ]);
    assert!(project(&basis.render(), &Log::from_acts(source).render(), 100).is_err());
}

#[test]
fn approvals_cannot_switch_the_round_to_a_different_constitution() {
    let (base, _) = established(true, joint());
    let amendment = act(
        ActKind::Ratification {
            text: "One delegate may decide".into(),
            rule: Ratify::Standing,
            scope: Some(scope("observatory")),
            governs: Some(Governs::All),
            personal: None,
        },
        55,
        ADA,
    );
    let mut acts = base.acts().to_vec();
    acts.push(amendment.clone());
    let basis = Log::from_acts(acts);
    let remove = act(
        ActKind::Withdraw {
            holder: BEN.into(),
            scope: scope("observatory"),
            rationale: String::new(),
        },
        70,
        ADA,
    );
    let mut source = basis.acts().to_vec();
    source.extend([vote(&amendment.id, BEN, 60), remove]);
    assert!(project(&basis.render(), &Log::from_acts(source).render(), 100).is_err());
}
