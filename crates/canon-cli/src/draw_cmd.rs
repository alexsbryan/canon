// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon draw` — announce a lot, seal a secret, open it, read the panel.
//!
//! Four subcommands and only three of them write an act. **The draw itself is
//! a query**: nobody performs it, so there is nothing to perform badly, and
//! two people running `canon draw show` on the same file see the same panel.
//!
//! The threat model is in `PRIMITIVES.md` under Primitive 9. Read it before
//! changing anything here — the shape of these commands is downstream of it,
//! not the other way round.

use std::path::Path;

use canon_core::{ActId, ActKind, Scope};

use crate::cmds::{fail, flag, has, load, positionals, write};
use crate::store;

/// Where a sealed secret waits for its boundary.
///
/// **Beside the log, never in it** — a secret in the log is not a secret. The
/// directory carries a `.gitignore` of its own the first time it is written,
/// because a codebase canon is committed and a secret that travels with the
/// repository has been published to everyone who clones it.
fn secrets_dir(dir: &Path) -> Result<std::path::PathBuf, String> {
    let d = dir.join("secrets");
    std::fs::create_dir_all(&d).map_err(|e| format!("creating {}: {e}", d.display()))?;
    let ignore = d.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, "*\n").map_err(|e| format!("writing {}: {e}", ignore.display()))?;
    }
    Ok(d)
}

/// Thirty-two bytes nobody can predict.
///
/// **Refuses rather than substituting.** A weak secret here is a draw someone
/// else can compute in advance, and "it fell back to the clock" is exactly the
/// silent substitution that makes a lottery look fair when it is not (§18.3).
fn fresh_secret() -> Result<String, String> {
    use std::io::Read;
    // `read_exact`, not `read` — `/dev/urandom` is an endless stream and
    // `fs::read` on it does not return. Read exactly what is needed.
    let mut bytes = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut bytes))
        .map_err(|e| {
            format!(
                "no source of randomness on this machine ({e}) — refusing to seal a \
                 secret somebody could guess"
            )
        })?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}

pub(crate) fn resolve_draw(canon: &canon_core::Canon, needle: &str) -> Result<ActId, String> {
    let hits: Vec<&ActId> = canon
        .draws
        .iter()
        .map(|d| &d.act)
        .filter(|id| id.as_str().starts_with(needle))
        .collect();
    match hits.len() {
        1 => Ok(hits[0].clone()),
        0 => Err(format!(
            "no draw matching `{needle}` — `canon draw` lists them"
        )),
        n => Err(format!(
            "`{needle}` matches {n} draws — use more characters"
        )),
    }
}

pub fn run(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("commit") => commit(args, &pos),
        Some("seal") => seal(args, &pos),
        Some("open") => open(args, &pos),
        Some("show") => show(args, &pos),
        None => list(args),
        Some(other) => fail(format!(
            "unknown draw command `{other}` — expected commit, seal, open, or show"
        )),
    }
}

fn commit(args: &[String], pos: &[&str]) -> i32 {
    if pos.len() < 3 {
        return fail(
            "usage: canon draw commit <scope> <seats> --after <YYYY-MM-DD> [-m \"<why>\"]",
        );
    }
    let Some(scope) = Scope::new(pos[1]) else {
        return fail(format!("`{}` is not a scope", pos[1]));
    };
    let Ok(count) = pos[2].parse::<usize>() else {
        return fail(format!("`{}` is not a number of seats", pos[2]));
    };
    let Some(raw) = flag(args, "--after") else {
        return fail("draw commit needs --after <YYYY-MM-DD> — the boundary");
    };
    let Some(after_ts) = canon_core::date::parse_ymd(raw) else {
        return fail(format!("`{raw}` is not a date — YYYY-MM-DD"));
    };
    let now = store::now();
    // Refused HERE as well as in the query, because a commit that can never
    // be drawn is a trap: it collects everybody's secrets and then declines.
    if after_ts <= now {
        return fail(format!(
            "the boundary {raw} is not in the future — a draw whose moment was chosen \
             after the fact is not a draw"
        ));
    }
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let pool = canon.who_decides(&scope, after_ts).len();
    match write(
        &d,
        ActKind::DrawCommit {
            scope: scope.clone(),
            count,
            after_ts,
            rationale: flag(args, "-m").unwrap_or_default().to_string(),
        },
    ) {
        Ok(act) => {
            println!("{}  {count} seat(s) from {scope}, after {raw}", act.id);
            println!("  everyone in the pool seals a secret BEFORE {raw}:");
            println!("    canon draw seal {}", act.id);
            println!("  and opens it after:");
            println!("    canon draw open {}", act.id);
            // Absence reported at the moment it can still be fixed.
            if pool == 0 {
                eprintln!("\nwarning: nobody holds standing over `{scope}` yet — as it stands this draw has no pool.");
            } else if count >= pool {
                eprintln!("\nwarning: {count} seat(s) from a pool of {pool} would select everyone, which is not a draw.");
            }
            0
        }
        Err(e) => fail(e),
    }
}

fn seal(_args: &[String], pos: &[&str]) -> i32 {
    let Some(needle) = pos.get(1) else {
        return fail("usage: canon draw seal <draw-id>");
    };
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let commit = match resolve_draw(&canon, needle) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let announced = canon
        .draws
        .iter()
        .find(|x| x.act == commit)
        .expect("resolved");
    let now = store::now();
    if now >= announced.after_ts {
        return fail(format!(
            "the boundary ({}) has passed — a secret sealed now commits to nothing",
            store::ymd(announced.after_ts)
        ));
    }
    let actor = store::actor();
    if canon
        .sealed
        .iter()
        .any(|s| s.commit == commit && s.actor == actor)
    {
        return fail(
            "you have already sealed a secret for this draw — the first one is the one \
             that counts, and a second would be a way to open whichever flatters you",
        );
    }
    let secret = match fresh_secret() {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let dir = match secrets_dir(&d) {
        Ok(x) => x,
        Err(e) => return fail(e),
    };
    let path = dir.join(commit.as_str());
    if let Err(e) = std::fs::write(&path, format!("{secret}\n")) {
        return fail(format!("writing {}: {e}", path.display()));
    }
    match write(
        &d,
        ActKind::DrawSecret {
            commit: commit.clone(),
            digest: canon_core::id::digest_hex(secret.as_bytes()),
        },
    ) {
        Ok(_) => {
            println!("sealed for {commit}");
            println!(
                "  your secret is in {} and is not in the log",
                path.display()
            );
            println!(
                "  open it after {}:  canon draw open {commit}",
                store::ymd(announced.after_ts)
            );
            0
        }
        Err(e) => fail(e),
    }
}

fn open(args: &[String], pos: &[&str]) -> i32 {
    let Some(needle) = pos.get(1) else {
        return fail("usage: canon draw open <draw-id>");
    };
    let (d, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let commit = match resolve_draw(&canon, needle) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let announced = canon
        .draws
        .iter()
        .find(|x| x.act == commit)
        .expect("resolved");
    if store::now() < announced.after_ts {
        return fail(format!(
            "the boundary is {} — opening now would hand your secret to anyone who has \
             not sealed yet",
            store::ymd(announced.after_ts)
        ));
    }
    // The secret may be supplied by hand, for the case where it was sealed on
    // another machine. Never generated here: what is opened must be what was
    // sealed, and inventing one would fail the digest check anyway.
    let secret = match flag(args, "--secret") {
        Some(s) => s.to_string(),
        None => {
            let path = match secrets_dir(&d) {
                Ok(x) => x.join(commit.as_str()),
                Err(e) => return fail(e),
            };
            match std::fs::read_to_string(&path) {
                Ok(s) => s.trim().to_string(),
                Err(e) => {
                    return fail(format!(
                        "no sealed secret at {} ({e}) — pass it with --secret if you \
                         sealed it elsewhere",
                        path.display()
                    ))
                }
            }
        }
    };
    match write(
        &d,
        ActKind::DrawReveal {
            commit: commit.clone(),
            secret,
        },
    ) {
        Ok(_) => {
            println!("opened for {commit}");
            println!("  read the panel:  canon draw show {commit}");
            0
        }
        Err(e) => fail(e),
    }
}

fn show(args: &[String], pos: &[&str]) -> i32 {
    let Some(needle) = pos.get(1) else {
        return fail("usage: canon draw show <draw-id>");
    };
    let (_, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let commit = match resolve_draw(&canon, needle) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    match canon.draw(&commit) {
        Ok(drawn) => {
            if has(args, "--json") {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&drawn).unwrap_or_default()
                );
                return 0;
            }
            for seat in &drawn.seats {
                println!("{seat}");
            }
            println!(
                "\n{} of {} drawn, seed {}",
                drawn.seats.len(),
                drawn.pool.len(),
                &drawn.seed[..16]
            );
            println!("  from {} opened secret(s)", drawn.contributed.len());
            if !drawn.withheld.is_empty() {
                // Named, not buried: sealing and staying silent is the one
                // influence this scheme leaves open, and it is visible.
                println!(
                    "  {} sealed and never opened, so they are out of the pool: {}",
                    drawn.withheld.len(),
                    drawn.withheld.join(", ")
                );
            }
            0
        }
        Err(e) => {
            // A refusal is the answer, not a failure to answer. Exit 2 so a
            // script can tell "not drawable" from "drawn".
            eprintln!("cannot draw: {e}");
            2
        }
    }
}

fn list(args: &[String]) -> i32 {
    let (_, _, canon) = match load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if has(args, "--json") {
        println!(
            "{}",
            serde_json::to_string_pretty(canon.draws_announced()).unwrap_or_default()
        );
        return 0;
    }
    if canon.draws.is_empty() {
        println!("no draws announced. `canon draw commit <scope> <seats> --after <date>`");
        return 0;
    }
    let now = store::now();
    for d in canon.draws_announced() {
        let sealed = canon.sealed.iter().filter(|s| s.commit == d.act).count();
        let opened = canon.opened.iter().filter(|o| o.commit == d.act).count();
        let state = if now < d.after_ts {
            format!("sealing until {}", store::ymd(d.after_ts))
        } else {
            match canon.draw(&d.act) {
                Ok(_) => "drawn".to_string(),
                Err(e) => format!("cannot draw: {e}"),
            }
        };
        println!(
            "{}  {} seat(s) from {}  [{sealed} sealed, {opened} opened]  {state}",
            d.act, d.count, d.scope
        );
    }
    0
}
