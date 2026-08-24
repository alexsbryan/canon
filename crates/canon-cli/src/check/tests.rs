// SPDX-License-Identifier: AGPL-3.0-or-later
//! `check` tests. The load-bearing ones are the profile invariants: the
//! personal profile never renders a verdict and never exits 1, on either
//! surface.

use canon_core::{Act, ActKind, Log, Rule};
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

/// The decision the shipped default reaches, so tests written before policy
/// existed keep asserting exactly what they always did.
fn shipped(canon: &Canon, standing: &Standing) -> Decision {
    Rule::Default.decide(standing, &Attributes::default(), canon)
}

/// The same commitments, in a canon that has adopted a rule of its own.
///
/// The renderer stays quiet about the authority ladder until a group has
/// opted into governance, so any test asserting on that line needs a canon
/// where somebody actually did.
fn governed(rule: Rule) -> Canon {
    let mut acts: Vec<Act> = TEXTS
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
        .collect();
    acts.push(Act::new(
        ActKind::Policy {
            text: "how this community decides".into(),
            rule,
            scope: None,
        },
        200,
        "human:alex",
    ));
    Log::from_acts(acts).derive()
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

    let text = render(
        Profile::Personal,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
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
    let p = payload(
        Profile::Personal,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(p.get("outcome").is_none(), "{p}");
    assert!(p.get("positions").is_some());
    // The other profiles do carry it.
    assert_eq!(
        payload(Profile::Code, &standing, &shipped(&canon, &standing), None)["outcome"],
        "conflicts"
    );
}

#[test]
fn the_code_profile_names_the_rule_it_conflicts_with() {
    let (canon, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    let text = render(
        Profile::Code,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(text.starts_with("CONFLICT"));
    assert!(text.contains("Mornings are protected"));
    assert!(text.contains("because: the rotation starts at 8"));
    assert!(text.contains("in force"));
}

#[test]
fn the_house_profile_says_which_act_the_proposal_needs() {
    let (canon, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    let text = render(
        Profile::House,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(text.contains("NEEDS AN AMENDMENT"));
    assert!(text.contains("canon supersede"));

    let (canon, standing) = stand(&[]);
    let text = render(
        Profile::House,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(text.contains("NEEDS A NEW RULE"));
    assert!(text.contains("canon add"));
}

#[test]
fn nothing_bearing_on_a_proposal_is_unaddressed_not_approval() {
    let (canon, standing) = stand(&[]);
    assert_eq!(standing.outcome(), Outcome::Unaddressed);
    let text = render(
        Profile::Code,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
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
    let text = render(
        Profile::Personal,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(
        text.contains("reliability is how I earn the autonomy"),
        "{text}"
    );
    assert!(text.contains("revisit by 2026-10-01"), "{text}");
}

// ── policy at the check surface ─────────────────────────────

#[test]
fn the_canon_own_policy_decides_the_check_and_not_what_shipped() {
    // The same evidence, two communities, two answers. That is the whole
    // claim of the policy layer, asserted at the surface a person uses.
    let (bare, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    assert_eq!(shipped(&bare, &standing).authority, Authority::AskOne);

    let canon = governed(Rule::Consent);
    let (standing, _) = Standing::cited(
        &canon,
        "take the 8am rotation",
        vec![canon_core::Position::of(
            canon.active().next().unwrap().id.clone(),
            Pull::Against,
            "the rotation starts at 8",
        )],
    );
    let consent = Rule::Consent.decide(&standing, &Attributes::default(), &canon);
    assert_eq!(consent.authority, Authority::Refuse);
    let text = render(Profile::Code, &canon, &standing, &consent, None);
    assert!(text.starts_with("CONFLICT"), "{text}");
    assert!(text.contains("not under this policy"), "{text}");
    assert!(
        text.contains("consent:"),
        "the rule that fired is named: {text}"
    );

    let lenient = Rule::Threshold { against: 2 }.decide(&standing, &Attributes::default(), &canon);
    let text = render(Profile::Code, &canon, &standing, &lenient, None);
    assert!(text.starts_with("SUPPORTED"), "{text}");
    assert!(text.contains("1 against, 2 needed"), "{text}");
}

#[test]
fn a_canon_that_adopted_no_rule_is_not_told_about_the_ladder() {
    // The fluency regression this closes. A fresh house canon printed "ask
    // one person with standing" to housemates who had never granted standing
    // and had never met the word, followed by "default: at least one
    // commitment pulls against" — the internal rule's name restating the
    // verdict two lines above it.
    //
    // Under the shipped default the authority IS the outcome: supported means
    // act, anything else means ask a person. Printing it says nothing and
    // costs a reader a vocabulary lesson.
    let (canon, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    assert!(canon.policies.is_empty(), "nobody adopted anything");
    let text = render(
        Profile::House,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(text.contains("NEEDS AN AMENDMENT"), "{text}");
    assert!(
        text.contains("canon supersede"),
        "the concrete act still shows"
    );
    for jargon in ["standing", "default:", "ask one person"] {
        assert!(!text.contains(jargon), "still saying `{jargon}`:\n{text}");
    }
}

#[test]
fn adopting_a_rule_makes_the_ladder_appear_and_keeps_it_visible() {
    // The other half, and it is the load-bearing one: once a group HAS
    // decided how it decides, the authority is what they decided, so it
    // prints even when it agrees with you. A rule that is invisible whenever
    // it agrees is one nobody notices they are governed by.
    let canon = governed(Rule::Consent);
    let (standing, _) = Standing::cited(
        &canon,
        "p",
        vec![canon_core::Position::of(
            canon.active().next().unwrap().id.clone(),
            Pull::Toward,
            "it serves it",
        )],
    );
    let d = Rule::Consent.decide(&standing, &Attributes::default(), &canon);
    let text = render(Profile::Code, &canon, &standing, &d, None);
    assert!(text.starts_with("SUPPORTED"));
    assert!(text.contains("\nact\n"), "{text}");
    assert!(text.contains("consent: no objection"), "{text}");

    // And the machine surface carries the ladder either way — an agent wants
    // it whether or not a person would have found it noise.
    let bare = canon_of(&TEXTS);
    let (bare_standing, _) = Standing::cited(&bare, "p", vec![]);
    let p = payload(
        Profile::Code,
        &bare_standing,
        &shipped(&bare, &bare_standing),
        None,
    );
    assert_eq!(p["authority"], "ask-one", "{p}");
}

#[test]
fn the_personal_profile_still_renders_no_verdict_under_any_policy() {
    // The invariant has to survive the policy layer, including a policy that
    // refuses outright.
    let (canon, standing) = stand(&[(1, "against", "the rotation starts at 8")]);
    let refused = Rule::Consent.decide(&standing, &Attributes::default(), &canon);
    let text = render(Profile::Personal, &canon, &standing, &refused, None);
    for banned in [
        "CONFLICT",
        "SUPPORTED",
        "UNADDRESSED",
        "not under this policy",
    ] {
        assert!(
            !text.contains(banned),
            "personal rendered `{banned}`:\n{text}"
        );
    }
    let p = payload(Profile::Personal, &standing, &refused, None);
    assert!(p.get("outcome").is_none(), "{p}");
    assert!(
        p.get("authority").is_none(),
        "an authority is a verdict too: {p}"
    );
}

#[test]
fn a_position_a_person_took_is_rendered_and_not_silently_dropped() {
    // `cite` quotes a commitment. An actor-sourced position has no commitment
    // to quote, and returning an empty string for it made a vote vanish from
    // the very surface a person reads to see the votes.
    let canon = canon_of(&TEXTS);
    let positions = vec![
        canon_core::Position::by("human:dana", Pull::Against, "school run until 8:30"),
        canon_core::Position::by("human:sam", Pull::Toward, "works for me"),
    ];
    let (standing, refused) = Standing::cited(&canon, "move standup to 8am", positions);
    assert!(refused.is_empty());
    let text = render(
        Profile::Code,
        &canon,
        &standing,
        &shipped(&canon, &standing),
        None,
    );
    assert!(text.contains("human:dana"), "{text}");
    assert!(text.contains("objects"), "{text}");
    assert!(text.contains("school run until 8:30"), "{text}");
    assert!(
        text.contains("human:sam"),
        "the supporting vote too: {text}"
    );
}

#[test]
fn a_subject_left_unwritten_on_purpose_is_not_reported_as_a_gap() {
    // The métis floor at the surface that matters. `UNADDRESSED` plus "write
    // a rule" is exactly the prompt that turns a working unwritten practice
    // into a rota nobody wanted.
    let canon = canon_of(&TEXTS);
    let (standing, _) = Standing::cited(&canon, "who cooks on a wednesday", vec![]);
    let decision = shipped(&canon, &standing);
    assert_eq!(decision.outcome, Outcome::Unaddressed);

    let plain = render(Profile::House, &canon, &standing, &decision, None);
    assert!(plain.contains("NEEDS A NEW RULE"), "{plain}");

    let s = canon_core::Silence {
        about: "who cooks on a wednesday".into(),
        rationale: "it works, and writing it down would turn it into a rota".into(),
        at: 1_767_225_600,
        actor: "human:alex".into(),
        act: canon_core::ActId::from_raw("can-000000000001"),
    };
    let quiet = render(Profile::House, &canon, &standing, &decision, Some(&s));
    assert!(quiet.contains("UNWRITTEN ON PURPOSE"), "{quiet}");
    assert!(!quiet.contains("NEEDS A NEW RULE"), "{quiet}");
    assert!(
        quiet.contains("turn it into a rota"),
        "it says what it protects"
    );
    // Still revisitable — a silence is an act like any other, not a lock.
    assert!(quiet.contains("canon undo"), "{quiet}");

    // The machine surface carries it too, or an agent would read the same
    // silence as a gap and propose a rule.
    let p = payload(Profile::House, &standing, &decision, Some(&s));
    assert!(p.get("silence").is_some(), "{p}");
    assert_eq!(p["outcome"], "unaddressed", "the outcome is still honest");
}
