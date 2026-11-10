// SPDX-License-Identifier: AGPL-3.0-or-later
//! What was said back to a question, kept beside it and never made a rule.
//!
//! A reply is an annotation canon-core carries and does not read: it mints
//! nothing, grants nothing and closes nothing. This module is the one reader
//! of it, and `why`, `open`, the browser and the MCP tools all show what it
//! returns. It decides no liveness of its own. A reply counts when the fold
//! carried it (so it was verified and not undone), when it pins the very
//! question it names by full digest, and when it is not dated after `now`.
//!
//! Closing a question is the existing `repeal`. Which withdrawal closed it
//! is joined from what the fold publishes, and named only when one fits
//! (`tests/closing_source_bar.rs` holds the histories and the one limit).

use canon_core::{sign, Act, ActId, ActKind, Canon, Log, Status, Verdict};
use serde::{Deserialize, Serialize};

pub const OP: &str = "question_response";
const SCHEMA: &str = "canon-question-response/1";

/// An act named by its whole id and the digest of what it said.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Pin {
    id: ActId,
    sha256: String,
}

impl Pin {
    fn of(act: &Act) -> Self {
        Self {
            id: act.id.clone(),
            sha256: sign::digest(act),
        }
    }
}

/// The reply as it is written. Only the question and the text are required;
/// an absent field means nobody said, never "none".
#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Said {
    #[serde(default)]
    pub schema: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<Pin>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corrects: Option<Pin>,
}

/// One reply that counts, as every surface shows it.
#[derive(Serialize)]
pub struct Reply {
    pub id: ActId,
    pub actor: String,
    pub at: i64,
    #[serde(flatten)]
    pub said: Said,
    /// The same writer's later reply that corrects this one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corrected_by: Option<ActId>,
}

#[derive(Serialize, Default)]
pub struct View {
    pub replies: Vec<Reply>,
    /// Who withdrew it and why: one entry when the record names the
    /// withdrawal in force, several when it cannot say which.
    pub closed_by: Vec<(String, String)>,
    /// Something a reader should look at before relying on this.
    pub attention: Vec<String>,
    /// Replies in the record that do not count, and why.
    pub set_aside: Vec<String>,
}

fn reply_to<'a>(act: &'a Act, q: &ActId) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    match &act.kind {
        ActKind::Annotation { kind, body } if kind == OP => {
            (body.get("question")?.get("id")?.as_str()? == q.as_str()).then_some(body)
        }
        _ => None,
    }
}

/// Everything said back to `q`, and who closed it, as of `now`.
pub fn view(log: &Log, canon: &Canon, now: i64, q: &ActId) -> View {
    let mut v = View::default();
    let Some(asked) = log.acts().iter().find(|a| a.id == *q) else {
        return v;
    };
    let pin = Pin::of(asked);
    let live = |id: &ActId| canon.carried().iter().any(|(x, _)| x == id);
    for act in log.acts().iter().filter(|a| a.ts_unix <= now) {
        let Some(body) = reply_to(act, q) else {
            continue;
        };
        let said = serde_json::from_value::<Said>(body.clone().into())
            .ok()
            .filter(|s| s.schema == SCHEMA && !s.text.trim().is_empty());
        let why_not = match &said {
            None => "is in a form this build does not read",
            Some(_) if canon.unverified().iter().any(|(x, _)| *x == act.id) => {
                "is not signed by the key its writer bound"
            }
            Some(_) if !live(&act.id) => "was undone",
            Some(s) if s.question.as_ref() != Some(&pin) => "pins a different question",
            Some(_) if act.ts_unix < asked.ts_unix => "is dated before the question",
            Some(_) => "",
        };
        match said {
            Some(said) if why_not.is_empty() => v.replies.push(Reply {
                id: act.id.clone(),
                actor: act.actor.clone(),
                at: act.ts_unix,
                said,
                corrected_by: None,
            }),
            said => {
                v.set_aside.push(format!("{} {why_not}", act.id));
                // A correction that stops counting must not quietly hand the
                // word back to the account it corrected.
                if let Some(old) = said.and_then(|s| s.corrects) {
                    let gone = format!("{} corrected {}, and no longer counts", act.id, old.id);
                    v.attention.push(gone);
                }
            }
        }
    }
    // A correction is the same writer's, of an earlier reply to this
    // question, pinned by digest. Anything else corrects nothing.
    for i in 0..v.replies.len() {
        let Some(old) = v.replies[i].said.corrects.clone() else {
            continue;
        };
        let (id, at) = (v.replies[i].id.clone(), v.replies[i].at);
        let actor = v.replies[i].actor.clone();
        let target = log.acts().iter().find(|a| a.id == old.id);
        match v.replies.iter_mut().find(|r| r.id == old.id) {
            Some(r)
                if r.actor == actor
                    && r.at <= at
                    && r.id != id
                    && target.map(sign::digest).as_deref() == Some(&*old.sha256) =>
            {
                r.corrected_by = Some(id);
            }
            Some(_) => {
                (v.set_aside).push(format!("{id} corrects nothing: {} is not its own", old.id))
            }
            None => (v.attention).push(format!("{id} corrects {}, which is not available", old.id)),
        }
    }
    if let Some(Status::Retracted { at }) = canon.question(q).map(|q| &q.status) {
        v.closed_by = closed_by(log, canon, q, *at);
        if v.replies.iter().any(|r| r.at > *at) {
            (v.attention).push("new information since it was closed".into());
        }
    }
    v
}

/// The withdrawals that fit the question's status: verified, not refused,
/// and taking effect at the status's own second.
fn closed_by(log: &Log, canon: &Canon, q: &ActId, at: i64) -> Vec<(String, String)> {
    let out = |id: &ActId| {
        canon.unverified().iter().any(|(x, _)| x == id)
            || canon.ungoverned().iter().any(|(x, _)| x == id)
    };
    let put = |id: &ActId| canon.retractions().iter().find(|r| r.act == *id);
    log.acts()
        .iter()
        .filter(|a| !out(&a.id))
        .filter_map(|a| match &a.kind {
            ActKind::Retract { target, rationale } if target == q => {
                let fits = match put(&a.id).map(|r| &r.verdict) {
                    Some(Verdict::Ratified { since, .. }) => *since == at,
                    Some(_) => false,
                    None => a.ts_unix == at,
                };
                fits.then(|| (a.actor.clone(), rationale.clone()))
            }
            _ => None,
        })
        .collect()
}

/// The reply `actor` would write to `q`, checked against the record first.
/// `corrects` is a prefix of one of the actor's own replies that count.
pub fn reply(
    log: &Log,
    canon: &Canon,
    actor: &str,
    q: &ActId,
    mut said: Said,
    corrects: Option<&str>,
) -> Result<ActKind, String> {
    let find = |id: &ActId| log.acts().iter().find(|a| a.id == *id);
    let asked = find(q).ok_or_else(|| format!("no question matching `{q}`"))?;
    let blank = |f: &Option<String>| f.as_deref().is_some_and(|s| s.trim().is_empty());
    let given = [&said.attribution, &said.source, &said.remaining];
    if said.text.trim().is_empty() || given.into_iter().any(blank) {
        return Err("a reply needs its words, and a field that is given cannot be blank".into());
    }
    if let Some(needle) = corrects {
        let mine = view(log, canon, i64::MAX, q).replies;
        let own = |r: &&Reply| r.actor == actor && r.id.as_str().starts_with(needle);
        let [old] = mine.iter().filter(own).collect::<Vec<_>>()[..] else {
            return Err(format!(
                "`{needle}` is not one reply of yours to {q} — a correction is to your own reply"
            ));
        };
        said.corrects = find(&old.id).map(Pin::of);
    }
    said.schema = SCHEMA.into();
    said.question = Some(Pin::of(asked));
    let kind = OP.into();
    let body = serde_json::to_value(said)
        .ok()
        .and_then(|v| v.as_object().cloned());
    body.map(|body| ActKind::Annotation { kind, body })
        .ok_or_else(|| "a reply is an object".into())
}

/// The lines `why` adds under a question: each reply with who entered it,
/// what was left unsaid said as unsaid, then who closed it and what to check.
pub fn lines(v: &View, closed: bool) -> Vec<String> {
    let person = crate::explain::person;
    let or = |label: &str, f: &Option<String>, none: &str| {
        format!("  {label}: {}", f.as_deref().unwrap_or(none))
    };
    let mut out = Vec::new();
    for r in &v.replies {
        let (when, who) = (crate::store::ymd(r.at), person(&r.actor));
        let said = &r.said;
        out.push(format!(
            "reply {}, entered {when} by {who}: {}",
            r.id, said.text
        ));
        out.push(or("said by", &said.attribution, "not separately stated"));
        out.push(or("source", &said.source, "not supplied"));
        out.push(or(
            "still open",
            &said.remaining,
            "not assessed in this reply",
        ));
        if let Some(by) = &r.corrected_by {
            out.push(format!("  corrected by {by}; kept as history"));
        }
    }
    let n = v.closed_by.len();
    match &v.closed_by[..] {
        [] if closed => {
            out.push("closed; the record does not show which withdrawal closed it".into())
        }
        [] => {}
        [(who, why)] => {
            let why = if why.is_empty() {
                "no reason given"
            } else {
                why
            };
            out.push(format!(
                "closed by {}, withdrawn from the active notebook: {why}",
                person(who)
            ));
        }
        many => {
            out.push(format!(
                "closed; {n} withdrawals fit and the record does not say which is in force:"
            ));
            out.extend(
                many.iter()
                    .map(|(who, why)| format!("  {}: {why}", person(who))),
            );
        }
    }
    if closed {
        out.push("closing stops tracking here; it does not say the question was answered".into());
    }
    out.extend(v.attention.iter().map(|a| format!("needs attention: {a}")));
    out.extend(v.set_aside.iter().map(|s| format!("not counted: {s}")));
    out
}

/// The questions a reader is shown: the open ones, a closed one that has had
/// something said since, and with `all` the answered and closed ones too.
pub fn listed<'a>(
    log: &Log,
    canon: &'a Canon,
    now: i64,
    all: bool,
) -> Vec<(&'a canon_core::Question, View)> {
    (canon.questions().iter())
        .map(|q| (q, view(log, canon, now, &q.id)))
        .filter(|(q, v)| all || matches!(q.status, Status::Active) || !v.attention.is_empty())
        .collect()
}

/// What a list says under one question, beyond its words.
pub fn notes(q: &canon_core::Question, v: &View) -> Vec<String> {
    let mut out = Vec::new();
    match &q.status {
        Status::Active => {}
        Status::Superseded { by } => out.push(format!("answered by the rule {by}")),
        _ => out.push("closed: tracking stopped, which does not say it was answered".into()),
    }
    let n = v.replies.len();
    if n > 0 {
        out.push(format!("{n} reply(ies) — canon why {}", q.id));
    }
    out.extend(v.attention.iter().map(|a| format!("needs attention: {a}")));
    out
}
