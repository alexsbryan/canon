// SPDX-License-Identifier: AGPL-3.0-or-later
//! `draft` tests. The two that matter most are the invariants the phase
//! exists to make structural: a citation is cut from the passage rather than
//! retyped by the model, and the reduce step never mints text.

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

/// One scripted extraction: `(first marker, last marker, rule text)`. The
/// model answers with a POSITION, so a test cannot hand it a quote — which is
/// the property under test.
fn extracted(items: &[(usize, usize, &str)]) -> String {
    completion(
        &json!({
            "commitments": items
                .iter()
                .map(|(f, l, t)| json!({ "kind": "rule", "first": f, "last": l, "text": t }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

/// The same, with a kind and an optional reason — for the two act kinds
/// beyond a commitment.
fn extracted_kinds(items: &[(&str, usize, usize, &str, &str)]) -> String {
    completion(
        &json!({
            "commitments": items
                .iter()
                .map(|(k, f, l, t, because)| json!({
                    "kind": k, "first": f, "last": l, "text": t, "because": because
                }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
}

/// A chunk whose first sentence is too small to evidence anything.
const SHORT: &str = "# House\n\nBe quiet. Quiet hours run from 11:00 PM until 7:00 AM.\n";

#[test]
fn the_citation_is_cut_from_the_passage_not_taken_from_the_reply() {
    // Cite-or-abstain, enforced in code rather than asked of the model
    // (§7.6) — and now by construction rather than by check. The model says
    // WHERE; the quote is a slice of the chunk, so it carries the passage's
    // own wording ("until 7:00 AM") and not the rule's ("to 7 AM").
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(
        200,
        extracted(&[(1, 1, "Quiet hours run from 11 PM to 7 AM.")]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert_eq!(kept.len(), 1, "{dropped:#?}");
    assert_eq!(kept[0].source, "house.md:3-4");
    assert_eq!(
        kept[0].quote,
        "Quiet hours run from 11:00 PM until 7:00 AM."
    );
    assert!(chunks[0].text.contains(&kept[0].quote));
}

#[test]
fn a_citation_keeps_the_line_breaks_the_passage_had() {
    // The passage wraps mid-sentence. A model asked to retype it reflows;
    // copying cannot, so the citation is byte-for-byte the source.
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(
        200,
        extracted(&[(
            2,
            2,
            "Music is played through headphones during quiet hours.",
        )]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert_eq!(kept.len(), 1, "dropped: {dropped:#?}");
    assert_eq!(
        kept[0].quote,
        "During quiet hours, music must\nbe played through headphones."
    );
}

#[test]
fn a_rule_spanning_two_sentences_cites_both() {
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(
        200,
        extracted(&[(1, 2, "Quiet hours run 11 PM to 7 AM, headphones only.")]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert_eq!(kept.len(), 1, "dropped: {dropped:#?}");
    assert_eq!(kept[0].quote, chunks[0].text);
}

#[test]
fn a_position_the_passage_does_not_have_drops_the_candidate() {
    // The one citation failure still reachable, and the reason names both
    // what was asked for and what was on offer.
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(
        200,
        extracted(&[(9, 9, "Quiet hours run from 11 PM to 7 AM.")]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert!(kept.is_empty(), "{kept:#?}");
    assert_eq!(dropped[0].reason, "cited sentence 9, but the passage has 2");
    // Nothing to quote, and nothing invented to stand in for one.
    assert!(dropped[0].quote.is_empty());
}

#[test]
fn a_cited_sentence_too_short_to_be_evidence_is_refused() {
    let chunks = chunk_text("house.md", SHORT);
    let mock = Mock::spawn(vec![(200, extracted(&[(1, 1, "Be quiet.")]))]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert!(kept.is_empty());
    assert!(dropped[0].reason.contains("too short"), "{dropped:#?}");
}

/// What the reading pass answers for a batch of rules, 1-based.
/// One scripted reading: `(rule number, [(value, unit, of, canonical)])`.
type Reading<'a> = (usize, &'a [(&'a str, &'a str, &'a str, &'a str)]);

fn read_as(rules: &[Reading]) -> String {
    completion(
        &json!({
            "rules": rules.iter().map(|(n, qs)| json!({
                "n": n,
                "quantities": qs.iter()
                    .map(|(v, u, o, c)| json!({ "value": v, "unit": u, "of": o, "canonical": c }))
                    .collect::<Vec<_>>(),
            })).collect::<Vec<_>>()
        })
        .to_string(),
    )
}

/// Rule readings as `support` hands them on: one list per candidate, in the
/// same order. The fold guard indexes straight into this, so a test supplies
/// one entry per candidate whether or not that rule states anything.
fn quantified(rules: &[&[(&str, &str, &str, &str)]]) -> Vec<Vec<quantify::Quantity>> {
    rules
        .iter()
        .map(|qs| {
            qs.iter()
                .map(|(v, u, o, c)| quantify::Quantity {
                    value: (*v).into(),
                    unit: (*u).into(),
                    of: (*o).into(),
                    canonical: (*c).into(),
                })
                .collect()
        })
        .collect()
}

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.into(),
        quote: format!("verbatim: {text}"),
        chunk: 0,
        source: "house.md:1-2".into(),
        kind: Kind::Rule,
        because: String::new(),
        sample: 0,
    }
}

/// One reading's finding: which sample said it, and the words it cited.
fn read(sample: usize, quote: &str, text: &str) -> Candidate {
    Candidate {
        text: text.into(),
        quote: quote.into(),
        chunk: 0,
        source: "house.md:1-2".into(),
        kind: Kind::Rule,
        because: String::new(),
        sample,
    }
}

/// One scripted partition of a proposed group: for each member, the member
/// it governs the same thing as. `dedupe` asks this before it folds anything,
/// so every test that reaches the fold scripts a second response per group.
fn named(same_as: &[usize]) -> String {
    completion(
        &json!({
            "rules": same_as
                .iter()
                .enumerate()
                .map(|(i, s)| json!({ "n": i + 1, "same_as": s }))
                .collect::<Vec<_>>()
        })
        .to_string(),
    )
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
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "groups": [[1, 3]] }).to_string())),
        (200, named(&[1, 1])),
    ]);
    // Both state the same hour, so nothing keeps them apart.
    let read = quantified(&[
        &[("11", "PM", "quiet hours start", "23:00")],
        &[],
        &[("eleven", "at night", "quiet hours start", "23:00")],
    ]);
    let (groups, kept) = dedupe(&mock.client(), &cands, &read).unwrap();
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
    let (groups, kept) = dedupe(&mock.client(), &cands, &quantified(&[&[], &[]])).unwrap();
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

#[test]
fn a_directory_is_read_in_a_stable_order() {
    // Chunk ids are positions, so a folder read in a different order twice
    // produces two artifacts that cannot be compared. Sorted, always.
    let root = std::env::temp_dir().join("canon-draft-walk");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    for (p, body) in [
        ("b.md", "second".as_bytes()),
        ("a.md", "first".as_bytes()),
        ("notes.bin", &[0xff, 0xfe, 0x00][..]),
        ("sub/c.txt", "third".as_bytes()),
        (".git/config", "not notes".as_bytes()),
    ] {
        std::fs::write(root.join(p), body).unwrap();
    }
    let mut got = crate::sources::Gathered::default();
    crate::sources::gather(&root, &mut got, false).unwrap();
    let names: Vec<String> = got.sources.iter().map(|s| s.name.clone()).collect();
    assert_eq!(names, vec!["a.md", "b.md", "sub/c.txt"], "{names:?}");
    // And the one it passed over is REPORTED. A folder with some readable
    // files used to drop the rest in silence, which is how a Slack export
    // sitting beside three documents was never opened by anyone.
    assert_eq!(got.skipped.get("not text"), Some(&1), "{:?}", got.skipped);
}

#[test]
fn the_extraction_asks_for_the_voice_the_canon_is_written_in() {
    // A house charter extracted without this came back as "I do not leave
    // dirty dishes in the sink" — one member's habit, not a house rule. The
    // profile is already known; using it costs nothing.
    let chunks = chunk_text("house.md", DOC);
    for (profile, expect, reject) in [
        (Profile::House, "household's rules", "one person's own"),
        (Profile::Personal, "one person's own", "household's rules"),
        (Profile::Code, "codebase's standards", "household's rules"),
    ] {
        let mock = Mock::spawn(vec![(200, extracted(&[]))]);
        extract(&mock.client(), &chunks[0], profile).unwrap();
        let system = mock.requests()[0]["messages"][0]["content"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(system.contains(expect), "{profile:?} prompt: {system}");
        assert!(!system.contains(reject), "{profile:?} prompt: {system}");
    }
}

// ── support: the citation has to carry the rule's numbers ───

#[test]
fn a_rule_stating_a_number_its_citation_does_not_is_dropped() {
    // The citation is verbatim by construction now, so this is the failure
    // that remains: the words are the passage's and the RULE is not. Observed
    // against a live endpoint — a candidate reading "at least three hours in
    // advance" over a sentence that said "three days ahead". Worse than a
    // missing rule, because the citation makes it look checked.
    let cands = vec![candidate(
        "Guests are announced at least three hours in advance.",
    )];
    // ONE call: the rule at 1 and its citation at 2, read together so their
    // canonical forms are agreed against each other rather than by luck.
    let mock = Mock::spawn(vec![(
        200,
        read_as(&[
            (1, &[("three", "hours", "notice", "3 hour")]),
            (2, &[("three", "days", "notice", "3 day")]),
        ]),
    )]);
    let Supported {
        candidates: kept,
        quantities,
        dropped,
    } = support(&mock.client(), cands).unwrap();
    assert!(kept.is_empty(), "{kept:#?}");
    assert!(quantities.is_empty());
    assert_eq!(dropped.len(), 1);
    assert!(
        dropped[0].reason.contains("which its citation does not"),
        "{:?}",
        dropped[0].reason
    );
}

#[test]
fn a_number_worded_differently_from_its_citation_survives() {
    // "11 PM" against a citation reading "11:00 p.m." is one instant in two
    // spellings. The canonical form makes them one without anyone keeping a
    // table of clock formats — which is the whole reason this stopped being
    // `measure.rs`.
    let cands = vec![candidate("Quiet hours start at 11 PM.")];
    let mock = Mock::spawn(vec![(
        200,
        read_as(&[
            (1, &[("11", "PM", "quiet hours start", "23:00")]),
            (2, &[("11:00", "p.m.", "quiet hours start", "23:00")]),
        ]),
    )]);
    let Supported {
        candidates: kept,
        quantities,
        dropped,
    } = support(&mock.client(), cands).unwrap();
    assert_eq!(kept.len(), 1, "{dropped:#?}");
    assert_eq!(quantities.len(), 1);
    assert_eq!(quantities[0][0].canonical, "23:00");
}

#[test]
fn a_unit_nobody_listed_is_still_checked_against_the_citation() {
    // dB(A) and dB(C) permit different sound. `measure.rs` read both as
    // stating no measure at all, so a rule could claim either and no guard
    // could tell. Nothing here knows what a decibel is.
    let cands = vec![candidate("Sound equipment may register up to 85 dBCs.")];
    let mock = Mock::spawn(vec![(
        200,
        read_as(&[
            (1, &[("85", "dBC", "sound level", "85 dBC")]),
            (2, &[("85", "dBA", "sound level", "85 dBA")]),
        ]),
    )]);
    let Supported {
        candidates: kept,
        dropped,
        ..
    } = support(&mock.client(), cands).unwrap();
    assert!(kept.is_empty(), "{kept:#?}");
    assert!(
        dropped[0].reason.contains("85 dBC"),
        "{:?}",
        dropped[0].reason
    );
}

#[test]
fn the_surviving_rules_carry_their_own_reading_forward() {
    // The fold guard indexes straight into what this returns. If a dropped
    // candidate left its reading behind, every later rule would be judged
    // against the quantities of the one before it — a misalignment with no
    // symptom until a fold goes wrong, and nothing downstream could see it.
    let cands = vec![
        candidate("Quiet hours start at 11 PM."),
        candidate("Guests are announced three hours ahead."),
        candidate("The kitchen is cleaned after use."),
    ];
    // Three pairs interleaved into one call: rule, citation, rule, citation…
    let mock = Mock::spawn(vec![(
        200,
        read_as(&[
            (1, &[("11", "PM", "quiet hours start", "23:00")]),
            (2, &[("11", "PM", "quiet hours start", "23:00")]),
            (3, &[("three", "hours", "notice", "3 hour")]),
            (4, &[("three", "days", "notice", "3 day")]),
            (5, &[]),
            (6, &[]),
        ]),
    )]);
    let Supported {
        candidates: kept,
        quantities,
        dropped,
    } = support(&mock.client(), cands).unwrap();
    assert_eq!(dropped.len(), 1, "{dropped:#?}");
    assert_eq!(kept.len(), 2);
    assert_eq!(quantities.len(), kept.len());
    assert_eq!(kept[1].text, "The kitchen is cleaned after use.");
    assert_eq!(quantities[0][0].canonical, "23:00");
    assert!(quantities[1].is_empty(), "that rule states no quantity");
}

#[test]
fn nothing_extracted_asks_the_model_nothing() {
    // An empty document must not spend two completions discovering it is
    // empty. Mock scripted for zero requests: a call would hang the test.
    let mock = Mock::spawn(Vec::new());
    let Supported {
        candidates: kept,
        quantities,
        dropped,
    } = support(&mock.client(), Vec::new()).unwrap();
    assert!(kept.is_empty() && quantities.is_empty() && dropped.is_empty());
}

#[test]
fn the_section_title_reaches_the_model_as_context_with_no_position_to_cite() {
    // The chunker recorded the heading from the first commit and never sent
    // it. A minute headed "Decision — 2026-02-10 — Weeknight Quiet Hours"
    // opens "the house met and resolved…" and never names its own subject;
    // read cold, its one operative sentence looks like narrative, and it was
    // dropped.
    let chunks = chunk_text("house.md", DOC);
    let mock = Mock::spawn(vec![(200, extracted(&[]))]);
    extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    let user = mock.requests()[0]["messages"][1]["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(user.contains("Article II — Quiet Hours"), "{user}");
    assert!(user.contains("cannot be cited"), "{user}");
    // Only the body is numbered, so there is no marker that names the title.
    assert!(user.contains("[1] Quiet hours run"), "{user}");
}

#[test]
fn a_passage_with_no_heading_is_sent_plain() {
    // Journals have no headings, and inventing a title would be inventing
    // context the source does not have.
    let chunks = chunk_text("journal.md", &format!("{}\n", "x".repeat(80)));
    let mock = Mock::spawn(vec![(200, extracted(&[]))]);
    extract(&mock.client(), &chunks[0], Profile::Personal).unwrap();
    let user = mock.requests()[0]["messages"][1]["content"]
        .as_str()
        .unwrap()
        .to_string();
    assert!(user.starts_with("Passage:"), "{user}");
}

// ── dedupe must not eat contradictions ──────────────────────

#[test]
fn two_rules_with_different_times_are_never_duplicates() {
    // Observed in a real run: the reduce step folded the 2026-02-10 decision
    // into the Article II charter rule. Both were extracted correctly; the
    // fold alone destroyed planted tensions T5 and T10 — the entire
    // unmarked-supersession category — before any comparison ran.
    let mut cands = vec![
        candidate("Quiet hours run from 11:00 PM to 7:00 AM every night."),
        candidate("Quiet hours begin at 10:00 PM from Sunday through Thursday."),
    ];
    cands.push(candidate("Members keep the back porch tidy."));
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "groups": [[1, 2]] }).to_string())),
        (200, named(&[1, 1])),
    ]);
    let read = quantified(&[
        &[("11:00 PM", "", "quiet hours start", "23:00")],
        &[("10:00 PM", "", "quiet hours start", "22:00")],
        &[],
    ]);
    let (groups, kept) = dedupe(&mock.client(), &cands, &read).unwrap();
    assert!(
        groups.is_empty(),
        "a contradiction is not a duplicate: {groups:?}"
    );
    assert_eq!(kept, vec![0, 1, 2], "both rules survive");
}

#[test]
fn a_genuine_reword_still_folds() {
    // The guard must not cost ordinary deduplication, or it trades one
    // failure for another.
    let cands = vec![
        candidate("Smoking is prohibited anywhere inside the house."),
        candidate("No form of smoking occurs indoors."),
    ];
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "groups": [[1, 2]] }).to_string())),
        (200, named(&[1, 1])),
    ]);
    // Neither states a quantity, so nothing can keep them apart.
    let (groups, kept) = dedupe(&mock.client(), &cands, &quantified(&[&[], &[]])).unwrap();
    assert_eq!(groups, vec![vec![0, 1]]);
    assert_eq!(kept, vec![0]);
}

#[test]
fn the_same_measure_stated_differently_still_folds() {
    let cands = vec![
        candidate("A guest may stay no more than two consecutive nights in any seven-day period."),
        candidate("Guests stay at most 2 nights per 7 days."),
    ];
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "groups": [[1, 2]] }).to_string())),
        (200, named(&[1, 1])),
    ]);
    // "two consecutive nights" and "2 nights" are one limit, and the reading
    // pass says so without anyone listing number words.
    let same: &[(&str, &str, &str, &str)] = &[
        ("2", "nights", "length of stay", "2 night"),
        ("7", "days", "period", "7 day"),
    ];
    let (groups, _) = dedupe(&mock.client(), &cands, &quantified(&[same, same])).unwrap();
    assert_eq!(groups, vec![vec![0, 1]], "same quantities, one rule");
}

#[test]
fn the_same_limit_about_a_different_permit_is_never_a_duplicate() {
    // The Des Moines failure, verbatim from the corpus. A permit schedule
    // restates one sentence per type, so these differ only in a letter: same
    // level, same distance, same wording. The quantity guard has no grounds
    // to refuse — both state 65 dBAs at 50 feet — and folding them deleted
    // the type "C" commitment and the planted supersession against it. Every
    // run of the bar measured 10 of 11 reachable with extraction missing none.
    let cands = vec![
        candidate(
            "Sound equipment under a type \"B\" permit may emit music or human speech \
             registering not more than 65 dBAs when measured at the real property boundary \
             or at a distance of 50 feet from the sound equipment, whichever is closer.",
        ),
        candidate(
            "Sound equipment under a type \"C\" permit may emit music or human speech \
             registering not more than 65 dBAs when measured at the real property boundary \
             or at a distance of 50 feet from the sound equipment, whichever is closer.",
        ),
    ];
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "groups": [[1, 2]] }).to_string())),
        (200, named(&[1, 2])),
    ]);
    let same: &[(&str, &str, &str, &str)] = &[
        ("65", "dBAs", "sound level", "65 dBA"),
        ("50", "feet", "measuring distance", "50 foot"),
    ];
    let (groups, kept) = dedupe(&mock.client(), &cands, &quantified(&[same, same])).unwrap();
    assert!(
        groups.is_empty(),
        "same limit, different permit: {groups:?}"
    );
    assert_eq!(kept, vec![0, 1], "both permits survive the fold");
}

#[test]
fn a_subject_the_model_did_not_name_refuses_the_fold() {
    // Uncertainty resolves toward keeping a rule out of a fold. A refused
    // fold costs precision; a wrong fold destroys a commitment outright, and
    // the bar gates on recall (§18.3).
    let cands = vec![
        candidate("Bins go out Tuesday."),
        candidate("Put bins out Tuesdays."),
    ];
    let mock = Mock::spawn(vec![
        (200, completion(&json!({ "groups": [[1, 2]] }).to_string())),
        // Only the first rule comes back named.
        (200, named(&[1])),
    ]);
    let (groups, kept) = dedupe(&mock.client(), &cands, &quantified(&[&[], &[]])).unwrap();
    assert!(groups.is_empty(), "an unread subject is not agreement");
    assert_eq!(kept, vec![0, 1]);
}

// ── a stage that fails keeps the work before it ─────────────

fn in_flight(chunks: Vec<Chunk>, candidates: Vec<Candidate>) -> DraftRun {
    DraftRun {
        schema: RUN_SCHEMA.into(),
        at: 1,
        endpoint: "http://localhost:9741/v1".into(),
        model: "primary".into(),
        profile: "house".into(),
        sources: vec!["ordinance.md".into()],
        skipped: Default::default(),
        already_read: 0,
        capped: 0,
        chunks,
        candidates,
        dropped: Vec::new(),
        unread: Vec::new(),
        duplicates: Vec::new(),
        kept: Vec::new(),
        tensions: Vec::new(),
        tension_passes: 0,
        tension_passes_unread: Vec::new(),
        tension_arrangements: Vec::new(),
        failed: None,
        samples: 1,
        stopped_after: None,
        replayed_from: None,
        tape: Vec::new(),
    }
}

#[test]
fn a_stage_that_fails_writes_what_ran_before_it() {
    // Observed: a Des Moines sweep died in the comparison stage twenty passes
    // in, having already spent thirty-four extraction calls, and wrote
    // nothing at all. The next attempt paid for those calls again and nobody
    // could see what the run had actually extracted.
    let dir = std::env::temp_dir().join("canon-abandon-keeps-work");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let chunks = chunk_text("house.md", DOC);
    let cands = vec![candidate("Quiet hours start at 11 PM.")];
    let mut artifact = in_flight(chunks, cands);
    let code = abandon(
        &dir,
        &mut artifact,
        "tensions",
        ModelError::Refused {
            status: 503,
            detail: "inference deadline exceeded after 300s".into(),
        },
        &crate::model::Client::replaying("http://x/v1", "primary", Vec::new()),
    );
    assert_ne!(code, 0, "an abandoned run is still a failure to the caller");

    let written: Vec<_> = std::fs::read_dir(dir.join(RUNS_DIR))
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(written.len(), 1, "the partial run is on disk");
    let got: DraftRun =
        serde_json::from_str(&std::fs::read_to_string(written[0].path()).unwrap()).unwrap();

    // The extraction survives...
    assert_eq!(got.candidates.len(), 1);
    assert_eq!(got.chunks.len(), 2);
    // ...and the artifact says why it is not a measurement, naming the stage
    // and the endpoint's own words.
    let why = got.failed.expect("a partial run must say it is one");
    assert!(why.starts_with("tensions:"), "{why}");
    assert!(why.contains("deadline exceeded"), "{why}");
}

#[test]
fn a_finished_run_carries_no_failure_marker() {
    // `failed` is skipped when absent, so the bar's check is "is the field
    // there", and a complete run can never trip it.
    let json = serde_json::to_string(&in_flight(Vec::new(), Vec::new())).unwrap();
    assert!(!json.contains("failed"), "{json}");
}

// ── three kinds, not one ────────────────────────────────────

const MEETING: &str = "\
House meeting, 3 April 2026

Wednesday cooking. Discussed making a rota. Decided NOT to. It has sorted
itself out every week for two years and a rota would turn a kindness into a
duty. Explicitly leaving this unwritten.

Allotment. Nobody has ever said who looks after the allotment. Left open.

Rent is due on the 1st.
";

#[test]
fn a_passage_yields_the_three_kinds_it_actually_records() {
    // **The gap this closes.** A group's normative content is three shapes,
    // and an extractor that could only mint commitments dropped two of them
    // on the floor — so a meeting note that says "decided NOT to write this
    // down" and "nobody has ever said who looks after the allotment"
    // onboarded as one rule about rent.
    let chunks = chunk_text("meeting.txt", MEETING);
    let mock = Mock::spawn(vec![(
        200,
        extracted_kinds(&[
            (
                "silence",
                4,
                6,
                "who cooks on a wednesday",
                "a rota would turn a kindness into a duty",
            ),
            ("question", 8, 9, "Who looks after the allotment?", ""),
            ("rule", 10, 10, "Rent is due on the 1st.", ""),
        ]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert_eq!(kept.len(), 3, "{dropped:#?}");
    assert_eq!(kept[0].kind, Kind::Silence);
    assert_eq!(kept[0].because, "a rota would turn a kindness into a duty");
    assert_eq!(kept[1].kind, Kind::Question);
    assert_eq!(kept[2].kind, Kind::Rule);
    // Every one still carries the passage it came from.
    assert!(kept.iter().all(|c| !c.quote.is_empty()));
}

#[test]
fn a_silence_with_no_stated_reason_is_refused() {
    // Cite-or-abstain, applied to silence. A deliberate silence with no
    // reason cannot be told apart from having forgotten, which is the entire
    // distinction it exists to make — so it is refused at the door rather
    // than written and discovered later.
    let chunks = chunk_text("meeting.txt", MEETING);
    let mock = Mock::spawn(vec![(
        200,
        extracted_kinds(&[("silence", 4, 6, "who cooks on a wednesday", "  ")]),
    )]);
    let (kept, dropped) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert!(kept.is_empty());
    assert_eq!(dropped.len(), 1);
    assert!(
        dropped[0].reason.contains("no stated reason"),
        "{dropped:#?}"
    );
}

#[test]
fn an_unreadable_kind_is_a_rule_and_not_a_way_past_the_guards() {
    // A rule is the kind that has to clear the citation and quantity guards.
    // Reading an unknown word as anything else would make a typo a bypass.
    let chunks = chunk_text("meeting.txt", MEETING);
    let mock = Mock::spawn(vec![(
        200,
        extracted_kinds(&[("stanadrd", 10, 10, "Rent is due on the 1st.", "")]),
    )]);
    let (kept, _) = extract(&mock.client(), &chunks[0], Profile::House).unwrap();
    assert_eq!(kept.len(), 1);
    assert_eq!(kept[0].kind, Kind::Rule);
}

#[test]
fn questions_and_silences_do_not_go_through_the_quantity_guard() {
    // The guard asks whether a rule states a number its citation does not.
    // A question states no number to disagree about, so putting it through
    // would spend a call to compare two empty lists — and would let a stray
    // reading refuse an open question over a limit nobody claimed.
    //
    // Scripted with NO responses at all: if support() called the endpoint,
    // this hangs or errors rather than passing quietly.
    let mock = Mock::spawn(Vec::new());
    let mut q = candidate("Who looks after the allotment?");
    q.kind = Kind::Question;
    let mut sil = candidate("who cooks on a wednesday");
    sil.kind = Kind::Silence;
    sil.because = "it would turn a kindness into a duty".into();

    let got = support(&mock.client(), vec![q, sil]).unwrap();
    assert_eq!(got.candidates.len(), 2);
    assert!(got.dropped.is_empty());
    // Same length and same order as the candidates — the reduce step indexes
    // both, and a mismatch would judge every later rule against the one
    // before it.
    assert_eq!(got.quantities.len(), 2);
    assert!(got.quantities.iter().all(Vec::is_empty));
}

// ── finishing a review later ────────────────────────────────

#[test]
fn resuming_offers_only_what_is_not_already_in_the_canon() {
    // **Why this exists.** There is no `--accept-all` on purpose: accepting
    // one at a time is what makes onboarding the first governance session
    // rather than disengagement at t=0. But a folder of documents yields
    // dozens of candidates, and "finish in one sitting or lose your place" is
    // how somebody quits at candidate nine and never comes back.
    //
    // The run artifact already holds every candidate and every citation, so
    // resuming costs no model call at all.
    use canon_core::{Act, ActKind, Log};

    let mut run = in_flight(Vec::new(), Vec::new());
    let mut q = candidate("Who looks after the allotment?");
    q.kind = Kind::Question;
    let mut sil = candidate("who cooks on a wednesday");
    sil.kind = Kind::Silence;
    sil.because = "it would turn a kindness into a duty".into();
    run.candidates = vec![
        candidate("Quiet hours run 11pm to 7am."),
        candidate("Bikes live in the hall."),
        q,
        sil,
    ];
    run.kept = vec![0, 1, 2, 3];

    let empty = Log::default().derive();
    assert_eq!(
        remaining(&run, &empty, &Seen::default()),
        vec![0, 1, 2, 3],
        "nothing done yet"
    );

    // A session that accepted the first rule, the question and the silence.
    let canon = Log::from_acts(vec![
        Act::new(
            ActKind::Assert {
                text: "Quiet hours run 11pm to 7am.".into(),
                from: None,
                source: None,
            },
            100,
            "human:mira",
        ),
        Act::new(
            ActKind::Question {
                text: "Who looks after the allotment?".into(),
                proposal: None,
            },
            101,
            "human:mira",
        ),
        Act::new(
            ActKind::Silence {
                about: "who cooks on a wednesday".into(),
                rationale: "it would turn a kindness into a duty".into(),
            },
            102,
            "human:mira",
        ),
    ])
    .derive();
    // All three kinds count as done, not just the commitments — otherwise
    // resuming re-offers every question somebody already answered.
    assert_eq!(
        remaining(&run, &canon, &Seen::default()),
        vec![1],
        "only the bikes rule is left"
    );

    // And resuming twice cannot write anything twice.
    let after = Log::from_acts(
        canon
            .active()
            .map(|c| {
                Act::new(
                    ActKind::Assert {
                        text: c.text.clone(),
                        from: None,
                        source: None,
                    },
                    c.asserted_at,
                    c.actor.clone(),
                )
            })
            .chain(std::iter::once(Act::new(
                ActKind::Assert {
                    text: "Bikes live in the hall.".into(),
                    from: None,
                    source: None,
                },
                200,
                "human:mira",
            )))
            .collect::<Vec<_>>(),
    )
    .derive();
    assert!(remaining(&run, &after, &Seen::default())
        .iter()
        .all(|i| *i != 1));
}

#[test]
fn a_candidate_already_declined_is_never_offered_again() {
    // The canon answers for what was ACCEPTED. Without the seen set nothing
    // answers for what was turned down, so re-pointing at the same feed asks
    // again — every morning — about the rule you already said no to.
    let mut run = in_flight(Vec::new(), Vec::new());
    run.candidates = vec![
        candidate("Bikes live in the hall."),
        candidate("Quiet hours begin at 10pm."),
    ];
    run.kept = vec![0, 1];
    let canon = canon_core::Log::default().derive();

    let d = std::env::temp_dir().join("canon-draft-declined");
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut seen = Seen::load(&d);
    seen.record("Quiet hours begin at 10pm.", crate::seen::Why::Rejected)
        .unwrap();

    assert_eq!(remaining(&run, &canon, &seen), vec![0]);
    // And it survives the process that recorded it.
    assert_eq!(remaining(&run, &canon, &Seen::load(&d)), vec![0]);
}

#[test]
fn text_with_no_blank_line_still_chunks() {
    // **Measured before the ceiling existed:** 500 CSV rows became ONE chunk
    // of 15,279 characters and 500 log lines one of 18,389, each sent to the
    // model as a single completion — a 2 MB log would have been one prompt.
    // `CHUNK_TARGET` was a flush threshold with nothing above it, and text
    // with no blank line and no heading never reached a boundary at all.
    for (name, text) in [
        (
            "csv",
            (0..500)
                .map(|i| format!("rule-{i},owner-{i},2024,active\n"))
                .collect::<String>(),
        ),
        (
            "log",
            (0..500)
                .map(|i| format!("10:{:02} event {i} policy 3.1 evaluated\n", i % 60))
                .collect::<String>(),
        ),
    ] {
        let chunks = chunk_text("probe", &text);
        let biggest = chunks
            .iter()
            .map(|c| c.text.chars().count())
            .max()
            .unwrap_or(0);
        assert!(chunks.len() > 1, "{name}: {} chunk(s)", chunks.len());
        assert!(
            biggest < CHUNK_TARGET * 2,
            "{name}: biggest chunk is {biggest} chars"
        );
        // Every chunk is a contiguous run of whole lines, so a citation is
        // still a slice of the source.
        for c in &chunks {
            assert!(!c.text.is_empty());
        }
    }
}

#[test]
fn one_line_longer_than_the_target_is_left_whole() {
    // There is no boundary inside it. Cutting anyway would put a citation
    // across two passages, which is the one thing `cite` cannot express.
    let text = format!("{}\n", "word ".repeat(CHUNK_TARGET));
    let chunks = chunk_text("probe", &text);
    assert_eq!(chunks.len(), 1, "{} chunk(s)", chunks.len());
}

// ── the convergence fold ────────────────────────────────────
//
// `converge` is the whole instrument of the fast-slot arm, so it is tested
// for the ways it could flatter the arm, not only for the ways it could
// crash. Every one of these is a way a fold could manufacture agreement.

#[test]
fn a_finding_two_readings_agree_on_survives_a_majority_fold() {
    let cands = vec![
        read(
            0,
            "quiet hours begin at 11:00 pm",
            "Quiet hours begin at 11:00 pm.",
        ),
        read(
            1,
            "quiet hours begin at 11:00 pm",
            "Quiet hours start at 11 pm.",
        ),
    ];
    let (groups, kept) = converge(&cands, majority(2));
    assert_eq!(kept, vec![0], "the two readings are one finding");
    assert_eq!(groups, vec![vec![0, 1]], "and the fold says so");
}

#[test]
fn a_finding_only_one_reading_saw_falls_out_at_k_two_and_stands_at_k_one() {
    let cands = vec![
        read(
            0,
            "quiet hours begin at 11:00 pm",
            "Quiet hours begin at 11:00 pm.",
        ),
        read(
            1,
            "quiet hours begin at 11:00 pm",
            "Quiet hours start at 11 pm.",
        ),
        read(
            2,
            "guests may stay two consecutive nights",
            "Guests may stay two nights.",
        ),
    ];
    assert_eq!(
        converge(&cands, 2).1,
        vec![0],
        "a lone sighting is not agreement"
    );
    assert_eq!(
        converge(&cands, 1).1,
        vec![0, 2],
        "at k=1 every sighting stands"
    );
}

#[test]
fn one_reading_saying_it_twice_is_one_vote_not_two() {
    // Two failures at once, both seen on real output. A model that returns
    // the same rule twice in one reply would clear a k=2 bar by itself, and
    // the curve would measure repetition rather than agreement — so the two
    // never share a group. And chunk 19 of the 2026-08-24 baseline states two
    // obligations in ONE sentence ("permitted only if every member approves"
    // and "kept out of the bedroom of any member who objects"), so both cite
    // the same words while being different rules: at k=1 both must stand.
    let cands = vec![
        read(
            0,
            "quiet hours begin at 11:00 pm",
            "Quiet hours begin at 11:00 pm.",
        ),
        read(
            0,
            "quiet hours begin at 11:00 pm",
            "Quiet hours bind guests too.",
        ),
    ];
    assert!(converge(&cands, 2).1.is_empty(), "one reading is one vote");
    assert_eq!(
        converge(&cands, 1).1,
        vec![0, 1],
        "one reading's two findings are two findings, not one repeated"
    );
    assert!(
        converge(&cands, 1).0.is_empty(),
        "and nothing folded, so nothing is reported as folded"
    );
}

#[test]
fn the_survivor_is_the_earliest_reading_not_the_wordiest() {
    // Shopping for the longest wording would inflate an anchor score that is
    // matched by phrase — the fold would be choosing its own grade.
    let cands = vec![
        read(0, "twenty gallons", "Tank capacity is twenty gallons."),
        read(
            1,
            "twenty gallons",
            "The shared tank capacity is twenty gallons, measured at the inlet.",
        ),
    ];
    let (_, kept) = converge(&cands, 2);
    assert_eq!(kept, vec![0]);
    assert_eq!(cands[kept[0]].text, "Tank capacity is twenty gallons.");
}

#[test]
fn a_citation_that_merely_contains_another_is_a_different_finding() {
    // **Measured on the 27B baseline artifact of 2026-08-24, which is why
    // this reads the way it does.** The fold matched on containment first,
    // and on that run it ate two real rules: chunk 12's "quiet hours apply to
    // the backyard" cites the whole resolving paragraph, which CONTAINS the
    // sentence chunk 12's second rule cites, so the two folded into one. A
    // fold that eats findings depresses every k on the curve, and the curve
    // would have been read as the fast slot reading badly.
    let cands = vec![
        read(
            0,
            "following complaints the house resolved that quiet hours begin at 11:00 pm",
            "Quiet hours apply outdoors too.",
        ),
        read(
            1,
            "quiet hours begin at 11:00 pm",
            "Quiet hours begin at 11pm.",
        ),
    ];
    assert!(
        converge(&cands, 2).1.is_empty(),
        "one span sitting inside another is not two readings agreeing"
    );
    assert_eq!(
        converge(&cands, 1).1,
        vec![0, 1],
        "at k=1 both findings stand"
    );
    assert!(
        converge(&cands, 1).0.is_empty(),
        "and neither was folded into the other"
    );
}

#[test]
fn whitespace_and_case_do_not_split_one_finding_in_two() {
    let cands = vec![
        read(
            0,
            "Quiet   hours\nbegin at 11:00 PM",
            "Quiet hours begin at 11pm.",
        ),
        read(
            1,
            "quiet hours begin at 11:00 pm",
            "Quiet hours start at 11pm.",
        ),
    ];
    assert_eq!(converge(&cands, 2).1, vec![0]);
}

#[test]
fn two_kinds_over_one_sentence_are_two_findings() {
    // A rule and an open question about the same words are different acts,
    // and folding them would let a question vote for a rule.
    let mut q = read(1, "the allotment", "Who looks after the allotment?");
    q.kind = Kind::Question;
    let cands = vec![
        read(0, "the allotment", "The allotment is tended weekly."),
        q,
    ];
    assert!(
        converge(&cands, 2).1.is_empty(),
        "different kinds never vote for each other"
    );
}

#[test]
fn the_same_words_in_two_passages_are_two_findings() {
    let mut b = read(
        1,
        "quiet hours begin at 11:00 pm",
        "Quiet hours begin at 11pm.",
    );
    b.chunk = 4;
    let cands = vec![
        read(
            0,
            "quiet hours begin at 11:00 pm",
            "Quiet hours begin at 11pm.",
        ),
        b,
    ];
    assert!(
        converge(&cands, 2).1.is_empty(),
        "agreement is per passage — two passages saying it is not two readings of one"
    );
}

#[test]
fn a_group_of_one_is_not_reported_as_a_fold() {
    let cands = vec![read(0, "twenty gallons", "Tank is twenty gallons.")];
    let (groups, kept) = converge(&cands, 1);
    assert_eq!(kept, vec![0]);
    assert!(groups.is_empty(), "folding nothing is not a fold");
}

#[test]
fn majority_is_more_than_half_the_readings() {
    assert_eq!(majority(1), 1);
    assert_eq!(majority(2), 2);
    assert_eq!(majority(3), 2);
    assert_eq!(majority(4), 3);
    assert_eq!(majority(5), 3);
}

// ── refold: the curve is replay, and it refuses what it cannot fold ──

/// Build a convergence artifact on disk and hand back its directory.
fn convergence_run(dir: &str, samples: usize, cands: Vec<Candidate>) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(dir);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let mut run = in_flight(vec![], cands);
    run.samples = samples;
    run.stopped_after = Some(format!("extract: --samples {samples}"));
    std::fs::write(d.join("run-1.json"), serde_json::to_string(&run).unwrap()).unwrap();
    d
}

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

#[test]
fn refold_rewrites_kept_at_the_k_it_was_asked_for_and_calls_no_model() {
    // No Mock, no endpoint, no config: a refold that needed one could not run
    // on the artifacts of a sweep whose server has since been repointed.
    let d = convergence_run(
        "canon-refold-k",
        2,
        vec![
            read(0, "twenty gallons", "Tank is twenty gallons."),
            read(1, "twenty gallons", "The tank holds twenty gallons."),
            read(0, "seven-day notice", "Notice is seven days."),
        ],
    );
    let out = d.join("k2");
    let code = refold(
        &args(&[
            "--refold",
            d.to_str().unwrap(),
            "--k",
            "2",
            "--out",
            out.to_str().unwrap(),
        ]),
        d.to_str().unwrap(),
    );
    assert_eq!(code, 0);

    let v: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("run-1.json")).unwrap()).unwrap();
    assert_eq!(
        v["kept"].as_array().unwrap().len(),
        1,
        "only the agreed finding survives k=2"
    );
    assert_eq!(
        v["candidates"].as_array().unwrap().len(),
        3,
        "every reading is kept as evidence"
    );
    assert!(
        v["stopped_after"].as_str().unwrap().contains("k=2"),
        "the k travels with the artifact: {}",
        v["stopped_after"]
    );
}

#[test]
fn refold_refuses_a_k_no_fold_could_reach() {
    // The failure it prevents: k=2 over one reading folds to nothing, and a
    // reachability of zero would be published as a finding about k rather
    // than as an impossible question.
    let d = convergence_run(
        "canon-refold-impossible",
        1,
        vec![read(0, "twenty gallons", "Tank is twenty gallons.")],
    );
    let code = refold(
        &args(&["--refold", d.to_str().unwrap(), "--k", "2"]),
        d.to_str().unwrap(),
    );
    assert_eq!(
        code, 2,
        "asking one reading for two votes is refused, not answered"
    );
}

#[test]
fn refold_without_a_k_is_refused_rather_than_defaulted() {
    let d = convergence_run(
        "canon-refold-no-k",
        3,
        vec![read(0, "twenty gallons", "Tank is twenty gallons.")],
    );
    let code = refold(
        &args(&["--refold", d.to_str().unwrap()]),
        d.to_str().unwrap(),
    );
    assert_eq!(code, 2, "a fold threshold is never guessed for you");
}

#[test]
fn a_run_written_before_samples_existed_reads_as_one_reading() {
    // Every artifact in fixtures/ predates this field. If it defaulted to 0,
    // `majority` would ask for 1 vote from 0 readings and the old runs would
    // silently refold to nothing.
    let old = serde_json::json!({
        "schema": "canon-draft-run/v1",
        "at": 1, "endpoint": "http://x/v1", "model": "primary", "profile": "house",
        "sources": [], "chunks": [], "candidates": [], "dropped": [],
        "duplicates": [], "kept": [], "tensions": []
    });
    let run: DraftRun = serde_json::from_value(old).unwrap();
    assert_eq!(run.samples, 1);
    assert_eq!(run.stopped_after, None);
}
