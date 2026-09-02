// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon replay` — run a governance scenario against a canon, with no model
//! and no endpoint.
//!
//! **The governance layer is pure**: `Log -> Canon -> policy -> Decision`.
//! Only `check`, `tensions` and `draft` reach a model, and each of those asks
//! for *positions* — which a scenario supplies directly. So a whole history of
//! decisions replays in milliseconds inside `cargo test`, and a fixture that
//! ever needs an endpoint would mean the split between extraction and decision
//! had been violated.
//!
//! Beyond testing this is counterfactual governance: **what would this policy
//! have done to the last six months?** `--policy` overrides what the canon
//! adopted and re-decides every step, which is the question a group actually
//! has before changing how it decides.
//!
//! ## The seed dialect
//!
//! A fixture's acts are written by hand, so they cannot carry content-addressed
//! ids — the id is derived from the body. Instead an act may be LABELLED
//! (`"as": "quiet-hours"`) and referred to by `"@quiet-hours"` anywhere an id
//! belongs. The loader resolves labels in order and calls `Act::new`, so ids
//! come from the one place that mints them and the result is ordinary acts.
//!
//! This is a loader, not a second format. Nothing downstream sees a label, and
//! `canon replay <dir> --out <path>` writes the wire form — a real canon you
//! can run every other verb against.

use std::collections::BTreeMap;
use std::path::Path;

use canon_core::{
    Act, ActId, ActKind, Attributes, Canon, Log, Policy, Position, Pull, Source, Standing,
};
use serde_json::{json, Map, Value};

use crate::cmds::{fail, flag, has, positionals};

// ── the seed dialect ────────────────────────────────────────

/// Resolve `@label` references and mint the act.
fn materialize(mut body: Value, labels: &BTreeMap<String, ActId>) -> Result<Act, String> {
    let obj = body.as_object_mut().ok_or("an act must be a JSON object")?;
    let at = obj
        .remove("at")
        .and_then(|v| v.as_str().map(str::to_string))
        .ok_or("an act needs `at` (YYYY-MM-DD)")?;
    let ts = canon_core::date::parse_ymd(&at).ok_or(format!("`{at}` is not a date"))?;
    let by = obj
        .remove("by")
        .and_then(|v| v.as_str().map(str::to_string))
        .ok_or("an act needs `by` (the actor)")?;
    obj.remove("as");
    resolve(&mut body, labels)?;
    let kind: ActKind = serde_json::from_value(body).map_err(|e| e.to_string())?;
    Ok(Act::new(kind, ts, by))
}

/// Replace every `"@label"` anywhere in the value with the id it names.
///
/// Recursive because references appear inside arrays (`old: ["@a"]`) as well
/// as at the top level. An unknown label is an ERROR, not a passthrough: a
/// typo silently becoming a literal `"@quiet-hours"` id would fold as a
/// dangling reference and the fixture would still be green (§4.3, §18.3).
fn resolve(v: &mut Value, labels: &BTreeMap<String, ActId>) -> Result<(), String> {
    match v {
        Value::String(s) => {
            if let Some(name) = s.strip_prefix('@') {
                let id = labels
                    .get(name)
                    .ok_or_else(|| format!("`@{name}` is not a label defined above it"))?;
                *s = id.to_string();
            }
            Ok(())
        }
        Value::Array(a) => a.iter_mut().try_for_each(|x| resolve(x, labels)),
        Value::Object(o) => o.values_mut().try_for_each(|x| resolve(x, labels)),
        _ => Ok(()),
    }
}

/// Read a seed file into acts, resolving labels as it goes.
pub fn load_seed(raw: &str) -> Result<(Vec<Act>, BTreeMap<String, ActId>), String> {
    let mut labels: BTreeMap<String, ActId> = BTreeMap::new();
    let mut acts = Vec::new();
    for (i, line) in raw.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let body: Value = serde_json::from_str(line).map_err(|e| format!("line {}: {e}", i + 1))?;
        let label = body.get("as").and_then(|v| v.as_str()).map(str::to_string);
        let act = materialize(body, &labels).map_err(|e| format!("line {}: {e}", i + 1))?;
        if let Some(name) = label {
            labels.insert(name, act.id.clone());
        }
        acts.push(act);
    }
    Ok((acts, labels))
}

/// A canon's acts, from whichever dialect the file is written in.
///
/// **There is no flag, and that is the point.** A fixture line carries `at`
/// and `by` because a person typed it; a canon on disk carries `id` and `v`
/// because `Act::new` minted it. Neither has the other, so the file says
/// which it is and nobody has to be told. Making a group convert their own
/// record before they may ask a question of it is exactly the ceremony that
/// gets a tool put down and not picked up again.
pub fn load_acts(raw: &str) -> Result<(Vec<Act>, BTreeMap<String, ActId>), String> {
    let first = raw
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with("//"));
    let minted = first.is_some_and(|l| {
        serde_json::from_str::<Value>(l).is_ok_and(|v| v.get("id").is_some() && v.get("at").is_none())
    });
    if !minted {
        return load_seed(raw);
    }
    // Parsed by the one place that owns the format, so a version this build
    // does not understand is refused here exactly as it is everywhere else.
    let log = Log::parse(raw).map_err(|e| e.to_string())?;
    Ok((log.acts().to_vec(), BTreeMap::new()))
}

// ── the scenario ────────────────────────────────────────────

/// What a step is, and what the world looked like after it.
pub struct Step {
    pub name: String,
    /// What was actually proposed, for the steps that propose something. The
    /// step's NAME is what the fixture author called the case; this is what
    /// the house was deciding, and it is the one a person recognises.
    pub subject: Option<String>,
    /// Which of Ostrom's eight this demonstrates, when it demonstrates one.
    pub principle: Option<u8>,
    /// `mechanism` — the tool provides it. `affordance` — the tool permits it
    /// and does not get in the way. **The difference is the deliverable**; a
    /// table where all eight read `mechanism` would be the tell that somebody
    /// stretched a definition.
    pub strength: Option<String>,
    pub result: Value,
}

pub struct Replay {
    /// Every act the run minted, seed and scenario together. The seed file
    /// is only about three quarters of this house: the agent's adjudication,
    /// the sanctions ladder, the carried contradiction and the sortition draw
    /// all arrive as `act` steps in the scenario.
    pub acts: Vec<Act>,
    pub steps: Vec<Step>,
    pub labels: BTreeMap<String, ActId>,
    /// The rule every step was decided under, when one was forced.
    pub forced: Option<canon_core::Rule>,
}

/// Run a scenario over a seed canon.
///
/// `override_policy` is the counterfactual: decide every step under this rule
/// instead of whatever the canon adopted.
pub fn run_scenario(
    seed: &str,
    scenario: &str,
    override_policy: Option<canon_core::Rule>,
) -> Result<Replay, String> {
    let (mut acts, mut labels) = load_acts(seed)?;
    let mut now = acts.iter().map(|a| a.ts_unix).max().unwrap_or(0);
    let mut steps = Vec::new();

    for (i, line) in scenario.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let at = |e: String| format!("scenario line {}: {e}", i + 1);
        let body: Value = serde_json::from_str(line).map_err(|e| at(e.to_string()))?;
        let kind = body
            .get("step")
            .and_then(|v| v.as_str())
            .ok_or_else(|| at("a step needs `step`".into()))?
            .to_string();
        let name = body
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&kind)
            .to_string();
        let canon = Log::from_acts(acts.clone()).derive_at(now);

        let result = match kind.as_str() {
            "clock" => {
                let raw = body
                    .get("at")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| at("clock needs `at`".into()))?;
                now = canon_core::date::parse_ymd(raw)
                    .ok_or_else(|| at(format!("`{raw}` is not a date")))?;
                json!({ "now": raw })
            }
            "act" => {
                let mut b = body.clone();
                let obj = b.as_object_mut().expect("object");
                obj.remove("step");
                obj.remove("name");
                obj.remove("principle");
                obj.remove("strength");
                let label = obj.get("as").and_then(|v| v.as_str()).map(str::to_string);
                let act = materialize(b, &labels).map_err(at)?;
                if let Some(l) = label {
                    labels.insert(l, act.id.clone());
                }
                now = now.max(act.ts_unix);
                let id = act.id.clone();
                acts.push(act);
                json!({ "act": id.to_string() })
            }
            "check" => {
                check_step(&canon, &body, &labels, now, override_policy.as_ref()).map_err(at)?
            }
            "who" => {
                let scope = scope_of(&body, "scope").map_err(at)?;
                let deciders = canon.who_decides(&scope, now);
                // `deciders` is deepest-first, which renders the levels but
                // does not NAME them: when the narrow holders happen to sort
                // first, a two-level boundary and a one-level one print the
                // same list. `holders` is the deepest level on its own — the
                // set subsidiarity would actually route to.
                let deepest = deciders.first().map(|g| g.scope.depth());
                json!({
                    "deciders": deciders
                        .iter()
                        .map(|g| g.actor.clone())
                        .collect::<Vec<_>>(),
                    "holders": deciders
                        .iter()
                        .filter(|g| Some(g.scope.depth()) == deepest)
                        .map(|g| g.actor.clone())
                        .collect::<Vec<_>>(),
                    "policy": canon.policy_for(Some(&scope)).name(),
                })
            }
            "draw" => {
                let commit = id_of(&body, "commit", &labels).map_err(at)?;
                match canon.draw(&commit) {
                    Ok(d) => json!({
                        "seats": d.seats,
                        "pool": d.pool.len(),
                        "withheld": d.withheld,
                        "seed": d.seed,
                    }),
                    Err(e) => json!({ "error": e.to_string() }),
                }
            }
            "overdue" => {
                let due = canon.overdue(now);
                json!({
                    "count": due.len(),
                    "targets": due.iter().map(|o| o.target.to_string()).collect::<Vec<_>>(),
                })
            }
            // Where a commitment stands under its scope's ratification rule.
            // The step a fixture uses to show a proposal becoming a rule, or
            // not, and on whose word.
            "status" => {
                let id = id_of(&body, "commitment", &labels).map_err(at)?;
                let Some(c) = canon.get(&id) else {
                    return Err(at(format!("no commitment {id}")));
                };
                let scope = canon.scope_of(&id);
                let (status, detail) = match &c.status {
                    canon_core::Status::Active => ("in-force", String::new()),
                    canon_core::Status::Proposed { needs } => ("proposed", needs.clone()),
                    canon_core::Status::Refused { by, why, .. } => ("refused", format!("{by}: {why}")),
                    canon_core::Status::Superseded { by } => ("superseded", by.to_string()),
                    canon_core::Status::Retracted { .. } => ("retracted", String::new()),
                };
                json!({
                    "status": status,
                    "detail": detail,
                    "rule": canon.ratification_for(scope).name(),
                    "by": c.actor,
                })
            }
            "unattended" => json!({
                "unattended": canon
                    .unattended
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>(),
            }),
            "voice" => {
                let who = body
                    .get("actor")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| at("voice needs `actor`".into()))?;
                let v = canon.voice_of(who);
                json!({
                    "asked": v.asked.len(),
                    "answered": v.answered(),
                    "open": v.open(),
                    "positions": v.positions.len(),
                    "decided": v.decided.len(),
                })
            }
            "lineage" => json!({
                "generation": canon.ancestry.as_ref().map(|a| a.generation.clone()),
                "lineage": canon.ancestry.as_ref().map(|a| a.lineage.clone()),
                // What was inherited from the seed, and what this community
                // wrote for itself. The divergence IS the congruence.
                "inherited": canon.active().filter(|c| c.from.is_some()).count(),
                "local": canon.active().filter(|c| c.from.is_none()).count(),
            }),
            "state" => json!({
                "acts": acts.len(),
                "live": canon.active().count(),
                "proposed": canon.proposed().count(),
                "ungoverned": canon.ungoverned.len(),
                "open_questions": canon.open().count(),
                "carried": canon.carried.len(),
                "tolerated": canon.tolerated().count(),
            }),
            other => return Err(at(format!("unknown step `{other}`"))),
        };

        steps.push(Step {
            name,
            subject: body
                .get("proposal")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            principle: body
                .get("principle")
                .and_then(Value::as_u64)
                .map(|n| n as u8),
            strength: body
                .get("strength")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            result,
        });
    }
    Ok(Replay {
        acts: acts.clone(),
        steps,
        labels,
        forced: override_policy,
    })
}

fn scope_of(body: &Value, key: &str) -> Result<canon_core::Scope, String> {
    let raw = body
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("needs `{key}`"))?;
    canon_core::Scope::new(raw).ok_or_else(|| format!("`{raw}` is not a scope"))
}

fn id_of(body: &Value, key: &str, labels: &BTreeMap<String, ActId>) -> Result<ActId, String> {
    let raw = body
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("needs `{key}`"))?;
    match raw.strip_prefix('@') {
        Some(name) => labels
            .get(name)
            .cloned()
            .ok_or_else(|| format!("`@{name}` is not a label")),
        None => Ok(ActId::from_raw(raw)),
    }
}

/// A `check` step: the positions a model would have produced, supplied
/// directly, decided under the canon's own policy.
fn check_step(
    canon: &Canon,
    body: &Value,
    labels: &BTreeMap<String, ActId>,
    now: i64,
    forced: Option<&canon_core::Rule>,
) -> Result<Value, String> {
    let proposal = body
        .get("proposal")
        .and_then(|v| v.as_str())
        .ok_or("check needs `proposal`")?;
    let about = body
        .get("about")
        .and_then(|v| v.as_str())
        .unwrap_or(proposal);

    let mut positions = Vec::new();
    for p in body
        .get("positions")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
    {
        let pull = match p.get("pull").and_then(|v| v.as_str()) {
            Some("against") => Pull::Against,
            Some("toward") => Pull::Toward,
            other => return Err(format!("a position needs `pull`, not {other:?}")),
        };
        let because = p
            .get("because")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        positions.push(match p.get("citing") {
            Some(_) => Position::of(id_of(p, "citing", labels)?, pull, because),
            None => Position::by(
                p.get("actor")
                    .and_then(|v| v.as_str())
                    .ok_or("a position needs `citing` or `actor`")?,
                pull,
                because,
            ),
        });
    }
    let (standing, refused) = Standing::cited(canon, proposal, positions);

    let mut attrs = Attributes::about(about).at(now);
    if let Some(a) = body.get("actor").and_then(|v| v.as_str()) {
        attrs = attrs.by(a);
    }
    if body.get("scope").is_some() {
        attrs = attrs.in_scope(scope_of(body, "scope")?);
    }
    if let Some(r) = body.get("reversible").and_then(Value::as_bool) {
        attrs = attrs.reversible(r);
    }
    if body.get("amends").is_some() {
        attrs = attrs.amending(id_of(body, "amends", labels)?);
    }

    let adopted = canon.policy_for(attrs.scope.as_ref()).clone();
    let rule = forced.cloned().unwrap_or(adopted);
    let decision = rule.decide(&standing, &attrs, canon);
    Ok(json!({
        "outcome": decision.outcome,
        "authority": decision.authority,
        "because": decision.because,
        "rule": rule.name(),
        "cites": standing
            .cited_commitments()
            .filter_map(|p| p.commitment().map(ToString::to_string))
            .collect::<Vec<_>>(),
        "voices": standing
            .positions
            .iter()
            .filter_map(|p| p.actor().map(str::to_string))
            .collect::<Vec<_>>(),
        // Absence reported, never defaulted: a fixture whose positions were
        // silently dropped would assert against a shorter answer.
        "refused": refused.len(),
        "silence": canon.silence_about(about).is_some(),
    }))
}

// ── the counterfactual, in English ──────────────────────────

/// One decision that a rule the community did not adopt would have changed.
pub struct Divergence {
    pub name: String,
    pub subject: String,
    pub was: (String, String),
    pub would: (String, String),
}

/// Diff two passes over the same history.
///
/// **Two passes, not a diff against `expected.json`.** The fixture's expected
/// values happen to be the adopted rule's answers, so comparing against them
/// looks equivalent — but it only works where a fixture exists, and it reports
/// "the file said X" when the question is "our rule said X". Deciding the same
/// log twice, once under each rule, answers the question that was asked.
pub fn diverge(adopted: &Replay, forced: &Replay) -> Vec<Divergence> {
    let mut out = Vec::new();
    for (a, f) in adopted.steps.iter().zip(&forced.steps) {
        let read = |s: &Step, k: &str| {
            s.result.get(k).and_then(Value::as_str).unwrap_or("").to_string()
        };
        // Only steps that produce a ruling can diverge. `who`, `lineage` and
        // the standing queries answer the same way under any policy.
        let (aa, fa) = (read(a, "authority"), read(f, "authority"));
        if aa.is_empty() || (aa == fa && read(a, "outcome") == read(f, "outcome")) {
            continue;
        }
        // `because` is prefixed with the rule that produced it, and the rule
        // is already named in the heading — so the prefix is noise here.
        let trim = |s: String| match s.split_once(": ") {
            Some((_, rest)) => rest.to_string(),
            None => s,
        };
        let rung = |raw: &str| {
            canon_core::Authority::parse(raw).map_or_else(|| raw.to_string(), |x| x.prose().into())
        };
        out.push(Divergence {
            name: a.name.clone(),
            subject: a.subject.clone().unwrap_or_else(|| a.name.replace('-', " ")),
            was: (rung(&aa), trim(read(a, "because"))),
            would: (rung(&fa), trim(read(f, "because"))),
        });
    }
    out
}

/// What a group actually wants to read before changing how it decides.
pub fn render_divergence(rows: &[Divergence], forced: &str, total: usize) -> String {
    // Deliberately NOT "instead of <rule>". A canon runs several rules at
    // once — one at the root, another over the kitchen, a third over the
    // laundry — and naming any single one of them here would be false.
    let head = |what: String| {
        format!("Under `{forced}` instead of the rules this canon adopted, {what}\n")
    };
    if rows.is_empty() {
        return head(format!("nothing changes. All {total} decision(s) land the same way."));
    }
    let mut out = head(format!("{} of {total} decision(s) change.", rows.len()));
    // The width of the rung column, so the reasons line up and the eye can
    // run down what changed. Clamped like `check`'s stakes table.
    let w = rows
        .iter()
        .flat_map(|r| [r.was.0.len(), r.would.0.len()])
        .max()
        .unwrap_or(0)
        .min(30);
    for r in rows {
        let twice = rows.iter().filter(|o| o.subject == r.subject).count() > 1;
        let qualifier = if twice {
            format!("   ({})", r.name.replace('-', " "))
        } else {
            String::new()
        };
        out.push_str(&format!("\n  {}{qualifier}\n", r.subject));
        for (label, (rung, why)) in [("was", &r.was), ("would", &r.would)] {
            out.push_str(&format!("    {label:<6}{rung:<w$}  {why}\n"));
        }
    }
    out
}

/// The same answer, glanceable: one line per decision that moved, grouped by
/// which way it moved. The reasons on each side are the full rendering's job;
/// on a projector the question is "how many, which ones, which direction".
pub fn render_divergence_brief(rows: &[Divergence], forced: &str, total: usize) -> String {
    let head = |what: String| {
        format!("Under `{forced}` instead of the rules this canon adopted, {what}\n")
    };
    if rows.is_empty() {
        return head(format!("nothing changes. All {total} decision(s) land the same way."));
    }
    // Mildest first, the same order `Authority` declares. A move down the
    // list is a decision that got harder to take; up, easier.
    let ladder = [
        canon_core::Authority::Act,
        canon_core::Authority::ActAndNotify,
        canon_core::Authority::AskOne,
        canon_core::Authority::AskPanel,
        canon_core::Authority::Refuse,
    ];
    let rank = |prose: &str| ladder.iter().position(|a| a.prose() == prose);
    let mut easier: Vec<&Divergence> = Vec::new();
    let mut harder: Vec<&Divergence> = Vec::new();
    let mut sideways: Vec<&Divergence> = Vec::new();
    for r in rows {
        match (rank(&r.was.0), rank(&r.would.0)) {
            (Some(a), Some(b)) if b < a => easier.push(r),
            (Some(a), Some(b)) if b > a => harder.push(r),
            _ => sideways.push(r),
        }
    }
    let mut out = head(format!("{} of {total} decisions land somewhere else.", rows.len()));
    let mut summary = Vec::new();
    if !easier.is_empty() {
        summary.push(format!("{} would be easier to do", easier.len()));
    }
    if !harder.is_empty() {
        summary.push(format!("{} harder", harder.len()));
    }
    if !sideways.is_empty() {
        summary.push(format!("{} decided the same way for a different reason", sideways.len()));
    }
    out.push_str(&format!("{}.\n", summary.join("; ")));
    let subj = |r: &Divergence| {
        let twice = rows.iter().filter(|o| o.subject == r.subject).count() > 1;
        if twice {
            format!("{}  ({})", r.subject, r.name.replace('-', " "))
        } else {
            r.subject.clone()
        }
    };
    // Two lines a decision: what it was about, then the move. One line
    // would need a column wide enough for "replace the front door lock with
    // a keypad only I know the code to", and there is no such projector.
    for (label, group) in [("EASIER", &easier), ("HARDER", &harder), ("SAME OUTCOME, DIFFERENT REASON", &sideways)] {
        if group.is_empty() {
            continue;
        }
        out.push_str(&format!("\n  {label}\n"));
        for r in group {
            out.push_str(&crate::wrap::hang("    ", &subj(r)));
            out.push_str(&format!("\n        {} → {}\n", r.was.0, r.would.0));
        }
    }
    out.push_str("\n  the reason on each side: the same command without --brief\n");
    out
}

// ── the verb ────────────────────────────────────────────────

/// Compare a run against what the fixture said would happen.
///
/// Returns the mismatches. A missing key in `expected` is not checked — a
/// fixture asserts what it means to assert, and nothing is inferred from
/// silence. Present keys are compared exactly, except `because`, which is a
/// SUBSTRING match: it names the rule that fired, so a right answer for the
/// wrong reason fails, without pinning the whole sentence.
pub fn compare(
    replay: &Replay,
    expected: &Map<String, Value>,
) -> Vec<(String, String, Value, Value)> {
    let mut bad = Vec::new();
    for step in &replay.steps {
        let Some(want) = expected.get(&step.name).and_then(Value::as_object) else {
            continue;
        };
        for (key, wanted) in want {
            let got = step.result.get(key).cloned().unwrap_or(Value::Null);
            let ok = if key == "because" {
                match (wanted.as_str(), got.as_str()) {
                    (Some(w), Some(g)) => g.contains(w),
                    _ => false,
                }
            } else {
                resolve_expected(wanted, &replay.labels) == got
            };
            if !ok {
                bad.push((step.name.clone(), key.clone(), wanted.clone(), got));
            }
        }
    }
    bad
}

fn resolve_expected(v: &Value, labels: &BTreeMap<String, ActId>) -> Value {
    let mut out = v.clone();
    let _ = resolve(&mut out, labels);
    out
}

// ── a scenario from the record ──────────────────────────────

/// One scenario line, with the keys in reading order.
///
/// `serde_json`'s map sorts alphabetically, which puts `step` last and
/// `positions` in the middle of a line somebody is meant to open and edit.
/// The order here is the order the fixtures are written in.
fn step_line(fields: &[(&str, Value)]) -> String {
    let body: Vec<String> = fields
        .iter()
        .map(|(k, v)| format!("{}:{v}", json!(k)))
        .collect();
    format!("{{{}}}", body.join(","))
}

/// A short, url-safe name for a subject, so steps can be told apart.
fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars().take(48) {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    let t = out.trim_end_matches('-').to_string();
    if t.is_empty() {
        "subject".to_string()
    } else {
        t
    }
}

/// The positions on record about a subject, in the scenario's own dialect.
fn positions_on(canon: &Canon, about: &str) -> Vec<Value> {
    canon
        .positions
        .iter()
        .filter(|p| p.about == about)
        .map(|p| {
            let mut o = Map::new();
            match &p.position.source {
                Source::Commitment(id) => {
                    o.insert("citing".into(), json!(id.to_string()));
                }
                Source::Actor(a) => {
                    o.insert("actor".into(), json!(a));
                }
            }
            o.insert(
                "pull".into(),
                json!(match p.position.pull {
                    Pull::Toward => "toward",
                    Pull::Against => "against",
                }),
            );
            o.insert("because".into(), json!(p.position.because));
            Value::Object(o)
        })
        .collect()
}

/// Which scope a subject sits in, when a position cited a scoped commitment.
fn scope_of_subject(canon: &Canon, about: &str) -> Option<String> {
    canon
        .positions
        .iter()
        .filter(|p| p.about == about)
        .find_map(|p| match &p.position.source {
            Source::Commitment(id) => canon.scope_of(id).map(ToString::to_string),
            Source::Actor(_) => None,
        })
}

/// What a subject is, in words.
///
/// `approve` and `object` file their positions under the PROPOSAL'S ID,
/// because that is the only handle a vote has. An id is the right key and the
/// wrong thing to read, so a step keys on the id and proposes the sentence.
fn subject_text(canon: &Canon, about: &str) -> String {
    if about.starts_with(canon_core::ID_PREFIX) {
        if let Some(c) = canon.get(&ActId::from_raw(about)) {
            return c.text.clone();
        }
    }
    about.to_string()
}

/// The questions to put to a canon nobody has written questions for.
///
/// **This is the difference between a capability and a thing anyone uses.**
/// The counterfactual is worth having, and nobody is going to hand-write
/// forty-five steps to get it — a tool that asks for that much setup before
/// it answers anything is a tool that falls into disrepair. So the questions
/// come from what the canon already holds: every subject somebody took a
/// position on, and every adjudication the group recorded. Those are not
/// hypotheticals. They are the decisions this house actually had, which is
/// what makes "9 of them would have gone differently" worth reading.
///
/// Returns the scenario and how many of its steps are decisions, because a
/// canon holding neither should be told so plainly rather than handed an
/// empty table and left to wonder.
fn derive_scenario(canon: &Canon, now: i64, last: i64) -> (String, usize) {
    let mut lines: Vec<String> = vec![
        "// Derived from this canon's own record — nobody wrote this file.".into(),
        "// Every `check` below is a subject people argued about or the group".into(),
        "// decided, carrying the positions that are actually on record.".into(),
        "//".into(),
        "// `canon replay --write-scenario questions.jsonl` writes it out to edit:".into(),
        "// put the proposal in your own words, name the `actor` who would be doing".into(),
        "// it, mark what cannot be undone with \"reversible\": false, drop what does".into(),
        "// not matter. `canon replay --scenario questions.jsonl` then uses yours.".into(),
    ];

    // The clock, said out loud — a replay whose answer depends on when it ran
    // is not a replay, so the day is a step like any other and moving it is
    // an edit rather than a surprise. Emitted ONLY when today is genuinely
    // later than the last act. A date truncates to midnight, and midnight on
    // the day a house wrote its first grants falls BEFORE them: setting the
    // clock there would report a canon that nobody holds.
    if let Some(midnight) = canon_core::date::parse_ymd(&canon_core::date::ymd(now)) {
        if midnight > last {
            lines.push(step_line(&[
                ("step", json!("clock")),
                ("at", json!(canon_core::date::ymd(now))),
            ]));
        }
    }

    // Who holds what — one line per boundary anybody holds. It costs nothing
    // and it is the picture a group wants beside the decisions.
    let mut scopes: Vec<String> = canon
        .grants
        .iter()
        .filter(|g| g.held_at(now))
        .map(|g| g.scope.to_string())
        .collect();
    scopes.sort();
    scopes.dedup();
    for sc in &scopes {
        lines.push(step_line(&[
            ("step", json!("who")),
            ("name", json!(format!("who-holds-{}", slug(sc)))),
            ("scope", json!(sc)),
        ]));
    }

    // Rulings first, because a `decided` act names who adjudicated — so the
    // step can also ask whether that person held the thing they ruled on.
    let subjects: Vec<(String, Option<String>)> = canon
        .rulings
        .iter()
        .map(|r| (r.about.clone(), Some(r.actor.clone())))
        .chain(canon.positions.iter().map(|p| (p.about.clone(), None)))
        .collect();
    let mut seen: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for (about, actor) in subjects {
        if about.trim().is_empty() || seen.contains(&about) {
            continue;
        }
        seen.push(about.clone());
        // The record keeps what a subject was CALLED, never a sentence
        // proposing it. A named subject is its own honest stand-in and a
        // proposal id resolves to the rule it names; the written-out file is
        // where better words go.
        let text = subject_text(canon, &about);
        let mut name = slug(&text);
        let mut n = 2;
        while names.contains(&name) {
            name = format!("{}-{n}", slug(&text));
            n += 1;
        }
        names.push(name.clone());
        let mut fields = vec![
            ("step", json!("check")),
            ("name", json!(name)),
            // Keyed on what the positions were filed under, which is what the
            // graduated ladder counts prior decisions by.
            ("about", json!(about)),
            ("proposal", json!(text)),
        ];
        if let Some(sc) = scope_of_subject(canon, &about) {
            fields.push(("scope", json!(sc)));
        }
        if let Some(a) = actor {
            fields.push(("actor", json!(a)));
        }
        fields.push(("positions", json!(positions_on(canon, &about))));
        lines.push(step_line(&fields));
    }
    let checks = seen.len();
    lines.push(step_line(&[
        ("step", json!("state")),
        ("name", json!("where-this-leaves-us")),
    ]));
    (lines.join("\n") + "\n", checks)
}

pub fn run(args: &[String]) -> i32 {
    let pos = positionals(args);
    // **No directory means this canon, here.** The everyday form of this verb
    // takes no arguments at all. A group that has to assemble a fixture
    // before it may ask what a different rule would have done to it will
    // never ask, and the answer is the most useful thing this tool computes.
    let dir = match pos.first() {
        Some(d) => std::path::PathBuf::from(d),
        None => match crate::cmds::dir() {
            Ok(d) => d,
            Err(e) => {
                return fail(format!(
                    "{e}\n       usage: canon replay [<dir>] [--policy <rule>] [--brief] \
                     [--scenario <file>] [--write-scenario <file>]\n       \
                     with no directory it replays the canon you are standing in"
                ))
            }
        },
    };
    let dir = dir.as_path();
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name))
            .map_err(|e| format!("reading {}: {e}", dir.join(name).display()))
    };
    let seed = match read(crate::store::FILE) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };

    // Where the questions come from, in order: the file you named, the one
    // sitting beside the acts, or — the case that decides whether anyone
    // ever runs this — one derived from the record itself.
    let (scenario, derived) = match flag(args, "--scenario") {
        Some(p) => match std::fs::read_to_string(p) {
            Ok(s) => (s, None),
            Err(e) => return fail(format!("reading {p}: {e}")),
        },
        None => match read("scenario.jsonl") {
            Ok(s) => (s, None),
            Err(_) => {
                let (acts, _) = match load_acts(&seed) {
                    Ok(v) => v,
                    Err(e) => return fail(e),
                };
                let last = acts.iter().map(|a| a.ts_unix).max().unwrap_or(0);
                let now = crate::store::now().max(last);
                let canon = Log::from_acts(acts).derive_at(now);
                let (text, checks) = derive_scenario(&canon, now, last);
                (text, Some(checks))
            }
        },
    };

    if let Some(out) = flag(args, "--write-scenario") {
        if let Err(e) = std::fs::write(out, &scenario) {
            return fail(format!("writing {out}: {e}"));
        }
        let steps = scenario
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with("//"))
            .count();
        println!("{steps} step(s) written to {out}");
        println!("edit it, then:  canon replay --scenario {out} --policy consent --brief");
        return 0;
    }

    // Absence reported as absence. A canon nobody has recorded a position or
    // a decision in has nothing to re-decide, and saying "0 of 0 changed"
    // would look like an answer.
    if derived == Some(0) {
        println!("nothing here records what anyone argued about or what the group decided,");
        println!("so there is nothing to re-decide under another rule. Either act does it:\n");
        println!("  canon position \"<subject>\" --against -m \"<why>\"");
        println!("  canon decide \"<subject>\" --outcome conflicts --authority ask-panel\n");
    }

    // The whole policy vocabulary, not three of it. The rule a group is
    // weighing is usually the one with a number in it — "what would two
    // objections have done to us" — and a counterfactual that cannot express
    // the rule under discussion is a demo.
    let forced = match flag(args, "--policy") {
        None => None,
        Some(name) => match crate::govern::rule_from(args, name) {
            Ok(r) => Some(r),
            Err(e) => return fail(e),
        },
    };
    let started = std::time::Instant::now();
    let replay = match run_scenario(&seed, &scenario, forced.clone()) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };

    // The counterfactual, decided twice. A forced run on its own can say what
    // happened under the other rule; it takes the adopted rule's own pass to
    // say what CHANGED, which is the question a group actually has.
    let counterfactual = match &forced {
        None => None,
        Some(rule) => match run_scenario(&seed, &scenario, None) {
            Err(e) => return fail(e),
            Ok(adopted) => {
                let rows = diverge(&adopted, &replay);
                // Decisions, not steps. `who` and `state` answer questions
                // and decide nothing, so counting them would put a
                // reassuring denominator under a real number.
                let decisions = adopted
                    .steps
                    .iter()
                    .filter(|s| s.result.get("authority").is_some())
                    .count();
                Some(if has(args, "--brief") {
                    render_divergence_brief(&rows, &rule.name(), decisions)
                } else {
                    render_divergence(&rows, &rule.name(), decisions)
                })
            }
        },
    };

    // `--out` materialises the fixture as a real canon. The seed dialect is a
    // loader for the ordinary format, so this writes ordinary acts with the
    // ids `Act::new` already minted — the same ids this verb prints, stable
    // across machines because they are derived from the body.
    if let Some(out) = flag(args, "--out") {
        let out = Path::new(out);
        let profile = flag(args, "--profile").unwrap_or("house");
        if let Err(e) = std::fs::create_dir_all(out)
            .and_then(|()| {
                std::fs::write(out.join(crate::store::FILE), Log::from_acts(replay.acts.clone()).render())
            })
            .and_then(|()| std::fs::write(out.join("profile"), format!("{profile}\n")))
        {
            return fail(format!("writing {}: {e}", out.display()));
        }
        crate::store::ignore_local(out);
        println!(
            "{} act(s) written to {}\n\nCANON_DIR={} canon list",
            replay.acts.len(),
            out.display(),
            out.display()
        );
        return 0;
    }

    if has(args, "--json") {
        let out: Map<String, Value> = replay
            .steps
            .iter()
            .map(|s| (s.name.clone(), s.result.clone()))
            .collect();
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        if let Some(rule) = &replay.forced {
            // The counterfactual has to name itself, or a reader cannot tell
            // a replay of what happened from a replay of what would have.
            println!("decided under a forced rule: {}\n", rule.name());
        }
        if let Some(text) = &counterfactual {
            print!("{text}");
        }
        // `--brief` is the whole answer and nothing else. The step-by-step
        // listing below is what you read when you are debugging a fixture,
        // not what you read when you are deciding whether to change a rule.
        // Without a forced rule the whole answer is the acceptance test: which
        // of Ostrom's eight principles this history exercised, and how.
        if has(args, "--brief") && replay.forced.is_none() {
            print!("{}", render_principles(&replay.steps));
        }
        for s in &replay.steps {
            if has(args, "--brief") {
                break;
            }
            let head = match (&s.principle, &s.strength) {
                (Some(n), Some(m)) => format!("  [Ostrom {n}, {m}]"),
                _ => String::new(),
            };
            println!("{}{head}", s.name);
            println!("  {}", compact(&s.result));
        }
    }

    // The expectations are optional: a replay is also a counterfactual, and
    // "what would consent have done to the last six months" has nothing to
    // compare against.
    let Ok(raw) = read("expected.json") else {
        // A derived run says what it just did and what the next question is.
        // Somebody who typed one word should not have to read the manual to
        // find out that the interesting flag exists.
        if let Some(n) = derived {
            if n > 0 && forced.is_none() && !has(args, "--json") {
                println!(
                    "\n{n} decision(s), from what this canon already records, in {}",
                    elapsed(started.elapsed())
                );
                println!("what another rule would have done:");
                println!("  canon replay --policy consent --brief");
                println!("  canon replay --policy threshold --objections 2 --brief");
            }
        }
        return 0;
    };
    let Ok(Value::Object(expected)) = serde_json::from_str::<Value>(&raw) else {
        return fail("expected.json is not a JSON object");
    };
    let bad = compare(&replay, &expected);
    if bad.is_empty() {
        println!(
            "\n{} step(s), all as expected, in {}",
            replay.steps.len(),
            elapsed(started.elapsed())
        );
        return 0;
    }
    // A briefed counterfactual has already said what moved, decision by
    // decision. Listing the same movements again as field-level mismatches
    // reads as twenty failures under a line that just said nine changed.
    if has(args, "--brief") && replay.forced.is_some() {
        return 0;
    }
    for (step, key, want, got) in &bad {
        eprintln!("{step}.{key}: expected {want}, got {got}");
    }
    eprintln!("\n{} mismatch(es)", bad.len());
    // A COUNTERFACTUAL HAS NO PASS. `expected.json` records what the rules
    // this canon adopted produce; a forced run answers a different question,
    // so differing from that file is the point rather than a failure. The
    // lines above are still printed — they say which recorded answers moved —
    // but exiting non-zero here would report "what if we decided differently"
    // as a broken fixture.
    i32::from(replay.forced.is_none())
}

fn elapsed(d: std::time::Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1000.0 {
        format!("{ms:.0} ms")
    } else {
        format!("{:.1} s", ms / 1000.0)
    }
}

/// The acceptance test, read off a replay: for each of Ostrom's eight, the
/// strength the fixture claims and the scenes that carry it, each on one
/// line with what it asked and what came back. Untagged steps are the acts
/// between the decisions and are counted on the last line rather than
/// listed.
///
/// The scenes are the substance. A table of eight green rows in eleven
/// milliseconds looks like a claim; the same table with "theo proposes a
/// rota for the kitchen → ask one person with standing" under it is a record
/// of what was tried and what the rules said.
pub fn render_principles(steps: &[Step]) -> String {
    const NAMES: [&str; 8] = [
        "clearly defined boundaries",
        "congruence with local conditions",
        "collective-choice arrangements",
        "monitors accountable to appropriators",
        "graduated sanctions",
        "rapid low-cost conflict resolution",
        "rights to organize not undermined",
        "nested enterprises",
    ];
    let mut rows: Vec<(String, Vec<String>)> = (0..8).map(|_| (String::new(), vec![])).collect();
    for s in steps {
        let Some(n) = s.principle else { continue };
        let Some(row) = usize::from(n).checked_sub(1).and_then(|i| rows.get_mut(i)) else {
            continue;
        };
        if let Some(m) = &s.strength {
            row.0 = m.clone();
        }
        row.1.push(scene_line(s));
    }
    let mut out = String::from("Ostrom's eight, over this history\n");
    for (i, (strength, scenes)) in rows.iter().enumerate() {
        // The fixture's words are `mechanism` and `affordance`; the room's
        // words are whether the tool does it or leaves it to people.
        let mark = match (scenes.is_empty(), strength.as_str()) {
            (true, _) => "-",
            (_, "mechanism") => "built in",
            (_, "affordance") => "left to people",
            (_, other) => other,
        };
        out.push_str(&format!("\n  {}  {:<38} {mark}\n", i + 1, NAMES[i]));
        for line in scenes {
            out.push_str(&crate::wrap::hang("       ", line));
            out.push('\n');
        }
    }
    let cleared = rows.iter().filter(|r| !r.1.is_empty()).count();
    let untagged = steps.iter().filter(|s| s.principle.is_none()).count();
    out.push_str(&format!(
        "\n{cleared} of 8 held; {untagged} step(s) are the everyday acts in between\n"
    ));
    out
}

/// One scene, in the words a person would use: what was asked, what the
/// rules answered. Reads the step's result by shape rather than by kind, so a
/// fixture step this build has never seen still gets its name printed.
fn scene_line(s: &Step) -> String {
    let r = &s.result;
    let str_ = |k: &str| r.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let list = |k: &str| {
        r.get(k)
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(|x| x.trim_start_matches("human:").to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_default()
    };
    let name = s.name.replace('-', " ");
    if r.get("authority").is_some() {
        let subject = s.subject.clone().unwrap_or_else(|| name.clone());
        let rung = canon_core::Authority::parse(&str_("authority"))
            .map_or_else(|| str_("authority"), |a| a.prose().to_string());
        return format!("{subject} → {rung}");
    }
    if r.get("holders").is_some() {
        return format!("{name}: {}", list("holders"));
    }
    if r.get("status").is_some() {
        let detail = str_("detail");
        return if detail.is_empty() {
            format!("{name} → {}", str_("status"))
        } else {
            format!("{name} → {} ({detail})", str_("status"))
        };
    }
    if r.get("generation").is_some() {
        return format!(
            "{name}: {} inherited, {} written here, on {}@{}",
            r.get("inherited").and_then(Value::as_u64).unwrap_or(0),
            r.get("local").and_then(Value::as_u64).unwrap_or(0),
            str_("lineage"),
            str_("generation")
        );
    }
    if r.get("unattended").is_some() {
        return format!("{name}: {} named to the house", list("unattended"));
    }
    if let Some(n) = r.get("count").and_then(Value::as_u64) {
        return format!("{name}: {n} overdue");
    }
    if r.get("positions").is_some() {
        return format!(
            "{name}: {} position(s), {} decision(s)",
            r.get("positions").and_then(Value::as_u64).unwrap_or(0),
            r.get("decided").and_then(Value::as_u64).unwrap_or(0)
        );
    }
    if r.get("live").is_some() {
        return format!(
            "{name}: {} live, {} carried knowingly",
            r.get("live").and_then(Value::as_u64).unwrap_or(0),
            r.get("tolerated").and_then(Value::as_u64).unwrap_or(0)
        );
    }
    name
}

fn compact(v: &Value) -> String {
    match v {
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| format!("{k}={}", v.to_string().trim_matches('"')))
            .collect::<Vec<_>>()
            .join("  "),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn house() -> (Vec<Act>, Canon, i64) {
        let ts = 1_700_000_000;
        let rule = Act::new(
            ActKind::Assert {
                text: "Bikes go against the left wall.".into(),
                from: None,
                source: None,
            },
            ts,
            "human:sam",
        );
        let grant = Act::new(
            ActKind::Grant {
                holder: "human:sam".into(),
                scope: canon_core::Scope::new("house").expect("scope"),
                horizon: None,
                rationale: String::new(),
            },
            ts,
            "human:sam",
        );
        let against = Act::new(
            ActKind::Position {
                about: rule.id.to_string(),
                citing: None,
                pull: Pull::Against,
                because: "the hall is too narrow for them".into(),
            },
            ts + 10,
            "human:dana",
        );
        let acts = vec![rule, grant, against];
        let canon = Log::from_acts(acts.clone()).derive_at(ts + 10);
        (acts, canon, ts + 10)
    }

    #[test]
    fn a_canon_on_disk_and_a_fixture_are_both_read_with_no_flag() {
        // Making somebody convert their own record before they may ask a
        // question of it is the ceremony that gets a tool put down.
        let (acts, _, _) = house();
        let minted = Log::from_acts(acts.clone()).render();
        let (got, labels) = load_acts(&minted).expect("minted acts");
        assert_eq!(got.len(), acts.len());
        assert!(labels.is_empty(), "minted acts carry ids, never labels");

        let seed = r#"{"at":"2026-01-01","by":"human:sam","as":"quiet","op":"assert","text":"Quiet after eleven."}"#;
        let (got, labels) = load_acts(seed).expect("seed dialect");
        assert_eq!(got.len(), 1);
        assert_eq!(labels.len(), 1, "and the label still resolves");
    }

    #[test]
    fn a_derived_scenario_asks_about_what_people_actually_argued_over() {
        let (acts, canon, last) = house();
        let (text, checks) = derive_scenario(&canon, last, last);
        assert_eq!(checks, 1, "one subject anybody took a position on");
        // `approve` and `object` file under the proposal's ID. The step keys
        // on the id — the ladder counts prior decisions by it — and reads as
        // the rule.
        assert!(
            text.contains(r#""proposal":"Bikes go against the left wall.""#),
            "{text}"
        );
        assert!(
            text.contains(r#""name":"bikes-go-against-the-left-wall""#),
            "{text}"
        );
        assert!(text.contains(r#""step":"who""#), "standing comes for free");
        assert!(
            text.contains("the hall is too narrow for them"),
            "the recorded reason travels with it"
        );

        // And it runs, which is the whole point of deriving it.
        let run = run_scenario(&Log::from_acts(acts).render(), &text, None).expect("replays");
        let decided = run
            .steps
            .iter()
            .find(|s| s.name == "bikes-go-against-the-left-wall")
            .expect("the derived decision");
        assert_eq!(decided.result["outcome"], json!("conflicts"));
    }

    #[test]
    fn the_derived_clock_never_lands_before_the_last_act() {
        // A date truncates to midnight, and midnight on the day a house wrote
        // its first grants falls BEFORE them. A clock set there reports a
        // canon nobody holds, which is a lie about standing, not a rounding.
        let (acts, canon, last) = house();
        let (text, _) = derive_scenario(&canon, last, last);
        assert!(!text.contains(r#""step":"clock""#), "{text}");

        let run = run_scenario(&Log::from_acts(acts).render(), &text, None).expect("replays");
        let who = run
            .steps
            .iter()
            .find(|s| s.name.starts_with("who-holds"))
            .expect("a who step");
        assert_eq!(
            who.result["holders"],
            json!(["human:sam"]),
            "standing is held at the clock the derivation chose"
        );
    }

    #[test]
    fn a_canon_that_records_no_argument_derives_no_decision() {
        // Reported as absence rather than answered with `0 of 0 changed`,
        // which would read like a finding.
        let ts = 1_700_000_000;
        let a = Act::new(
            ActKind::Assert {
                text: "Quiet after eleven.".into(),
                from: None,
                source: None,
            },
            ts,
            "human:sam",
        );
        let canon = Log::from_acts(vec![a]).derive_at(ts);
        let (_, checks) = derive_scenario(&canon, ts, ts);
        assert_eq!(checks, 0);
    }
}
