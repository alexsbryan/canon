// SPDX-License-Identifier: AGPL-3.0-or-later
//! What `canon draft` was pointed at, and what it could not read.
//!
//! **Nobody writes a canon one rule at a time.** The normative content
//! already exists — a handbook, two years of meeting notes, the channel where
//! things actually get decided — and onboarding is pointing at that folder.
//! Which makes the reader the first thing a new user's trust rests on.
//!
//! **There is no format list, and that is the whole design.** An earlier
//! version read `.md`, `.txt` and JSON chat exports and passed over the rest,
//! which meant the tool worked on the corpora we happened to test it against
//! and quietly ignored everyone else's. A canon lives in whatever its group
//! already writes in: `.org`, `.rst`, `.eml`, a `NOTES` file with no
//! extension, a transcript pasted into a `.log`. So readability is decided by
//! the BYTES. Text is read. Anything that is not valid UTF-8 is skipped and
//! reported.
//!
//! Three things a walk still passes over, each reported and each with a way
//! round it:
//!
//! - **What the project itself calls generated** — `git check-ignore`, so the
//!   authority is the person's own `.gitignore` and not a list of build
//!   directories we guessed at. Without it, pointing at any checked-out repo
//!   reads its `target/` or `node_modules/`. `--include-ignored` reads them.
//! - **Structured data**, which is not writing. A file that parses as whole
//!   JSON and holds no conversation is a lockfile or an export, and a
//!   `package-lock.json` read as prose produces commitments cited to
//!   dependency names. This is a test of the CONTENT, so it catches a `.lock`
//!   that happens to be JSON and lets a `.json` full of minutes through.
//! - **A file larger than [`MAX_BYTES`]**, which is a log or a database
//!   rather than anybody's writing.
//!
//! A file NAMED directly is read whatever it is — the person said so. Only a
//! walk filters, because a walk is a guess about intent.
//!
//! **The rule underneath all of it: a file that was not read is reported.**
//! Pointing at a directory containing *some* readable files used to drop the
//! rest in silence — so a folder of documents plus a Slack export read as
//! "3 source(s)" with no mention of the fourth, and two rules that existed
//! only in chat were never seen by anyone. That asymmetry is exactly the
//! defaulted absence §18.3 forbids, in the one place a new user has no way to
//! check the work.
//!
//! Chat is not prose, and is not chunked as though it were. A channel export
//! is a stream of short lines by different people; the paragraph splitter
//! would make one chunk of a year. Messages are rendered with their author and
//! separated into BURSTS on a time gap, so the existing chunker cuts them at
//! conversation boundaries and a citation quotes the exchange a rule was
//! actually decided in.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

/// Past this, a walk passes a file over: it is a log, a dump or a database,
/// not writing. Reported when it happens, and naming the file directly reads
/// it regardless.
pub const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// A gap this long starts a new burst, and therefore a new chunk.
///
/// Half an hour: long enough that a conversation with pauses in it stays one
/// passage, short enough that this morning and last Tuesday do not merge into
/// a single citation nobody can locate.
const BURST_GAP_SECS: f64 = 1_800.0;

/// A burst is also cut here, so one relentless channel does not become one
/// chunk the size of a novel.
const BURST_MAX_MESSAGES: usize = 25;

/// One thing to read, named the way a person would name it.
pub struct Source {
    /// Relative to the root that was pointed at. Absolute paths are 90
    /// characters of noise in the review loop, where the whole job is reading
    /// quotes.
    pub name: String,
    pub text: String,
}

/// What was read, and what was passed over.
#[derive(Default)]
pub struct Gathered {
    pub sources: Vec<Source>,
    /// Why each unread file was unread, and how many of them there were.
    /// Rendered by [`Gathered::skipped_note`]; never dropped.
    pub skipped: BTreeMap<String, usize>,
}

impl Gathered {
    fn skip(&mut self, reason: impl Into<String>) {
        *self.skipped.entry(reason.into()).or_insert(0) += 1;
    }

    /// What to tell the person before they spend a model run on this.
    ///
    /// `None` only when everything under the path was read.
    pub fn skipped_note(&self) -> Option<String> {
        if self.skipped.is_empty() {
            return None;
        }
        let total: usize = self.skipped.values().sum();
        let mut out = format!("{total} file(s) were not read:\n");
        for (why, n) in &self.skipped {
            out.push_str(&format!("  {n} x {why}\n"));
        }
        // Named, because a report with no way out of it is a dead end.
        out.push_str(
            "  Naming a file directly reads it whatever it is; \
             --include-ignored reads what .gitignore covers.",
        );
        Some(out)
    }
}

/// Read one path — a file or a directory tree — into sources.
///
/// `include_ignored` overrides the `.gitignore` filter that a walk applies.
pub fn gather(root: &Path, into: &mut Gathered, include_ignored: bool) -> Result<(), String> {
    if !root.is_dir() {
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| root.to_string_lossy().to_string());
        // No size cap and no structure test: the person named this file.
        return match read_one(root, &name, None) {
            Ok(s) => {
                into.sources.push(s);
                Ok(())
            }
            Err(why) => Err(format!("{}: {why}", root.display())),
        };
    }

    let base = root.to_path_buf();
    let mut found: Vec<(PathBuf, String)> = Vec::new();
    walk(&base, &base, &mut found)?;
    // One `git check-ignore` for the whole walk rather than one per file: a
    // repo of any size makes that thousands of processes.
    let generated = if include_ignored {
        BTreeSet::new()
    } else {
        ignored(&base, &found)
    };
    for (path, rel) in found {
        if generated.contains(&rel) {
            into.skip("ignored by .gitignore");
            continue;
        }
        match read_one(&path, &rel, Some(MAX_BYTES)) {
            Ok(s) => into.sources.push(s),
            Err(why) => into.skip(why),
        }
    }
    Ok(())
}

fn walk(base: &Path, dir: &Path, found: &mut Vec<(PathBuf, String)>) -> Result<(), String> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)
        .map_err(|e| format!("reading {}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    // Sorted, because chunk ids are positions and a run that reads the same
    // folder twice in a different order produces an artifact that cannot be
    // compared with the first (§18.4).
    entries.sort();
    for path in entries {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        // Hidden directories are infrastructure, not notes: `.git` alone
        // would bury a run in objects, and `.canon` is the tool's own state.
        // Not counted as skipped — nobody meant them.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(base, &path, found)?;
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        found.push((path, rel));
    }
    Ok(())
}

/// Which of these paths the project itself calls generated.
///
/// Shelling out to `git check-ignore` rather than carrying a list of build
/// directories: `target/`, `node_modules/`, `_build/`, `.venv/` and whatever
/// this year's toolchain emits are already enumerated, correctly, in the
/// repo's own `.gitignore`. A guessed list is a whitelist wearing a different
/// hat — it works on the ecosystems we thought of.
///
/// No git, or a folder outside a repo, means nothing is declared generated
/// and nothing is skipped. That is silence rather than a failure: a folder
/// that never declared a `.gitignore` has not withheld anything.
///
/// **Asked in names relative to `base`, with git run from `base`.** Feeding
/// it the paths as built — which are relative to the CWD whenever `--from`
/// was — and running git from `base` makes it resolve `notes/x.md` inside
/// `notes/`, match nothing, and skip nothing: gitignore filtering that
/// silently does not happen, which is the defaulted absence §18.3 forbids.
/// Relative names remove the distinction rather than papering over it, so
/// there is no absolute-vs-relative case left to get wrong.
fn ignored(base: &Path, found: &[(PathBuf, String)]) -> BTreeSet<String> {
    if found.is_empty() {
        return BTreeSet::new();
    }
    let Ok(mut child) = Command::new("git")
        .args(["check-ignore", "--stdin", "-z"])
        .current_dir(base)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return BTreeSet::new();
    };
    let Some(mut sink) = child.stdin.take() else {
        return BTreeSet::new();
    };
    let payload: Vec<u8> = found
        .iter()
        .flat_map(|(_, rel)| {
            let mut b = rel.as_bytes().to_vec();
            b.push(0);
            b
        })
        .collect();
    // Written from its own thread. git answers as it reads, so a walk whose
    // paths exceed the pipe buffer — about 64 KiB, which is a few thousand
    // files — deadlocks if one thread tries to do both.
    let writer = std::thread::spawn(move || {
        let _ = sink.write_all(&payload);
    });
    let out = child.wait_with_output();
    let _ = writer.join();
    let Ok(out) = out else {
        return BTreeSet::new();
    };
    // git echoes back the names it was given, so these are the same relative
    // names `found` holds and compare equal without normalising.
    out.stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8_lossy(s).into_owned())
        .collect()
}

/// Read a file, or say why it was passed over.
///
/// `limit` is `None` for a file the person named and `Some` for one a walk
/// found. The structure test is likewise only applied to a walk: naming
/// `decisions.json` reads it.
fn read_one(path: &Path, name: &str, limit: Option<u64>) -> Result<Source, String> {
    if let Some(max) = limit {
        let len = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        if len > max {
            return Err(format!("larger than {} MiB", max / (1024 * 1024)));
        }
    }
    let bytes = std::fs::read(path).map_err(|e| format!("unreadable ({e})"))?;
    // The one readability test. Not an extension, not a magic number: can
    // these bytes be shown to a person as text.
    let text = String::from_utf8(bytes).map_err(|_| "not text".to_string())?;
    if text.trim().is_empty() {
        return Err("empty".to_string());
    }
    // Chat is sniffed from the content, so an export named `#general.txt`
    // reads as chat and a `.json` full of minutes does not have to be.
    let head = text.trim_start();
    if head.starts_with('{') || head.starts_with('[') {
        if let Some(rendered) = render_chat(&text) {
            return Ok(Source {
                name: name.to_string(),
                text: rendered,
            });
        }
        // Structured data that holds no conversation is machine output, and
        // reading it as prose produces commitments cited to dependency names.
        if limit.is_some() && serde_json::from_str::<Value>(&text).is_ok() {
            return Err("structured data, not a chat export".to_string());
        }
    }
    Ok(Source {
        name: name.to_string(),
        text,
    })
}

// ── chat ────────────────────────────────────────────────────

/// A message, from whichever export shape it arrived in.
struct Message {
    who: String,
    text: String,
    at: f64,
}

/// Render a chat export as text, or `None` if it is not one.
///
/// Tolerant about the shape because every tool exports differently and none
/// of them is going to change: a bare array, or an object with `messages`,
/// or JSONL. What it is NOT tolerant about is emptiness — a file that parses
/// but yields no message is reported as unread rather than counted as a
/// source that contributed nothing.
pub fn render_chat(raw: &str) -> Option<String> {
    let messages = parse_chat(raw)?;
    if messages.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut since_break = 0usize;
    let mut previous: Option<f64> = None;
    for m in &messages {
        let gap = previous.is_some_and(|p| m.at - p > BURST_GAP_SECS);
        if (gap || since_break >= BURST_MAX_MESSAGES) && !out.is_empty() {
            // A blank line is what the chunker cuts on, so a burst boundary
            // and a chunk boundary are the same thing by construction rather
            // than by a second splitter that could disagree with the first.
            out.push('\n');
            since_break = 0;
        }
        // **Rendered as block quotes, and that is load-bearing.** `locate`
        // cuts a passage into sentences to give the model a coordinate
        // system, and its splitter is prose-shaped: nine chat lines with no
        // full stops came out as ONE sentence, so the model was shown one
        // giant `[1]`, cited positions 1-2 and 3-4, and both rules it had
        // correctly found were dropped for citing past the end.
        //
        // A `>` already opens a unit for that splitter. Making the rendering
        // fit the coordinate system costs nothing and risks nothing; widening
        // the splitter's marker rule would reach every prose corpus, and that
        // rule was deliberately narrowed after a house charter's wrapped
        // "door." read as an enumerator.
        out.push_str(&format!("> {}: {}\n", m.who, m.text.trim()));
        since_break += 1;
        previous = Some(m.at);
    }
    Some(out)
}

fn parse_chat(raw: &str) -> Option<Vec<Message>> {
    // Whole-file JSON first, JSONL second — and in that order, because JSONL
    // also starts with `{`. Sniffing the first character sent every JSONL
    // export down the object branch, where it failed to parse and came back
    // as "not a chat export".
    let values: Vec<Value> = match serde_json::from_str::<Value>(raw) {
        Ok(Value::Array(a)) => a,
        Ok(Value::Object(o)) => o
            .get("messages")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default(),
        Ok(_) => return None,
        // A line that does not parse is skipped rather than failing the
        // file: exports routinely carry a trailing partial line.
        Err(_) => raw
            .lines()
            .filter_map(|l| serde_json::from_str::<Value>(l.trim()).ok())
            .collect(),
    };
    let messages: Vec<Message> = values.iter().filter_map(message).collect();
    (!messages.is_empty()).then_some(messages)
}

fn message(v: &Value) -> Option<Message> {
    // Joins, leaves and pins are not things anybody decided.
    if v.get("subtype").and_then(Value::as_str).is_some_and(|s| {
        s.contains("join") || s.contains("leave") || s.contains("pin") || s.contains("purpose")
    }) {
        return None;
    }
    let text = ["text", "content", "message", "body"]
        .iter()
        .find_map(|k| v.get(*k).and_then(Value::as_str))
        .map(str::trim)
        .filter(|t| !t.is_empty())?;
    let who = v
        .get("user_profile")
        .and_then(|p| p.get("display_name").or_else(|| p.get("real_name")))
        .and_then(Value::as_str)
        .or_else(|| {
            v.get("author")
                .and_then(|a| a.get("username").or_else(|| a.get("name")))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            ["username", "user_name", "name", "user", "author"]
                .iter()
                .find_map(|k| v.get(*k).and_then(Value::as_str))
        })
        .unwrap_or("someone");
    let at = ["ts", "timestamp", "created_at", "time"]
        .iter()
        .find_map(|k| v.get(*k))
        .and_then(|t| {
            t.as_f64()
                .or_else(|| t.as_str().and_then(|s| s.parse::<f64>().ok()))
        })
        .unwrap_or(0.0);
    Some(Message {
        who: who.to_string(),
        text: text.to_string(),
        at,
    })
}

#[cfg(test)]
mod tests;
