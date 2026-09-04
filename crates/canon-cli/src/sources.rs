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
//! the BYTES. Text is read. Anything that is not text is skipped and
//! reported.
//!
//! **The walk is [`ignore`], ripgrep's, and that is deliberate.** It used to
//! be thirty lines here, and those thirty lines had three bugs that only a
//! decade of other people's bug reports finds: `is_dir()` follows symlinks,
//! so a link in the folder walked out of the tree it was pointed at and read
//! whatever it found — the traversal escape `SECURITY.md` names; one
//! unreadable subdirectory returned `Err` and ended the walk at zero sources;
//! and a fifo passed the size cap with a length of 0 and then blocked on
//! `read` forever, with no output and no timeout. Directory walking is a
//! solved problem that does not look like one.
//!
//! Four things a walk passes over, each reported and each with a way round it:
//!
//! - **What the project itself calls generated** — `.gitignore`, so the
//!   authority is the person's own file and not a list of build directories
//!   we guessed at. Without it, pointing at any checked-out repo reads its
//!   `target/` or `node_modules/`. `--include-ignored` reads them.
//! - **Structured data**, which is not writing. A file that parses as whole
//!   JSON and holds no conversation is a lockfile or an export, and a
//!   `package-lock.json` read as prose produces commitments cited to
//!   dependency names. This is a test of the CONTENT, so it catches a `.lock`
//!   that happens to be JSON and lets a `.json` full of minutes through.
//! - **A file larger than [`MAX_BYTES`]**, which is a log or a database
//!   rather than anybody's writing.
//! - **Anything that leaves the tree that was pointed at.** A symlink is
//!   followed only when its target stays inside the root, because every
//!   passage read here is quoted verbatim into a proposal, and therefore into
//!   `acts.jsonl`, and therefore into somebody's git history. A link to
//!   `~/.aws` is not a thing to resolve quietly.
//!
//! A file NAMED directly is read whatever it is — the person said so. Only a
//! walk filters, because a walk is a guess about intent. The single exception
//! is that it has to be a FILE: `--from /dev/zero` and `--from some.fifo` are
//! not large reads, they are reads that never return, and "the person said
//! so" cannot have meant that. `--from -` is the way to stream.
//!
//! **Encodings are read only where the bytes DECLARE one.** A byte-order mark
//! is the file saying what it is, so `EF BB BF` is stripped rather than left
//! to lead the first citation, and `FF FE` / `FE FF` are decoded as UTF-16 —
//! which is what every Windows editor writes when a person picks "Unicode".
//! What this deliberately does NOT do is guess. Sniffing an encoding
//! statistically would read more files, and would sometimes read them WRONG:
//! a mis-guessed Shift-JIS file is not skipped and reported, it is mojibake
//! that looks like a quotation, and it goes into the log as one. Every decode
//! here is a declaration the file made about itself.
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
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
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
    /// quotes. Made unique across roots by [`Gathered::resolve_names`].
    pub name: String,
    pub text: String,
    /// Where it came from, when it came from disk. `None` for stdin and for
    /// commit bodies, which have no path to be disambiguated by.
    pub path: Option<PathBuf>,
}

impl Source {
    /// A source that did not come from a path — stdin, or a commit body.
    pub fn unplaced(name: impl Into<String>, text: impl Into<String>) -> Self {
        Source {
            name: name.into(),
            text: text.into(),
            path: None,
        }
    }
}

/// What was read, and what was passed over.
#[derive(Default)]
pub struct Gathered {
    pub sources: Vec<Source>,
    /// Why each unread file was unread, and how many of them there were.
    /// Rendered by [`Gathered::skipped_note`]; never dropped.
    pub skipped: BTreeMap<String, usize>,
    /// Canonical paths already taken, so that overlapping roots — `--from .
    /// ./notes`, or the same file named twice — read a file once. Not
    /// reported: nothing was withheld, it is the same passage.
    seen: BTreeSet<PathBuf>,
}

impl Gathered {
    fn skip(&mut self, reason: impl Into<String>) {
        *self.skipped.entry(reason.into()).or_insert(0) += 1;
    }

    fn take(&mut self, source: Source) {
        if let Some(p) = &source.path {
            let key = p.canonicalize().unwrap_or_else(|_| p.clone());
            if !self.seen.insert(key) {
                return;
            }
        }
        self.sources.push(source);
    }

    /// Give every source a name no other source has.
    ///
    /// **A citation is the thing a reader checks a rule against**, so two
    /// passages answering to the same coordinates is the one ambiguity this
    /// module cannot ship. Names are relative to the root they were found
    /// under, which is what keeps `handbook.md:3-4` readable — and which made
    /// `--from project-a project-b` produce two sources both called
    /// `README.md`, where `README.md:3-4` named a passage in neither.
    ///
    /// Widened by one leading path component at a time, and only for the
    /// names that actually collide, so the common case keeps its short names
    /// and the ambiguous case gets exactly enough path to be told apart.
    pub fn resolve_names(&mut self) {
        loop {
            let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
            for (i, s) in self.sources.iter().enumerate() {
                by_name.entry(s.name.as_str()).or_default().push(i);
            }
            let clashing: Vec<usize> = by_name
                .into_values()
                .filter(|group| group.len() > 1)
                .flatten()
                .collect();
            if clashing.is_empty() {
                return;
            }
            let mut widened = false;
            for i in clashing {
                if let Some(longer) = widen(&self.sources[i]) {
                    self.sources[i].name = longer;
                    widened = true;
                }
            }
            // Two sources with no path left to spend — a `--as` name that
            // matches a file, say. Nothing more to try, and a loop that
            // cannot make progress must not run again.
            if !widened {
                return;
            }
        }
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

/// One more leading path component than the name already carries, or `None`
/// when the name has spent the whole path and there is nothing left to add.
fn widen(source: &Source) -> Option<String> {
    let path = source.path.as_ref()?;
    let parts: Vec<_> = path.components().collect();
    let have = Path::new(&source.name).components().count();
    if have >= parts.len() {
        return None;
    }
    let tail: PathBuf = parts[parts.len() - (have + 1)..].iter().collect();
    Some(tail.to_string_lossy().to_string())
}

/// Read one path — a file or a directory tree — into sources.
///
/// `include_ignored` overrides the `.gitignore` filter that a walk applies.
pub fn gather(root: &Path, into: &mut Gathered, include_ignored: bool) -> Result<(), String> {
    // Follows a link, because the person named this path and meant the thing
    // at the end of it. Only what a WALK finds is held to the root.
    let meta = std::fs::metadata(root).map_err(|e| format!("{}: {}", root.display(), why(&e)))?;
    if meta.is_dir() {
        walk(root, into, include_ignored);
        return Ok(());
    }
    if !meta.is_file() {
        // The hang this replaces: a character device or a fifo has a length
        // of 0, so it passed every size cap, and then `read` blocked forever
        // with nothing printed and no timeout.
        return Err(format!(
            "{}: not a regular file — a device, socket or pipe. \
             `… | canon draft --from -` streams instead.",
            root.display()
        ));
    }
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| root.to_string_lossy().to_string());
    // No size cap and no structure test: the person named this file.
    match read_one(root, &name, None) {
        Ok(s) => {
            into.take(s);
            Ok(())
        }
        Err(why) => Err(format!("{}: {why}", root.display())),
    }
}

/// Walk a tree, reading what is writing and reporting what is not.
///
/// **Nothing is pruned, and the ignore decision is asked per file.** Pruning
/// an ignored directory is faster and loses the count, and "N file(s) were not
/// read" is the promise this module is built around — a walk that silently
/// declines to mention `node_modules` is the same defaulted absence as one
/// that silently declines to mention a Slack export. This still spawns no
/// process and pipes no paths, so it is strictly less work than the
/// `git check-ignore` shell-out it replaces.
fn walk(root: &Path, into: &mut Gathered, include_ignored: bool) {
    let mut matcher = if include_ignored {
        None
    } else {
        // A second builder, with the standard filters left ON, purely to
        // answer "would git ignore this?". `.ignore` files are switched off
        // so the reported reason stays true to the word `.gitignore`.
        WalkBuilder::new(root).ignore(false).build_matchers().pop()
    };
    let root_abs = root.canonicalize().ok();
    // Sorted, because chunk ids are positions and a run that reads the same
    // folder twice in a different order produces an artifact that cannot be
    // compared with the first (§18.4).
    let walker = WalkBuilder::new(root)
        .standard_filters(false)
        // Hidden directories are infrastructure, not notes: `.git` alone
        // would bury a run in objects, and `.canon` is the tool's own state.
        // Not counted as skipped — nobody meant them.
        .hidden(true)
        .follow_links(false)
        .sort_by_file_name(|a, b| a.cmp(b))
        .build();

    // Ignored directories, so a file inherits its parent's fate the way git
    // decides it. Asking only about the file would read `target/debug/x.log`
    // out of a `target/` nobody wanted walked.
    let mut ignored_dirs: Vec<PathBuf> = Vec::new();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            // One unreadable directory used to end the walk and return zero
            // sources: a single `chmod 000` folder anywhere under
            // `~/Documents` failed the whole ingest. It is one skip now, and
            // the walk goes on.
            Err(_) => {
                into.skip("in a folder that could not be read");
                continue;
            }
        };
        let Some(kind) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        if let Some(m) = matcher.as_mut() {
            if ignored_dirs.iter().any(|d| rel.starts_with(d)) {
                if !kind.is_dir() {
                    into.skip("ignored by .gitignore");
                }
                continue;
            }
            if m.matched(rel, kind.is_dir()).is_ignore() {
                if kind.is_dir() {
                    ignored_dirs.push(rel.to_path_buf());
                } else {
                    into.skip("ignored by .gitignore");
                }
                continue;
            }
        }
        if kind.is_dir() {
            continue;
        }
        let name = rel.to_string_lossy().to_string();
        if kind.is_symlink() {
            match resolved(path, root_abs.as_deref()) {
                // Its files are reached by their real path inside this same
                // walk, so nothing is withheld by not descending it twice.
                Some(target) if target.is_dir() => continue,
                Some(_) => {}
                None => {
                    into.skip("a link out of the folder that was pointed at");
                    continue;
                }
            }
        } else if !kind.is_file() {
            // Carrying its own way out, because the general note below —
            // "naming a file directly reads it whatever it is" — is the one
            // hint that is NOT true of a pipe.
            into.skip("not a regular file (a device, socket or pipe); `… | canon draft --from -` streams one");
            continue;
        }
        match read_one(path, &name, Some(MAX_BYTES)) {
            Ok(s) => into.take(s),
            Err(reason) => into.skip(reason),
        }
    }
}

/// Where a link actually points, or `None` if that is outside `root` — or
/// nowhere at all, which a dangling link is.
///
/// **This is the traversal escape `SECURITY.md` asks to hear about first.**
/// Every passage read here is quoted verbatim into a proposal, into
/// `acts.jsonl`, and into somebody's git history. A link inside the tree is
/// somebody's own organisation of their own notes and is followed; a link to
/// `~/.aws` is not.
fn resolved(path: &Path, root_abs: Option<&Path>) -> Option<PathBuf> {
    let target = path.canonicalize().ok()?;
    match root_abs {
        // No canonical root to compare against means no way to prove the
        // target is inside it, and an unprovable containment is a no.
        Some(base) => target.starts_with(base).then_some(target),
        None => None,
    }
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
    let bytes = std::fs::read(path).map_err(|e| format!("unreadable ({})", why(&e)))?;
    // The one readability test. Not an extension, not a magic number: can
    // these bytes be shown to a person as text.
    let text = decode(&bytes)?;
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
                path: Some(path.to_path_buf()),
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
        path: Some(path.to_path_buf()),
    })
}

// ── encodings ───────────────────────────────────────────────

/// Bytes as text, where the bytes said which text they are.
///
/// Every branch below is a DECLARATION the file made about itself, never a
/// guess. See the module note on why there is no statistical sniffing here.
fn decode(bytes: &[u8]) -> Result<String, String> {
    let text = declared(bytes)?;
    // **A NUL is not something that can be shown to a person, and
    // `String::from_utf8` accepts it.** UTF-16 ASCII with no byte-order mark
    // is VALID UTF-8 — one NUL between every letter — so it passed the
    // readability test whole and went to the model as a passage with a NUL
    // in every gap. Valid is not the same as text.
    if text.contains('\0') {
        return Err(unreadable(bytes));
    }
    Ok(text)
}

/// Text in whichever encoding the bytes said they were in.
fn declared(bytes: &[u8]) -> Result<String, String> {
    if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        // Stripped rather than kept: left in, U+FEFF leads the file, so it
        // leads the first chunk, so it leads the first quotation a person is
        // asked to check a rule against.
        return String::from_utf8(rest.to_vec()).map_err(|_| "not UTF-8 text".to_string());
    }
    // Before the UTF-16 marks, because a UTF-32LE file also starts `FF FE`.
    if bytes.starts_with(&[0xFF, 0xFE, 0x00, 0x00]) || bytes.starts_with(&[0x00, 0x00, 0xFE, 0xFF])
    {
        return Err("UTF-32 text, which nothing writes prose in".to_string());
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(rest, u16::from_le_bytes);
    }
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        return utf16(rest, u16::from_be_bytes);
    }
    String::from_utf8(bytes.to_vec()).map_err(|_| unreadable(bytes))
}

fn utf16(rest: &[u8], order: fn([u8; 2]) -> u16) -> Result<String, String> {
    if !rest.len().is_multiple_of(2) {
        return Err("truncated UTF-16 text".to_string());
    }
    let units: Vec<u16> = rest
        .chunks_exact(2)
        .map(|pair| order([pair[0], pair[1]]))
        .collect();
    String::from_utf16(&units).map_err(|_| "malformed UTF-16 text".to_string())
}

/// Why bytes could not be shown to a person, said usefully.
///
/// A file with no byte-order mark is not decoded — but "not text" is a lie
/// about a UTF-16 file, and a person who is told the truth can re-save it.
/// Reporting a shape is not the same as guessing at content: nothing here
/// decides what the bytes SAY.
fn unreadable(bytes: &[u8]) -> String {
    let head = &bytes[..bytes.len().min(512)];
    let nuls = head.iter().filter(|b| **b == 0).count();
    if head.len() >= 8 && nuls * 3 >= head.len() {
        return "not UTF-8 (looks like UTF-16 with no byte-order mark)".to_string();
    }
    "not text".to_string()
}

/// An IO error the way a person would say it.
fn why(e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => "no such file or folder".to_string(),
        std::io::ErrorKind::PermissionDenied => "permission denied".to_string(),
        _ => e.to_string(),
    }
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
