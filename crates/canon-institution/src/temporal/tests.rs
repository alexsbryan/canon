use super::*;
use canon_core::{Governs, Pull};
const A: &str = "human:ada";
const B: &str = "human:ben";
const AGENT: &str = "agent:scribe";

// Fixture preparation only. The shipped interpreter requires explicit initial
// arrangements; it does not guess a community's migration or founding policy.
fn legacy_arrangements(basis: &Log, at: i64) -> Result<Arrangements, String> {
    let native = basis.derive_at(at);
    let mut effects = Vec::new();
    for a in basis.acts() {
        if native.ungoverned().iter().any(|(id, _)| id == &a.id) {
            continue;
        }
        let since = match &a.kind {
            ActKind::Assert { .. } | ActKind::Supersede { .. } => {
                native.get(&a.id).and_then(|c| match native.ratify(c, at) {
                    Verdict::Ratified { since, .. } => Some(since),
                    _ => None,
                })
            }
            ActKind::Ratification { .. } => native
                .ratifications()
                .iter()
                .find(|r| r.act == a.id)
                .and_then(|r| match r.verdict {
                    Verdict::Ratified { since, .. } => Some(since),
                    _ => None,
                }),
            _ => Some(a.ts_unix),
        };
        if let Some(effective_at) = since {
            effects.push(Effect {
                source: a.id.clone(),
                effective_at,
            });
        }
    }
    let pending = basis
        .acts()
        .iter()
        .filter(|a| {
            is_birth(a)
                && !effects.iter().any(|e| e.source == a.id)
                && native
                    .get(&a.id)
                    .is_some_and(|c| matches!(c.status(), Status::Proposed { .. }))
        })
        .map(|a| a.id.clone())
        .collect();
    Ok(Arrangements {
        boundary: at,
        effects,
        pending,
    })
}
fn act(kind: ActKind, at: i64, actor: &str) -> Act {
    Act::new(kind, at, actor)
}
fn scope() -> Scope {
    Scope::new("club").unwrap()
}
fn vote(id: &ActId, who: &str, at: i64) -> Act {
    act(
        ActKind::Position {
            about: id.to_string(),
            citing: None,
            pull: Pull::Toward,
            because: "I agree".into(),
        },
        at,
        who,
    )
}
fn seat(who: &str, at: i64, actor: &str) -> Act {
    act(
        ActKind::Grant {
            holder: who.into(),
            scope: scope(),
            horizon: None,
            rationale: String::new(),
        },
        at,
        actor,
    )
}
fn basis(adopted: bool, rule: Ratify) -> (Log, Act) {
    let proposal = act(
        ActKind::Assert {
            text: "A rule for another commons".into(),
            from: None,
            source: None,
        },
        2,
        AGENT,
    );
    let mut acts = vec![
        seat(A, 0, "human:founder"),
        seat(B, 0, "human:founder"),
        act(
            ActKind::Ratification {
                scope: Some(scope()),
                rule,
                text: "Our procedure".into(),
                governs: Some(Governs::All),
                personal: None,
            },
            1,
            A,
        ),
        proposal.clone(),
        act(
            ActKind::Scoped {
                commitment: proposal.id.clone(),
                scope: scope(),
            },
            2,
            AGENT,
        ),
    ];
    if adopted {
        acts.extend([vote(&proposal.id, A, 3), vote(&proposal.id, B, 4)]);
    }
    (Log::from_acts(acts), proposal)
}
fn joint() -> Ratify {
    Ratify::Joint {
        holders: vec![A.into(), B.into()],
    }
}
fn run(basis: &Log, extra: Vec<Act>, at: i64) -> Institution {
    let start = basis.acts().iter().map(|a| a.ts_unix).max().unwrap();
    let arrangements = legacy_arrangements(basis, start).unwrap();
    let mut source = basis.acts().to_vec();
    source.extend(extra);
    interpret(&Log::from_acts(source), &arrangements, at).unwrap()
}
fn receipt<'a>(result: &'a Institution, id: &ActId) -> &'a Decision {
    result.admissions.iter().find(|r| r.source == *id).unwrap()
}

#[test]
fn a_human_authorized_agent_grant_applies_at_assent_not_at_submission() {
    let (basis, _) = basis(true, joint());
    let request = seat("human:cy", 10, AGENT);
    let result = run(
        &basis,
        vec![
            request.clone(),
            vote(&request.id, A, 20),
            vote(&request.id, B, 30),
        ],
        40,
    );
    let grant = result
        .projection()
        .grants()
        .iter()
        .find(|g| g.actor == "human:cy")
        .unwrap();
    assert!(!grant.held_at(29));
    assert!(grant.held_at(30));
    assert_eq!(receipt(&result, &request.id).effective_at, Some(30));
}

#[test]
fn adoption_then_cancellation_in_one_history_does_not_restore_authorship_power() {
    let (basis, proposal) = basis(false, joint());
    let repeal = act(
        ActKind::Retract {
            target: proposal.id.clone(),
            rationale: "mine?".into(),
        },
        12,
        AGENT,
    );
    let result = run(
        &basis,
        vec![
            vote(&proposal.id, A, 10),
            vote(&proposal.id, B, 11),
            repeal.clone(),
        ],
        20,
    );
    assert!(result.projection().get(&proposal.id).is_some());
    assert_eq!(
        receipt(&result, &repeal.id).status(),
        crate::AdmissionState::Proposed
    );
    assert_eq!(receipt(&result, &proposal.id).effective_at, Some(11));
}

#[test]
fn an_amendment_takes_old_assent_and_does_not_lower_already_pending_requirements() {
    let (basis, _) = basis(true, joint());
    let amendment = act(
        ActKind::Ratification {
            scope: Some(scope()),
            rule: Ratify::Standing,
            text: "Delegate decisions".into(),
            governs: Some(Governs::All),
            personal: None,
        },
        10,
        A,
    );
    let old = seat("human:cy", 11, A);
    let new = seat("human:dee", 21, A);
    let result = run(
        &basis,
        vec![
            amendment.clone(),
            old.clone(),
            vote(&amendment.id, B, 20),
            new.clone(),
        ],
        25,
    );
    assert_eq!(receipt(&result, &amendment.id).effective_at, Some(20));
    assert_eq!(
        receipt(&result, &old.id).status(),
        crate::AdmissionState::Proposed
    );
    assert_eq!(
        receipt(&result, &new.id).status(),
        crate::AdmissionState::Admitted
    );
}

#[test]
fn consent_can_complete_without_another_record_and_late_objections_do_not_unmake_it() {
    let (basis, proposal) = basis(false, Ratify::Consent { days: 1 });
    let mut no = vote(&proposal.id, A, 90_000);
    no.kind = ActKind::Position {
        about: proposal.id.to_string(),
        citing: None,
        pull: Pull::Against,
        because: "late".into(),
    };
    no = Act::new(no.kind, no.ts_unix, &no.actor);
    let result = run(&basis, vec![no], 100_000);
    assert_eq!(receipt(&result, &proposal.id).effective_at, Some(86_402));
    assert!(result.projection().get(&proposal.id).is_some());
}

#[test]
fn an_unadmitted_grant_cannot_supply_its_own_voter() {
    let (basis, _) = basis(
        true,
        Ratify::Threshold {
            approve: 2,
            block: 1,
        },
    );
    let request = seat("human:cy", 10, A);
    let result = run(
        &basis,
        vec![request.clone(), vote(&request.id, "human:cy", 11)],
        20,
    );
    assert_eq!(
        receipt(&result, &request.id).status(),
        crate::AdmissionState::Proposed
    );
    assert!(!result
        .projection()
        .grants()
        .iter()
        .any(|g| g.actor == "human:cy"));
}

#[test]
fn restoring_authority_does_not_backdate_a_previously_unpassable_decision() {
    let (base, _) = basis(
        true,
        Ratify::Threshold {
            approve: 2,
            block: 1,
        },
    );
    let resigned = act(
        ActKind::Withdraw {
            holder: B.into(),
            scope: scope(),
            rationale: "leaving".into(),
        },
        5,
        B,
    );
    let mut records = base.acts().to_vec();
    records.extend([seat("human:cy", 0, "human:founder"), resigned.clone()]);
    let initial = Log::from_acts(records);
    let request = seat("human:dee", 10, AGENT);
    let restore = act(
        ActKind::Revert {
            targets: vec![resigned.id],
            rationale: "restore membership".into(),
        },
        20,
        A,
    );
    let result = run(
        &initial,
        vec![
            request.clone(),
            vote(&request.id, A, 11),
            vote(&request.id, B, 12),
            restore.clone(),
            vote(&restore.id, "human:cy", 21),
        ],
        30,
    );
    assert_eq!(receipt(&result, &restore.id).effective_at, Some(21));
    assert_eq!(receipt(&result, &request.id).effective_at, Some(21));
}

#[test]
fn unknown_or_already_selected_pending_arrangements_are_rejected() {
    let (basis, _) = basis(true, joint());
    let mut roots = legacy_arrangements(&basis, 4).unwrap();
    roots.pending.push(ActId::from_raw("unknown"));
    assert!(interpret(&basis, &roots, 10).is_err());
}

#[test]
fn strict_manifests_and_pending_serialization_do_not_silently_drop_meaning() {
    assert!(serde_json::from_value::<Arrangements>(
        json!({"boundary":0,"effects":[],"pending":[],"extra_authority":true})
    )
    .is_err());
    let (basis, proposal) = basis(false, joint());
    let result = run(&basis, Vec::new(), 5);
    let serialized = serde_json::to_value(&result).unwrap();
    assert_eq!(serialized, result.to_json());
    assert!(serialized["pending"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["id"] == proposal.id.as_str()));
    assert!(result.projection().get(&proposal.id).is_none());
}

#[test]
fn an_author_cannot_undo_a_transfer_without_assent_from_both_domains() {
    let (base, proposal) = basis(true, joint());
    let gallery = Scope::new("gallery").unwrap();
    let mut records = base.acts().to_vec();
    for who in [A, "human:cy"] {
        records.push(act(
            ActKind::Grant {
                holder: who.into(),
                scope: gallery.clone(),
                horizon: None,
                rationale: String::new(),
            },
            0,
            "human:founder",
        ));
    }
    records.push(act(
        ActKind::Ratification {
            scope: Some(gallery.clone()),
            rule: Ratify::Joint {
                holders: vec![A.into(), "human:cy".into()],
            },
            text: "Gallery procedure".into(),
            governs: None,
            personal: None,
        },
        1,
        A,
    ));
    let initial = Log::from_acts(records);
    let transfer = act(
        ActKind::Scoped {
            commitment: proposal.id.clone(),
            scope: gallery,
        },
        10,
        A,
    );
    let undo = act(
        ActKind::Revert {
            targets: vec![transfer.id.clone()],
            rationale: "reverse my transfer".into(),
        },
        20,
        A,
    );
    let prefix = vec![
        transfer.clone(),
        vote(&transfer.id, B, 11),
        vote(&transfer.id, "human:cy", 12),
        undo.clone(),
        vote(&undo.id, B, 21),
    ];
    let one = run(&initial, prefix.clone(), 22);
    assert_eq!(
        receipt(&one, &undo.id).status(),
        crate::AdmissionState::Proposed
    );
    assert_eq!(
        one.projection().scope_of(&proposal.id).unwrap().as_str(),
        "gallery"
    );
    let mut both = prefix;
    both.push(vote(&undo.id, "human:cy", 23));
    let both = run(&initial, both, 24);
    assert_eq!(
        both.projection().scope_of(&proposal.id).unwrap().as_str(),
        "club"
    );
    assert_eq!(receipt(&both, &undo.id).effective_at, Some(23));
}

#[test]
fn all_ten_portable_histories_hold_under_temporal_admission_too() {
    let capture = std::env::var_os("CANON_INSTITUTION_CAPTURE").is_some();
    let frozen: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/arrangements.json")).unwrap();
    let (mut manifests, mut reports) = (Vec::new(), Vec::new());
    let cases: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/cases.json")).unwrap();
    let histories: Vec<Value> =
        serde_json::from_str(include_str!("../../tests/fixtures/histories.json")).unwrap();
    for h in histories {
        let basis = Log::parse(h["basis"].as_str().unwrap()).unwrap();
        let start = basis.acts().iter().map(|a| a.ts_unix).max().unwrap();
        let source = Log::parse(&format!(
            "{}{}",
            h["basis"].as_str().unwrap(),
            h["appended"].as_str().unwrap()
        ))
        .unwrap();
        let now = source.acts().iter().map(|a| a.ts_unix).max().unwrap();
        let arrangements = if capture && frozen.is_empty() {
            legacy_arrangements(&basis, start).unwrap()
        } else {
            serde_json::from_value(
                frozen
                    .iter()
                    .find(|r| r["experiment"] == h["experiment"] && r["case"] == h["case"])
                    .expect("frozen initial arrangements")["arrangements"]
                    .clone(),
            )
            .unwrap()
        };
        let result = interpret(&source, &arrangements, now).unwrap().to_json();
        let want = &cases[h["experiment"].as_str().unwrap()]["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == h["case"])
            .unwrap()["expect"];
        let rule = |id: &Value| {
            result["state"]["commitments"]
                .as_array()
                .unwrap()
                .iter()
                .chain(result["pending"].as_array().unwrap())
                .find(|c| c["id"] == *id)
        };
        for (key, expected) in want.as_object().unwrap() {
            let observed =
                match key.as_str() {
                    "status" => rule(&h["target"])
                        .map_or(json!("absent"), |c| c["status"]["status"].clone()),
                    "original_status" => rule(&h["original"])
                        .map_or(json!("absent"), |c| c["status"]["status"].clone()),
                    "scope" => result["state"]["scopes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|b| b[0] == h["target"])
                        .map_or(Value::Null, |b| b[1].clone()),
                    "sam_holds" => {
                        json!(result["state"]["grants"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|g| g["actor"] == "human:Sam"
                                && g["scope"] == "courtyard"
                                && g["withdrawn_at"].is_null()))
                    }
                    _ => panic!("unassessed expectation"),
                };
            assert_eq!(
                observed, *expected,
                "{} {} {key}",
                h["experiment"], h["case"]
            );
        }
        manifests.push(
            json!({"experiment":h["experiment"], "case":h["case"], "arrangements":arrangements}),
        );
        reports.push(json!({"experiment":h["experiment"], "case":h["case"], "result":result}));
    }
    if capture {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        std::fs::write(
            root.join("arrangements.json"),
            serde_json::to_string_pretty(&manifests).unwrap() + "\n",
        )
        .unwrap();
        std::fs::write(
            root.join("temporal-results.json"),
            serde_json::to_string_pretty(&reports).unwrap() + "\n",
        )
        .unwrap();
    }
}
