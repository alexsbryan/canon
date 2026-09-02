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

use serde_json::{json, Map, Value};

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

/// How much of its comparison schedule a run must actually get through
/// before its tension counts mean anything.
///
/// Not 100%. One refused pass out of 690 costs a single pair one of its two
/// looks, and discarding a six-hour run over that would be its own kind of
/// waste. Well clear of the 15% that prompted this bar, and a run landing
/// near it should be re-run rather than argued about.
const MIN_COVERAGE: f64 = 0.95;

/// A run whose comparison stage never got through its passes.
///
/// **The fourth way a run fails to be a measurement, and the only one that
/// leaves no marker on the artifact.** Every stage ran, nothing errored, the
/// exit code was zero. On 2026-08-26 a founding run met a daemon shedding
/// load — `host busy`, instantly, 588 times — and finished with 102 of 690
/// passes weighed. It carried no `failed`, no `stopped_after` and no
/// `checkpoint`, so it read exactly like a completed run, and the only
/// contrary signal was a warning printed after the number.
///
/// Deciding it here rather than in `draft` follows the same rule as the
/// other three: the run keeps its partial work on purpose, so refusing to
/// SCORE it is the scorer's job (§18.3).
fn thin_comparison(v: &Value) -> Option<String> {
    let passes = v["tension_passes"].as_u64()?;
    // A corpus small enough to fit one pass has nothing to be thin about.
    if passes == 0 {
        return None;
    }
    let unread = v["tension_passes_unread"]
        .as_array()
        .map_or(0, |a| a.len() as u64);
    let read = passes.saturating_sub(unread);
    let coverage = read as f64 / passes as f64;
    (coverage < MIN_COVERAGE).then(|| {
        format!(
            "comparison weighed {read} of {passes} passes ({:.0}%), under the {:.0}% a \
             measurement needs",
            100.0 * coverage,
            100.0 * MIN_COVERAGE
        )
    })
}

// ── scoring ─────────────────────────────────────────────────

/// A section of the source document: `article:II`, `date:2026-02-10`,
/// `sec:42-258(6)`, or `ord:16064/42-258(6)`.
///
/// `truth.json` keys every labeled pair by one of these, and they are unique
/// within the document. The last two forms exist because a corpus built from
/// municipal ordinances is keyed by the code section a document amends and by
/// the ordinance that amends it — a charter article numeral cannot say which
/// of two readings of the same section it means.
/// `U.S. Constitution, Article I, Section 8` -> `constitution:I.8`.
///
/// The fifth key form, and the only one whose vocabulary is not in this file:
/// a corpus that interleaves several enacting INSTRUMENTS declares them in its
/// own manifest, because "Article II" names a different rule in the Articles
/// of Confederation than it does in the Constitution and no amount of parsing
/// can tell them apart. Two levels, like `ord:<n>/<sec>`, and for the same
/// reason — the outer level is which instrument is speaking.
fn instrument_key(heading: &str, instruments: &Map<String, Value>) -> Option<String> {
    let (doc, rest) = heading.split_once(", ")?;
    let slug = instruments.get(doc.trim())?.as_str()?;
    let roman = |s: &str| -> String {
        s.chars()
            .take_while(|c| matches!(c, 'I' | 'V' | 'X' | 'L' | 'C'))
            .collect()
    };
    let section = |s: &str| -> Option<String> {
        let n: String = s
            .split(", Section ")
            .nth(1)?
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        (!n.is_empty()).then_some(n)
    };
    let numbered = |prefix: &str, mark: &str| -> Option<String> {
        let after = rest.strip_prefix(prefix)?;
        let num = roman(after);
        if num.is_empty() {
            return None;
        }
        let mut key = format!("{slug}:{mark}{num}");
        if let Some(sec) = section(rest) {
            key.push('.');
            key.push_str(&sec);
        }
        Some(key)
    };
    numbered("Article ", "")
        .or_else(|| numbered("Amendment ", "amend."))
        .or_else(|| {
            // A named part rather than a numbered one: the Declaration's
            // self-evident truths, a preamble. Lowercased and hyphenated so the
            // manifest and the heading cannot drift on spacing alone.
            Some(format!(
                "{slug}:{}",
                rest.trim().to_lowercase().replace(' ', "-")
            ))
        })
}

fn section_key(heading: &str, instruments: &Map<String, Value>) -> Option<String> {
    if let Some(k) = instrument_key(heading, instruments) {
        return Some(k);
    }
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
    // A manifest whose corpus keys directly names the key and nothing else.
    if let Some(k) = side.as_str() {
        return Some(k.to_string());
    }
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
    /// What each arrangement of the list contributed: name, passes, distinct
    /// pairs, and how many of those no earlier arrangement had. The last one
    /// is what decides whether the union keeps its second arrangement.
    arrangements: Vec<(String, usize, usize, usize)>,
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

    // Which enacting instruments this corpus interleaves, if any. Empty for a
    // single-document corpus, which is every corpus that existed before this
    // one, so their keys are unchanged.
    let empty = Map::new();
    let instruments = truth["instruments"].as_object().unwrap_or(&empty);
    // candidate position (within `kept`) -> section key
    let section_of = |kept_pos: usize| -> Option<String> {
        let ci = kept.get(kept_pos)?.as_u64()? as usize;
        let chunk = chunks.get(candidates.get(ci)?["chunk"].as_u64()? as usize)?;
        section_key(chunk["heading"].as_str()?, instruments)
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
        arrangements: run["tension_arrangements"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|x| {
                        let n = |k: &str| x[k].as_u64().unwrap_or(0) as usize;
                        (
                            x["arrangement"].as_str().unwrap_or("?").to_string(),
                            n("passes"),
                            n("proposed"),
                            n("added"),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
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
    //
    // Four ways to fall short of a measurement, and all of them are refused
    // here. `failed` means a
    // stage errored. `stopped_after` means a run stopped on purpose — a
    // convergence arm reads passages N times and stops at extraction, so it
    // has no tensions at all; scored as a finished run it would read as
    // recall 0.00 and kill the bar for a measurement that never claimed to
    // make one. `checkpoint` means the process was killed mid-run and this is
    // the .partial.json it left: strictly less than the run would have
    // produced, and the sweep script copies it out with the rest. The fourth
    // is thin comparison coverage, which leaves no marker at all — see
    // [`thin_comparison`].
    let abandoned: Vec<(PathBuf, String)> = paths
        .iter()
        .filter_map(|p| {
            let raw = std::fs::read_to_string(p).ok()?;
            let v: Value = serde_json::from_str(&raw).ok()?;
            let why = v["failed"]
                .as_str()
                .or_else(|| v["stopped_after"].as_str())
                .map(str::to_string)
                .or_else(|| {
                    v["checkpoint"]
                        .as_str()
                        .map(|s| format!("killed mid-run, last stage finished: {s}"))
                })
                .or_else(|| thin_comparison(&v))?;
            Some((p.clone(), why))
        })
        .collect();
    // A hybrid run is SCORED, and said so.
    //
    // Its later stages are real calls and its earlier ones came off a
    // recording — which holds extraction constant across arms, making it a
    // better controlled experiment than two live sweeps, not a worse one. But
    // a reader who cannot see that a number came from a partly replayed run
    // cannot compare it with one that did not.
    for p in &paths {
        if let Some(from) = std::fs::read_to_string(p)
            .ok()
            .and_then(|r| serde_json::from_str::<Value>(&r).ok())
            .and_then(|v| v["replayed_from"].as_str().map(str::to_string))
        {
            println!(
                "  replayed  {}  <- {from}",
                p.file_name().unwrap().to_string_lossy()
            );
        }
    }

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

    // What each arrangement bought. The comparison runs once per arrangement
    // and folds the results, so every recall number above is a UNION number —
    // and an arrangement whose `added` column is empty across these runs is
    // paying `k(k+1)/2` calls for pairs the run already had.
    let mut by_arrangement: Vec<(String, usize, usize, usize, usize)> = Vec::new();
    for s in &scores {
        for (name, passes, proposed, added) in &s.arrangements {
            match by_arrangement.iter_mut().find(|e| e.0 == *name) {
                Some(e) => {
                    e.1 += 1;
                    e.2 += passes;
                    e.3 += proposed;
                    e.4 += added;
                }
                None => by_arrangement.push((name.clone(), 1, *passes, *proposed, *added)),
            }
        }
    }
    if by_arrangement.len() > 1 {
        println!("arrangements, over {} run(s):", scores.len());
        for (name, runs, passes, proposed, added) in &by_arrangement {
            println!(
                "  {name:<11} {passes:>3} pass(es), {proposed:>3} pair(s), {added:>3} no earlier \
                 arrangement had  ({:.1} added per run)",
                *added as f64 / (*runs).max(1) as f64
            );
        }
        println!();
    }

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
///
/// **What "reachable" means here, and what it does not (corrected
/// 2026-08-26).** The anchors are phrases lifted VERBATIM from the documents
/// — their own file says so. The extractor is instructed to paraphrase, and
/// writes "one self-contained sentence stating the rule as the holder would
/// write it". Matching a verbatim source phrase against a paraphrase agrees
/// only by luck, and this bar was doing exactly that: it read `c["text"]`
/// alone and reported 9 of 17 founding tensions unreachable. SIX of those
/// eight verdicts were false and were checked by hand — `amend.XIII.1`'s
/// anchor "Neither slavery nor involuntary servitude" was extracted as
/// "Slavery and involuntary servitude shall not exist within the United
/// States"; `amend.XXI.1`'s "is hereby repealed" as "is repealed";
/// `amend.XVII`'s "elected by the people thereof" as "elected by the people
/// of their respective States".
///
/// So the verdict now runs over the candidate's TEXT **or** its CITATION,
/// which is verbatim by construction (`locate::cite` cuts it out of the
/// passage). That answers the question this bar is for — did extraction keep
/// the passage within reach — and it is a CEILING, not a prediction: a
/// section that yields many candidates clears it easily.
///
/// The text-only count is still computed and printed, because comparison
/// sees `c.text` and nothing else (`tensions::compare`). The gap between the
/// two is the paraphrase gap, and it is a real fact about the corpus rather
/// than an instrument fault. Neither number alone is the recall ceiling, and
/// both have been read as one.
#[test]
#[ignore = "needs draft runs: ./scripts/draft-bar.sh 3"]
fn extraction_coverage() {
    let anchors = anchors();
    let manifest = truth();
    let empty = Map::new();
    let instruments = manifest["instruments"].as_object().unwrap_or(&empty);
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
            section_key(
                chunks[c["chunk"].as_u64()? as usize]["heading"].as_str()?,
                instruments,
            )
        };
        let kept_idx: std::collections::BTreeSet<usize> = kept
            .iter()
            .filter_map(|k| k.as_u64().map(|v| v as usize))
            .collect();

        let mut extracted: std::collections::BTreeMap<String, String> = Default::default();
        let mut survived: std::collections::BTreeMap<String, String> = Default::default();
        // A THIRD haystack: what a guard refused after extraction produced it.
        //
        // `support` removes what it drops from `artifact.candidates` before the
        // artifact is written, so a rule the model DID find and a guard then
        // refused was indistinguishable here from a rule the model never
        // found. Both read as NEVER EXTRACTED, and the two want opposite
        // fixes — one is a prompt, the other is a guard. Measured 2026-08-24:
        // T1's anchor was reported as never extracted when the model had in
        // fact returned "Overnight guests are not permitted ... for any number
        // of nights" and the quantity guard refused it for stating `any
        // number`, a universal quantifier it read as a numeral. The evidence
        // was in the artifact the whole time; this bar simply was not reading
        // it (§18.3 — absence reported, never defaulted).
        let mut refused: std::collections::BTreeMap<String, Vec<(String, String)>> =
            Default::default();
        for d in run["dropped"]
            .as_array()
            .map(|v| v.as_slice())
            .unwrap_or(&[])
        {
            let Some(sec) = section_of(d) else { continue };
            refused.entry(sec).or_default().push((
                d["text"].as_str().unwrap_or("").to_lowercase(),
                d["reason"].as_str().unwrap_or("unstated").to_string(),
            ));
        }
        let mut stated: std::collections::BTreeMap<String, String> = Default::default();
        for (i, c) in candidates.iter().enumerate() {
            let Some(sec) = section_of(c) else { continue };
            let words = format!(" || {}", c["text"].as_str().unwrap_or("").to_lowercase());
            // The citation is verbatim by construction, so it is where a
            // source-phrase anchor can actually land.
            let evidence = format!(
                "{words} || {}",
                c["quote"].as_str().unwrap_or("").to_lowercase()
            );
            extracted
                .entry(sec.clone())
                .or_default()
                .push_str(&evidence);
            stated.entry(sec.clone()).or_default().push_str(&words);
            if kept_idx.contains(&i) {
                survived.entry(sec).or_default().push_str(&evidence);
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
        let mut guarded_away = Vec::new();
        for (tid, sides) in anchors["anchors"].as_object().unwrap() {
            let mut missing = Vec::new();
            let mut lost_to_dedupe = Vec::new();
            let mut lost_to_guard = Vec::new();
            for side in sides.as_array().unwrap() {
                let section = side["section"].as_str().unwrap_or("").to_string();
                let ext = extracted.get(&section).cloned().unwrap_or_default();
                let sur = survived.get(&section).cloned().unwrap_or_default();
                let refs = refused.get(&section).cloned().unwrap_or_default();
                for alts in side["must"].as_array().unwrap() {
                    if !has(&ext, alts) {
                        // Before calling it never found, ask whether a guard
                        // refused it — the two want opposite fixes.
                        match refs.iter().find(|(text, _)| has(text, alts)) {
                            Some((_, why)) => {
                                lost_to_guard.push(format!("{section}: {alts} refused — {why}"))
                            }
                            None => missing.push(format!("{section} never yielded {alts}")),
                        }
                    } else if !has(&sur, alts) {
                        // Extraction did its job and the reduce step undid it.
                        lost_to_dedupe.push(format!("{section}: {alts} folded away"));
                    }
                }
            }
            if !missing.is_empty() {
                blocked.push(format!("{tid}: {}", missing.join("; ")));
            } else if !lost_to_guard.is_empty() {
                guarded_away.push(format!("{tid}: {}", lost_to_guard.join("; ")));
            } else if !lost_to_dedupe.is_empty() {
                folded_away.push(format!("{tid}: {}", lost_to_dedupe.join("; ")));
            } else {
                findable.push(tid.clone());
            }
        }
        // The same anchors against the candidates' OWN sentences, which is
        // all the comparison stage ever sees. Not a verdict — the distance
        // between this and `findable` is the paraphrase gap.
        let in_own_words = anchors["anchors"]
            .as_object()
            .unwrap()
            .values()
            .filter(|sides| {
                sides.as_array().unwrap().iter().all(|side| {
                    let sec = side["section"].as_str().unwrap_or("").to_string();
                    let hay = stated.get(&sec).cloned().unwrap_or_default();
                    side["must"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|a| has(&hay, a))
                })
            })
            .count();

        let total = anchors["anchors"].as_object().unwrap().len();
        println!(
            "\n{}  reachable {}/{total}   (extraction missed {}, a guard refused {}, dedupe folded {})",
            path.file_name().unwrap().to_string_lossy(),
            findable.len(),
            blocked.len(),
            guarded_away.len(),
            folded_away.len()
        );
        println!(
            "  of those, {in_own_words}/{total} carry the anchor in a candidate's own sentence; \
             {} reach comparison only as paraphrase",
            findable.len().saturating_sub(in_own_words)
        );
        for b in &blocked {
            println!("  NEVER EXTRACTED  {b}");
        }
        for g in &guarded_away {
            println!("  REFUSED BY GUARD {g}");
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
        section_key("Maple House Charter, Article II — Quiet Hours", &Map::new()).as_deref(),
        Some("article:II")
    );
    assert_eq!(
        section_key(
            "Maple House Charter, Article XI — Quiet Study Hours",
            &Map::new()
        )
        .as_deref(),
        Some("article:XI")
    );
    assert_eq!(
        section_key("Decision — 2026-02-10 — Weeknight Quiet Hours", &Map::new()).as_deref(),
        Some("date:2026-02-10")
    );
    assert_eq!(section_key("Just some prose", &Map::new()), None);
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
        .filter_map(|l| section_key(l.trim_start_matches('#').trim(), &Map::new()))
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
        section_key(
            "# Maple House Charter, Article II — Quiet Hours",
            &Map::new()
        )
        .as_deref(),
        Some("article:II")
    );
    assert_eq!(
        section_key(
            "# Decision — 2026-02-10 — Weeknight Quiet Hours",
            &Map::new()
        )
        .as_deref(),
        Some("date:2026-02-10")
    );
}

#[test]
fn an_ordinance_heading_keys_by_ordinance_not_by_its_date() {
    // Load-bearing ordering. Every section of Ordinance 16,064 carries the
    // same adoption date, so keying on the date would collapse sixteen
    // distinct readings onto one key and score them as one pair.
    assert_eq!(
        section_key(
            r#"# Ordinance 16,064, adopted 2021-10-18 — Sec. 42-258(6), Type "F" permit"#,
            &Map::new(),
        )
        .as_deref(),
        Some("ord:16064/42-258(6)")
    );
    assert_eq!(
        section_key(
            r#"# Ordinance 16,127, adopted 2022-05-23 — Sec. 42-258(17), Type "Q" permit"#,
            &Map::new(),
        )
        .as_deref(),
        Some("ord:16127/42-258(17)")
    );
}

#[test]
fn every_key_the_founding_manifest_names_is_a_heading_in_the_corpus() {
    // The instrument the founding corpus is scored with, validated before any
    // number it produces (§18.4). Its manifest keys passages directly rather
    // than by descriptor, so a key that no heading parses to would report the
    // pair as unmappable and the tension as missed — a scorer defect wearing
    // a model defect's clothes.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/founding");
    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(root.join("truth.json")).expect("truth.json"),
    )
    .expect("truth JSON");
    let instruments = manifest["instruments"].as_object().expect("instruments");
    let corpus = std::fs::read_to_string(root.join("founding.md")).expect("founding.md");
    let headings: BTreeSet<String> = corpus
        .lines()
        .filter(|l| l.starts_with("# "))
        .filter_map(|l| section_key(l.trim_start_matches('#').trim(), instruments))
        .collect();
    assert_eq!(
        headings.len(),
        91,
        "every heading must key, and key uniquely"
    );

    let sides = manifest["planted_tensions"]
        .as_array()
        .unwrap()
        .iter()
        .chain(manifest["expected_non_tensions"].as_array().unwrap())
        .flat_map(|t| [t["a"].clone(), t["b"].clone()])
        .filter_map(|v| truth_key(&v));
    for key in sides {
        assert!(
            headings.contains(&key),
            "the manifest names `{key}`, which no heading in the corpus parses to"
        );
    }
    // And the two levels really do separate the instruments: Article II names
    // a different rule in each document and must not collapse to one key.
    assert_ne!(
        section_key("Articles of Confederation, Article II", instruments),
        section_key("U.S. Constitution, Article II, Section 1", instruments)
    );
    assert_eq!(
        section_key("U.S. Constitution, Amendment XIV, Section 2", instruments).as_deref(),
        Some("constitution:amend.XIV.2")
    );
}

#[test]
fn a_codified_section_and_its_amended_reading_are_different_keys() {
    // The whole corpus turns on this: the same section number under two
    // documents is two readings, and a scorer that cannot tell them apart
    // cannot score an unmarked supersession at all.
    let codified = section_key(
        r#"# Des Moines Municipal Code, Sec. 42-258(6) — Type "F" permit"#,
        &Map::new(),
    );
    let amended = section_key(
        r#"# Ordinance 16,064, adopted 2021-10-18 — Sec. 42-258(6), Type "F" permit"#,
        &Map::new(),
    );
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

#[test]
fn a_run_that_barely_compared_anything_is_not_a_measurement() {
    // Observed 2026-08-26. A founding run met a daemon shedding load and
    // finished with 102 of 690 passes weighed. Nothing errored, every stage
    // ran, the exit code was zero — so `failed`, `stopped_after` and
    // `checkpoint` were all absent and it read as a complete run. The only
    // contrary signal was a warning printed after the number.
    let thin = json!({
        "tension_passes": 690,
        "tension_passes_unread": vec![""; 588],
    });
    let why = thin_comparison(&thin).expect("refused");
    assert!(why.contains("102 of 690"), "{why}");
    assert!(why.contains("15%"), "names the coverage it got: {why}");

    // One lost pass out of 690 costs a single pair one of its two looks.
    // Discarding a six-hour run for that would be its own waste.
    let whole = json!({
        "tension_passes": 690,
        "tension_passes_unread": vec![""; 1],
    });
    assert_eq!(thin_comparison(&whole), None);

    // The bar bites somewhere between, and not before.
    let at_bar = json!({
        "tension_passes": 100,
        "tension_passes_unread": vec![""; 5],
    });
    assert_eq!(
        thin_comparison(&at_bar),
        None,
        "95% is the bar, not under it"
    );
    let under = json!({
        "tension_passes": 100,
        "tension_passes_unread": vec![""; 6],
    });
    assert!(thin_comparison(&under).is_some(), "94% is under it");

    // A corpus small enough to fit one pass has nothing to be thin about,
    // and a run with no comparison stage is judged by the other three
    // markers, not invented into a failure here.
    assert_eq!(thin_comparison(&json!({"tension_passes": 0})), None);
    assert_eq!(thin_comparison(&json!({})), None);
}

// ── two-up: can the comparison stage see a tension at all? ───
//
// The sweep is six hours and has never finished. This asks the cheapest
// version of its question — hand the stage the two commitments that carry a
// planted tension, ALONE, and see whether it says so — for seventeen model
// calls and about two minutes.
//
// **It is an UPPER BOUND, not a proof.** Two-up is strictly easier than the
// real window, where the same pair arrives among twenty-two distractors.
// Passing here does not mean the sweep succeeds; failing here means it
// cannot, and no sweep should be paid for.
//
// It runs the SHIPPED path and adds no production code: a canon holding
// exactly two commitments is at or below `BATCH`, so `canon tensions` makes
// exactly one comparison pass over exactly that pair.

/// How many of the eleven supersessions must be visible two-up before a
/// sweep is worth its six hours.
///
/// Derived, not chosen. Publication needs mean recall >= 0.50 over the
/// eleven, which is 5.5 pairs, which is six. If the stage cannot see six
/// when handed both sides alone, it certainly cannot see six inside a
/// 24-wide window, and the publish gate is unreachable by construction.
const TWO_UP_FLOOR: usize = 6;

/// The endpoint the two-up loop calls, matching `scripts/draft-bar.sh`.
fn two_up_endpoint() -> (String, String) {
    (
        std::env::var("CANON_ENDPOINT").unwrap_or_else(|_| "http://localhost:9741/v1".into()),
        std::env::var("CANON_MODEL").unwrap_or_else(|_| "primary".into()),
    )
}

/// Which haystack named the candidate — and therefore how much the pair is
/// worth.
///
/// `OwnWords` is the strong grade: the comparison stage reads `c.text` and
/// nothing else, so a candidate selected on its own sentence was selected on
/// the evidence the stage will actually be handed. `Citation` is weaker — the
/// anchor was found in a verbatim quote the stage never sees, and the pair is
/// only as good as the extractor's paraphrase of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Grade {
    OwnWords,
    Citation,
}

impl std::fmt::Display for Grade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Grade::OwnWords => "own words",
            Grade::Citation => "citation",
        })
    }
}

/// A planted tension resolved to the two commitments the stage will be handed,
/// and the evidence each side was chosen on.
struct Pairing {
    id: String,
    kind: String,
    a: (usize, Grade),
    b: (usize, Grade),
}

/// One side of a planted tension, resolved to the candidate that carries it.
///
/// **The anchors were authored for a SECTION-level question and this is a
/// CANDIDATE-level one.** `extraction_coverage` asks "did anything this
/// section produced carry the phrase", and text-OR-citation is the right
/// haystack for it. Two-up asks which single commitment to hand the stage,
/// and the stage reads `c.text` alone — so selecting on a citation selects on
/// evidence the thing being measured cannot see. Written the other way first,
/// and it cost a whole 17-call run: `constitution:III.2` resolved to "the
/// judicial Power shall extend to all Cases in Law and Equity" rather than to
/// "...cases between a State and Citizens of another State", because all
/// fourteen candidates from that section share one wide citation containing
/// the anchor. The stage was handed a pair that does not conflict and was
/// right to say so.
///
/// So: prefer the candidate whose OWN sentence carries every `must` group
/// (alternatives within a group are any-of), fall back to the citation only
/// when no candidate in the section states it, and REFUSE when the fallback
/// cannot name one. 152 of this corpus's 334 candidates share a citation with
/// a sibling, so "lowest index wins" among them is a coin toss wearing a
/// number (§18.3 — absence is reported, never defaulted).
fn side_to_candidate(
    run: &Value,
    instruments: &Map<String, Value>,
    side: &Value,
) -> Result<(usize, Grade), String> {
    let want = side["section"].as_str().unwrap_or("").to_string();
    let chunks = run["chunks"].as_array().ok_or("run has no chunks")?;
    let candidates = run["candidates"]
        .as_array()
        .ok_or("run has no candidates")?;
    let groups = side["must"].as_array().ok_or("a side must carry `must`")?;

    let carries = |hay: &str| -> bool {
        groups.iter().all(|alts| {
            alts.as_array()
                .map(|a| {
                    a.iter()
                        .any(|m| hay.contains(&m.as_str().unwrap_or("").to_lowercase()))
                })
                .unwrap_or(false)
        })
    };

    let mut in_section = 0usize;
    let mut own: Vec<usize> = Vec::new();
    let mut cited: Vec<usize> = Vec::new();
    for (i, c) in candidates.iter().enumerate() {
        let heading = c["chunk"]
            .as_u64()
            .and_then(|n| chunks.get(n as usize))
            .and_then(|ch| ch["heading"].as_str());
        let Some(sec) = heading.and_then(|h| section_key(h, instruments)) else {
            continue;
        };
        if sec != want {
            continue;
        }
        in_section += 1;
        if carries(&c["text"].as_str().unwrap_or("").to_lowercase()) {
            own.push(i);
        } else if carries(&c["quote"].as_str().unwrap_or("").to_lowercase()) {
            cited.push(i);
        }
    }
    // Several candidates stating the phrase themselves are each a legitimate
    // carrier of it; the lowest keeps two runs of this resolver in agreement.
    if let Some(i) = own.first() {
        return Ok((*i, Grade::OwnWords));
    }
    match cited.len() {
        1 => Ok((cited[0], Grade::Citation)),
        0 if in_section == 0 => Err(format!("{want}: no candidate came from that section")),
        0 => Err(format!(
            "{want}: {in_section} candidate(s), none carrying {groups:?}"
        )),
        n => Err(format!(
            "{want}: no candidate STATES {groups:?}, and {n} share a citation carrying it \
             ({}) — the anchor cannot name one",
            cited
                .iter()
                .take(6)
                .map(|i| format!("c{i}"))
                .collect::<Vec<_>>()
                .join(", ")
        )),
    }
}

/// Run one comparison over exactly two commitments, on the shipped path.
///
/// `Ok(true)` the stage named the pair, `Ok(false)` it saw nothing, `Err` it
/// could not judge. The third is never folded into the second: a refused call
/// counted as "not seen" would quietly deflate the number this exists to
/// produce (§18.3).
/// One pair, shown alone. Returns whether the stage called it a tension, and
/// what the endpoint said actually answered — an alias is not provenance.
fn two_up_once(
    bin: &Path,
    scratch: &Path,
    a: &str,
    b: &str,
    distractors: &[String],
) -> Result<(bool, Option<String>), String> {
    let (endpoint, model) = two_up_endpoint();
    let dir = scratch.join(".canon");
    let _ = std::fs::remove_dir_all(scratch);
    std::fs::create_dir_all(&dir).map_err(|e| format!("scratch: {e}"))?;

    let run = |args: &[&str]| -> Result<std::process::Output, String> {
        std::process::Command::new(bin)
            .args(args)
            .env("CANON_DIR", &dir)
            .env("CANON_ENDPOINT", &endpoint)
            .env("CANON_MODEL", &model)
            .output()
            .map_err(|e| format!("{bin:?} {args:?}: {e}"))
    };
    let must = |args: &[&str]| -> Result<(), String> {
        let out = run(args)?;
        if out.status.success() {
            return Ok(());
        }
        Err(format!(
            "canon {} exited {:?}: {}",
            args[0],
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ))
    };
    must(&["init", "--profile", "house"])?;
    // `add` prints `<id>  <text>`. With a window we have to know WHICH pair
    // came back, not merely that something did.
    let added = |text: &str| -> Result<String, String> {
        let out = run(&["add", text])?;
        if !out.status.success() {
            return Err(format!(
                "add: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ));
        }
        String::from_utf8_lossy(&out.stdout)
            .split_whitespace()
            .next()
            .map(str::to_string)
            .ok_or_else(|| format!("add printed no id for {text:?}"))
    };
    let (id_a, id_b) = (added(a)?, added(b)?);
    for d in distractors {
        added(d)?;
    }

    let out = run(&["tensions", "--json"])?;
    if !out.status.success() {
        return Err(format!(
            "tensions exited {:?}: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let served = String::from_utf8_lossy(&out.stderr).lines().find_map(|l| {
        l.rsplit_once(" answered by ")
            .map(|(_, m)| m.trim().to_string())
    });
    let body = String::from_utf8_lossy(&out.stdout);
    let found: Value = serde_json::from_str(body.trim())
        .map_err(|e| format!("tensions --json: {e} in {body:?}"))?;
    let pairs = found.as_array().ok_or("tensions --json is not an array")?;
    if distractors.is_empty() {
        // Two commitments admit exactly one unordered pair, so any entry at
        // all is that pair. Nothing to match on, and matching on ids would
        // assert against `add`'s hashing rather than against the stage.
        return Ok((!pairs.is_empty(), served));
    }
    // In a window the stage may report several pairs and most of them are
    // not the one being asked about. A conflict is symmetric, so match the
    // unordered pair.
    let hit = pairs.iter().any(|c| {
        let (x, y) = (c["a"].as_str().unwrap_or(""), c["b"].as_str().unwrap_or(""));
        (x == id_a && y == id_b) || (x == id_b && y == id_a)
    });
    Ok((hit, served))
}

#[test]
#[ignore = "makes 17 live model calls: CANON_BAR_RUNS=fixtures/founding/runs/qwen-27b CANON_BAR_TRUTH=fixtures/founding/truth.json CANON_BAR_ANCHORS=fixtures/founding/extraction-anchors.json cargo test --test draft_bar -- --ignored two_up --nocapture"]
fn two_up_upper_bound() {
    let anchors = anchors();
    let manifest = truth();
    let empty = Map::new();
    let instruments = manifest["instruments"].as_object().unwrap_or(&empty);
    // Decoys carry the kind `decoy` and are counted apart from everything
    // else. A recall figure with no false-positive figure beside it is the
    // half of the measurement that flatters (§18.6).
    let kinds: std::collections::BTreeMap<String, String> = manifest["planted_tensions"]
        .as_array()
        .expect("planted_tensions")
        .iter()
        .filter_map(|p| {
            Some((
                p["id"].as_str()?.to_string(),
                p["type"].as_str().unwrap_or("unlabelled").to_string(),
            ))
        })
        .chain(
            manifest["expected_non_tensions"]
                .as_array()
                .map(|v| v.as_slice())
                .unwrap_or(&[])
                .iter()
                .filter_map(|p| Some((p["id"].as_str()?.to_string(), "decoy".to_string()))),
        )
        .collect();

    // ONE artifact, named. Resolution is the only thing that varies between
    // runs and any one of them yields a valid upper bound; a mean over two
    // instruments would be about neither (§18.4).
    let dir = runs_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| {
            panic!(
                "no runs at {}: {e}{}",
                dir.display(),
                runs_one_level_down(&dir)
            )
        })
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    paths.sort();
    let path = paths
        .last()
        .unwrap_or_else(|| panic!("no runs at {}{}", dir.display(), runs_one_level_down(&dir)));
    let run: Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();

    let bin = Path::new(env!("CARGO_BIN_EXE_canon"));
    let (endpoint, model) = two_up_endpoint();
    println!("\ntwo-up  upper bound on the comparison stage");
    println!("  artifact {}", path.display());
    println!("  endpoint {endpoint} (model {model})");
    println!("  binary   {}", bin.display());

    // Resolve first, print the bill, then spend it. A resolver failure is
    // free to discover and costs a model call to discover late.
    let mut resolved: Vec<Pairing> = Vec::new();
    let mut unresolved: Vec<(String, String, String)> = Vec::new();
    let empty_map = Map::new();
    let sides_of = anchors["anchors"]
        .as_object()
        .expect("anchors")
        .iter()
        .chain(anchors["decoys"].as_object().unwrap_or(&empty_map));
    for (tid, sides) in sides_of {
        let kind = kinds
            .get(tid)
            .cloned()
            .unwrap_or_else(|| "unlabelled".into());
        let sides = sides.as_array().expect("sides");
        assert_eq!(sides.len(), 2, "{tid}: two-up needs exactly two sides");
        match (
            side_to_candidate(&run, instruments, &sides[0]),
            side_to_candidate(&run, instruments, &sides[1]),
        ) {
            (Ok(a), Ok(b)) => resolved.push(Pairing {
                id: tid.clone(),
                kind,
                a,
                b,
            }),
            (a, b) => unresolved.push((
                tid.clone(),
                kind,
                [a.err(), b.err()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join("; "),
            )),
        }
    }
    // By id, not by the tuple: `Grade` has no ordering and does not need one.
    resolved.sort_by(|x, y| x.id.cmp(&y.id));
    unresolved.sort_by(|x, y| x.0.cmp(&y.0));
    let both_own = resolved
        .iter()
        .filter(|p| p.a.1 == Grade::OwnWords && p.b.1 == Grade::OwnWords)
        .count();
    println!(
        "  {} of {} pairs resolve to a candidate pair — {} model call(s)",
        resolved.len(),
        anchors["anchors"].as_object().unwrap().len()
            + anchors["decoys"].as_object().map(Map::len).unwrap_or(0),
        resolved.len()
    );
    println!(
        "  {both_own} of those name both sides from the candidates' OWN sentences, \
         which is all the stage reads\n"
    );

    // The free half, runnable on its own. Resolution is where a founding
    // corpus most often stops the instrument, and discovering that after
    // seventeen model calls is paying to learn something that was already
    // on disk.
    let dry = std::env::var("CANON_BAR_TWO_UP_DRY").is_ok();

    // What actually answered, from the first call that said so. `model` is
    // the alias we asked for, and on a mesh the two are routinely different:
    // `primary` named the 27B when this loop was first run and names a 35B
    // MoE now. An artifact that records only the alias cannot be attributed
    // to a model afterwards, and three in this repository already cannot.
    let mut served: Option<String> = None;

    // Phase 1c. A table of edits to the resolved sides, keyed by pair id.
    // When set, ONLY the pairs it names are asked — the arm is about those
    // nine and asking the rest would spend calls on nothing.
    let perturb: Option<Value> = std::env::var("CANON_BAR_PERTURB").ok().map(|p| {
        serde_json::from_str(
            &std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("CANON_BAR_PERTURB {p}: {e}")),
        )
        .unwrap_or_else(|e| panic!("CANON_BAR_PERTURB {p}: {e}"))
    });

    // How many times each pair is asked. ONE IS AN ANECDOTE HERE TOO, and
    // this instrument was believed silent until it was checked: temperature is
    // 0.0 and two runs agreed on 12 of 12 pairs, which read as determinism.
    // The third run reproduced 16 of 17 and flipped P2 from not-seen to seen
    // on identical candidate indices — so a one-run delta cannot be told from
    // the endpoint's own wobble, and any arm compared at n=1 is measuring
    // noise (§18.5). Majority decides; the flip count is printed as the
    // instrument's own noise reading and belongs beside every number here.
    let runs: usize = std::env::var("CANON_BAR_TWO_UP_RUNS")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|n| *n > 0)
        .unwrap_or(1);

    let candidates = run["candidates"].as_array().unwrap();

    // ── the window, and how its distractors are chosen ──
    //
    // `CANON_BAR_WINDOW=24` asks each pair inside a canon of that many
    // commitments instead of a canon of two. 24 is `BATCH`, so the whole
    // window still fits one comparison call and the ONLY thing that changed
    // between this and the two-up number is the company the pair keeps.
    //
    // Three exclusions, fixed here before the arm was run, because each one
    // can move the number:
    //   1. a candidate sharing a citation with either side — a near-duplicate
    //      of a side makes "did it find THE pair" unanswerable;
    //   2. a candidate that is a resolved side of any OTHER planted pair —
    //      otherwise the window carries a second planted tension and one call
    //      is doing two jobs;
    //   3. the two sides themselves.
    // What is left is sorted by index and sampled at an even stride, so the
    // distractors are spread across the corpus rather than drawn from
    // whichever document happens to sit at the front, and two runs of this
    // harness build the same window.
    let window: usize = std::env::var("CANON_BAR_WINDOW")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let sides: BTreeSet<usize> = resolved.iter().flat_map(|p| [p.a.0, p.b.0]).collect();
    let distractors_for = |a: usize, b: usize| -> Vec<String> {
        if window < 3 {
            return Vec::new();
        }
        let cite = |i: usize| candidates[i]["source"].as_str().unwrap_or("").to_string();
        let (ca, cb) = (cite(a), cite(b));
        let eligible: Vec<usize> = (0..candidates.len())
            .filter(|i| !sides.contains(i))
            .filter(|i| cite(*i) != ca && cite(*i) != cb)
            .collect();
        let want = window - 2;
        if eligible.len() < want {
            return Vec::new();
        }
        let stride = eligible.len() / want;
        (0..want)
            .map(|k| {
                candidates[eligible[k * stride]]["text"]
                    .as_str()
                    .unwrap_or("")
                    .to_string()
            })
            .collect()
    };
    let scratch = std::env::var("CANON_BAR_TWO_UP_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dir.join("two-up"));
    let mut seen: Vec<String> = Vec::new();
    let mut blind: Vec<String> = Vec::new();
    let mut refused: Vec<(String, String)> = Vec::new();
    let mut rows: Vec<Value> = Vec::new();
    let mut flips = 0usize;
    // A pair whose window could not be built is REFUSED, never scored as a
    // miss: too few eligible distractors is a fact about the corpus.
    let mut failed_window: Vec<String> = Vec::new();

    for Pairing {
        id: tid,
        kind,
        a: (a, ga),
        b: (b, gb),
    } in &resolved
    {
        let mut ta = candidates[*a]["text"].as_str().unwrap_or("").to_string();
        let mut tb = candidates[*b]["text"].as_str().unwrap_or("").to_string();
        if let Some(table) = &perturb {
            let Some(entry) = table["perturbations"].get(tid) else {
                continue;
            };
            let side = entry["side"].as_str().unwrap_or("b");
            let target = if side == "a" { &mut ta } else { &mut tb };
            for e in entry["edits"].as_array().expect("edits is a list") {
                let (find, rep) = (e[0].as_str().unwrap(), e[1].as_str().unwrap());
                // The one way this experiment could lie in its own favour: a
                // `find` that is not there leaves the pair intact, the stage
                // rightly still calls it a tension, and a no-op scores as
                // evidence of reading. Refuse instead.
                assert!(
                    target.contains(find),
                    "{tid}: side {side} does not contain {find:?} — the edit would be a no-op \
                     and a no-op scores as reading. Fix the table, not the result."
                );
                *target = target.replace(find, rep);
            }
        }
        let (ta, tb) = (ta.as_str(), tb.as_str());
        let how = format!("c{a} [{ga}] x c{b} [{gb}]");
        // Built before the dry-run guard on purpose: a window that cannot be
        // built is a fact about the corpus and is worth learning for free.
        let company = distractors_for(*a, *b);
        if window >= 3 && company.is_empty() {
            failed_window.push(tid.clone());
        }
        if dry {
            println!("  window      {} commitment(s)", company.len() + 2);
            println!("  would ask   {tid} [{kind}]  {how}");
            println!("      a: {ta}");
            println!("      b: {tb}");
            continue;
        }
        // One retry per ask, because a shed is a property of the host and not
        // of the stage. Twice refused is reported as refused, never as
        // not-seen.
        let mut votes: Vec<bool> = Vec::new();
        let mut failed: Option<String> = None;
        for _ in 0..runs {
            let mut verdict = two_up_once(bin, &scratch.join(tid), ta, tb, &company);
            if verdict.is_err() {
                verdict = two_up_once(bin, &scratch.join(tid), ta, tb, &company);
            }
            match verdict {
                Ok((v, who)) => {
                    votes.push(v);
                    if let Some(m) = who {
                        served.get_or_insert(m);
                    }
                }
                Err(e) => {
                    failed = Some(e);
                    break;
                }
            }
        }
        // The per-pair canon is throwaway — two acts and a profile — and it
        // sits under the runs directory where the evidence sidecars live. Left
        // behind, 21 of them turn `git add` on that directory into a commit of
        // scratch. The sidecar is the artifact; this is not.
        let _ = std::fs::remove_dir_all(scratch.join(tid));

        let yes = votes.iter().filter(|v| **v).count();
        let majority = yes * 2 > votes.len();
        let flipped = yes > 0 && yes < votes.len();
        if flipped {
            flips += 1;
        }
        let tally = if runs > 1 {
            format!(
                "  [{yes}/{} {}]",
                votes.len(),
                if flipped { "FLIPPED" } else { "stable" }
            )
        } else {
            String::new()
        };
        match &failed {
            Some(e) => {
                refused.push((tid.clone(), e.clone()));
                println!("  REFUSED   {tid} [{kind}]  {e}");
            }
            None if majority => {
                seen.push(tid.clone());
                println!("  SEEN      {tid} [{kind}]  {how}{tally}");
            }
            None => {
                blind.push(tid.clone());
                println!("  not seen  {tid} [{kind}]  {how}{tally}");
                println!("      a: {ta}");
                println!("      b: {tb}");
            }
        }
        rows.push(json!({
            "id": tid,
            "type": kind,
            "a": { "candidate": a, "text": ta, "resolved_by": ga.to_string() },
            "b": { "candidate": b, "text": tb, "resolved_by": gb.to_string() },
            "votes_seen": yes,
            "votes_cast": votes.len(),
            "flipped": flipped,
            "verdict": match (&failed, majority) {
                (Some(_), _) => "refused",
                (None, true) => "seen",
                (None, false) => "not_seen",
            },
            "error": failed,
        }));
    }
    for (tid, kind, why) in &unresolved {
        println!("  UNRESOLVED {tid} [{kind}]  {why}");
        rows.push(json!({ "id": tid, "type": kind, "verdict": "unresolved", "error": why }));
    }

    if dry {
        println!(
            "\n  dry run — {} pair(s) would be asked, {} unresolved. Nothing was spent.",
            resolved.len(),
            unresolved.len()
        );
        return;
    }

    let of_kind = |ids: &[String], k: &str| -> usize {
        ids.iter()
            .filter(|t| kinds.get(*t).map(String::as_str) == Some(k))
            .count()
    };
    let planted_of = |k: &str| kinds.values().filter(|v| *v == k).count();
    let sup_seen = of_kind(&seen, "unmarked_supersession");
    let sup_total = planted_of("unmarked_supersession");
    let pri_seen = of_kind(&seen, "principle_vs_rule");
    let pri_total = planted_of("principle_vs_rule");
    // A decoy "seen" is a FALSE POSITIVE: the pair is labelled compatible.
    let decoy_flagged = of_kind(&seen, "decoy");
    let decoy_total = planted_of("decoy");

    // The data lands whatever the verdict is — a refusal that ships no
    // evidence cannot be argued with.
    let out = json!({
        "at": std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        "instrument": "two-up",
        "bound": if window >= 3 {
            "in a window — each pair among distractors, as the sweep will show it"
        } else {
            "upper — each pair shown alone, not in a 24-wide window"
        },
        "window": window,
        "windows_not_built": failed_window,
        "artifact": path.display().to_string(),
        "endpoint": endpoint,
        "model": model,
        "served_model": served,
        "arm": perturb.as_ref().and_then(|p| p["arm"].as_str()),
        "perturbations": std::env::var("CANON_BAR_PERTURB").ok(),
        "floor": TWO_UP_FLOOR,
        "runs_per_pair": runs,
        "pairs_that_flipped": flips,
        "supersessions": { "seen": sup_seen, "of": sup_total },
        "principles": { "seen": pri_seen, "of": pri_total },
        "decoys_flagged": { "flagged": decoy_flagged, "of": decoy_total },
        "unresolved": unresolved.len(),
        "refused": refused.len(),
        "tensions": rows,
    });
    let _ = std::fs::create_dir_all(&scratch);
    let sidecar = scratch.join(format!("two-up-{}.json", out["at"].as_u64().unwrap_or(0)));
    if let Err(e) = std::fs::write(&sidecar, serde_json::to_string_pretty(&out).unwrap()) {
        println!("\nwarning: could not write {}: {e}", sidecar.display());
    } else {
        println!("\n  evidence {}", sidecar.display());
    }

    println!(
        "\n  supersessions {sup_seen}/{sup_total}   principles {pri_seen}/{pri_total}   \
         DECOYS FLAGGED {decoy_flagged}/{decoy_total}   unresolved {}   refused {}",
        unresolved.len(),
        refused.len()
    );
    println!("  UPPER BOUND — each pair was shown alone. The sweep shows it among 22 others.");
    if runs > 1 {
        println!(
            "  n={runs} per pair, majority decides. {flips} pair(s) did not answer the same way \
             every time — that is this instrument's noise floor, and no delta smaller than it \
             is readable."
        );
    } else {
        println!(
            "  n=1. Runs of 2026-08-27 reproduced 16 of 17 and flipped one; set \
             CANON_BAR_TWO_UP_RUNS=3 before comparing this against another arm."
        );
    }

    // Four verdicts, not two (§18.1). A refused call is never folded into
    // "not seen", and a pair the anchors could not resolve never reached the
    // stage at all — so a shortfall counted against the full eleven would
    // blame the stage for the fixture.
    assert!(
        refused.is_empty(),
        "{} pair(s) could not be judged, twice each — this is not a measurement: {refused:?}",
        refused.len()
    );

    // Two-up is strictly easier than the sweep, so this is a floor of sanity
    // rather than the sweep's bar: a stage flagging most of the labelled
    // compatible pairs when shown two commitments alone is not discriminating
    // at all, and any recall number it produces is noise.
    assert!(
        decoy_flagged < KILL_DECOY_CEILING,
        "{decoy_flagged} of {decoy_total} COMPATIBLE pairs were called tensions, at or past the \
         ceiling of {KILL_DECOY_CEILING}. Recall is not meaningful beside this."
    );

    let sup_unresolved = unresolved
        .iter()
        .filter(|(_, k, _)| k == "unmarked_supersession")
        .count();
    let reachable = sup_seen + sup_unresolved;
    if sup_seen >= TWO_UP_FLOOR {
        // Met on a subset is met. Nothing the unresolved pairs could have
        // done would take it back below the floor.
        return;
    }
    assert!(
        reachable < TWO_UP_FLOOR,
        "CANNOT JUDGE. The stage saw {sup_seen} of the {} supersessions it was shown, and \
         {sup_unresolved} more never reached it because the anchors could not name one \
         candidate — so the true figure is somewhere in {sup_seen}..={reachable} against a \
         floor of {TWO_UP_FLOOR}. THE FIXTURE IS THE BLOCKER, NOT THE COMPARISON PROMPT: \
         give those sides an anchor phrase that appears in a candidate's own sentence, \
         then re-run. Do not touch the prompt on this evidence, and do not buy a sweep.",
        sup_total - sup_unresolved
    );
    panic!(
        "two-up saw {sup_seen} of {sup_total} supersessions and could reach at most \
         {reachable}, under the floor of {TWO_UP_FLOOR}. The comparison prompt is the lever — \
         fix it here, where a pass costs one call, not in a six-hour sweep."
    );
}

#[test]
fn a_side_resolves_to_the_one_candidate_that_carries_the_whole_anchor() {
    // The resolver's own gate, run with no endpoint: a resolver that silently
    // returned the wrong candidate would send seventeen correct pairs to the
    // stage as seventeen wrong ones, and the number would look like a model
    // failure (§18.4 — validate the instrument before the result).
    let instruments = Map::new();
    let run = json!({
        "chunks": [
            { "heading": "Maple House Charter, Article II — Quiet Hours" },
            { "heading": "Maple House Charter, Article XI — Quiet Study Hours" }
        ],
        "candidates": [
            // Right section, carries neither group.
            { "chunk": 0, "text": "Bins go out on Tuesday.", "quote": "Bins go out on Tuesday." },
            // Right section, carries ONE group. A section-wide reading would
            // accept this; two-up must not, because the stage is handed one
            // commitment and would never see the other half.
            { "chunk": 0, "text": "Quiet hours begin at ten.", "quote": "Quiet hours begin at ten." },
            // Wrong section, carries both. Proves the section actually binds.
            { "chunk": 1, "text": "Quiet hours begin at ten and end at seven.",
              "quote": "Quiet hours begin at ten and end at seven." },
            // Right section, carries both — in the citation, not the text,
            // which is where a verbatim source phrase usually lands.
            { "chunk": 0, "text": "Nights are for sleeping.",
              "quote": "Quiet hours begin at ten and end at seven." }
        ]
    });
    let side = |sec: &str, must: Value| json!({ "section": sec, "must": must });
    let both = json!([["quiet hours begin at ten"], ["end at seven"]]);

    // No candidate STATES both groups, exactly one cites both: the weak
    // grade, and it says so rather than passing as the strong one.
    assert_eq!(
        side_to_candidate(&run, &instruments, &side("article:II", both.clone())),
        Ok((3, Grade::Citation)),
        "the one candidate in the right section whose citation carries EVERY group"
    );

    // Any-of within a group, and a candidate that states it outranks any
    // number of candidates that merely cite it.
    assert_eq!(
        side_to_candidate(
            &run,
            &instruments,
            &side(
                "article:II",
                json!([["never appears", "quiet hours begin at ten"]])
            )
        ),
        Ok((1, Grade::OwnWords))
    );

    // A section with candidates but no carrier, and a section with none, are
    // different findings and say so.
    let none = side_to_candidate(
        &run,
        &instruments,
        &side("article:II", json!([["bicycles"]])),
    );
    assert!(
        none.as_ref()
            .unwrap_err()
            .contains("3 candidate(s), none carrying"),
        "{none:?}"
    );
    let empty = side_to_candidate(&run, &instruments, &side("article:IV", both));
    assert!(
        empty
            .as_ref()
            .unwrap_err()
            .contains("no candidate came from that section"),
        "{empty:?}"
    );

    // Two candidates sharing one wide citation that carries the anchor, and
    // neither stating it. This is the shape that cost a 17-call run: 152 of
    // the founding corpus's 334 candidates share a citation with a sibling,
    // so picking the lowest index is arbitrary and must refuse instead.
    let shared = json!({
        "chunks": [{ "heading": "Maple House Charter, Article II — Quiet Hours" }],
        "candidates": [
            { "chunk": 0, "text": "Nights are for sleeping.",
              "quote": "Quiet hours begin at ten and end at seven." },
            { "chunk": 0, "text": "Mornings are for coffee.",
              "quote": "Quiet hours begin at ten and end at seven." }
        ]
    });
    let tie = side_to_candidate(
        &shared,
        &instruments,
        &side("article:II", json!([["quiet hours begin at ten"]])),
    );
    let why = tie.as_ref().unwrap_err();
    assert!(why.contains("2 share a citation"), "{why}");
    assert!(why.contains("c0, c1"), "names them: {why}");
    assert!(why.contains("cannot name one"), "{why}");
}
