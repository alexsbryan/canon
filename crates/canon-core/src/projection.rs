// SPDX-License-Identifier: AGPL-3.0-or-later
//! Caller-selected mechanical effects. Selection is not proof of institutional assent.
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use serde::{Deserialize, Serialize};

use crate::{ActId, ActKind, Canon, Commitment, Grant, Log, Proposal, Ratify, Scope, Verdict};

/// A source record selected to take effect at an explicit moment. The source
/// envelope, its identifier, timestamp and signature are never rewritten.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Effect {
    pub source: ActId,
    pub effective_at: i64,
}

/// Invalid mechanical input, not a ruling about who may decide.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProjectionError {
    UnknownSource(ActId),
    UnverifiedSource(ActId),
    DuplicateSource(ActId),
    BeforeSource(ActId),
    FutureEffect(ActId),
    BeforeTarget(ActId),
}

impl core::fmt::Display for ProjectionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "invalid mechanical projection: {self:?}")
    }
}
impl core::error::Error for ProjectionError {}

/// A mechanical view, distinct from a canon's institutional verdict. Its state
/// is read-only; it cannot be extracted as a mutable or newly minted `Canon`.
#[derive(Debug)]
pub struct Projection {
    state: Canon,
    live: Vec<ActId>,
}

impl Serialize for Projection {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut value = serde_json::to_value(&self.state).map_err(serde::ser::Error::custom)?;
        let fields = value.as_object_mut().expect("projection is an object");
        for key in [
            "unattended",
            "ungoverned",
            "bootstrap",
            "pending_policies",
            "retractions",
        ] {
            fields.remove(key);
        }
        for rule in fields
            .get_mut("ratifications")
            .and_then(|v| v.as_array_mut())
            .into_iter()
            .flatten()
        {
            let at = rule["verdict"]["since"].clone();
            let fields = rule.as_object_mut().expect("procedure is an object");
            fields.remove("verdict");
            fields.remove("under");
            fields.insert("effective_at".into(), at);
        }
        value.serialize(serializer)
    }
}

impl Projection {
    pub fn is_live(&self, source: &ActId) -> bool {
        self.live.contains(source)
    }
    pub fn bindings(&self) -> &[(ActId, Scope)] {
        &self.state.scopes
    }
    pub fn ratifications(&self) -> &[crate::AdoptedRatify] {
        self.state.ratifications()
    }
    pub fn policies(&self) -> &[crate::Adopted] {
        self.state.policies()
    }
    pub fn commitments(&self) -> &[Commitment] {
        self.state.commitments()
    }
    pub fn get(&self, id: &ActId) -> Option<&Commitment> {
        self.state.get(id)
    }
    pub fn grants(&self) -> &[Grant] {
        self.state.grants()
    }
    pub fn scope_of(&self, id: &ActId) -> Option<&Scope> {
        self.state.scope_of(id)
    }
    pub fn procedure_at(&self, scope: Option<&Scope>, at: i64) -> Option<&Ratify> {
        self.state.adopted_at(scope, at).map(|r| &r.rule)
    }
    /// Reuse the procedural primitive over this explicit evidence context.
    pub fn ratify_under(&self, rule: &Ratify, p: &Proposal<'_>, now: i64) -> Verdict {
        self.state.ratify_under(rule, p, now)
    }
    pub fn holders_at(&self, scope: Option<&Scope>, at: i64) -> alloc::collections::BTreeSet<&str> {
        self.state.holders_at(scope, at)
    }
    pub fn carried(&self) -> &[(ActId, String)] {
        self.state.carried()
    }

    /// Apply only supplied effects, after verifying against the complete source.
    /// There are no authorship, standing, ratification or default-policy gates.
    pub fn from_effects(log: &Log, effects: &[Effect], now: i64) -> Result<Self, ProjectionError> {
        let sources: BTreeMap<_, _> = log.acts().iter().map(|a| (&a.id, a)).collect();
        let verified = crate::sign::verify(log.acts());
        let mut selected = BTreeMap::new();
        for e in effects {
            let a = sources
                .get(&e.source)
                .ok_or_else(|| ProjectionError::UnknownSource(e.source.clone()))?;
            if verified.unverified.iter().any(|(id, _)| id == &e.source) {
                return Err(ProjectionError::UnverifiedSource(e.source.clone()));
            }
            if e.effective_at < a.ts_unix {
                return Err(ProjectionError::BeforeSource(e.source.clone()));
            }
            if e.effective_at > now {
                return Err(ProjectionError::FutureEffect(e.source.clone()));
            }
            if selected.insert(&e.source, e).is_some() {
                return Err(ProjectionError::DuplicateSource(e.source.clone()));
            }
        }
        for e in effects {
            let targets = match &sources[&e.source].kind {
                ActKind::Supersede { old, .. } => old.as_slice(),
                ActKind::Revert { targets, .. } => targets.as_slice(),
                ActKind::Retract { target, .. } => core::slice::from_ref(target),
                _ => &[],
            };
            for target in targets {
                if !sources.contains_key(target) {
                    return Err(ProjectionError::UnknownSource(target.clone()));
                }
                if selected
                    .get(target)
                    .is_some_and(|t| t.effective_at > e.effective_at)
                {
                    return Err(ProjectionError::BeforeTarget(e.source.clone()));
                }
            }
        }
        let mut ordered: Vec<_> = effects.iter().collect();
        ordered.sort_by(|a, b| {
            a.effective_at
                .cmp(&b.effective_at)
                .then(a.source.cmp(&b.source))
        });
        let acts: Vec<_> = ordered
            .iter()
            .map(|e| (*sources[&e.source]).clone())
            .collect();
        let live = crate::effects::liveness(&acts, &vec![true; acts.len()]);
        let mut state = Canon::empty();
        // Birth before references within an equal-time batch. Later births may
        // not precede their selected structural effects (checked above).
        for (i, a) in acts.iter().enumerate().filter(|(i, _)| live[*i]) {
            if matches!(
                a.kind,
                ActKind::Assert { .. } | ActKind::Supersede { .. } | ActKind::Question { .. }
            ) {
                crate::effects::introduce(&mut state, a, ordered[i].effective_at);
            }
        }
        for (i, a) in acts.iter().enumerate().filter(|(i, _)| live[*i]) {
            crate::effects::apply_selected(&mut state, a, ordered[i].effective_at);
        }
        state.unverified = verified.unverified;
        let live = acts
            .iter()
            .enumerate()
            .filter(|(i, _)| live[*i])
            .map(|(_, a)| a.id.clone())
            .collect();
        Ok(Self { state, live })
    }
}

#[cfg(test)]
mod tests;
