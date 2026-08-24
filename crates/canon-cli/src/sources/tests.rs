// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("canon-sources-{name}"));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn read(root: &Path) -> Gathered {
    let mut got = Gathered::default();
    gather(root, &mut got, false).unwrap();
    got
}

#[test]
fn whatever_it_is_called_it_gets_read() {
    // **The objection this module was rewritten for.** The reader used to
    // hold a list of extensions WE chose — `.md`, `.txt`, `.json` — so it
    // worked on the corpora we happened to test against and silently ignored
    // everyone else's. A canon lives in whatever its group already writes in.
    let d = scratch("anything");
    std::fs::write(d.join("charter.org"), "* quiet after 11").unwrap();
    std::fs::write(d.join("bylaws.rst"), "Rent is due on the 1st.").unwrap();
    std::fs::write(d.join("NOTES"), "we never merge on a Friday").unwrap();
    std::fs::write(d.join("thread.eml"), "Subject: rota\n\nSam waters.").unwrap();
    std::fs::write(d.join("standup.log"), "10:02 no deploys after 4pm").unwrap();

    let got = read(&d);
    assert_eq!(got.sources.len(), 5, "{:?}", got.skipped);
    assert!(got.skipped_note().is_none(), "{:?}", got.skipped);
}

#[test]
fn a_file_that_was_not_read_is_reported_and_counted() {
    // A folder of documents plus a Slack export read as "3 source(s)" and
    // never mentioned the fourth file, so two rules that existed only in
    // chat were never seen by anyone — and the person who pointed at their
    // own folder had no way to find out.
    let d = scratch("skips");
    std::fs::write(d.join("handbook.md"), "The house is quiet at 11pm.\n").unwrap();
    // Real bytes, not the word "binary": readability is now a property of
    // the bytes, so a test that writes text and calls it a jpeg proves
    // nothing.
    std::fs::write(d.join("photo.jpg"), [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]).unwrap();
    std::fs::write(d.join("scan.pdf"), [0x25, 0x50, 0x44, 0x46, 0xc0, 0x80]).unwrap();
    std::fs::write(d.join("empty.md"), "   \n\n").unwrap();

    let got = read(&d);
    assert_eq!(got.sources.len(), 1, "{:?}", got.skipped);
    let note = got.skipped_note().expect("three files went unread");
    assert!(note.contains("3 file(s) were not read"), "{note}");
    assert!(note.contains("not text"), "{note}");
    assert!(note.contains("empty"), "{note}");
    // A report with no way out of it is a dead end.
    assert!(note.contains("Naming a file directly"), "{note}");
}

#[test]
fn what_the_project_calls_generated_is_skipped_and_can_be_read_anyway() {
    // Without this, pointing at any checked-out repo reads its `target/` or
    // `node_modules/` — thousands of machine-written files, one completion
    // each. The authority is the person's own .gitignore rather than a list
    // of build directories we guessed at, which would be the same whitelist
    // wearing a different hat.
    let d = scratch("ignored");
    assert!(std::process::Command::new("git")
        .arg("init")
        .current_dir(&d)
        .output()
        .is_ok());
    std::fs::write(d.join(".gitignore"), "target/\n*.tmp\n").unwrap();
    std::fs::write(d.join("HANDBOOK.md"), "We review before merging.").unwrap();
    std::fs::write(d.join("scratch.tmp"), "throwaway").unwrap();
    std::fs::create_dir_all(d.join("target/debug")).unwrap();
    std::fs::write(d.join("target/debug/build.log"), "compiling").unwrap();

    let got = read(&d);
    assert_eq!(got.sources.len(), 1, "{:?}", got.skipped);
    assert_eq!(got.sources[0].name, "HANDBOOK.md");
    let note = got.skipped_note().expect("two generated files");
    assert!(note.contains("ignored by .gitignore"), "{note}");

    // And the person can overrule it.
    let mut all = Gathered::default();
    gather(&d, &mut all, true).unwrap();
    assert_eq!(all.sources.len(), 3, "{:?}", all.skipped);
}

#[test]
fn a_folder_that_is_not_a_repo_withholds_nothing() {
    // No git, or a folder outside a repo, means nothing is DECLARED
    // generated. That is silence, not a failure — and certainly not a reason
    // to skip everything.
    let d = scratch("norepo");
    std::fs::write(d.join("a.md"), "x").unwrap();
    std::fs::write(d.join("b.rst"), "y").unwrap();
    let got = read(&d);
    assert_eq!(got.sources.len(), 2, "{:?}", got.skipped);
}

#[test]
fn something_too_big_to_be_writing_is_reported_not_read() {
    let d = scratch("huge");
    std::fs::write(d.join("small.md"), "a rule").unwrap();
    std::fs::write(d.join("server.log"), "x".repeat(MAX_BYTES as usize + 1)).unwrap();
    let got = read(&d);
    assert_eq!(got.sources.len(), 1);
    let note = got.skipped_note().unwrap();
    assert!(note.contains("larger than 2 MiB"), "{note}");
}

#[test]
fn a_named_file_is_read_whatever_it_is() {
    // A walk filters because a walk is a guess about intent. Naming the file
    // is not a guess — no size cap, no structure test, no ignore rule.
    let d = scratch("named");
    let p = d.join("decisions.json");
    std::fs::write(&p, r#"{"we":"decided","this":"in json"}"#).unwrap();
    let mut got = Gathered::default();
    gather(&p, &mut got, false).unwrap();
    assert_eq!(got.sources.len(), 1);
    assert_eq!(got.sources[0].name, "decisions.json");
    assert!(got.skipped.is_empty());
}

#[test]
fn a_named_file_that_is_not_text_says_so() {
    let d = scratch("namedbin");
    let p = d.join("photo.jpg");
    std::fs::write(&p, [0xff, 0xd8, 0xff]).unwrap();
    let mut got = Gathered::default();
    let e = gather(&p, &mut got, false).unwrap_err();
    assert!(e.contains("not text"), "{e}");
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
    let got = read(&d);
    assert_eq!(got.sources[0].name, "meeting-notes/2026-01.txt");
}

#[test]
fn hidden_directories_are_not_read_and_are_not_reported_as_skipped() {
    // `.git` alone would bury a run in objects, `.canon` is the tool's own
    // state, and reporting ten thousand skipped files would make the report
    // the thing people learn to ignore.
    let d = scratch("hidden");
    std::fs::create_dir_all(d.join(".git")).unwrap();
    std::fs::write(d.join(".git/config"), "[core]").unwrap();
    std::fs::write(d.join("a.md"), "x").unwrap();
    let got = read(&d);
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
fn chat_is_recognised_by_its_content_not_its_name() {
    // The extension gate is gone, so an export saved as `#general.txt` — or
    // with no extension at all — still reads as a conversation.
    let d = scratch("chatext");
    std::fs::write(d.join("general.txt"), SLACK).unwrap();
    let got = read(&d);
    assert_eq!(got.sources.len(), 1);
    assert!(
        got.sources[0].text.contains("> mira:"),
        "{}",
        got.sources[0].text
    );
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
    let spans = crate::locate::units(&out).0;
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
fn structured_data_is_not_writing() {
    // Reading a lockfile as prose produces commitments cited to dependency
    // names. This is a test of the CONTENT, so it also catches the ones not
    // called `.json` — and lets a `.json` full of minutes through, because
    // that one parses as a conversation.
    let d = scratch("notchat");
    std::fs::write(d.join("a.md"), "We review before merging.").unwrap();
    std::fs::write(
        d.join("package-lock.json"),
        r#"{"name":"x","lockfileVersion":3,"packages":{}}"#,
    )
    .unwrap();
    std::fs::write(d.join("deps.lock"), r#"[{"name":"serde","version":"1.0"}]"#).unwrap();
    let got = read(&d);
    assert_eq!(got.sources.len(), 1, "{:?}", got.skipped);
    let note = got.skipped_note().expect("reported");
    assert!(note.contains("structured data"), "{note}");
    assert_eq!(
        *got.skipped
            .get("structured data, not a chat export")
            .unwrap(),
        2
    );
}

#[test]
fn markdown_that_opens_with_a_bracket_is_still_markdown() {
    // A badge line is not JSON, and the sniff must fall through to prose
    // rather than refusing the file.
    let d = scratch("badge");
    std::fs::write(
        d.join("README.md"),
        "[![build](x.svg)](y)\n\nWe review before merging.",
    )
    .unwrap();
    let got = read(&d);
    assert_eq!(got.sources.len(), 1, "{:?}", got.skipped);
    assert!(got.sources[0].text.contains("review before merging"));
}

#[test]
fn an_empty_chat_file_is_unread_not_a_source_that_said_nothing() {
    assert!(render_chat("[]").is_none());
    assert!(render_chat(r#"{"messages":[]}"#).is_none());
    assert!(render_chat("not json at all").is_none());
}
