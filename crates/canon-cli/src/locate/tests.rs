// SPDX-License-Identifier: AGPL-3.0-or-later
//! `locate` tests. The one that matters most is structural: a citation is a
//! slice of its passage, so there is no input on which it can be a
//! paraphrase.

use super::*;

/// Real text, copied from `fixtures/des-moines-noise`. Hard-wrapped by the
/// PDF extraction, numbered the way an ordinance is, and carrying the
/// abbreviations that a naive splitter cuts in half.
const ORDINANCE: &str = "\
(2) Type \"B\" permit—Parks located in residential zones. A type \"B\" permit may be used for sound
equipment emitting music or human speech registering not more than 65 dBAs when measured
at the real property boundary or at a distance of 50 feet from the sound equipment, whichever
distance is closer to the sound equipment. Sound equipment permitted under a type \"B\" permit
may be used only in public parks owned and operated by the city or public grounds owned and
operated by another governmental body, located in a residentially zoned district from 9:00 a.m.
to the time the park closes for events authorized and approved by the park and recreation board
 or other body having jurisdiction over the park or public grounds. A type \"B\" permit will be
issued for one day up to one week with the days to be designated on the permit application.";

const EXCEPTIONS: &str = "\
This article shall not apply to the following:
(1) The emission of sound for the purpose of alerting persons to the existence of an emergency.
(2) The emission of sound in the performance of emergency work.
(6) Snowmobiles regulated by I.C. ch. 321G.";

const TABLE: &str = "\
Table 1. Sound Levels By Receiving Land Use

| Zoning Category of Receiving Land Use | Time | Sound Level Limit, dBA |
| Residential zones: R1-80 to R-6,R-HD and a residential PUD | 7:00 a.m. to 10:00 p.m. | 60 |
| Mixed use and commercial zones: PUD to C-4 | At all times | 65 |";

// ── the invariant the module exists for ─────────────────────

#[test]
fn every_citation_is_a_slice_of_its_passage() {
    // Not "checked against" — IS. Exhaustive over every span this text has,
    // singly and in every legal combination, because the promise is that no
    // input produces a citation the passage does not contain.
    for text in [ORDINANCE, EXCEPTIONS, TABLE] {
        let spans = sentences(text);
        assert!(!spans.is_empty(), "no sentences in:\n{text}");
        for first in 1..=spans.len() {
            for last in first..=(first + SPAN_MAX - 1).min(spans.len()) {
                if let Ok(quoted) = cite(text, &spans, first, last) {
                    assert!(
                        text.contains(&quoted),
                        "cite({first},{last}) is not in its passage:\n{quoted}"
                    );
                }
            }
        }
    }
}

#[test]
fn spans_ascend_and_never_overlap() {
    // A coordinate system is only one if the positions are disjoint and
    // ordered: `[3]` in the prompt must be the third thing a reader sees.
    for text in [ORDINANCE, EXCEPTIONS, TABLE] {
        let spans = sentences(text);
        for w in spans.windows(2) {
            assert!(w[0].end <= w[1].start, "{:?} overlaps {:?}", w[0], w[1]);
        }
        assert!(spans.iter().all(|s| s.start < s.end && s.end <= text.len()));
    }
}

#[test]
fn the_model_is_shown_the_positions_the_code_copies_from() {
    // One segmentation, two readers (§10.6). If the prompt numbered anything
    // else, an index would name one sentence and copy another — a citation
    // that is verbatim and about the wrong rule, which is worse than a
    // paraphrase because nothing downstream can see it.
    let spans = sentences(ORDINANCE);
    let shown = numbered(ORDINANCE, &spans);
    let flat = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
    for (i, line) in shown.lines().enumerate() {
        let body = line.split_once(' ').unwrap().1;
        assert_eq!(line.split(' ').next().unwrap(), format!("[{}]", i + 1));
        assert_eq!(
            body,
            flat(&ORDINANCE[spans[i].clone()]),
            "position {}",
            i + 1
        );
    }
    assert_eq!(shown.lines().count(), spans.len());
}

#[test]
fn a_span_carries_the_document_between_its_sentences() {
    // Sentences 1 and 2 are cited together; what separated them in the
    // source comes along, so the result is still one contiguous slice.
    let spans = sentences(ORDINANCE);
    let quoted = cite(ORDINANCE, &spans, 1, 2).unwrap();
    assert!(quoted.starts_with("(2) Type \"B\" permit"), "{quoted}");
    assert!(quoted.contains("whichever\ndistance is closer"), "{quoted}");
    assert!(ORDINANCE.contains(&quoted));
}

// ── the segmenter, against text that broke the old path ─────

#[test]
fn a_sentence_wrapped_by_the_pdf_extractor_stays_one_sentence() {
    // The line breaks here are the extractor's, not the drafter's. Cutting
    // on them would number fragments and every citation would be a clause.
    let spans = sentences(ORDINANCE);
    let s = &ORDINANCE[spans[1].clone()];
    assert!(s.starts_with("A type \"B\" permit may be used"), "{s}");
    assert!(s.ends_with("closer to the sound equipment."), "{s}");
    assert!(s.contains('\n'), "expected a wrapped sentence: {s}");
}

#[test]
fn the_documents_own_numbering_starts_a_sentence() {
    // "…apply to the following:" ends in a colon, so nothing in prose marks
    // the boundary. The list markers do, and they are the document's.
    let spans = sentences(EXCEPTIONS);
    let text = |i: usize| &EXCEPTIONS[spans[i].clone()];
    let all: Vec<&str> = spans.iter().map(|s| &EXCEPTIONS[s.clone()]).collect();
    assert_eq!(spans.len(), 4, "{all:#?}");
    assert!(text(0).ends_with("the following:"));
    assert!(text(1).starts_with("(1) The emission"));
    assert!(text(2).starts_with("(2) The emission"));
}

#[test]
fn an_abbreviation_does_not_end_a_sentence() {
    // `I.C. ch. 321G` is one citation to the Iowa Code, and `9:00 a.m.` is
    // one clock time. A splitter that cut on either would leave `I.` as a
    // citable position.
    let spans = sentences(EXCEPTIONS);
    let last = &EXCEPTIONS[spans[3].clone()];
    assert_eq!(last, "(6) Snowmobiles regulated by I.C. ch. 321G.");

    let spans = sentences(ORDINANCE);
    let joined: Vec<&str> = spans.iter().map(|s| &ORDINANCE[s.clone()]).collect();
    assert!(
        joined
            .iter()
            .any(|s| s.contains("from 9:00 a.m.\nto the time")),
        "{joined:#?}"
    );
}

#[test]
fn a_table_row_is_a_position_of_its_own() {
    // A limit lives in one row. Citing the whole table would evidence every
    // limit equally, which is to evidence none of them.
    let spans = sentences(TABLE);
    let rows: Vec<&str> = spans
        .iter()
        .map(|s| &TABLE[s.clone()])
        .filter(|s| s.starts_with('|'))
        .collect();
    assert_eq!(rows.len(), 3, "{spans:#?}");
    assert!(
        rows[2].contains("Mixed use and commercial zones"),
        "{rows:#?}"
    );
}

#[test]
fn a_decimal_point_ends_nothing() {
    let text = "The fee is 42.50 dollars per event. Payment is due on filing.";
    let spans = sentences(text);
    assert_eq!(spans.len(), 2, "{spans:#?}");
    assert_eq!(
        &text[spans[0].clone()],
        "The fee is 42.50 dollars per event."
    );
}

#[test]
fn prose_with_no_markers_still_splits() {
    // Journals and charters have none of an ordinance's structure. The
    // sentence rule has to carry them on its own.
    let text = "Quiet hours run from 11:00 PM until 7:00 AM. During quiet hours, music must\nbe played through headphones.";
    let spans = sentences(text);
    assert_eq!(spans.len(), 2, "{spans:#?}");
    assert_eq!(
        &text[spans[1].clone()],
        "During quiet hours, music must\nbe played through headphones."
    );
}

// ── what code refuses ───────────────────────────────────────

#[test]
fn an_index_the_passage_does_not_have_is_refused() {
    let spans = sentences(EXCEPTIONS);
    let err = cite(EXCEPTIONS, &spans, 12, 12).unwrap_err();
    assert_eq!(
        err,
        Miscited::OutOfRange {
            first: 12,
            last: 12,
            have: 4
        }
    );
    assert_eq!(err.to_string(), "cited sentence 12, but the passage has 4");
    // Zero is not a position either — a missing field must not read as one.
    assert!(cite(EXCEPTIONS, &spans, 0, 0).is_err());
}

#[test]
fn a_backwards_span_is_refused_rather_than_swapped() {
    // Reordering the model's answer would be repairing an answer nobody can
    // see is broken (§18.3). It is refused, counted, and reported.
    let spans = sentences(EXCEPTIONS);
    assert_eq!(
        cite(EXCEPTIONS, &spans, 3, 2).unwrap_err(),
        Miscited::Backwards { first: 3, last: 2 }
    );
}

#[test]
fn a_citation_wider_than_the_cap_is_refused() {
    let spans = sentences(ORDINANCE);
    assert_eq!(
        cite(ORDINANCE, &spans, 1, 4).unwrap_err(),
        Miscited::TooWide { n: 4 }
    );
    assert!(cite(ORDINANCE, &spans, 1, SPAN_MAX).is_ok());
}

#[test]
fn a_span_too_short_to_be_evidence_is_refused() {
    let text = "Be quiet.\n\nQuiet hours run from 11:00 PM until 7:00 AM.";
    let spans = sentences(text);
    assert_eq!(
        cite(text, &spans, 1, 1).unwrap_err(),
        Miscited::TooShort { chars: 9 }
    );
    assert!(cite(text, &spans, 2, 2).is_ok());
}

#[test]
fn a_passage_with_no_sentence_break_is_one_position() {
    // Never zero: a chunk the segmenter cannot cut is still citable whole,
    // which is the citation the chunk id already carries.
    let text = "no terminator anywhere in this passage at all";
    let spans = sentences(text);
    assert_eq!(spans.len(), 1);
    assert_eq!(&text[spans[0].clone()], text);
}

// ── what real corpora broke, kept broken ────────────────────

/// Real text, `fixtures/des-moines-noise`. Two enumerated sub-items under a
/// lettered one, and a table with a numbered caption.
const ENUMERATED: &str = "\
b. The operation of the following domestic power tools or equipment between the hours of
7:00 a.m. and 10:00 p.m.:
1. Electrical power tools.
2. Motor-powered, muffler-equipped lawn, garden and tree trimming equipment.

Table 1. Sound Levels By Receiving Land Use";

/// Real text, `fixtures/maple-house`. The wrap puts a four-letter word at the
/// start of a line, followed by a period.
const WRAPPED: &str = "\
A personal item left in a common area for more
than one full day may be moved by any other member to the owner's bedroom
door. Perishable personal food left out in a common area may be thrown away
after a day.";

#[test]
fn an_enumerator_stays_with_the_item_it_labels() {
    // `1.` cut from `Electrical power tools.` leaves a position that is a
    // marker and a position that is a rule with no number, and the model can
    // cite either.
    let spans = sentences(ENUMERATED);
    let all: Vec<&str> = spans.iter().map(|s| &ENUMERATED[s.clone()]).collect();
    assert!(all.contains(&"1. Electrical power tools."), "{all:#?}");
    assert!(
        all.contains(&"Table 1. Sound Levels By Receiving Land Use"),
        "{all:#?}"
    );
    assert!(!all.iter().any(|s| s.trim() == "1."), "{all:#?}");
}

#[test]
fn a_sentence_ending_in_a_number_still_ends() {
    // The counterexample to the rule above: same shape, mid-line, and a real
    // sentence boundary. Position is the only thing that separates them.
    let text = "…sound levels in excess of those shown in table 3. If the sound has not abated                 within a reasonable time, the official may apply to the court.";
    let spans = sentences(text);
    assert_eq!(spans.len(), 2, "{spans:#?}");
    assert!(text[spans[0].clone()].ends_with("shown in table 3."));
}

#[test]
fn a_wrapped_line_starting_with_a_short_word_is_not_a_marker() {
    // `door.` opening a line read as an enumerator, which cut the verb off
    // the rule it belonged to and made the rule uncitable in one piece.
    let spans = sentences(WRAPPED);
    let all: Vec<&str> = spans.iter().map(|s| &WRAPPED[s.clone()]).collect();
    assert_eq!(all.len(), 2, "{all:#?}");
    assert!(
        all[0].ends_with("to the owner's bedroom\ndoor."),
        "{all:#?}"
    );
    assert!(!all.iter().any(|s| s.trim() == "door."), "{all:#?}");
}
