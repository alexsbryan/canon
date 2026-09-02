// SPDX-License-Identifier: AGPL-3.0-or-later
//! Parsing and rendering the log — over strings, never files.
//!
//! `canon-core` has no filesystem dependency by design (see the crate's
//! `Cargo.toml`). The CLI owns IO; this module owns the format.

use crate::act::{Act, FORMAT_VERSION};

#[derive(Debug)]
pub enum ParseError {
    /// A line declares a format version this build does not understand.
    /// REFUSED rather than guessed at — a reader that misinterprets an act it
    /// does not understand corrupts the fold silently.
    UnknownVersion {
        line: usize,
        found: u32,
    },
    Malformed {
        line: usize,
        detail: String,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownVersion { line, found } => write!(
                f,
                "line {line}: format version {found} is newer than this build understands (v{FORMAT_VERSION}) — upgrade canon rather than reading it partially"
            ),
            Self::Malformed { line, detail } => write!(f, "line {line}: {detail}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// An ordered set of acts.
#[derive(Debug, Clone, Default)]
pub struct Log {
    acts: Vec<Act>,
}

impl Log {
    pub fn acts(&self) -> &[Act] {
        &self.acts
    }

    pub fn is_empty(&self) -> bool {
        self.acts.is_empty()
    }

    pub fn len(&self) -> usize {
        self.acts.len()
    }

    /// Parse JSONL. Blank lines are skipped; unknown versions are refused.
    ///
    /// Acts are deduplicated by id and sorted by `(ts_unix, id)`, which is what
    /// makes a git union-merge of two branches fold identically to either side
    /// having been appended in sequence.
    pub fn parse(s: &str) -> Result<Self, ParseError> {
        let mut acts: Vec<Act> = Vec::new();
        for (i, raw) in s.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let probe: serde_json::Value =
                serde_json::from_str(line).map_err(|e| ParseError::Malformed {
                    line: i + 1,
                    detail: e.to_string(),
                })?;
            match probe.get("v").and_then(|v| v.as_u64()) {
                Some(v) if v as u32 > FORMAT_VERSION => {
                    return Err(ParseError::UnknownVersion {
                        line: i + 1,
                        found: v as u32,
                    })
                }
                Some(_) => {}
                None => {
                    return Err(ParseError::Malformed {
                        line: i + 1,
                        detail: "missing `v` (format version)".into(),
                    })
                }
            }
            let act: Act = serde_json::from_str(line).map_err(|e| ParseError::Malformed {
                line: i + 1,
                detail: e.to_string(),
            })?;
            acts.push(act);
        }
        Ok(Self::from_acts(acts))
    }

    /// Build from acts, applying the same dedupe-and-sort discipline as
    /// [`Log::parse`]. This is the merge rule: union, dedupe by
    /// content-addressed id, sort by time.
    ///
    /// The `id` tiebreak matters and is user-visible: several acts routinely
    /// land in the same second, and within that second the canonical order is
    /// by id — deterministic, and arbitrary to a reader. It is chosen over
    /// preserving file order because two machines must render byte-identical
    /// files after a merge. The fold does not depend on it
    /// (`same_second_ordering_does_not_change_the_result`), so this affects
    /// display and bytes only.
    pub fn from_acts(mut acts: Vec<Act>) -> Self {
        acts.sort_by(|a, b| a.ts_unix.cmp(&b.ts_unix).then_with(|| a.id.cmp(&b.id)));
        acts.dedup_by(|a, b| a.id == b.id);
        Self { acts }
    }

    /// Append one act, preserving order and ignoring an exact duplicate.
    pub fn push(&mut self, act: Act) {
        if self.acts.iter().any(|a| a.id == act.id) {
            return;
        }
        self.acts.push(act);
        self.acts
            .sort_by(|a, b| a.ts_unix.cmp(&b.ts_unix).then_with(|| a.id.cmp(&b.id)));
    }

    /// Render as JSONL, one act per line, newline-terminated.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for act in &self.acts {
            out.push_str(&serde_json::to_string(act).expect("act serializes"));
            out.push('\n');
        }
        out
    }

    /// Fold at the moment of the last act. Deterministic, so a replay of the
    /// same file folds the same way on every machine.
    pub fn derive(&self) -> crate::fold::Canon {
        crate::fold::derive(&self.acts)
    }

    /// Fold as of `now`. A consent window that has run out, or standing
    /// that has lapsed, reads differently today than it did when the last
    /// act was written; the CLI folds at the wall clock and a replay at the
    /// scenario's clock.
    pub fn derive_at(&self, now: i64) -> crate::fold::Canon {
        crate::fold::derive_at(&self.acts, now)
    }
}
