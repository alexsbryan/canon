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

/// `args` are `adopt`'s own: `--allow-remote`, `--carry` and `-m` are read
/// from them here.
pub(super) fn onto(
    dir: &Path,
    canon: &Canon,
    args: &[String],
    target: &Snapshot,
    target_log: Option<&Canon>,
    source: &str,
) -> i32 {
    let allow_remote = crate::cmds::has(args, "--allow-remote");
    let carry = crate::cmds::flags(args, "--carry");
    let why = crate::cmds::flag(args, "-m").unwrap_or_default();
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
    let stops = match rebase::obstacles(
        || model::client_for(dir, allow_remote),
        &d,
        canon,
        &seed,
        &base,
    ) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "not moved: whether this canon's changes conflict with {}@{} could not be \
                 judged, and nothing was written",
                target.lineage, target.generation
            );
            return model::report(e);
        }
    };
    let carries = match carried_anyway(&stops, &carry, why) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let left: Vec<&rebase::Obstacle> = stops
        .iter()
        .filter(|o| !carries.iter().any(|(l, _)| o.local.as_ref() == Some(l)))
        .collect();
    if !left.is_empty() {
        let lines: Vec<&str> = left.iter().map(|o| o.line.as_str()).collect();
        // The way through, named, because only the person can take it. Not a
        // command to paste: the reason is theirs to write.
        let ids: Vec<String> = left
            .iter()
            .filter(|o| o.carriable())
            .filter_map(|o| o.local.as_ref().map(|l| format!("--carry {l}")))
            .collect();
        let through = if ids.is_empty() {
            String::new()
        } else {
            format!(
                "\n  A conflict you judge does not hold can be carried anyway, as your ruling on \
                 the record:\n    {} -m \"<why it stands against the base>\"",
                ids.join(" ")
            )
        };
        return fail(format!(
            "not moved onto {}@{}: the rebase leaves {} change(s) for a person, and nothing \
             was written.\n\n{}\n\n  `canon rebase --onto {source}@{}` prints the whole \
             reading.{through}",
            target.lineage,
            target.generation,
            left.len(),
            lines.join("\n"),
            target.generation
        ));
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
    match write(
        dir, canon, &seed, &carried, target, source, plan, &carries, why,
    ) {
        Ok(lines) => {
            print!("{lines}");
            0
        }
        Err(e) => fail(e),
    }
}

/// The conflicts the person named with `--carry`, each as the local rule
/// and the base rule the reading set it against.
///
/// **The model can only refuse; only the person overrides.** A carry names
/// a conflict this reading reported, by the local id the refusal printed,
/// and needs a reason: it is written as an `accept` of the pair, the act
/// canon already has for a contradiction carried knowingly, authored by
/// whoever runs the move.
fn carried_anyway(
    stops: &[rebase::Obstacle],
    carry: &[&str],
    why: &str,
) -> Result<Vec<(ActId, SnapshotCommitment)>, String> {
    let needles: Vec<&str> = carry
        .iter()
        .flat_map(|n| n.split(','))
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .collect();
    if needles.is_empty() {
        return Ok(Vec::new());
    }
    if why.trim().is_empty() {
        return Err(
            "--carry needs -m \"<why it stands against the base>\": carrying a conflict is a \
             ruling, and a ruling says what it protects. Nothing was written"
                .into(),
        );
    }
    let mut out: Vec<(ActId, SnapshotCommitment)> = Vec::new();
    for needle in needles {
        let hits: Vec<&rebase::Obstacle> = stops
            .iter()
            .filter(|o| o.conflict)
            .filter(|o| {
                o.local
                    .as_ref()
                    .is_some_and(|l| l.as_str() == needle || l.as_str().starts_with(needle))
            })
            .collect();
        let o = match hits.as_slice() {
            [o] => *o,
            [] => {
                return Err(format!(
                    "`{needle}` is not a conflict in this reading — nothing was written. A \
                     reading can differ from the last; the refusal names what blocks this one"
                ))
            }
            many => {
                return Err(format!(
                    "`{needle}` matches {} conflicts — use more characters; nothing was written",
                    many.len()
                ))
            }
        };
        let (Some(local), Some(against)) = (o.local.clone(), o.against.clone()) else {
            return Err(format!(
                "`{needle}` conflicts with no base rule the reading named, so there is no pair \
                 to carry it against. Nothing was written; a fresh reading may name one"
            ));
        };
        if !out.iter().any(|(l, _)| l == &local) {
            out.push((local, against));
        }
    }
    Ok(out)
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

#[allow(clippy::too_many_arguments)]
fn write(
    dir: &Path,
    canon: &Canon,
    seed: &Snapshot,
    carried: &[(ActId, ActId)],
    target: &Snapshot,
    source: &str,
    plan: Plan,
    carries: &[(ActId, SnapshotCommitment)],
    why: &str,
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
    let mut asserted: Vec<(ActId, ActId)> = Vec::new();
    for t in &plan.asserted {
        let act = crate::cmds::write(
            dir,
            ActKind::Assert {
                text: t.text.clone(),
                from: Some(t.id.clone()),
                source: None,
            },
        )?;
        asserted.push((t.id.clone(), act.id));
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
    // Each conflict the person carried: an `accept` of their rule and this
    // canon's holder of the base rule it was read against.
    for (local, against) in carries {
        let holder = pairs
            .iter()
            .chain(asserted.iter())
            .find(|(t, _)| t == &against.id)
            .map(|(_, l)| l.clone())
            .or_else(|| {
                canon
                    .commitments()
                    .iter()
                    .find(|c| c.from() == Some(&against.id))
                    .map(|c| c.id().clone())
            })
            .ok_or_else(|| {
                format!(
                    "moved, but {} has no holder here to carry {local} against; nothing \
                     records the carry — `canon accept {local} <id> -m \"…\"` does",
                    against.id
                )
            })?;
        crate::cmds::write(
            dir,
            ActKind::Accept {
                a: local.clone(),
                b: holder,
                rationale: format!(
                    "{} (carried onto {at} against the rebase's reading that it conflicts \
                     with {})",
                    why.trim(),
                    against.id
                ),
                revisit: None,
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
    if !carries.is_empty() {
        out.push_str(&format!(
            "  {} conflict(s) carried as your ruling, each an `accept` on the record\n",
            carries.len()
        ));
    }
    for (_, text) in &plan.retracted {
        out.push_str(&format!("    retracted \"{text}\"\n"));
    }
    out.push_str(&format!(
        "  canon diff --upstream   how you differ from {at}\n"
    ));
    Ok(out)
}
