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

pub fn read(dir: &Path) -> Result<Log, String> {
    let path = dir.join(FILE);
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    Log::parse(&raw).map_err(|e| format!("{}: {e}", path.display()))
}

/// Append one act. Append-only: we never rewrite the file, so a concurrent
/// writer's line is never lost and git sees an additive diff.
pub fn append(dir: &Path, act: &Act) -> Result<(), String> {
    let path = dir.join(FILE);
    let line = serde_json::to_string(act).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("opening {}: {e}", path.display()))?;
    writeln!(f, "{line}").map_err(|e| format!("writing {}: {e}", path.display()))
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

pub fn ymd(ts: i64) -> String {
    // Civil-from-days (Howard Hinnant's algorithm). Avoids a chrono dependency
    // for the one thing we need dates for: printing them.
    let days = ts.div_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

#[cfg(test)]
mod tests {
    use super::ymd;

    #[test]
    fn dates_render_correctly() {
        assert_eq!(ymd(0), "1970-01-01");
        assert_eq!(ymd(1_771_027_200), "2026-02-14");
        assert_eq!(ymd(1_609_459_200), "2021-01-01");
    }
}
