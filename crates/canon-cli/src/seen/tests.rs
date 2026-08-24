// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("canon-seen-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_declined_candidate_is_not_offered_again() {
    // The bug: nothing recorded a REJECT, so re-pointing at the same feed
    // asked again about the rule you had already turned down.
    let d = scratch("reject");
    let mut s = Seen::load(&d);
    assert!(!s.was_rejected("Quiet hours begin at 10pm."));
    s.record("Quiet hours begin at 10pm.", Why::Rejected)
        .unwrap();

    let reloaded = Seen::load(&d);
    assert!(reloaded.was_rejected("Quiet hours begin at 10pm."));
    // And the two kinds do not answer for each other.
    assert!(!reloaded.was_read("Quiet hours begin at 10pm."));
}

#[test]
fn a_passage_already_read_is_not_read_again() {
    let d = scratch("read");
    let mut s = Seen::load(&d);
    s.record("the whole passage", Why::Read).unwrap();
    assert!(Seen::load(&d).was_read("the whole passage"));
    assert!(!Seen::load(&d).was_read("a different passage"));
}

#[test]
fn recording_twice_writes_one_line() {
    let d = scratch("dedup");
    let mut s = Seen::load(&d);
    s.record("x", Why::Read).unwrap();
    s.record("x", Why::Read).unwrap();
    let body = std::fs::read_to_string(d.join(FILE)).unwrap();
    assert_eq!(body.lines().filter(|l| !l.starts_with('#')).count(), 1);
}

#[test]
fn the_file_says_deleting_it_is_safe() {
    // Anything that reads as authoritative and is not gets treated as
    // authoritative. This one has to disclaim itself in the file.
    let d = scratch("header");
    let mut s = Seen::load(&d);
    s.record("x", Why::Read).unwrap();
    let body = std::fs::read_to_string(d.join(FILE)).unwrap();
    assert!(body.contains("not part of the canon"), "{body}");
    assert!(body.contains("deleting this file"), "{body}");
}

#[test]
fn a_preview_reads_the_set_and_never_adds_to_it() {
    // The trap this closes: preview, decide to keep three, run for real, be
    // told there is nothing there because the preview marked it all read.
    let d = scratch("preview");
    let mut real = Seen::load(&d);
    real.record("already declined", Why::Rejected).unwrap();

    let mut pre = Seen::preview(&d);
    assert!(
        pre.was_rejected("already declined"),
        "a preview still reads"
    );
    pre.record("a fresh passage", Why::Read).unwrap();
    assert!(
        !Seen::load(&d).was_read("a fresh passage"),
        "a preview must leave nothing behind"
    );
}

#[test]
fn a_corrupt_line_costs_that_line_and_nothing_else() {
    let d = scratch("corrupt");
    std::fs::write(
        d.join(FILE),
        "# header\nabc123 read\ngarbage-with-no-kind\ndef456 rejected\nzzz nonsense\n",
    )
    .unwrap();
    let s = Seen::load(&d);
    assert_eq!(s.read.len(), 1);
    assert_eq!(s.rejected.len(), 1);
}

#[test]
fn per_machine_state_is_kept_out_of_a_shared_canon() {
    // `acts.jsonl` is meant to be committed and merged — that is the point of
    // a text log, and there is a `merge-driver` verb for it. `seen` beside it
    // is one person's ingest state: committed, it conflicts on every pull and
    // tells the team which candidates somebody personally declined.
    let d = scratch("gitignore");
    let mut s = Seen::load(&d);
    s.record("x", Why::Rejected).unwrap();
    let ignore = std::fs::read_to_string(d.join(".gitignore")).expect("written on first use");
    assert!(ignore.contains("seen"), "{ignore}");
    assert!(ignore.contains("draft-runs/"), "{ignore}");
}

#[test]
fn a_gitignore_the_group_edited_is_left_alone() {
    let d = scratch("gitignore-kept");
    std::fs::write(d.join(".gitignore"), "we meant this\n").unwrap();
    let mut s = Seen::load(&d);
    s.record("x", Why::Read).unwrap();
    assert_eq!(
        std::fs::read_to_string(d.join(".gitignore")).unwrap(),
        "we meant this\n"
    );
}
