// SPDX-License-Identifier: AGPL-3.0-or-later
//! What `canon draft` was pointed at, and what it could not read.
//!
//! **Nobody writes a canon one rule at a time.** The normative content
//! already exists — a handbook, two years of meeting notes, the channel where
//! things actually get decided — and onboarding is pointing at that folder.
//! Which makes the reader the first thing a new user's trust rests on.
//!
//! **The rule this module exists to hold: a file that was not read is
//! reported.** Before this, pointing at a directory containing nothing
//! readable failed loudly and pointing at one containing *some* readable files
//! dropped the rest in silence — so a folder of documents plus a Slack export
//! read as "3 source(s)" with no mention of the fourth, and two rules that
//! existed only in chat were never seen by anyone. That asymmetry is exactly
//! the defaulted absence §18.3 forbids, in the one place a new user has no way
//! to check the work.
//!
//! Chat is not prose, and is not chunked as though it were. A channel export
//! is a stream of short lines by different people; the paragraph splitter
//! would make one chunk of a year. Messages are rendered with their author and
//! separated into BURSTS on a time gap, so the existing chunker cuts them at
//! conversation boundaries and a citation quotes the exchange a rule was
//! actually decided in.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Extensions read as prose, verbatim.
pub const PROSE: &[&str] = &["md", "markdown", "txt", "text"];

/// Extensions probed as a chat export.
pub const CHAT: &[&str] = &["json", "jsonl"];

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
        out.push_str("  canon reads .md .txt and Slack/Discord .json exports.");
        Some(out)
    }
}

/// Read one path — a file or a directory tree — into sources.
pub fn gather(root: &Path, into: &mut Gathered) -> Result<(), String> {
    if root.is_dir() {
        let base = root.to_path_buf();
        return walk(&base, &base, into);
    }
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    // A file named EXPLICITLY is read whatever its extension — the person
    // said so. Only a walk filters, because a walk is a guess about intent.
    match read_one(root, &name) {
        Some(s) => into.sources.push(s),
        None => {
            return Err(format!(
                "{} is not readable as prose or as a chat export",
                root.display()
            ))
        }
    }
    Ok(())
}

fn walk(base: &Path, dir: &Path, into: &mut Gathered) -> Result<(), String> {
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
        // would bury a run in objects. Not counted as skipped — nobody meant
        // them.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(base, &path, into)?;
            continue;
        }
        let rel = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        let ext = path
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        if !PROSE.contains(&ext.as_str()) && !CHAT.contains(&ext.as_str()) {
            into.skip(if ext.is_empty() {
                "with no extension".to_string()
            } else {
                format!(".{ext}")
            });
            continue;
        }
        match read_one(&path, &rel) {
            Some(s) => into.sources.push(s),
            // A `.json` that is not a chat export is a real skip and says so
            // — "we read json" and "we read YOUR json" are different claims.
            None => into.skip(format!(".{ext} that is not a chat export")),
        }
    }
    Ok(())
}

fn read_one(path: &Path, name: &str) -> Option<Source> {
    let text = std::fs::read_to_string(path).ok()?;
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if CHAT.contains(&ext.as_str()) {
        let rendered = render_chat(&text)?;
        return Some(Source {
            name: name.to_string(),
            text: rendered,
        });
    }
    Some(Source {
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
