// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("canon-sources-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn a_file_that_was_not_read_is_reported_and_counted() {
    // **The bug this module was written for.** A folder of documents plus a
    // Slack export read as "3 source(s)" and never mentioned the fourth file,
    // so two rules that existed only in chat were never seen by anyone — and
    // the person who pointed at their own folder had no way to find out.
    let d = scratch("skips");
    std::fs::write(
        d.join("handbook.md"),
        "# H\n\nThe house is quiet at 11pm.\n",
    )
    .unwrap();
    std::fs::write(d.join("charter.pdf"), "%PDF-1.4 binary").unwrap();
    std::fs::write(d.join("photo.jpg"), "jpeg").unwrap();
    std::fs::write(d.join("rota.xlsx"), "xlsx").unwrap();
    std::fs::write(d.join("Makefile"), "all:\n").unwrap();

    let mut got = Gathered::default();
    gather(&d, &mut got).unwrap();
    assert_eq!(got.sources.len(), 1);
    let note = got.skipped_note().expect("four files went unread");
    assert!(note.contains("4 file(s) were not read"), "{note}");
    for ext in [".pdf", ".jpg", ".xlsx", "with no extension"] {
        assert!(note.contains(ext), "`{ext}` is not named:\n{note}");
    }
    // And it says what it DOES read, or the report is a dead end.
    assert!(note.contains(".md"), "{note}");
}

#[test]
fn everything_read_means_nothing_to_report() {
    let d = scratch("clean");
    std::fs::write(d.join("a.md"), "x").unwrap();
    std::fs::write(d.join("b.txt"), "y").unwrap();
    let mut got = Gathered::default();
    gather(&d, &mut got).unwrap();
    assert_eq!(got.sources.len(), 2);
    assert!(got.skipped_note().is_none(), "a clean run says nothing");
}

#[test]
fn a_named_file_is_read_whatever_it_is_called() {
    // A walk filters because a walk is a guess about intent. Naming the file
    // is not a guess, so `--from notes.org` reads notes.org.
    let d = scratch("named");
    let p = d.join("notes.org");
    std::fs::write(&p, "* quiet after 11").unwrap();
    let mut got = Gathered::default();
    gather(&p, &mut got).unwrap();
    assert_eq!(got.sources.len(), 1);
    assert_eq!(got.sources[0].name, "notes.org");
    assert!(got.skipped.is_empty());
}

#[test]
fn sources_are_named_relative_to_what_was_pointed_at() {
    // Absolute paths are ninety characters of noise in the loop whose whole
    // job is reading quotes.
    let d = scratch("relative");
    std::fs::create_dir_all(d.join("meeting-notes")).unwrap();
    std::fs::write(
        d.join("meeting-notes/2026-01.txt"),
        "rent is due on the 1st",
    )
    .unwrap();
    let mut got = Gathered::default();
    gather(&d, &mut got).unwrap();
    assert_eq!(got.sources[0].name, "meeting-notes/2026-01.txt");
}

#[test]
fn hidden_directories_are_not_read_and_are_not_reported_as_skipped() {
    // `.git` alone would bury a run in objects, and reporting ten thousand
    // skipped files would make the report the thing people learn to ignore.
    let d = scratch("hidden");
    std::fs::create_dir_all(d.join(".git")).unwrap();
    std::fs::write(d.join(".git/config"), "[core]").unwrap();
    std::fs::write(d.join("a.md"), "x").unwrap();
    let mut got = Gathered::default();
    gather(&d, &mut got).unwrap();
    assert_eq!(got.sources.len(), 1);
    assert!(got.skipped.is_empty(), "{:?}", got.skipped);
}

// ── chat ────────────────────────────────────────────────────

const SLACK: &str = r#"[
 {"type":"message","user":"U01","user_profile":{"display_name":"mira"},
  "text":"can we agree the heating goes off at 11","ts":"1772232000.000100"},
 {"type":"message","user":"U02","user_profile":{"display_name":"dana"},
  "text":"fine by me","ts":"1772232060.000100"},
 {"type":"message","subtype":"channel_join","user":"U03",
  "text":"sam has joined the channel","ts":"1772232120.000100"},
 {"type":"message","user":"U03","user_profile":{"display_name":"sam"},
  "text":"recycling goes out sunday night not monday","ts":"1772240000.000100"}
]"#;

#[test]
fn a_slack_export_becomes_text_with_who_said_it() {
    let out = render_chat(SLACK).expect("reads as chat");
    assert!(
        out.contains("> mira: can we agree the heating goes off at 11"),
        "{out}"
    );
    assert!(out.contains("> dana: fine by me"), "{out}");
    // Who said it is load-bearing: a rule somebody proposed and a rule the
    // channel agreed to are different things, and the reader cannot tell
    // them apart without the names.
    assert!(out.contains("> sam: recycling"), "{out}");
    // Joins are not things anybody decided.
    assert!(!out.contains("has joined"), "{out}");
}

#[test]
fn a_gap_in_the_conversation_becomes_a_chunk_boundary() {
    // Chat is not prose. Rendered as one unbroken block, a year of a channel
    // is one chunk and every citation points at all of it. The blank line is
    // what the existing chunker cuts on, so a burst boundary and a chunk
    // boundary are the same thing by construction.
    let out = render_chat(SLACK).unwrap();
    let bursts: Vec<&str> = out.split("\n\n").collect();
    assert_eq!(
        bursts.len(),
        2,
        "two conversations, two hours apart:\n{out}"
    );
    assert!(bursts[0].contains("mira") && bursts[0].contains("dana"));
    assert!(bursts[1].contains("sam"));

    // And every message opens a unit the sentence splitter will cut on, or
    // the whole burst becomes one `[1]` and every citation into it is
    // refused for pointing past the end.
    let spans = crate::locate::sentences(&out);
    assert!(
        spans.len() >= 3,
        "{} span(s) for three messages — the splitter is not seeing them:\n{out}",
        spans.len()
    );
}

#[test]
fn the_other_export_shapes_read_too() {
    // Discord: an object with `messages`, `author.username`, `content`.
    let discord = r#"{"messages":[
      {"author":{"username":"omar"},"content":"no unwrap in library code",
       "timestamp":1772232000}]}"#;
    let out = render_chat(discord).expect("discord");
    assert!(out.contains("> omar: no unwrap in library code"), "{out}");

    // JSONL, one message per line, with a trailing partial line that exports
    // routinely carry.
    let jsonl = "{\"user\":\"tess\",\"text\":\"four spaces never tabs\",\"ts\":1}\n{\"user\":\"pri";
    let out = render_chat(jsonl).expect("jsonl");
    assert!(out.contains("> tess: four spaces never tabs"), "{out}");
}

#[test]
fn json_that_is_not_a_chat_export_is_reported_rather_than_read_as_prose() {
    // The alternative is worse than skipping it: a package-lock read as
    // prose produces candidates cited to dependency names.
    let d = scratch("notchat");
    std::fs::write(d.join("a.md"), "x").unwrap();
    std::fs::write(
        d.join("package-lock.json"),
        r#"{"name":"x","lockfileVersion":3,"packages":{}}"#,
    )
    .unwrap();
    let mut got = Gathered::default();
    gather(&d, &mut got).unwrap();
    assert_eq!(got.sources.len(), 1);
    let note = got.skipped_note().expect("reported");
    assert!(note.contains("not a chat export"), "{note}");
}

#[test]
fn an_empty_chat_file_is_unread_not_a_source_that_said_nothing() {
    assert!(render_chat("[]").is_none());
    assert!(render_chat(r#"{"messages":[]}"#).is_none());
    assert!(render_chat("not json at all").is_none());
}
