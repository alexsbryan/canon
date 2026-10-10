// SPDX-License-Identifier: AGPL-3.0-or-later
//! What waits on whom. The fold is asked; these pin that the asking gets
//! the fold's answer for the shapes people actually meet.

use super::*;
use canon_core::Scope;

/// A house: alex holds it, sam holds the kitchen, and an agent has
/// proposed a rule for each. Acts a second apart so grants predate what
/// they govern.
pub(crate) fn house() -> (Log, ActId, ActId) {
    let grant = |holder: &str, scope: &str, ts: i64| {
        Act::new(
            ActKind::Grant {
                holder: holder.into(),
                scope: Scope::new(scope).unwrap(),
                horizon: None,
                rationale: String::new(),
            },
            ts,
            "human:alex",
        )
    };
    let propose = |text: &str, scope: &str, ts: i64| {
        let a = Act::new(
            ActKind::Assert {
                text: text.into(),
                from: None,
                source: None,
            },
            ts,
            "agent:euphemia",
        );
        let s = Act::new(
            ActKind::Scoped {
                commitment: a.id.clone(),
                scope: Scope::new(scope).unwrap(),
            },
            ts,
            "agent:euphemia",
        );
        (a, s)
    };
    let (quiet, q_scope) = propose("Quiet hours are 10pm to 7am.", "house", 200);
    let (pans, p_scope) = propose("Pans are washed the same evening.", "house.kitchen", 200);
    let log = Log::from_acts(vec![
        grant("human:alex", "house", 100),
        grant("human:sam", "house.kitchen", 100),
        quiet.clone(),
        q_scope,
        pans.clone(),
        p_scope,
    ]);
    (log, quiet.id, pans.id)
}

fn waiting(log: &Log, actor: &str) -> Vec<ActId> {
    let st = log.derive_at(300);
    queue(log, &st, actor, 300)
        .into_iter()
        .filter(|i| i.yours)
        .map(|i| i.id)
        .collect()
}

#[test]
fn a_proposal_waits_on_whoever_holds_its_room_and_nobody_else() {
    let (log, quiet, pans) = house();
    assert_eq!(waiting(&log, "human:alex"), vec![quiet.clone()]);
    // The kitchen is sam's: the narrowest holder decides, not the house.
    assert_eq!(waiting(&log, "human:sam"), vec![pans]);
    assert!(waiting(&log, "human:dana").is_empty(), "no seat, no say");
    assert!(
        waiting(&log, "agent:euphemia").is_empty(),
        "agents are never asked"
    );
}

#[test]
fn once_you_have_answered_it_is_not_waiting_on_you() {
    let (log, quiet, _) = house();
    let approve = Act::new(
        ActKind::Position {
            about: quiet.as_str().into(),
            citing: None,
            pull: Pull::Toward,
            because: "approved".into(),
        },
        250,
        "human:alex",
    );
    let log = Log::from_acts([log.acts().to_vec(), vec![approve]].concat());
    assert!(waiting(&log, "human:alex").is_empty());
    assert_eq!(outcome(&log.derive_at(300), &quiet), "in force");
}

#[test]
fn a_proposal_reads_as_what_it_would_change() {
    let (log, quiet, _) = house();
    let st = log.derive_at(300);
    let item = queue(&log, &st, "human:alex", 300)
        .into_iter()
        .find(|i| i.id == quiet)
        .unwrap();
    assert_eq!(item.kind, Kind::Rule);
    assert_eq!(item.text, "Quiet hours are 10pm to 7am.");
    assert_eq!(item.scope.as_deref(), Some("house"));
    assert_eq!(item.by, "agent:euphemia");
    assert!(item.needs.contains("holds house"), "{}", item.needs);
}

#[test]
fn a_source_is_quoted_only_from_inside_the_canons_folder() {
    let root = std::env::temp_dir().join(format!("canon-review-passage-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".canon")).unwrap();
    std::fs::create_dir_all(root.join("notes")).unwrap();
    std::fs::write(root.join("notes/meeting.md"), "one\ntwo\nthree\n").unwrap();
    let outside = std::env::temp_dir().join(format!("canon-review-secret-{}", std::process::id()));
    std::fs::write(&outside, "do not show this\n").unwrap();
    let d = root.join(".canon");

    assert_eq!(
        passage(&d, "notes/meeting.md:2-3").as_deref(),
        Some("two\nthree")
    );
    assert_eq!(passage(&d, &format!("{}:1-1", outside.display())), None);
    let up = format!("../canon-review-secret-{}:1-1", std::process::id());
    assert_eq!(passage(&d, &up), None, "a path that climbs out");
    assert_eq!(
        passage(&d, "notes/meeting.md:3-2"),
        None,
        "a backwards range"
    );
    assert_eq!(passage(&d, "notes/meeting.md"), None, "no range, no quote");
}

#[test]
fn ago_says_it_the_way_a_person_does() {
    assert_eq!(ago(1000, 1010), "just now");
    assert_eq!(ago(0, 600), "10 minutes ago");
    assert_eq!(ago(0, 3 * 3600), "3 hours ago");
    assert_eq!(ago(0, 2 * 86_400), "2 days ago");
    assert!(ago(0, 30 * 86_400).starts_with("on "));
}
