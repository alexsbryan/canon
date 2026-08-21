// SPDX-License-Identifier: AGPL-3.0-or-later
//! The verbs. Every model-free command is here in full; the three that need an
//! endpoint report that plainly rather than degrading into something that
//! looks like an answer.

use std::path::{Path, PathBuf};

use canon_core::{Act, ActKind, Canon, Log};

use crate::config::{Config, Key};
use crate::profile::Profile;
use crate::store;

// ── plumbing ────────────────────────────────────────────────

pub fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}

pub fn has(args: &[String], name: &str) -> bool {
    args.iter().any(|a| a == name)
}

pub fn positionals(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut skip = false;
    for a in args {
        if skip {
            skip = false;
            continue;
        }
        if matches!(
            a.as_str(),
            "-m" | "--from" | "--profile" | "--revisit" | "--since" | "--onto" | "--endpoint"
        ) {
            skip = true;
            continue;
        }
        if a.starts_with('-') {
            continue;
        }
        out.push(a.as_str());
    }
    out
}

pub fn dir() -> Result<PathBuf, String> {
    if let Ok(d) = std::env::var("CANON_DIR") {
        return Ok(PathBuf::from(d));
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    store::locate(&cwd)
        .ok_or_else(|| "no canon here. `canon init` to start one, or set CANON_DIR".to_string())
}

pub fn load() -> Result<(PathBuf, Log, Canon), String> {
    let d = dir()?;
    let log = store::read(&d)?;
    let st = log.derive();
    Ok((d, log, st))
}

pub fn fail(e: impl std::fmt::Display) -> i32 {
    eprintln!("error: {e}");
    2
}

pub fn write(d: &Path, kind: ActKind) -> Result<Act, String> {
    let act = Act::new(kind, store::now(), store::actor());
    store::append(d, &act)?;
    Ok(act)
}

// ── record ──────────────────────────────────────────────────

pub fn init(args: &[String]) -> i32 {
    let profile = match Profile::parse(flag(args, "--profile").unwrap_or("personal")) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    let base = match std::env::var("CANON_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => match std::env::current_dir() {
            Ok(c) => c.join(store::DIR),
            Err(e) => return fail(e),
        },
    };
    if base.join(store::FILE).exists() {
        return fail(format!("a canon already exists at {}", base.display()));
    }
    if let Err(e) = std::fs::create_dir_all(&base) {
        return fail(e);
    }
    if let Err(e) = std::fs::write(base.join("profile"), format!("{}\n", profile.as_str())) {
        return fail(e);
    }
    if let Err(e) = std::fs::write(base.join(store::FILE), "") {
        return fail(e);
    }
    println!(
        "canon initialised at {} (profile: {})",
        base.display(),
        profile.as_str()
    );
    println!("  canon add \"<your first commitment>\"");
    0
}

pub fn add(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(text) = pos.first() else {
        return fail("usage: canon add \"<commitment>\"");
    };
    let d = match dir() {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    match write(
        &d,
        ActKind::Assert {
            text: (*text).to_string(),
            from: None,
            source: None,
        },
    ) {
        Ok(act) => {
            println!("{}  {}", act.id, text);
            0
        }
        Err(e) => fail(e),
    }
}

pub fn list(args: &[String]) -> i32 {
    let (_, _, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if has(args, "--json") {
        let live: Vec<_> = st.active().collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&live).unwrap_or_default()
        );
        return 0;
    }
    let live: Vec<_> = st.active().collect();
    if live.is_empty() {
        println!("no live commitments. `canon add \"...\"` to start.");
        return 0;
    }
    for c in &live {
        println!("{}  {}", c.id, c.text);
    }
    println!("\n{} live", live.len());
    let carried = st.tolerated().count();
    if carried > 0 {
        println!("{carried} contradiction(s) carried knowingly — `canon list --json` for detail");
    }
    // A hole in the record is louder than a missing feature.
    if !st.dangling.is_empty() {
        eprintln!(
            "\nwarning: {} act(s) reference a commitment that is not in this log:",
            st.dangling.len()
        );
        for (act, missing) in &st.dangling {
            eprintln!("  {act} -> {missing}");
        }
    }
    // Absence of attribution is reported, never defaulted.
    if !st.unattended.is_empty() {
        eprintln!(
            "\nwarning: {} adjudication(s) were not authored by a person: {}",
            st.unattended.len(),
            st.unattended
                .iter()
                .map(|i| i.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    0
}

pub fn why(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(needle) = pos.first() else {
        return fail("usage: canon why <id>");
    };
    let (_, log, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let id = match crate::explain::resolve(&st, needle) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    match crate::explain::explain(&log, &st, &id) {
        Ok(e) => {
            print!("{}", e.render("  "));
            0
        }
        Err(e) => fail(e),
    }
}

pub fn supersede(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon supersede <id> \"<new text>\" -m \"<reason>\"");
    }
    let (d, _, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let old = match crate::explain::resolve(&st, pos[0]) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    match write(
        &d,
        ActKind::Supersede {
            text: pos[1].to_string(),
            old: vec![old.clone()],
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(act) => {
            println!("{}  {}", act.id, pos[1]);
            println!("  replaces {old}");
            0
        }
        Err(e) => fail(e),
    }
}

pub fn retract(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(needle) = pos.first() else {
        return fail("usage: canon retract <id> -m \"<reason>\"");
    };
    let (d, _, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let target = match crate::explain::resolve(&st, needle) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    match write(
        &d,
        ActKind::Retract {
            target: target.clone(),
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(_) => {
            println!("retracted {target}");
            0
        }
        Err(e) => fail(e),
    }
}

pub fn accept(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon accept <a> <b> -m \"<reason>\"");
    }
    // The rationale is required here and nowhere else: an accepted
    // contradiction must say what it protects.
    let Some(rationale) = flag(args, "-m").filter(|r| !r.trim().is_empty()) else {
        return fail("accept requires -m \"<reason>\" — a tolerated contradiction must say why");
    };
    let (d, _, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let (a, b) = match (
        crate::explain::resolve(&st, pos[0]),
        crate::explain::resolve(&st, pos[1]),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return fail(e),
    };
    match write(
        &d,
        ActKind::Accept {
            a: a.clone(),
            b: b.clone(),
            rationale: rationale.to_string(),
            revisit: flag(args, "--revisit").map(str::to_string),
        },
    ) {
        Ok(_) => {
            println!("carrying {a} against {b} knowingly");
            println!("  {rationale}");
            0
        }
        Err(e) => fail(e),
    }
}

pub fn dismiss(args: &[String]) -> i32 {
    let pos = positionals(args);
    if pos.len() < 2 {
        return fail("usage: canon dismiss <a> <b> [-m \"<reason>\"]");
    }
    let (d, _, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let (a, b) = match (
        crate::explain::resolve(&st, pos[0]),
        crate::explain::resolve(&st, pos[1]),
    ) {
        (Ok(a), Ok(b)) => (a, b),
        (Err(e), _) | (_, Err(e)) => return fail(e),
    };
    match write(
        &d,
        ActKind::Dismiss {
            a: a.clone(),
            b: b.clone(),
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(_) => {
            println!("dismissed: {a} and {b} are not in conflict");
            0
        }
        Err(e) => fail(e),
    }
}

pub fn undo(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(target) = pos.first() else {
        return fail("usage: canon undo <act-id> -m \"<reason>\"");
    };
    let (d, log, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let hits: Vec<&Act> = log
        .acts()
        .iter()
        .filter(|a| a.id.as_str() == *target || a.id.as_str().starts_with(target))
        .collect();
    let act_id = match hits.len() {
        1 => hits[0].id.clone(),
        0 => {
            return fail(format!(
                "no act matching `{target}` — `canon log` to see them"
            ))
        }
        n => return fail(format!("`{target}` matches {n} acts — use more characters")),
    };
    match write(
        &d,
        ActKind::Revert {
            targets: vec![act_id.clone()],
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(_) => {
            println!("reverted {act_id} (itself revertible)");
            0
        }
        Err(e) => fail(e),
    }
}

pub fn log(args: &[String]) -> i32 {
    let (_, log, _) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if has(args, "--json") {
        print!("{}", log.render());
        return 0;
    }
    for act in log.acts() {
        let what = match &act.kind {
            ActKind::Assert { text, .. } => format!("assert     {text}"),
            ActKind::Supersede { text, .. } => format!("supersede  {text}"),
            ActKind::Retract { target, .. } => format!("retract    {target}"),
            ActKind::Accept { a, b, .. } => format!("accept     {a} / {b}"),
            ActKind::Dismiss { a, b, .. } => format!("dismiss    {a} / {b}"),
            ActKind::Revert { targets, .. } => format!("revert     {}", targets.len()),
            ActKind::Adopt {
                lineage,
                generation,
                ..
            } => {
                format!("adopt      {lineage}@{generation}")
            }
        };
        println!(
            "{}  {}  {}  {}",
            act.id,
            store::ymd(act.ts_unix),
            act.actor,
            what
        );
    }
    println!("\n{} acts", log.len());
    0
}

pub fn share(_args: &[String]) -> i32 {
    let (d, _, st) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    // Name the snapshot after the directory holding the canon, so a pasted
    // block says where it came from without anyone configuring anything.
    let name = std::fs::read_to_string(d.join("name"))
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            d.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "canon".into());
    let live: Vec<_> = st.active().collect();

    // A snapshot is not a log: it carries derived current state and drops
    // supersession history and rationales, which name incidents and people.
    // Enough to adopt, not enough to audit.
    let profile = match Profile::load(&d) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    println!(
        "--- canon {name} · {} · snapshot {}",
        profile.as_str(),
        store::ymd(store::now())
    );
    for c in &live {
        println!("{}  ({})", c.text, c.id);
    }
    println!("--- {} live · adopt: canon adopt --paste", live.len());
    0
}

// ── configuration ───────────────────────────────────────────

pub fn config(args: &[String]) -> i32 {
    let pos = positionals(args);
    let d = match dir() {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    match pos.first().copied() {
        Some("set") => {
            if pos.len() < 3 {
                return fail("usage: canon config set <key> <value>");
            }
            let key = match Key::parse(pos[1]) {
                Ok(k) => k,
                Err(e) => return fail(e),
            };
            match Config::write(&d, key, pos[2]) {
                Ok(()) => {
                    println!("{} = {}", key.as_str(), pos[2]);
                    0
                }
                Err(e) => fail(e),
            }
        }
        Some("get") => {
            if pos.len() < 2 {
                return fail("usage: canon config get <key>");
            }
            let key = match Key::parse(pos[1]) {
                Ok(k) => k,
                Err(e) => return fail(e),
            };
            let cfg = match Config::load(&d) {
                Ok(c) => c,
                Err(e) => return fail(e),
            };
            match cfg.get(key) {
                Some(v) => {
                    println!("{v}");
                    0
                }
                // Unset is not empty-string. Exit 2 so a script can tell the
                // difference without parsing stdout.
                None => {
                    eprintln!("{} is not set", key.as_str());
                    2
                }
            }
        }
        Some("show") | None => {
            let cfg = match Config::load(&d) {
                Ok(c) => c,
                Err(e) => return fail(e),
            };
            let rendered = cfg.render();
            if rendered.is_empty() {
                println!(
                    "nothing configured. `canon config set endpoint http://localhost:8080/v1`"
                );
            } else {
                print!("{rendered}");
            }
            0
        }
        Some(other) => fail(format!(
            "unknown config command `{other}` — expected set, get, or show"
        )),
    }
}
