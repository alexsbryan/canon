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

use std::io::{BufRead, Read as _, Write};
use std::path::{Path, PathBuf};

use canon_core::ActKind;
use serde::{Deserialize, Serialize};

use crate::locate::{self, Quote};
use crate::model::{self, Client, ModelError, Schema};
use crate::profile::Profile;
use crate::quantify;
use crate::seen::{Seen, Why};
use crate::sources::{self, Gathered, Source};
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
    /// When this passage was said in its source, if the source supplies a
    /// clock. A range is necessary for a chat burst containing two speakers.
    /// Neither endpoint is the act's `ts_unix` or a date of legal effect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said_at: Option<SourceTime>,
    /// Read one unit per line, because prose splitting found no structure in
    /// it ([`locate::Basis::Lines`]). Recorded because it changes what a
    /// citation into this passage MEANS — one row of a table, not one
    /// sentence of an argument — and a run that cannot say which cannot be
    /// read properly afterwards.
    #[serde(default, skip_serializing_if = "is_false")]
    pub by_line: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTime {
    pub from: i64,
    pub to: i64,
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// What kind of act a candidate would become.
///
/// **Three, because a group's normative content is three shapes and a tool
/// that only knows one imports a list of rules rather than onboarding
/// anybody.** A meeting note that says "nobody has ever said who looks after
/// the allotment" is recording a QUESTION, and one that says "decided not to
/// make a rota — it would turn a kindness into a duty" is recording a
/// SILENCE. Both were being dropped on the floor by an extractor that could
/// only mint commitments, and both are first-class acts in the format.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// A rule, a standard, a stated value.
    #[default]
    Rule,
    /// Something the passage says nobody has decided.
    Question,
    /// Something the passage says is deliberately unwritten.
    Silence,
    /// Something the passage says HAPPENED, or why the body acted.
    ///
    /// A recital, a finding, a grievance. Past tense, about a particular
    /// actor or occasion, with nothing in it to honour or breach — so no
    /// proposal can ever sit with or against it.
    ///
    /// **Measured on the founding corpus.** The Declaration's twenty-nine
    /// accusations against George III — "He has refused his Assent to Laws"
    /// — came back as twenty-nine RULES, because a rule was the only shape
    /// on offer. They are 6.8% of that corpus and they entered the all-pairs
    /// comparison, where every one of them was weighed against all three
    /// hundred and seventeen others and could not have conflicted with any.
    /// A body's founding document usually carries a justification, and a
    /// justification is not a commitment.
    ///
    /// **The first run after this kind shipped minted ONE record in 342
    /// candidates, and that was a prompt defect, not a modelling one
    /// (2026-08-26).** Two of the three grievance chunks came back
    /// `{"commitments": []}` — the extractor read twenty-two accusations and
    /// returned nothing. The prompt asked for "normative commitments",
    /// defined one as "something that says how things should be", and closed
    /// with "a passage stating no rule returns an empty list"; a model
    /// returning nothing for a page of grievances was obeying it. The
    /// `record` bullet contradicted the task statement instead of extending
    /// it. Adding `record` to the return-field list — the fix the code shape
    /// suggests — changed nothing on its own; widening the closing rule and
    /// the task statement took the same twenty-two grievances from 0 to 11.
    /// Of the three records the extractor DID mint, two more died to a
    /// citation guard that had not yet been told what kind it was refusing
    /// (see [`Kind::span_max`]).
    Record,
}

/// The word the model answered with, as a kind.
///
/// Named and separate so the fallback is stated once and can be tested. A
/// word that is not one of the three is a RULE, which is the kind that has to
/// clear the citation and quantity guards — so a typo lands on the strictest
/// treatment rather than skipping both.
pub fn kind_of(word: &str) -> Kind {
    match word.trim() {
        "question" => Kind::Question,
        "silence" => Kind::Silence,
        "record" => Kind::Record,
        _ => Kind::Rule,
    }
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Rule => "RULE",
            Kind::Question => "QUESTION",
            Kind::Silence => "SILENCE",
            Kind::Record => "RECORD",
        }
    }

    /// Can a proposal sit with or against this?
    ///
    /// **The one decider for what enters comparison**, and the reason the
    /// kinds are enumerated by their effect on adjudication rather than by
    /// subject matter. Before this the filter was `kind == Kind::Rule`
    /// written out at the comparison site, so every new kind was a silent
    /// vote for "not compared" at one call site and "compared" at another.
    ///
    /// Comparison is all-pairs and therefore QUADRATIC in what this returns
    /// true for: on the founding corpus, dropping the twenty-nine recitals
    /// takes 318 commitments to 289, and 756 comparison passes to 630.
    /// Modelling the corpus better and making it affordable are the same
    /// lever, not two projects.
    ///
    /// **That saving has never been this function's (2026-08-26).** The
    /// 2026-08-26 run carried 330 bearing candidates and ONE record, because
    /// the recitals were not classified out here — they were never extracted
    /// (see [`Kind::Record`]). The cost was already absent, so the number
    /// above is what this SHOULD save once the extractor mints recitals
    /// again, not what it has been observed saving.
    pub fn bears(self) -> bool {
        match self {
            Kind::Rule => true,
            // A question is a gap, a silence is a gap held on purpose, and a
            // record is about the past. None of the three can be breached.
            Kind::Question | Kind::Silence | Kind::Record => false,
        }
    }

    /// The widest citation this kind may make, in sentences of a passage that
    /// has `have` of them.
    ///
    /// **The second kind-blind decider, found 2026-08-26.** `locate::cite`
    /// ran twelve lines BEFORE the kind was read, so [`locate::SPAN_MAX`] —
    /// justified for a rule, and whose own refusal text says "evidences the
    /// passage, not the rule" — was applied to every kind. The Declaration's
    /// chunk 5 returned THREE records and two were refused for citing five
    /// sentences, which is how `Kind::Record` came to fire once in 342
    /// candidates and read as an extractor that declines to mint recitals.
    ///
    /// A rule states itself in a sentence or two. A record is about an
    /// occasion, and a passage narrates an occasion across a paragraph. So
    /// the share of the passage is the honest measure for a recital rather
    /// than a count fitted to the two citations that were refused — and it is
    /// never stricter than a rule's, so a short passage cannot make a record
    /// harder to cite than the commitment beside it.
    pub fn span_max(self, have: usize) -> usize {
        match self {
            Kind::Rule | Kind::Question | Kind::Silence => locate::SPAN_MAX,
            Kind::Record => (have / 2).max(locate::SPAN_MAX),
        }
    }
}

/// A rule, question or silence a reading proposed, with the words it cites.
///
/// `Q` is the quote: a [`Quote`] everywhere a candidate is shown or judged,
/// and a bare `String` only between reading a run file and
/// [`DraftRun::bound`], which is the one way across. `Candidate<Quote>` has
/// no `Deserialize`, so a quote from disk cannot skip being found in its
/// passage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate<Q = Quote> {
    pub text: String,
    pub quote: Q,
    pub chunk: usize,
    pub source: String,
    /// Defaulted so a run artifact written before this existed still reads —
    /// everything in one was a rule.
    #[serde(default)]
    pub kind: Kind,
    /// What a deliberate silence protects. Required for `Kind::Silence` and
    /// empty otherwise; a silence with no reason cannot be told apart from
    /// having forgotten, which is the whole distinction it exists to make.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub because: String,
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
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DraftRun<Q = Quote> {
    pub schema: String,
    pub at: i64,
    pub endpoint: String,
    pub model: String,
    /// What the endpoint reported as answering, when it named one. `model` is
    /// what was asked for and is often an alias; this is what replied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub served_model: Option<String>,
    /// Which voice the extraction asked for. A run scored against another
    /// must have used the same one.
    #[serde(default)]
    pub profile: String,
    pub sources: Vec<String>,
    /// Files under those paths that were not read, by reason. Part of the
    /// artifact because a coverage number computed without it is wrong.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")]
    pub skipped: std::collections::BTreeMap<String, usize>,
    pub chunks: Vec<Chunk>,
    pub candidates: Vec<Candidate<Q>>,
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
    /// The comparison schedule this run used: how many passes, how many
    /// commitments in one, and how many times every pair was weighed. A
    /// recall number is a number ABOUT a schedule, and two runs on different
    /// schedules are not comparable — so the schedule travels with the
    /// number rather than living in whoever ran it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tension_schedule: Option<tensions::Schedule>,
    /// Passages skipped because this canon had already read them.
    ///
    /// A count taken from a run with this set is a count over what was NEW,
    /// not over the document — and a reader who cannot see that is being
    /// misled about coverage the same way `unread` would mislead them.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub already_read: usize,
    /// Passages `--max-chunks` refused to read. A cap nobody was told about
    /// reads as coverage (§18.5).
    #[serde(default, skip_serializing_if = "is_zero")]
    pub capped: usize,
    /// The stage that ended this run early, if one did.
    ///
    /// A run with this set is EVIDENCE, never a measurement: the stages after
    /// it never ran, so every count in it is a count about a pipeline that
    /// stopped. The bar refuses to score one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<String>,
    /// Set when this run came off a recording.
    ///
    /// Names the artifact it was replayed from. Without it a replay is
    /// indistinguishable from a live run in the artifact, and a scorer would
    /// average a re-score in with a measurement.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replayed_from: Option<String>,
    /// Set when this run was finished from another run's recorded extraction.
    ///
    /// Names that artifact and why it had stopped. The passages were read
    /// once, by the run named here; this one paid only for the stages after.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continued_from: Option<String>,
    /// Rules the support stage kept without a number check, because no
    /// reading of them came back. A drop count from a run with entries here
    /// is a count over the rules that WERE read.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub support_unchecked: Vec<String>,
    /// Set when a run stopped on purpose rather than on an error.
    ///
    /// Distinct from `failed`, which means something went wrong. Both make the
    /// run EVIDENCE rather than a measurement of the whole pipeline, and the
    /// bar refuses to score either — but a reader who cannot tell a deliberate
    /// extract-only arm from a crash is being misled about both (§18.3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stopped_after: Option<String>,
    /// Set while a run is STILL GOING, naming the last stage that finished.
    ///
    /// The third way a run can be incomplete, and the only one no Rust code
    /// gets to report. `failed` means a stage errored and `stopped_after`
    /// means a run stopped on purpose — both are written by code that ran.
    /// This one means the process was KILLED: SIGKILL, an OOM kill, a power
    /// cut, a laptop rebooting under a launchd job. Like the other two it
    /// makes the run EVIDENCE and never a measurement, and the bar refuses to
    /// score it.
    ///
    /// It appears only in the `.partial.json` that [`checkpoint`] writes and
    /// a finishing run removes, so a finished artifact never carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checkpoint: Option<String>,
    /// Every reply the endpoint gave this run, in order.
    ///
    /// **This is what makes an arm cost a stage instead of a run.** The
    /// expensive half of a run is the model; everything canon does afterwards
    /// — cutting citations, refusing a silence with no reason, refusing a rule
    /// whose number its citation lacks, folding duplicates — is pure code over
    /// these strings. With them on disk,
    /// `--replay` re-runs all of it at zero model cost.
    ///
    /// Written on dry runs only: a real run is a person's canon, not evidence,
    /// and their notes should not be copied into an artifact nobody asked for.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tape: Vec<model::TapeEntry>,
}

impl DraftRun<String> {
    /// The run as recorded, each candidate's quote found in the passage it
    /// names: its own chunk, by id and source. A quote that passage does not
    /// have refuses the file and names the candidate, because a run that
    /// misquotes its own passages is not a record of a reading.
    pub fn bound(self) -> Result<DraftRun, String> {
        // Every field by name, so a field added to the run is a compile
        // error here until it is carried across.
        let DraftRun {
            schema,
            at,
            endpoint,
            model,
            served_model,
            profile,
            sources,
            skipped,
            chunks,
            candidates,
            dropped,
            unread,
            duplicates,
            kept,
            tensions,
            tension_passes,
            tension_passes_unread,
            tension_schedule,
            already_read,
            capped,
            failed,
            replayed_from,
            continued_from,
            support_unchecked,
            stopped_after,
            checkpoint,
            tape,
        } = self;
        let candidates = candidates
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let passage = chunks
                    .get(c.chunk)
                    .filter(|p| p.id == c.chunk && p.source == c.source)
                    .ok_or_else(|| {
                        format!(
                            "candidate {i} cites chunk {} of {}, which the run does not have",
                            c.chunk, c.source
                        )
                    })?;
                let quote = Quote::within(&passage.text, &c.quote).ok_or_else(|| {
                    format!("candidate {i}'s quote is not in its passage, {}", c.source)
                })?;
                Ok(Candidate {
                    text: c.text,
                    quote,
                    chunk: c.chunk,
                    source: c.source,
                    kind: c.kind,
                    because: c.because,
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(DraftRun {
            schema,
            at,
            endpoint,
            model,
            served_model,
            profile,
            sources,
            skipped,
            chunks,
            candidates,
            dropped,
            unread,
            duplicates,
            kept,
            tensions,
            tension_passes,
            tension_passes_unread,
            tension_schedule,
            already_read,
            capped,
            failed,
            replayed_from,
            continued_from,
            support_unchecked,
            stopped_after,
            checkpoint,
            tape,
        })
    }
}

/// A run file, read and bound. The only way a recorded run is read.
pub fn read_run(raw: &str) -> Result<DraftRun, String> {
    serde_json::from_str::<DraftRun<String>>(raw)
        .map_err(|e| e.to_string())?
        .bound()
}

fn is_zero(n: &usize) -> bool {
    *n == 0
}

/// Write what a run produced before the stage that ended it, then report.
///
/// A run that dies at the comparison step has already spent every extraction
/// call it made — thirty-three minutes of them, on the sweep that prompted
/// this. Discarding that makes the next attempt pay for it again and leaves
/// nobody able to see what the run actually held. The artifact is marked
/// `failed`, so it reads as evidence and can never be scored as a result.
fn abandon(
    dir: &Path,
    artifact: &mut DraftRun,
    stage: &str,
    e: ModelError,
    client: &Client,
) -> i32 {
    // The calls that DID land are the expensive half, and a stage failure is
    // exactly when you want to re-run the code below it without paying again.
    artifact.tape = client.tape();
    artifact.failed = Some(format!("{stage}: {e}"));
    match persist(dir, artifact) {
        Ok(p) => {
            // This artifact supersedes the checkpoint — it holds everything
            // the checkpoint did, plus the stage that ended the run. Keeping
            // both would enter one run twice in the bar's refusal list.
            clear_checkpoint(dir, artifact);
            eprintln!(
                "the {stage} step failed. What ran before it is kept at {}",
                p.display()
            );
        }
        // And here the checkpoint STAYS. It is now the only copy of the work
        // this run paid for.
        Err(w) => eprintln!(
            "the {stage} step failed, and the partial run could not be written: {w} — \
             the last checkpoint is still at {}",
            checkpoint_path(dir, artifact).display()
        ),
    }
    model::report(e)
}

/// The one file a run checkpoints into.
///
/// Keyed by `at` so it belongs to this run and no other, and `.partial` so
/// that neither a reader nor a glob over finished runs mistakes it for one.
fn checkpoint_path(dir: &Path, run: &DraftRun) -> PathBuf {
    dir.join(RUNS_DIR).join(format!("{}.partial.json", run.at))
}

/// Does this recording have a hole where a call should be?
///
/// Extraction makes exactly one call per chunk. A recording with
/// fewer lost one, and because the tape is a QUEUE every call after the hole
/// pops the reply meant for the call before it: wrong citations, wrong
/// candidates, exit 0.
///
/// Measured on the founding checkpoint of 2026-08-26 — 103 calls for 104
/// chunks, chunk 1 having met a backend 503. Replaying it produced 256
/// candidates instead of 342, 88 of them dropped for citing a passage they
/// had nothing to do with, and the only thing that noticed was a stage label
/// mismatching on the very last call. A hole in the LAST chunk would have
/// gone through clean.
///
/// `taped_reads == 0` means the recording predates stage labels and cannot
/// be checked this way; those are left alone rather than refused.
fn tape_hole(target: &str, taped_reads: usize, chunks: usize) -> Option<String> {
    (taped_reads > 0 && taped_reads != chunks).then(|| {
        format!(
            "{target} recorded {taped_reads} extraction call(s) where {chunks} were made, \
             one per chunk. The recording has a hole: every call after it would read the \
             reply meant for the call before. Re-run it live."
        )
    })
}

/// Write what the run holds so far, into one file that replaces itself.
///
/// [`abandon`] covers a stage that ERRORS, which is the failure Rust code
/// gets to see. This covers the one it does not: the process is killed and
/// nothing of ours runs at all. On 2026-08-25 a reboot took a founding run
/// that had already read all 104 chunks and left NOTHING on disk, because the
/// artifact was written only at the end — so those two hours have to be paid
/// again.
///
/// **What it actually saves is the extraction.** With the candidates on disk,
/// `--continue <partial>` runs every stage below extraction without paying
/// the endpoint for it a second time.
///
/// Unlike [`persist`] this OVERWRITES, and the difference is deliberate.
/// `persist` refuses to, because two runs landing in one second are two
/// measurements and one must not eat the other. A checkpoint is the SAME run
/// advancing: what it replaces describes a strictly earlier moment of itself.
///
/// A checkpoint that cannot be written does not end the run — a full disk
/// should not abort two hours of work at stage one — but it says so loudly,
/// because an operator who believes a kill is survivable and finds out
/// afterwards that it was not has been misled by silence (§18.3).
///
/// The marker is set only for the duration of the write. A finished artifact
/// must not carry one, and making that a property of this function rather
/// than a line somebody has to remember at the end is the difference between
/// an invariant and a habit (§7).
fn checkpoint(dir: &Path, artifact: &mut DraftRun, after: &str, client: &Client) {
    artifact.checkpoint = Some(after.to_string());
    artifact.tape = client.tape();
    let path = checkpoint_path(dir, artifact);
    let wrote = serde_json::to_string_pretty(&artifact)
        .map_err(|e| e.to_string())
        .and_then(|body| {
            std::fs::create_dir_all(dir.join(RUNS_DIR)).map_err(|e| e.to_string())?;
            std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))
        });
    match wrote {
        Ok(()) => eprintln!("  checkpoint after {after} -> {}", path.display()),
        Err(e) => eprintln!(
            "WARNING: could not checkpoint after {after} ({e}) — if this run is \
             killed, the calls it has already paid for are lost"
        ),
    }
    artifact.checkpoint = None;
}

/// Remove a run's checkpoint, once it has written the artifact superseding it.
///
/// A `.partial.json` on disk means a run that did not finish. One left behind
/// by a run that DID tells the next reader a lie, and hands the bar a second
/// artifact for a single run.
fn clear_checkpoint(dir: &Path, run: &DraftRun) {
    let path = checkpoint_path(dir, run);
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("WARNING: {} could not be removed: {e}", path.display()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunTension {
    pub a: usize,
    pub b: usize,
    pub reason: String,
}

// ── chunking (no model) ─────────────────────────────────────

/// Split text into chunks at the first boundary past [`CHUNK_TARGET`].
///
/// Format-agnostic by default: a paragraph break is a chunk boundary in any
/// plain text. The heading rule costs nothing on unstructured prose (there
/// are no headings to find) and keeps one article, one decision, or one dated
/// journal entry whole where the text does have structure.
///
/// **A LINE BREAK is a boundary too, and that is what bounds this.** Cutting
/// only on blank lines and headings means text with neither never cuts at
/// all: measured, 500 CSV rows became one chunk of 15,279 characters and 500
/// log lines one chunk of 18,389 — each sent to the model as a single
/// completion, and a 2 MB log would have been one prompt. `CHUNK_TARGET` was
/// a flush threshold with nothing above it.
///
/// So the rule is uniform: a chunk ends at the first boundary of ANY kind —
/// heading, blank line, or line break — once it is big enough to be worth its
/// own completion. A single line longer than the target has no boundary
/// inside it and stays whole, which is honest: the alternative is cutting
/// mid-sentence, and a citation must be a contiguous slice of its passage.
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
            said_at: None,
            // Set by the caller, which is where the coordinate system is
            // built. `chunk_text` takes no view on how a passage is read.
            by_line: false,
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
        // The line break before this line is a boundary, and the chunk is
        // already big enough. Taking it here rather than waiting for a blank
        // line is what keeps a file that has none from being one chunk.
        if cur.trim().chars().count() >= CHUNK_TARGET {
            flush(&mut cur, start, end, &cur_heading);
            cur_heading = heading.clone();
            start = no;
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

/// Annotate chunks after splitting so the citation coordinates still refer
/// to exactly the same original text, regardless of whether it has a clock.
fn chunks_from_source(src: &Source) -> Vec<Chunk> {
    let mut chunks = chunk_text(&src.name, &src.text);
    for chunk in &mut chunks {
        if let Some((_, span)) = chunk.source.rsplit_once(':') {
            if let Some((first, last)) = span.split_once('-') {
                if let (Ok(first), Ok(last)) = (first.parse(), last.parse()) {
                    chunk.said_at = src
                        .time_for_lines(first, last)
                        .map(|(from, to)| SourceTime { from, to });
                }
            }
        }
    }
    chunks
}

// ── map: extract (one completion per chunk) ─────────────────

const EXTRACT_SYSTEM: &str = "\
You read a passage and return what it states.

Most of what a passage states is a COMMITMENT — a rule, a standard, or a \
stated value, something that says how things should be. Some passages state \
no commitment at all and state something else; return that instead.

A passage often records a decision, and a decision that changes a rule \
STATES a rule. Extract the rule it establishes, not the meeting that \
established it: \"Quiet hours begin at 10:00 PM Sunday through Thursday\" is a \
commitment; \"the house met and resolved to move quiet hours earlier\" is not. \
When a passage changes one part of a rule and leaves the rest, the part that \
changed is a commitment.

The passage is given as numbered sentences, one per line. Each is marked \
[n]. The markers are not part of the text.

Most of what you return is a RULE. Three other kinds count, and only when \
the passage says them outright:
- question: the passage says nobody has decided something, or leaves it open. \
Write text as the open question.
- silence: the passage says something is deliberately NOT being written \
down. Write text as the subject of that silence.
- record: the passage states what HAPPENED or why the body acted — a \
grievance, a finding, a recital. Past tense, about a particular actor or \
occasion, with nothing anyone could keep or break. Write text as what it says \
happened.

A subject the passage simply does not mention is not a question. A rule that \
is merely absent is not a silence. A rule about how things must be done from \
now on is not a record, however old the document is.

For each one, return:
- kind: rule, question, silence, or record
- first: the marker of the sentence that states the rule
- last: the marker of the last sentence the rule runs to — the same as first \
when one sentence states it
- text: one self-contained sentence stating the rule as the holder would \
write it in a list of their own commitments. Never write about the author — \
\"Mornings are protected.\", not \"The speaker intends to protect their \
mornings.\" It must make sense with no passage in front of it.
- because: for a silence, what leaving it unwritten protects — a silence \
without this is refused. For a rule or a question, the reason the passage \
gives, or an empty string when it gives none.

Rules:
- Extract every distinct rule, question, silence and record the passage \
states. A passage stating none of the four returns an empty list.
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
    first: crate::model::Pos,
    #[serde(default)]
    last: crate::model::Pos,
    #[serde(default)]
    text: String,
    /// Unreadable or absent reads as `rule`, which is the kind that has to
    /// clear the citation and quantity guards. A silence or a question let
    /// through by a typo would bypass both.
    #[serde(default)]
    kind: String,
    #[serde(default)]
    because: String,
}

/// Written in sorted order, the order this stage was decoded in before
/// [`Schema`] existed and so the order it was measured in. Moving a property
/// is a measured change, not a tidy-up.
///
/// `because` is required so the KEY cannot be omitted. It was described in
/// the silence bullet but missing from the return list, so a model following
/// that list emitted kind/first/last/text and wrote the rationale as a
/// trailing "because" clause inside `text` — and every silence was then
/// refused for having no stated reason. A capability with code, a test and a
/// README section that could not fire.
pub(crate) const EXTRACT_SCHEMA: Schema = Schema::new(
    r#"{
  "type": "object",
  "properties": {
    "commitments": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "because": { "type": "string" },
          "first": { "type": "integer" },
          "kind": { "type": "string", "enum": ["rule", "question", "silence", "record"] },
          "last": { "type": "integer" },
          "text": { "type": "string" }
        },
        "required": ["kind", "first", "last", "text", "because"],
        "additionalProperties": false
      }
    }
  },
  "required": ["commitments"],
  "additionalProperties": false
}"#,
);

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
    let (spans, _) = locate::units(&chunk.text);
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
             cited: \"{h}\"\n\nPassage:\n{shown}\nReturn what this passage \
             states."
        ),
        None => format!("Passage:\n{shown}\nReturn what this passage states."),
    };
    let got: Extracted = client.complete_json(&system, &user, "commitments", &EXTRACT_SCHEMA)?;
    let mut kept = Vec::new();
    let mut dropped = Vec::new();
    let refuse = |text: String, quote: String, reason: String| Dropped {
        text,
        quote,
        chunk: chunk.id,
        reason,
    };
    for c in got.commitments {
        let text = strip_emphasis(c.text.trim());
        if text.is_empty() {
            dropped.push(refuse(text, String::new(), "empty commitment text".into()));
            continue;
        }
        // A citation that could not be cut carries no quote to report, so the
        // reason names the position that was asked for and the ones on offer.
        let (Some(first), Some(last)) = (c.first.get(), c.last.get()) else {
            dropped.push(refuse(
                text,
                String::new(),
                format!("citation [{}-{}] is not a position", c.first, c.last),
            ));
            continue;
        };
        // The kind is resolved FIRST because the citation guard needs it:
        // how wide a citation may be is a property of what is being cited.
        // Anything but the three named words is a rule, which is the kind
        // that has to clear the citation and quantity guards. A silence let
        // through by a typo would bypass both.
        let kind = kind_of(&c.kind);
        // The citation proves the words are the passage's. Whether the RULE
        // matches them is a different reading over different text, and it is
        // its own stage — see `support`.
        let quote = match locate::cite(&chunk.text, &spans, first, last, kind.span_max(spans.len()))
        {
            Ok(q) => q,
            Err(e) => {
                dropped.push(refuse(text, String::new(), e.to_string()));
                continue;
            }
        };
        let because = c.because.trim().to_string();
        // Cite-or-abstain, applied to silence. A deliberate silence with no
        // stated reason cannot be told apart from having forgotten, which is
        // the entire distinction it exists to make — so it is refused here
        // rather than written as one and discovered later.
        if kind == Kind::Silence && because.is_empty() {
            dropped.push(refuse(
                text,
                quote.to_string(),
                "a silence with no stated reason is indistinguishable from a gap".into(),
            ));
            continue;
        }
        kept.push(Candidate {
            text,
            quote,
            chunk: chunk.id,
            source: chunk.source.clone(),
            kind,
            because,
        });
    }
    Ok((kept, dropped))
}

/// Markdown emphasis out of a rule's text, and never out of its quote.
///
/// A model reading `**build_self_manifest must derive…**` copies the stars
/// into the sentence it writes, and the stars are then the first thing a
/// person is asked to accept. The quote keeps them: it is the passage's
/// words, verbatim, and evidence is not tidied. Only the text is cleaned.
///
/// Backtick spans are left whole — a symbol is quoted as written, stars and
/// all. Doubled markers (`**`, `__`) are removed only when they pair up, so
/// a lone `**` is not evidence of emphasis and stays. A single `*` or `_`
/// wrapping the whole sentence is removed; one inside a word is a word.
pub fn strip_emphasis(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    // Count doubled markers outside code first: an odd number is not a
    // pair, and removing half of one is worse than leaving both.
    let mut pairs = std::collections::BTreeMap::from([('*', 0usize), ('_', 0usize)]);
    let mut in_code = false;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            in_code = !in_code;
        } else if !in_code && (c == '*' || c == '_') && chars.get(i + 1) == Some(&c) {
            *pairs.get_mut(&c).unwrap_or(&mut 0) += 1;
            i += 2;
            continue;
        }
        i += 1;
    }
    let strips = |c: char| pairs.get(&c).is_some_and(|n| n % 2 == 0 && *n > 0);

    let mut out = String::with_capacity(text.len());
    in_code = false;
    i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '`' {
            in_code = !in_code;
        } else if !in_code && strips(c) && chars.get(i + 1) == Some(&c) {
            i += 2;
            continue;
        }
        out.push(c);
        i += 1;
    }
    let t = out.trim();
    for m in ['*', '_'] {
        if t.chars().count() > 2 && t.starts_with(m) && t.ends_with(m) {
            return t[m.len_utf8()..t.len() - m.len_utf8()].trim().to_string();
        }
    }
    t.to_string()
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
    /// Rules kept WITHOUT a number check because no reading of them came
    /// back, each naming the rule and why. Kept, since the refusing default
    /// for a missing reading is "states no quantity"; listed, so a reviewer
    /// is not told a rule was checked when it was not.
    pub unchecked: Vec<String>,
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
    // **Only rules are read for quantities.** The guard asks whether a rule
    // states a number its citation does not, and a question or a silence
    // states no number to disagree about — putting them through it would
    // spend a call per candidate to compare two empty lists, and would let a
    // stray reading refuse an open question for a limit nobody claimed.
    let (rules, other): (Vec<Candidate>, Vec<Candidate>) =
        candidates.into_iter().partition(|c| c.kind == Kind::Rule);
    if rules.is_empty() {
        let quantities = vec![Vec::new(); other.len()];
        return Ok(Supported {
            candidates: other,
            quantities,
            dropped: Vec::new(),
            unchecked: Vec::new(),
        });
    }
    // Each rule is read alongside its own citation, in one call, because the
    // canonical form the comparison depends on is only agreed within a call.
    let pairs: Vec<(&str, &str)> = rules
        .iter()
        .map(|c| (c.text.as_str(), c.quote.as_str()))
        .collect();
    let quantified = quantify::quantify_pairs(client, &pairs)?;
    let unchecked: Vec<String> = quantified
        .unread
        .iter()
        .map(|u| format!("{} — {}", rules[u.index].text, u.error))
        .collect();
    let read = quantified.readings;

    let mut kept = Vec::new();
    let mut quantities = Vec::new();
    let mut dropped = Vec::new();
    for (c, (rule, cited)) in rules.into_iter().zip(read) {
        // The rule and its citation travel to the guard alongside their
        // readings: a reading can be wrong, the text cannot.
        match quantify::unsupported(&rule, &cited, &c.text, &c.quote) {
            Some(m) => dropped.push(Dropped {
                text: c.text,
                quote: c.quote.to_string(),
                chunk: c.chunk,
                reason: format!("states `{m}`, which its citation does not"),
            }),
            None => {
                kept.push(c);
                quantities.push(rule);
            }
        }
    }
    // Questions and silences ride along, each with an empty reading, so
    // `candidates` and `quantities` stay the same length and the same order —
    // the reduce step indexes both, and splitting them would judge every
    // later rule against the one before it.
    for c in other {
        kept.push(c);
        quantities.push(Vec::new());
    }
    Ok(Supported {
        candidates: kept,
        quantities,
        dropped,
        unchecked,
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

pub(crate) const DEDUPE_SCHEMA: Schema = Schema::new(
    r#"{
  "type": "object",
  "properties": {
    "groups": {
      "type": "array",
      "items": { "type": "array", "items": { "type": "integer" } }
    }
  },
  "required": ["groups"],
  "additionalProperties": false
}"#,
);

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
    let got: Grouped = client.complete_json(DEDUPE_SYSTEM, &user, "groups", &DEDUPE_SCHEMA)?;

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

/// Finish a run that stopped after extraction, from what it recorded.
///
/// The passages were paid for once. A run that failed in `support` or later,
/// or was killed there, kept its chunks and candidates; this runs the stages
/// after extraction on them as a real run, without reading a passage again.
/// It never reviews: the result is a run on disk, and `--resume` is where a
/// person takes it up.
fn continue_run(
    dir: &Path,
    profile: Profile,
    args: &[String],
    target: &str,
    seen: &mut Seen,
) -> i32 {
    let recorded: DraftRun = match std::fs::read_to_string(target)
        .map_err(|e| format!("reading {target}: {e}"))
        .and_then(|raw| read_run(&raw).map_err(|e| format!("{target}: {e}")))
    {
        Ok(r) => r,
        Err(e) => return crate::cmds::fail(e),
    };
    let client = match model::client_for(dir, crate::cmds::has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    continue_with(dir, profile, recorded, target, client, seen, args)
}

fn continue_with(
    dir: &Path,
    profile: Profile,
    recorded: DraftRun,
    target: &str,
    client: Client,
    seen: &mut Seen,
    args: &[String],
) -> i32 {
    let why = match (&recorded.failed, &recorded.checkpoint) {
        (Some(f), _) => format!("failed in `{}`", f.split(':').next().unwrap_or(f)),
        (None, Some(c)) => format!("killed after `{c}`"),
        (None, None) => {
            return crate::cmds::fail(format!(
                "{target} finished, so there is nothing to continue. `canon draft --resume` reviews it."
            ))
        }
    };
    if recorded.replayed_from.is_some() {
        return crate::cmds::fail(format!(
            "{target} is a replay, which is a measurement; its candidates are not a run to finish"
        ));
    }
    if recorded.candidates.is_empty() {
        return crate::cmds::fail(format!(
            "{target} extracted nothing, so there is nothing to continue"
        ));
    }
    if Profile::parse(&recorded.profile).ok() != Some(profile) {
        return crate::cmds::fail(format!(
            "{target} was drafted in the `{}` voice and this canon is `{}`; continue it where it was drafted",
            recorded.profile,
            profile.as_str()
        ));
    }
    eprintln!(
        "continuing {target} ({why}): {} candidate(s) from {} chunk(s), no passage read again",
        recorded.candidates.len(),
        recorded.chunks.len()
    );
    let pipeline = Pipeline {
        dir,
        profile,
        xclient: client.for_leg(),
        client,
        chunks: recorded.chunks,
        sources: recorded.sources,
        skipped: recorded.skipped,
        already_read: recorded.already_read,
        capped: recorded.capped,
        dry_run: false,
        replayed_from: None,
        extracted: Some(CarriedExtraction {
            candidates: recorded.candidates,
            dropped: recorded.dropped,
            unread: recorded.unread,
        }),
        continued_from: Some(format!("{target} ({why})")),
    };
    execute(pipeline, seen, args)
}

/// Re-run a recorded run's pipeline against its tape — every stage, no model.
///
/// A replay judges PURE CODE over recorded model output: citation cutting,
/// the silence guard, the quantity guard, the fold, the tension rendering. It
/// cannot judge a change to the calls themselves — a different prompt, a
/// different schema, a different chunking, an extra pass — and it does not
/// pretend to: the tape checks each call's path and refuses when the sequence
/// diverges rather than answering from the wrong recording (§18.3). "Re-run
/// it live" is the honest answer there.
///
/// Chunks come from the artifact, not from the document. A replay measures the
/// run that was recorded, and re-reading the sources would let the file change
/// underneath the evidence.
fn replay(dir: &Path, args: &[String], target: &str) -> i32 {
    let raw = match std::fs::read_to_string(target) {
        Ok(r) => r,
        Err(e) => return crate::cmds::fail(format!("reading {target}: {e}")),
    };
    let recorded = match read_run(&raw) {
        Ok(r) => r,
        Err(e) => return crate::cmds::fail(format!("{target}: {e}")),
    };
    if recorded.tape.is_empty() {
        return crate::cmds::fail(format!(
            "{target} carries no tape — it was recorded before runs kept their \
             replies, or it is not a dry run. Only a `--dry-run` artifact can be \
             replayed."
        ));
    }
    let profile = match Profile::parse(&recorded.profile) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(format!("{target}: profile: {e}")),
    };
    let calls = recorded.tape.len();

    // A tape with a HOLE replays silently wrong, and this is the cheap check
    // that catches one. Extraction makes exactly one call per chunk; a
    // recording with fewer lost a call, and every call after the hole pops
    // the reply meant for the one before it — wrong citations, wrong
    // candidates, exit 0. Recordings made before calls carried stage labels
    // cannot be checked this way and are left alone.
    let taped_reads = recorded
        .tape
        .iter()
        .filter(|e| e.stage == "commitments")
        .count();
    if let Some(why) = tape_hole(target, taped_reads, recorded.chunks.len()) {
        return crate::cmds::fail(why);
    }

    eprintln!(
        "replaying {} chunk(s) and {calls} recorded call(s) from {target} — no model",
        recorded.chunks.len()
    );
    let client = Client::replaying(&recorded.endpoint, &recorded.model, recorded.tape);
    let pipeline = Pipeline {
        dir,
        profile,
        // One tape, one order. The extract leg shares the client rather than
        // taking its own, because two readers of one tape would interleave.
        xclient: client.for_leg(),
        client,
        chunks: recorded.chunks,
        sources: recorded.sources,
        skipped: recorded.skipped,
        already_read: recorded.already_read,
        capped: recorded.capped,
        // A replay is a measurement and writes nothing to the canon.
        dry_run: true,
        replayed_from: Some(format!("{target}, whole")),
        extracted: None,
        continued_from: None,
    };
    // A replay neither consults nor updates what this canon has already read:
    // it is re-scoring recorded evidence, not reading a feed.
    let mut seen = Seen::preview(dir);
    execute(pipeline, &mut seen, args)
}

fn read_sources(args: &[String]) -> Result<Gathered, String> {
    let mut got = Gathered::default();
    let include_ignored = crate::cmds::has(args, "--include-ignored");
    if crate::cmds::has(args, "--from-git") {
        let since = crate::cmds::flag(args, "--since").unwrap_or("1y");
        for (name, text, at) in read_git(since)? {
            let mut source = Source::unplaced(name, text);
            source.said_at = Some(at);
            got.sources.push(source);
        }
    }
    let paths = from_paths(args);
    // `--from` with nothing after it is its own mistake, and the general
    // "nothing to draft from" answers it by printing the command that was
    // just run.
    if crate::cmds::has(args, "--from") && paths.is_empty() {
        return Err(
            "`--from` needs at least one path — a file, a folder, or `-` for stdin".to_string(),
        );
    }
    for p in &paths {
        if p == "-" {
            got.sources.push(read_stdin(args)?);
            continue;
        }
        sources::gather(Path::new(p), &mut got, include_ignored)?;
    }
    if got.sources.is_empty() {
        return Err(nothing_read(&paths, &got));
    }
    // Two sources cannot answer to the same citation. Resolved once, here,
    // because a collision is a property of the SET rather than of any root.
    got.resolve_names();
    Ok(got)
}

/// Why a run that was pointed somewhere came back with nothing.
///
/// **One sentence used to answer three different questions**, and two of them
/// it answered by printing the command the person had just run: pointing at an
/// empty folder said "nothing to draft from — `canon draft --from <paths>`".
/// A folder that holds no writing, a folder whose every file was filtered, and
/// a run pointed nowhere at all are different mistakes with different ways out.
fn nothing_read(paths: &[String], got: &Gathered) -> String {
    if paths.is_empty() {
        return "nothing to draft from — `canon draft --from <paths>`, `--from -` for stdin, \
                or `--from-git --since 1y`"
            .to_string();
    }
    let where_at = paths.join(", ");
    match got.skipped_note() {
        // Every file under there was passed over, and the note says by what.
        Some(note) => format!("nothing readable in {where_at}\n{note}"),
        // It exists, it was walked, and there was nothing in it.
        None => format!(
            "nothing readable in {where_at} — the walk found no files there. \
             Hidden folders are passed over; `canon draft --from <file>` reads one by name."
        ),
    }
}

/// Files were read, and none of them held a passage worth citing.
///
/// **This is the chunker, not the reader.** Saying only "nothing readable in
/// those sources" sent a person back to a folder that was never the problem —
/// a list of errands is readable, it just has nothing in it long enough to
/// check a rule against.
fn nothing_long_enough(sources: usize) -> String {
    format!(
        "{sources} source(s) read, but no passage in them reached the {CHUNK_MIN} characters \
         a citation needs to be checkable against."
    )
}

/// More passages than anybody means to pay for without being asked.
///
/// Extraction is one completion per passage, so a folder is a bill. The
/// failure this prevents is pointing at `~/Documents`, walking away, and
/// finding out afterwards.
const CONFIRM_ABOVE: usize = 200;

/// Whether this run is one to ask about, kept apart from the asking so the
/// rule can be tested without a terminal.
///
/// **`interactive` is a precondition, not a convenience.** A pipeline, an
/// agent on the MCP surface, and `--from -` — which has already eaten stdin —
/// all reach here with nothing able to answer, and a prompt nobody can answer
/// is a hang rather than a safeguard.
fn needs_asking(args: &[String], passages: usize, interactive: bool) -> bool {
    passages > CONFIRM_ABOVE && interactive && !crate::cmds::has(args, "--yes")
}

/// Say what a large run will cost, and on a terminal ask before spending it.
fn confirmed(args: &[String], sources: usize, passages: usize) -> bool {
    if passages <= CONFIRM_ABOVE {
        return true;
    }
    // Said either way. The count is the thing a person needed before the run,
    // whether or not there is anybody there to answer for it.
    eprintln!("{sources} source(s) read, {passages} passage(s) — one model call each.");
    !needs_asking(args, passages, crate::term::interactive())
        || crate::term::yes("That is a large run. Proceed?")
}

/// `--from` takes every following argument until the next flag, because a
/// shell expands `--from ~/notes/**/*.md` into many arguments.
fn from_paths(args: &[String]) -> Vec<String> {
    let Some(i) = args.iter().position(|a| a == "--from") else {
        return Vec::new();
    };
    args[i + 1..]
        .iter()
        // A bare `-` is a path here, not a flag: it means stdin. Without
        // this it is taken for the next option and `--from -` reads nothing.
        .take_while(|a| a.as_str() == "-" || !a.starts_with('-'))
        .cloned()
        .collect()
}

/// Whatever was piped in, as one source.
///
/// **This is the whole integration surface, and it is deliberately not an
/// API.** `cat anything | canon draft --from - --json` connects canon to any
/// system that can emit text, which is all of them, and costs this tool no
/// connector, no vendor schema and no endpoint of its own. A Slack
/// integration would have supported Slack; this supports whatever the person
/// already has an agent for, including the systems nobody has heard of yet.
///
/// `--as` names the source, so a citation reads `#eng-decisions:3-4` rather
/// than `stdin:3-4` — which matters most on a feed, where the passage is
/// gone by the time anyone reads the candidate.
fn read_stdin(args: &[String]) -> Result<Source, String> {
    let mut text = String::new();
    std::io::stdin()
        .read_to_string(&mut text)
        .map_err(|e| format!("reading stdin: {e}"))?;
    if text.trim().is_empty() {
        return Err("nothing arrived on stdin".to_string());
    }
    // Sniffed exactly the way a file is, so piping a chat export works
    // without anyone having to declare that it is one.
    let head = text.trim_start();
    if head.starts_with('{') || head.starts_with('[') {
        if let Some((rendered, line_times)) = sources::render_chat_timed(&text) {
            text = rendered;
            let mut source =
                Source::unplaced(crate::cmds::flag(args, "--as").unwrap_or("stdin"), text);
            source.line_times = line_times;
            return Ok(source);
        }
    }
    Ok(Source::unplaced(
        crate::cmds::flag(args, "--as").unwrap_or("stdin"),
        text,
    ))
}

/// Commit bodies as source text. Extends `store::actor`'s shell-out pattern
/// rather than taking a git dependency.
fn read_git(since: &str) -> Result<Vec<(String, String, i64)>, String> {
    let out = std::process::Command::new("git")
        .args([
            "log",
            &format!("--since={since}"),
            "--format=%H%x1f%ct%x1f%B%x1e",
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
            let (sha, rest) = record.trim_start().split_once('\u{1f}')?;
            let (at, body) = rest.split_once('\u{1f}')?;
            let at = at.parse::<i64>().ok()?;
            let body = body.trim();
            (!body.is_empty()).then(|| {
                (
                    format!("git:{}", &sha[..sha.len().min(12)]),
                    body.into(),
                    at,
                )
            })
        })
        .collect())
}

// ── the verb ────────────────────────────────────────────────

/// Finish a review that was started and quit.
///
/// **The reason there is no `--accept-all` needs a way to be survivable.**
/// Accepting one at a time is what makes onboarding the first governance
/// session rather than disengagement at t=0 — but a folder of documents
/// yields dozens of candidates, and "you must finish in one sitting or lose
/// your place" is how a person quits at candidate nine and never comes back.
///
/// The run artifact already holds every candidate and every citation, so this
/// costs NO model call. Anything already in the canon is skipped, so resuming
/// twice cannot write a thing twice.
///
/// **Every run with something left, oldest first**, not the newest run
/// alone. An import large enough to split across runs used to strand every
/// run but the last, candidates and all, because resume took the file that
/// sorted last. Naming one run picks it: `canon draft --resume <file>`.
/// `[q]uit` ends the sitting, not the run — the next run would otherwise
/// open under the person's hands the moment they said stop.
fn resume(dir: &Path, profile: Profile, seen: &mut Seen, pick: Option<&str>) -> i32 {
    let (found, partial) = match runs_to_resume(dir, pick) {
        Ok(v) => v,
        Err(e) => return crate::cmds::fail(e),
    };
    // Said, not skipped in silence: the person who ran `--resume` while an
    // import was drafting is the person who needs to know it is still going.
    for p in &partial {
        eprintln!(
            "a run is still being drafted and is not offered: {}",
            p.file_name().unwrap_or_default().to_string_lossy()
        );
    }
    if found.is_empty() {
        return crate::cmds::fail("no draft runs to resume — `canon draft --from <paths>`");
    }
    let mut accepted = 0usize;
    let mut opened = 0usize;
    for path in &found {
        let raw = match std::fs::read_to_string(path) {
            Ok(r) => r,
            Err(e) => return crate::cmds::fail(format!("reading {}: {e}", path.display())),
        };
        let run = match read_run(&raw) {
            Ok(r) => r,
            Err(e) => return crate::cmds::fail(format!("{}: {e}", path.display())),
        };
        // Re-read per run: what the last run's review wrote is already in
        // the canon, and the next run must not offer it again.
        let Ok(canon) = store::read(dir).map(|l| l.derive()) else {
            return crate::cmds::fail("cannot read this canon");
        };
        // Already in the canon, by its own words. Text is what a person
        // edited and what they will recognise; an id would not survive the
        // `[e]dit` path that rewrites the text before it is written.
        let remaining = remaining(&run, &canon, seen);
        if remaining.is_empty() {
            continue;
        }
        opened += 1;
        println!(
            "resuming {} — {} of {} left, no model call",
            path.file_name().unwrap_or_default().to_string_lossy(),
            remaining.len(),
            run.kept.len()
        );
        match review(dir, &run.candidates, &run.chunks, &remaining, seen) {
            Ok((a, quit)) => {
                accepted += a.len();
                if quit {
                    break;
                }
            }
            Err(e) => return crate::cmds::fail(e),
        }
    }
    if opened == 0 {
        println!("nothing left to review.");
    } else if accepted == 0 {
        println!("nothing accepted.");
    } else {
        println!("\n{} accepted.", profile.count(accepted));
    }
    0
}

/// The finished runs to offer, oldest first, and the checkpoints that are
/// not offered.
///
/// A `.partial.json` is a run still being drafted, or one that died. Its
/// extension is `json` too, and a run started later sorts later, so while
/// an eighty-minute import ran `--resume` opened its half-written checkpoint
/// and hid the finished run beside it. A checkpoint has no `kept` to review;
/// `--continue` is the way back into one.
fn runs_to_resume(dir: &Path, pick: Option<&str>) -> Result<(Vec<PathBuf>, Vec<PathBuf>), String> {
    let runs = dir.join(RUNS_DIR);
    let all: Vec<PathBuf> = match std::fs::read_dir(&runs) {
        Ok(rd) => rd
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "json"))
            .collect(),
        Err(e) => return Err(format!("no draft runs to resume ({e})")),
    };
    let (mut partial, mut found): (Vec<PathBuf>, Vec<PathBuf>) = all
        .into_iter()
        .partition(|p| p.to_string_lossy().ends_with(".partial.json"));
    found.sort();
    partial.sort();
    if let Some(name) = pick {
        // By file name, by the timestamp alone, or by any tail of the path.
        let stem = format!("{name}.json");
        found.retain(|p| {
            p.file_name().is_some_and(|f| f == name || *f == *stem)
                || p.to_string_lossy().ends_with(name)
        });
        if found.is_empty() {
            return Err(format!(
                "no draft run named `{name}` — they are in {}",
                runs.display()
            ));
        }
    }
    Ok((found, partial))
}

/// Candidates from a run that are neither already in the canon nor already
/// declined.
///
/// Matched **by text**, because text is what a person recognises and what
/// they may have rewritten on the `[e]dit` path before it was written. An id
/// would not survive that edit, so resuming would offer the same candidate
/// again in the words the model chose, which is the one wording the person
/// has already rejected.
///
/// The canon answers for what was accepted. Only [`Seen`] answers for what
/// was turned down — without it, resuming re-asks every rejection.
fn remaining(run: &DraftRun, canon: &canon_core::Canon, seen: &Seen) -> Vec<usize> {
    let already: std::collections::BTreeSet<&str> = canon
        .active()
        .map(|c| c.text())
        .chain(canon.open().map(|q| q.text.as_str()))
        .chain(canon.silences().iter().map(|s| s.about.as_str()))
        .collect();
    run.kept
        .iter()
        .copied()
        .filter(|i| {
            run.candidates
                .get(*i)
                .is_some_and(|c| !already.contains(c.text.as_str()) && !seen.was_rejected(&c.text))
        })
        .collect()
}

pub fn run(args: &[String]) -> i32 {
    if crate::cmds::has(args, "--accept-all") {
        eprintln!("error: there is no --accept-all, on purpose.");
        eprintln!("  A canon adopted wholesale is disengagement at t=0. Accepting one at a");
        eprintln!("  time is what makes onboarding the first governance session.");
        return 2;
    }
    let dry_run = crate::cmds::has(args, "--dry-run");
    // **Refused before anything is spent.** The review loop reads its
    // answers from stdin and so does `--from -`, so the two together read
    // the document, pay for every extraction call, then take end-of-input as
    // "quit" and accept nothing. The person gets a bill and an empty canon.
    // There is no coherent non-dry stdin flow to allow instead: accepting is
    // one at a time on purpose, and there is no --accept-all.
    if !dry_run && from_paths(args).iter().any(|p| p == "-") {
        eprintln!("error: `--from -` reads the document from stdin, and so does the review.");
        eprintln!("  Use `--dry-run --json` to see what a pipe produces, then");
        eprintln!("  `canon draft --resume` to review it without a second model run.");
        return 2;
    }

    let dir = match crate::cmds::dir() {
        Ok(d) => d,
        Err(e) => return crate::cmds::fail(e),
    };
    let profile = match Profile::load(&dir) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(e),
    };
    // A dry run READS the set — a preview should not re-offer what you
    // already declined — and writes nothing to it, or you would preview a
    // folder, decide to keep three, run it for real and be told there is
    // nothing there.
    let mut seen = if dry_run {
        Seen::preview(&dir)
    } else {
        Seen::load(&dir)
    };
    if crate::cmds::has(args, "--resume") {
        let pick = crate::cmds::positionals(args).first().copied();
        return resume(&dir, profile, &mut seen, pick);
    }
    if let Some(target) = crate::cmds::flag(args, "--replay") {
        return replay(&dir, args, target);
    }
    if let Some(target) = crate::cmds::flag(args, "--continue") {
        if dry_run {
            return crate::cmds::fail(
                "`--continue` finishes a real run; a dry run has nothing to finish".to_string(),
            );
        }
        return continue_run(&dir, profile, args, target, &mut seen);
    }
    let gathered = match read_sources(args) {
        Ok(s) => s,
        Err(e) => return crate::cmds::fail(e),
    };
    // **Before the model run, not after.** What was skipped changes whether
    // this run is worth paying for, and a person who finds out afterwards
    // has already spent the time.
    if let Some(note) = gathered.skipped_note() {
        eprintln!("{note}");
    }
    let sources = &gathered.sources;

    let mut chunks: Vec<Chunk> = Vec::new();
    for src in sources {
        chunks.extend(chunks_from_source(src));
    }
    let found = chunks.len();
    // **What makes pointing at a growing feed affordable.** Extraction is one
    // completion per passage, so re-reading a channel every morning costs the
    // whole channel every morning unless the passages already read are
    // dropped here.
    chunks.retain(|c| !seen.was_read(&c.text));
    let already_read = found - chunks.len();
    if chunks.is_empty() {
        if found == 0 {
            // Files WERE read — this is the chunker, not the reader, and
            // saying only "nothing readable" sends a person back to a folder
            // that was never the problem.
            return crate::cmds::fail(nothing_long_enough(sources.len()));
        }
        // Nothing new is a RESULT, not an error: an agent polling a feed
        // gets this most of the time, and it has to be cheap and quiet. Same
        // artifact shape either way, so a caller parses one schema (§10.6).
        // No candidates, so no quotes: `String` is the quote type with a
        // `Default`, and it serialises the same.
        let quiet = DraftRun::<String> {
            schema: RUN_SCHEMA.into(),
            at: store::now(),
            profile: profile.as_str().to_string(),
            sources: sources.iter().map(|s| s.name.clone()).collect(),
            skipped: gathered.skipped.clone(),
            already_read: found,
            ..Default::default()
        };
        if crate::cmds::has(args, "--json") {
            println!(
                "{}",
                serde_json::to_string_pretty(&quiet).unwrap_or_default()
            );
        } else {
            println!("nothing new — all {found} passage(s) have been read before.");
        }
        return 0;
    }
    if already_read > 0 {
        eprintln!("{already_read} passage(s) already read — skipping");
    }
    // Loud AND recorded: a capped run is a run about a fraction of the
    // sources, and a cap the reader cannot see reads as coverage (§18.5).
    let capped = match crate::cmds::flag(args, "--max-chunks").map(str::parse::<usize>) {
        Some(Err(e)) => return crate::cmds::fail(format!("--max-chunks: {e}")),
        Some(Ok(n)) if chunks.len() > n => {
            let dropped = chunks.len() - n;
            chunks.truncate(n);
            eprintln!("--max-chunks {n}: {dropped} passage(s) left for a later run");
            dropped
        }
        _ => 0,
    };
    for (i, c) in chunks.iter_mut().enumerate() {
        c.id = i;
        c.by_line = locate::units(&c.text).1 == locate::Basis::Lines;
    }
    // Said out loud, because it changes what every citation from this run
    // points at. Silence is how the collapse this replaces went unnoticed.
    let by_line = chunks.iter().filter(|c| c.by_line).count();
    if by_line > 0 {
        eprintln!(
            "{by_line} of {} passage(s) are line-oriented — cited by line, not by sentence",
            chunks.len()
        );
    }

    // **Before the endpoint, because after it the money is spent.** A folder
    // is a bill: one completion per passage, and the person who pointed at
    // their home directory has no way to know that until it is running.
    if !confirmed(args, sources.len(), chunks.len()) {
        eprintln!(
            "nothing read. `--max-chunks <n>` reads a slice first; `--yes` skips this prompt."
        );
        return 0;
    }

    let client = match model::client_for(&dir, crate::cmds::has(args, "--allow-remote")) {
        // A dry run is a MEASUREMENT, so it keeps its evidence. A real run is
        // somebody's canon and records nothing.
        Ok(c) if dry_run => c.recording(),
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!(
        "{} chunk(s) from {} source(s) on {}",
        chunks.len(),
        sources.len(),
        client.describe()
    );

    // The extract leg may be pointed at its own slot. Everything else on this
    // run keeps the client `client_for` acquired, so a per-leg routing choice
    // cannot reach the locality rule.
    let xclient = match model::extract_client(&dir, &client) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    if xclient.model() != client.model() {
        eprintln!(
            "extracting on {} (rest of the run: {})",
            xclient.model(),
            client.model()
        );
    }

    let pipeline = Pipeline {
        dir: &dir,
        profile,
        client,
        xclient,
        chunks,
        sources: sources.iter().map(|s| s.name.clone()).collect(),
        skipped: gathered.skipped.clone(),
        already_read,
        capped,
        dry_run,
        replayed_from: None,
        extracted: None,
        continued_from: None,
    };
    execute(pipeline, &mut seen, args)
}

/// Everything a pipeline run needs, however it was assembled.
///
/// Two front doors build one of these and hand it to [`execute`]: `run` reads
/// documents and talks to a server, `replay` reads an artifact and talks to a
/// tape. One body downstream, so a replayed number and a live number come from
/// the same code and not from a second implementation of it (§10.6).
struct Pipeline<'a> {
    dir: &'a Path,
    profile: Profile,
    /// Everything but extraction.
    client: Client,
    /// Extraction, which may be pointed at its own slot.
    xclient: Client,
    chunks: Vec<Chunk>,
    sources: Vec<String>,
    skipped: std::collections::BTreeMap<String, usize>,
    already_read: usize,
    capped: usize,
    dry_run: bool,
    /// Set when the run came off a recording.
    replayed_from: Option<String>,
    /// Extraction an earlier run already did. When set, no passage is read and
    /// the pipeline starts at `support`.
    extracted: Option<CarriedExtraction>,
    /// The run that extraction came from, for the artifact.
    continued_from: Option<String>,
}

/// What an extraction produced, carried over from a recorded run.
struct CarriedExtraction {
    candidates: Vec<Candidate>,
    dropped: Vec<Dropped>,
    unread: Vec<Unread>,
}

fn execute(r: Pipeline, seen: &mut Seen, args: &[String]) -> i32 {
    let Pipeline {
        dir,
        profile,
        client,
        xclient,
        chunks,
        sources,
        skipped,
        already_read,
        capped,
        dry_run,
        replayed_from,
        extracted,
        continued_from,
    } = r;
    // A continued run arrives with its extraction done. Reading a passage
    // again would pay for it twice.
    let continuing = extracted.is_some();
    let (mut candidates, mut dropped, mut unread) = match extracted {
        Some(x) => (x.candidates, x.dropped, x.unread),
        None => (Vec::new(), Vec::new(), Vec::new()),
    };
    for chunk in chunks.iter().filter(|_| !continuing) {
        crate::term::progress(&format!("extracting {}/{}…", chunk.id + 1, chunks.len()));
        match extract(&xclient, chunk, profile) {
            Ok((k, d)) => {
                candidates.extend(k);
                dropped.extend(d);
                // Only on an answer. A chunk that errored must stay unseen,
                // or one bad reply blinds this canon to that passage for
                // good.
                if let Err(e) = seen.record(&chunk.text, Why::Read) {
                    eprintln!("\nwarning: {e}");
                }
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
    let unread_chunks: std::collections::BTreeSet<usize> = unread.iter().map(|u| u.chunk).collect();
    if unread_chunks.len() == chunks.len() {
        eprintln!("\nno chunk produced an answer.");
        return 3;
    }
    crate::term::done(&format!(
        "{} candidate(s), {} dropped for a bad citation",
        candidates.len(),
        dropped.len()
    ));

    // From here the artifact exists and every exit writes it. A stage that
    // fails costs its own work and nothing before it.
    let mut artifact = DraftRun {
        schema: RUN_SCHEMA.into(),
        at: store::now(),
        endpoint: client.endpoint().to_string(),
        model: client.model().to_string(),
        served_model: client.served_model(),
        profile: profile.as_str().to_string(),
        sources: sources.clone(),
        // Recorded so a re-score can tell "the extractor found nothing there"
        // from "nothing there was ever opened" (§18.3).
        skipped: skipped.clone(),
        already_read,
        capped,
        chunks: chunks.clone(),
        candidates: candidates.clone(),
        dropped: dropped.clone(),
        unread: unread.clone(),
        duplicates: Vec::new(),
        kept: Vec::new(),
        tensions: Vec::new(),
        tension_passes: 0,
        tension_passes_unread: Vec::new(),
        tension_schedule: None,
        failed: None,
        stopped_after: None,
        checkpoint: None,
        replayed_from,
        continued_from: continued_from.clone(),
        support_unchecked: Vec::new(),
        tape: Vec::new(),
    };

    // Extraction is the long pole — 104 chunks at ~42s is over an hour on the
    // founding corpus — and until this line it lived only in memory.
    checkpoint(dir, &mut artifact, "extract", &client);

    // Every rule read once, checked against its own citation, and the reading
    // carried forward to the fold guard.
    let supported = match support(&client, candidates) {
        Ok(v) => v,
        Err(e) => return abandon(dir, &mut artifact, "support", e, &client),
    };
    if !supported.dropped.is_empty() {
        eprintln!(
            "{} candidate(s) dropped for stating a number their citation does not",
            supported.dropped.len()
        );
    }
    if !supported.unchecked.is_empty() {
        eprintln!(
            "{} rule(s) kept without a number check — no reading of them came back",
            supported.unchecked.len()
        );
    }
    artifact.support_unchecked = supported.unchecked;
    dropped.extend(supported.dropped);
    let (candidates, quantities) = (supported.candidates, supported.quantities);
    artifact.candidates = candidates.clone();
    artifact.dropped = dropped.clone();
    checkpoint(dir, &mut artifact, "support", &client);
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
        Err(e) => return abandon(dir, &mut artifact, "dedupe", e, &client),
    };
    artifact.duplicates = groups.clone();
    artifact.kept = kept.clone();
    // The last one before the comparison stage, which on a large canon is
    // hundreds of passes and hours of wall clock.
    checkpoint(dir, &mut artifact, "dedupe", &client);
    if !groups.is_empty() {
        eprintln!("{} duplicate group(s) folded", groups.len());
    }

    // Tensions compare RULES. Two open questions do not contradict each
    // other, and a silence contradicts nothing by construction — it is the
    // absence of a rule, held on purpose.
    let kept_texts: Vec<&str> = kept
        .iter()
        .filter(|i| candidates[**i].kind.bears())
        .map(|i| candidates[*i].text.as_str())
        .collect();
    // In a dry run nothing is accepted, so tensions runs over every surviving
    // candidate — that is what the bar scores. In a real run it runs over what
    // the person accepted, below.
    let compared = if dry_run {
        match tensions::detect_over(&client, &kept_texts) {
            Ok(v) => v,
            Err(e) => return abandon(dir, &mut artifact, "tensions", e, &client),
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
    artifact.tension_schedule = Some(compared.schedule);
    artifact.tape = client.tape();
    let path = match persist(dir, &artifact) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(e),
    };
    clear_checkpoint(dir, &artifact);

    if dry_run {
        if crate::cmds::has(args, "--json") {
            println!(
                "{}",
                serde_json::to_string_pretty(&artifact).unwrap_or_default()
            );
        } else {
            for i in &kept {
                let c = &candidates[*i];
                println!(
                    "{}",
                    crate::wrap::hang(&format!("  {:<9} ", c.kind.label()), &c.text)
                );
                if !c.because.is_empty() {
                    println!("{}", crate::wrap::hang("            because: ", &c.because));
                }
                println!("            {}", c.source);
            }
            // Counted by kind, because "12 candidates" hides whether this run
            // found a body of rules or twelve open questions.
            let n = |k: Kind| kept.iter().filter(|i| candidates[**i].kind == k).count();
            println!(
                "\n{} rule(s), {} question(s), {} silence(s), {} record(s), \
                 {} tension(s) proposed. Nothing written.",
                n(Kind::Rule),
                n(Kind::Question),
                n(Kind::Silence),
                n(Kind::Record),
                artifact.tensions.len()
            );
            println!("run recorded at {}", path.display());
        }
        return 0;
    }
    eprintln!("run recorded at {}", path.display());
    // A continued run is finished, not reviewed. Review is a person's act, and
    // a run being rescued is often being rescued by a job nobody is sitting at.
    if continued_from.is_some() {
        eprintln!("review it with `canon draft --resume`");
        return 0;
    }

    // ── one at a time ───────────────────────────────────────
    let accepted = match review(dir, &candidates, &chunks, &kept, seen).map(|(a, _)| a) {
        Ok(a) => a,
        Err(e) => return crate::cmds::fail(e),
    };
    if accepted.is_empty() {
        println!("nothing accepted.");
        return 0;
    }
    println!("\n{} accepted.", profile.count(accepted.len()));

    // ── the moment it has to produce ────────────────────────
    let Ok(canon) = store::read(dir).map(|l| l.derive()) else {
        return 0;
    };
    let fresh: Vec<&canon_core::Commitment> = canon
        .active()
        .filter(|c| accepted.contains(c.id()))
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

/// The extra context must come from the same recorded chunk as the quotation.
/// An older or altered run without that chunk cannot silently borrow another
/// passage merely because it has the same words.
fn recorded_context<'a>(candidate: &Candidate, chunks: &'a [Chunk]) -> Option<&'a Chunk> {
    chunks.get(candidate.chunk).filter(|chunk| {
        chunk.id == candidate.chunk
            && chunk.source == candidate.source
            && !candidate.quote.is_empty()
            && chunk.text.contains(candidate.quote.as_str())
    })
}

fn render_recorded_context(candidate: &Candidate, chunks: &[Chunk]) -> String {
    let Some(chunk) = recorded_context(candidate, chunks) else {
        return "  recorded passage unavailable for this citation\n".into();
    };
    let mut out = format!("  recorded passage — {}:\n", chunk.source);
    if let Some(title) = &chunk.heading {
        out.push_str(&format!("    section title, for context only: {title}\n"));
    }
    for line in chunk.text.lines() {
        out.push_str(&format!("    {line}\n"));
    }
    out
}

/// Interactive review. `[a]ccept [e]dit [c]ontext [r]eject [s]kip [q]uit`, one at a
/// time, no bulk verb.
///
/// Returns what was accepted and whether the person quit — which ends the
/// sitting for a caller with more than one run to offer.
fn review(
    dir: &Path,
    candidates: &[Candidate],
    chunks: &[Chunk],
    kept: &[usize],
    seen: &mut Seen,
) -> Result<(Vec<canon_core::ActId>, bool), String> {
    let stdin = std::io::stdin();
    let mut lines = stdin.lock().lines();
    let mut accepted = Vec::new();
    let mut quit = false;
    // A record is not offered. It says what happened, and there is no act
    // that means "the canon now holds that George III refused his Assent" —
    // asking a person to accept or reject one is asking them to rule on the
    // past. They are reported in the dry run and in the artifact, where they
    // are evidence about the document, and they stop there.
    let offered: Vec<usize> = kept
        .iter()
        .copied()
        .filter(|i| candidates[*i].kind != Kind::Record)
        .collect();
    let held = kept.len() - offered.len();
    if held > 0 {
        println!(
            "{held} record(s) of what happened are not offered — nothing in one can be kept or broken."
        );
    }
    enum Choice {
        Text(String),
        Skip,
        Quit,
    }
    for (n, i) in offered.iter().enumerate() {
        let c = &candidates[*i];
        println!("\nCandidate {} of {}", n + 1, offered.len());
        // The kind is shown because accepting writes a different act for each
        // one, and "accept" has to mean what the person thought it meant.
        println!(
            "{}",
            crate::wrap::hang(
                &format!("  {:<9} \"", c.kind.label()),
                &format!("{}\"", c.text)
            )
        );
        if !c.because.is_empty() {
            println!("{}", crate::wrap::hang("        because: ", &c.because));
        }
        println!();
        println!("  from {}:", c.source);
        for l in c.quote.lines() {
            println!("    {l}");
        }
        let answer = loop {
            print!("\n  [a]ccept  [e]dit  [c]ontext  [r]eject  [s]kip  [q]uit: ");
            let _ = std::io::stdout().flush();
            let Some(Ok(answer)) = lines.next() else {
                println!("\n(end of input)");
                break Choice::Quit;
            };
            // Piped input echoes nothing, so the prompt and the reply would
            // run together in a transcript. A terminal supplies this newline.
            println!();
            match answer.trim() {
                "c" | "context" => {
                    print!("{}", render_recorded_context(c, chunks));
                    continue;
                }
                "a" | "accept" => break Choice::Text(c.text.clone()),
                "e" | "edit" => {
                    print!("  text: ");
                    let _ = std::io::stdout().flush();
                    break match lines.next() {
                        Some(Ok(t)) if !t.trim().is_empty() => Choice::Text(t.trim().to_string()),
                        _ => {
                            println!("  (nothing entered — skipped)");
                            Choice::Skip
                        }
                    };
                }
                "r" | "reject" => {
                    // Only rejection, not skip, records a declined candidate.
                    if let Err(e) = seen.record(&c.text, Why::Rejected) {
                        eprintln!("  warning: {e}");
                    }
                    break Choice::Skip;
                }
                "q" | "quit" => break Choice::Quit,
                _ => break Choice::Skip,
            }
        };
        let text = match answer {
            Choice::Text(text) => text,
            Choice::Skip => continue,
            Choice::Quit => {
                quit = true;
                break;
            }
        };
        let kind = match c.kind {
            Kind::Rule => ActKind::Assert {
                text,
                from: None,
                source: Some(c.source.clone()),
            },
            Kind::Question => ActKind::Question {
                text,
                proposal: None,
            },
            Kind::Silence => ActKind::Silence {
                about: text,
                rationale: c.because.clone(),
            },
            // Filtered out above. Unreachable rather than mapped to an act,
            // because inventing one here is how a kind that cannot be
            // committed to quietly becomes a commitment.
            Kind::Record => unreachable!("records are not offered for review"),
        };
        let act = crate::cmds::write(dir, kind)?;
        println!("  {}", act.id);
        accepted.push(act.id);
    }
    Ok((accepted, quit))
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
