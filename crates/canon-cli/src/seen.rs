// SPDX-License-Identifier: AGPL-3.0-or-later
//! What this canon has already been shown. Ingest hygiene, not governance.
//!
//! **A feed nags forever without this.** `draft` skips candidates already in
//! the canon, which covers everything you ACCEPTED — and records nothing
//! about what you turned down. Point it at the same channel tomorrow and it
//! re-extracts the same passages, re-proposes the same rules, and asks you
//! again about the one you already said no to. On a folder you read once that
//! is a papercut. On a live feed it is the whole experience, and the second
//! day is when people stop running it.
//!
//! Two kinds of entry, both keyed by [`canon_core::short_digest`] of the
//! text:
//!
//! - **read** — a passage the model has already extracted from. Skipping it
//!   is the only thing that makes re-pointing at a growing feed cost the
//!   delta rather than the whole history. Recorded only when the model
//!   actually answered: a chunk that errored must stay unseen, or one bad
//!   reply blinds the tool to that passage permanently.
//! - **rejected** — a candidate the person declined. Never proposed again.
//!
//! **This file is not part of the canon and must never become one.** Nothing
//! here is an act, nothing here is derivable state, and no answer `check`
//! gives depends on it. Deleting `.canon/seen` costs a re-extraction and
//! changes no commitment — which is the test for whether a thing belongs in
//! `acts.jsonl` or beside it. `[s]kip` deliberately writes nothing: skip
//! means not now, and only `[r]eject` means no.

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::{Path, PathBuf};

pub const FILE: &str = "seen";

/// Why a digest is here. Closed set, so it is an enum (§2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Why {
    /// A passage already put through the extractor.
    Read,
    /// A candidate the person declined.
    Rejected,
}

impl Why {
    pub fn as_str(self) -> &'static str {
        match self {
            Why::Read => "read",
            Why::Rejected => "rejected",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "read" => Some(Why::Read),
            "rejected" => Some(Why::Rejected),
            _ => None,
        }
    }
}

#[derive(Default)]
pub struct Seen {
    path: PathBuf,
    /// A preview reads the set and never adds to it.
    writes: bool,
    read: BTreeSet<String>,
    rejected: BTreeSet<String>,
}

impl Seen {
    /// Load, or an empty set. A missing or unparseable line is not an error:
    /// the worst a lost entry costs is one re-extraction.
    pub fn load(dir: &Path) -> Self {
        Seen::open(dir, true)
    }

    /// Read the set, but never write to it. What `--dry-run` uses.
    ///
    /// A preview that recorded what it read would make the real run that
    /// follows it find nothing — you would look at the candidates, decide to
    /// keep three, run it for real and be told there is nothing there.
    /// Reading is still right: a preview should not re-offer what you have
    /// already declined.
    pub fn preview(dir: &Path) -> Self {
        Seen::open(dir, false)
    }

    fn open(dir: &Path, writes: bool) -> Self {
        let path = dir.join(FILE);
        let mut out = Seen {
            path,
            writes,
            ..Default::default()
        };
        let Ok(raw) = std::fs::read_to_string(&out.path) else {
            return out;
        };
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((digest, why)) = line.split_once(char::is_whitespace) else {
                continue;
            };
            match Why::parse(why.trim()) {
                Some(Why::Read) => out.read.insert(digest.to_string()),
                Some(Why::Rejected) => out.rejected.insert(digest.to_string()),
                None => continue,
            };
        }
        out
    }

    pub fn was_read(&self, text: &str) -> bool {
        self.read.contains(&canon_core::short_digest(text))
    }

    pub fn was_rejected(&self, text: &str) -> bool {
        self.rejected.contains(&canon_core::short_digest(text))
    }

    /// Append one entry, and hold it in memory for the rest of this run.
    ///
    /// Appended as it happens rather than written at the end: a run
    /// interrupted after forty passages should not re-read all forty.
    pub fn record(&mut self, text: &str, why: Why) -> Result<(), String> {
        let digest = canon_core::short_digest(text);
        let fresh = match why {
            Why::Read => self.read.insert(digest.clone()),
            Why::Rejected => self.rejected.insert(digest.clone()),
        };
        if !fresh || !self.writes {
            return Ok(());
        }
        let new = !self.path.exists();
        if new {
            // Most canons predate this file, so `init` alone would not have
            // covered them. The write that creates the problem closes it.
            if let Some(dir) = self.path.parent() {
                crate::store::ignore_local(dir);
            }
        }
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("writing {}: {e}", self.path.display()))?;
        if new {
            // A person who finds this file should be able to tell in one
            // line that deleting it is safe.
            let _ = writeln!(
                f,
                "# passages already read and candidates already declined.\n\
                 # ingest hygiene, not part of the canon — deleting this file\n\
                 # costs a re-extraction and changes no commitment."
            );
        }
        writeln!(f, "{digest} {}", why.as_str())
            .map_err(|e| format!("writing {}: {e}", self.path.display()))
    }
}

#[cfg(test)]
mod tests;
