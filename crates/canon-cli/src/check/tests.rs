// SPDX-License-Identifier: AGPL-3.0-or-later
//! `check` tests. The load-bearing ones are the profile invariants: the
//! personal profile never renders a verdict and never exits 1, on either
//! surface.

use canon_core::{Act, ActKind, Log};
use serde_json::json;

use super::*;
use crate::testing::{completion, Mock};

fn canon_of(texts: &[&str]) -> Canon {
    Log::from_acts(
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| {
                Act::new(
                    ActKind::Assert {
                        text: (*t).into(),
                        from: None,
                        source: None,
                    },
                    100 + i as i64,
                    "human:alex",
                )
            })
            .collect(),
    )
    .derive()
}

const TEXTS: [&str; 2] = [
    "Mornings are protected; I do not schedule before 11.",
    "Be someone the team can rely on.",
];

fn judged(items: &[(usize, &str, &str)]) -> String {
    completion(
        &json!({
            "bearings": items
                .iter()
                .map(|(c, p, b)| json!({ "commitment": c, "pull": p, "because": b }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

fn stand(items: &[(usize, &str, &str)]) -> (Canon, Standing) {
    let canon = canon_of(&TEXTS);
    let mock = Mock::spawn(vec![(200, judged(items))]);
    let (standing, _) = assess(&mock.client(), &canon, "take the 8am rotation").unwrap();
    (canon, standing)
}

#[test]
fn the_personal_profile_never_returns_exit_one() {
    // The invariant, pinned. Whatever the outcome, a personal canon reports
    // stakes; an exit 1 is a machine saying a life choice failed a check.
    for outcome in [Outcome::Supported, Outcome::Conflicts, Outcome::Unaddressed] {
        assert_eq!(exit_code(Profile::Personal, outcome), 0, "{outcome:?}");
    }
    assert_eq!(exit_code(Profile::Code, Outcome::Conflicts), 1);
    assert_eq!(exit_code(Profile::House, Outcome::Conflicts), 1);
    assert_eq!(exit_code(Profile::Code, Outcome::Unaddressed), 2);
    assert_eq!(exit_code(Profile::Code, Outcome::Supported), 0);
}

#[test]
fn the_personal_profile_never_renders_a_verdict_on_either_surface() {
    let (canon, standing) = stand(&[
        (1, "against", "the rotation starts at 8"),
        (2, "toward", "the team is short"),
    ]);
    assert_eq!(standing.outcome(), Outcome::Conflicts);

    let text = render(Profile::Personal, &canon, &standing);
    for banned in ["CONFLICT", "SUPPORTED", "UNADDRESSED", "verdict"] {
        assert!(
            !text.contains(banned),
            "personal rendered `{banned}`:\n{text}"
        );
    }
    assert!(text.contains("STAKE"));
    assert!(text.contains("pulls against") && text.contains("pulls toward"));

    // And the machine-readable surface carries no outcome either: an outcome
    // is a verdict however it is serialized.
    let p = payload(Profile::Personal, &standing);
    assert!(p.get("outcome").is_none(), "{p}");
    assert!(p.get("positions").is_some());
    // The other profiles do carry it.
    assert_eq!(payload(Profile::Code, &standing)["outcome"], "conflicts");
}

#[test]
fn the_code_profile_names_the_rule_it_conflicts_with() {
    let (canon, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    let text = render(Profile::Code, &canon, &standing);
    assert!(text.starts_with("CONFLICT"));
    assert!(text.contains("Mornings are protected"));
    assert!(text.contains("because: the rotation starts at 8"));
    assert!(text.contains("in force"));
}

#[test]
fn the_house_profile_says_which_act_the_proposal_needs() {
    let (canon, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    let text = render(Profile::House, &canon, &standing);
    assert!(text.contains("NEEDS AN AMENDMENT"));
    assert!(text.contains("canon supersede"));

    let (canon, standing) = stand(&[]);
    let text = render(Profile::House, &canon, &standing);
    assert!(text.contains("NEEDS A NEW RULE"));
    assert!(text.contains("canon add"));
}

#[test]
fn nothing_bearing_on_a_proposal_is_unaddressed_not_approval() {
    let (canon, standing) = stand(&[]);
    assert_eq!(standing.outcome(), Outcome::Unaddressed);
    let text = render(Profile::Code, &canon, &standing);
    assert!(text.starts_with("UNADDRESSED"));
    assert!(!text.contains("SUPPORTED"));
    // And it offers the act that closes the gap.
    assert!(text.contains("canon question"));
}

#[test]
fn a_bearing_naming_a_commitment_that_does_not_exist_is_refused() {
    // The model returns a number outside the list. Clamping would attribute
    // the conflict to whichever rule happened to be last.
    let canon = canon_of(&TEXTS);
    let mock = Mock::spawn(vec![(200, judged(&[(99, "against", "invented")]))]);
    let (standing, _) = assess(&mock.client(), &canon, "p").unwrap();
    assert!(standing.positions.is_empty());
    assert_eq!(standing.outcome(), Outcome::Unaddressed);
}

#[test]
fn a_bearing_with_no_reason_is_refused_and_reported() {
    let canon = canon_of(&TEXTS);
    let mock = Mock::spawn(vec![(200, judged(&[(1, "against", "   ")]))]);
    let (standing, refused) = assess(&mock.client(), &canon, "p").unwrap();
    assert!(standing.positions.is_empty());
    assert_eq!(
        refused.len(),
        1,
        "the refusal must be reportable, not silent"
    );
}

#[test]
fn an_unreadable_pull_is_dropped_rather_than_guessed() {
    let canon = canon_of(&TEXTS);
    let mock = Mock::spawn(vec![(200, judged(&[(1, "maybe", "unsure")]))]);
    let (standing, _) = assess(&mock.client(), &canon, "p").unwrap();
    assert!(standing.positions.is_empty());
}

#[test]
fn a_carried_contradiction_is_shown_rather_than_relitigated() {
    // The personal profile's whole stance: a contradiction someone already
    // chose to hold is reported with what it protects, never re-argued.
    let a = Act::new(
        ActKind::Assert {
            text: TEXTS[0].into(),
            from: None,
            source: None,
        },
        100,
        "human:alex",
    );
    let b = Act::new(
        ActKind::Assert {
            text: TEXTS[1].into(),
            from: None,
            source: None,
        },
        101,
        "human:alex",
    );
    let acc = Act::new(
        ActKind::Accept {
            a: a.id.clone(),
            b: b.id.clone(),
            rationale: "reliability is how I earn the autonomy, for now".into(),
            revisit: Some("2026-10-01".into()),
        },
        200,
        "human:alex",
    );
    let canon = Log::from_acts(vec![a, b, acc]).derive();
    let mock = Mock::spawn(vec![(
        200,
        judged(&[
            (1, "against", "starts at 8"),
            (2, "toward", "team is short"),
        ]),
    )]);
    let (standing, _) = assess(&mock.client(), &canon, "take the rotation").unwrap();
    let text = render(Profile::Personal, &canon, &standing);
    assert!(
        text.contains("reliability is how I earn the autonomy"),
        "{text}"
    );
    assert!(text.contains("revisit by 2026-10-01"), "{text}");
}
