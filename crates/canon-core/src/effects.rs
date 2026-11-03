// SPDX-License-Identifier: AGPL-3.0-or-later
//! Mechanical effects consume explicit admission; they never decide authority.
use alloc::{collections::BTreeMap, vec::Vec};

use crate::{Act, ActKind, Canon, Commitment, Grant, Scope, Status};

pub(crate) fn grant(
    grants: &mut Vec<Grant>,
    act: &Act,
    holder: &str,
    scope: &Scope,
    horizon: Option<i64>,
    at: i64,
) {
    for g in grants
        .iter_mut()
        .filter(|g| g.actor == holder && g.scope == *scope && g.withdrawn_at.is_none())
    {
        g.withdrawn_at = Some(at);
    }
    grants.push(Grant {
        actor: holder.into(),
        scope: scope.clone(),
        horizon,
        granted_at: at,
        withdrawn_at: None,
        act: act.id.clone(),
    });
}

pub(crate) fn withdraw(grants: &mut [Grant], holder: &str, scope: &Scope, at: i64) {
    for g in grants
        .iter_mut()
        .filter(|g| g.actor == holder && scope.covers(&g.scope) && g.withdrawn_at.is_none())
    {
        g.withdrawn_at = Some(at);
    }
}

pub(crate) fn commitment(act: &Act) -> Option<Commitment> {
    let (text, replaces, from, source) = match &act.kind {
        ActKind::Assert { text, from, source } => (text, Vec::new(), from.clone(), source.clone()),
        ActKind::Supersede { text, old, .. } => (text, old.clone(), None, None),
        _ => return None,
    };
    Some(Commitment {
        id: act.id.clone(),
        text: text.clone(),
        status: Status::Active,
        asserted_at: act.ts_unix,
        actor: act.actor.clone(),
        replaces,
        from,
        source,
    })
}

pub(crate) fn introduce(state: &mut Canon, act: &Act, at: i64) {
    if let Some(c) = commitment(act) {
        state.commitments.push(c);
    }
    if let Some(q) = question(act, at) {
        state.questions.push(q);
    }
}

pub(crate) fn question(act: &Act, at: i64) -> Option<crate::Question> {
    if let ActKind::Question { text, proposal } = &act.kind {
        return Some(crate::Question {
            id: act.id.clone(),
            text: text.clone(),
            status: Status::Active,
            asked_at: at,
            actor: act.actor.clone(),
            proposal: proposal.clone(),
        });
    }
    None
}

fn change_status(state: &mut Canon, target: &crate::ActId, status: Status) {
    if let Some(c) = state.commitments.iter_mut().find(|c| c.id == *target) {
        c.status = status;
    } else if let Some(q) = state.questions.iter_mut().find(|q| q.id == *target) {
        q.status = status;
    }
}

pub(crate) fn apply_selected(state: &mut Canon, act: &Act, at: i64) {
    match &act.kind {
        ActKind::Assert { .. }
        | ActKind::Question { .. }
        | ActKind::Revert { .. }
        | ActKind::Key { .. } => {}
        ActKind::Supersede { old, .. } => {
            for target in old {
                change_status(state, target, Status::Superseded { by: act.id.clone() });
            }
        }
        ActKind::Retract { target, .. } => change_status(state, target, Status::Retracted { at }),
        ActKind::Grant {
            holder,
            scope,
            horizon,
            ..
        } => grant(&mut state.grants, act, holder, scope, *horizon, at),
        ActKind::Withdraw { holder, scope, .. } => withdraw(&mut state.grants, holder, scope, at),
        ActKind::Scoped { commitment, scope } => {
            state.scopes.retain(|(id, _)| id != commitment);
            state.scopes.push((commitment.clone(), scope.clone()));
        }
        ActKind::Position {
            about,
            citing,
            pull,
            because,
        } => state.positions.push(crate::Stated {
            about: about.clone(),
            position: citing.as_ref().map_or_else(
                || crate::Position::by(&act.actor, *pull, because),
                |id| crate::Position::of(id.clone(), *pull, because),
            ),
            by: act.actor.clone(),
            at,
            act: act.id.clone(),
        }),
        ActKind::Ratification {
            text,
            rule,
            scope,
            governs,
            personal,
        } => {
            let under = state
                .ratification_for_at(scope.as_ref(), act.ts_unix)
                .clone();
            state.ratifications.push(crate::AdoptedRatify {
                scope: scope.clone(),
                rule: rule.clone(),
                text: text.clone(),
                at: act.ts_unix,
                actor: act.actor.clone(),
                act: act.id.clone(),
                under,
                governs: *governs,
                personal: *personal,
                verdict: crate::Verdict::Ratified {
                    since: at,
                    how: "caller-selected mechanical effect; no assent certified".into(),
                },
            });
        }
        ActKind::Policy { text, rule, scope } => {
            state.policies.retain(|p| p.scope != *scope);
            state.policies.push(crate::Adopted {
                scope: scope.clone(),
                rule: rule.clone(),
                text: text.clone(),
                at,
                actor: act.actor.clone(),
                act: act.id.clone(),
                verdict: None,
            });
        }
        ActKind::Adopt {
            lineage,
            generation,
            source,
        } => {
            state.ancestry = Some(crate::Ancestry {
                lineage: lineage.clone(),
                generation: generation.clone(),
                source: source.clone(),
                at,
            })
        }
        ActKind::Silence { about, rationale } => {
            state.silences.retain(|s| s.about != *about);
            state.silences.push(crate::Silence {
                about: about.clone(),
                rationale: rationale.clone(),
                at,
                actor: act.actor.clone(),
                act: act.id.clone(),
            });
        }
        ActKind::Rank { commitment, rank } => {
            state.ranks.retain(|(id, _)| id != commitment);
            state.ranks.push((commitment.clone(), rank.clone()));
        }
        ActKind::Horizon {
            target,
            at: due,
            rationale,
        } => {
            state.horizons.retain(|h| h.target != *target);
            state.horizons.push(crate::Horizon {
                target: target.clone(),
                at: *due,
                rationale: rationale.clone(),
                set_at: at,
                act: act.id.clone(),
            });
        }
        ActKind::DrawCommit {
            scope,
            count,
            after_ts,
            rationale,
        } => state.draws.push(crate::draw::Committed {
            act: act.id.clone(),
            scope: scope.clone(),
            count: *count,
            after_ts: *after_ts,
            at,
            drawer: act.actor.clone(),
            rationale: rationale.clone(),
        }),
        ActKind::DrawSecret { commit, digest } => {
            if !state
                .sealed
                .iter()
                .any(|s| s.commit == *commit && s.actor == act.actor)
            {
                state.sealed.push(crate::draw::Sealed {
                    commit: commit.clone(),
                    actor: act.actor.clone(),
                    digest: digest.clone(),
                    at,
                });
            }
        }
        ActKind::DrawReveal { commit, secret } => {
            if !state
                .opened
                .iter()
                .any(|s| s.commit == *commit && s.actor == act.actor)
            {
                state.opened.push(crate::draw::Opened {
                    commit: commit.clone(),
                    actor: act.actor.clone(),
                    secret: secret.clone(),
                    at,
                });
            }
        }
        ActKind::Allot {
            text,
            unit,
            units,
            scope,
        } => {
            state.allotments.retain(|a| a.scope != *scope);
            state.allotments.push(crate::Allotment {
                scope: scope.clone(),
                unit: unit.clone(),
                units: units.clone(),
                text: text.clone(),
                at,
                actor: act.actor.clone(),
                act: act.id.clone(),
            });
        }
        ActKind::Allocation { text, rule, scope } => {
            state.allocations.push(crate::AdoptedAllocation {
                scope: scope.clone(),
                rule: rule.clone(),
                text: text.clone(),
                at,
                actor: act.actor.clone(),
                act: act.id.clone(),
            })
        }
        ActKind::Decided {
            about,
            outcome,
            authority,
            rationale,
            citing,
        } => state.rulings.push(crate::Ruling {
            about: about.clone(),
            outcome: *outcome,
            authority: *authority,
            rationale: rationale.clone(),
            at,
            actor: act.actor.clone(),
            act: act.id.clone(),
            citing: citing.clone(),
        }),
        ActKind::Accept {
            a,
            b,
            rationale,
            revisit,
        } => state.conflicts.push(crate::Conflict {
            a: a.clone(),
            b: b.clone(),
            at,
            disposition: crate::Disposition::Tolerated {
                rationale: rationale.clone(),
                revisit: revisit.clone(),
            },
        }),
        ActKind::Dismiss { a, b, rationale } => state.conflicts.push(crate::Conflict {
            a: a.clone(),
            b: b.clone(),
            at,
            disposition: crate::Disposition::Dismissed {
                rationale: rationale.clone(),
            },
        }),
        ActKind::Annotation { kind, .. } => state.carried.push((act.id.clone(), kind.clone())),
    }
}

/// An act is dead iff a live, admitted reversal targets it. References, not file
/// position, determine the result: a same-second undo can sort before its target.
pub(crate) fn liveness(acts: &[Act], admitted_reversals: &[bool]) -> Vec<bool> {
    assert_eq!(acts.len(), admitted_reversals.len());
    let mut reverters: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (j, act) in acts
        .iter()
        .enumerate()
        .filter(|(j, _)| admitted_reversals[*j])
    {
        if let ActKind::Revert { targets, .. } = &act.kind {
            for target in targets {
                reverters.entry(target.as_str()).or_default().push(j);
            }
        }
    }
    fn resolve(
        i: usize,
        acts: &[Act],
        reverters: &BTreeMap<&str, Vec<usize>>,
        mark: &mut [Option<bool>],
    ) -> bool {
        if let Some(alive) = mark[i] {
            return alive;
        }
        // Provisional true preserves the old fail-live result on malformed
        // cycles. Content-addressed valid histories cannot contain such cycles.
        mark[i] = Some(true);
        let alive = !reverters
            .get(acts[i].id.as_str())
            .is_some_and(|js| js.iter().any(|&j| resolve(j, acts, reverters, mark)));
        mark[i] = Some(alive);
        alive
    }
    let mut mark = vec![None; acts.len()];
    (0..acts.len())
        .map(|i| resolve(i, acts, &reverters, &mut mark))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::String;

    fn assertion() -> Act {
        Act::new(
            ActKind::Assert {
                text: "A rule".into(),
                from: None,
                source: None,
            },
            1,
            "agent:writer",
        )
    }
    fn undo(target: &Act) -> Act {
        Act::new(
            ActKind::Revert {
                targets: vec![target.id.clone()],
                rationale: String::new(),
            },
            1,
            "agent:other",
        )
    }

    #[test]
    fn mechanical_reversal_uses_the_supplied_admission_not_actor_standing() {
        let target = assertion();
        let reversal = undo(&target);
        let acts = [target, reversal];
        assert_eq!(liveness(&acts, &[true, false]), vec![true, true]);
        assert_eq!(liveness(&acts, &[true, true]), vec![false, true]);
    }

    #[test]
    fn reversing_a_reversal_reapplies_the_original() {
        let target = assertion();
        let reversal = undo(&target);
        let undo_undo = undo(&reversal);
        assert_eq!(
            liveness(&[target, reversal, undo_undo], &[true; 3]),
            vec![true, false, true]
        );
    }

    #[test]
    fn reference_order_not_arrival_order_decides_same_second_reversals() {
        let target = assertion();
        let reversal = undo(&target);
        assert_eq!(liveness(&[reversal, target], &[true; 2]), vec![true, false]);
    }

    #[test]
    #[should_panic]
    fn a_mask_for_a_different_history_is_a_bug() {
        liveness(&[assertion()], &[]);
    }
}
