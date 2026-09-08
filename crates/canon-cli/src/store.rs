// SPDX-License-Identifier: AGPL-3.0-or-later
//! Filesystem and clock — everything `canon-core` deliberately refuses to do.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use canon_core::{Act, Log};

pub const DIR: &str = ".canon";
pub const FILE: &str = "acts.jsonl";

/// Find the nearest `.canon` walking up from `start`, the way git finds `.git`.
/// A codebase canon is committed at the repo root; a personal one lives in
/// `$HOME`. Falling back to `$HOME` is what makes `canon` work from anywhere.
pub fn locate(start: &Path) -> Option<PathBuf> {
    let mut cur = Some(start);
    while let Some(dir) = cur {
        let candidate = dir.join(DIR);
        if candidate.join(FILE).is_file() {
            return Some(candidate);
        }
        cur = dir.parent();
    }
    home()
        .map(|h| h.join(DIR))
        .filter(|p| p.join(FILE).is_file())
}

pub fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Lines that claim a time earlier than a line above them, as a note, or
/// nothing.
///
/// The fold sorts by `(ts_unix, id)` and forgets file order on purpose, so
/// this is the one place that can see the shape a hand-appended, backdated
/// line has: it sits after acts it claims to predate. `canon add` writes
/// at the wall clock and a rendered merge is sorted, so the honest causes
/// are a merge nobody re-rendered and two clocks that disagree. A note,
/// never a refusal — the record is still the record — and it names what
/// can actually check: `canon witness`, where git is holding the file.
pub fn disorder(dir: &Path) -> Option<String> {
    let raw = std::fs::read_to_string(dir.join(FILE)).ok()?;
    let mut latest = i64::MIN;
    let mut first: Option<String> = None;
    let mut count = 0usize;
    for line in raw.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let ts = v.get("ts_unix").and_then(serde_json::Value::as_i64)?;
        if ts < latest {
            count += 1;
            if first.is_none() {
                first = v.get("id").and_then(|i| i.as_str()).map(str::to_string);
            }
        }
        latest = latest.max(ts);
    }
    (count > 0).then(|| {
        format!(
            "note: {count} act(s) appear after acts they claim to predate (first: {}); \
             `canon witness` reads the git history if this canon is in a repository",
            first.unwrap_or_default()
        )
    })
}

pub fn read(dir: &Path) -> Result<Log, String> {
    let path = dir.join(FILE);
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    Log::parse(&raw).map_err(|e| format!("{}: {e}", path.display()))
}

/// Append one act. Append-only: we never rewrite the file, so a concurrent
/// writer's line is never lost and git sees an additive diff.
/// What inside `.canon` belongs to this machine rather than to the group.
///
/// **The canon is meant to be committed.** `acts.jsonl` is a text file people
/// put in git, merge, and resolve with `canon merge-driver` — that is the
/// point of a text log. Two things beside it are NOT the group's: `seen` is
/// one person's ingest state, and `draft-runs/` is the evidence from their
/// model runs. Committed, they conflict on every pull and tell the rest of
/// the team which candidates somebody personally declined.
///
/// Written on `init`, and again the first time a `seen` file appears, because
/// most canons already exist by then. Never overwritten: a group that has
/// edited this file meant to.
pub fn ignore_local(dir: &Path) {
    let path = dir.join(".gitignore");
    if path.exists() {
        return;
    }
    let _ = std::fs::write(
        &path,
        "# The canon itself is meant to be committed. These two are not:\n\
         # `seen` is this machine's ingest state and `draft-runs/` is the\n\
         # evidence from its model runs. Neither is an act.\n\
         seen\n\
         draft-runs/\n",
    );
}

pub fn append(dir: &Path, act: &Act) -> Result<(), String> {
    let path = dir.join(FILE);
    let line = serde_json::to_string(act).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("opening {}: {e}", path.display()))?;
    // **One `write_all`, newline included, and that is the whole point.**
    // `writeln!` formats in pieces and issues TWO writes — the line, then the
    // newline — so two `canon add`s running at once could interleave under
    // `O_APPEND` into `{act}{act}\n\n`: one unparseable line and one empty
    // one, in the file that IS the record. A single append-mode write is
    // atomic, which is what makes the promise above true.
    f.write_all(format!("{line}\n").as_bytes())
        .map_err(|e| format!("writing {}: {e}", path.display()))
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Who is acting. `CANON_ACTOR`, else git's configured name, else `$USER`.
///
/// Prefixed `human:` because a person is running a CLI. Automation that writes
/// acts must set `CANON_ACTOR` to a non-human label so the fold can report it
/// — see `State::unattended`.
pub fn actor() -> String {
    if let Ok(a) = std::env::var("CANON_ACTOR") {
        return a;
    }
    let git = Command::new("git")
        .args(["config", "--get", "user.name"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let name = git
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "unknown".into());
    format!("human:{name}")
}

/// Render a timestamp as `YYYY-MM-DD`.
///
/// Delegates. The civil calendar lives in `canon-core::date` because the
/// format itself carries date strings (`Accept.revisit`) and the staleness
/// query has to read them — two implementations would be two answers to "is
/// this overdue" (§10.6).
pub fn ymd(ts: i64) -> String {
    canon_core::date::ymd(ts)
}

#[cfg(test)]
mod tests {
    #[test]
    fn dates_come_from_the_one_calendar() {
        // Not a second test of the algorithm — a test that this module has
        // not grown its own.
        assert_eq!(
            super::ymd(1_771_027_200),
            canon_core::date::ymd(1_771_027_200)
        );
        assert_eq!(super::ymd(0), "1970-01-01");
    }

    #[test]
    fn two_writers_at_once_cannot_corrupt_the_record() {
        // `writeln!` formats in pieces and issues TWO writes, the line and
        // then the newline, so two appends racing under `O_APPEND` could
        // interleave into one unparseable line and one empty one — in the
        // file that IS the record, and whose own doc comment promises that a
        // concurrent writer's line is never lost.
        use canon_core::{Act, ActKind};
        let dir = std::env::temp_dir().join("canon-store-race");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let (writers, each) = (8, 40);
        let done: Vec<_> = (0..writers)
            .map(|w| {
                let dir = dir.clone();
                std::thread::spawn(move || {
                    for i in 0..each {
                        let act = Act::new(
                            ActKind::Assert {
                                text: format!("writer {w} commitment {i}"),
                                from: None,
                                source: None,
                            },
                            1_771_027_200 + i,
                            format!("human:writer-{w}"),
                        );
                        super::append(&dir, &act).unwrap();
                    }
                })
            })
            .collect();
        for t in done {
            t.join().unwrap();
        }

        // Every line lands, and every line parses. Either failure is the bug.
        let log = super::read(&dir).expect("the record is still readable");
        assert_eq!(log.len(), (writers * each) as usize);
    }
}
