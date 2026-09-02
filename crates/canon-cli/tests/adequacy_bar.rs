// SPDX-License-Identifier: AGPL-3.0-or-later
//! The stopping rule, enforced.
//!
//! `PRIMITIVES.md` claims governance is policy over a small mechanical core.
//! The claim is falsifiable in two directions and this suite checks both.
//!
//! **Forwards:** every technology of political economy the design argument
//! says is `built` must compose from ops that actually exist. A table row
//! naming mechanism this build does not have is a promise, not a primitive.
//!
//! **Backwards, which is the direction that matters here:** every op in the
//! format must be named by at least one technology, and must be listed in the
//! census below with the primitive it serves and the composition it was not
//! reachable by. **A new op fails this suite until somebody writes down why it
//! exists.** That is the whole point — a design document that ran once and a
//! design document that runs on every commit are different objects.

use std::collections::BTreeSet;
use std::path::Path;

use canon_core::act::{KNOWN_ANNOTATIONS, STRUCTURAL};

/// Every op, the primitive it serves, and what no composition reached.
///
/// The third column is the argument. Adding a row is cheap and writing a
/// truthful third column is not, which is the intended ratio.
const CENSUS: &[(&str, &str, &str)] = &[
    ("assert", "1", "a commitment entering the record is the record"),
    ("supersede", "1", "replacement that keeps the reason is not deletion plus insertion"),
    ("retract", "1", "withdrawal with no replacement is a third transition, not either of the others"),
    ("revert", "2", "tomb-stoning an act is not the same as reversing what it said"),
    ("accept", "3", "a contradiction carried on purpose has to say what it protects"),
    ("dismiss", "3", "detector noise and a real conflict must not derive to one thing"),
    ("question", "3", "a gap is commitment-shaped and nothing else in the set is shaped like it"),
    ("silence", "3", "what was left unwritten on purpose is invisible to every other op"),
    ("adopt", "3", "ancestry has to survive a file that arrives by paste, so it cannot be metadata"),
    ("position", "5", "evidence with a source, a direction and a reason is not a commitment"),
    ("grant", "6", "who holds what cannot be derived from what has been written"),
    ("withdraw", "6", "standing ending is not standing never granted"),
    ("scoped", "6", "which boundary a commitment sits in is not in its text"),
    ("policy", "7", "the rule a canon decides under has to be citable and supersedable"),
    ("ratification", "7", "how a proposal becomes a rule is a level above the rule"),
    ("decided", "7", "an adjudication is a thing the group did; no observation op exists"),
    ("rank", "7", "principle and convention differ in how hard they are to amend"),
    ("horizon", "8", "one date op pays for term limits, sunsets, trials, revisits and rotation"),
    ("draw_commit", "9", "a lot nobody can steer needs its moment announced before it"),
    ("draw_secret", "9", "a seed no participant controls needs sealed contributions"),
    ("draw_reveal", "9", "and the opening has to be a separate act from the sealing"),
    ("allot", "10", "no composition reaches what a scope has to share"),
    ("allocation", "10", "policy returns an authority; nothing returned an assignment"),
];

const VERDICTS: [&str; 4] = ["built", "carried", "absent", "out-of-scope"];

fn primitives_md() -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../PRIMITIVES.md");
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("reading {}: {e}", p.display()))
}

/// The adequacy table, as `(technology, composition, verdict)`.
///
/// Parsed out of the document rather than copied into the test, so the two
/// cannot drift. A table nobody can run is the defect this file exists for.
fn table() -> Vec<(String, String, String)> {
    let md = primitives_md();
    md.lines()
        .skip_while(|l| !l.starts_with("| Technology | Composition | Verdict |"))
        .skip(2)
        .take_while(|l| l.starts_with('|'))
        .map(|l| {
            let cells: Vec<&str> = l.trim_matches('|').split('|').map(str::trim).collect();
            assert_eq!(cells.len(), 3, "malformed row: {l}");
            (
                cells[0].to_string(),
                cells[1].to_string(),
                cells[2].to_string(),
            )
        })
        .collect()
}

fn ops() -> BTreeSet<&'static str> {
    STRUCTURAL.iter().chain(KNOWN_ANNOTATIONS.iter()).copied().collect()
}

/// `op:grant` → `grant`, from any cell.
fn ops_named(composition: &str) -> Vec<String> {
    composition
        .split('`')
        .filter_map(|t| t.strip_prefix("op:"))
        .map(str::to_string)
        .collect()
}

#[test]
fn every_op_is_in_the_census_and_the_census_invents_none() {
    // **The stopping rule with teeth.** A new op does not compile past this
    // until it is listed with the primitive it serves and the composition it
    // was not reachable by. Adding the row is the cheap part; writing a
    // truthful third column is the bar.
    let listed: BTreeSet<&str> = CENSUS.iter().map(|(op, _, _)| *op).collect();
    let actual = ops();
    let missing: Vec<&&str> = actual.difference(&listed).collect();
    assert!(
        missing.is_empty(),
        "op(s) in the format and not in the census: {missing:?} — add them to \
         adequacy_bar.rs with the primitive they serve and the composition \
         they were not reachable by, or do not add the op"
    );
    let invented: Vec<&&str> = listed.difference(&actual).collect();
    assert!(invented.is_empty(), "census names op(s) the format does not have: {invented:?}");
    assert_eq!(CENSUS.len(), actual.len(), "one row per op, no duplicates");

    for (op, primitive, why) in CENSUS {
        assert!(
            why.split_whitespace().count() >= 6,
            "{op}: the argument is one line and it has to be an argument"
        );
        assert!(
            primitives_md().contains(&format!("## Primitive {primitive} —")),
            "{op} claims Primitive {primitive}, which PRIMITIVES.md does not have"
        );
    }
}

#[test]
fn a_technology_marked_built_composes_from_ops_that_exist() {
    let actual = ops();
    let mut built = 0;
    for (tech, composition, verdict) in table() {
        assert!(
            VERDICTS.contains(&verdict.as_str()),
            "{tech}: verdict `{verdict}` is not one of {VERDICTS:?}"
        );
        let named = ops_named(&composition);
        if verdict == "built" {
            built += 1;
            assert!(
                !named.is_empty(),
                "{tech} is marked built and names no op — a composition is the claim"
            );
            for op in named {
                assert!(
                    actual.contains(op.as_str()),
                    "{tech} composes from `{op}`, which this build does not have. \
                     Either the op is missing or the verdict is a promise."
                );
            }
        }
    }
    assert!(built >= 15, "only {built} technologies marked built; the table has thinned");
}

#[test]
fn no_op_exists_that_no_technology_needs() {
    // The reverse check, and the one that catches bloat. An op nothing in the
    // table reaches for is mechanism nobody asked for — which is exactly the
    // failure mode the stopping rule exists to prevent, arriving quietly.
    let rows = table();
    let mut reached: BTreeSet<String> = BTreeSet::new();
    for (_, composition, verdict) in &rows {
        if verdict == "built" {
            reached.extend(ops_named(composition));
        }
    }
    let all = ops();
    let orphans: Vec<&&str> = all.iter().filter(|op| !reached.contains(**op)).collect();
    assert!(
        orphans.is_empty(),
        "op(s) no technology in the adequacy table needs: {orphans:?} — either a \
         technology is missing from the table or the op is"
    );
}

#[test]
fn the_frontier_is_written_down_rather_than_implied() {
    // A table of nothing but successes is a table that stopped being a test.
    // The rows that do NOT span are the ones that say where the next primitive
    // would come from, and each is a candidate that has to pass the stopping
    // rule before it becomes one.
    let rows = table();
    let open: Vec<&(String, String, String)> = rows
        .iter()
        .filter(|(_, _, v)| v == "absent" || v == "carried")
        .collect();
    assert!(
        open.len() >= 4,
        "the adequacy table records {} unmet technologies; a design document \
         whose frontier is empty has stopped looking",
        open.len()
    );
}
