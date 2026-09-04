// SPDX-License-Identifier: AGPL-3.0-or-later
//! What each verb accepts, declared in one place and enforced before dispatch.
//!
//! **The parser used to fail open in every direction**, and every one of these
//! wrote to somebody's canon:
//!
//! - `canon add "..." --nonsense` — accepted, flag ignored.
//! - `canon add "..." --scope=house` — accepted, and the **scope silently
//!   dropped**, so a commitment was recorded at the wrong level. The `=` form
//!   was never parsed at all; only the spaced form was.
//! - `canon add "..." --scope` with no value — accepted, scope dropped.
//! - `canon draft --from notes --dry-runn` — the typo made `has("--dry-run")`
//!   false, so the flag that means WRITE NOTHING failed open and the run
//!   would have amended the canon.
//!
//! All four are the same bug: an argument nobody recognised was passed over
//! in silence, which is the defaulted absence §18.3 forbids, sitting in front
//! of every verb that writes an act.
//!
//! **This validates; it does not replace the handlers.** Each verb still
//! reads `&[String]` through `cmds::flag` and `cmds::has`, because those
//! carry canon's own argument grammar — `--from` taking every path until the
//! next flag, positionals that mean different things per verb. What clap adds
//! is the thing that grammar never had: a declared surface to check against,
//! and `strsim` to say "did you mean `--dry-run`?" instead of nothing at all.
//!
//! The table is derived from the code rather than written beside it, and
//! [`tests::every_flag_the_code_reads_is_declared`] is what keeps it that way
//! — a flag added to a handler and forgotten here fails the suite rather than
//! becoming an error for the person who types it.

use clap::{Arg, ArgAction, Command};

/// Verb, the flags that take a value, and the flags that do not.
#[rustfmt::skip]
const VERBS: &[(&str, &[&str], &[&str])] = &[
    ("init", &["--profile"], &[]),
    ("add", &["--scope"], &[]),
    ("approve", &["-m"], &[]),
    ("object", &["-m"], &[]),
    ("list", &[], &["--json"]),
    ("why", &[], &["--json"]),
    ("supersede", &["--scope", "-m"], &[]),
    ("retract", &["-m"], &[]),
    ("accept", &["--revisit", "-m"], &[]),
    ("dismiss", &["-m"], &[]),
    ("undo", &["-m"], &[]),
    ("log", &[], &["--json"]),
    ("question", &["--from-proposal"], &[]),
    ("who", &[], &["--json"]),
    ("grant", &["--horizon", "-m"], &[]),
    ("withdraw", &["-m"], &[]),
    ("scope", &[], &[]),
    ("policy", &["--entrench", "--graduated", "--objections", "--of", "--scope", "-m"], &["--cautious", "--json"]),
    ("ratification", &["--scope", "-m"], &["--json"]),
    ("allot", &["--count", "--named", "--unit", "-m"], &[]),
    ("allocation", &["--from-draw", "--order", "--per", "--scope", "--step", "-m"], &[]),
    ("pool", &["--at"], &["--json"]),
    ("position", &["--citing", "-m"], &["--against", "--toward"]),
    ("decide", &["--authority", "--outcome", "-m"], &[]),
    ("rank", &[], &[]),
    ("horizon", &["-m"], &[]),
    ("overdue", &[], &["--json"]),
    ("draw", &["--after", "--secret", "-m"], &["--json"]),
    ("silence", &["-m"], &[]),
    ("voice", &[], &[]),
    ("leave", &["--why", "-m"], &[]),
    ("replay", &["--entrench", "--graduated", "--objections", "--of", "--out", "--policy", "--profile", "--scenario", "--write-scenario"], &["--brief", "--cautious", "--json"]),
    ("open", &[], &["--json"]),
    ("mcp", &[], &[]),
    ("share", &[], &[]),
    ("adopt", &[], &["--paste"]),
    ("diff", &[], &["--json", "--propose", "--upstream"]),
    ("upgrade", &[], &[]),
    ("rebase", &["--onto"], &["--allow-remote", "--json"]),
    ("merge-driver", &[], &[]),
    ("tensions", &[], &["--allow-remote", "--json"]),
    ("config", &[], &[]),
    ("draft", &["--as", "--from", "--k", "--live-from", "--max-chunks", "--out", "--refold", "--replay", "--samples", "--since"], &["--accept-all", "--allow-remote", "--dry-run", "--from-git", "--include-ignored", "--json", "--resume", "--yes"]),
    ("check", &["--about", "--amends", "--scope"], &["--allow-remote", "--irreversible", "--json"]),
];

/// Check `rest` against what `verb` declares, and hand back the arguments
/// with any `--flag=value` split into two, which is the form the handlers
/// read.
///
/// A verb this table does not know — `--version`, or a typo — passes through
/// untouched, because `main` already has an answer for those.
pub fn check(verb: &str, rest: &[String]) -> Result<Vec<String>, String> {
    let Some((name, valued, switches)) = VERBS.iter().find(|(v, _, _)| *v == verb) else {
        return Ok(rest.to_vec());
    };
    let args = split_equals(rest, valued);
    match command(name, valued, switches).try_get_matches_from(&args) {
        Ok(_) => Ok(args),
        Err(e) => Err(refusal(name, valued, switches, &e)),
    }
}

/// clap's finding, said the way canon says things.
///
/// **A parser has done half its job when it refuses; the message is the other
/// half.** Everything else in this tool names the thing that is wrong and then
/// the way out of it, in that order and in one line — `` `spring` is not a
/// date — YYYY-MM-DD ``, `` `ab` matches 3 acts — use more characters ``.
/// clap's own rendering opens with `unexpected argument found` and closes with
/// a usage block, which is a second tool talking over the first.
///
/// Anything not recognised here keeps clap's wording rather than being
/// flattened into something vaguer: an unanticipated refusal that still says
/// what happened beats a tidy one that does not.
fn refusal(verb: &str, valued: &[&str], switches: &[&str], e: &clap::Error) -> String {
    use clap::error::{ContextKind, ErrorKind};
    let Some(arg) = e.get(ContextKind::InvalidArg).map(|a| a.to_string()) else {
        return passed_through(e);
    };
    match e.kind() {
        ErrorKind::UnknownArgument => Some(match e.get(ContextKind::SuggestedArg) {
            Some(near) => format!("`canon {verb}` has no `{arg}` — did you mean `{near}`?"),
            None => format!(
                "`canon {verb}` has no `{arg}` — {}",
                takes(valued, switches)
            ),
        }),
        // clap names the argument as `--scope <scope>`; the flag is the half
        // of that the person typed.
        ErrorKind::InvalidValue => {
            let flag = arg.split_whitespace().next().unwrap_or(&arg);
            Some(format!("`{flag}` needs {}", wants(flag)))
        }
        _ => None,
    }
    .unwrap_or_else(|| passed_through(e))
}

/// clap's own words, minus the prefix `cmds::fail` is about to add.
fn passed_through(e: &clap::Error) -> String {
    let said = e.to_string();
    said.strip_prefix("error: ")
        .unwrap_or(&said)
        .trim()
        .to_string()
}

/// What this verb does accept — because a refusal with no way out of it is a
/// dead end, and the table already knows the answer.
fn takes(valued: &[&str], switches: &[&str]) -> String {
    let mut all: Vec<&str> = valued.iter().chain(switches).copied().collect();
    all.sort_unstable();
    match all.len() {
        0 => "it takes no flags".to_string(),
        // Past a handful the list stops being a hint and starts being a wall.
        7.. => "`canon help all` lists what it takes".to_string(),
        _ => format!(
            "it takes {}",
            all.iter()
                .map(|f| format!("`{f}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

/// What a flag was waiting for.
fn wants(flag: &str) -> &'static str {
    match flag {
        // The one flag whose answer is genuinely three different things.
        "--from" => "at least one path — a file, a folder, or `-` for stdin",
        _ => match value_name(flag) {
            "date" => "a date after it — YYYY-MM-DD",
            "n" => "a number after it",
            "why" => "a reason after it, in quotes",
            "path" => "a path after it",
            "scope" => "a scope after it",
            _ => "a value after it",
        },
    }
}

/// `--scope=house` as `--scope house`.
///
/// Done here rather than left to clap because the HANDLERS are the ones that
/// have to read it, and `cmds::flag` looks at the next argument. clap accepts
/// either form, so validating the split version is the same check.
fn split_equals(rest: &[String], valued: &[&str]) -> Vec<String> {
    let mut out = Vec::with_capacity(rest.len());
    for a in rest {
        match a.split_once('=') {
            Some((name, value)) if valued.contains(&name) => {
                out.push(name.to_string());
                out.push(value.to_string());
            }
            _ => out.push(a.clone()),
        }
    }
    out
}

fn command(verb: &'static str, valued: &[&'static str], switches: &[&'static str]) -> Command {
    let mut c = Command::new(verb)
        .no_binary_name(true)
        // canon's own help is a written document, and `main` serves it before
        // anything reaches here. clap must not grow a second one.
        .disable_help_flag(true)
        .disable_version_flag(true)
        .arg(Arg::new("positional").num_args(0..));
    for f in valued {
        let a = declare(f).action(ArgAction::Append);
        // `--from` takes every path until the next flag, because a shell
        // expands `--from ~/notes/**/*.md` into many arguments.
        //
        // **Not `allow_hyphen_values`**, which reads as the way to let
        // `--from -` mean stdin and is in fact the way to lose every flag
        // after it: it made `--from ok --dry-runn` swallow the typo as a
        // fourth path, which is the exact failure this module exists to
        // stop. A bare `-` is already a value to clap; a `--flag` is not.
        c = c.arg(if *f == "--from" {
            a.num_args(1..)
        } else {
            a.num_args(1)
        });
    }
    for f in switches {
        c = c.arg(declare(f).action(ArgAction::SetTrue));
    }
    c
}

/// What a flag's value is called when clap has to name it.
///
/// Without this the id is the name, so a missing value read `a value is
/// required for '--from <--from>'`. The default covers the flags whose value
/// has no better word than "value".
fn value_name(flag: &str) -> &'static str {
    match flag {
        "--from" | "--out" | "--onto" | "--refold" | "--replay" => "path",
        "-m" | "--why" => "why",
        "--scope" | "--of" => "scope",
        "--after" | "--horizon" | "--revisit" | "--at" | "--since" => "date",
        "--count" | "--k" | "--samples" | "--max-chunks" | "--step" => "n",
        _ => "value",
    }
}

fn declare(flag: &'static str) -> Arg {
    let a = Arg::new(flag).value_name(value_name(flag));
    match flag.strip_prefix("--") {
        Some(long) => a.long(long),
        // `-m` is the only short flag, and it carries the reason for an act.
        None => match flag.strip_prefix('-').and_then(|s| s.chars().next()) {
            Some(short) => a.short(short),
            None => a,
        },
    }
}

#[cfg(test)]
mod tests;
