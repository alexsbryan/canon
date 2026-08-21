// SPDX-License-Identifier: AGPL-3.0-or-later
//! `draft` tests. The two that matter most are the invariants the phase
//! exists to make structural: a paraphrase never becomes a citation, and the
//! reduce step never mints text.

use serde_json::json;

use super::*;
use crate::testing::{completion, Mock};

const DOC: &str = "\
# Article II — Quiet Hours

Quiet hours run from 11:00 PM until 7:00 AM. During quiet hours, music must
be played through headphones.

# Article III — Kitchen Cleanup

The kitchen must be cleaned by whoever used it, immediately after that
person finishes cooking.
";

#[test]
fn chunking_never_merges_across_a_heading() {
    // Two short articles are each well under the chunk target, so a
    // size-driven chunker would glue them together and every candidate from
    // the pair would cite the wrong article.
    let chunks = chunk_text("house.md", DOC);
    assert_eq!(chunks.len(), 2, "{chunks:#?}");
    assert!(chunks[0].text.contains("Quiet hours"));
    assert!(!chunks[0].text.contains("kitchen"));
    assert_eq!(
        chunks[0].heading.as_deref(),
        Some("Article II — Quiet Hours")
    );
    assert_eq!(
        chunks[1].heading.as_deref(),
        Some("Article III — Kitchen Cleanup")
    );
}

#[test]
fn a_chunk_cites_the_lines_it_came_from() {
    let chunks = chunk_text("house.md", DOC);
    assert_eq!(chunks[0].source, "house.md:3-4");
    assert_eq!(chunks[1].source, "house.md:8-9");
}

#[test]
fn unstructured_prose_still_chunks_and_still_cites() {
    // Journals have no headings. The heading rule must cost nothing there.
    let text = "a\n\nb\n\nc\n";
    let chunks = chunk_text("journal.md", &format!("{}{}", "x".repeat(60), text));
    assert!(!chunks.is_empty());
    assert!(chunks.iter().all(|c| c.heading.is_none()));
    assert!(chunks[0].source.starts_with("journal.md:"));
}

fn extracted(items: &[(&str, &str)]) -> String {
    completion(
        &json!({
            "commitments": items
                .iter()
                .map(|(t, q)| json!({ "text": t, "quote": q }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

#[test]
fn a_paraphrased_quote_drops_the_candidate() {
    // Cite-or-abstain, enforced in code rather than asked of the model
    // (§7.6). A commitment whose quote is not in the passage is the model
    // inventing a rule the user never wrote.
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(
        200,
        extracted(&[
            (
                "Quiet hours run from 11 PM to 7 AM.",
                "Quiet hours run from 11:00 PM until 7:00 AM",
            ),
            (
                "Noise is discouraged in the evening.",
                "the house prefers a peaceful evening atmosphere",
            ),
        ]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0]).unwrap();
    assert_eq!(kept.len(), 1, "{kept:#?}");
    assert_eq!(kept[0].source, "house.md:3-4");
    assert_eq!(dropped.len(), 1);
    assert!(
        dropped[0].reason.contains("not in the passage"),
        "{dropped:#?}"
    );
}

#[test]
fn a_quote_reflowed_across_lines_still_cites() {
    // The passage wraps mid-sentence; a model quoting it returns one line.
    // Same words, so the same citation — whitespace is not evidence.
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(
        200,
        extracted(&[(
            "Music is played through headphones during quiet hours.",
            "During quiet hours, music must be played through headphones.",
        )]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0]).unwrap();
    assert_eq!(kept.len(), 1, "dropped: {dropped:#?}");
}

#[test]
fn a_quote_too_short_to_be_evidence_is_refused() {
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(200, extracted(&[("Be quiet.", "quiet")]))]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0]).unwrap();
    assert!(kept.is_empty());
    assert!(dropped[0].reason.contains("too short"));
}

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.into(),
        quote: format!("verbatim: {text}"),
        chunk: 0,
        source: "house.md:1-2".into(),
    }
}

#[test]
fn the_reduce_step_keeps_the_first_of_each_group_with_its_citation() {
    // The model returns POSITIONS, never text. A reduce allowed to rewrite
    // would produce a tidier list whose quotations no longer match anything.
    let cands = vec![
        candidate("Quiet hours start at 11 PM."),
        candidate("Kitchen is cleaned by whoever used it."),
        candidate("Quiet time begins at eleven at night."),
    ];
    let mock = Mock::spawn(vec![(
        200,
        completion(&json!({ "groups": [[1, 3]] }).to_string()),
    )]);
    let (groups, kept) = dedupe(&mock.client(), &cands).unwrap();
    assert_eq!(groups, vec![vec![0, 2]]);
    assert_eq!(kept, vec![0, 1]);
    // The survivor is the ORIGINAL candidate, citation intact.
    assert_eq!(
        cands[kept[0]].quote,
        "verbatim: Quiet hours start at 11 PM."
    );
}

#[test]
fn a_group_naming_a_position_that_does_not_exist_is_dropped_not_wrapped() {
    let cands = vec![candidate("a"), candidate("b")];
    let mock = Mock::spawn(vec![(
        200,
        completion(&json!({ "groups": [[1, 99]] }).to_string()),
    )]);
    let (groups, kept) = dedupe(&mock.client(), &cands).unwrap();
    // Only one valid member survives the filter, so it is not a group at all.
    assert!(groups.is_empty(), "{groups:?}");
    assert_eq!(kept, vec![0, 1]);
}

#[test]
fn from_takes_every_path_a_shell_expanded() {
    let args: Vec<String> = ["--from", "a.md", "b.md", "c.md", "--dry-run"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    assert_eq!(from_paths(&args), vec!["a.md", "b.md", "c.md"]);
}

#[test]
fn there_is_no_accept_all() {
    // Pinned, because the pressure to add it is real and the reason not to is
    // a product decision rather than an oversight.
    let args: Vec<String> = vec!["--accept-all".into()];
    assert_eq!(run(&args), 2);
}
