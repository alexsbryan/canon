// SPDX-License-Identifier: AGPL-3.0-or-later
use canon_core::{Act, ActId, ActKind, Log, Scope};

use super::*;

fn asserted(text: &str, ts: i64, from: Option<&str>) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: from.map(|f| ActId::from_raw(f.to_string())),
            source: None,
        },
        ts,
        "human:alex",
    )
}

fn placed(id: &ActId, scope: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Scoped {
            commitment: id.clone(),
            scope: Scope::new(scope).unwrap(),
        },
        ts,
        "human:alex",
    )
}

fn ranked(id: &ActId, rank: &str, ts: i64) -> Act {
    Act::new(
        ActKind::Rank {
            commitment: id.clone(),
            rank: rank.into(),
        },
        ts,
        "human:alex",
    )
}

/// Two architecture rules, one a principle, one under a nested scope; one
/// procedure rule; one inherited and one local unscoped rule.
fn ersilia() -> Canon {
    let boundary = asserted("Measure and recommend; a person enacts.", 100, None);
    let store = asserted("**One store.** Postgres is the only state.", 101, None);
    let small = asserted("Keep diffs small.", 102, None);
    let mine = asserted("Name things for what they are.", 103, None);
    let theirs = asserted("Cite or abstain.", 104, Some("can-000000000001"));
    Log::from_acts(vec![
        placed(&boundary.id, "arch.boundary", 100),
        placed(&store.id, "arch", 101),
        placed(&small.id, "procedure", 102),
        ranked(&store.id, "principle", 105),
        boundary,
        store,
        small,
        mine,
        theirs,
    ])
    .derive()
}

#[test]
fn rules_are_grouped_by_scope_with_ranked_ones_first() {
    let doc = markdown(&ersilia(), "ersilia", Profile::Code);
    assert!(doc.starts_with("# ersilia\n"), "{doc}");
    let at = |needle: &str| {
        doc.find(needle)
            .unwrap_or_else(|| panic!("`{needle}` missing from:\n{doc}"))
    };
    // Top-level scopes as sections, deeper ones under them, sorted.
    assert!(at("## arch\n") < at("### arch.boundary\n"));
    assert!(at("### arch.boundary\n") < at("## procedure\n"));
    // The rank is a label, so a text with its own bold is not double-bolded.
    assert!(
        doc.contains("- **principle:** **One store.** Postgres is the only state. `can-"),
        "{doc}"
    );
    assert!(at("## Unscoped\n") < at("Name things for what they are."));
    // Inherited rules are apart from the canon's own.
    assert!(at("## Inherited\n") < at("Cite or abstain."));
    assert!(!doc.contains("proposed, not yet in force"), "{doc}");
}

#[test]
fn the_render_has_no_clock_in_it() {
    // A committed copy is stale only when the canon changed; a date in the
    // output would fail `--check` every morning.
    let canon = ersilia();
    let a = markdown(&canon, "ersilia", Profile::Code);
    let b = markdown(&canon, "ersilia", Profile::Code);
    assert_eq!(a, b);
    assert!(!a.contains("2026"), "{a}");
}

#[test]
fn a_canon_with_no_scopes_is_one_list_with_no_heading_over_it() {
    let canon = Log::from_acts(vec![asserted("Quiet after eleven.", 100, None)]).derive();
    let doc = markdown(&canon, "house", Profile::House);
    assert!(!doc.contains("## "), "{doc}");
    assert!(doc.contains("- Quiet after eleven. `can-"), "{doc}");
}

#[test]
fn list_json_carries_scope_and_rank_on_each_commitment() {
    let canon = ersilia();
    let v = with_scope_and_rank(&canon);
    let cs = v["commitments"].as_array().unwrap();
    let store = cs
        .iter()
        .find(|c| c["text"].as_str().unwrap().contains("One store"))
        .unwrap();
    assert_eq!(store["scope"], "arch");
    assert_eq!(store["rank"], "principle");
    let mine = cs
        .iter()
        .find(|c| c["text"] == "Name things for what they are.")
        .unwrap();
    assert!(mine.get("scope").is_none() && mine.get("rank").is_none());
    // The pairs stay where they were, for anything already reading them.
    assert!(v["scopes"].is_array());
}
