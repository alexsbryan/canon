// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explaining one commitment — the single implementation.
//!
//! The CLI's `why` and the MCP `canon_why` tool render the same explanation;
//! before this module they carried a copy each, and the copies had already
//! drifted into the same defect (§10.6 — one decider, one name).

use canon_core::{ActId, ActKind, Canon, Disposition, Log, Status};

use crate::store;

/// One line of an explanation. The caller supplies the prefix so a CLI can
/// indent and an MCP payload need not.
pub struct Explanation {
    pub headline: String,
    pub lines: Vec<String>,
}

impl Explanation {
    pub fn render(&self, indent: &str) -> String {
        let mut out = self.headline.clone();
        out.push('\n');
        for l in &self.lines {
            out.push_str(indent);
            out.push_str(l);
            out.push('\n');
        }
        out
    }
}

/// Resolve a commitment by id or unique prefix.
///
/// An ambiguous prefix is an error rather than a guess: silently picking the
/// first match would attribute a decision to the wrong rule.
pub fn resolve(canon: &Canon, needle: &str) -> Result<ActId, String> {
    let hits: Vec<&ActId> = canon
        .commitments
        .iter()
        .map(|c| &c.id)
        .filter(|id| id.as_str() == needle || id.as_str().starts_with(needle))
        .collect();
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(format!("no commitment matching `{needle}`")),
        n => Err(format!(
            "`{needle}` matches {n} commitments — use more characters"
        )),
    }
}

/// Resolve a commitment OR a question by id or unique prefix.
///
/// `supersede` and `retract` take either, because answering a question is
/// superseding it and withdrawing one is retracting it — the same two acts,
/// not a second vocabulary.
pub fn resolve_any(canon: &Canon, needle: &str) -> Result<ActId, String> {
    let hits: Vec<&ActId> = canon
        .commitments
        .iter()
        .map(|c| &c.id)
        .chain(canon.questions.iter().map(|q| &q.id))
        .filter(|id| id.as_str() == needle || id.as_str().starts_with(needle))
        .collect();
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(format!("nothing matching `{needle}`")),
        n => Err(format!(
            "`{needle}` matches {n} records — use more characters"
        )),
    }
}

pub fn explain(log: &Log, canon: &Canon, id: &ActId) -> Result<Explanation, String> {
    if let Some(q) = canon.question(id) {
        return Ok(explain_question(canon, q));
    }
    let c = canon
        .get(id)
        .ok_or_else(|| format!("no commitment `{id}`"))?;

    let headline = format!("{}  {}", c.id, c.text);
    let mut lines = vec![format!(
        "asserted {} by {}",
        store::ymd(c.asserted_at),
        c.actor
    )];

    if let Some(src) = &c.source {
        lines.push(format!("drafted from {src}"));
    }
    if let Some(up) = &c.from {
        lines.push(format!("inherited from upstream {up}"));
    }

    // Why this commitment is HERE. Its introducing act is the act whose id it
    // carries, so a commitment born from a supersession finds its rationale
    // there. Looking only at acts that *retired* it — which both copies of
    // this code did — answers a different question and leaves the common case
    // silent.
    for act in log.acts() {
        if act.id != *id {
            continue;
        }
        if let ActKind::Supersede { rationale, .. } = &act.kind {
            if !rationale.is_empty() {
                lines.push(format!("reason for the change: {rationale}"));
            }
        }
    }

    for old in &c.replaces {
        let prev = canon
            .get(old)
            .map(|p| p.text.as_str())
            .unwrap_or("(unknown)");
        lines.push(format!("replaced {old}: \"{prev}\""));
    }

    match &c.status {
        Status::Active => lines.push("status: in force".into()),
        Status::Superseded { by } => {
            let next = canon
                .get(by)
                .map(|p| p.text.as_str())
                .unwrap_or("(unknown)");
            lines.push(format!("status: SUPERSEDED by {by} — \"{next}\""));
        }
        Status::Retracted { at } => lines.push(format!("status: RETRACTED {}", store::ymd(*at))),
    }

    // Why it stopped being in force, if it did.
    for act in log.acts() {
        match &act.kind {
            ActKind::Supersede { old, rationale, .. }
                if old.contains(id) && !rationale.is_empty() =>
            {
                lines.push(format!("reason it was replaced: {rationale}"))
            }
            ActKind::Retract { target, rationale } if target == id && !rationale.is_empty() => {
                lines.push(format!("reason it was retracted: {rationale}"))
            }
            _ => {}
        }
    }

    for conflict in canon.conflicts.iter().filter(|x| x.a == *id || x.b == *id) {
        let other = if conflict.a == *id {
            &conflict.b
        } else {
            &conflict.a
        };
        match &conflict.disposition {
            Disposition::Tolerated { rationale, revisit } => {
                lines.push(format!("carried against {other}: {rationale}"));
                if let Some(r) = revisit {
                    lines.push(format!("  revisit by {r}"));
                }
            }
            Disposition::Dismissed { rationale } => {
                let why = if rationale.is_empty() {
                    "detector noise"
                } else {
                    rationale
                };
                lines.push(format!("not in conflict with {other}: {why}"));
            }
            Disposition::Open { reason } => {
                lines.push(format!("open tension with {other}: {reason}"))
            }
        }
    }

    Ok(Explanation { headline, lines })
}

fn explain_question(canon: &Canon, q: &canon_core::Question) -> Explanation {
    let mut lines = vec![format!("asked {} by {}", store::ymd(q.asked_at), q.actor)];
    if let Some(p) = &q.proposal {
        lines.push(format!("surfaced by the proposal: \"{p}\""));
    }
    match &q.status {
        Status::Active => {
            lines.push("status: OPEN — the canon does not cover this".into());
            lines.push(format!(
                "answer it:  canon supersede {} \"<the rule>\" -m \"<reason>\"",
                q.id
            ));
        }
        Status::Superseded { by } => {
            let text = canon
                .get(by)
                .map(|c| c.text.as_str())
                .unwrap_or("(unknown)");
            lines.push(format!("status: ANSWERED by {by} — \"{text}\""));
        }
        Status::Retracted { at } => lines.push(format!("status: WITHDRAWN {}", store::ymd(*at))),
    }
    Explanation {
        headline: format!("{}  ? {}", q.id, q.text),
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use canon_core::Act;

    fn asserted(text: &str, ts: i64) -> Act {
        Act::new(
            ActKind::Assert {
                text: text.into(),
                from: None,
                source: None,
            },
            ts,
            "human:alex",
        )
    }

    #[test]
    fn a_superseding_commitment_reports_the_reason_it_exists() {
        // Found by driving the real MCP server: `why` on a rule born from a
        // supersession showed what it replaced but never the rationale,
        // because both copies searched for acts that RETIRED it.
        let old = asserted("quiet hours at 11", 100);
        let new = Act::new(
            ActKind::Supersede {
                text: "quiet hours at 10 on weeknights".into(),
                old: vec![old.id.clone()],
                rationale: "house meeting 2026-02-10".into(),
            },
            200,
            "human:priya",
        );
        let log = Log::from_acts(vec![old, new.clone()]);
        let canon = log.derive();

        let e = explain(&log, &canon, &new.id).unwrap();
        assert!(
            e.lines
                .iter()
                .any(|l| l.contains("house meeting 2026-02-10")),
            "the reason the current rule exists must be shown: {:?}",
            e.lines
        );
    }

    #[test]
    fn a_superseded_commitment_reports_the_reason_it_was_replaced() {
        let old = asserted("quiet hours at 11", 100);
        let new = Act::new(
            ActKind::Supersede {
                text: "quiet hours at 10".into(),
                old: vec![old.id.clone()],
                rationale: "house meeting".into(),
            },
            200,
            "human:priya",
        );
        let log = Log::from_acts(vec![old.clone(), new]);
        let canon = log.derive();

        let e = explain(&log, &canon, &old.id).unwrap();
        assert!(e.lines.iter().any(|l| l.contains("SUPERSEDED")));
        assert!(e
            .lines
            .iter()
            .any(|l| l.contains("reason it was replaced: house meeting")));
    }

    #[test]
    fn an_ambiguous_prefix_is_an_error_not_a_guess() {
        let a = asserted("one", 100);
        let b = asserted("two", 110);
        let canon = Log::from_acts(vec![a, b]).derive();
        assert!(resolve(&canon, "can-").is_err());
        assert!(resolve(&canon, "can-zzzzzz").is_err());
    }
}
