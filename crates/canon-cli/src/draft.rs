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
//! sentence split, per chunk                   no model
//! map: extract per chunk, cite by position    N completions
//! support: read each rule and its citation    2 x ceil(N/10) completions
//! reduce: group duplicates                    1 completion
//! accept one at a time                        no model
//! tensions over what was accepted             1 completion
//! ```
//!
//! Two invariants are structural rather than remembered.
//!
//! **Every candidate carries its source passage or it is not shown.** The map
//! step answers with the POSITION of the sentence it read, and the code cuts
//! the citation out of the chunk ([`crate::locate`]) — so "the quote is not
//! in the passage" stops being a check that fires and becomes a state that
//! cannot be reached. What can still fail is an index pointing at a sentence
//! the passage does not have, and that is refused and counted. Either way it
//! is cite-or-abstain applied to onboarding: a drafted commitment with no
//! citation is the model inventing a value the user never held.
//!
//! **A rule must be carried by the sentence it cites.** The citation proves
//! the words are the passage's; whether the RULE matches them is a second
//! question, answered by reading the quantities out of both and comparing
//! structure ([`crate::quantify`]). That reading is done once per rule and
//! used twice — here, and by the fold guard, which needs the same answer to
//! refuse to merge two rules stating different limits.
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

use crate::locate;
use crate::model::{self, Client, ModelError};
use crate::profile::Profile;
use crate::quantify;
use crate::store;
use crate::subject;
use crate::tensions;

/// Target chunk size in characters. Big enough that a rule and its rationale
/// stay together; small enough that a quote can be checked against it and
/// that N completions stay affordable.
const CHUNK_TARGET: usize = 1500;
/// Below this, a paragraph is not worth a completion of its own — it is
/// merged forward instead.
const CHUNK_MIN: usize = 40;

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
    /// How many comparison passes the run attempted.
    #[serde(default)]
    pub tension_passes: usize,
    /// Passes that produced no answer, each naming itself and why. A tension
    /// count taken from a run with entries here is a count over a FRACTION of
    /// the pairs, and a reader who cannot see that is being misled (§18.3).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tension_passes_unread: Vec<String>,
    /// The stage that ended this run early, if one did.
    ///
    /// A run with this set is EVIDENCE, never a measurement: the stages after
    /// it never ran, so every count in it is a count about a pipeline that
    /// stopped. The bar refuses to score one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
}

/// Write what a run produced before the stage that ended it, then report.
///
/// A run that dies at the comparison step has already spent every extraction
/// call it made — thirty-three minutes of them, on the sweep that prompted
/// this. Discarding that makes the next attempt pay for it again and leaves
/// nobody able to see what the run actually held. The artifact is marked
/// `failed`, so it reads as evidence and can never be scored as a result.
fn abandon(dir: &Path, artifact: &mut DraftRun, stage: &str, e: ModelError) -> i32 {
    artifact.failed = Some(format!("{stage}: {e}"));
    match persist(dir, artifact) {
        Ok(p) => eprintln!(
            "the {stage} step failed. What ran before it is kept at {}",
            p.display()
        ),
        Err(w) => {
            eprintln!("the {stage} step failed, and the partial run could not be written: {w}")
        }
    }
    model::report(e)
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

The passage is given as numbered sentences, one per line. Each is marked \
[n]. The markers are not part of the text.

For each commitment, return:
- first: the marker of the sentence that states the rule
- last: the marker of the last sentence the rule runs to — the same as first \
when one sentence states it
- text: one self-contained sentence stating the rule as the holder would \
write it in a list of their own commitments. Never write about the author — \
\"Mornings are protected.\", not \"The speaker intends to protect their \
mornings.\" It must make sense with no passage in front of it.

Rules:
- Extract every distinct rule the passage states. A passage stating no rule \
returns an empty list.
- Point at the fewest sentences that state the rule, and never more than \
three.
- first and last must be markers this passage has. Never write one it does \
not.
- Never state a commitment the passage does not.
- Every number, time, day and unit in your sentence must be the one the \
passage uses. Do not convert or round: if the passage says three days, the \
rule says three days, never three hours.";

#[derive(Debug, Deserialize)]
struct Extracted {
    #[serde(default)]
    commitments: Vec<ExtractedOne>,
}

/// Where the rule is, then what it says.
///
/// The order is load-bearing under constrained decoding: the model settles on
/// a position in the passage BEFORE it writes the sentence, which is the
/// grounding the old `quote` field supplied by making it retype the source.
/// Pointing is the cheaper way to get it.
#[derive(Debug, Deserialize)]
struct ExtractedOne {
    /// 1-based marker of the first sentence, as [`locate::numbered`] wrote
    /// it. Defaults to 0 — not a position — so an answer that omits the
    /// field is refused rather than read as "the first sentence".
    #[serde(default)]
    first: usize,
    #[serde(default)]
    last: usize,
    #[serde(default)]
    text: String,
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
                        "first": { "type": "integer" },
                        "last": { "type": "integer" },
                        "text": { "type": "string" },
                    },
                    "required": ["first", "last", "text"],
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

/// Extract from one chunk, cutting each citation out of the chunk at the
/// position the model pointed to.
///
/// The citation guarantee is code, not an instruction to the model (§7.6) —
/// and since the words are copied rather than repeated, it is now a property
/// of the construction rather than a check that has to pass.
pub fn extract(
    client: &Client,
    chunk: &Chunk,
    profile: Profile,
) -> Result<(Vec<Candidate>, Vec<Dropped>), ModelError> {
    let system = format!("{EXTRACT_SYSTEM}\n\n{}", voice(profile));
    let spans = locate::sentences(&chunk.text);
    let shown = locate::numbered(&chunk.text, &spans);
    // The heading goes to the model as CONTEXT, not as citable text.
    //
    // The chunker has always recorded it and never sent it, which threw away
    // what the document itself supplies. A minute headed "Decision —
    // 2026-02-10 — Weeknight Quiet Hours" opens "After repeated complaints
    // about noise on work nights, the house met and resolved…" — the body
    // never names its own subject, and read cold the operative sentence looks
    // like narrative.
    //
    // A rule evidenced only by a title is not evidenced, and that restriction
    // used to be a sentence in the prompt. It is now the shape of the input:
    // only the body is numbered, so the title has no position to cite.
    let user = match &chunk.heading {
        Some(h) => format!(
            "Section title, for context only — it has no marker and cannot be \
             cited: \"{h}\"\n\nPassage:\n{shown}\nReturn the commitments this \
             passage states."
        ),
        None => format!("Passage:\n{shown}\nReturn the commitments this passage states."),
    };
    let got: Extracted = client.complete_json(&system, &user, "commitments", &extract_schema())?;
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    let refuse = |text: String, quote: String, reason: String| Dropped {
        text,
        quote,
        chunk: chunk.id,
        reason,
    };
    for c in got.commitments {
        let text = c.text.trim().to_string();
        if text.is_empty() {
            dropped.push(refuse(text, String::new(), "empty commitment text".into()));
            continue;
        }
        // A citation that could not be cut carries no quote to report, so the
        // reason names the position that was asked for and the ones on offer.
        let quote = match locate::cite(&chunk.text, &spans, c.first, c.last) {
            Ok(q) => q,
            Err(e) => {
                dropped.push(refuse(text, String::new(), e.to_string()));
                continue;
            }
        };
        // The citation proves the words are the passage's. Whether the RULE
        // matches them is a different reading over different text, and it is
        // its own stage — see `support`.
        kept.push(Candidate {
            text,
            quote,
            chunk: chunk.id,
            source: chunk.source.clone(),
        });
    }
    Ok((kept, dropped))
}

// ── support: does the citation carry the rule's numbers? ────

/// What the support stage hands on.
///
/// `candidates` and `quantities` are the same length and the same order, and
/// the reduce step indexes both. Splitting a candidate from its reading would
/// judge every later rule against the one before it, with no symptom until a
/// fold goes wrong — so they travel together.
#[derive(Debug, Default)]
pub struct Supported {
    pub candidates: Vec<Candidate>,
    pub quantities: Vec<Vec<quantify::Quantity>>,
    /// Refused, each naming the number its citation did not carry.
    pub dropped: Vec<Dropped>,
}

/// Read every rule and every citation, drop the rules their own citation does
/// not support, and hand the rule readings on.
///
/// One reading of each rule, used twice. The fold guard needs to know what a
/// rule states in order to refuse to merge it with one stating something
/// else; this guard needs the same answer to check it against the sentence it
/// cites. Two readings would be two answers to one question (§10.6), so the
/// quantities travel with the survivors instead of being asked for again.
///
/// A rule and its citation are read TOGETHER — see
/// [`quantify::quantify_pairs`]. Read in separate passes they need not
/// canonicalise the same instant the same way, and a rule was refused for
/// stating a time its own citation stated.
pub fn support(client: &Client, candidates: Vec<Candidate>) -> Result<Supported, ModelError> {
    if candidates.is_empty() {
        return Ok(Supported::default());
    }
    // Each rule is read alongside its own citation, in one call, because the
    // canonical form the comparison depends on is only agreed within a call.
    let pairs: Vec<(&str, &str)> = candidates
        .iter()
        .map(|c| (c.text.as_str(), c.quote.as_str()))
        .collect();
    let read = quantify::quantify_pairs(client, &pairs)?;

    let mut kept = Vec::new();
    let mut quantities = Vec::new();
    let mut dropped = Vec::new();
    for (c, (rule, cited)) in candidates.into_iter().zip(read) {
        match quantify::unsupported(&rule, &cited) {
            Some(m) => dropped.push(Dropped {
                text: c.text,
                quote: c.quote,
                chunk: c.chunk,
                reason: format!("states `{m}`, which its citation does not"),
            }),
            None => {
                kept.push(c);
                quantities.push(rule);
            }
        }
    }
    Ok(Supported {
        candidates: kept,
        quantities,
        dropped,
    })
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
    quantities: &[Vec<quantify::Quantity>],
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

    // Clean the proposal up before asking anything about it.
    let mut proposed: Vec<Vec<usize>> = Vec::new();
    for g in got.groups {
        let mut members: Vec<usize> = g
            .into_iter()
            .filter(|n| *n >= 1 && *n <= candidates.len())
            .map(|n| n - 1)
            .collect();
        members.sort_unstable();
        members.dedup();
        if members.len() >= 2 {
            proposed.push(members);
        }
    }

    // A rule stating a different quantity from the one it is being folded
    // into is a contradiction, not a duplicate. Enforced in code rather than
    // asked of the reduce step (§7.6): losing this is losing the whole
    // unmarked-supersession category.
    //
    // The quantities are READ BY THE MODEL, one narrow question per rule,
    // and compared here as structure. What stood here compared them with a
    // hand-kept list of units, and on a municipal noise code `85 dBA` and
    // `85 dBC` both parsed as stating no measure at all — the guard never
    // fired and five planted supersessions were deleted. A list is always
    // one document behind; see `quantify`.
    //
    // The reading arrives with the candidates, from `support`, which already
    // had to ask what every rule states. Asking again here would be a second
    // answer to one question, and the two could disagree (§10.6).
    let empty: Vec<quantify::Quantity> = Vec::new();
    let of = |i: &usize| quantities.get(*i).unwrap_or(&empty).as_slice();

    // The second guard: the SAME number about a DIFFERENT thing. A permit
    // schedule restates one sentence per permit type, so type "B" and type
    // "C" state 65 dBAs at 50 feet in almost the same words — the reduce step
    // proposes them as duplicates and the quantity guard has no grounds to
    // refuse. Folding them deleted the type "C" commitment and the planted
    // supersession against it; the Des Moines bar measured 10 of 11 reachable
    // with extraction missing none.
    //
    // Read per GROUP rather than per candidate, because two rules only name
    // one thing the same way when one call names them both — the lesson
    // `quantify_pairs` paid for. See `subject`.
    let group_texts: Vec<Vec<&str>> = proposed
        .iter()
        .map(|g| g.iter().map(|i| candidates[*i].text.as_str()).collect())
        .collect();
    let subjects = subject::same_thing(client, &group_texts)?;

    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut folded: Vec<bool> = vec![false; candidates.len()];
    for (gi, mut members) in proposed.into_iter().enumerate() {
        if let Some(&head) = members.first() {
            let rep = &subjects[gi];
            let head_rep = rep.first().copied().unwrap_or(0);
            let mut at = 0usize;
            members.retain(|m| {
                let here = at;
                at += 1;
                if *m == head {
                    return true;
                }
                if quantify::differs_by_quantity(of(&head), of(m)) {
                    eprintln!(
                        "\nkept apart, not folded — these state different quantities:\n  {}\n  {}",
                        candidates[head].text, candidates[*m].text
                    );
                    return false;
                }
                if rep.get(here).copied().unwrap_or(here) != head_rep {
                    eprintln!(
                        "\nkept apart, not folded — these govern different things:\n  {}\n  {}",
                        candidates[head].text, candidates[*m].text
                    );
                    return false;
                }
                true
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

    // From here the artifact exists and every exit writes it. A stage that
    // fails costs its own work and nothing before it.
    let mut artifact = DraftRun {
        schema: RUN_SCHEMA.into(),
        at: store::now(),
        endpoint: client.endpoint().to_string(),
        model: client.model().to_string(),
        profile: profile.as_str().to_string(),
        sources: sources.iter().map(|(p, _)| p.clone()).collect(),
        chunks: chunks.clone(),
        candidates: candidates.clone(),
        dropped: dropped.clone(),
        unread: unread.clone(),
        duplicates: Vec::new(),
        kept: Vec::new(),
        tensions: Vec::new(),
        tension_passes: 0,
        tension_passes_unread: Vec::new(),
        failed: None,
    };

    // Every rule read once, checked against its own citation, and the reading
    // carried forward to the fold guard.
    let supported = match support(&client, candidates) {
        Ok(v) => v,
        Err(e) => return abandon(&dir, &mut artifact, "support", e),
    };
    if !supported.dropped.is_empty() {
        eprintln!(
            "{} candidate(s) dropped for stating a number their citation does not",
            supported.dropped.len()
        );
    }
    dropped.extend(supported.dropped);
    let (candidates, quantities) = (supported.candidates, supported.quantities);
    artifact.candidates = candidates.clone();
    artifact.dropped = dropped.clone();
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

    let (groups, kept) = match dedupe(&client, &candidates, &quantities) {
        Ok(v) => v,
        Err(e) => return abandon(&dir, &mut artifact, "dedupe", e),
    };
    artifact.duplicates = groups.clone();
    artifact.kept = kept.clone();
    if !groups.is_empty() {
        eprintln!("{} duplicate group(s) folded", groups.len());
    }

    let kept_texts: Vec<&str> = kept.iter().map(|i| candidates[*i].text.as_str()).collect();
    // In a dry run nothing is accepted, so tensions runs over every surviving
    // candidate — that is what the bar scores. In a real run it runs over what
    // the person accepted, below.
    let compared = if dry_run {
        match tensions::detect_over(&client, &kept_texts) {
            Ok(v) => v,
            Err(e) => return abandon(&dir, &mut artifact, "tensions", e),
        }
    } else {
        tensions::Compared::default()
    };
    let run_tensions: Vec<RunTension> = compared
        .pairs
        .into_iter()
        .map(|p| RunTension {
            a: p.a,
            b: p.b,
            reason: p.reason,
        })
        .collect();

    artifact.tensions = run_tensions;
    artifact.tension_passes = compared.passes;
    artifact.tension_passes_unread = compared.unread;
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
