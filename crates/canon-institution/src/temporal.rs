//! Source-linked temporal admission. The fixed-basis v1 reader remains available.
use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};
use canon_core::{
    Act, ActId, ActKind, Effect, Log, Projection, Proposal, Ratify, Scope, Status, Verdict,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Explicit initial arrangements, not authority inferred from an empty record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Arrangements {
    pub boundary: i64,
    pub effects: Vec<Effect>,
    pub pending: Vec<ActId>,
}

#[derive(Debug, Serialize)]
pub struct Decision {
    source: ActId,
    status: crate::AdmissionState,
    effective_at: Option<i64>,
    requirements: Vec<Requirement>,
}
impl Decision {
    pub fn source(&self) -> &ActId {
        &self.source
    }
    pub fn status(&self) -> crate::AdmissionState {
        self.status
    }
    pub fn effective_at(&self) -> Option<i64> {
        self.effective_at
    }
}
#[derive(Debug, Serialize)]
struct Requirement {
    scope: Option<Scope>,
    rule: Ratify,
    verdict: Verdict,
}

/// Only the interpreter constructs an institutional result. Core projection is
/// mechanical data; admission and pending decisions belong to this type.
#[derive(Debug, Serialize)]
pub struct Institution {
    interpretation: &'static str,
    at: i64,
    source_sha256: String,
    arrangements_sha256: String,
    effects: Vec<Effect>,
    admissions: Vec<Decision>,
    state: Projection,
    pending: Vec<Value>,
}
impl Institution {
    pub fn projection(&self) -> &Projection {
        &self.state
    }
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }
    pub fn decisions(&self) -> &[Decision] {
        &self.admissions
    }
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).expect("institution serializes")
    }
}

struct Motion<'a> {
    act: &'a Act,
    ready_after: i64,
    rules: Vec<(Option<Scope>, Ratify)>,
    context: Vec<Effect>,
    decision: Decision,
}
fn is_birth(a: &Act) -> bool {
    matches!(a.kind, ActKind::Assert { .. } | ActKind::Supersede { .. })
}
fn source<'a>(log: &'a Log, id: &ActId) -> &'a Act {
    log.acts()
        .iter()
        .find(|a| a.id == *id)
        .expect("validated effect source")
}
fn is_evidence(a: &Act) -> bool {
    matches!(
        a.kind,
        ActKind::Position { .. }
            | ActKind::Question { .. }
            | ActKind::Annotation { .. }
            | ActKind::Key { .. }
    )
}
fn targets(a: &Act) -> &[ActId] {
    match &a.kind {
        ActKind::Retract { target, .. } => core::slice::from_ref(target),
        ActKind::Revert { targets, .. } => targets,
        _ => &[],
    }
}
fn birth_scope(log: &Log, a: &Act) -> Result<Option<Scope>, String> {
    let found: BTreeSet<_> = log
        .acts()
        .iter()
        .filter_map(|s| match &s.kind {
            ActKind::Scoped { commitment, scope }
                if commitment == &a.id && s.actor == a.actor && s.ts_unix == a.ts_unix =>
            {
                Some(scope.clone())
            }
            _ => None,
        })
        .collect();
    if found.len() > 1 {
        return Err("a proposal has conflicting initial jurisdictions".into());
    }
    Ok(found.into_iter().next())
}
fn initial_place(log: &Log, a: &Act) -> bool {
    matches!(&a.kind, ActKind::Scoped { commitment, .. } if log.acts().iter().any(|b| b.id == *commitment && is_birth(b) && b.actor == a.actor && b.ts_unix == a.ts_unix))
}
fn view(log: &Log, effects: &[Effect], at: i64) -> Result<Projection, String> {
    Projection::from_effects(log, effects, at).map_err(|e| e.to_string())
}

// Describe the actual changed objects. Repeal, undo, placement and electorate
// changes all feed this same requirement calculation, without per-verb powers.
fn changed<'a, T: PartialEq>(before: &'a [T], after: &'a [T]) -> impl Iterator<Item = &'a T> {
    before
        .iter()
        .chain(after)
        .filter(move |v| !before.contains(v) || !after.contains(v))
}

fn jurisdictions(before: &Projection, after: &Projection) -> BTreeSet<Option<Scope>> {
    let mut scopes = BTreeSet::new();
    for c in changed(before.commitments(), after.commitments()) {
        scopes.insert(
            before
                .scope_of(c.id())
                .cloned()
                .or_else(|| after.scope_of(c.id()).cloned()),
        );
    }
    for (_, scope) in changed(before.bindings(), after.bindings()) {
        scopes.insert(Some(scope.clone()));
    }
    for g in changed(before.grants(), after.grants()) {
        scopes.insert(Some(g.scope.clone()));
    }
    for r in changed(before.ratifications(), after.ratifications()) {
        scopes.insert(r.scope.clone());
    }
    for p in changed(before.policies(), after.policies()) {
        scopes.insert(p.scope.clone());
    }
    scopes
}

fn cancellation<'a>(log: &'a Log, a: &Act, effects: &[Effect]) -> Vec<&'a Act> {
    targets(a)
        .iter()
        .filter_map(|id| log.acts().iter().find(|b| b.id == *id && is_birth(b)))
        .filter(|b| !effects.iter().any(|e| e.source == b.id))
        .collect()
}

fn motion<'a>(log: &'a Log, a: &'a Act, effects: &[Effect], at: i64) -> Result<Motion<'a>, String> {
    let before = view(log, effects, at)?;
    let mut preview = effects.to_vec();
    preview.push(Effect {
        source: a.id.clone(),
        effective_at: at,
    });
    let after = view(log, &preview, at)?;
    let old = serde_json::to_value(&before).expect("projection serializes");
    let new = serde_json::to_value(&after).expect("projection serializes");
    if old.as_object().expect("projection object").keys().any(|k| {
        ![
            "commitments",
            "grants",
            "scopes",
            "ratifications",
            "policies",
        ]
        .contains(&k.as_str())
            && old[k] != new[k]
    }) {
        return Err(
            "effect changes an institutional object this profile cannot yet classify".into(),
        );
    }
    let personal_exit = matches!(&a.kind, ActKind::Withdraw { holder, .. } if holder == &a.actor);
    let cancelled = cancellation(log, a, effects);
    let personal_cancel = !cancelled.is_empty()
        && cancelled.len() == targets(a).len()
        && cancelled.iter().all(|b| b.actor == a.actor);
    let mut scopes = if personal_exit || personal_cancel {
        BTreeSet::new()
    } else {
        jurisdictions(&before, &after)
    };
    if !(personal_exit || personal_cancel) {
        for b in cancelled {
            scopes.insert(before.scope_of(&b.id).cloned().or(birth_scope(log, b)?));
        }
    }
    if scopes
        .iter()
        .any(|s| s.is_none() || before.holders_at(s.as_ref(), at).is_empty())
    {
        return Err(
            "jurisdiction has no established electorate; supply initial arrangements".into(),
        );
    }
    let rules = scopes
        .into_iter()
        .map(|scope| {
            let rule = before
                .procedure_at(scope.as_ref(), a.ts_unix)
                .cloned()
                .unwrap_or(Ratify::Standing);
            (scope, rule)
        })
        .collect();
    Ok(Motion {
        act: a,
        ready_after: at,
        rules,
        context: effects.to_vec(),
        decision: Decision {
            source: a.id.clone(),
            status: crate::AdmissionState::Proposed,
            effective_at: None,
            requirements: Vec::new(),
        },
    })
}

fn context(log: &Log, m: &Motion<'_>, effects: &[Effect], at: i64) -> Result<Projection, String> {
    let mut selected = m.context.clone();
    for e in effects {
        if selected.iter().any(|old| old.source == e.source) {
            continue;
        }
        let a = source(log, &e.source);
        // Procedures are pinned at submission. Membership changes count only
        // after their admitted effective times; an unadmitted grant never votes.
        if !matches!(
            a.kind,
            ActKind::Ratification { .. } | ActKind::Policy { .. } | ActKind::Scoped { .. }
        ) {
            selected.push(e.clone());
        }
    }
    view(log, &selected, at)
}

fn settle(
    log: &Log,
    motions: &mut [Motion<'_>],
    effects: &mut Vec<Effect>,
    at: i64,
) -> Result<(), String> {
    loop {
        let mut completed = false;
        for m in motions
            .iter_mut()
            .filter(|m| m.decision.status == crate::AdmissionState::Proposed)
        {
            let evidence = context(log, m, effects, at)?;
            let cancelled = effects.iter().any(|e| {
                evidence.is_live(&e.source) && targets(source(log, &e.source)).contains(&m.act.id)
            });
            if cancelled {
                m.ready_after = at;
                continue;
            }
            let p = Proposal {
                id: &m.act.id,
                scope: None,
                at: m.act.ts_unix,
                actor: &m.act.actor,
            };
            m.decision.requirements = m
                .rules
                .iter()
                .map(|(scope, rule)| Requirement {
                    scope: scope.clone(),
                    rule: rule.clone(),
                    verdict: evidence.ratify_under(
                        rule,
                        &Proposal {
                            scope: scope.as_ref(),
                            ..p
                        },
                        at,
                    ),
                })
                .collect();
            let (status, since) = crate::decision::resolve(
                m.decision.requirements.iter().map(|r| &r.verdict),
                m.ready_after,
            );
            m.decision.status = status;
            if let Some(since) = since {
                effects.push(Effect {
                    source: m.act.id.clone(),
                    effective_at: since,
                });
                m.decision.effective_at = Some(since);
                completed = true;
            } else if status == crate::AdmissionState::Proposed {
                // A later authority reversal must not backdate an admission
                // that this interpreter already observed could not pass.
                m.ready_after = at;
            }
        }
        if !completed {
            return Ok(());
        }
    }
}

/// Interpret from explicit initial effects. Raw records remain unchanged.
pub fn interpret(log: &Log, arrangements: &Arrangements, now: i64) -> Result<Institution, String> {
    if now < arrangements.boundary {
        return Err("view predates initial arrangements".into());
    }
    let verified = canon_core::sign::verify(log.acts());
    if !verified.unverified.is_empty() {
        return Err("source contains unverified acts".into());
    }
    let mut effects = arrangements.effects.clone();
    view(log, &effects, arrangements.boundary)?;
    let mut motions = Vec::new();
    let roots: BTreeSet<_> = effects.iter().map(|e| e.source.clone()).collect();
    if arrangements
        .pending
        .iter()
        .any(|id| roots.contains(id) || !log.acts().iter().any(|a| a.id == *id))
    {
        return Err("pending arrangement must name an unselected source record".into());
    }
    let mut remaining: Vec<_> = log
        .acts()
        .iter()
        .filter(|a| {
            !roots.contains(&a.id)
                && a.ts_unix <= now
                && (a.ts_unix > arrangements.boundary || arrangements.pending.contains(&a.id))
        })
        .collect();
    // Unselected births already pending at activation remain proposals under
    // the declared arrangements, rather than silently becoming active.
    remaining.sort_by(|a, b| {
        a.ts_unix
            .max(arrangements.boundary)
            .cmp(&b.ts_unix.max(arrangements.boundary))
            .then(a.id.cmp(&b.id))
    });
    let mut i = 0;
    while i < remaining.len() {
        let at = remaining[i].ts_unix.max(arrangements.boundary);
        settle(log, &mut motions, &mut effects, at)?;
        let end = remaining[i..]
            .iter()
            .position(|a| a.ts_unix.max(arrangements.boundary) != at)
            .map_or(remaining.len(), |n| i + n);
        for a in &remaining[i..end] {
            if is_evidence(a) || initial_place(log, a) {
                effects.push(Effect {
                    source: a.id.clone(),
                    effective_at: at,
                });
            }
        }
        let before = effects.clone();
        for a in &remaining[i..end] {
            if !(is_evidence(a) || initial_place(log, a)) {
                motions.push(motion(log, a, &before, at)?);
            }
        }
        // A still-pending proposal may be withdrawn before same-time assent
        // makes it a rule. Previously settled rules are never reopened here.
        motions.sort_by_key(|m| usize::from(targets(m.act).is_empty()));
        settle(log, &mut motions, &mut effects, at)?;
        i = end;
    }
    settle(log, &mut motions, &mut effects, now)?;
    let state = view(log, &effects, now)?;
    let mut pending = Vec::new();
    for m in &motions {
        if !is_birth(m.act) || effects.iter().any(|e| e.source == m.act.id) {
            continue;
        }
        let cancel = effects
            .iter()
            .filter(|e| state.is_live(&e.source))
            .map(|e| (source(log, &e.source), e))
            .find(|(a, _)| targets(a).contains(&m.act.id));
        if cancel.is_some_and(|(a, _)| matches!(a.kind, ActKind::Revert { .. })) {
            continue;
        }
        let status = if let Some((_, e)) = cancel {
            Status::Retracted { at: e.effective_at }
        } else if let Some((at, by, why)) =
            m.decision
                .requirements
                .iter()
                .find_map(|r| match &r.verdict {
                    Verdict::Refused { at, by, why, .. } => Some((*at, by.clone(), why.clone())),
                    _ => None,
                })
        {
            Status::Refused { at, by, why }
        } else {
            Status::Proposed {
                needs: "assent under the proposal's governing procedure".into(),
            }
        };
        let text = match &m.act.kind {
            ActKind::Assert { text, .. } | ActKind::Supersede { text, .. } => text,
            _ => unreachable!(),
        };
        pending.push(json!({"id":m.act.id, "text":text, "actor":m.act.actor, "status":status, "scope":state.scope_of(&m.act.id), "asserted_at":m.act.ts_unix}));
    }
    Ok(Institution {
        interpretation: "shared-assent-v2",
        at: now,
        source_sha256: canon_core::id::digest_hex(log.render().as_bytes()),
        arrangements_sha256: canon_core::id::digest_hex(
            serde_json::to_string(arrangements)
                .expect("arrangements serialize")
                .as_bytes(),
        ),
        effects,
        admissions: motions.into_iter().map(|m| m.decision).collect(),
        state,
        pending,
    })
}

#[cfg(test)]
mod tests;
