// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explaining one commitment — the single implementation.
//!
//! The CLI's `why` and the MCP `canon_why` tool render the same explanation;
//! before this module they carried a copy each, and the copies had already
//! drifted into the same defect (§10.6 — one decider, one name).

use canon_core::{ActId, ActKind, Canon, Disposition, Log, Status};

use crate::store;

/// What an id names — a rule, or the question a rule answered.
///
/// **Answering a question is superseding it**, which is the whole reason this
/// format has no separate `answer` op. So a commitment's `replaces` can name
/// a question as easily as a rule, and rendering the second as "(unknown)"
/// hides exactly the link that makes the answer make sense: the gap somebody
/// noticed, and the rule they wrote to close it.
/// One line of a text, cut at a word if it runs long. A `why` that quotes
/// a whole paragraph to name a rule has stopped being about the rule.
pub fn one_line(text: &str) -> String {
    const MAX: usize = 72;
    if text.chars().count() <= MAX {
        return text.to_string();
    }
    let cut: String = text.chars().take(MAX).collect();
    let at = cut.rfind(' ').unwrap_or(cut.len());
    format!("{}…", cut[..at].trim_end())
}

/// A commitment or question named the way a person names it: by what it
/// says, with the id after so the next command has something to type.
/// Every surface that used to print a bare id goes through here.
pub fn named(canon: &Canon, id: &ActId) -> String {
    if let Some(c) = canon.get(id) {
        return format!("\"{}\"  {id}", one_line(c.text()));
    }
    match canon.questions().iter().find(|q| q.id == *id) {
        Some(q) => format!("the question \"{}\"  {id}", one_line(&q.text)),
        None => format!("{id}  (not in this canon)"),
    }
}

/// A person by name. `human:` is the format's business, not the reader's;
/// an agent keeps its prefix because "agent:helper" is the fact.
pub fn person(actor: &str) -> &str {
    actor
        .strip_prefix(canon_core::person::PREFIX)
        .unwrap_or(actor)
}

/// What follows "refused" on a line that says so: " by Kai", then ", <date>"
/// when the line dates it. A refusal nobody made alone — a share of the
/// house that can no longer be reached, a count of objections — has no
/// author, and gets only the date: "refused by :" named nobody as though
/// they were somebody. `shown` is how the surface writes a name.
pub fn refused_by(by: &str, at: Option<i64>, shown: impl Fn(&str) -> &str) -> String {
    let when = at.map(store::ymd);
    match (by.is_empty(), when) {
        (true, None) => String::new(),
        (true, Some(when)) => format!(" {when}"),
        (false, None) => format!(" by {}", shown(by)),
        (false, Some(when)) => format!(" by {}, {when}", shown(by)),
    }
}

/// Why it was refused, as read. With an author it is their own words. With
/// none it is the fold's sentence, which names people by id, and they read
/// by name like everywhere else.
pub fn refused_why(by: &str, why: &str) -> String {
    if by.is_empty() {
        person_in(why)
    } else {
        why.to_string()
    }
}

/// One line of an explanation. The caller supplies the prefix so a CLI can
/// indent and an MCP payload need not.
pub struct Explanation {
    pub headline: String,
    pub lines: Vec<String>,
}

impl Explanation {
    pub fn render(&self, indent: &str) -> String {
        // The headline is `<id>  <text>`; wrap the text under itself.
        let mut out = match self.headline.split_once("  ") {
            Some((id, text)) => crate::wrap::hang(&format!("{id}  "), text),
            None => self.headline.clone(),
        };
        out.push('\n');
        for l in &self.lines {
            // A line that starts with spaces is a sub-line of the one above
            // it — who ruled, under what was ruled. `hang` splits on
            // whitespace, so the lead has to ride in the prefix to survive.
            let lead = l.len() - l.trim_start().len();
            let prefix = format!("{indent}{}", " ".repeat(lead));
            out.push_str(&crate::wrap::hang(&prefix, l.trim_start()));
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
    pick(
        canon,
        canon.commitments().iter().map(|c| c.id()),
        needle,
        "commitment",
    )
}

/// Resolve a commitment OR a question by id or unique prefix.
///
/// `supersede` and `retract` take either, because answering a question is
/// superseding it and withdrawing one is retracting it — the same two acts,
/// not a second vocabulary.
pub fn resolve_any(canon: &Canon, needle: &str) -> Result<ActId, String> {
    let ids = canon
        .commitments()
        .iter()
        .map(|c| c.id())
        .chain(canon.questions().iter().map(|q| &q.id));
    pick(canon, ids, needle, "record")
}

/// Resolve anything `why` can explain: any act the log holds. A silence was
/// the first non-rule, non-question id that answered "no record matching"
/// (2026-10-28); the house audit then hit the same wall with a ratification
/// and a grant (2026-10-29), so the log is the list now.
pub fn resolve_why(canon: &Canon, log: &Log, needle: &str) -> Result<ActId, String> {
    let mut ids: Vec<&ActId> = log.acts().iter().map(|a| &a.id).collect();
    for id in canon
        .commitments()
        .iter()
        .map(|c| c.id())
        .chain(canon.questions().iter().map(|q| &q.id))
        .chain(canon.silences().iter().map(|s| &s.act))
    {
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    pick_acts(canon, ids.into_iter(), needle)
}

fn pick_acts<'a>(
    canon: &'a Canon,
    ids: impl Iterator<Item = &'a ActId>,
    needle: &str,
) -> Result<ActId, String> {
    let matches = |id: &ActId| id.as_str() == needle || id.as_str().starts_with(needle);
    let mut hits: Vec<&ActId> = ids.filter(|id| matches(id)).collect();
    // An adopted rule is known by its upstream id, which has no local act.
    if hits.is_empty() {
        hits = canon
            .commitments()
            .iter()
            .filter(|c| c.from().is_some_and(matches))
            .map(|c| c.id())
            .collect();
    }
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(format!("no record matching `{needle}`")),
        n => Err(format!(
            "`{needle}` matches {n} records — use more characters"
        )),
    }
}

/// Resolve a question by id or unique prefix.
///
/// `second` takes only a question: weight added to a commitment would be a
/// position on a proposal, which ratification counts, and an unattributed one
/// must never be.
pub fn resolve_question(canon: &Canon, needle: &str) -> Result<ActId, String> {
    pick(
        canon,
        canon.questions().iter().map(|q| &q.id),
        needle,
        "question",
    )
}

/// Resolve anything a person can approve or object to: a commitment, a
/// change to how a scope decides, or a retraction written without standing.
///
/// `list` and every write print `canon approve <id>` under each of the
/// three, so all three have to resolve here — a ratification change used to
/// answer its own hint with "no commitment matching". A change of policy
/// waiting under `governs: all` prints the same hint and resolves here too.
pub fn resolve_proposal(canon: &Canon, needle: &str) -> Result<ActId, String> {
    let ids = canon
        .commitments()
        .iter()
        .map(|c| c.id())
        .chain(canon.ratifications().iter().map(|r| &r.act))
        .chain(canon.retractions().iter().map(|r| &r.act))
        .chain(canon.pending_policies().iter().map(|p| &p.act));
    pick(canon, ids, needle, "proposal")
}

/// One id out of `ids` by id or unique prefix, else the commitment inherited
/// from the upstream id `needle` names.
///
/// **An adopted canon is read in its seed's ids.** Each inherited commitment
/// gets a local id and keeps the upstream one only in `from`, so retracting
/// the seed's rules by the ids its maintainers use meant resolving each one
/// through `list --json` first. Upstream is tried only when nothing local
/// matches, so a local id always means itself.
fn pick<'a>(
    canon: &'a Canon,
    ids: impl Iterator<Item = &'a ActId>,
    needle: &str,
    noun: &str,
) -> Result<ActId, String> {
    let matches = |id: &ActId| id.as_str() == needle || id.as_str().starts_with(needle);
    let hits: Vec<&ActId> = ids.filter(|id| matches(id)).collect();
    let hits = if hits.is_empty() {
        canon
            .commitments()
            .iter()
            .filter(|c| c.from().is_some_and(matches))
            .map(|c| c.id())
            .collect()
    } else {
        hits
    };
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(format!("no {noun} matching `{needle}`")),
        n => Err(format!(
            "`{needle}` matches {n} {noun}s — use more characters"
        )),
    }
}

pub fn explain(log: &Log, canon: &Canon, id: &ActId) -> Result<Explanation, String> {
    if let Some(s) = canon.silences().iter().find(|s| &s.act == id) {
        return Ok(explain_silence(s));
    }
    if let Some(q) = canon.question(id) {
        return Ok(explain_question(canon, q));
    }
    let c = match canon.get(id) {
        Some(c) => c,
        None => {
            // Any other act in the log: a ratification, a grant, a position,
            // an adoption. `why` used to stop at rules and questions.
            if let Some(act) = log.acts().iter().find(|a| &a.id == id) {
                return Ok(explain_act(act, canon));
            }
            return Err(format!("no record matching `{id}`"));
        }
    };

    let headline = format!("{}  {}", c.id(), c.text());
    let mut lines = vec![format!(
        "written {} by {}{}",
        store::ymd(c.asserted_at()),
        person(c.actor()),
        c.source()
            .as_ref()
            .map_or_else(String::new, |src| format!(", from {src}"))
    )];

    if let Some(up) = &c.from() {
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

    for old in c.replaces() {
        lines.push(format!("replaced {}", named(canon, old)));
    }

    match &c.status() {
        // How it became a rule, in a canon somebody holds. "Why is this rule
        // like this" includes who made it one — and, under `twice`, who
        // had joined by the second vote.
        Status::Active => match canon.ratify(c, store::now()) {
            canon_core::Verdict::Ratified { how, .. } if !canon.grants().is_empty() => {
                lines.push(format!("in force — {}", person_in(&how)))
            }
            _ => lines.push("in force".into()),
        },
        Status::Superseded { by } => lines.push(format!("superseded by {}", named(canon, by))),
        Status::Retracted { at } => lines.push(format!("retracted {}", store::ymd(*at))),
        Status::Proposed { needs } => lines.push(format!(
            "proposed, not yet a rule — needs {}",
            person_in(needs)
        )),
        Status::Refused { at, by, why } => lines.push(format!(
            "refused{}: {}",
            refused_by(by, Some(*at), person),
            refused_why(by, why)
        )),
    }

    // Why it stopped being in force, if it did.
    for act in log.acts() {
        match &act.kind {
            ActKind::Supersede { old, rationale, .. }
                if old.contains(id) && !rationale.is_empty() =>
            {
                lines.push(format!("reason it was replaced: {rationale}"))
            }
            // A retraction written without standing is a proposal until a
            // holder approves it, and reads as one: "reason it was
            // retracted" over a rule still in force would be untrue.
            ActKind::Retract { target, rationale } if target == id => {
                use canon_core::Verdict;
                let put = canon.retractions().iter().find(|r| r.act == act.id);
                match put.map(|r| &r.verdict) {
                    Some(Verdict::Proposed { needs, .. }) => {
                        lines.push(format!(
                            "{} proposed retracting it, {}{}",
                            person(&act.actor),
                            store::ymd(act.ts_unix),
                            said(rationale)
                        ));
                        lines.push(format!(
                            "  needs {} — canon approve {}",
                            person_in(needs),
                            act.id
                        ));
                    }
                    Some(Verdict::Refused { at, by, why, .. }) => {
                        lines.push(format!(
                            "{} proposed retracting it, {}{}",
                            person(&act.actor),
                            store::ymd(act.ts_unix),
                            said(rationale)
                        ));
                        lines.push(format!(
                            "  refused{}: {}",
                            refused_by(by, Some(*at), person),
                            refused_why(by, why)
                        ));
                    }
                    Some(Verdict::Ratified { how, .. }) => {
                        lines.push(format!(
                            "{} proposed retracting it{}",
                            person(&act.actor),
                            said(rationale)
                        ));
                        lines.push(format!("  {}", person_in(how)));
                    }
                    None if !rationale.is_empty()
                        && !canon.ungoverned().iter().any(|(x, _)| *x == act.id) =>
                    {
                        lines.push(format!("reason it was retracted: {rationale}"))
                    }
                    None => {}
                }
            }
            _ => {}
        }
    }

    // Rulings that were made and did not take: somebody without standing
    // over this pair said the two do or do not conflict. On the record,
    // shown, and marked — a `why` that hid them would be the tool quietly
    // showing less than the log holds.
    for act in log
        .acts()
        .iter()
        .filter(|a| canon.ungoverned().iter().any(|(x, _)| x == &a.id))
    {
        let (verb, a, b, why) = match &act.kind {
            ActKind::Dismiss { a, b, rationale } => {
                ("called not in conflict with", a, b, rationale)
            }
            ActKind::Accept {
                a, b, rationale, ..
            } => ("would carry against", a, b, rationale),
            _ => continue,
        };
        if a != id && b != id {
            continue;
        }
        let other = if a == id { b } else { a };
        lines.push(format!(
            "{} {verb} {}",
            person(&act.actor),
            named(canon, other)
        ));
        lines.push(format!(
            "  {}, not applied: outside their standing{}",
            store::ymd(act.ts_unix),
            if why.is_empty() {
                String::new()
            } else {
                format!(". \"{why}\"")
            }
        ));
    }

    for conflict in canon
        .conflicts()
        .iter()
        .filter(|x| x.a == *id || x.b == *id)
    {
        let other = if conflict.a == *id {
            &conflict.b
        } else {
            &conflict.a
        };
        // WHO ruled, and when. A ruling with no name reads as the tool's own,
        // and the whole point of recording one is that it was somebody's — a
        // helper agent's dismissal and the house overruling it are two acts by
        // two actors, and `why` has to show both as such.
        let ruled = |want_accept: bool| -> String {
            log.acts()
                .iter()
                .rfind(|act| match &act.kind {
                    ActKind::Accept { a, b, .. } => want_accept && conflict.is_pair(a, b),
                    ActKind::Dismiss { a, b, .. } => !want_accept && conflict.is_pair(a, b),
                    _ => false,
                })
                .map(|act| format!("by {}, {}", person(&act.actor), store::ymd(act.ts_unix)))
                .unwrap_or_default()
        };
        // Who ruled and why on its own line under what was ruled: one event,
        // read top to bottom, rather than one sentence carrying an id, a
        // name, a date and a reason.
        match &conflict.disposition {
            Disposition::Tolerated { rationale, revisit } => {
                lines.push(format!("carried against {}", named(canon, other)));
                let who = ruled(true);
                lines.push(if who.is_empty() {
                    format!("  {rationale}")
                } else {
                    format!("  {who}: {rationale}")
                });
                if let Some(r) = revisit {
                    lines.push(format!("  look again by {r}"));
                }
            }
            Disposition::Dismissed { rationale } => {
                let why = if rationale.is_empty() {
                    "detector noise"
                } else {
                    rationale
                };
                lines.push(format!(
                    "called not in conflict with {}",
                    named(canon, other)
                ));
                let who = ruled(false);
                lines.push(if who.is_empty() {
                    format!("  {why}")
                } else {
                    format!("  {who}: {why}")
                });
            }
            Disposition::Open { reason } => {
                lines.push(format!("open tension with {}", named(canon, other)));
                lines.push(format!("  {reason}"));
            }
        }
    }

    Ok(Explanation { headline, lines })
}

/// A reason, as the tail of a sentence, or nothing.
fn said(rationale: &str) -> String {
    if rationale.trim().is_empty() {
        String::new()
    } else {
        format!(": {rationale}")
    }
}

/// `human:` stripped inside a sentence the fold wrote, such as a verdict's
/// `how` or `needs`. The fold names actors as the format does; this surface
/// names them as people do.
pub fn person_in(text: &str) -> String {
    // A name reads without its prefix; a personal SCOPE keeps it, because
    // there the prefix is part of the path — "holds phylla.glasses" would
    // name a scope that does not exist.
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find(canon_core::person::PREFIX) {
        out.push_str(&rest[..i]);
        let after = &rest[i + canon_core::person::PREFIX.len()..];
        let word = after
            .find(|c: char| c.is_whitespace() || matches!(c, ',' | ';' | ')' | '`'))
            .map_or(after, |end| &after[..end]);
        if word.trim_end_matches('.').contains('.') {
            out.push_str(canon_core::person::PREFIX);
        }
        rest = after;
    }
    out.push_str(rest);
    out
}

/// A silence, as the act that recorded it: what is unwritten, when, by
/// whom, and the reason it is kept unwritten. The same words `canon silence`
/// prints, with the `why` headline so every explanation reads alike.
fn explain_silence(s: &canon_core::Silence) -> Explanation {
    Explanation {
        headline: format!("{}  unwritten on purpose: \"{}\"", s.act, s.about),
        lines: vec![
            format!("written {} by {}", store::ymd(s.at), person(&s.actor)),
            s.rationale.clone(),
            format!(
                "`canon check --about \"{}\"` will say so rather than call it a gap",
                s.about
            ),
        ],
    }
}

/// A governance act as the act that recorded it: who and when, its own
/// fields, and whether the fold took it. The June's setup: a ratification in
/// the log was unexplainable, so the house refused the record.
fn explain_act(act: &canon_core::Act, canon: &Canon) -> Explanation {
    let kind = serde_json::to_value(&act.kind).unwrap_or(serde_json::Value::Null);
    let op = kind
        .get("op")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("act");
    let mut lines = vec![format!(
        "written {} by {}",
        store::ymd(act.ts_unix),
        person(&act.actor)
    )];
    if let serde_json::Value::Object(fields) = &kind {
        for (k, v) in fields {
            if k == "op" {
                continue;
            }
            let v = match v {
                serde_json::Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            lines.push(format!("{k}: {v}"));
        }
    }
    if let Some((_, why)) = canon.ungoverned().iter().find(|(id, _)| id == &act.id) {
        lines.push(format!("NOT APPLIED: {why}"));
    }
    Explanation {
        headline: format!("{}  {op}", act.id),
        lines,
    }
}

fn explain_question(canon: &Canon, q: &canon_core::Question) -> Explanation {
    let mut lines = vec![format!(
        "asked {} by {}",
        store::ymd(q.asked_at),
        person(&q.actor)
    )];
    if let Some(p) = &q.proposal {
        lines.push(format!("surfaced by the proposal: \"{p}\""));
    }
    match &q.status {
        Status::Active => {
            lines.push("open — the canon does not cover this".into());
            lines.push(format!(
                "answer it:  canon amend {} \"<the rule>\" -m \"<reason>\"",
                q.id
            ));
        }
        Status::Superseded { by } => lines.push(format!("answered by {}", named(canon, by))),
        Status::Retracted { at } => lines.push(format!("withdrawn {}", store::ymd(*at))),
        Status::Proposed { .. } | Status::Refused { .. } => {}
    }
    Explanation {
        headline: format!("{}  ? {}", q.id, q.text),
        lines,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_name_loses_its_prefix_and_a_personal_scope_keeps_it() {
        assert_eq!(
            super::person_in("human:phylla holds human:phylla.glasses, ask human:sam."),
            "phylla holds human:phylla.glasses, ask sam."
        );
    }

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
        assert!(e.lines.iter().any(|l| l.contains("superseded by")));
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

#[cfg(test)]
mod answered_question_tests {
    use super::*;
    use canon_core::Act;

    #[test]
    fn a_rule_that_answered_a_question_names_the_question_it_answered() {
        // Answering a question is superseding it — there is deliberately no
        // `answer` op — so `replaces` routinely names a question. Reporting
        // that as "(unknown)" loses the gap somebody noticed, which is half
        // of why the rule reads the way it does.
        let q = Act::new(
            ActKind::Question {
                text: "Who waters the plants when everyone travels at once?".into(),
                proposal: None,
            },
            100,
            "human:mira",
        );
        let answer = Act::new(
            ActKind::Supersede {
                text: "Whoever is away longest waters them.".into(),
                old: vec![q.id.clone()],
                rationale: "three plants died in August".into(),
            },
            200,
            "human:mira",
        );
        let log = Log::from_acts(vec![q.clone(), answer.clone()]);
        let canon = log.derive();
        let out = explain(&log, &canon, &answer.id)
            .expect("an explanation")
            .render("");
        assert!(
            out.contains("the question \"Who waters the plants when everyone travels at once?\""),
            "{out}"
        );
        assert!(!out.contains("(unknown)"), "{out}");
    }
}
