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
//! `canon replay --dump` writes the wire form.

use std::collections::BTreeMap;
use std::path::Path;

use canon_core::{Act, ActId, ActKind, Attributes, Canon, Log, Policy, Position, Pull, Standing};
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

// ── the scenario ────────────────────────────────────────────

/// What a step is, and what the world looked like after it.
pub struct Step {
    pub name: String,
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
    let (mut acts, mut labels) = load_seed(seed)?;
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
        let canon = Log::from_acts(acts.clone()).derive();

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
                json!({
                    "deciders": canon
                        .who_decides(&scope, now)
                        .iter()
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
                "open_questions": canon.open().count(),
                "carried": canon.carried.len(),
                "tolerated": canon.tolerated().count(),
            }),
            other => return Err(at(format!("unknown step `{other}`"))),
        };

        steps.push(Step {
            name,
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

pub fn run(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(dir) = pos.first() else {
        return fail("usage: canon replay <fixture-dir> [--policy <rule>] [--json]");
    };
    let dir = Path::new(dir);
    let read = |name: &str| {
        std::fs::read_to_string(dir.join(name))
            .map_err(|e| format!("reading {}: {e}", dir.join(name).display()))
    };
    let (seed, scenario) = match (read("acts.jsonl"), read("scenario.jsonl")) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return fail(e),
    };
    let forced = match flag(args, "--policy") {
        None => None,
        Some(name) => match name {
            "default" => Some(canon_core::Rule::Default),
            "consent" => Some(canon_core::Rule::Consent),
            "subsidiarity" => Some(canon_core::Rule::Subsidiarity),
            other => return fail(format!("`{other}` is not a rule this verb can force")),
        },
    };
    let replay = match run_scenario(&seed, &scenario, forced) {
        Ok(r) => r,
        Err(e) => return fail(e),
    };

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
        for s in &replay.steps {
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
        return 0;
    };
    let Ok(Value::Object(expected)) = serde_json::from_str::<Value>(&raw) else {
        return fail("expected.json is not a JSON object");
    };
    let bad = compare(&replay, &expected);
    if bad.is_empty() {
        println!("\n{} step(s), all as expected", replay.steps.len());
        return 0;
    }
    for (step, key, want, got) in &bad {
        eprintln!("{step}.{key}: expected {want}, got {got}");
    }
    eprintln!("\n{} mismatch(es)", bad.len());
    1
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
