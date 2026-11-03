use super::*;

fn source(kind: ActKind, at: i64) -> crate::Act {
    crate::Act::new(kind, at, "agent:unseated")
}
fn effect(a: &crate::Act, at: i64) -> Effect {
    Effect {
        source: a.id.clone(),
        effective_at: at,
    }
}
fn grant() -> crate::Act {
    source(
        ActKind::Grant {
            holder: "human:guest".into(),
            scope: Scope::new("shared").unwrap(),
            horizon: None,
            rationale: String::new(),
        },
        10,
    )
}
fn assertion() -> crate::Act {
    source(
        ActKind::Assert {
            text: "A selected commitment".into(),
            from: None,
            source: None,
        },
        10,
    )
}

#[test]
fn selected_effects_apply_without_a_second_authority_decision_and_use_effective_time() {
    let a = grant();
    let log = Log::from_acts(vec![a.clone()]);
    let bytes = log.render();
    let view = Projection::from_effects(&log, &[effect(&a, 80)], 100).unwrap();
    assert!(!view.grants()[0].held_at(79));
    assert!(view.grants()[0].held_at(80));
    assert_eq!(view.grants()[0].act, a.id);
    assert_eq!(log.render(), bytes);
    assert_eq!(log.acts()[0].ts_unix, 10);
}

#[test]
fn source_birth_and_retraction_preserve_ids_without_authorship_privileges() {
    let a = assertion();
    let retract = crate::Act::new(
        ActKind::Retract {
            target: a.id.clone(),
            rationale: "selected by an interpreter".into(),
        },
        20,
        "agent:other",
    );
    let log = Log::from_acts(vec![a.clone(), retract.clone()]);
    let view =
        Projection::from_effects(&log, &[effect(&a, 30), effect(&retract, 40)], 100).unwrap();
    assert_eq!(view.get(&a.id).unwrap().asserted_at(), 10);
    assert_eq!(
        view.get(&a.id).unwrap().status(),
        &crate::Status::Retracted { at: 40 }
    );
}

#[test]
fn reversing_a_selected_reversal_restores_the_original_at_the_requested_view() {
    let a = assertion();
    let undo = source(
        ActKind::Revert {
            targets: vec![a.id.clone()],
            rationale: String::new(),
        },
        20,
    );
    let restore = source(
        ActKind::Revert {
            targets: vec![undo.id.clone()],
            rationale: String::new(),
        },
        30,
    );
    let log = Log::from_acts(vec![restore.clone(), undo.clone(), a.clone()]);
    assert!(
        Projection::from_effects(&log, &[effect(&a, 10), effect(&undo, 20)], 20)
            .unwrap()
            .get(&a.id)
            .is_none()
    );
    assert!(Projection::from_effects(
        &log,
        &[effect(&restore, 30), effect(&a, 10), effect(&undo, 20)],
        30
    )
    .unwrap()
    .get(&a.id)
    .is_some());
}

#[test]
fn invalid_selections_fail_without_returning_a_partial_view() {
    let a = assertion();
    let log = Log::from_acts(vec![a.clone()]);
    assert!(matches!(
        Projection::from_effects(&log, &[effect(&a, 9)], 100),
        Err(ProjectionError::BeforeSource(_))
    ));
    assert!(matches!(
        Projection::from_effects(&log, &[effect(&a, 101)], 100),
        Err(ProjectionError::FutureEffect(_))
    ));
    assert!(matches!(
        Projection::from_effects(&log, &[effect(&a, 10), effect(&a, 10)], 100),
        Err(ProjectionError::DuplicateSource(_))
    ));
    let absent = Effect {
        source: ActId::from_raw("absent"),
        effective_at: 10,
    };
    assert!(matches!(
        Projection::from_effects(&log, &[absent], 100),
        Err(ProjectionError::UnknownSource(_))
    ));
    assert!(serde_json::from_value::<Effect>(
        serde_json::json!({"source":a.id,"effective_at":10,"override_kind":"grant"})
    )
    .is_err());
}

#[test]
fn a_structural_effect_cannot_precede_its_selected_target_birth() {
    let a = assertion();
    let r = source(
        ActKind::Retract {
            target: a.id.clone(),
            rationale: String::new(),
        },
        20,
    );
    let log = Log::from_acts(vec![a.clone(), r.clone()]);
    assert!(matches!(
        Projection::from_effects(&log, &[effect(&a, 80), effect(&r, 30)], 100),
        Err(ProjectionError::BeforeTarget(_))
    ));
}

#[test]
fn a_recorded_procedure_only_exists_in_the_view_when_selected() {
    let r = source(
        ActKind::Ratification {
            scope: Some(Scope::new("shared").unwrap()),
            text: "Two people decide".into(),
            rule: Ratify::Joint {
                holders: vec!["human:a".into(), "human:b".into()],
            },
            governs: None,
            personal: None,
        },
        10,
    );
    let log = Log::from_acts(vec![r.clone()]);
    assert!(Projection::from_effects(&log, &[], 100)
        .unwrap()
        .procedure_at(Some(&Scope::new("shared").unwrap()), 100)
        .is_none());
    let selected = Projection::from_effects(&log, &[effect(&r, 80)], 100).unwrap();
    assert!(selected
        .procedure_at(Some(&Scope::new("shared").unwrap()), 79)
        .is_none());
    assert!(selected
        .procedure_at(Some(&Scope::new("shared").unwrap()), 80)
        .is_some());
}

#[test]
fn known_annotations_have_a_mechanical_path_without_a_policy_gate() {
    let a = source(
        ActKind::Silence {
            about: "not implemented in this seam yet".into(),
            rationale: String::new(),
        },
        10,
    );
    let log = Log::from_acts(vec![a.clone()]);
    let selected = Projection::from_effects(&log, &[effect(&a, 10)], 100).unwrap();
    let data = serde_json::to_value(&selected).unwrap();
    assert_eq!(data["silences"].as_array().unwrap().len(), 1);
}
