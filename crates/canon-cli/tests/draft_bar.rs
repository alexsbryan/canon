// SPDX-License-Identifier: AGPL-3.0-or-later
//! The Maple House bar — does standalone `draft` actually find planted
//! tensions, and can it tell a decoy from a conflict?
//!
//! **This scores by REPLAY, never by re-running the model.** It reads the
//! artifacts `canon draft --dry-run` persists to `.canon/draft-runs/*.json`
//! and scores them against `fixtures/maple-house/truth.json`. A run that
//! cannot be re-scored without a second inference call is not instrumented
//! (§18.4), and scoring against different evidence than the tool consumed
//! measures the wrong thing.
//!
//! `#[ignore]` by default: it needs artifacts, which need an endpoint.
//!
//! ```sh
//! ./scripts/draft-bar.sh 3                       # produce three runs
//! cargo test --test draft_bar -- --ignored --nocapture
//! ```
//!
//! **A single run is not a measurement** (§18.5). Fewer than
//! [`MIN_RUNS`] artifacts is a hard failure rather than a number, because the
//! spread between repeat runs against the same document is the noise floor
//! every published figure has to clear.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

// ── the bars, pre-registered ────────────────────────────────
//
// Written before the first run existed, which is the only order in which a
// bar means anything. The spec's claim is narrow — "standalone degrades on
// precision; it does not fail" — so precision is REPORTED and not gated,
// while the two ways the claim could be false are gated.

/// Below this, `draft` finds almost nothing that was planted, and the cold
/// start it exists for does not happen.
const KILL_RECALL_FLOOR: f64 = 0.30;

/// Of the seven labeled compatible pairs, flagging this many means it cannot
/// tell a decoy from a conflict — the failure the spec calls "collapses on
/// the decoys", where `draft` ships daemon-only or not at all.
const KILL_DECOY_CEILING: usize = 5;

/// One run is an anecdote.
const MIN_RUNS: usize = 3;

// ── scoring ─────────────────────────────────────────────────

/// A section of the source document: `article:II` or `date:2026-02-10`.
/// `truth.json` keys every labeled pair by one of these, and they are unique
/// within the document.
fn section_key(heading: &str) -> Option<String> {
    if let Some(date) = heading.split_whitespace().find(|w| {
        let b = w.as_bytes();
        b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && w.chars().filter(|c| c.is_ascii_digit()).count() == 8
    }) {
        return Some(format!("date:{date}"));
    }
    let after = heading.split("Article ").nth(1)?;
    let numeral: String = after
        .chars()
        .take_while(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C'))
        .collect();
    (!numeral.is_empty()).then(|| format!("article:{numeral}"))
}

/// `{"article": "II"}` / `{"date": "2026-02-10"}` from the manifest.
fn truth_key(side: &Value) -> Option<String> {
    if let Some(a) = side.get("article").and_then(Value::as_str) {
        return Some(format!("article:{a}"));
    }
    side.get("date")
        .and_then(Value::as_str)
        .map(|d| format!("date:{d}"))
}

/// Unordered pair, so `(a,b)` and `(b,a)` are one entry.
fn pair(a: &str, b: &str) -> (String, String) {
    if a <= b {
        (a.into(), b.into())
    } else {
        (b.into(), a.into())
    }
}

#[derive(Debug, Default)]
struct Score {
    run: String,
    candidates: usize,
    dropped: usize,
    /// Distinct CROSS-section pairs the run proposed. Pairs inside one
    /// section are excluded and counted separately: `truth.json` labels only
    /// cross-section pairs, so scoring an intra-section pair either way would
    /// be scoring against a label that does not exist.
    proposed: BTreeSet<(String, String)>,
    intra_section: usize,
    unmapped: usize,
    hits: BTreeSet<String>,
    decoys: BTreeSet<String>,
}

impl Score {
    fn precision(&self) -> f64 {
        if self.proposed.is_empty() {
            return 0.0;
        }
        self.hits.len() as f64 / self.proposed.len() as f64
    }
    fn recall(&self, planted: usize) -> f64 {
        if planted == 0 {
            return 0.0;
        }
        self.hits.len() as f64 / planted as f64
    }
}

fn score_run(path: &Path, truth: &Value) -> Score {
    let raw = std::fs::read_to_string(path).expect("read run");
    let run: Value = serde_json::from_str(&raw).expect("run is JSON");
    assert_eq!(
        run["schema"].as_str(),
        Some("canon-draft-run/v1"),
        "{} is not a draft run this bar understands",
        path.display()
    );

    let chunks = run["chunks"].as_array().expect("chunks");
    let candidates = run["candidates"].as_array().expect("candidates");
    let kept = run["kept"].as_array().expect("kept");

    // candidate position (within `kept`) -> section key
    let section_of = |kept_pos: usize| -> Option<String> {
        let ci = kept.get(kept_pos)?.as_u64()? as usize;
        let chunk = chunks.get(candidates.get(ci)?["chunk"].as_u64()? as usize)?;
        section_key(chunk["heading"].as_str()?)
    };

    let mut s = Score {
        run: path.file_name().unwrap().to_string_lossy().into(),
        candidates: kept.len(),
        dropped: run["dropped"].as_array().map(Vec::len).unwrap_or(0),
        ..Score::default()
    };
    for t in run["tensions"].as_array().expect("tensions") {
        let (Some(a), Some(b)) = (
            t["a"].as_u64().and_then(|i| section_of(i as usize)),
            t["b"].as_u64().and_then(|i| section_of(i as usize)),
        ) else {
            s.unmapped += 1;
            continue;
        };
        if a == b {
            s.intra_section += 1;
            continue;
        }
        s.proposed.insert(pair(&a, &b));
    }

    for p in truth["planted_tensions"].as_array().expect("planted") {
        let (Some(a), Some(b)) = (truth_key(&p["a"]), truth_key(&p["b"])) else {
            continue;
        };
        if s.proposed.contains(&pair(&a, &b)) {
            s.hits.insert(p["id"].as_str().unwrap_or("?").to_string());
        }
    }
    for p in truth["expected_non_tensions"].as_array().expect("non") {
        let (Some(a), Some(b)) = (truth_key(&p["a"]), truth_key(&p["b"])) else {
            continue;
        };
        if s.proposed.contains(&pair(&a, &b)) {
            s.decoys.insert(p["id"].as_str().unwrap_or("?").to_string());
        }
    }
    s
}

fn runs_dir() -> PathBuf {
    match std::env::var("CANON_BAR_RUNS") {
        Ok(d) => PathBuf::from(d),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/maple-house/runs")
            .canonicalize()
            .unwrap_or_else(|_| PathBuf::from("fixtures/maple-house/runs")),
    }
}

fn truth() -> Value {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/maple-house/truth.json");
    serde_json::from_str(&std::fs::read_to_string(&path).expect("truth.json")).expect("truth JSON")
}

#[test]
#[ignore = "needs draft runs: ./scripts/draft-bar.sh 3"]
fn maple_house_bar() {
    let truth = truth();
    let planted = truth["planted_tensions"].as_array().unwrap().len();
    let non = truth["expected_non_tensions"].as_array().unwrap().len();

    let dir = runs_dir();
    // BUILD.txt names the commit and model that produced these artifacts.
    // Printed, not parsed: a number that cannot say which build it describes
    // cannot be compared with anything, including itself next month.
    if let Ok(build) = std::fs::read_to_string(dir.join("BUILD.txt")) {
        println!(
            "
{}",
            build.trim()
        );
    }
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "no runs at {}: {e}\n  produce them: ./scripts/draft-bar.sh 3",
                dir.display()
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    assert!(
        paths.len() >= MIN_RUNS,
        "{} run(s) at {} — a single run is not a measurement (§18.5). Need {MIN_RUNS}.",
        paths.len(),
        dir.display()
    );

    let scores: Vec<Score> = paths.iter().map(|p| score_run(p, &truth)).collect();

    println!("\nMaple House bar — {planted} planted tensions, {non} labeled compatible pairs");
    println!("{} run(s) from {}\n", scores.len(), dir.display());
    println!(
        "{:<22} {:>5} {:>5} {:>7} {:>9} {:>6} {:>7} {:>6}",
        "run", "cand", "drop", "pairs", "precision", "recall", "hits", "decoy"
    );
    for s in &scores {
        println!(
            "{:<22} {:>5} {:>5} {:>7} {:>9.2} {:>6.2} {:>7} {:>6}",
            s.run,
            s.candidates,
            s.dropped,
            s.proposed.len(),
            s.precision(),
            s.recall(planted),
            s.hits.len(),
            s.decoys.len(),
        );
    }

    let mean = |f: &dyn Fn(&Score) -> f64| -> f64 {
        scores.iter().map(f).sum::<f64>() / scores.len() as f64
    };
    let spread = |f: &dyn Fn(&Score) -> f64| -> (f64, f64) {
        let v: Vec<f64> = scores.iter().map(f).collect();
        (
            v.iter().cloned().fold(f64::INFINITY, f64::min),
            v.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let p = mean(&|s| s.precision());
    let r = mean(&|s| s.recall(planted));
    let (pl, ph) = spread(&|s| s.precision());
    let (rl, rh) = spread(&|s| s.recall(planted));
    let worst_decoys = scores.iter().map(|s| s.decoys.len()).max().unwrap_or(0);

    println!("\nprecision  {p:.2}   (noise floor across runs: {pl:.2}–{ph:.2})");
    println!("recall     {r:.2}   (noise floor across runs: {rl:.2}–{rh:.2})");
    println!("decoys flagged, worst run: {worst_decoys} of {non}");

    let hit_ever: BTreeSet<&String> = scores.iter().flat_map(|s| s.hits.iter()).collect();
    let missed: Vec<&str> = truth["planted_tensions"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|t| t["id"].as_str())
        .filter(|id| !hit_ever.iter().any(|h| h.as_str() == *id))
        .collect();
    println!(
        "never found in any run: {}",
        if missed.is_empty() {
            "(none)".into()
        } else {
            missed.join(", ")
        }
    );
    let intra: usize = scores.iter().map(|s| s.intra_section).sum();
    let unmapped: usize = scores.iter().map(|s| s.unmapped).sum();
    println!("excluded: {intra} intra-section pair(s), {unmapped} unmappable\n");

    // The pre-registered bars, applied last so the numbers print either way.
    assert!(
        r >= KILL_RECALL_FLOOR,
        "KILL: mean recall {r:.2} is below the pre-registered floor {KILL_RECALL_FLOOR:.2} — \
         standalone draft finds almost nothing that was planted"
    );
    assert!(
        worst_decoys < KILL_DECOY_CEILING,
        "KILL: {worst_decoys} of {non} labeled compatible pairs flagged as tensions — \
         standalone draft cannot tell a decoy from a conflict"
    );
}

#[test]
fn section_keys_parse_out_of_the_documents_own_headings() {
    // Not ignored: the scorer's own instrument, validated before the result
    // it produces (§18.4). A section_key that silently returned None would
    // report every pair as unmappable and every score as zero.
    assert_eq!(
        section_key("Maple House Charter, Article II — Quiet Hours").as_deref(),
        Some("article:II")
    );
    assert_eq!(
        section_key("Maple House Charter, Article XI — Quiet Study Hours").as_deref(),
        Some("article:XI")
    );
    assert_eq!(
        section_key("Decision — 2026-02-10 — Weeknight Quiet Hours").as_deref(),
        Some("date:2026-02-10")
    );
    assert_eq!(section_key("Just some prose"), None);
}

#[test]
fn every_labeled_pair_in_the_manifest_maps_to_a_heading_in_the_document() {
    // If truth.json names a section the document does not have, every score
    // computed against it is silently wrong. Validate the instrument first.
    let truth = truth();
    let doc = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/maple-house/maple-house.md"),
    )
    .expect("maple-house.md");
    let present: BTreeSet<String> = doc
        .lines()
        .filter(|l| l.starts_with('#'))
        .filter_map(|l| section_key(l.trim_start_matches('#').trim()))
        .collect();
    assert_eq!(present.len(), 24, "expected 24 sections, found {present:?}");

    for group in ["planted_tensions", "expected_non_tensions"] {
        for p in truth[group].as_array().unwrap() {
            for side in ["a", "b"] {
                let key = truth_key(&p[side]).expect("a keyed side");
                assert!(
                    present.contains(&key),
                    "{group} {} names {key}, which is not a heading in maple-house.md",
                    p["id"]
                );
            }
        }
    }
}
