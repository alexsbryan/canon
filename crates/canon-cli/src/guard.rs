// SPDX-License-Identifier: AGPL-3.0-or-later
//! `witness` and `guard` — what defends the ledger, from outside the fold.
//!
//! The fold is a pure function of the log, and that is the property
//! everything else rests on: replay, union merge, no server, no accounts. It
//! also means the fold takes two things as given that it cannot check —
//! `ts_unix` and `actor` are strings somebody wrote — and so whoever can
//! append to `acts.jsonl` writes the derived state. Two hand-written lines, a
//! self-grant backdated to before the founder's and a `ratification standing`
//! after it, unseat a house with every rule satisfied. The rules are not
//! broken. The attacker is a non-standard model of them.
//!
//! No rule inside the fold closes that, and none is attempted here. What
//! defends the file is whoever may append to it: git in a repository, the
//! household on a shared folder. Neither is canon's. This module is the
//! second-order tool for the first case — the ledger checked against a
//! witness it does not govern — and it lives outside the format, outside
//! the op census and outside the ledger, because a guard recorded inside
//! the thing it guards is removed by the rewrite it exists to catch.
//!
//! **Opt in.** A canon with no guard trusts its log and `guard show` says so
//! in one line. Nothing here runs unless somebody asked for it.
//!
//! **Two modes, one check.** [`witness_pair`] compares a parent file with a
//! child file: every parent act still present, no added act claiming a time
//! earlier than what the parent already held (beyond a slack), none claiming
//! a time after it was committed. History mode applies it to every commit
//! that touched the ledger; gate mode applies it once, base against working
//! tree, and that is what a hook and a CI step run.
//!
//! **What it cannot see** is printed every time it runs, because an
//! unstated residual is an undefended one: an act backdated by less than the
//! slack; a rewrite of git history itself; whoever holds the remote; and who
//! typed the actor string, which is what signed commits and code-owner
//! review are for.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;

use crate::cmds::{fail, flag, has, positionals};
use crate::store;

/// A day. The default slack: an offline append that lands a few hours after
/// somebody else's is honest, and a self-grant backdated to before the
/// founding is not a few hours.
const DEFAULT_SLACK: i64 = 86_400;

/// Something the witness saw between two versions of the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "finding", rename_all = "snake_case")]
pub enum Finding {
    /// An act the parent held and the child does not. An edited line shows
    /// up as removed plus added, because ids are content-derived.
    Removed { id: String },
    /// An added act claiming a time earlier than the newest act the parent
    /// already held, by more than the slack.
    Backdated {
        id: String,
        claims: i64,
        latest_before: i64,
    },
    /// An added act claiming a time later than it was committed, by more
    /// than the slack.
    FutureDated {
        id: String,
        claims: i64,
        committed: i64,
    },
    /// A line the reader could not parse at all.
    Unreadable { line: usize },
}

struct Line {
    id: String,
    ts: i64,
}

fn parse(text: &str) -> (Vec<Line>, Vec<usize>) {
    let mut ok = Vec::new();
    let mut bad = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        if raw.trim().is_empty() {
            continue;
        }
        let parsed = serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|v| {
                Some(Line {
                    id: v.get("id")?.as_str()?.to_string(),
                    ts: v.get("ts_unix")?.as_i64()?,
                })
            });
        match parsed {
            Some(l) => ok.push(l),
            None => bad.push(i + 1),
        }
    }
    (ok, bad)
}

/// The one check. Pure: two file contents, when the child was committed,
/// and the slack.
pub fn witness_pair(parent: &str, child: &str, committed_at: i64, slack: i64) -> Vec<Finding> {
    let (before, _) = parse(parent);
    let (after, unreadable) = parse(child);
    let mut out: Vec<Finding> = unreadable
        .into_iter()
        .map(|line| Finding::Unreadable { line })
        .collect();
    let latest_before = before.iter().map(|l| l.ts).max();
    let kept: std::collections::BTreeSet<&str> = after.iter().map(|l| l.id.as_str()).collect();
    for l in &before {
        if !kept.contains(l.id.as_str()) {
            out.push(Finding::Removed { id: l.id.clone() });
        }
    }
    let had: std::collections::BTreeSet<&str> = before.iter().map(|l| l.id.as_str()).collect();
    for l in after.iter().filter(|l| !had.contains(l.id.as_str())) {
        if let Some(latest) = latest_before {
            if l.ts < latest - slack {
                out.push(Finding::Backdated {
                    id: l.id.clone(),
                    claims: l.ts,
                    latest_before: latest,
                });
            }
        }
        if l.ts > committed_at + slack {
            out.push(Finding::FutureDated {
                id: l.id.clone(),
                claims: l.ts,
                committed: committed_at,
            });
        }
    }
    out
}

fn humanize(secs: i64) -> String {
    let s = secs.abs();
    if s >= 86_400 {
        format!("{}d", s / 86_400)
    } else if s >= 3_600 {
        format!("{}h", s / 3_600)
    } else if s >= 60 {
        format!("{}m", s / 60)
    } else {
        format!("{s}s")
    }
}

fn describe(f: &Finding) -> String {
    match f {
        Finding::Removed { id } => format!("removed     {id}  held before, gone here"),
        Finding::Backdated {
            id,
            claims,
            latest_before,
        } => format!(
            "backdated   {id}  claims {}, {} earlier than the newest act already held ({})",
            store::ymd(*claims),
            humanize(latest_before - claims),
            store::ymd(*latest_before)
        ),
        Finding::FutureDated {
            id,
            claims,
            committed,
        } => format!(
            "future      {id}  claims {}, {} after it was committed ({})",
            store::ymd(*claims),
            humanize(claims - committed),
            store::ymd(*committed)
        ),
        Finding::Unreadable { line } => format!("unreadable  line {line}"),
    }
}

// ── git ─────────────────────────────────────────────────────

struct Repo {
    root: PathBuf,
    /// The ledger's path inside the repository, forward slashes.
    ledger: String,
}

fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| format!("running git {}: {e}", args.join(" ")))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if err.is_empty() {
            format!("git {} failed", args.join(" "))
        } else {
            err
        });
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
}

fn repo_for(dir: &Path) -> Option<Repo> {
    let root = git(dir, &["rev-parse", "--show-toplevel"]).ok()?;
    let root = PathBuf::from(root);
    let file = dir.join(store::FILE);
    let file = file.canonicalize().unwrap_or(file);
    let root_c = root.canonicalize().unwrap_or_else(|_| root.clone());
    let rel = file.strip_prefix(&root_c).ok()?;
    let ledger = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join("/");
    Some(Repo { root, ledger })
}

fn tracked(repo: &Repo) -> bool {
    git(
        &repo.root,
        &["ls-files", "--error-unmatch", "--", &repo.ledger],
    )
    .is_ok()
}

/// The file at a revision, or nothing if it was not there.
fn show(repo: &Repo, rev: &str) -> Option<String> {
    git(&repo.root, &["show", &format!("{rev}:{}", repo.ledger)]).ok()
}

struct Commit {
    hash: String,
    short: String,
    author: String,
    at: i64,
    /// `%G?`: G good, B bad, U unknown key, X expired, Y expired key,
    /// R revoked, E cannot check, N none.
    sig: String,
}

fn commits(repo: &Repo) -> Result<Vec<Commit>, String> {
    let raw = git(
        &repo.root,
        &[
            "log",
            "--reverse",
            "--format=%H%x1f%h%x1f%an%x1f%ct%x1f%G?",
            "--",
            &repo.ledger,
        ],
    )?;
    Ok(raw
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split('\x1f').collect();
            Some(Commit {
                hash: f.first()?.to_string(),
                short: f.get(1)?.to_string(),
                author: f.get(2)?.to_string(),
                at: f.get(3)?.parse().ok()?,
                sig: f
                    .get(4)
                    .map_or("N", |s| if s.is_empty() { "N" } else { s })
                    .to_string(),
            })
        })
        .collect())
}

/// Every commit that touched the ledger, checked against its parent.
struct History {
    commits: Vec<(Commit, Vec<Finding>)>,
    acts: usize,
}

fn history(repo: &Repo, slack: i64) -> Result<History, String> {
    let mut out = Vec::new();
    for c in commits(repo)? {
        let child = show(repo, &c.hash).unwrap_or_default();
        let parent = show(repo, &format!("{}^", c.hash)).unwrap_or_default();
        let findings = witness_pair(&parent, &child, c.at, slack);
        out.push((c, findings));
    }
    let acts = std::fs::read_to_string(repo.root.join(&repo.ledger))
        .map(|t| t.lines().filter(|l| !l.trim().is_empty()).count())
        .unwrap_or(0);
    Ok(History { commits: out, acts })
}

fn residual(slack: i64) -> String {
    format!(
        "not seen: an act backdated by less than {}; a rewrite of git history itself; \
         whoever holds the remote; who typed the actor string — signed commits and \
         code-owner review are for that",
        humanize(slack)
    )
}

fn slack_from(args: &[String]) -> Result<i64, String> {
    match flag(args, "--slack") {
        None => Ok(DEFAULT_SLACK),
        Some(raw) => crate::govern::every(raw),
    }
}

fn located() -> Result<(PathBuf, Repo), String> {
    let dir = crate::cmds::dir()?;
    let repo = repo_for(&dir).ok_or_else(|| {
        format!(
            "{} is not inside a git repository, so nothing outside the fold is holding it. \
             This canon trusts its log.",
            dir.display()
        )
    })?;
    Ok((dir, repo))
}

// ── canon witness ───────────────────────────────────────────

/// The ledger checked against git.
///
/// No `--base`: every commit that touched the ledger, against its parent —
/// what the pre-push hook runs, because a push carries commits and a bad
/// one is already in `main` locally by the time the hook fires. With
/// `--base <ref>`: that revision against the working tree, for a CI step
/// that knows what a pull request is against. `--gate` exits 1 on any
/// finding in either mode.
pub fn witness(args: &[String]) -> i32 {
    let (dir, repo) = match located() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if !tracked(&repo) {
        return fail(format!(
            "{} is not tracked by git; commit it before asking git to witness it",
            repo.ledger
        ));
    }
    let slack = match slack_from(args) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let gate = has(args, "--gate");
    if let Some(base) = flag(args, "--base") {
        let parent = show(&repo, base).unwrap_or_default();
        let child = std::fs::read_to_string(dir.join(store::FILE)).unwrap_or_default();
        let findings = witness_pair(&parent, &child, store::now(), slack);
        if has(args, "--json") {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "base": base,
                    "slack": slack,
                    "findings": findings,
                }))
                .unwrap_or_default()
            );
        } else {
            let added = parse(&child).0.len().saturating_sub(parse(&parent).0.len());
            println!(
                "witness: {} against {base}, slack {}",
                repo.ledger,
                humanize(slack)
            );
            if findings.is_empty() {
                println!(
                    "  {added} act(s) added, none removed, none backdated, none from the future"
                );
            }
            for f in &findings {
                println!("  {}", describe(f));
            }
            if !findings.is_empty() {
                println!(
                    "\n{} finding(s). An offline append can land a little late; a self-grant \
                     dated before the founding cannot. `--slack` widens what is forgiven.",
                    findings.len()
                );
            }
            println!("{}", crate::wrap::hang("  ", &residual(slack)));
        }
        return i32::from(gate && !findings.is_empty());
    }

    let h = match history(&repo, slack) {
        Ok(h) => h,
        Err(e) => return fail(e),
    };
    let total: usize = h.commits.iter().map(|(_, f)| f.len()).sum();
    if has(args, "--json") {
        let rows: Vec<serde_json::Value> = h
            .commits
            .iter()
            .map(|(c, f)| {
                serde_json::json!({
                    "commit": c.hash,
                    "author": c.author,
                    "at": c.at,
                    "signature": c.sig,
                    "findings": f,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ledger": repo.ledger,
                "slack": slack,
                "acts": h.acts,
                "commits": rows,
            }))
            .unwrap_or_default()
        );
        return 0;
    }
    let signed = h.commits.iter().filter(|(c, _)| c.sig == "G").count();
    if total == 0 {
        println!(
            "witness: {} commit(s), {} act(s), append-only, none backdated beyond {}; {signed} of {} \
             commit(s) signed",
            h.commits.len(),
            h.acts,
            humanize(slack),
            h.commits.len()
        );
    } else {
        println!(
            "witness: {} commit(s), {} act(s), {total} finding(s); {signed} of {} commit(s) signed",
            h.commits.len(),
            h.acts,
            h.commits.len()
        );
        for (c, f) in &h.commits {
            if f.is_empty() {
                continue;
            }
            println!(
                "\n  {}  {}  {}  signature {}",
                c.short,
                store::ymd(c.at),
                c.author,
                c.sig
            );
            for x in f {
                println!("    {}", describe(x));
            }
        }
    }
    println!("{}", crate::wrap::hang("  ", &residual(slack)));
    i32::from(has(args, "--gate") && total > 0)
}

// ── canon guard ─────────────────────────────────────────────

const HOOK: &str = "#!/usr/bin/env sh\n\
# Installed by `canon guard git`. The ledger leaves this machine append-only\n\
# and not backdated, or it does not leave. `canon witness` says what it saw.\n\
exec canon witness --gate\n";

const CI_SNIPPET: &str = concat!(
    "      - name: Ledger (append-only, not backdated)\n",
    "        run: canon witness --gate\n",
    "    # needs `fetch-depth: 0` on the checkout, so every commit is there to check.\n",
    "    # A repository whose older history is not clean gates the diff instead:\n",
    "    #   canon witness --gate --base origin/${{ github.base_ref || 'main' }}\n",
);

fn defends() -> &'static str {
    "defends against: a line removed or edited in the ledger; an act added with a time \
     earlier than the ledger already reached; an act dated in the future; a push that \
     rewrites main, once the branch is protected"
}

/// What guards this canon, by looking, and what each guard cannot see.
pub fn guard(args: &[String]) -> i32 {
    let pos = positionals(args);
    match pos.first().copied() {
        Some("git") => guard_git(args),
        Some("show") | None => guard_show(args),
        Some(other) => fail(format!("unknown guard `{other}` — expected git or show")),
    }
}

fn hook_path(repo: &Repo) -> Option<PathBuf> {
    let hooks = git(&repo.root, &["rev-parse", "--git-path", "hooks"]).ok()?;
    let p = PathBuf::from(&hooks);
    Some(
        if p.is_absolute() {
            p
        } else {
            repo.root.join(p)
        }
        .join("pre-push"),
    )
}

/// Is the check in the hook — or in a script the hook hands off to? A
/// repository that keeps its gate in `scripts/pre-push.sh` and a one-line
/// shim in the hook has the guard; the shim just does not spell it.
fn hook_installed(repo: &Repo) -> bool {
    let Some(text) = hook_path(repo).and_then(|p| std::fs::read_to_string(p).ok()) else {
        return false;
    };
    if text.contains("canon witness") {
        return true;
    }
    let root = repo.root.display().to_string();
    text.split_whitespace()
        .map(|t| {
            t.trim_matches(|c| c == '"' || c == '\'')
                .replace("$(git rev-parse --show-toplevel)", &root)
        })
        .filter(|t| t.ends_with(".sh"))
        .map(|t| {
            let p = PathBuf::from(&t);
            if p.is_absolute() {
                p
            } else {
                repo.root.join(p)
            }
        })
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .any(|s| s.contains("canon witness"))
}

fn gh(args: &[&str], root: &Path) -> Result<String, String> {
    let out = Command::new("gh")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|_| "`gh` is not installed".to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// `owner/name` and the default branch, from the remote through `gh`.
fn remote(repo: &Repo) -> Result<(String, String), String> {
    let raw = gh(
        &["repo", "view", "--json", "nameWithOwner,defaultBranchRef"],
        &repo.root,
    )?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let name = v
        .get("nameWithOwner")
        .and_then(|s| s.as_str())
        .ok_or("gh did not name the repository")?
        .to_string();
    let branch = v
        .pointer("/defaultBranchRef/name")
        .and_then(|s| s.as_str())
        .unwrap_or("main")
        .to_string();
    Ok((name, branch))
}

fn protection(repo: &Repo) -> Result<Option<bool>, String> {
    let (name, branch) = remote(repo)?;
    match gh(
        &["api", &format!("repos/{name}/branches/{branch}/protection")],
        &repo.root,
    ) {
        Ok(_) => Ok(Some(true)),
        Err(e) if e.contains("404") || e.contains("Branch not protected") => Ok(Some(false)),
        Err(e) => Err(e),
    }
}

fn guard_show(args: &[String]) -> i32 {
    let dir = match crate::cmds::dir() {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    let Some(repo) = repo_for(&dir) else {
        println!(
            "no guard: this canon trusts its log. `canon guard git` if it lives in a repository."
        );
        return 0;
    };
    let slack = match slack_from(args) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    println!("guard: git, at {}", repo.root.display());
    let yes = |b: bool| if b { "yes" } else { "no " };
    let is_tracked = tracked(&repo);
    println!("  ledger tracked      {}  {}", yes(is_tracked), repo.ledger);
    println!(
        "  pre-push hook       {}  {}",
        yes(hook_installed(&repo)),
        hook_path(&repo).map_or_else(String::new, |p| p.display().to_string())
    );
    match protection(&repo) {
        Ok(Some(true)) => println!("  branch protection   yes"),
        Ok(Some(false)) => println!("  branch protection   no   `canon guard git --apply` sets it"),
        Ok(None) => {}
        Err(e) => println!(
            "  branch protection   ?    {}",
            e.lines().next().unwrap_or("")
        ),
    }
    if !is_tracked {
        println!("\n  commit {} and git can start witnessing it", repo.ledger);
        return 0;
    }
    match history(&repo, slack) {
        Ok(h) => {
            let total: usize = h.commits.iter().map(|(_, f)| f.len()).sum();
            let signed = h.commits.iter().filter(|(c, _)| c.sig == "G").count();
            println!("  signed commits      {signed} of {}", h.commits.len());
            println!(
                "  history             {} commit(s), {} act(s), {}",
                h.commits.len(),
                h.acts,
                if total == 0 {
                    format!("append-only, none backdated beyond {}", humanize(slack))
                } else {
                    format!("{total} finding(s) — `canon witness` lists them")
                }
            );
        }
        Err(e) => println!("  history             ?    {e}"),
    }
    println!("\n{}", crate::wrap::hang("  ", defends()));
    println!("{}", crate::wrap::hang("  ", &residual(slack)));
    0
}

fn guard_git(args: &[String]) -> i32 {
    let (_, repo) = match located() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if !tracked(&repo) {
        return fail(format!(
            "{} is not tracked by git; commit it first, then git can witness it",
            repo.ledger
        ));
    }
    println!("guard git, at {}", repo.root.display());

    // 1. The hook. Into wherever this repository keeps hooks; never over
    //    one somebody already wrote.
    match hook_path(&repo) {
        None => println!("  hook: could not find this repository's hooks directory"),
        Some(p) => {
            if hook_installed(&repo) {
                println!("  hook: already installed at {}", p.display());
            } else if p.exists() {
                println!(
                    "  hook: {} exists and is not ours — add this line to it:\n    canon witness --gate || exit 1",
                    p.display()
                );
            } else {
                let write = std::fs::create_dir_all(p.parent().unwrap_or(&repo.root))
                    .and_then(|()| std::fs::write(&p, HOOK))
                    .and_then(|()| {
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755))
                        }
                        #[cfg(not(unix))]
                        {
                            Ok(())
                        }
                    });
                match write {
                    Ok(()) => println!("  hook: installed {}", p.display()),
                    Err(e) => println!("  hook: could not write {}: {e}", p.display()),
                }
            }
        }
    }

    // 2. CI. Printed, not written: every repository lays its workflow out
    //    differently, and a step pasted into the wrong job is worse than a
    //    step somebody placed.
    println!("\n  CI: add this step to the job that builds canon\n{CI_SNIPPET}");

    // 3. Branch protection. The settings, and with `--apply`, the settings.
    println!("  branch protection wanted on the default branch:");
    for s in [
        "pull requests only, one approving review, from a code owner",
        "required status check (the CI job above), up to date with the branch",
        "no force-push, no deletion, linear history",
        "signed commits required — an unsigned local setup is rejected until signing is on",
    ] {
        println!("    - {s}");
    }
    if has(args, "--apply") {
        match apply_protection(&repo, flag(args, "--check")) {
            Ok(msg) => println!("\n  applied: {msg}"),
            Err(e) => {
                println!("\n  not applied: {e}");
                println!("  the settings above are in Settings → Branches on the repository");
            }
        }
    } else {
        println!(
            "\n  `canon guard git --apply [--check <status check name>]` sets them through gh"
        );
    }

    println!("\n{}", crate::wrap::hang("  ", defends()));
    let slack = slack_from(args).unwrap_or(DEFAULT_SLACK);
    println!("{}", crate::wrap::hang("  ", &residual(slack)));
    0
}

fn apply_protection(repo: &Repo, check: Option<&str>) -> Result<String, String> {
    gh(&["auth", "status"], &repo.root).map_err(|_| "`gh` is not logged in — `gh auth login`")?;
    let (name, branch) = remote(repo)?;
    let contexts: Vec<&str> = check.into_iter().collect();
    let body = serde_json::json!({
        "required_status_checks": { "strict": true, "contexts": contexts },
        "enforce_admins": true,
        "required_pull_request_reviews": {
            "required_approving_review_count": 1,
            "require_code_owner_reviews": true,
            "dismiss_stale_reviews": true
        },
        "restrictions": null,
        "required_linear_history": true,
        "allow_force_pushes": false,
        "allow_deletions": false,
        "required_conversation_resolution": false
    });
    let mut cmd = Command::new("gh");
    cmd.args([
        "api",
        "-X",
        "PUT",
        &format!("repos/{name}/branches/{branch}/protection"),
        "--input",
        "-",
    ])
    .current_dir(&repo.root)
    .stdin(std::process::Stdio::piped())
    .stdout(std::process::Stdio::piped())
    .stderr(std::process::Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().ok_or("no stdin")?;
        stdin
            .write_all(body.to_string().as_bytes())
            .map_err(|e| e.to_string())?;
    }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    gh(
        &[
            "api",
            "-X",
            "POST",
            &format!("repos/{name}/branches/{branch}/protection/required_signatures"),
        ],
        &repo.root,
    )?;
    Ok(format!(
        "{name} {branch}: pull requests with a code-owner review, {} linear history, no \
         force-push, no deletion, signed commits",
        match check {
            Some(c) => format!("`{c}` required and current,"),
            None => "no status check named yet (`--check <name>` adds one),".into(),
        }
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: &str, ts: i64) -> String {
        format!(
            r#"{{"id":"{id}","v":2,"ts_unix":{ts},"actor":"human:x","op":"assert","text":"t"}}"#
        )
    }

    #[test]
    fn an_honest_append_is_clean_and_a_backdated_self_grant_is_not() {
        let parent = format!("{}\n{}\n", line("can-a", 1_000), line("can-b", 2_000));
        let child = format!("{parent}{}\n", line("can-c", 2_100));
        assert!(witness_pair(&parent, &child, 2_200, 60).is_empty());

        // The stranger's line: earlier than anything the ledger held.
        let child = format!("{parent}{}\n", line("can-s", 500));
        let f = witness_pair(&parent, &child, 2_200, 60);
        assert!(
            matches!(&f[..], [Finding::Backdated { id, .. }] if id == "can-s"),
            "{f:?}"
        );
    }

    #[test]
    fn a_late_offline_append_inside_the_slack_is_forgiven() {
        let parent = format!("{}\n", line("can-a", 10_000));
        let child = format!("{parent}{}\n", line("can-b", 9_500));
        assert!(witness_pair(&parent, &child, 10_100, 86_400).is_empty());
        assert_eq!(witness_pair(&parent, &child, 10_100, 60).len(), 1);
    }

    #[test]
    fn a_removed_or_edited_line_is_seen_and_so_is_the_future() {
        let parent = format!("{}\n{}\n", line("can-a", 1_000), line("can-b", 2_000));
        // Edit: the id changes with the content, so it reads as removed + added.
        let child = format!("{}\n{}\n", line("can-a", 1_000), line("can-b2", 2_000));
        let f = witness_pair(&parent, &child, 2_200, 60);
        assert!(f
            .iter()
            .any(|x| matches!(x, Finding::Removed { id } if id == "can-b")));
        let child = format!("{parent}{}\n", line("can-c", 9_000_000));
        let f = witness_pair(&parent, &child, 2_200, 60);
        assert!(matches!(&f[..], [Finding::FutureDated { .. }]), "{f:?}");
    }

    #[test]
    fn a_first_commit_has_nothing_to_be_earlier_than() {
        let child = format!("{}\n{}\n", line("can-a", 1_000), line("can-b", 500));
        assert!(witness_pair("", &child, 2_000, 60).is_empty());
    }
}
