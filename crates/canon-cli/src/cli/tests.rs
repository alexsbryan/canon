// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn args(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| s.to_string()).collect()
}

fn root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn the_four_ways_the_parser_used_to_fail_open() {
    // Every one of these was accepted, and every one of them wrote an act.
    let unknown = check("add", &args(&["a rule", "--nonsense-flag"])).unwrap_err();
    assert!(unknown.contains("--nonsense-flag"), "{unknown}");

    // The one that mattered most: `--dry-run` means WRITE NOTHING, and a
    // typo used to make `has("--dry-run")` false and amend the canon.
    let typo = check("draft", &args(&["--from", "notes", "--dry-runn"])).unwrap_err();
    assert!(typo.contains("--dry-run"), "{typo}");

    // A valued flag with nothing after it dropped the value in silence.
    let bare = check("add", &args(&["a rule", "--scope"])).unwrap_err();
    assert!(bare.contains("--scope"), "{bare}");

    // A real flag, on a verb that does not take it.
    let elsewhere = check("add", &args(&["a rule", "--max-chunks", "5"])).unwrap_err();
    assert!(elsewhere.contains("--max-chunks"), "{elsewhere}");
}

#[test]
fn the_equals_form_reaches_the_handler() {
    // `--scope=house` parsed as nothing at all, so `canon add` wrote the
    // commitment and silently dropped the scope — a rule recorded at the
    // wrong level, which in a governance tool is the whole ballgame.
    let checked = check("add", &args(&["a rule", "--scope=house"])).unwrap();
    assert_eq!(checked, args(&["a rule", "--scope", "house"]));
    // And a positional that merely contains `=` is left alone.
    let kept = check("config", &args(&["set", "endpoint", "http://x/v1"])).unwrap();
    assert_eq!(kept, args(&["set", "endpoint", "http://x/v1"]));
}

#[test]
fn from_still_takes_every_path_and_a_bare_dash() {
    // A shell expands `--from ~/notes/**/*.md` into many arguments, and
    // `--from -` means stdin. Declaring `--from` greedy is what keeps both —
    // but `allow_hyphen_values`, which looks like the way to accept `-`, made
    // `--dry-runn` a fourth path instead of an error.
    let many = check(
        "draft",
        &args(&["--from", "a.md", "b.md", "c.md", "--dry-run"]),
    )
    .unwrap();
    assert_eq!(many, args(&["--from", "a.md", "b.md", "c.md", "--dry-run"]));
    let stdin = check("draft", &args(&["--from", "-", "--as", "#house"])).unwrap();
    assert_eq!(stdin, args(&["--from", "-", "--as", "#house"]));
}

#[test]
fn a_verb_this_table_does_not_know_passes_through() {
    // `--version` and an outright typo are `main`'s to answer, and a second
    // opinion here would just be a worse error message.
    assert_eq!(check("--version", &[]).unwrap(), Vec::<String>::new());
    let through = check("not-a-verb", &args(&["--whatever"])).unwrap();
    assert_eq!(through, args(&["--whatever"]));
}

#[test]
fn every_flag_the_code_reads_is_declared() {
    // **The table is derived from the code, and this is what keeps it that
    // way.** A flag added to a handler and forgotten here does not fail
    // quietly — it becomes `unexpected argument` for the person who types the
    // documented command. So the suite fails first.
    let declared: std::collections::BTreeSet<&str> = VERBS
        .iter()
        .flat_map(|(_, valued, switches)| valued.iter().chain(switches.iter()).copied())
        .collect();

    let src = root().join("crates/canon-cli/src");
    let mut read: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).unwrap().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") {
                continue;
            }
            // Its own tests name flags that are deliberately wrong.
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name == "tests.rs" || name == "cli.rs" {
                continue;
            }
            let body = std::fs::read_to_string(&path).unwrap();
            for (call, rest) in body
                .match_indices("flag(args, \"")
                .chain(body.match_indices("has(args, \""))
            {
                let after = &body[call + rest.len()..];
                if let Some(end) = after.find('"') {
                    read.insert(after[..end].to_string(), name.clone());
                }
            }
        }
    }
    assert!(!read.is_empty(), "the scan found nothing to check");
    let missing: Vec<String> = read
        .iter()
        .filter(|(f, _)| !declared.contains(f.as_str()))
        .map(|(f, where_at)| format!("{f} (read in {where_at})"))
        .collect();
    assert!(
        missing.is_empty(),
        "flags the code reads but no verb declares: {missing:?}"
    );
}

#[test]
fn every_command_line_in_the_documentation_still_parses() {
    // **The docs are the contract.** A table derived from the code can still
    // put a flag on the wrong verb, and the place that mistake shows up is
    // somebody following GETTING_STARTED and being told their flag does not
    // exist. Every invocation the project publishes is run through the
    // parser here.
    let mut checked = 0usize;
    let mut broken: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(root()).unwrap().flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "md") {
            continue;
        }
        let body = std::fs::read_to_string(&path).unwrap();
        for line in body.lines() {
            let Some(rest) = published(line) else {
                continue;
            };
            let Some(tokens) = tokenize(rest) else {
                continue;
            };
            let Some((verb, rest)) = tokens.split_first() else {
                continue;
            };
            if !VERBS.iter().any(|(v, _, _)| v == verb) {
                continue;
            }
            checked += 1;
            if let Err(e) = check(verb, rest) {
                broken.push(format!(
                    "{}: `canon {}` -> {}",
                    path.file_name().unwrap().to_string_lossy(),
                    tokens.join(" "),
                    e.lines().next().unwrap_or("")
                ));
            }
        }
    }
    assert!(
        checked > 50,
        "only {checked} invocations found — scan broke"
    );
    assert!(
        broken.is_empty(),
        "{} broken:\n{}",
        broken.len(),
        broken.join("\n")
    );
}

/// The part of a documentation line that is a `canon` invocation.
fn published(line: &str) -> Option<&str> {
    let at = line.find("canon ")?;
    // Only where `canon` opens the command: mid-sentence prose mentioning a
    // flag is not an invocation, and neither is `svrn code canon`.
    let before = line[..at].trim_end();
    let opens = before.is_empty()
        || before.ends_with('$')
        || before.ends_with('|')
        || before.ends_with('`')
        || before.ends_with('(');
    opens.then(|| &line[at + "canon ".len()..])
}

/// Split a published command line, or `None` if it is shell rather than a
/// plain invocation.
fn tokenize(rest: &str) -> Option<Vec<String>> {
    if rest.contains("$(") || rest.contains("&&") || rest.contains("...") {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut quote: Option<char> = None;
    for c in rest.chars() {
        match (quote, c) {
            (Some(q), _) if c == q => quote = None,
            (Some(_), _) => cur.push(c),
            (None, '\'' | '"') => quote = Some(c),
            // A backtick closes an inline code span, so it ends the command
            // wherever it falls — including welded to the last flag, which is
            // how `\u{60}canon diff --upstream\u{60} shows` read as a flag named
            // "--upstream\u{60}".
            (None, '`') => break,
            // A trailing `# comment`, a pipe, or a redirect ends it too.
            (None, '#' | '|' | '>') if cur.is_empty() => break,
            (None, c) if c.is_whitespace() => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            (None, _) => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    (quote.is_none()).then_some(out)
}

#[test]
fn a_refused_argument_speaks_the_way_the_rest_of_the_tool_does() {
    // Everything else here names what is wrong and then the way out, in that
    // order and in one line. clap's own rendering opens with `unexpected
    // argument found` and closes with a usage block, which reads as a second
    // tool talking over the first.
    let typo = check("draft", &args(&["--from", "ok", "--dry-runn"])).unwrap_err();
    assert_eq!(
        typo,
        "`canon draft` has no `--dry-runn` — did you mean `--dry-run`?"
    );

    // No near miss to offer, so the table answers instead: a refusal with no
    // way out of it is a dead end.
    // Far enough from `--scope` that strsim offers nothing — pick a nearer
    // word and clap suggests, which is the branch above.
    let listed = check("add", &args(&["a rule", "--nonsense-flag"])).unwrap_err();
    assert_eq!(
        listed,
        "`canon add` has no `--nonsense-flag` — it takes `--scope`"
    );

    // Past a handful, the list stops being a hint and becomes a wall.
    let many = check("draft", &args(&["--nonsense-flag"])).unwrap_err();
    assert!(
        many.ends_with("`canon help all` lists what it takes"),
        "{many}"
    );

    let none = check("rank", &args(&["--nonsense-flag"])).unwrap_err();
    assert!(none.ends_with("it takes no flags"), "{none}");
}

#[test]
fn a_flag_left_empty_says_what_it_was_waiting_for() {
    for (verb, given, expect) in [
        (
            "draft",
            vec!["--from"],
            "`--from` needs at least one path — a file, a folder, or `-` for stdin",
        ),
        (
            "add",
            vec!["a rule", "--scope"],
            "`--scope` needs a scope after it",
        ),
        (
            "grant",
            vec!["ana", "--horizon"],
            "`--horizon` needs a date after it — YYYY-MM-DD",
        ),
        (
            "approve",
            vec!["abc", "-m"],
            "`-m` needs a reason after it, in quotes",
        ),
        (
            "draft",
            vec!["--from", "ok", "--max-chunks"],
            "`--max-chunks` needs a number after it",
        ),
    ] {
        assert_eq!(check(verb, &args(&given)).unwrap_err(), expect);
    }
}

#[test]
fn a_refusal_nobody_anticipated_keeps_claps_words() {
    // The fallback matters more than the phrasing: an unanticipated refusal
    // that still says what happened beats a tidy one that does not. What it
    // must not do is arrive with clap's `error: ` prefix, because `fail` is
    // about to add one.
    let e = clap::Command::new("draft")
        .no_binary_name(true)
        .arg(
            clap::Arg::new("n")
                .long("n")
                .value_parser(clap::value_parser!(u8)),
        )
        .try_get_matches_from(["--n", "999"])
        .unwrap_err();
    let said = passed_through(&e);
    assert!(!said.starts_with("error: "), "{said}");
    assert!(!said.is_empty());
}
