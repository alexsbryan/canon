// SPDX-License-Identifier: AGPL-3.0-or-later
//! What code decides about a change before a model is asked anything.
//!
//! **A model was asked what the record already said.** On 2026-10-23 a fork
//! of commonwealth-ai was rebased onto a family canon that holds most of the
//! same rules word for word, and the model called four retractions of rules
//! the family canon holds verbatim "orphaned — the rule this was about is
//! gone from the new base". Carried out, that verdict drops the retraction
//! and puts the rule back in force without anyone deciding to. Whether a
//! rule is in the new base word for word, or was withdrawn there too, is a
//! lookup, so it is answered here and only the rest goes to the model.
//!
//! Only identical text counts as the same rule. Provenance (the same id, or
//! a base rule whose `from` is the rule this canon inherited) picks which of
//! two identical texts a change lands on, and says when the base withdrew a
//! rule itself; a reworded rule is a judgement, and stays the model's.

use canon_core::Status;

use super::{ActId, Base, Basis, Change, Landed, Verdict};

/// The changes code decided, each with its verdict, and the ones it left for
/// the model, in the order they came.
pub(super) fn settle(changes: &[Change], base: &Base) -> (Vec<Landed>, Vec<Change>) {
    let mut decided = Vec::new();
    let mut rest = Vec::new();
    for c in changes {
        match decide(&c.basis, base) {
            Some((verdict, target, because)) => decided.push(Landed {
                change: c.clone(),
                verdict,
                target,
                because: because.to_string(),
            }),
            None => rest.push(c.clone()),
        }
    }
    (decided, rest)
}

/// A verdict, the 1-based target rule it lands on, and why, for a change the
/// new base settles by itself. `None` when it takes judgement.
fn decide(basis: &Basis, base: &Base) -> Option<(Verdict, Option<usize>, &'static str)> {
    const HOLDS: &str = "decided in code: the new base holds this rule word for word.";
    match basis {
        Basis::Retracted { upstream, text } => match holds(base, Some(upstream), text) {
            Some(k) => Some((Verdict::Carries, Some(k), HOLDS)),
            None if withdrew(base, upstream) => Some((
                Verdict::Already,
                None,
                "decided in code: the new base withdrew this rule too.",
            )),
            None => None,
        },
        Basis::Superseded {
            upstream,
            text,
            now,
        } => match (holds(base, Some(upstream), text), holds(base, None, now)) {
            // The base holds the rule as it was, and not its replacement:
            // the supersession lands on it unchanged.
            (Some(k), None) => Some((Verdict::Carries, Some(k), HOLDS)),
            (None, Some(k)) => Some((
                Verdict::Already,
                Some(k),
                "decided in code: the new base already says this word for word.",
            )),
            // Both, or neither: which one the change is about is a judgement.
            _ => None,
        },
        Basis::Accepted { upstream, text } => {
            holds(base, Some(upstream), text).map(|k| (Verdict::Carries, Some(k), HOLDS))
        }
        Basis::Added { text } => holds(base, None, text).map(|k| {
            (
                Verdict::Already,
                Some(k),
                "decided in code: the new base already says this word for word.",
            )
        }),
    }
}

/// The 1-based number of the base rule whose text is `text`, preferring the
/// one whose provenance is `upstream` when the text appears more than once.
fn holds(base: &Base, upstream: Option<&ActId>, text: &str) -> Option<usize> {
    let same: Vec<usize> = base
        .snapshot
        .commitments
        .iter()
        .enumerate()
        .filter(|(_, c)| c.text == text)
        .map(|(i, _)| i)
        .collect();
    let traced = upstream.and_then(|u| {
        same.iter().copied().find(|&i| {
            let id = &base.snapshot.commitments[i].id;
            id == u || base.log.and_then(|l| l.get(id)).and_then(|c| c.from()) == Some(u)
        })
    });
    traced.or(same.first().copied()).map(|i| i + 1)
}

/// Did the base retract the rule this canon inherited as `upstream`? Only a
/// base that came with its log can say; a pasted one cannot.
fn withdrew(base: &Base, upstream: &ActId) -> bool {
    base.log.is_some_and(|l| {
        l.commitments().iter().any(|c| {
            (c.id() == upstream || c.from() == Some(upstream))
                && matches!(c.status(), Status::Retracted { .. })
        })
    })
}
