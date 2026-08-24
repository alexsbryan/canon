// SPDX-License-Identifier: AGPL-3.0-or-later
//! The governance bar — does standalone `draft` actually find planted
//! tensions, and can it tell a decoy from a conflict?
//!
//! Two corpora, never blended: `fixtures/maple-house` (a hand-written house
//! charter, train-contaminated) and `fixtures/des-moines-noise` (municipal
//! ordinances, labels the council wrote). `CANON_BAR_TRUTH` and
//! `CANON_BAR_ANCHORS` choose which; a mean over both would be about
//! neither.
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

/// A section of the source document: `article:II`, `date:2026-02-10`,
/// `sec:42-258(6)`, or `ord:16064/42-258(6)`.
///
/// `truth.json` keys every labeled pair by one of these, and they are unique
/// within the document. The last two forms exist because a corpus built from
/// municipal ordinances is keyed by the code section a document amends and by
/// the ordinance that amends it — a charter article numeral cannot say which
/// of two readings of the same section it means.
fn section_key(heading: &str) -> Option<String> {
    // `Sec. 42-258(6)` under an `Ordinance 16,064,` heading names the amending
    // reading; the same section number without one names the codified reading.
    if let Some(sec) = code_section(heading) {
        return Some(match ordinance_number(heading) {
            Some(ord) => format!("ord:{ord}/{sec}"),
            None => format!("sec:{sec}"),
        });
    }
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

/// `Sec. 42-258(6)` -> `42-258(6)`.
fn code_section(heading: &str) -> Option<String> {
    let after = heading.split("Sec. ").nth(1)?;
    let sec: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '-' | '(' | ')'))
        .collect();
    (sec.contains('-')).then_some(sec)
}

/// `Ordinance 16,064,` -> `16064`. Commas are how a clerk writes it and are
/// not part of the identity.
fn ordinance_number(heading: &str) -> Option<String> {
    let after = heading.split("Ordinance ").nth(1)?;
    let num: String = after
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .filter(|c| c.is_ascii_digit())
        .collect();
    (!num.is_empty()).then_some(num)
}

/// `{"article": "II"}` / `{"date": "2026-02-10"}` from the manifest.
fn truth_key(side: &Value) -> Option<String> {
    if let Some(a) = side.get("article").and_then(Value::as_str) {
        return Some(format!("article:{a}"));
    }
    if let Some(sec) = side.get("section").and_then(Value::as_str) {
        return Some(match side.get("ordinance").and_then(Value::as_str) {
            Some(ord) => format!("ord:{ord}/{sec}"),
            None => format!("sec:{sec}"),
        });
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

/// Where a manifest is complete enough to divide by.
///
/// `exhaustive: true` claims the whole document. `exhaustive_within` claims a
/// named REGION and says nothing about the rest — which is the honest shape
/// for a corpus built from one article of a municipal code, where the permit
/// block can be accounted for pair by pair and the general sections cannot.
///
/// The distinction is load-bearing for precision and irrelevant to recall. A
/// planted tension is found or it is not, wherever it sits; but a proposal the
/// manifest does not name is only a FALSE one where the manifest names
/// everything, and counting it as false anywhere else measures the manifest
/// (§18.3).
enum Region {
    Document,
    Within {
        name: String,
        members: BTreeSet<String>,
    },
    Undeclared,
}

impl Region {
    fn read(truth: &Value) -> Self {
        if truth["exhaustive"].as_bool() == Some(true) {
            return Region::Document;
        }
        let w = &truth["exhaustive_within"];
        match (w["region"].as_str(), w["members"].as_array()) {
            (Some(name), Some(members)) => Region::Within {
                name: name.to_string(),
                members: members
                    .iter()
                    .filter_map(|m| m.as_str().map(str::to_string))
                    .collect(),
            },
            _ => Region::Undeclared,
        }
    }

    /// Is this pair one the manifest promises to have labelled?
    fn holds(&self, p: &(String, String)) -> bool {
        match self {
            Region::Document => true,
            Region::Within { members, .. } => members.contains(&p.0) && members.contains(&p.1),
            Region::Undeclared => false,
        }
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
    /// The subset of `proposed` sitting inside the region the manifest calls
    /// complete. This, not `proposed`, is what precision divides by.
    judged: BTreeSet<(String, String)>,
    /// Proposed pairs the manifest makes no promise about. Reported, never
    /// silently folded into either side of the ratio.
    outside: usize,
    intra_section: usize,
    unmapped: usize,
    /// Comparison passes the run attempted, and how many came back unusable.
    /// A tension count from a run with unread passes is a count over a
    /// FRACTION of the pairs, and precision and recall both inherit that.
    passes: usize,
    passes_unread: usize,
    hits: BTreeSet<String>,
    /// Planted tensions found INSIDE the region — precision's numerator, so
    /// that it can never exceed its own denominator.
    hits_judged: usize,
    decoys: BTreeSet<String>,
}

impl Score {
    fn precision(&self) -> f64 {
        if self.judged.is_empty() {
            return 0.0;
        }
        self.hits_judged as f64 / self.judged.len() as f64
    }
    fn recall(&self, planted: usize) -> f64 {
        if planted == 0 {
            return 0.0;
        }
        self.hits.len() as f64 / planted as f64
    }
}

fn score_run(path: &Path, truth: &Value, region: &Region) -> Score {
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

    // **RULES only, and this is what keeps the numbers comparable.**
    // Extraction also mints `question` and `silence` now. Both are real
    // findings about a corpus, neither is a commitment, and counting them
    // here would move precision and recall for a reason that has nothing to
    // do with whether extraction got better — which is exactly how a bar
    // stops measuring the thing it was pre-registered to measure.
    //
    // A run written before kinds existed has no `kind` field and every
    // candidate in it was a rule, so absent reads as `rule`.
    let is_rule = |ci: usize| -> bool {
        candidates
            .get(ci)
            .map(|c| c["kind"].as_str().unwrap_or("rule") == "rule")
            .unwrap_or(false)
    };
    let kept: Vec<Value> = run["kept"]
        .as_array()
        .expect("kept")
        .iter()
        .filter(|k| k.as_u64().is_some_and(|i| is_rule(i as usize)))
        .cloned()
        .collect();
    let kept = &kept;

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
        passes: run["tension_passes"].as_u64().unwrap_or(0) as usize,
        passes_unread: run["tension_passes_unread"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0),
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
        let p = pair(&a, &b);
        if region.holds(&p) {
            s.judged.insert(p.clone());
        } else {
            s.outside += 1;
        }
        s.proposed.insert(p);
    }

    for p in truth["planted_tensions"].as_array().expect("planted") {
        let (Some(a), Some(b)) = (truth_key(&p["a"]), truth_key(&p["b"])) else {
            continue;
        };
        let key = pair(&a, &b);
        if s.proposed.contains(&key) {
            s.hits.insert(p["id"].as_str().unwrap_or("?").to_string());
            if s.judged.contains(&key) {
                s.hits_judged += 1;
            }
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

/// Runs sitting in subdirectories, named in the failure.
///
/// One directory is one instrument: a model, or a build. So "no runs here" is
/// usually "the runs are in one of these", and a bar that will not say which
/// reads as a broken bar rather than a wrong path — the reader's next move is
/// to re-run the sweep they already ran.
fn runs_one_level_down(dir: &Path) -> String {
    let mut found = String::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let p = e.path();
        if !p.is_dir() {
            continue;
        }
        let n = std::fs::read_dir(&p)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|f| f.path().extension().is_some_and(|x| x == "json"))
            .count();
        if n > 0 {
            found.push_str(&format!("\n  CANON_BAR_RUNS={} ({n} run(s))", p.display()));
        }
    }
    if found.is_empty() {
        return String::new();
    }
    format!("\n\nRuns are one level down. Score one instrument, never a mean over two:{found}")
}

fn anchors() -> Value {
    let path = match std::env::var("CANON_BAR_ANCHORS") {
        Ok(p) => PathBuf::from(p),
        Err(_) => PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures/maple-house/extraction-anchors.json"),
    };
    serde_json::from_str(&std::fs::read_to_string(&path).expect("extraction-anchors.json"))
        .expect("anchors JSON")
}

fn truth() -> Value {
    let path = match std::env::var("CANON_BAR_TRUTH") {
        Ok(p) => PathBuf::from(p),
        Err(_) => {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/maple-house/truth.json")
        }
    };
    serde_json::from_str(&std::fs::read_to_string(&path).expect("truth.json")).expect("truth JSON")
}

#[test]
#[ignore = "needs draft runs: ./scripts/draft-bar.sh 3"]
fn governance_bar() {
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

    // A run that stopped at a stage is evidence, not a measurement: the
    // stages after it never ran. `draft` keeps those artifacts on purpose
    // rather than discarding the work, so the bar has to be the thing that
    // refuses them — silently averaging one in would publish a number over a
    // pipeline that did not finish (§18.3).
    let abandoned: Vec<(PathBuf, String)> = paths
        .iter()
        .filter_map(|p| {
            let raw = std::fs::read_to_string(p).ok()?;
            let v: Value = serde_json::from_str(&raw).ok()?;
            let why = v["failed"].as_str()?.to_string();
            Some((p.clone(), why))
        })
        .collect();
    if !abandoned.is_empty() {
        println!("\nnot scored — these runs stopped before the pipeline finished:");
        for (p, why) in &abandoned {
            println!("  {}  {why}", p.file_name().unwrap().to_string_lossy());
        }
        let keep: BTreeSet<&PathBuf> = abandoned.iter().map(|(p, _)| p).collect();
        paths.retain(|p| !keep.contains(p));
    }

    assert!(
        paths.len() >= MIN_RUNS,
        "{} run(s) at {} — a single run is not a measurement (§18.5). Need {MIN_RUNS}.{}",
        paths.len(),
        dir.display(),
        runs_one_level_down(&dir)
    );

    let region = Region::read(&truth);
    let scores: Vec<Score> = paths
        .iter()
        .map(|p| score_run(p, &truth, &region))
        .collect();

    // Name the corpus the manifest names. A banner that says "Maple House"
    // while scoring an ordinance is a number about the wrong document, and
    // the reader has no way to tell (§18.3).
    let corpus = truth["corpus_id"].as_str().unwrap_or("(unnamed corpus)");
    println!("\n{corpus} bar — {planted} planted tensions, {non} labeled compatible pairs");
    println!("{} run(s) from {}\n", scores.len(), dir.display());
    println!(
        "{:<22} {:>5} {:>5} {:>7} {:>7} {:>9} {:>6} {:>7} {:>6}",
        "run", "cand", "drop", "pairs", "judged", "precision", "recall", "hits", "decoy"
    );
    for s in &scores {
        println!(
            "{:<22} {:>5} {:>5} {:>7} {:>7} {:>9.2} {:>6.2} {:>7} {:>6}",
            s.run,
            s.candidates,
            s.dropped,
            s.proposed.len(),
            s.judged.len(),
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

    // Precision counts every proposed pair the manifest does not name as a
    // false positive. That is only a statement about the TOOL when the
    // manifest labels every cross-section pair; where it does not, an
    // unlabelled pair and a wrong one are indistinguishable, and printing a
    // number anyway would be a measurement of the manifest's size (§18.3).
    let outside: usize = scores.iter().map(|s| s.outside).sum();
    match &region {
        Region::Document => {
            println!("\nprecision  {p:.2}   (noise floor across runs: {pl:.2}–{ph:.2})");
        }
        Region::Within { name, members } => {
            println!("\nprecision  {p:.2}   (noise floor across runs: {pl:.2}–{ph:.2})");
            println!(
                "           over {name} — {} sections, every pair of them labelled",
                members.len()
            );
            // Said out loud every time. A ratio taken over part of a document
            // reads exactly like one taken over all of it, and the reader
            // cannot tell which they are holding unless it says so.
            println!(
                "           {outside} proposed pair(s) reached outside that region and are NOT scored"
            );
        }
        Region::Undeclared => println!(
            "\nprecision  not scoreable — this manifest names no region it labels completely, \
             so an unlabelled proposal cannot be told from a false one \
             (raw {p:.2} over {} proposed)",
            scores[0].proposed.len()
        ),
    }
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
    println!("excluded: {intra} intra-section pair(s), {unmapped} unmappable, {outside} outside the labelled region");

    // Said before the bars, because it qualifies every number above it. A run
    // that could not weigh some of its pairs did not measure what the reader
    // thinks it measured (§18.3).
    let unread: usize = scores.iter().map(|s| s.passes_unread).sum();
    let attempted: usize = scores.iter().map(|s| s.passes).sum();
    if unread > 0 {
        println!(
            "WARNING: {unread} of {attempted} comparison pass(es) across these runs went unread — \n\
             \x20        every number above is over {:.0}% of the pairs, not all of them",
            100.0 * (attempted - unread) as f64 / attempted.max(1) as f64
        );
    }
    println!();

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

/// The extraction stage's own bar: is each planted tension FINDABLE AT ALL?
///
/// A tension whose load-bearing clause never made it out of the passage
/// cannot be found by any amount of comparison, and a tensions-step recall
/// number computed over such a candidate set is measuring the wrong stage.
/// This separates the two so a bad number can be attributed.
///
/// Observed at the time this was written: the 2026-02-10 decision exists
/// solely to move quiet hours to 10:00 PM, and extraction kept only the
/// sentence saying what had NOT changed — putting T5 and T10 out of reach
/// before the comparison started.
#[test]
#[ignore = "needs draft runs: ./scripts/draft-bar.sh 3"]
fn extraction_coverage() {
    let anchors = anchors();
    let dir = runs_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("no runs at {}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no runs at {}", dir.display());

    let mut worst = usize::MAX;
    for path in &paths {
        let run: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let chunks = run["chunks"].as_array().unwrap();
        let candidates = run["candidates"].as_array().unwrap();
        let kept = run["kept"].as_array().unwrap();

        // Everything a section contributed, in two haystacks: what
        // extraction produced, and what survived the reduce step. Scoring
        // only the survivors blames extraction for a dedupe fold — which is
        // exactly what happened here, and it sent me editing the extraction
        // prompt for a defect one stage later.
        let section_of = |c: &Value| -> Option<String> {
            chunks[c["chunk"].as_u64()? as usize]["heading"]
                .as_str()
                .and_then(section_key)
        };
        let kept_idx: std::collections::BTreeSet<usize> = kept
            .iter()
            .filter_map(|k| k.as_u64().map(|v| v as usize))
            .collect();

        let mut extracted: std::collections::BTreeMap<String, String> = Default::default();
        let mut survived: std::collections::BTreeMap<String, String> = Default::default();
        for (i, c) in candidates.iter().enumerate() {
            let Some(sec) = section_of(c) else { continue };
            let text = format!(" || {}", c["text"].as_str().unwrap_or("").to_lowercase());
            extracted.entry(sec.clone()).or_default().push_str(&text);
            if kept_idx.contains(&i) {
                survived.entry(sec).or_default().push_str(&text);
            }
        }
        // Any one alternative satisfies an anchor.
        let has = |hay: &str, alts: &Value| -> bool {
            alts.as_array()
                .map(|a| {
                    a.iter()
                        .any(|m| hay.contains(&m.as_str().unwrap_or("").to_lowercase()))
                })
                .unwrap_or(false)
        };

        let mut findable = Vec::new();
        let mut blocked = Vec::new();
        let mut folded_away = Vec::new();
        for (tid, sides) in anchors["anchors"].as_object().unwrap() {
            let mut missing = Vec::new();
            let mut lost_to_dedupe = Vec::new();
            for side in sides.as_array().unwrap() {
                let section = side["section"].as_str().unwrap_or("").to_string();
                let ext = extracted.get(&section).cloned().unwrap_or_default();
                let sur = survived.get(&section).cloned().unwrap_or_default();
                for alts in side["must"].as_array().unwrap() {
                    if !has(&ext, alts) {
                        missing.push(format!("{section} never yielded {alts}"));
                    } else if !has(&sur, alts) {
                        // Extraction did its job and the reduce step undid it.
                        lost_to_dedupe.push(format!("{section}: {alts} folded away"));
                    }
                }
            }
            if !missing.is_empty() {
                blocked.push(format!("{tid}: {}", missing.join("; ")));
            } else if !lost_to_dedupe.is_empty() {
                folded_away.push(format!("{tid}: {}", lost_to_dedupe.join("; ")));
            } else {
                findable.push(tid.clone());
            }
        }
        let total = anchors["anchors"].as_object().unwrap().len();
        println!(
            "\n{}  reachable {}/{total}   (extraction missed {}, dedupe folded {})",
            path.file_name().unwrap().to_string_lossy(),
            findable.len(),
            blocked.len(),
            folded_away.len()
        );
        for b in &blocked {
            println!("  NEVER EXTRACTED  {b}");
        }
        for f in &folded_away {
            println!("  FOLDED BY DEDUPE {f}");
        }
        worst = worst.min(findable.len());

        // Every surviving candidate carries a verbatim quote from its own
        // chunk. This is the invariant `draft` actually promises, checked
        // end to end against the persisted evidence.
        //
        // Since the citation is cut from the chunk at a position the model
        // pointed to, this can no longer fail on a well-formed run — which is
        // the point of checking it here rather than trusting it. What it
        // still catches is the chunk id and the citation disagreeing, which
        // would misattribute a rule to a section it never came from.
        //
        // What stood here before was a keyword scan — a rule mentioning "day"
        // whose passage did not was a failure — and it was a SECOND opinion
        // about fidelity that disagreed with the shipped one (§10.6). It
        // fired on "at all times" rendered as "throughout the day and night":
        // a paraphrase with no number attached, which the measure guard is
        // documented to allow and which the unit tests pin. A bar must not
        // assert a promise the tool never made.
        for k in kept {
            let c = &candidates[k.as_u64().unwrap() as usize];
            let chunk = &chunks[c["chunk"].as_u64().unwrap() as usize];
            let flat = |s: &str| {
                s.split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .to_lowercase()
            };
            let quote = flat(c["quote"].as_str().unwrap_or(""));
            let src = flat(chunk["text"].as_str().unwrap_or(""));
            assert!(
                !quote.is_empty() && src.contains(&quote),
                "a surviving candidate's quote is not in its own passage:\n  {}\n  {}",
                c["quote"],
                chunk["source"]
            );
        }
    }

    let total = anchors["anchors"].as_object().unwrap().len();
    println!("\nworst run: {worst}/{total} tensions were reachable at all");
    // Extraction must not put the tensions bar out of reach before the
    // comparison starts. Derived from the pre-registered recall floor rather
    // than chosen: a ceiling below it makes that floor unreachable by
    // construction, and a run that fails for THAT reason is not a statement
    // about tension detection.
    let need = (KILL_RECALL_FLOOR * total as f64).ceil() as usize;
    assert!(
        worst >= need,
        "extraction left only {worst}/{total} tensions reachable — the recall floor of \
         {KILL_RECALL_FLOOR:.2} needs at least {need}, so a tensions number here would be \
         measuring extraction"
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

// ── keys ────────────────────────────────────────────────────

#[test]
fn a_charter_article_and_a_dated_decision_still_key_as_they_did() {
    assert_eq!(
        section_key("# Maple House Charter, Article II — Quiet Hours").as_deref(),
        Some("article:II")
    );
    assert_eq!(
        section_key("# Decision — 2026-02-10 — Weeknight Quiet Hours").as_deref(),
        Some("date:2026-02-10")
    );
}

#[test]
fn an_ordinance_heading_keys_by_ordinance_not_by_its_date() {
    // Load-bearing ordering. Every section of Ordinance 16,064 carries the
    // same adoption date, so keying on the date would collapse sixteen
    // distinct readings onto one key and score them as one pair.
    assert_eq!(
        section_key(r#"# Ordinance 16,064, adopted 2021-10-18 — Sec. 42-258(6), Type "F" permit"#)
            .as_deref(),
        Some("ord:16064/42-258(6)")
    );
    assert_eq!(
        section_key(r#"# Ordinance 16,127, adopted 2022-05-23 — Sec. 42-258(17), Type "Q" permit"#)
            .as_deref(),
        Some("ord:16127/42-258(17)")
    );
}

#[test]
fn a_codified_section_and_its_amended_reading_are_different_keys() {
    // The whole corpus turns on this: the same section number under two
    // documents is two readings, and a scorer that cannot tell them apart
    // cannot score an unmarked supersession at all.
    let codified = section_key(r#"# Des Moines Municipal Code, Sec. 42-258(6) — Type "F" permit"#);
    let amended =
        section_key(r#"# Ordinance 16,064, adopted 2021-10-18 — Sec. 42-258(6), Type "F" permit"#);
    assert_eq!(codified.as_deref(), Some("sec:42-258(6)"));
    assert_ne!(codified, amended);
}

#[test]
fn the_manifest_and_the_document_agree_on_key_shape() {
    use serde_json::json;
    assert_eq!(
        truth_key(&json!({"section": "42-258(6)"})).as_deref(),
        Some("sec:42-258(6)")
    );
    assert_eq!(
        truth_key(&json!({"ordinance": "16064", "section": "42-258(6)"})).as_deref(),
        Some("ord:16064/42-258(6)")
    );
}
