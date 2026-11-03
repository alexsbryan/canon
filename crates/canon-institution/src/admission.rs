// SPDX-License-Identifier: AGPL-3.0-or-later
use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use canon_core::{Act, ActId, ActKind, Canon, Log, Proposal, Scope, Status, Verdict};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Serialize)]
struct Requirement {
    scope: Scope,
    verdict: Verdict,
}

#[derive(Debug, Serialize)]
struct Admission {
    act: ActId,
    status: crate::AdmissionState,
    because: &'static str,
    requirements: Vec<Requirement>,
}

fn scope_of(canon: &Canon, target: &ActId) -> Result<Scope, String> {
    canon
        .scope_of(target)
        .cloned()
        .ok_or_else(|| format!("unscoped target {target}"))
}

// Operation dispatch describes effects. It does not decide quorum or permissions.
fn effects(
    act: &Act,
    target: &Act,
    basis: &Canon,
    original: &BTreeMap<ActId, &Act>,
    walking: &mut BTreeSet<ActId>,
    inverse: bool,
) -> Result<BTreeSet<Scope>, String> {
    let mut scopes = BTreeSet::new();
    match &target.kind {
        ActKind::Assert { .. } | ActKind::Supersede { .. } => {
            let c = basis
                .get(&target.id)
                .ok_or_else(|| format!("unknown commitment {}", target.id))?;
            if !(c.actor() == act.actor && matches!(c.status(), Status::Proposed { .. })) {
                scopes.insert(scope_of(basis, &target.id)?);
            }
        }
        ActKind::Retract { target: id, .. } => {
            if act.id == target.id && !inverse {
                let source = original
                    .get(id)
                    .ok_or_else(|| format!("unknown removal target {id}"))?;
                return effects(act, source, basis, original, walking, false);
            }
            scopes.insert(scope_of(basis, id)?);
        }
        ActKind::Revert { targets, .. } => {
            if !walking.insert(target.id.clone()) {
                return Err("cyclic reversal targets".into());
            }
            for id in targets {
                let t = original
                    .get(id)
                    .ok_or_else(|| format!("unknown reversal target {id}"))?;
                scopes.extend(effects(act, t, basis, original, walking, !inverse)?);
            }
            walking.remove(&target.id);
        }
        ActKind::Grant { holder, scope, .. } | ActKind::Withdraw { holder, scope, .. } => {
            let withdrawal = matches!(target.kind, ActKind::Withdraw { .. });
            if !((withdrawal != inverse) && holder == &act.actor) {
                scopes.insert(scope.clone());
                if withdrawal {
                    scopes.extend(
                        basis
                            .grants()
                            .iter()
                            .filter(|g| g.actor == *holder && scope.covers(&g.scope))
                            .map(|g| g.scope.clone()),
                    );
                }
            }
        }
        ActKind::Scoped { commitment, scope } => {
            if inverse {
                return Err("undo of a placement needs its pre-act jurisdiction; unsupported in this fixed-basis round".into());
            }
            scopes.insert(scope_of(basis, commitment)?);
            scopes.insert(scope.clone());
        }
        _ => return Err(format!("unsupported effect {}", target.id)),
    }
    Ok(scopes)
}

fn admit(
    act: &Act,
    scopes: BTreeSet<Scope>,
    basis: &Canon,
    evidence: &Canon,
    at: i64,
) -> Admission {
    let requirements: Vec<_> = scopes
        .into_iter()
        .map(|scope| {
            let rule = basis.ratification_for_at(Some(&scope), act.ts_unix);
            let verdict = evidence.ratify_under(
                rule,
                &Proposal {
                    id: &act.id,
                    scope: Some(&scope),
                    at: act.ts_unix,
                    actor: &act.actor,
                },
                at,
            );
            Requirement { scope, verdict }
        })
        .collect();
    let status = crate::decision::resolve(requirements.iter().map(|r| &r.verdict), act.ts_unix).0;
    Admission {
        act: act.id.clone(),
        status,
        because: if requirements.is_empty() {
            "own pending proposal or own resignation"
        } else {
            "assent under every affected jurisdiction's established procedure"
        },
        requirements,
    }
}

pub fn project(basis_text: &str, source_text: &str, at: i64) -> Result<Value, String> {
    let basis = Log::parse(basis_text).map_err(|e| e.to_string())?;
    let source = Log::parse(source_text).map_err(|e| e.to_string())?;
    let original: BTreeMap<_, _> = basis.acts().iter().map(|a| (a.id.clone(), a)).collect();
    for act in basis.acts() {
        if act.ts_unix > at || !source.acts().iter().any(|a| a == act) {
            return Err(
                "source must extend the complete basis, and basis must not be in the future".into(),
            );
        }
    }
    // Verify against the complete source before any institutional filtering.
    let unverified = canon_core::sign::verify(source.acts()).unverified.len();
    if unverified != 0 {
        return Err(format!(
            "{unverified} act(s) failed the record's signature checks"
        ));
    }
    let requests: Vec<_> = source
        .acts()
        .iter()
        .filter(|a| !original.contains_key(&a.id) && a.ts_unix <= at)
        .collect();
    let latest_basis = basis
        .acts()
        .iter()
        .map(|a| a.ts_unix)
        .max()
        .unwrap_or(i64::MIN);
    if requests.iter().any(|a| a.ts_unix < latest_basis) {
        return Err("a new request predates the activation basis".into());
    }
    let established = basis.derive_at(at);
    let mut voting = basis.acts().to_vec();
    voting.extend(
        requests
            .iter()
            .filter(|a| matches!(a.kind, ActKind::Position { .. }))
            .map(|a| (*a).clone()),
    );
    let evidence = Log::from_acts(voting).derive_at(at);
    if evidence.ratifications() != established.ratifications()
        || evidence.grants() != established.grants()
    {
        return Err("new evidence changes the basis's constitution or electorate; establish a fresh basis first".into());
    }
    let changes_phase = established.commitments().iter().any(|c| {
        evidence.get(c.id()).is_some_and(|e| {
            core::mem::discriminant(c.status()) != core::mem::discriminant(e.status())
        })
    });
    let mut admitted = basis.acts().to_vec();
    let mut admissions = Vec::new();
    for act in requests {
        if matches!(
            act.kind,
            ActKind::Position { .. } | ActKind::Annotation { .. }
        ) {
            admitted.push(act.clone());
            continue;
        }
        if changes_phase {
            return Err("assent changes an existing rule's phase in the same round as a mutation; establish a fresh basis first".into());
        }
        let scopes = effects(
            act,
            act,
            &established,
            &original,
            &mut BTreeSet::new(),
            false,
        )?;
        let decision = admit(act, scopes, &established, &evidence, at);
        if decision.status == crate::AdmissionState::Admitted {
            admitted.push(act.clone());
        }
        admissions.push(decision);
    }
    let admitted = Log::from_acts(admitted);
    let projected = admitted.derive_at(at);
    if !projected.unverified().is_empty() {
        return Err("filtering changed signature/reference verification; this needs a lower projection seam".into());
    }
    let accepted: BTreeSet<_> = admissions
        .iter()
        .filter(|a| a.status == crate::AdmissionState::Admitted)
        .map(|a| &a.act)
        .collect();
    if projected
        .ungoverned()
        .iter()
        .any(|(id, _)| accepted.contains(id))
    {
        return Err("an admitted request was rejected by canon's native fold; this needs a lower projection seam".into());
    }
    let mut state = serde_json::to_value(&projected).map_err(|e| e.to_string())?;
    state["commitments"] = json!(projected
        .commitments()
        .iter()
        .map(|c| json!({
            "id": c.id(), "text": c.text(), "status": c.status(), "actor": c.actor(),
            "scope": projected.scope_of(c.id()),
        }))
        .collect::<Vec<_>>());
    Ok(json!({
        "interpretation": "shared-assent-v1", "boundary": "fixed-basis admission round", "at": at,
        "basis_sha256": canon_core::id::digest_hex(basis_text.as_bytes()),
        "source_sha256": canon_core::id::digest_hex(source_text.as_bytes()),
        "admissions": admissions, "state": state, "admitted_log": admitted.render(),
        "future_acts": source.acts().iter().filter(|a| a.ts_unix > at).map(|a| &a.id).collect::<Vec<_>>(),
        "limits": ["fixed electorate and procedures for this round", "current snapshot, not historical activation times", "unsupported request kinds refuse the round"],
    }))
}

#[cfg(test)]
mod tests;
