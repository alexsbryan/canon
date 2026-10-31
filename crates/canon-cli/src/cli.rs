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

/// A verb's handler: its arguments, checked, to an exit code.
pub(crate) type Run = fn(&[String]) -> i32;

/// A verb, the flags that take a value, the flags that do not, and what runs
/// it.
type Flags = &'static [&'static str];
pub(crate) type Verb = (&'static str, Flags, Flags, Run);

/// Every verb. The one list: `main` dispatches from it, [`check`] parses by
/// it, and the help test reads it, so a verb cannot be in one and missing
/// from another.
#[rustfmt::skip]
pub(crate) const VERBS: &[Verb] = &[
    ("init", &["--profile"], &["--plain", "--open"], crate::cmds::init),
    ("add", &["--area", "-m", "--cite"], &[], crate::cmds::add),
    ("review", &["--area"], &["--json"], crate::review::run),
    ("ui", &[], &[], crate::ui::run),
    ("approve", &["--about", "--citing", "-m"], &[], crate::cmds::approve),
    ("object", &["--about", "--citing", "-m"], &[], crate::cmds::object),
    ("list", &["--out"], &["--check", "--json", "--markdown"], crate::cmds::list),
    ("why", &[], &["--json"], crate::cmds::why),
    ("amend", &["--area", "-m"], &[], crate::cmds::supersede),
    ("repeal", &["-m"], &[], crate::cmds::retract),
    ("accept", &["--revisit", "-m"], &[], crate::cmds::accept),
    ("dismiss", &["-m"], &[], crate::cmds::dismiss),
    ("undo", &["-m"], &[], crate::cmds::undo),
    ("log", &[], &["--json"], crate::cmds::log),
    ("question", &["--from-proposal"], &[], crate::cmds::question),
    ("who", &[], &["--json"], crate::govern::who),
    ("give", &["--horizon", "-m"], &[], crate::govern::grant),
    ("withdraw", &["--ask", "-m"], &[], crate::govern::withdraw),
    ("area", &[], &[], crate::govern::scoped),
    ("rulings", &["--entrench", "--graduated", "--objections", "--of", "--area", "-m"], &["--by-about", "--cautious", "--json", "--no-repair"], crate::govern::policy),
    ("ratification", &["--governs", "--personal", "--area", "-m"], &["--json"], crate::govern::ratification),
    ("allot", &["--count", "--named", "--unit", "-m"], &[], crate::govern::allot),
    ("allocation", &["--from-draw", "--order", "--per", "--area", "--step", "-m"], &[], crate::govern::allocation),
    ("pool", &["--at"], &["--json"], crate::govern::pool),
    ("position", &["--citing", "-m"], &["--against", "--toward"], crate::govern::position),
    ("decide", &["--authority", "--citing", "--outcome", "-m"], &[], crate::govern::decide),
    ("rank", &[], &[], crate::govern::rank),
    ("revisit", &["-m"], &[], crate::govern::horizon),
    ("overdue", &[], &["--json"], crate::govern::overdue),
    ("draw", &["--after", "--secret", "-m"], &["--json"], crate::draw_cmd::run),
    ("silence", &["-m"], &[], crate::govern::silence),
    ("voice", &[], &[], crate::govern::voice),
    ("second", &[], &[], crate::govern::second),
    ("replay", &["--entrench", "--expect", "--graduated", "--objections", "--of", "--out", "--policy", "--profile", "--ratification", "--scenario", "--write-scenario"], &["--brief", "--by-about", "--cautious", "--json", "--no-repair"], crate::replay::run),
    ("open", &[], &["--json"], crate::cmds::open),
    ("witness", &["--base", "--slack"], &["--gate", "--json"], crate::guard::witness),
    ("guard", &["--check", "--slack"], &["--apply"], crate::guard::guard),
    ("identify", &[], &[], crate::identify::run),
    ("mcp", &[], &[], |_| crate::mcp::serve()),
    ("share", &[], &[], crate::lineage::share),
    ("adopt", &["--carry", "--except", "-m"], &["--allow-remote", "--dry-run", "--paste", "--rebase"], crate::lineage::adopt),
    ("diff", &[], &["--json", "--propose", "--upstream"], crate::lineage::diff),
    ("upgrade", &[], &["--paste"], crate::lineage::upgrade),
    ("merge-driver", &[], &[], crate::lineage::merge_driver),
    ("conflicts", &[], &["--allow-remote", "--json"], crate::tensions::run),
    ("config", &[], &[], crate::cmds::config),
    ("draft", &["--as", "--continue", "--from", "--max-chunks", "--replay", "--since"], &["--accept-all", "--allow-remote", "--dry-run", "--from-git", "--include-ignored", "--json", "--resume", "--yes"], crate::draft::run),
    ("check", &["--about", "--amends", "--area"], &["--allow-remote", "--irreversible", "--json"], crate::check::run),
];

/// Old spellings, `(old, new)`: a verb renamed into plain words, and a flag.
/// Neither is in help or in `contract/cli.txt`, which list what is
/// documented; both still work, because scripts and fingers already type
/// them. [`resolve`] is the one place they are read.
#[rustfmt::skip]
pub(crate) const VERB_ALIASES: &[(&str, &str)] = &[
    ("supersede", "amend"), ("retract", "repeal"), ("grant", "give"), ("policy", "rulings"),
    ("horizon", "revisit"), ("tensions", "conflicts"), ("scope", "area"),
];
pub(crate) const FLAG_ALIASES: &[(&str, &str)] = &[("--scope", "--area")];

/// The command line with every old spelling in it made new, and one note per
/// spelling for stderr. Stdout is the new verb's, byte for byte, so a script
/// that parses it does not break.
///
/// **The note is for a person typing, so only `to_a_person` gets it.** `main`
/// passes whether stderr is a terminal; an agent, a hook or the Slack bot
/// reads a pipe, and a line it did not ask for is noise it may log or relay.
/// The flag is the seam the tests drive, since they cannot be a terminal.
///
/// A flag is renamed only on a verb that takes the new one, so `--scope` on a
/// verb that never took it is refused under the name that was typed.
pub fn resolve(args: &[String], to_a_person: bool) -> (Vec<String>, Vec<String>) {
    let (mut out, mut notes) = (args.to_vec(), Vec::new());
    for i in 0..out.len() {
        let (name, value) = out[i].split_once('=').unwrap_or((&out[i], ""));
        let flags = verb(&out[0]).map_or(&[][..], |v| v.1);
        let renamed = |a: &&(&str, &str)| a.0 == name && (i == 0 || flags.contains(&a.1));
        let table = if i == 0 { VERB_ALIASES } else { FLAG_ALIASES };
        if let Some((old, new)) = table.iter().find(renamed) {
            let c = if i == 0 { "canon " } else { "" };
            if to_a_person {
                notes.push(format!("`{c}{old}` is now `{c}{new}`; both still work"));
            }
            out[i] = [new, value].join(if value.is_empty() { "" } else { "=" });
        }
    }
    (out, notes)
}

/// The table's entry for a verb, under its documented name.
pub(crate) fn verb(name: &str) -> Option<&'static Verb> {
    VERBS.iter().find(|v| v.0 == name)
}

/// What each word canon uses means, one line each. Content rather than
/// program, so it is a file a reader can open, and `canon help <word>` and
/// each verb's help read it.
const GLOSSARY: &str = include_str!("../glossary.txt");

/// Every line of the glossary that defines `term`, or none. `consent:14d`
/// is found as `consent` and `areas` as `area`; a word with two senses, like
/// `area`, has two lines.
pub fn define(term: &str) -> Vec<&'static str> {
    let term = term.to_lowercase();
    let head = term.split(':').next().unwrap_or_default();
    for t in [&term[..], head, term.strip_suffix('s').unwrap_or_default()] {
        let entry =
            |l: &&str| !l.starts_with('#') && l.split_once(": ").is_some_and(|(k, _)| k == t);
        let found: Vec<&str> = GLOSSARY.lines().filter(entry).collect();
        if !found.is_empty() {
            return found;
        }
    }
    Vec::new()
}

/// Did somebody ask a verb what it does? `main` answers before dispatch, so
/// the handler never sees the flag.
pub fn wants_help(rest: &[String]) -> bool {
    rest.iter().any(|a| a == "--help" || a == "-h")
}

/// What one verb does and takes: its lines from `canon help all`, the flags
/// this table declares for it, and for `init`, what it writes.
///
/// **Every verb used to refuse `--help`** with "has no `--help` — it takes
/// `--profile`", which names a flag and not what the verb does, so finding
/// out what `init` puts in a repository meant initialising a scratch
/// directory. The text is cut from the long help rather than written beside
/// it, so the two cannot disagree. `None` for a verb this table does not
/// know.
pub fn help_for(verb: &str, help_all: &str) -> Option<String> {
    let (name, valued, switches, _) = self::verb(verb)?;
    // A verb's entry is a line at two spaces that opens with its name, and
    // every deeper-indented line under it.
    let mut out = String::new();
    let mut inside = false;
    for line in help_all.lines() {
        let indent = line.len() - line.trim_start().len();
        if indent == 2 {
            let head = line.trim_start();
            inside = head == *name || head.starts_with(&format!("{name} "));
        } else if indent < 2 {
            inside = false;
        }
        if inside {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push('\n');
    let mut flags: Vec<String> = valued
        .iter()
        .map(|f| format!("{f} <{}>", value_name(f)))
        .chain(switches.iter().map(|f| f.to_string()))
        .collect();
    flags.sort_unstable();
    if flags.is_empty() {
        out.push_str("  takes no flags\n");
    } else {
        out.push_str(&format!("  flags: {}\n", flags.join("  ")));
    }
    for (_, said) in define(name).iter().filter_map(|l| l.split_once(": ")) {
        out.push_str(&format!("  in a word: {said}\n"));
    }
    if *name == "init" {
        out.push_str(
            "  writes .canon/acts.jsonl, the canon itself, one act per line, starting\n  \
             with the one that turns the v4 knobs on (--plain starts it empty);\n  \
             .canon/profile, which says how `check` answers; and .canon/.gitignore,\n  \
             which keeps this machine's model config, draft runs and copies of the\n  \
             canons you copied rules from out of git.\n  \
             Commit the rest.\n",
        );
    }
    out.push_str("\n  `canon help all` lists every verb.\n");
    Some(out)
}

/// Check `rest` against what `verb` declares, and hand back the arguments
/// with any `--flag=value` split into two, which is the form the handlers
/// read.
///
/// A verb this table does not know — `--version`, or a typo — passes through
/// untouched, because `main` already has an answer for those. So does a
/// request for help, which `main` answers with [`help_for`] instead of
/// dispatching.
pub fn check(verb: &str, rest: &[String]) -> Result<Vec<String>, String> {
    let Some((name, valued, switches, _)) = self::verb(verb) else {
        return Ok(rest.to_vec());
    };
    if wants_help(rest) {
        return Ok(rest.to_vec());
    }
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
            "area" => "an area after it, like `house.kitchen`",
            "id" => "an id after it, as the pasted block prints it",
            _ => "a value after it",
        },
    }
}

/// `--area=house` as `--area house`.
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
        "--continue" | "--expect" | "--from" | "--out" | "--replay" => "path",
        "-m" => "why",
        "--ask" => "question",
        "--area" | "--of" => "area",
        "--except" => "id",
        "--after" | "--horizon" | "--revisit" | "--at" | "--since" => "date",
        "--count" | "--max-chunks" | "--step" => "n",
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
