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

// ── what the walk will not do ───────────────────────────────

#[test]
#[cfg(unix)]
fn a_link_out_of_the_folder_is_not_followed() {
    // **The traversal escape SECURITY.md asks to hear about first.** The walk
    // used to decide "is this a directory?" with `is_dir()`, which follows
    // links, so one symlink in a notes folder read whatever it pointed at —
    // and every passage read here is quoted verbatim into a proposal, into
    // `acts.jsonl`, and into somebody's git history. A link to `~/.aws` is
    // not a thing to resolve quietly.
    let d = scratch("escape");
    std::fs::create_dir_all(d.join("pointed")).unwrap();
    std::fs::create_dir_all(d.join("private")).unwrap();
    std::fs::write(d.join("pointed/handbook.md"), "The house is quiet at 11pm.").unwrap();
    std::fs::write(d.join("private/creds.env"), "AWS_SECRET_ACCESS_KEY=hunter2").unwrap();
    std::os::unix::fs::symlink("../private", d.join("pointed/elsewhere")).unwrap();

    let got = read(&d.join("pointed"));
    let names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["handbook.md"], "{names:?}");
    assert!(
        !got.sources.iter().any(|s| s.text.contains("hunter2")),
        "a secret from outside the root reached a source"
    );
    // And it is REPORTED, because a file that was not read is reported.
    let note = got.skipped_note().expect("the link went unread");
    assert!(note.contains("out of the folder"), "{note}");
}

#[test]
#[cfg(unix)]
fn a_link_inside_the_folder_is_still_followed() {
    // The rule is containment, not suspicion of links. Somebody who keeps
    // `current.md -> archive/2026.md` inside their own notes meant that, and
    // refusing every link to fix the escape would break their folder.
    let d = scratch("inside-link");
    std::fs::create_dir_all(d.join("archive")).unwrap();
    std::fs::write(d.join("archive/2026.md"), "Rent is due on the first.").unwrap();
    std::os::unix::fs::symlink("archive/2026.md", d.join("current.md")).unwrap();

    let got = read(&d);
    // One source, not two: the link and its target are the same file, and
    // reading both is two identical chunks and two completions paid for.
    let names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["archive/2026.md"], "{names:?}");
    assert!(got.skipped_note().is_none(), "{:?}", got.skipped);
}

#[test]
#[cfg(unix)]
fn a_link_is_the_only_way_to_some_content_and_still_works() {
    // The containment rule has to leave the link USEFUL, not merely
    // unreported. Here the target sits in a hidden folder the walk passes
    // over, so the link is the only route to it — and it is inside the root,
    // so it is followed.
    let d = scratch("only-link");
    std::fs::create_dir_all(d.join(".archive")).unwrap();
    std::fs::write(d.join(".archive/2026.md"), "Rent is due on the first.").unwrap();
    std::os::unix::fs::symlink(".archive/2026.md", d.join("current.md")).unwrap();

    let got = read(&d);
    let names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["current.md"], "{names:?}");
    assert!(got.sources[0].text.contains("Rent is due"));
}

#[test]
#[cfg(unix)]
fn one_unreadable_folder_does_not_end_the_walk() {
    // It used to return `Err` on the first `read_dir` that failed, so a
    // single `chmod 000` directory anywhere under `~/Documents` failed the
    // whole ingest and read ZERO files. One locked folder is one skip.
    use std::os::unix::fs::PermissionsExt;
    let d = scratch("locked");
    std::fs::create_dir_all(d.join("open")).unwrap();
    std::fs::create_dir_all(d.join("locked")).unwrap();
    std::fs::write(d.join("open/handbook.md"), "We review before merging.").unwrap();
    std::fs::write(d.join("locked/secret.md"), "not readable").unwrap();
    std::fs::set_permissions(d.join("locked"), std::fs::Permissions::from_mode(0o000)).unwrap();
    // Root ignores the mode bits, and a test that silently proves nothing is
    // worse than no test.
    if std::fs::read_dir(d.join("locked")).is_ok() {
        let _ = std::fs::set_permissions(d.join("locked"), std::fs::Permissions::from_mode(0o755));
        return;
    }

    let mut got = Gathered::default();
    let outcome = gather(&d, &mut got, false);
    let _ = std::fs::set_permissions(d.join("locked"), std::fs::Permissions::from_mode(0o755));

    assert!(outcome.is_ok(), "{outcome:?}");
    let names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["open/handbook.md"], "{names:?}");
    let note = got.skipped_note().expect("the locked folder is reported");
    assert!(note.contains("could not be read"), "{note}");
}

#[test]
#[cfg(unix)]
fn a_pipe_in_the_folder_does_not_stop_the_walk_forever() {
    // A fifo reports a length of 0, so it passed every size cap, and then
    // `read` blocked on it forever — no output, no timeout, and Ctrl-C the
    // only way out of `canon draft`.
    //
    // **A regression here HANGS**, which no assertion can catch from the same
    // thread, so the walk runs on its own and the timeout is the assertion.
    let d = scratch("fifo");
    std::fs::write(d.join("handbook.md"), "The house is quiet at 11pm.").unwrap();
    let made = std::process::Command::new("mkfifo")
        .arg(d.join("channel"))
        .status();
    if !made.map(|s| s.success()).unwrap_or(false) {
        return;
    }

    let (tx, rx) = std::sync::mpsc::channel();
    let probe = d.clone();
    std::thread::spawn(move || {
        let mut got = Gathered::default();
        let _ = gather(&probe, &mut got, false);
        let names: Vec<String> = got.sources.iter().map(|s| s.name.clone()).collect();
        let _ = tx.send((names, got.skipped_note()));
    });
    let (names, note) = rx
        .recv_timeout(std::time::Duration::from_secs(20))
        .expect("the walk blocked on a fifo");

    assert_eq!(names, vec!["handbook.md"], "{names:?}");
    let note = note.expect("the pipe is reported");
    assert!(note.contains("not a regular file"), "{note}");
}

#[test]
#[cfg(unix)]
fn a_device_named_directly_is_refused_rather_than_read_forever() {
    // "A file NAMED directly is read whatever it is" could not have meant
    // this: `--from /dev/zero` is not a large read, it is a read that never
    // returns. The one thing a named path has to be is a FILE.
    if !Path::new("/dev/zero").exists() {
        return;
    }
    let mut got = Gathered::default();
    let outcome = gather(Path::new("/dev/zero"), &mut got, false);
    let message = outcome.expect_err("a character device is not a document");
    assert!(message.contains("not a regular file"), "{message}");
    // A refusal with no way forward is a dead end.
    assert!(message.contains("--from -"), "{message}");
}

#[test]
fn a_path_that_is_not_there_says_so() {
    let mut got = Gathered::default();
    let message = gather(Path::new("./no-such-folder-here"), &mut got, false)
        .expect_err("nothing is at that path");
    assert!(message.contains("no such file or folder"), "{message}");
}

// ── one citation, one passage ───────────────────────────────

#[test]
fn two_roots_cannot_produce_one_citation() {
    // A citation is the thing a reader checks a rule against. Names are
    // relative to the root they were found under, so `--from project-a
    // project-b` produced two sources both called `README.md` — and
    // `README.md:3-4` then named a passage in neither.
    let d = scratch("collide");
    std::fs::create_dir_all(d.join("project-a")).unwrap();
    std::fs::create_dir_all(d.join("project-b")).unwrap();
    std::fs::write(
        d.join("project-a/README.md"),
        "We never deploy on a Friday.",
    )
    .unwrap();
    std::fs::write(d.join("project-b/README.md"), "We deploy whenever we like.").unwrap();

    let mut got = Gathered::default();
    gather(&d.join("project-a"), &mut got, false).unwrap();
    gather(&d.join("project-b"), &mut got, false).unwrap();
    got.resolve_names();

    let mut names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["project-a/README.md", "project-b/README.md"]);
}

#[test]
fn a_name_that_does_not_clash_keeps_its_short_form() {
    // Widening every name to be safe would put 90 characters of path in front
    // of every quotation in the review loop, which is the whole job.
    let d = scratch("noclash");
    std::fs::create_dir_all(d.join("sub")).unwrap();
    std::fs::write(d.join("handbook.md"), "The house is quiet at 11pm.").unwrap();
    std::fs::write(d.join("sub/rota.md"), "Sam waters the plants.").unwrap();

    let mut got = read(&d);
    got.resolve_names();
    let names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["handbook.md", "sub/rota.md"], "{names:?}");
}

#[test]
fn the_same_file_reached_twice_is_read_once() {
    // `--from . ./notes` and `--from a.md a.md` both used to read a file
    // twice, which is two chunks of identical text, two completions paid for,
    // and a duplicate rule proposed from each.
    let d = scratch("twice");
    std::fs::create_dir_all(d.join("notes")).unwrap();
    std::fs::write(d.join("notes/rota.md"), "Sam waters the plants on Sundays.").unwrap();

    let mut got = Gathered::default();
    gather(&d, &mut got, false).unwrap();
    gather(&d.join("notes"), &mut got, false).unwrap();
    gather(&d.join("notes/rota.md"), &mut got, false).unwrap();
    assert_eq!(got.sources.len(), 1, "{:?}", got.sources.len());
}

// ── encodings, where the bytes declared one ─────────────────

#[test]
fn a_byte_order_mark_does_not_lead_the_first_citation() {
    // Left in, U+FEFF leads the file, so it leads the first chunk, so it
    // leads the first quotation a person is asked to check a rule against.
    let d = scratch("bom");
    std::fs::write(
        d.join("handbook.md"),
        b"\xef\xbb\xbfThe house is quiet at 11pm.",
    )
    .unwrap();
    let got = read(&d);
    assert_eq!(got.sources.len(), 1, "{:?}", got.skipped);
    assert!(
        got.sources[0].text.starts_with("The house"),
        "{:?}",
        got.sources[0].text
    );
}

#[test]
fn what_windows_calls_unicode_is_read() {
    // UTF-16 with a mark is a file DECLARING its encoding, not a guess about
    // it — and it is what every Windows editor writes when a person picks
    // "Unicode". Reported as "not text", it was the one corpus that could not
    // be onboarded at all.
    let d = scratch("utf16");
    let body = "Rent is due on the first of the month.";
    let mut le: Vec<u8> = vec![0xff, 0xfe];
    let mut be: Vec<u8> = vec![0xfe, 0xff];
    for unit in body.encode_utf16() {
        le.extend_from_slice(&unit.to_le_bytes());
        be.extend_from_slice(&unit.to_be_bytes());
    }
    std::fs::write(d.join("little.txt"), &le).unwrap();
    std::fs::write(d.join("big.txt"), &be).unwrap();

    let got = read(&d);
    assert_eq!(got.sources.len(), 2, "{:?}", got.skipped);
    for source in &got.sources {
        assert_eq!(source.text, body, "{}", source.name);
    }
}

#[test]
fn bytes_with_no_mark_are_reported_as_what_they_look_like() {
    // The line this module will not cross: nothing here GUESSES what
    // undeclared bytes say, because a wrong guess is not a skip anybody sees
    // — it is mojibake that looks like a quotation and goes into the log as
    // one. Reporting a shape is not deciding a meaning.
    let d = scratch("nomark");
    let mut bare: Vec<u8> = Vec::new();
    for unit in "The house is quiet at 11pm.".encode_utf16() {
        bare.extend_from_slice(&unit.to_le_bytes());
    }
    std::fs::write(d.join("notepad.txt"), &bare).unwrap();
    std::fs::write(d.join("photo.jpg"), [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]).unwrap();

    let got = read(&d);
    assert!(got.sources.is_empty(), "nothing declared an encoding");
    let note = got.skipped_note().unwrap();
    assert!(note.contains("no byte-order mark"), "{note}");
    assert!(note.contains("not text"), "{note}");
}

#[test]
fn a_gitignored_folder_takes_its_contents_with_it() {
    // git ignores what is under an ignored directory without asking about
    // each file, and asking only about the file read `target/debug/build.log`
    // out of a `target/` nobody wanted walked.
    let d = scratch("ignored-tree");
    assert!(std::process::Command::new("git")
        .arg("init")
        .current_dir(&d)
        .output()
        .is_ok());
    std::fs::write(d.join(".gitignore"), "target/\n").unwrap();
    std::fs::write(d.join("HANDBOOK.md"), "We review before merging.").unwrap();
    std::fs::create_dir_all(d.join("target/debug/deps")).unwrap();
    std::fs::write(d.join("target/debug/deps/build.log"), "compiling").unwrap();
    std::fs::write(d.join("target/debug/notes.md"), "machine output").unwrap();

    let got = read(&d);
    let names: Vec<&str> = got.sources.iter().map(|s| s.name.as_str()).collect();
    assert_eq!(names, vec!["HANDBOOK.md"], "{names:?}");
    // Counted, not merely absent: "N file(s) were not read" is the promise.
    assert_eq!(
        got.skipped.get("ignored by .gitignore"),
        Some(&2),
        "{:?}",
        got.skipped
    );
}
