// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon adopt <url>@<gen> --rebase`: move a canon that already descends
//! from one lineage onto another, keeping its own record.
//!
//! `rebase --onto` proposes and writes nothing, `adopt` refused a canon that
//! already descends from a lineage, and `upgrade` takes only newer
//! generations of the same one, so nothing completed the move. Re-adopting
//! into a fresh canon throws away what the fork decided: its ratified
//! additions, its retractions, the positions taken on them.
//!
//! **The record is not rewritten.** The canon's inherited rules carry
//! `from` the lineage it first adopted, and that link stays. What the move
//! adds is one `adopt` act naming the new base, the seed it diffs against,
//! and [`super::CARRIED_FILE`], pairing each rule of the new base with the
//! local commitment that already holds it. Acts are written only where the
//! new base asks for one and nothing local stands in the way:
//!
//! - a rule of the base this canon holds no counterpart of is asserted, as
//!   `adopt` asserts every rule;
//! - a rule the base reworded, where this canon left the original alone, is
//!   superseded to the new words, as `upgrade` does;
//! - an inherited rule the base does not hold, where this canon left it
//!   alone, is retracted, as `upgrade` does when upstream withdraws one.
//!
//! **It refuses wherever the rebase would report a conflict**, through the
//! same reading [`crate::rebase::obstacles`] gives, and wherever the base
//! reworded a rule this canon had already changed. Every act it writes is
//! authored by the person who runs it.

use std::path::Path;

use canon_core::{ActId, ActKind, Canon, Commitment, Snapshot, SnapshotCommitment, Status};

use crate::cmds::fail;
use crate::model;
use crate::rebase::{self, Base};

/// What the move will write, decided in full before anything is.
#[derive(Debug, Default)]
struct Plan {
    /// A base rule's id, and the local commitment that already holds it.
    carried: Vec<(ActId, ActId)>,
    /// Base rules with no local counterpart.
    asserted: Vec<SnapshotCommitment>,
    /// A local rule left alone, and the base's rewording of it.
    reworded: Vec<(ActId, SnapshotCommitment)>,
    /// Inherited rules left alone that the base does not hold.
    retracted: Vec<(ActId, String)>,
}

pub(super) fn onto(
    dir: &Path,
    canon: &Canon,
    allow_remote: bool,
    target: &Snapshot,
    target_log: Option<&Canon>,
    source: &str,
) -> i32 {
    let seed = match super::load_seed(dir) {
        Ok(s) => s,
        Err(e) => return fail(e),
    };
    if target.lineage == seed.lineage {
        return fail(format!(
            "this canon already descends from {} — `canon upgrade` takes a newer generation \
             of it; nothing was written",
            seed.lineage
        ));
    }
    let carried = match super::load_carried(dir) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let d = canon_core::Divergence::compute_carried(&seed, canon, &carried);
    let base = Base {
        snapshot: target,
        log: target_log,
    };
    match rebase::obstacles(
        || model::client_for(dir, allow_remote),
        &d,
        canon,
        &seed,
        &base,
    ) {
        Ok(stops) if !stops.is_empty() => {
            return fail(format!(
                "not moved onto {}@{}: the rebase leaves {} change(s) for a person, and \
                 nothing was written.\n\n{}\n\n  `canon rebase --onto {source}@{}` prints the \
                 whole reading.",
                target.lineage,
                target.generation,
                stops.len(),
                stops.join("\n"),
                target.generation
            ))
        }
        Ok(_) => {}
        Err(e) => {
            eprintln!(
                "not moved: whether this canon's changes conflict with {}@{} could not be \
                 judged, and nothing was written",
                target.lineage, target.generation
            );
            return model::report(e);
        }
    }

    let plan = match plan(&seed, canon, &carried, target, target_log) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };
    // **The new `adopt` must sort after the old one.** The fold reads
    // ancestry from the last `adopt` in `(ts_unix, id)` order, so a move in
    // the same second as the adoption it replaces could sort first, by id,
    // and leave the canon reading the old base. Seen in this module's own
    // tests, where both land in one second; a person is unlikely to, and the
    // wait is under a second when they do.
    if let Some(a) = canon.ancestry() {
        while crate::store::now() <= a.at {
            eprintln!("waiting out the second the last adoption was written in");
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    }
    match write(dir, &seed, &carried, target, source, plan) {
        Ok(lines) => {
            print!("{lines}");
            0
        }
        Err(e) => fail(e),
    }
}

/// Is `c` exactly as it arrived: in force, and not carried knowingly
/// against another rule? Only such a rule follows the base on its own.
fn untouched(canon: &Canon, c: &Commitment) -> bool {
    matches!(c.status(), Status::Active)
        && !canon.tolerated().any(|x| &x.a == c.id() || &x.b == c.id())
}

fn plan(
    seed: &Snapshot,
    canon: &Canon,
    carried: &[(ActId, ActId)],
    target: &Snapshot,
    target_log: Option<&Canon>,
) -> Result<Plan, String> {
    // The local commitment holding each rule this canon inherited.
    let inherited: Vec<&Commitment> = seed
        .commitments
        .iter()
        .filter_map(|s| super::local_of(canon, carried, &s.id))
        .collect();
    let mut claimed: Vec<ActId> = Vec::new();
    let mut plan = Plan::default();
    let free = |c: &&Commitment, claimed: &[ActId]| !claimed.contains(c.id());

    for t in &target.commitments {
        // The same rule by provenance: a base rule `from` what this canon
        // inherited, or one of this canon's own `from` it.
        let from = target_log.and_then(|l| l.get(&t.id)).and_then(|c| c.from());
        let by_provenance = canon
            .commitments()
            .iter()
            .find(|c| c.from() == Some(&t.id) && free(c, &claimed))
            .or_else(|| {
                from.and_then(|f| super::local_of(canon, carried, f))
                    .filter(|c| free(c, &claimed))
            });
        if let Some(c) = by_provenance.filter(|c| c.text() == t.text) {
            claimed.push(c.id().clone());
            if c.from() != Some(&t.id) {
                plan.carried.push((t.id.clone(), c.id().clone()));
            }
            continue;
        }
        // The base reworded a rule this canon inherited.
        if let Some(c) = reworded_from(target_log, &t.id)
            .iter()
            .find_map(|a| super::local_of(canon, carried, a))
            .filter(|c| free(c, &claimed) && c.text() != t.text)
        {
            if !untouched(canon, c) {
                return Err(format!(
                    "not moved: {}@{} rewords \"{}\" as \"{}\", and this canon has already \
                     changed it. Nothing was written; decide which words stand first.",
                    target.lineage,
                    target.generation,
                    c.text(),
                    t.text
                ));
            }
            claimed.push(c.id().clone());
            plan.reworded.push((c.id().clone(), t.clone()));
            continue;
        }
        // The same words, from no traceable source: a pasted base, or a rule
        // both sides wrote. An inherited rule first, then one of this
        // canon's own that is in force.
        let by_text = inherited
            .iter()
            .copied()
            .find(|c| c.text() == t.text && free(c, &claimed))
            .or_else(|| {
                canon.commitments().iter().find(|c| {
                    c.text() == t.text
                        && free(c, &claimed)
                        && matches!(c.status(), Status::Active | Status::Proposed { .. })
                })
            });
        if let Some(c) = by_text {
            claimed.push(c.id().clone());
            plan.carried.push((t.id.clone(), c.id().clone()));
            continue;
        }
        plan.asserted.push(t.clone());
    }

    for c in inherited {
        if !claimed.contains(c.id()) && untouched(canon, c) {
            plan.retracted.push((c.id().clone(), c.text().to_string()));
        }
    }
    Ok(plan)
}

/// The ids and `from`s a base rule's supersession chain reaches, the rule
/// itself excluded. Empty for a pasted base, which has no chain.
fn reworded_from(target_log: Option<&Canon>, id: &ActId) -> Vec<ActId> {
    let Some(log) = target_log else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut frontier: Vec<ActId> = log
        .get(id)
        .map(|c| c.replaces().to_vec())
        .unwrap_or_default();
    while let Some(cur) = frontier.pop() {
        if out.contains(&cur) {
            continue;
        }
        if let Some(c) = log.get(&cur) {
            if let Some(f) = c.from() {
                out.push(f.clone());
            }
            frontier.extend(c.replaces().iter().cloned());
        }
        out.push(cur);
    }
    out
}

fn write(
    dir: &Path,
    seed: &Snapshot,
    carried: &[(ActId, ActId)],
    target: &Snapshot,
    source: &str,
    plan: Plan,
) -> Result<String, String> {
    let at = format!("{}@{}", target.lineage, target.generation);
    crate::cmds::write(
        dir,
        ActKind::Adopt {
            lineage: target.lineage.clone(),
            generation: target.generation.clone(),
            source: Some(source.to_string()),
        },
    )?;
    let mut pairs: Vec<(ActId, ActId)> = carried.to_vec();
    pairs.extend(plan.carried.iter().cloned());
    for (local, t) in &plan.reworded {
        let act = crate::cmds::write(
            dir,
            ActKind::Supersede {
                text: t.text.clone(),
                old: vec![local.clone()],
                rationale: format!("reworded in {at}, the base this canon moved onto"),
            },
        )?;
        pairs.push((t.id.clone(), act.id));
    }
    for t in &plan.asserted {
        crate::cmds::write(
            dir,
            ActKind::Assert {
                text: t.text.clone(),
                from: Some(t.id.clone()),
                source: None,
            },
        )?;
    }
    for (local, _) in &plan.retracted {
        crate::cmds::write(
            dir,
            ActKind::Retract {
                target: local.clone(),
                rationale: format!("not in {at}, the base this canon moved onto"),
            },
        )?;
    }
    super::save_carried(dir, &pairs)?;
    super::save_seed(dir, target)?;

    let mut out = format!(
        "moved onto {at} from {}@{} — your own record is unchanged\n  \
         {} rule(s) of the new base carried onto the ones this canon holds\n",
        seed.lineage,
        seed.generation,
        target.commitments.len() - plan.asserted.len() - plan.reworded.len()
    );
    for (label, n) in [
        ("asserted, new to this canon", plan.asserted.len()),
        ("superseded to the base's new words", plan.reworded.len()),
        (
            "retracted, because the base does not hold them",
            plan.retracted.len(),
        ),
    ] {
        if n > 0 {
            out.push_str(&format!("  {n} {label}\n"));
        }
    }
    for (_, text) in &plan.retracted {
        out.push_str(&format!("    retracted \"{text}\"\n"));
    }
    out.push_str(&format!(
        "  canon diff --upstream   how you differ from {at}\n"
    ));
    Ok(out)
}
