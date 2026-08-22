// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon draft` — the cold start.
//!
//! Nobody has written down how they like to be treated; no team has an
//! `ARCHITECTURE.md` matching what it enforces; no house has a charter until
//! its second bad argument. But the normative content already exists,
//! unextracted, in text everyone already has.
//!
//! Map-reduce over plain completions, which is what keeps this working
//! against a bare `/v1/chat/completions` with no daemon behind it:
//!
//! ```text
//! chunk (paragraph split, heading-aware)      no model
//! map: extract per chunk, keep the chunk id   N completions
//! reduce: group duplicates                    1 completion
//! accept one at a time                        no model
//! tensions over what was accepted             1 completion
//! ```
//!
//! Two invariants are structural rather than remembered.
//!
//! **Every candidate carries its source passage or it is not shown.** The map
//! step must return the words it extracted from, verbatim, and a quote that
//! is not in the chunk drops the candidate. That is cite-or-abstain applied
//! to onboarding: a drafted commitment with no citation is the model
//! inventing a value the user never held.
//!
//! **The reduce step never mints text.** It returns groups of positions, and
//! the first member of each group survives with its own citation intact. A
//! reduce step allowed to rewrite would produce a tidier list whose
//! quotations no longer match anything.
//!
//! And there is no `--accept-all`: a canon adopted wholesale is disengagement
//! at t=0, so onboarding *is* the first governance session.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use canon_core::ActKind;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::model::{self, Client, ModelError};
use crate::profile::Profile;
use crate::store;
use crate::tensions;

/// Target chunk size in characters. Big enough that a rule and its rationale
/// stay together; small enough that a quote can be checked against it and
/// that N completions stay affordable.
const CHUNK_TARGET: usize = 1500;
/// Below this, a paragraph is not worth a completion of its own — it is
/// merged forward instead.
const CHUNK_MIN: usize = 40;
/// A quote shorter than this cannot be evidence of anything.
const QUOTE_MIN: usize = 20;

// ── the artifact ────────────────────────────────────────────

pub const RUNS_DIR: &str = "draft-runs";
pub const RUN_SCHEMA: &str = "canon-draft-run/v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: usize,
    /// `path:first-last`, the span a candidate cites.
    pub source: String,
    /// The nearest preceding markdown heading, when the text has one. Absent
    /// for prose with no structure, which is most journals.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub heading: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub text: String,
    pub quote: String,
    pub chunk: usize,
    pub source: String,
}

/// A chunk the endpoint could not answer for.
///
/// Recorded rather than fatal. One malformed reply out of twenty-four is a
/// partial result, and a partial result reported as a whole one is the exact
/// failure §18.3 names — but so is throwing away twenty-three good answers
/// because the twenty-fourth came back wrong. The artifact carries both the
/// answers and the holes, so a number scored from it can say how much of the
/// document was actually read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unread {
    pub chunk: usize,
    pub source: String,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dropped {
    pub text: String,
    pub quote: String,
    pub chunk: usize,
    pub reason: String,
}

/// Everything a run consumed and produced, persisted so the bar re-scores by
/// REPLAY rather than by re-running the model (§18.4). A run that cannot be
/// re-scored without a second inference call is not instrumented.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftRun {
    pub schema: String,
    pub at: i64,
    pub endpoint: String,
    pub model: String,
    /// Which voice the extraction asked for. A run scored against another
    /// must have used the same one.
    #[serde(default)]
    pub profile: String,
    pub sources: Vec<String>,
    pub chunks: Vec<Chunk>,
    pub candidates: Vec<Candidate>,
    /// Candidates the citation check refused. Kept, because "the extractor
    /// paraphrased nine times" is a measurement, not noise.
    pub dropped: Vec<Dropped>,
    /// Chunks that produced no answer at all.
    #[serde(default)]
    pub unread: Vec<Unread>,
    /// Groups of duplicate candidate positions, as the reduce step found them.
    pub duplicates: Vec<Vec<usize>>,
    /// Candidate positions that survived the reduce, in order.
    pub kept: Vec<usize>,
    /// Tensions proposed over `kept`, in `kept` positions.
    pub tensions: Vec<RunTension>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTension {
    pub a: usize,
    pub b: usize,
    pub reason: String,
}

// ── chunking (no model) ─────────────────────────────────────

/// Split text into chunks on blank lines, never merging across a heading.
///
/// Format-agnostic by default: a paragraph break is a chunk boundary in any
/// plain text. The heading rule costs nothing on unstructured prose (there
/// are no headings to find) and keeps one article, one decision, or one dated
/// journal entry whole where the text does have structure.
pub fn chunk_text(path: &str, text: &str) -> Vec<Chunk> {
    let lines: Vec<&str> = text.lines().collect();
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut cur = String::new();
    let mut start = 1usize;
    let mut end = 1usize;
    let mut heading: Option<String> = None;
    let mut cur_heading: Option<String> = None;

    let is_heading = |l: &str| {
        let t = l.trim_start();
        t.starts_with('#') && t.trim_start_matches('#').starts_with(' ')
    };

    let mut flush = |cur: &mut String, start: usize, end: usize, h: &Option<String>| {
        let body = cur.trim().to_string();
        cur.clear();
        if body.chars().count() < CHUNK_MIN {
            return;
        }
        chunks.push(Chunk {
            id: 0,
            source: format!("{path}:{start}-{end}"),
            heading: h.clone(),
            text: body,
        });
    };

    for (i, raw) in lines.iter().enumerate() {
        let no = i + 1;
        if is_heading(raw) {
            flush(&mut cur, start, end, &cur_heading);
            heading = Some(raw.trim_start().trim_start_matches('#').trim().to_string());
            cur_heading = heading.clone();
            start = no;
            end = no;
            continue;
        }
        if raw.trim().is_empty() {
            // A paragraph break ends the chunk only once it is big enough to
            // be worth its own completion.
            if cur.trim().chars().count() >= CHUNK_TARGET {
                flush(&mut cur, start, end, &cur_heading);
                cur_heading = heading.clone();
                start = no + 1;
            }
            cur.push('\n');
            continue;
        }
        if cur.trim().is_empty() {
            start = no;
            cur_heading = heading.clone();
        }
        cur.push_str(raw);
        cur.push('\n');
        end = no;
    }
    flush(&mut cur, start, end, &cur_heading);

    for (i, c) in chunks.iter_mut().enumerate() {
        c.id = i;
    }
    chunks
}

/// Whitespace-insensitive containment. Models reflow quoted text across line
/// breaks; that is the same words, so it is the same citation.
fn normalize(s: &str) -> String {
    s.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Units a rule measures things in. Deliberately short: these are the words
/// that change what a rule MEANS when they are wrong.
const UNITS: &[&str] = &[
    "minute", "hour", "day", "night", "week", "month", "year", "gallon", "dollar",
];

/// Number words a rule might use where the passage used a digit, or the
/// other way round. `twice a month` and `two times per month` are the same
/// rule and must not be read as different ones.
fn as_number(word: &str) -> Option<u32> {
    const WORDS: &[(&str, u32)] = &[
        ("once", 1),
        ("one", 1),
        ("single", 1),
        ("twice", 2),
        ("two", 2),
        ("three", 3),
        ("four", 4),
        ("five", 5),
        ("six", 6),
        ("seven", 7),
        ("eight", 8),
        ("nine", 9),
        ("ten", 10),
        ("eleven", 11),
        ("twelve", 12),
        ("fifteen", 15),
        ("twenty", 20),
        ("thirty", 30),
        ("sixty", 60),
    ];
    let w = word.trim_matches(|c: char| !c.is_ascii_alphanumeric());
    if let Ok(n) = w.parse::<u32>() {
        return Some(n);
    }
    WORDS.iter().find(|(s, _)| *s == w).map(|(_, n)| *n)
}

fn singular(word: &str) -> String {
    let w: String = word
        .trim_matches(|c: char| !c.is_ascii_alphanumeric())
        .to_string();
    w.strip_suffix('s').map(str::to_string).unwrap_or(w)
}

/// Every clock time in `s`, canonicalised: `11:00 PM`, `11 pm` and `11pm` all
/// become `11pm`.
fn clock_times(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let b: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < b.len() && b[i].is_ascii_digit() {
            i += 1;
        }
        let hour: String = b[start..i].iter().collect();
        if hour.len() > 2 {
            continue;
        }
        let mut j = i;
        // Optional `:mm`, then optional space, then am/pm.
        if j < b.len() && b[j] == ':' {
            j += 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
        }
        while j < b.len() && b[j] == ' ' {
            j += 1;
        }
        let rest: String = b[j..b.len().min(j + 2)].iter().collect();
        if rest == "am" || rest == "pm" {
            out.push(format!("{hour}{rest}"));
            i = j + 2;
        }
    }
    out
}

/// How far apart a number and its unit may sit and still be one measure.
/// "two consecutive nights" is a measure; a number three sentences from a
/// unit is not.
const MEASURE_GAP: usize = 2;

/// Words that turn a number into a time of day rather than a count.
/// "eleven at night" and "11 PM" are the same instant, and reading the first
/// as a count of nights makes two identical rules look like a contradiction.
const TIME_OF_DAY: &[(&str, &str)] = &[("night", "pm"), ("evening", "pm"), ("morning", "am")];

/// Words, split on whitespace AND hyphens.
///
/// Hyphens matter: "within any seven-day period" states the same measure as
/// "per 7 days", and a tokeniser that only splits on spaces sees no number at
/// all in the first.
fn words_of(s: &str) -> Vec<String> {
    s.split(|c: char| c.is_whitespace() || c == '-')
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Is this unit being used as a time of day — "at night", "in the morning"?
fn is_time_of_day(words: &[String], unit_at: usize) -> bool {
    let unit = singular(&words[unit_at]);
    if !TIME_OF_DAY.iter().any(|(u, _)| *u == unit) {
        return false;
    }
    words[..unit_at]
        .iter()
        .rev()
        .take(2)
        .any(|w| matches!(singular(w).as_str(), "at" | "in" | "the"))
}

/// Number-and-unit pairs in `s`, as `(value, unit)`. Times of day are not
/// counts and are excluded — [`clock_times`] picks those up instead.
fn measures(s: &str) -> Vec<(u32, String)> {
    let words = words_of(s);
    let mut out = Vec::new();
    for i in 0..words.len() {
        let Some(n) = as_number(&words[i]) else {
            continue;
        };
        for j in (i + 1)..(i + 2 + MEASURE_GAP).min(words.len()) {
            let u = singular(&words[j]);
            if UNITS.contains(&u.as_str()) {
                if !is_time_of_day(&words, j) {
                    out.push((n, u));
                }
                break;
            }
        }
    }
    out
}

/// Times written in words: "eleven at night" -> `11pm`.
fn spelled_times(s: &str) -> Vec<String> {
    let words = words_of(s);
    let mut out = Vec::new();
    for i in 0..words.len() {
        let Some(n) = as_number(&words[i]) else {
            continue;
        };
        for j in (i + 1)..(i + 2 + MEASURE_GAP).min(words.len()) {
            let u = singular(&words[j]);
            if let Some((_, half)) = TIME_OF_DAY.iter().find(|(t, _)| *t == u) {
                if is_time_of_day(&words, j) {
                    out.push(format!("{n}{half}"));
                }
                break;
            }
        }
    }
    out
}

/// Does this rule state a measure its passage does not?
///
/// **The citation check proves the QUOTE is real. It does not prove the RULE
/// matches it.** Observed against a live endpoint: a candidate read "at least
/// three hours in advance" while its own verbatim quote said "three days
/// ahead". A rule that misstates a time or a count is worse than a missing
/// rule — it is a house rule contradicting the sentence printed beneath it,
/// and the citation makes it look checked.
///
/// Narrow on purpose. Only NUMBER-AND-UNIT pairs and clock times are
/// compared, so a rule that rewords "within any seven-day period" as "per
/// week" survives — that is a paraphrase, not a different rule. What does not
/// survive is a quantity the passage never states.
fn unstated_measure(text: &str, chunk: &str) -> Option<String> {
    let (t, c) = (normalize(text), normalize(chunk));
    let source: Vec<(u32, String)> = measures(&c);
    for (n, unit) in measures(&t) {
        if !source.iter().any(|(m, u)| *m == n && *u == unit) {
            return Some(format!("{n} {unit}(s)"));
        }
    }
    let mut source_times = clock_times(&c);
    source_times.extend(spelled_times(&c));
    let mut stated = clock_times(&t);
    stated.extend(spelled_times(&t));
    stated.into_iter().find(|time| !source_times.contains(time))
}

// ── map: extract (one completion per chunk) ─────────────────

const EXTRACT_SYSTEM: &str = "\
You extract normative commitments from a passage.

A commitment is a rule, a standard, or a stated value — something that says \
how things should be.

A passage often records a decision, and a decision that changes a rule \
STATES a rule. Extract the rule it establishes, not the meeting that \
established it: \"Quiet hours begin at 10:00 PM Sunday through Thursday\" is a \
commitment; \"the house met and resolved to move quiet hours earlier\" is not. \
When a passage changes one part of a rule and leaves the rest, the part that \
changed is a commitment.

For each one, return:
- text: one self-contained sentence stating the rule as the holder would \
write it in a list of their own commitments. Never write about the author — \
\"Mornings are protected.\", not \"The speaker intends to protect their \
mornings.\" It must make sense with no passage in front of it.
- quote: the words from the passage it came from, copied exactly. Do not \
paraphrase, do not fix wording, do not shorten with ellipses.

Rules:
- Extract every distinct rule the passage states. A passage stating no rule \
returns an empty list.
- Never state a commitment the passage does not.
- Every number, time, day and unit in your sentence must be the one the \
passage uses. Do not convert or round: if the passage says three days, the \
rule says three days, never three hours.
- A quote that is not word-for-word from the passage is a failure.";

#[derive(Debug, Deserialize)]
struct Extracted {
    #[serde(default)]
    commitments: Vec<ExtractedOne>,
}

#[derive(Debug, Deserialize)]
struct ExtractedOne {
    #[serde(default)]
    text: String,
    #[serde(default)]
    quote: String,
}

fn extract_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "commitments": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "text": { "type": "string" },
                        "quote": { "type": "string" },
                    },
                    "required": ["text", "quote"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["commitments"],
        "additionalProperties": false,
    })
}

/// Whose voice the extracted rules are written in.
///
/// The profile is already known and it is the difference between a house
/// charter reading "Quiet hours run 11pm-7am" and reading "I observe quiet
/// hours" — which is what a house canon extracted without this said, and it
/// is wrong in a way a reader notices immediately.
fn voice(profile: Profile) -> &'static str {
    match profile {
        Profile::Personal => {
            "These are one person's own commitments. Write each as they would state it about \
             themselves: \"Mornings are protected; I do not schedule before 11.\""
        }
        Profile::Code => {
            "These are a codebase's standards. Write each as a standard the code is held to: \
             \"One implementation per threshold, scorer, schema and key.\" Never write about \
             a team or an author."
        }
        Profile::House => {
            "These are a household's rules, held by the house rather than by any one member. \
             Write each as a house rule: \"Quiet hours run 11pm to 7am.\" Never write \"I\" \
             or \"the speaker\"."
        }
    }
}

/// Extract from one chunk, keeping only candidates whose quote is actually in
/// the chunk. The check is the citation guarantee, and it is code rather than
/// an instruction to the model (§7.6).
pub fn extract(
    client: &Client,
    chunk: &Chunk,
    profile: Profile,
) -> Result<(Vec<Candidate>, Vec<Dropped>), ModelError> {
    let system = format!("{EXTRACT_SYSTEM}\n\n{}", voice(profile));
    // The heading goes to the model as CONTEXT, not as citable text.
    //
    // The chunker has always recorded it and never sent it, which threw away
    // what the document itself supplies. A minute headed "Decision —
    // 2026-02-10 — Weeknight Quiet Hours" opens "After repeated complaints
    // about noise on work nights, the house met and resolved…" — the body
    // never names its own subject, and read cold the operative sentence looks
    // like narrative.
    //
    // Quoting is still restricted to the body, because a rule evidenced only
    // by a title is not evidenced.
    let user = match &chunk.heading {
        Some(h) => format!(
            "Section title, for context only — do not quote from it: \"{h}\"\n\n\
             Passage:\n{}\n\nReturn the commitments this passage states.",
            chunk.text
        ),
        None => format!(
            "Passage:\n{}\n\nReturn the commitments this passage states.",
            chunk.text
        ),
    };
    let got: Extracted = client.complete_json(&system, &user, "commitments", &extract_schema())?;
    let haystack = normalize(&chunk.text);
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    for c in got.commitments {
        let text = c.text.trim().to_string();
        let quote = c.quote.trim().to_string();
        let reason: Option<String> = if text.is_empty() {
            Some("empty commitment text".into())
        } else if quote.chars().count() < QUOTE_MIN {
            Some("quote too short to be evidence".into())
        } else if !haystack.contains(&normalize(&quote)) {
            Some("quote is not in the passage — paraphrased, not cited".into())
        } else {
            unstated_measure(&text, &chunk.text)
                .map(|m| format!("states `{m}`, which the passage does not"))
        };
        match reason {
            Some(r) => dropped.push(Dropped {
                text,
                quote,
                chunk: chunk.id,
                reason: r,
            }),
            None => kept.push(Candidate {
                text,
                quote,
                chunk: chunk.id,
                source: chunk.source.clone(),
            }),
        }
    }
    Ok((kept, dropped))
}

// ── reduce: group duplicates (one completion) ───────────────

const DEDUPE_SYSTEM: &str = "\
You group duplicate commitments.

Two commitments are duplicates when they state the same rule, even in \
different words. A general rule and a narrower rule are not duplicates.

Two rules about the same subject that say DIFFERENT things are never \
duplicates. If they state different times, counts or limits, that is a \
contradiction — \"quiet hours start at 11 PM\" and \"quiet hours start at 10 \
PM\" are two rules, not one, and collapsing them destroys the disagreement.

Return one group per set of duplicates, as the numbers of its members. \
A commitment with no duplicate is not returned.";

/// Every measure a rule states, as comparable strings.
fn measure_set(s: &str) -> std::collections::BTreeSet<String> {
    let n = normalize(s);
    measures(&n)
        .into_iter()
        .map(|(v, u)| format!("{v} {u}"))
        .chain(clock_times(&n))
        .chain(spelled_times(&n))
        .collect()
}

/// Do these two rules state different measures?
///
/// **This is the guard that keeps `draft` able to find an unmarked
/// supersession at all.** Two rules that contradict each other look exactly
/// like near-duplicates to a reduce step — same subject, different content —
/// and folding them deletes the disagreement before anything can notice it.
/// Observed: "Quiet hours run from 11:00 PM to 7:00 AM every night" and
/// "Quiet hours begin at 10:00 PM from Sunday through Thursday" were grouped
/// as duplicates, which on its own destroyed two of the eleven planted
/// tensions in the bench fixture.
///
/// Only fires when BOTH rules state measures. A rule with no numbers in it
/// may still be a duplicate of one that has them.
fn differs_by_measure(a: &str, b: &str) -> bool {
    let (ma, mb) = (measure_set(a), measure_set(b));
    !ma.is_empty() && !mb.is_empty() && ma != mb
}

#[derive(Debug, Deserialize)]
struct Grouped {
    #[serde(default)]
    groups: Vec<Vec<usize>>,
}

fn dedupe_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "groups": {
                "type": "array",
                "items": { "type": "array", "items": { "type": "integer" } },
            },
        },
        "required": ["groups"],
        "additionalProperties": false,
    })
}

/// Group duplicates and keep the first of each group.
///
/// Returns `(groups, kept_positions)`. The model never returns text here, so
/// a surviving candidate keeps the quotation it was extracted with.
pub fn dedupe(
    client: &Client,
    candidates: &[Candidate],
) -> Result<(Vec<Vec<usize>>, Vec<usize>), ModelError> {
    if candidates.len() < 2 {
        return Ok((Vec::new(), (0..candidates.len()).collect()));
    }
    let mut user = String::from("Commitments:\n");
    for (i, c) in candidates.iter().enumerate() {
        user.push_str(&format!("{}. {}\n", i + 1, c.text));
    }
    user.push_str("\nReturn the groups of duplicates.");
    let got: Grouped = client.complete_json(DEDUPE_SYSTEM, &user, "groups", &dedupe_schema())?;

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut folded: Vec<bool> = vec![false; candidates.len()];
    for g in got.groups {
        let mut members: Vec<usize> = g
            .into_iter()
            .filter(|n| *n >= 1 && *n <= candidates.len())
            .map(|n| n - 1)
            .collect();
        members.sort_unstable();
        members.dedup();
        // A rule stating different measures from the one it is being folded
        // into is a contradiction, not a duplicate. Enforced here rather than
        // asked of the model (§7.6): losing this is losing the whole
        // unmarked-supersession category.
        if let Some(&head) = members.first() {
            members.retain(|m| {
                let keep =
                    *m == head || !differs_by_measure(&candidates[head].text, &candidates[*m].text);
                if !keep {
                    eprintln!(
                        "\nkept apart, not folded — these state different measures:\n  {}\n  {}",
                        candidates[head].text, candidates[*m].text
                    );
                }
                keep
            });
        }
        if members.len() < 2 {
            continue;
        }
        // Everything after the first member of a group folds into it.
        for m in &members[1..] {
            folded[*m] = true;
        }
        groups.push(members);
    }
    let kept: Vec<usize> = (0..candidates.len()).filter(|i| !folded[*i]).collect();
    Ok((groups, kept))
}

// ── sources ─────────────────────────────────────────────────

fn read_sources(args: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut out: Vec<(String, String)> = Vec::new();
    if crate::cmds::has(args, "--from-git") {
        let since = crate::cmds::flag(args, "--since").unwrap_or("1y");
        out.extend(read_git(since)?);
    }
    for p in from_paths(args) {
        let path = PathBuf::from(&p);
        if path.is_dir() {
            let found = walk(&path)?;
            if found.is_empty() {
                return Err(format!("{p} has no {} files in it", READABLE.join(" or ")));
            }
            for f in found {
                let text = std::fs::read_to_string(&f)
                    .map_err(|e| format!("reading {}: {e}", f.display()))?;
                out.push((f.to_string_lossy().to_string(), text));
            }
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("reading {p}: {e}"))?;
        out.push((p, text));
    }
    if out.is_empty() {
        return Err(
            "nothing to draft from — `canon draft --from <paths>` or `--from-git --since 1y`"
                .into(),
        );
    }
    Ok(out)
}

/// What a directory walk will pick up. Deliberately narrow: pointing `draft`
/// at a folder should read the notes in it, not every binary underneath.
const READABLE: &[&str] = &["md", "txt", "markdown", "text"];

/// Every readable file under a directory, sorted.
///
/// Sorted because chunk ids are positions, and a run that reads the same
/// folder twice in a different order produces a draft-run artifact that
/// cannot be compared with the first (§18.4).
fn walk(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("reading {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // Hidden directories are infrastructure, not notes: `.git` alone
        // would bury a run in objects.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            out.extend(walk(&path)?);
        } else if path
            .extension()
            .map(|e| READABLE.contains(&e.to_string_lossy().as_ref()))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
    Ok(out)
}

/// `--from` takes every following argument until the next flag, because a
/// shell expands `--from ~/notes/**/*.md` into many arguments.
fn from_paths(args: &[String]) -> Vec<String> {
    let Some(i) = args.iter().position(|a| a == "--from") else {
        return Vec::new();
    };
    args[i + 1..]
        .iter()
        .take_while(|a| !a.starts_with('-'))
        .cloned()
        .collect()
}

/// Commit bodies as source text. Extends `store::actor`'s shell-out pattern
/// rather than taking a git dependency.
fn read_git(since: &str) -> Result<Vec<(String, String)>, String> {
    let out = std::process::Command::new("git")
        .args([
            "log",
            &format!("--since={since}"),
            "--format=%H%x1f%B%x1e",
            "--no-merges",
        ])
        .output()
        .map_err(|e| format!("running git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "git log failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    Ok(text
        .split('\u{1e}')
        .filter_map(|record| {
            let (sha, body) = record.trim_start().split_once('\u{1f}')?;
            let body = body.trim();
            (!body.is_empty()).then(|| (format!("git:{}", &sha[..sha.len().min(12)]), body.into()))
        })
        .collect())
}

// ── the verb ────────────────────────────────────────────────

pub fn run(args: &[String]) -> i32 {
    if crate::cmds::has(args, "--accept-all") {
        eprintln!("error: there is no --accept-all, on purpose.");
        eprintln!("  A canon adopted wholesale is disengagement at t=0. Accepting one at a");
        eprintln!("  time is what makes onboarding the first governance session.");
        return 2;
    }
    let dry_run = crate::cmds::has(args, "--dry-run");
    let dir = match crate::cmds::dir() {
        Ok(d) => d,
        Err(e) => return crate::cmds::fail(e),
    };
    let profile = match Profile::load(&dir) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(e),
    };
    let sources = match read_sources(args) {
        Ok(s) => s,
        Err(e) => return crate::cmds::fail(e),
    };

    let mut chunks: Vec<Chunk> = Vec::new();
    for (path, text) in &sources {
        chunks.extend(chunk_text(path, text));
    }
    for (i, c) in chunks.iter_mut().enumerate() {
        c.id = i;
    }
    if chunks.is_empty() {
        return crate::cmds::fail("nothing readable in those sources");
    }

    let client = match model::client_for(&dir, crate::cmds::has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!(
        "{} chunk(s) from {} source(s) on {}",
        chunks.len(),
        sources.len(),
        client.describe()
    );

    let mut candidates: Vec<Candidate> = Vec::new();
    let mut dropped: Vec<Dropped> = Vec::new();
    let mut unread: Vec<Unread> = Vec::new();
    for chunk in &chunks {
        eprint!("\rextracting {}/{}…", chunk.id + 1, chunks.len());
        let _ = std::io::stderr().flush();
        match extract(&client, chunk, profile) {
            Ok((k, d)) => {
                candidates.extend(k);
                dropped.extend(d);
            }
            // One chunk's failure is not the document's. Record which
            // passage went unread and keep going; the alternative throws away
            // every good answer because one reply came back wrong.
            Err(e) => {
                eprintln!("\nwarning: {} produced no answer: {e}", chunk.source);
                unread.push(Unread {
                    chunk: chunk.id,
                    source: chunk.source.clone(),
                    error: e.to_string(),
                });
            }
        }
    }
    if unread.len() == chunks.len() {
        eprintln!("\nno chunk produced an answer.");
        return 3;
    }
    eprintln!(
        "\r{} candidate(s), {} dropped for a bad citation",
        candidates.len(),
        dropped.len()
    );
    if !unread.is_empty() {
        // Loud, because every number computed from this run is a number about
        // a fraction of the document.
        eprintln!(
            "WARNING: {} of {} passage(s) went unread — this run saw {:.0}% of the document",
            unread.len(),
            chunks.len(),
            100.0 * (chunks.len() - unread.len()) as f64 / chunks.len() as f64
        );
    }

    let (groups, kept) = match dedupe(&client, &candidates) {
        Ok(v) => v,
        Err(e) => return model::report(e),
    };
    if !groups.is_empty() {
        eprintln!("{} duplicate group(s) folded", groups.len());
    }

    let kept_texts: Vec<&str> = kept.iter().map(|i| candidates[*i].text.as_str()).collect();
    // In a dry run nothing is accepted, so tensions runs over every surviving
    // candidate — that is what the bar scores. In a real run it runs over what
    // the person accepted, below.
    let run_tensions: Vec<RunTension> = if dry_run {
        match tensions::detect_over(&client, &kept_texts) {
            Ok(v) => v
                .into_iter()
                .map(|p| RunTension {
                    a: p.a,
                    b: p.b,
                    reason: p.reason,
                })
                .collect(),
            Err(e) => return model::report(e),
        }
    } else {
        Vec::new()
    };

    let artifact = DraftRun {
        schema: RUN_SCHEMA.into(),
        at: store::now(),
        endpoint: client.endpoint().to_string(),
        model: client.model().to_string(),
        profile: profile.as_str().to_string(),
        sources: sources.iter().map(|(p, _)| p.clone()).collect(),
        chunks: chunks.clone(),
        candidates: candidates.clone(),
        dropped,
        unread,
        duplicates: groups,
        kept: kept.clone(),
        tensions: run_tensions,
    };
    let path = match persist(&dir, &artifact) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(e),
    };

    if dry_run {
        if crate::cmds::has(args, "--json") {
            println!(
                "{}",
                serde_json::to_string_pretty(&artifact).unwrap_or_default()
            );
        } else {
            for i in &kept {
                println!("  {}\n    {}", candidates[*i].text, candidates[*i].source);
            }
            println!(
                "\n{} candidate(s), {} tension(s) proposed. Nothing written.",
                kept.len(),
                artifact.tensions.len()
            );
            println!("run recorded at {}", path.display());
        }
        return 0;
    }
    eprintln!("run recorded at {}", path.display());

    // ── one at a time ───────────────────────────────────────
    let accepted = match review(&dir, &candidates, &kept) {
        Ok(a) => a,
        Err(e) => return crate::cmds::fail(e),
    };
    if accepted.is_empty() {
        println!("nothing accepted.");
        return 0;
    }
    println!("\n{} commitment(s) accepted.", accepted.len());

    // ── the moment it has to produce ────────────────────────
    let Ok(canon) = store::read(&dir).map(|l| l.derive()) else {
        return 0;
    };
    let fresh: Vec<&canon_core::Commitment> = canon
        .active()
        .filter(|c| accepted.contains(&c.id))
        .collect();
    if fresh.len() < 2 {
        return 0;
    }
    eprintln!("looking for tensions in what you just accepted…");
    match tensions::detect(&client, &fresh) {
        Ok(found) => {
            let (open, settled) = tensions::unsettled(&canon, found);
            if !open.is_empty() {
                println!("\nYou already disagree with yourself:\n");
            }
            print!("{}", tensions::render(&canon, &open, settled));
        }
        // The commitments are written and safe; only the closing report
        // failed. Say which, rather than implying the accepts were lost.
        Err(e) => {
            eprintln!("commitments were written. The tensions pass could not run: {e}");
            return e.exit_code();
        }
    }
    0
}

/// Interactive review. `[a]ccept [e]dit [r]eject [s]kip [q]uit`, one at a
/// time, no bulk verb.
fn review(
    dir: &Path,
    candidates: &[Candidate],
    kept: &[usize],
) -> Result<Vec<canon_core::ActId>, String> {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut accepted = Vec::new();
    for (n, i) in kept.iter().enumerate() {
        let c = &candidates[*i];
        println!("\nCandidate {} of {}", n + 1, kept.len());
        println!("  \"{}\"\n", c.text);
        println!("  from {}:", c.source);
        for l in c.quote.lines() {
            println!("    {l}");
        }
        print!("\n  [a]ccept  [e]dit  [r]eject  [s]kip  [q]uit: ");
        let _ = std::io::stdout().flush();
        let Some(Ok(answer)) = lines.next() else {
            println!("\n(end of input)");
            break;
        };
        // Piped input echoes nothing, so the prompt and the reply would run
        // together in a transcript. A terminal supplies this newline itself.
        println!();
        let text = match answer.trim() {
            "a" | "accept" => c.text.clone(),
            "e" | "edit" => {
                print!("  text: ");
                let _ = std::io::stdout().flush();
                match lines.next() {
                    Some(Ok(t)) if !t.trim().is_empty() => t.trim().to_string(),
                    _ => {
                        println!("  (nothing entered — skipped)");
                        continue;
                    }
                }
            }
            "q" | "quit" => break,
            _ => continue,
        };
        let act = crate::cmds::write(
            dir,
            ActKind::Assert {
                text,
                from: None,
                // The citation travels into the log, so `why` can show where a
                // drafted commitment came from months later.
                source: Some(c.source.clone()),
            },
        )?;
        println!("  {}", act.id);
        accepted.push(act.id);
    }
    Ok(accepted)
}

fn persist(dir: &Path, run: &DraftRun) -> Result<PathBuf, String> {
    let runs = dir.join(RUNS_DIR);
    std::fs::create_dir_all(&runs).map_err(|e| format!("creating {}: {e}", runs.display()))?;
    let mut path = runs.join(format!("{}.json", run.at));
    let mut n = 1;
    // Repeat runs for a noise floor land in the same second often enough to
    // matter; a run that silently overwrote its predecessor would corrupt the
    // measurement it exists to support.
    while path.exists() {
        path = runs.join(format!("{}-{n}.json", run.at));
        n += 1;
    }
    let body = serde_json::to_string_pretty(run).map_err(|e| e.to_string())?;
    std::fs::write(&path, body).map_err(|e| format!("writing {}: {e}", path.display()))?;
    Ok(path)
}

#[cfg(test)]
mod tests;
