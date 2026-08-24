// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn three() -> Offered {
    Offered::new(
        vec!["first".into(), "second".into(), "third".into()],
        "rule",
    )
}

#[test]
fn an_answer_naming_something_that_was_not_offered_is_dropped_and_never_clamped() {
    // The failure this prevents is a confident citation of the WRONG rule.
    // Clamping 99 to the last item reads as an answer and is not one.
    let o = three();
    assert_eq!(o.at(1), Some(0));
    assert_eq!(o.at(3), Some(2));
    assert_eq!(o.at(0), None, "there is no rule zero");
    assert_eq!(o.at(4), None);
    assert_eq!(o.at(99), None);
    assert_eq!(
        o.refused(),
        3,
        "and every refusal is counted, not swallowed"
    );
}

#[test]
fn the_numbering_the_model_sees_is_the_numbering_that_is_checked() {
    let o = three();
    assert_eq!(o.numbered(), "1. first\n2. second\n3. third\n");
    // The bracket form exists because municipal drafting numbers its own
    // paragraphs `(1)` and `a.`, and two coordinate systems in one prompt is
    // an ambiguity the model resolves however it likes.
    let b = three().marked(Mark::Bracket);
    assert_eq!(b.numbered(), "[1] first\n[2] second\n[3] third\n");
}

#[test]
fn flattening_is_for_the_prompt_and_not_for_the_evidence() {
    let o = Offered::new(vec!["a rule\nsplit over\nlines".into()], "rule").flattened();
    assert_eq!(o.numbered(), "1. a rule split over lines\n");
    // Unflattened is the default, because most callers show text that is
    // already one line and reflowing it would be a change nobody asked for.
    let raw = Offered::new(vec!["a rule\nsplit".into()], "rule");
    assert_eq!(raw.numbered(), "1. a rule\nsplit\n");
}

#[test]
fn an_empty_offer_accepts_nothing() {
    let o = Offered::new(Vec::new(), "rule");
    assert!(o.is_empty());
    assert_eq!(o.at(1), None);
    assert_eq!(o.numbered(), "");
}
