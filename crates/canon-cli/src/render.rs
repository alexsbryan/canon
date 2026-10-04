// SPDX-License-Identifier: AGPL-3.0-or-later
//! The canon as a document, for a reader without the CLI.
//!
//! **Every canon read by agents outside the CLI was going to write this
//! again.** A build loop's harnesses do not carry `canon`, so the rules in
//! force have to be a file in the repository, and `share` is an adoption
//! snapshot, not a reader's document. Ersilia's first week produced a
//! `jq` script that folded the log by hand to get scope and rank, which
//! `list --json` did not carry, and a `--check` for a stale copy. This is
//! that script, once, from the fold rather than beside it.
//!
//! **No date and no clock in the output**, so a committed render is stale
//! only when the canon changed, and `--check` can gate CI without failing
//! every morning.

use std::collections::BTreeMap;

use canon_core::{Canon, Commitment, Status};

use crate::profile::Profile;

/// What is in force, grouped by scope, ranked rules first in each.
pub fn markdown(canon: &Canon, name: &str, profile: Profile) -> String {
    let mut out = format!("# {name}\n\n");
    // Only the first line carries the profile's noun, so the rest wraps the
    // same on every profile.
    out.push_str(&format!(
        "The {} in force in this canon, rendered from `.canon/acts.jsonl` by\n\
         `canon list --markdown`. Change the canon, not this file. `canon why <id>`\n\
         gives the history of any of them, and `canon list --markdown --out <file>\n\
         --check` fails when this copy is stale.\n",
        profile.nouns()
    ));

    let mut scoped: BTreeMap<String, Vec<&Commitment>> = BTreeMap::new();
    let mut unscoped: Vec<&Commitment> = Vec::new();
    let mut inherited: Vec<&Commitment> = Vec::new();
    for c in canon.active() {
        match canon.scope_of(&c.id) {
            Some(s) => scoped.entry(s.to_string()).or_default().push(c),
            None if c.from.is_some() => inherited.push(c),
            None => unscoped.push(c),
        }
    }

    let mut top: Option<&str> = None;
    for (scope, cs) in &scoped {
        let head = scope.split('.').next().unwrap_or(scope);
        if top != Some(head) {
            out.push_str(&format!("\n## {head}\n"));
            top = Some(head);
        }
        if scope != head {
            out.push_str(&format!("\n### {scope}\n"));
        }
        items(&mut out, canon, cs);
    }
    // A canon that uses no scopes is one list, and a heading over it would
    // only say so.
    if !unscoped.is_empty() {
        if !scoped.is_empty() || !inherited.is_empty() {
            out.push_str("\n## Unscoped\n");
        }
        items(&mut out, canon, &unscoped);
    }
    if !inherited.is_empty() {
        match &canon.ancestry {
            Some(a) => out.push_str(&format!(
                "\n## Inherited from {}@{}\n\n`canon diff --upstream` says where this canon has \
                 diverged from them.\n",
                a.lineage, a.generation
            )),
            None => out.push_str("\n## Inherited\n"),
        }
        items(&mut out, canon, &inherited);
    }

    let waiting = canon.proposed().count()
        + canon
            .retractions
            .iter()
            .filter(|r| matches!(r.verdict, canon_core::Verdict::Proposed { .. }))
            .count();
    if waiting > 0 {
        out.push_str(&format!(
            "\n---\n\n{waiting} proposed, not yet in force — `canon list` shows them.\n"
        ));
    }
    out
}

/// One bullet per commitment: ranked ones first, the rank as a label rather
/// than as bold text, because the text may carry its own emphasis.
fn items(out: &mut String, canon: &Canon, cs: &[&Commitment]) {
    out.push('\n');
    let (ranked, plain): (Vec<&&Commitment>, Vec<&&Commitment>) =
        cs.iter().partition(|c| canon.rank_of(&c.id).is_some());
    for c in ranked.into_iter().chain(plain) {
        debug_assert!(matches!(c.status, Status::Active));
        let text = c.text.replace('\n', " ");
        match canon.rank_of(&c.id) {
            Some(rank) => out.push_str(&format!("- **{rank}:** {text} `{}`\n", c.id)),
            None => out.push_str(&format!("- {text} `{}`\n", c.id)),
        }
    }
}

/// `list --json`'s commitments, each carrying its scope and rank.
///
/// The fold keeps both beside the commitments — `scopes` as pairs, `ranks`
/// as pairs — which is the right shape for a fold and the wrong one for a
/// reader, who had to join three arrays (and read `acts.jsonl` for rank)
/// to answer "which rules are architecture principles".
pub fn with_scope_and_rank(canon: &Canon) -> serde_json::Value {
    let mut v = serde_json::to_value(canon).unwrap_or_default();
    if let Some(cs) = v.get_mut("commitments").and_then(|c| c.as_array_mut()) {
        for c in cs {
            let Some(id) = c
                .get("id")
                .and_then(|i| i.as_str())
                .and_then(|i| canon.commitments.iter().find(|x| x.id.as_str() == i))
                .map(|x| x.id.clone())
            else {
                continue;
            };
            if let Some(obj) = c.as_object_mut() {
                if let Some(s) = canon.scope_of(&id) {
                    obj.insert("scope".into(), serde_json::json!(s.to_string()));
                }
                if let Some(r) = canon.rank_of(&id) {
                    obj.insert("rank".into(), serde_json::json!(r));
                }
            }
        }
    }
    v
}

#[cfg(test)]
mod tests;
