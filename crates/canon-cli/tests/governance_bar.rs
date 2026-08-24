// SPDX-License-Identifier: AGPL-3.0-or-later
//! The governance bar — **can a community implement Ostrom's eight design
//! principles with these primitives and nothing else?**
//!
//! That is the acceptance test for the whole primitive set, and it is narrow
//! on purpose. "The fixture demonstrates some advanced applications" is
//! unfalsifiable. The eight principles are empirically derived from
//! long-enduring common-pool-resource institutions, somebody else derived
//! them, and a principle that cannot be demonstrated here is a finding about
//! the primitives rather than a scenario to reword.
//!
//! **Not `#[ignore]`.** It runs in `cargo test` because it needs no endpoint:
//! the governance layer is `Log -> Canon -> policy -> Decision` and every step
//! is decided by code. The positions a model would have produced are written
//! into the fixture. If this ever needs a model, the split between extraction
//! and decision has been broken and that is the thing to fix.
//!
//! **Two fixtures, and the pair is the claim.** One house, one codebase. A
//! single fixture demonstrates a feature; two demonstrate a substrate.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

// ── the bars, pre-registered ────────────────────────────────
//
// Written from the design and the fixture READMEs before the scorer existed.

/// All eight, in both fixtures. Seven of eight would be a finding, not a pass.
const PRINCIPLES: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

/// A step claiming the tool PROVIDES something must assert it. `affordance`
/// steps may be descriptive.
const MECHANISM: &str = "mechanism";

/// Keys that pin WHICH, not merely how many.
///
/// A principle demonstrated only by counts has shown that a number came out,
/// not that the right thing did — `overdue: 1` passes whether the lapsed
/// grant is the agent's or somebody else's. Every mechanism principle has to
/// name something: the rule that fired, the commitment cited, the act
/// surfaced, the people routed to, or the names drawn.
const IDENTIFYING: [&str; 7] = [
    "because",
    "cites",
    "targets",
    "unattended",
    "deciders",
    "voices",
    "seats",
];

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures")
}

struct Step {
    name: String,
    kind: String,
    principle: Option<u8>,
    strength: Option<String>,
}

fn read_scenario(dir: &Path) -> Vec<Step> {
    let raw = std::fs::read_to_string(dir.join("scenario.jsonl"))
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()));
    raw.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//"))
        .map(|l| {
            let v: Value = serde_json::from_str(l).expect("scenario line is JSON");
            let kind = v["step"].as_str().expect("step").to_string();
            Step {
                name: v["name"].as_str().unwrap_or(&kind).to_string(),
                kind,
                principle: v["principle"].as_u64().map(|n| n as u8),
                strength: v["strength"].as_str().map(str::to_string),
            }
        })
        .collect()
}

fn read_expected(dir: &Path) -> BTreeMap<String, Value> {
    let raw = std::fs::read_to_string(dir.join("expected.json")).expect("expected.json");
    let Value::Object(m) = serde_json::from_str(&raw).expect("expected.json is JSON") else {
        panic!("expected.json is not an object")
    };
    m.into_iter().collect()
}

/// Run the real verb, through the real binary.
fn replay(dir: &Path) -> (bool, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .arg("replay")
        .arg(dir)
        .output()
        .expect("canon replay runs");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.success(), text)
}

// ── the verdict ─────────────────────────────────────────────

#[test]
fn ostrom_bar() {
    let mut table: BTreeMap<u8, BTreeMap<&'static str, (String, usize)>> = BTreeMap::new();
    let mut failures: Vec<String> = Vec::new();

    for fixture in ["fernwood-commons", "eleven-principles"] {
        let dir = fixtures().join(fixture);
        let steps = read_scenario(&dir);
        let expected = read_expected(&dir);

        // 1. The scenario does what it says it does.
        let (ok, text) = replay(&dir);
        if !ok {
            failures.push(format!("{fixture}: `canon replay` did not pass\n{text}"));
        }

        // 2. Every `mechanism` step is ASSERTED. A step that claims the tool
        //    provides something and checks nothing is a sentence, not a test.
        for s in &steps {
            let Some(p) = s.principle else { continue };
            let strength = s.strength.clone().unwrap_or_default();
            let entry = table
                .entry(p)
                .or_default()
                .entry(fixture)
                .or_insert((strength.clone(), 0));
            entry.1 += 1;
            if entry.0 != strength {
                failures.push(format!(
                    "{fixture}: principle {p} is marked `{}` on one step and `{strength}` on \
                     another — a mark is about the principle, not the step",
                    entry.0
                ));
            }
            if strength != MECHANISM {
                continue;
            }
            let Some(want) = expected.get(&s.name).and_then(Value::as_object) else {
                failures.push(format!(
                    "{fixture}: `{}` claims Ostrom {p} as a MECHANISM and expected.json \
                     asserts nothing about it",
                    s.name
                ));
                continue;
            };
            // 3. A `check` asserts the outcome AND the authority. Half of the
            //    answer passing is how a right answer for the wrong reason
            //    survives: two policies reach `conflicts` and disagree
            //    completely about what you may then do.
            if s.kind == "check" {
                for key in ["outcome", "authority"] {
                    if !want.contains_key(key) {
                        failures.push(format!(
                            "{fixture}: `{}` is a MECHANISM check that does not assert `{key}`",
                            s.name
                        ));
                    }
                }
            }
        }

        // 4. Every principle marked `mechanism` names the rule that fired or
        //    the commitment cited, somewhere. Otherwise the scenario proves a
        //    verdict arrived and not that it arrived for the stated reason.
        let mut reasoned: BTreeSet<u8> = BTreeSet::new();
        for s in &steps {
            let (Some(p), Some(want)) = (
                s.principle,
                expected.get(&s.name).and_then(Value::as_object),
            ) else {
                continue;
            };
            if IDENTIFYING.iter().any(|k| want.contains_key(*k)) {
                reasoned.insert(p);
            }
        }
        for p in PRINCIPLES {
            let marks = table.get(&p).and_then(|m| m.get(fixture));
            match marks {
                None => failures.push(format!(
                    "{fixture}: Ostrom principle {p} is not demonstrated at all"
                )),
                Some((strength, _)) if strength == MECHANISM && !reasoned.contains(&p) => {
                    failures.push(format!(
                        "{fixture}: Ostrom {p} is claimed as a MECHANISM but no step pins \
                         WHICH — add one of {IDENTIFYING:?}"
                    ));
                }
                _ => {}
            }
        }
    }

    // 5. **A mark may not be downgraded to make a table green.** If a
    //    principle is a mechanism in the house and an affordance in the
    //    codebase, one of the two is wrong, and the plan says a principle
    //    that cannot be demonstrated is recorded as a FINDING in
    //    PRIMITIVES.md rather than quietly reclassified.
    for (p, per_fixture) in &table {
        let marks: BTreeSet<&str> = per_fixture.values().map(|(s, _)| s.as_str()).collect();
        if marks.len() > 1 {
            failures.push(format!(
                "Ostrom {p} is marked {marks:?} in different fixtures — a principle does not \
                 change strength because the institution changed"
            ));
        }
    }

    print_table(&table);

    assert!(
        failures.is_empty(),
        "the governance bar did not clear:\n  {}",
        failures.join("\n  ")
    );
}

fn print_table(table: &BTreeMap<u8, BTreeMap<&'static str, (String, usize)>>) {
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
    println!("\nOstrom's eight design principles, over two institutions");
    println!(
        "{:<3} {:<38} {:<12} {:>6} {:>6}",
        "#", "principle", "strength", "house", "code"
    );
    let mut mechanism = 0;
    for (i, name) in NAMES.iter().enumerate() {
        let p = (i + 1) as u8;
        let row = table.get(&p);
        let strength = row
            .and_then(|m| m.values().next())
            .map(|(s, _)| s.clone())
            .unwrap_or_else(|| "MISSING".into());
        if strength == MECHANISM {
            mechanism += 1;
        }
        let count = |f: &str| {
            row.and_then(|m| m.get(f))
                .map(|(_, n)| n.to_string())
                .unwrap_or_else(|| "-".into())
        };
        println!(
            "{p:<3} {name:<38} {strength:<12} {:>6} {:>6}",
            count("fernwood-commons"),
            count("eleven-principles")
        );
    }
    // Both numbers, always. A table reporting only the mechanisms would be
    // reporting the flattering half.
    println!(
        "\n{mechanism} of 8 are mechanisms, {} are affordances — the tool permits them and \
         gets out of the way.",
        8 - mechanism
    );
}

/// The claim that makes the fixtures fast, asserted rather than assumed.
#[test]
fn the_case_studies_never_reach_an_endpoint() {
    // A fixture that grew a model call would still pass `ostrom_bar` — it
    // would just be slow, and then someone would mark it `#[ignore]`, and the
    // acceptance test for the whole primitive set would stop running. So the
    // property is pinned where it can fail loudly: no fixture file may name a
    // model, an endpoint, or a URL that is not the lineage it forked from.
    for fixture in ["fernwood-commons", "eleven-principles"] {
        let dir = fixtures().join(fixture);
        for name in ["acts.jsonl", "scenario.jsonl"] {
            let raw = std::fs::read_to_string(dir.join(name)).expect("fixture file");
            // The ACTS, not the prose about them — the first cut of this
            // failed on its own comment explaining that there is no endpoint.
            let acts: String = raw
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
            for needle in ["endpoint", "localhost", "127.0.0.1", "\"model\""] {
                assert!(
                    !acts.contains(needle),
                    "{fixture}/{name} mentions `{needle}` — the decision layer is pure and \
                     these fixtures are what proves it"
                );
            }
        }
    }
}

/// Replay is a counterfactual too, and that has to keep working.
#[test]
fn a_forced_policy_re_decides_the_whole_history() {
    // "What would consent have done to the last six months?" is the question
    // a group has before changing how it decides, and it is the reason
    // `replay` is a verb rather than a test harness.
    let dir = fixtures().join("fernwood-commons");
    let out = Command::new(env!("CARGO_BIN_EXE_canon"))
        .args(["replay", dir.to_str().unwrap(), "--policy", "default"])
        .output()
        .expect("runs");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("decided under a forced rule: default"),
        "a counterfactual has to say it is one:\n{text}"
    );
    // Under the shipped default the sabotage proposal is NOT refused — it
    // asks a person instead. Same evidence, different rule, different answer,
    // which is the whole point of the policy layer.
    assert!(
        text.contains("a-sabotage-proposal-dies-on-unaddressed\n  authority=ask-one"),
        "the forced rule did not actually re-decide:\n{text}"
    );
}
