use canon_core::{Act, ActKind, Log, Scope};
use std::{fs, process::Command};

#[test]
fn executable_projects_from_files_without_rewriting_the_source() {
    let dir = std::env::temp_dir().join(format!("canon-institution-cli-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let target = Act::new(
        ActKind::Assert {
            text: "An agent's pending proposal".into(),
            from: None,
            source: None,
        },
        20,
        "agent:writer",
    );
    let basis = Log::from_acts(vec![
        Act::new(
            ActKind::Grant {
                holder: "human:owner".into(),
                scope: Scope::new("lab").unwrap(),
                horizon: None,
                rationale: String::new(),
            },
            10,
            "human:founder",
        ),
        target.clone(),
        Act::new(
            ActKind::Scoped {
                commitment: target.id.clone(),
                scope: Scope::new("lab").unwrap(),
            },
            20,
            "agent:writer",
        ),
    ]);
    let mut source = basis.acts().to_vec();
    source.push(Act::new(
        ActKind::Revert {
            targets: vec![target.id],
            rationale: "withdraw my draft".into(),
        },
        30,
        "agent:writer",
    ));
    let source = Log::from_acts(source).render();
    fs::write(dir.join("basis.jsonl"), basis.render()).unwrap();
    fs::write(dir.join("source.jsonl"), &source).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_canon-institution"))
        .args([
            "--basis",
            dir.join("basis.jsonl").to_str().unwrap(),
            "--log",
            dir.join("source.jsonl").to_str().unwrap(),
            "--at",
            "100",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let value: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(value["interpretation"], "shared-assent-v1");
    assert_eq!(value["admissions"][0]["status"], "admitted");
    assert_eq!(value["state"]["commitments"].as_array().unwrap().len(), 0);
    assert_eq!(
        fs::read_to_string(dir.join("source.jsonl")).unwrap(),
        source
    );
    let pending = basis
        .acts()
        .iter()
        .find(|a| matches!(a.kind, ActKind::Assert { .. }))
        .unwrap();
    let arrangements = canon_institution::Arrangements {
        boundary: 20,
        effects: basis
            .acts()
            .iter()
            .filter(|a| a.id != pending.id)
            .map(|a| canon_core::Effect {
                source: a.id.clone(),
                effective_at: a.ts_unix,
            })
            .collect(),
        pending: vec![pending.id.clone()],
    };
    fs::write(
        dir.join("arrangements.json"),
        serde_json::to_string(&arrangements).unwrap(),
    )
    .unwrap();
    let temporal = Command::new(env!("CARGO_BIN_EXE_canon-institution"))
        .args([
            "--arrangements",
            dir.join("arrangements.json").to_str().unwrap(),
            "--log",
            dir.join("source.jsonl").to_str().unwrap(),
            "--at",
            "100",
        ])
        .output()
        .unwrap();
    assert!(
        temporal.status.success(),
        "{}",
        String::from_utf8_lossy(&temporal.stderr)
    );
    let temporal: serde_json::Value = serde_json::from_slice(&temporal.stdout).unwrap();
    assert_eq!(temporal["interpretation"], "shared-assent-v2");
    assert_eq!(
        temporal["state"]["commitments"].as_array().unwrap().len(),
        0
    );
    assert_eq!(
        fs::read_to_string(dir.join("source.jsonl")).unwrap(),
        source
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn missing_explicit_inputs_are_could_not_judge() {
    let out = Command::new(env!("CARGO_BIN_EXE_canon-institution"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
}
