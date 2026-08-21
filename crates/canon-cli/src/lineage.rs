// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lineages: sharing, adopting, and how far you have drifted from your seed.
//!
//! **A lineage is a git repository** holding `acts.jsonl`, and generations
//! are its tags. Hosting, ancestry, review and distribution come free, and
//! this module builds only the three things git does not give:
//!
//! 1. **A merge driver** — two people append on two machines and git sees a
//!    textual conflict where semantically there is none. The driver unions,
//!    dedupes by act id and sorts by time. Exact rather than heuristic,
//!    because ids are content hashes.
//! 2. **Ancestry in the log, not only in git** — `adopt` is an ACT, and
//!    inherited commitments carry `from` the seed's id. So `diff --upstream`
//!    works on a file that arrived with no `.git` at all.
//! 3. **`diff --upstream`** — pure computation over two sets of commitments.
//!
//! **Git is never load-bearing for correctness.** Delete `.git` and every
//! answer here is identical. Only `adopt <url>` and `upgrade` need the
//! network, and both have a paste equivalent — which is how most communities
//! will always share, not a phase before something better.

use std::io::Read;
use std::path::Path;
use std::process::Command;

use canon_core::{ActKind, Divergence, Fate, Log, Snapshot};

use crate::cmds::{fail, has, positionals};
use crate::profile::Profile;
use crate::store;

pub const UPSTREAM_DIR: &str = "upstream";
pub const SEED_FILE: &str = "seed.json";
/// Where a cloned lineage lives. Inside `.canon` on purpose: exit is still
/// deleting one directory.
pub const CACHE_DIR: &str = "lineages";

// ── share ───────────────────────────────────────────────────

/// The name this canon travels under. A file the holder can set, else the
/// directory holding the canon — so a pasted block says where it came from
/// without anyone configuring anything.
fn name_of(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("name"))
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            dir.parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "canon".into())
}

pub fn share(_args: &[String]) -> i32 {
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let profile = match Profile::load(&dir) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    if canon.active().next().is_none() {
        return fail("nothing to share — this canon has no live commitments");
    }
    let now = store::now();
    let snap = Snapshot::of(&canon, name_of(&dir), profile.as_str(), now);
    print!("{}", snap.render(&store::ymd(now)));
    0
}

// ── adopt ───────────────────────────────────────────────────

pub fn adopt(args: &[String]) -> i32 {
    let dir = match crate::cmds::dir() {
        Ok(d) => d,
        Err(e) => return fail(e),
    };
    let (_, canon) = match store::read(&dir).map(|l| (l.len(), l.derive())) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if let Some(a) = &canon.ancestry {
        return fail(format!(
            "this canon already descends from {}@{} — `canon upgrade <gen>` takes a newer \
             generation of it",
            a.lineage, a.generation
        ));
    }

    let pos = positionals(args);
    let (snapshot, source) = if has(args, "--paste") {
        let mut block = String::new();
        if let Err(e) = std::io::stdin().read_to_string(&mut block) {
            return fail(format!("reading the pasted block: {e}"));
        }
        match Snapshot::parse(&block) {
            Ok(s) => (s, "paste".to_string()),
            Err(e) => return fail(e),
        }
    } else if let Some(target) = pos.first() {
        let (url, gen) = split_generation(target);
        match fetch(&dir, url, gen) {
            Ok((s, _)) => (s, url.to_string()),
            Err(e) => return fail(e),
        }
    } else {
        return fail("usage: canon adopt <url>[@generation]  |  canon adopt --paste");
    };

    match write_adoption(&dir, &snapshot, &source) {
        Ok(n) => {
            println!(
                "adopted {}@{} — {n} commitment(s), each carrying where it came from",
                snapshot.lineage, snapshot.generation
            );
            println!("  canon diff --upstream   how you have diverged, later");
            0
        }
        Err(e) => fail(e),
    }
}

/// `url@generation` — the generation is optional and is a git tag.
fn split_generation(target: &str) -> (&str, Option<&str>) {
    // rsplit so an `@` in a scp-style git URL (`git@host:path`) is not
    // mistaken for a generation.
    match target.rsplit_once('@') {
        Some((url, gen)) if !url.is_empty() && !gen.contains('/') && !gen.contains(':') => {
            (url, Some(gen))
        }
        _ => (target, None),
    }
}

/// A cache directory name for a lineage URL: readable, and unambiguous.
///
/// The readable half is lossy — `a.b` and `a-b` flatten to the same thing —
/// so it carries a digest of the URL. Without it two lineages could share a
/// cache directory and one would silently serve the other's rules, which is
/// the worst failure this tool has.
fn slug(url: &str) -> String {
    let readable: String = url
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    format!("{readable}-{}", canon_core::short_digest(url))
}

fn git(args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut cmd = Command::new("git");
    cmd.args(args);
    if let Some(d) = cwd {
        cmd.current_dir(d);
    }
    let out = cmd
        .output()
        .map_err(|e| format!("running git {}: {e}", args.join(" ")))?;
    if !out.status.success() {
        // Some git failures report on stdout and some on stderr, and a git
        // error with no message attached is undiagnosable. Carry both.
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if err.is_empty() {
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        } else {
            err
        };
        if msg.contains("would clobber existing tag") {
            return Err(format!(
                "upstream moved a tag this canon has already seen.\n                   A generation names a specific set of rules; moving one changes what \
                 everybody who adopted it is holding, without them knowing.\n                   Ask the maintainer to publish a new generation, or delete the cached \
                 lineage to accept the change knowingly.\n  git said: {msg}"
            ));
        }
        return Err(format!("git {} failed: {msg}", args.join(" ")));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

/// Clone or refresh a lineage and read what is live at `generation`.
///
/// The adopter never types git: this is the whole reason it is here.
fn fetch(
    dir: &Path,
    url: &str,
    generation: Option<&str>,
) -> Result<(Snapshot, canon_core::Canon), String> {
    let cache = dir.join(CACHE_DIR).join(slug(url));
    if cache.join(".git").is_dir() {
        // Not --quiet: a rejected tag update is reported on stderr and is
        // exactly the thing worth seeing.
        git(&["fetch", "--tags", "origin"], Some(&cache))?;
    } else {
        std::fs::create_dir_all(cache.parent().unwrap_or(&cache))
            .map_err(|e| format!("creating the lineage cache: {e}"))?;
        git(&["clone", "--quiet", url, &cache.to_string_lossy()], None)?;
    }
    if let Some(g) = generation {
        git(&["checkout", "--quiet", g], Some(&cache)).map_err(|e| {
            format!(
                "{e}\n  generations are tags; `git -C {} tag` lists them",
                cache.display()
            )
        })?;
    }
    // A lineage repository publishes `acts.jsonl` at its root; a project
    // repository carries the same file inside `.canon/`. Both shapes are real
    // and a maintainer should not have to restructure to publish, so look in
    // both — and SAY which one was read, because "it adopted something, but
    // what?" is the confusing failure.
    let root = cache.join(store::FILE);
    let nested = cache.join(store::DIR).join(store::FILE);
    let (file, home) = if root.is_file() {
        (root, cache.clone())
    } else if nested.is_file() {
        (nested, cache.join(store::DIR))
    } else {
        return Err(format!(
            "{url} does not look like a lineage: no {} at its root and none in {}/",
            store::FILE,
            store::DIR
        ));
    };
    eprintln!("reading {}", file.display());
    let raw = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
    let upstream = Log::parse(&raw)
        .map_err(|e| format!("{}: {e}", file.display()))?
        .derive();
    if upstream.active().next().is_none() {
        return Err(format!("{url} has no live commitments at that generation"));
    }
    let profile = Profile::load(&home)?;
    // The lineage's own name if it published one, else the repository name.
    let lineage = std::fs::read_to_string(home.join("name"))
        .map(|s| s.trim().to_string())
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            url.trim_end_matches('/')
                .trim_end_matches(".git")
                .rsplit('/')
                .next()
                .unwrap_or(url)
                .to_string()
        });
    let snapshot = Snapshot::of(&upstream, lineage, profile.as_str(), store::now());
    // The derived canon comes back too, because `upgrade` needs the
    // supersession STRUCTURE upstream: a snapshot carries what is live, and
    // cannot say that a new rule replaced an old one rather than being new.
    Ok((snapshot, upstream))
}

/// Record the adoption: one `Adopt` act, then one inherited `Assert` per
/// commitment, then the seed for later diffing.
fn write_adoption(dir: &Path, snapshot: &Snapshot, source: &str) -> Result<usize, String> {
    crate::cmds::write(
        dir,
        ActKind::Adopt {
            lineage: snapshot.lineage.clone(),
            generation: snapshot.generation.clone(),
            source: Some(source.to_string()),
        },
    )?;
    for c in &snapshot.commitments {
        crate::cmds::write(
            dir,
            ActKind::Assert {
                text: c.text.clone(),
                // The link a divergence is computed from. Not position, not
                // text — either would break the moment someone rewords a rule
                // in place, and text matching would call it a different rule.
                from: Some(c.id.clone()),
                source: None,
            },
        )?;
    }
    save_seed(dir, snapshot)?;
    Ok(snapshot.commitments.len())
}

pub fn save_seed(dir: &Path, snapshot: &Snapshot) -> Result<(), String> {
    let up = dir.join(UPSTREAM_DIR);
    std::fs::create_dir_all(&up).map_err(|e| format!("creating {}: {e}", up.display()))?;
    let body = serde_json::to_string_pretty(snapshot).map_err(|e| e.to_string())?;
    std::fs::write(up.join(SEED_FILE), body)
        .map_err(|e| format!("writing {}: {e}", up.join(SEED_FILE).display()))
}

pub fn load_seed(dir: &Path) -> Result<Snapshot, String> {
    let path = dir.join(UPSTREAM_DIR).join(SEED_FILE);
    let raw = std::fs::read_to_string(&path).map_err(|_| {
        "this canon has no seed — nothing was adopted, so there is nothing to diff against"
            .to_string()
    })?;
    serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", path.display()))
}

// ── diff --upstream ─────────────────────────────────────────

pub fn diff(args: &[String]) -> i32 {
    if !has(args, "--upstream") {
        return fail("usage: canon diff --upstream [--propose] [--json]");
    }
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let seed = match load_seed(&dir) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let d = Divergence::compute(&seed, &canon);
    if has(args, "--json") {
        println!("{}", serde_json::to_string_pretty(&d).unwrap_or_default());
        return 0;
    }
    if has(args, "--propose") {
        print!("{}", render_proposal(&d, &canon));
    } else {
        print!("{}", render_divergence(&d, &canon, &seed));
    }
    0
}

fn render_divergence(d: &Divergence, canon: &canon_core::Canon, seed: &Snapshot) -> String {
    let sup = d.count(|f| matches!(f, Fate::Superseded { .. }));
    let ret = d.count(|f| matches!(f, Fate::Retracted));
    let acc = d.count(|f| matches!(f, Fate::Accepted { .. }));
    let unt = d.count(|f| matches!(f, Fate::Untouched));
    let nev = d.count(|f| matches!(f, Fate::Never));

    let mut out = format!(
        "adopted  {}@{}{}\n",
        d.lineage,
        d.generation,
        if seed.at > 0 {
            format!("  ({})", store::ymd(seed.at))
        } else {
            String::new()
        }
    );
    out.push_str(&format!(
        "SUPERSEDED ({sup}) · RETRACTED ({ret}) · ACCEPTED ({acc}) · ADDED ({}) · UNTOUCHED ({unt})\n",
        d.added.len()
    ));
    if nev > 0 {
        // Reported, never folded into "untouched": a rule that never landed
        // is a hole in the adoption, not a rule you kept.
        out.push_str(&format!(
            "NEVER LANDED ({nev}) — the seed had these and this canon does not\n"
        ));
    }

    for i in &d.inherited {
        match &i.fate {
            Fate::Untouched => continue,
            Fate::Superseded { text, .. } => {
                out.push_str(&format!(
                    "\nSUPERSEDED  {}\n  was: \"{}\"\n  now: \"{}\"\n",
                    i.upstream, i.text, text
                ));
            }
            Fate::Retracted => {
                out.push_str(&format!(
                    "\nRETRACTED   {}\n  was: \"{}\"\n",
                    i.upstream, i.text
                ));
            }
            Fate::Accepted { rationale } => {
                out.push_str(&format!(
                    "\nACCEPTED    {}\n  \"{}\"\n  carried knowingly: {rationale}\n",
                    i.upstream, i.text
                ));
            }
            Fate::Never => {
                out.push_str(&format!(
                    "\nNEVER LANDED {}\n  \"{}\"\n",
                    i.upstream, i.text
                ));
            }
        }
    }
    for id in &d.added {
        if let Some(c) = canon.get(id) {
            out.push_str(&format!("\nADDED       {}\n  \"{}\"\n", c.id, c.text));
        }
    }
    out
}

/// Upstream the SHAPE, not the rationale.
///
/// What changed and in which direction, with no `-m` text anywhere.
/// Rationales name incidents and people; sending one is a separate,
/// deliberate act by the person who wrote it.
fn render_proposal(d: &Divergence, canon: &canon_core::Canon) -> String {
    let mut out = format!("--- canon proposal · from {}@{}\n", d.lineage, d.generation);
    let mut n = 0;
    for i in &d.inherited {
        match &i.fate {
            Fate::Superseded { text, .. } => {
                n += 1;
                out.push_str(&format!(
                    "SUPERSEDE {}\n  was: \"{}\"\n  now: \"{}\"\n",
                    i.upstream, i.text, text
                ));
            }
            Fate::Retracted => {
                n += 1;
                out.push_str(&format!("RETRACT {}\n  was: \"{}\"\n", i.upstream, i.text));
            }
            // A tolerated contradiction is reported as a shape too: upstream
            // learns the pair is hard to hold, without learning why this
            // household decided to hold it.
            Fate::Accepted { .. } => {
                n += 1;
                out.push_str(&format!("TENSION {}\n  \"{}\"\n", i.upstream, i.text));
            }
            Fate::Untouched | Fate::Never => {}
        }
    }
    for id in &d.added {
        if let Some(c) = canon.get(id) {
            n += 1;
            out.push_str(&format!("ADD\n  \"{}\"\n", c.text));
        }
    }
    out.push_str(&format!("--- {n} change(s) · no rationales included\n"));
    out
}

// ── upgrade ─────────────────────────────────────────────────

pub fn upgrade(args: &[String]) -> i32 {
    let pos = positionals(args);
    let Some(generation) = pos.first() else {
        return fail("usage: canon upgrade <generation>");
    };
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    let Some(ancestry) = canon.ancestry.clone() else {
        return fail("this canon was not adopted from a lineage — nothing to upgrade");
    };
    let Some(url) = ancestry.source.clone().filter(|s| s != "paste") else {
        return fail(
            "this canon was adopted from a pasted block, which has no upstream to fetch. \
             Ask for a fresh block and `canon adopt --paste` into a new canon",
        );
    };
    let seed = match load_seed(&dir) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    let (next, upstream) = match fetch(&dir, &url, Some(generation)) {
        Ok(v) => v,
        Err(e) => return fail(e),
    };
    if next.generation == seed.generation {
        println!(
            "already on {}@{} — nothing upstream changed",
            seed.lineage, seed.generation
        );
        return 0;
    }

    let d = Divergence::compute(&seed, &canon);
    let fate_of = |upstream_id: &canon_core::ActId| -> Option<&Fate> {
        d.inherited
            .iter()
            .find(|i| &i.upstream == upstream_id)
            .map(|i| &i.fate)
    };
    let local_of = |upstream_id: &canon_core::ActId| {
        canon
            .commitments
            .iter()
            .find(|c| c.from.as_ref() == Some(upstream_id))
    };

    let mut applied = 0;
    let mut held: Vec<String> = Vec::new();

    for c in &next.commitments {
        match seed_ancestor(&upstream, &seed, &c.id) {
            // Upstream has not touched this one since the seed.
            Some(ref anc) if anc == &c.id => {}
            // Upstream evolved a rule this canon inherited. Follow only where
            // this canon left it alone; where it did not, nothing happens and
            // the divergence is named. Auto-resolving here would discard
            // local law without anyone deciding to.
            Some(anc) => match (fate_of(&anc), local_of(&anc)) {
                (Some(Fate::Untouched), Some(local)) => {
                    if let Err(e) = crate::cmds::write(
                        &dir,
                        ActKind::Supersede {
                            text: c.text.clone(),
                            old: vec![local.id.clone()],
                            rationale: format!("upstream {}@{generation}", next.lineage),
                        },
                    ) {
                        return fail(e);
                    }
                    applied += 1;
                }
                (Some(other), _) => held.push(format!(
                    "  {anc}  \"{}\"\n    upstream now says: \"{}\"\n    here it is {}",
                    seed_text(&seed, &anc),
                    c.text,
                    fate_phrase(other)
                )),
                _ => {}
            },
            // Genuinely new upstream law. Nothing local to conflict with, so
            // this is not a decision anyone has to make.
            None => {
                if let Err(e) = crate::cmds::write(
                    &dir,
                    ActKind::Assert {
                        text: c.text.clone(),
                        from: Some(c.id.clone()),
                        source: None,
                    },
                ) {
                    return fail(e);
                }
                applied += 1;
            }
        }
    }

    // Upstream withdrew a rule outright.
    for sc in &seed.commitments {
        let still_there = next.commitments.iter().any(|c| c.id == sc.id)
            || next
                .commitments
                .iter()
                .any(|c| seed_ancestor(&upstream, &seed, &c.id).as_ref() == Some(&sc.id));
        if still_there {
            continue;
        }
        match (fate_of(&sc.id), local_of(&sc.id)) {
            (Some(Fate::Untouched), Some(local)) => {
                if let Err(e) = crate::cmds::write(
                    &dir,
                    ActKind::Retract {
                        target: local.id.clone(),
                        rationale: format!("withdrawn upstream in {}@{generation}", next.lineage),
                    },
                ) {
                    return fail(e);
                }
                applied += 1;
            }
            (Some(other), _) => held.push(format!(
                "  {}  \"{}\"\n    upstream withdrew it\n    here it is {}",
                sc.id,
                sc.text,
                fate_phrase(other)
            )),
            _ => {}
        }
    }

    if let Err(e) = crate::cmds::write(
        &dir,
        ActKind::Adopt {
            lineage: next.lineage.clone(),
            generation: next.generation.clone(),
            source: Some(url.clone()),
        },
    ) {
        return fail(e);
    }
    if let Err(e) = save_seed(&dir, &next) {
        return fail(e);
    }

    println!(
        "upgraded {} from {} to {} — {applied} change(s) applied",
        next.lineage, seed.generation, next.generation
    );
    if !held.is_empty() {
        // Nothing auto-resolves. Naming it is the contribution.
        println!(
            "\n{} upstream change(s) were NOT applied, because this canon had already changed them:\n",
            held.len()
        );
        for h in &held {
            println!("{h}\n");
        }
        println!(
            "  `canon rebase --onto {}@{generation}` proposes how to carry your law onto the new base.",
            url
        );
    }
    0
}

fn seed_text(seed: &Snapshot, id: &canon_core::ActId) -> String {
    seed.commitments
        .iter()
        .find(|c| &c.id == id)
        .map(|c| c.text.clone())
        .unwrap_or_default()
}

fn fate_phrase(f: &Fate) -> &'static str {
    match f {
        Fate::Untouched => "unchanged",
        Fate::Superseded { .. } => "already superseded locally",
        Fate::Retracted => "already retracted locally",
        Fate::Accepted { .. } => "carried knowingly against another rule",
        Fate::Never => "not present",
    }
}

/// Walk an upstream commitment back to the seed commitment it descends from.
///
/// Upstream supersession mints a new id, so "is this new law or a rewrite of
/// law I already hold?" cannot be answered from a snapshot — only from the
/// upstream log's `replaces` links. Getting this wrong leaves an adopter
/// holding both the old rule and its replacement, which is the one outcome
/// nobody wants.
fn seed_ancestor(
    upstream: &canon_core::Canon,
    seed: &Snapshot,
    id: &canon_core::ActId,
) -> Option<canon_core::ActId> {
    let mut frontier = vec![id.clone()];
    let mut seen: Vec<canon_core::ActId> = Vec::new();
    while let Some(cur) = frontier.pop() {
        if seen.contains(&cur) {
            continue;
        }
        if seed.commitments.iter().any(|s| s.id == cur) {
            return Some(cur);
        }
        seen.push(cur.clone());
        if let Some(c) = upstream.commitments.iter().find(|c| c.id == cur) {
            frontier.extend(c.replaces.iter().cloned());
        }
    }
    None
}

// ── merge driver ────────────────────────────────────────────

const MERGE_SETUP: &str = "\
canon merge-driver — union two divergent act logs.

Two people appending on two machines produce a textual conflict in git where
semantically there is none. This driver unions, dedupes by act id and sorts
by time. It is exact rather than heuristic, because ids are content hashes:
the same act from both sides collapses to one, and different acts both
survive.

Set it up once, in the repository holding your canon:

  git config merge.canon.name \"canon act log union\"
  git config merge.canon.driver 'canon merge-driver %O %A %B'
  echo '.canon/acts.jsonl merge=canon' >> .gitattributes

`.gitattributes` is committed, so everyone who clones gets the rule; the
`git config` lines are local and each person runs them once.
";

/// `%O %A %B` — base, ours, theirs. Git takes the merged result from `%A`.
pub fn merge_driver(args: &[String]) -> i32 {
    let paths = positionals(args);
    if paths.len() < 3 {
        print!("{MERGE_SETUP}");
        return if paths.is_empty() { 0 } else { 2 };
    }
    let mut acts = Vec::new();
    for p in &paths[..3] {
        // A missing side is empty, not an error: git passes an empty base for
        // a file added on both branches.
        let raw = std::fs::read_to_string(p).unwrap_or_default();
        match Log::parse(&raw) {
            Ok(log) => acts.extend(log.acts().iter().cloned()),
            Err(e) => {
                eprintln!("canon merge-driver: {p}: {e}");
                return 1;
            }
        }
    }
    let merged = Log::from_acts(acts);
    match std::fs::write(paths[1], merged.render()) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("canon merge-driver: writing {}: {e}", paths[1]);
            1
        }
    }
}

#[cfg(test)]
mod tests;
