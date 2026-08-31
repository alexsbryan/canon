// SPDX-License-Identifier: AGPL-3.0-or-later
//! The transfer bar — **does Ostrom governance fall out of these primitives
//! for an arbitrary common-pool resource, and can it fail to?**
//!
//! `governance_bar.rs` asks whether the eight design principles can be
//! demonstrated at all, over two fixtures written by hand. This asks the two
//! questions that come next.
//!
//! **Generality.** Ten institutions — a makerspace, a coliving building, a
//! monorepo, a compute mesh, a build farm, an allotment site, a forum, an
//! alpine pasture, an irrigation canal and an inshore fishery — are built
//! from ONE spine under `fixtures/cpr/_spine/` and one vocabulary of nouns
//! each. The spine is byte identical across all ten. If the eight principles
//! hold in all of them, they hold because of the primitives and not because
//! of the domain.
//!
//! **Falsifiability.** Four more fixtures are single-variable ablations of
//! those ten: one line of vocabulary removes one use of one primitive. Each
//! names, before the run, which principles it expects to lose. A bar that
//! cannot go red is not a bar, and a study whose instrument reports success
//! on a broken institution has measured nothing.
//!
//! **The criteria are the instrument and they are domain-neutral.** Each one
//! below is a property of the replay output, written from the principle's
//! definition. None of them mentions a house, a repository or a canal.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

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

/// The ladder, in the order an authority may only ever climb.
const RUNGS: [&str; 5] = ["act", "act-and-notify", "ask-one", "ask-panel", "refuse"];

/// A vocabulary names nouns. These keys are the typed governance the library
/// reads, and a vocabulary that could set one of them would be choosing
/// mechanism, not naming a resource.
/// What `canon replay` prints between a diverging field and its values.
const MISMATCH: &str = ": expected ";

/// The sorted set of `(step, field)` pairs a forced rule changes.
fn divergence(fixture: &str, rule: &str) -> Vec<String> {
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(["replay", cpr().join(fixture).to_str().unwrap(), "--policy", rule])
        .output()
        .expect("canon replay runs");
    let mut pairs: Vec<String> = String::from_utf8_lossy(&out.stderr)
        .lines()
        .filter_map(|l| l.split_once(MISMATCH).map(|(k, _)| k.to_string()))
        .filter(|k| k.contains('.') && !k.contains(' '))
        .collect();
    pairs.sort();
    pairs
}

const RESERVED: [&str; 6] = ["rule", "authority", "outcome", "horizon", "principle", "strength"];

fn cpr() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/cpr")
}

struct Fixture {
    name: String,
    vocab: Value,
    steps: Vec<String>,
    run: BTreeMap<String, Value>,
}

/// A vocabulary, with `extends` resolved.
///
/// An ablation's own file is the parent's name plus the one line that differs,
/// which is the whole point of it — so anything reading a vocabulary has to
/// read the effective one or it will conclude an ablation has no monitor.
fn read_vocab(dir: &Path) -> Value {
    let mut v: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("vocab.json")).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    let Some(parent) = v["extends"].as_str().map(str::to_string) else {
        return v;
    };
    let mut base = read_vocab(&cpr().join(parent));
    merge(&mut base, &mut v);
    base
}

fn merge(base: &mut Value, over: &mut Value) {
    let (Some(b), Some(o)) = (base.as_object_mut(), over.as_object_mut()) else {
        *base = over.take();
        return;
    };
    for (k, val) in o.iter_mut() {
        match b.get_mut(k) {
            Some(existing) if existing.is_object() && val.is_object() => merge(existing, val),
            _ => {
                b.insert(k.clone(), val.take());
            }
        }
    }
}

fn fixtures() -> Vec<Fixture> {
    let mut out = Vec::new();
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(cpr())
        .expect("fixtures/cpr")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.join("vocab.json").exists())
        .collect();
    dirs.sort();
    for dir in dirs {
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let vocab = read_vocab(&dir);
        let scenario = std::fs::read_to_string(dir.join("scenario.jsonl")).unwrap();
        let steps = scenario
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with("//"))
            .map(|l| {
                let v: Value = serde_json::from_str(l).expect("scenario line is JSON");
                v["name"]
                    .as_str()
                    .unwrap_or_else(|| v["step"].as_str().unwrap())
                    .to_string()
            })
            .collect();
        let out_json = Command::new(env!("CARGO_BIN_EXE_canon"))
            .args(["replay", dir.to_str().unwrap(), "--json"])
            .output()
            .expect("canon replay runs");
        let stdout = String::from_utf8_lossy(&out_json.stdout);
        let end = stdout.rfind('}').expect("replay printed an object") + 1;
        let run: BTreeMap<String, Value> =
            serde_json::from_str(&stdout[..end]).expect("replay --json is an object");
        out.push(Fixture { name, vocab, steps, run });
    }
    assert!(out.len() >= 14, "the study is 14 fixtures, found {}", out.len());
    out
}

// ── the criteria, one per principle ─────────────────────────

impl Fixture {
    fn get(&self, step: &str, key: &str) -> Option<&Value> {
        self.run.get(step).and_then(|s| s.get(key))
    }
    fn s(&self, step: &str, key: &str) -> String {
        self.get(step, key)
            .and_then(Value::as_str)
            .unwrap_or("<missing>")
            .to_string()
    }
    fn n(&self, step: &str, key: &str) -> i64 {
        self.get(step, key).and_then(Value::as_i64).unwrap_or(-1)
    }
    fn list(&self, step: &str, key: &str) -> Vec<String> {
        self.get(step, key)
            .and_then(Value::as_array)
            .map(|a| a.iter().map(|v| v.to_string()).collect())
            .unwrap_or_default()
    }

    /// Returns `Err(why)` when the principle does not hold.
    fn principle(&self, p: u8) -> Result<(), String> {
        let want = |got: String, want: &str, what: &str| -> Result<(), String> {
            if got == want {
                Ok(())
            } else {
                Err(format!("{what}: expected `{want}`, got `{got}`"))
            }
        };
        match p {
            // 1. A boundary routes: the people who hold it may act, someone
            //    with only wider standing may not, and a boundary nobody
            //    holds refuses rather than defaulting to whoever asked.
            1 => {
                want(self.s("boundary-an-insider-decides", "authority"), "act",
                     "a holder of the inner boundary")?;
                want(self.s("boundary-an-outsider-does-not", "authority"), "ask-one",
                     "someone with only wider standing")?;
                want(self.s("boundary-nobody-holds-this", "authority"), "refuse",
                     "a boundary nobody holds")
            }
            // 2. What came from upstream and what this community wrote for
            //    itself are both visible, and both non-empty. The divergence
            //    IS the congruence.
            2 => {
                let (i, l) = (self.n("congruence-forked-and-diverged", "inherited"),
                              self.n("congruence-forked-and-diverged", "local"));
                if i > 0 && l > 0 { Ok(()) }
                else { Err(format!("inherited {i}, local {l} — one of them is empty")) }
            }
            // 3. The people governed by a rule changed that rule, and the
            //    change is what decided the next thing.
            3 => {
                let before = self.s("boundary-who-holds-the-inner", "policy");
                let after = self.s("nesting-the-inner", "policy");
                if before == after {
                    return Err(format!("the inner rule is still `{before}`; nobody changed it"));
                }
                let decided = self.s("collective-choice-under-the-new-rule", "rule");
                if decided != after {
                    return Err(format!("changed to `{after}` but decided under `{decided}`"));
                }
                Ok(())
            }
            // 4. The monitor's reading is welcome, its ADJUDICATION is named
            //    to the community, and its standing lapses without anyone
            //    having to remember.
            4 => {
                // Attribution, which has two branches and needs both. A
                // machine's adjudication has no person behind it and
                // `unattended` is what says so; a person's adjudication
                // already carries their name, and `unattended` reporting it
                // would be reporting a person as unattributed. Requiring the
                // first branch everywhere would be requiring every commons
                // to monitor with a bot — the opposite of the principle.
                let by_machine = self.vocab["monitor"]
                    .as_str()
                    .is_some_and(|m| m.starts_with("agent:"));
                let surfaced = self.list("monitors-adjudication-is-surfaced", "unattended");
                match (by_machine, surfaced.is_empty()) {
                    (true, true) => {
                        return Err("a machine adjudicated and nothing surfaced it".into())
                    }
                    (false, false) => {
                        return Err(format!(
                            "a person adjudicated and it was reported unattended: {surfaced:?}"
                        ))
                    }
                    _ => {}
                }
                if self.n("monitors-standing-lapsed", "count") < 1 {
                    return Err("the monitor's standing does not lapse on its own".into());
                }
                let (pos, dec) = (self.n("monitors-record-is-queryable", "positions"),
                                  self.n("monitors-record-is-queryable", "decided"));
                if pos < 1 || dec != 0 {
                    return Err(format!("record is {pos} position(s), {dec} decision(s)"));
                }
                Ok(())
            }
            // 5. Mild first, escalating — counted from DECISIONS, and a
            //    different subject starts at the bottom again.
            5 => {
                let rung = |name: &str| RUNGS.iter().position(|r| *r == self.s(name, "authority"));
                let (a, b, c) = (rung("graduated-first"), rung("graduated-second"),
                                 rung("graduated-third"));
                match (a, b, c) {
                    (Some(a), Some(b), Some(c)) if a < b && b < c => {}
                    _ => return Err("the three occurrences do not escalate".into()),
                }
                if rung("graduated-a-different-subject-restarts") != a {
                    return Err("a different subject did not start at the bottom".into());
                }
                Ok(())
            }
            // 6. A conflict is surfaced with both sides cited and carried
            //    knowingly in ONE act — no meeting, no model call.
            6 => {
                want(self.s("conflict-surfaced", "outcome"), "conflicts", "the clash")?;
                if self.list("conflict-surfaced", "cites").len() < 2 {
                    return Err("the conflict does not cite both sides".into());
                }
                let d_tol = self.n("conflict-after", "tolerated")
                    - self.n("conflict-before", "tolerated");
                let d_acts = self.n("conflict-after", "acts") - self.n("conflict-before", "acts");
                if d_tol != 1 || d_acts != 1 {
                    return Err(format!(
                        "carrying it cost {d_acts} act(s) and moved tolerated by {d_tol}"));
                }
                Ok(())
            }
            // 7. Upstream shipped a new generation. What this community wrote
            //    for itself is still here.
            7 => {
                let (before, after) = (self.n("congruence-forked-and-diverged", "local"),
                                       self.n("organize-the-fork-keeps-what-it-wrote", "local"));
                if after < before {
                    return Err(format!("wrote {before} of its own, kept {after}"));
                }
                let (g0, g1) = (self.s("congruence-forked-and-diverged", "generation"),
                                self.s("organize-the-fork-keeps-what-it-wrote", "generation"));
                if g0 == g1 {
                    return Err("upstream never shipped, so nothing was tested".into());
                }
                Ok(())
            }
            // 8. Each level has its own deciders AND its own rule.
            8 => {
                // Where a commons has a third level, it has to be a real one:
                // its own holders, distinct from both the level above and the
                // level below. Two levels was the shape of the first fixtures
                // and is not the shape of the principle.
                if self.run.contains_key("nesting-the-middle") {
                    let mid = self.list("nesting-the-middle", "holders");
                    for (other, which) in [("nesting-the-inner", "inner"),
                                           ("nesting-the-outer", "outer")] {
                        if mid == self.list(other, "holders") {
                            return Err(format!(
                                "the middle level is held by the same people as the {which}"
                            ));
                        }
                    }
                }
                let (inner, outer) = (self.list("nesting-the-inner", "holders"),
                                      self.list("nesting-the-outer", "holders"));
                if inner == outer {
                    return Err("the inner and outer levels are held at the same depth, \
                                so there is only one level".into());
                }
                if inner.len() >= self.list("nesting-the-inner", "deciders").len() {
                    return Err("the inner level is not narrower than what reaches it".into());
                }
                let (pi, po) = (self.s("nesting-the-inner", "policy"),
                                self.s("nesting-the-outer", "policy"));
                if pi == po {
                    return Err(format!("both levels are decided by `{pi}`"));
                }
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn predicted_to_fail(&self) -> BTreeSet<u8> {
        self.vocab["predicted_to_fail"]
            .as_array()
            .map(|a| a.iter().filter_map(|v| v.as_u64().map(|n| n as u8)).collect())
            .unwrap_or_default()
    }

    /// Principles this commons says do not apply to it, with the reason.
    ///
    /// A commons founded rather than forked has no upstream, and principles 2
    /// and 7 are both about divergence from an upstream. Forcing every
    /// institution to be a fork so that the table stays green would be
    /// fitting the study to the instrument. The escape is narrow on purpose:
    /// the reason is written down, at most two may be declared, a declared
    /// principle must actually FAIL (or the declaration is wrong), and the
    /// study as a whole still has to demonstrate every principle in at least
    /// eight of its institutions.
    fn not_applicable(&self) -> BTreeMap<u8, String> {
        self.vocab["not_applicable"]
            .as_object()
            .map(|m| {
                m.iter()
                    .filter_map(|(k, v)| {
                        Some((k.parse().ok()?, v.as_str().unwrap_or_default().to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The steps this commons runs, minus the anonymous acts that carry it
    /// between them.
    fn named_steps(&self) -> Vec<String> {
        self.steps
            .iter()
            .filter(|s| *s != "act" && *s != "clock")
            .cloned()
            .collect()
    }

    fn has_middle(&self) -> bool {
        self.vocab.get("middle_leaf").is_some()
    }
}

/// Steps a commons runs only if it has the shape for them.
const SHAPE_STEPS: [&str; 1] = ["nesting-the-middle"];

// ── the verdict ─────────────────────────────────────────────

#[test]
fn transfer_bar() {
    let all = fixtures();
    let mut failures: Vec<String> = Vec::new();
    let mut rows: Vec<(String, bool, BTreeSet<u8>, BTreeSet<u8>)> = Vec::new();

    // A. The spine is one spine, and the only steps that may differ between
    //    institutions are the ones a SHAPE turns on — a third level of
    //    nesting either exists or it does not. Everything else is identical,
    //    in the same order, in a makerspace and an alpine pasture.
    let positives: Vec<&Fixture> =
        all.iter().filter(|f| f.predicted_to_fail().is_empty()).collect();
    let core = |f: &Fixture| -> Vec<String> {
        f.named_steps()
            .into_iter()
            .filter(|s| !SHAPE_STEPS.contains(&s.as_str()))
            .collect()
    };
    let spine = core(positives[0]);
    for f in &positives {
        if core(f) != spine {
            failures.push(format!(
                "{}: runs a different spine from {} — the study's whole claim is that it does not",
                f.name, positives[0].name
            ));
        }
        // A shape step is present exactly when the shape is, in both
        // directions: a commons cannot claim a third level it does not run,
        // and cannot run one it did not declare.
        let runs_middle = f.named_steps().iter().any(|s| s == "nesting-the-middle");
        if runs_middle != f.has_middle() {
            failures.push(format!(
                "{}: declares middle_leaf={} but {} a middle level",
                f.name,
                f.has_middle(),
                if runs_middle { "runs" } else { "does not run" }
            ));
        }
    }

    // A2. The study is only worth n institutions if they are n SHAPES. Ten
    //     vocabularies over one shape is one institution in ten coats of
    //     paint, and would make every result below a restatement that
    //     renaming strings changes nothing.
    let shapes: BTreeSet<(usize, usize, bool, bool, bool)> = positives
        .iter()
        .map(|f| {
            let n = |k: &str| f.vocab[k].as_array().map(Vec::len).unwrap_or(0);
            (
                n("members"),
                n("insiders"),
                f.has_middle(),
                f.vocab["monitor"].as_str().is_some_and(|m| m.starts_with("agent:")),
                f.vocab["forked"].as_bool().unwrap_or(true),
            )
        })
        .collect();
    if shapes.len() < positives.len() {
        failures.push(format!(
            "{} institutions but only {} distinct shapes (members, holders, depth, monitor \
             kind, ancestry) — the duplicates are not independent evidence",
            positives.len(),
            shapes.len()
        ));
    }

    // B. A vocabulary names nouns and cannot choose mechanism.
    for f in &all {
        let mut found = Vec::new();
        walk(&f.vocab, &mut |k| {
            if RESERVED.contains(&k) {
                found.push(k.to_string());
            }
        });
        if !found.is_empty() {
            failures.push(format!(
                "{}: vocabulary sets typed governance {found:?} — a vocabulary may name a \
                 resource, never a rule",
                f.name
            ));
        }
    }

    // C. The eight, in every institution — and red exactly where predicted.
    let mut held: BTreeMap<u8, usize> = BTreeMap::new();
    for f in &all {
        let predicted = f.predicted_to_fail();
        let na = f.not_applicable();
        if na.len() > 2 {
            failures.push(format!(
                "{}: declares {} principles inapplicable; two is the most a shape can excuse",
                f.name,
                na.len()
            ));
        }
        for (p, why) in &na {
            if why.trim().is_empty() {
                failures.push(format!("{}: Ostrom {p} is declared inapplicable with no reason",
                                      f.name));
            }
        }
        let mut failed = BTreeSet::new();
        for p in 1u8..=8 {
            let outcome = f.principle(p);
            if outcome.is_ok() && predicted.is_empty() {
                *held.entry(p).or_default() += 1;
            }
            if let Some(why) = na.get(&p) {
                // A principle you excused yourself from had better actually
                // not hold. If it holds anyway, the excuse is wrong and the
                // institution should be scored on it like everybody else.
                if outcome.is_ok() {
                    failures.push(format!(
                        "{}: declares Ostrom {p} inapplicable ({why}) but it holds — \
                         drop the declaration",
                        f.name
                    ));
                }
                continue;
            }
            if let Err(why) = outcome {
                failed.insert(p);
                if !predicted.contains(&p) {
                    failures.push(format!(
                        "{}: Ostrom {p} ({}) does not hold — {why}",
                        f.name, NAMES[p as usize - 1]
                    ));
                }
            }
        }
        for p in &predicted {
            if !failed.contains(p) {
                failures.push(format!(
                    "{}: predicted to lose Ostrom {p} ({}) and did not. The ablation removed a \
                     primitive and the principle survived it, so the criterion is not measuring \
                     what it claims to.",
                    f.name, NAMES[*p as usize - 1]
                ));
            }
        }
        rows.push((f.name.clone(), predicted.is_empty(), failed, na.keys().copied().collect()));
    }

    // D. `not_applicable` must stay an exception. Every principle has to be
    //    demonstrated by most of the study or the escape hatch is the result.
    let floor = positives.len() - 2;
    for p in 1u8..=8 {
        let n = held.get(&p).copied().unwrap_or(0);
        if n < floor {
            failures.push(format!(
                "Ostrom {p} ({}) holds in only {n} of {} institutions; {floor} is the floor",
                NAMES[p as usize - 1],
                positives.len()
            ));
        }
    }

    print_table(&rows, shapes.len());
    assert!(
        failures.is_empty(),
        "the transfer bar did not clear:\n  {}",
        failures.join("\n  ")
    );
}

fn walk(v: &Value, f: &mut impl FnMut(&str)) {
    match v {
        Value::Object(m) => {
            for (k, val) in m {
                f(k);
                walk(val, f);
            }
        }
        Value::Array(a) => a.iter().for_each(|x| walk(x, f)),
        _ => {}
    }
}

fn print_table(rows: &[(String, bool, BTreeSet<u8>, BTreeSet<u8>)], shapes: usize) {
    println!("\nOstrom's eight, over {} institutions built from one spine", rows.len());
    println!("{:<28} {:<9}  1 2 3 4 5 6 7 8", "institution", "kind");
    for (name, positive, failed, na) in rows {
        let marks: Vec<&str> = (1u8..=8)
            .map(|p| {
                if na.contains(&p) {
                    "n"
                } else if failed.contains(&p) {
                    "x"
                } else {
                    "."
                }
            })
            .collect();
        println!(
            "{name:<28} {:<9}  {}",
            if *positive { "resource" } else { "ablation" },
            marks.join(" ")
        );
    }
    let (res, abl) = (rows.iter().filter(|r| r.1).count(), rows.iter().filter(|r| !r.1).count());
    println!("\n`.` holds   `x` does not   `n` declared inapplicable, with a reason");
    println!(
        "{res} resources in {shapes} distinct shapes; {abl} ablations, red where each predicted."
    );
}

/// **Which decisions a rule change touches does not depend on the domain.**
///
/// `canon replay --policy X` re-decides a whole history under a rule the
/// community did not adopt. The SET of (step, field) pairs that come out
/// differently is byte identical across ten institutions of ten different
/// shapes — 6 to 24 people, two levels or three, forked or founded.
///
/// **Read this narrowly.** It says *where* a policy change lands is a
/// property of the policy and the spine, not of the nouns. It does not say
/// the decisions themselves are identical, and it is not by itself evidence
/// that the eight principles generalise — the ablation table is that. The
/// null control below is what stops it being a restatement that the
/// generator is deterministic.
#[test]
fn the_counterfactual_is_identical_across_institutions() {
    // Null control, first: a metric that returns the same answer for
    // everything measures nothing. Two of the four ablations change the
    // structure a policy reads, and the signature has to notice.
    let all = fixtures();
    let sig = |f: &Fixture, rule: &str| -> Vec<String> { divergence(&f.name, rule) };
    let base = sig(
        all.iter().find(|f| f.name == "harbourside-makerspace").expect("baseline"),
        "default",
    );
    for name in ["harbourside-no-boundary", "meridian-imposed-rules"] {
        let f = all.iter().find(|f| f.name == name).expect("ablation");
        assert_ne!(
            sig(f, "default"),
            base,
            "{name} removed a primitive a policy reads and the divergence signature did not \
             change — the signature is insensitive to structure, so its agreeing across ten \
             institutions would mean nothing"
        );
    }

    for rule in ["default", "consent", "subsidiarity"] {
        let mut seen: BTreeMap<Vec<String>, Vec<String>> = BTreeMap::new();
        for f in all.iter().filter(|f| f.predicted_to_fail().is_empty()) {
            let diverged = divergence(&f.name, rule);
            assert!(!diverged.is_empty(), "{}: --policy {rule} changed nothing", f.name);
            seen.entry(diverged).or_default().push(f.name.clone());
        }
        assert_eq!(
            seen.len(), 1,
            "--policy {rule} diverges differently across institutions, so the outcomes are \
             the domain's and not the rule's: {:?}",
            seen.values().collect::<Vec<_>>()
        );
        let (pairs, who) = seen.into_iter().next().unwrap();
        println!(
            "--policy {rule:<13} {} divergences, one signature, {} institutions",
            pairs.len(), who.len()
        );
    }
}
