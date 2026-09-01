// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon` — hold a body of commitments, record what was decided about them,
//! and check proposals against them.
//!
//! Exit codes are part of the contract, because CI and agents read them:
//!   0 supported / ok · 1 conflicts · 2 unaddressed or usage · 3 cannot judge
//!
//! A canon on the `personal` profile never returns 1. See
//! [`check::exit_code`].

mod check;
mod cmds;
mod config;
mod draft;
mod draw_cmd;
mod explain;
mod govern;
mod lineage;
mod locate;
mod mcp;
mod model;
mod profile;
mod quantify;
mod rebase;
mod replay;
mod resolver;
mod seen;
mod sources;
mod store;
mod subject;
mod tensions;
mod wrap;
#[cfg(test)]
mod testing;

/// What a new person is shown.
///
/// **Six verbs, because the other forty-one are the reason this does not
/// spread.** A tool somebody has to be introduced to twice is one that stops
/// at the person who installed it. Everything past `check` is something a
/// group grows into — most never will — and it is one command away.
const HELP: &str = "\
canon — the rules you already have, and what was decided about them

  init [--profile personal|code|house]   start one in this directory
  draft --from <folder>                  read what you already have
  add \"<text>\"                           write one down by hand
  check \"<proposal>\"                     does it clash with one?
  list                                   what is live now
  why <id>                               what replaced what, when, and why
  log                                    the raw acts, oldest first

Nobody writes one of these a rule at a time. `draft` reads a folder — anything
in it that is text, in whatever format you keep it — or a pipe: `cat whatever |
canon draft --from - --dry-run --json`. `--resume` finishes a review later.

`draft` and `check` need a model; everything else folds a text file.
A canon on the `personal` profile never returns a verdict and never exits 1.

  canon help all    who decides what, how you decide, what has gone stale,
                    forking and merging, drawing lots, the agent surface
";

/// Everything. Reached by `canon help all`, never by accident.
const HELP_ALL: &str = "\
canon — every verb. The short list is `canon --help`.

USAGE
  canon <command> [args]

RECORD                                        (no model needed)
  init [--profile personal|code|house]  start a canon here
  add \"<text>\"                          assert a commitment
  list                                   what is live now
  why <id>                               what this replaced, when, and why
  supersede <id> \"<text>\" -m \"<reason>\"  replace a commitment
  retract <id> -m \"<reason>\"             withdraw one, no replacement
  accept <a> <b> -m \"<reason>\"           carry a contradiction knowingly
  dismiss <a> <b> [-m \"<reason>\"]        not actually a conflict
  undo <act-id> [-m \"<reason>\"]          revert an act; itself revertible
  question \"<text>\"                      record what the canon does not cover
  open                                   the open questions
  log                                    the raw acts
  mcp                                    serve the agent surface on stdio

GOVERN                                        (no model needed)
  who <scope>                            who may decide this, and under what
  grant <actor> <scope> [--horizon <d>]  give someone standing
  withdraw <actor> <scope>               step back from a scope, or stand down
  scope <id> <scope>                     put a commitment in a scope
  policy show | set <rule> [-m \"...\"]    what this canon decides under
  position \"<about>\" --against|--toward  a vote, an objection, a second
  decide \"<about>\" --outcome --authority record what the group decided
  rank <id> <rank>                       a principle, not a convention
  horizon <act-id> <YYYY-MM-DD>          look at this again by then
  overdue                                what has gone past its date
  draw commit <scope> <seats> --after    announce a lot nobody can steer
  draw seal | open <draw-id>             your secret, before and after
  draw show <draw-id>                    the panel, recomputed from the log
  silence \"<subject>\" -m \"<why>\"         unwritten on purpose, not by neglect
  voice [<actor>]                        what someone raised, and what came of it
  leave <scope> [-m \"<question>\"]        step out, and leave the question behind
  replay <dir> [--policy <rule>]         replay a scenario; --policy asks
                                         what another rule would have done

LINEAGE                             (git optional; only rebase needs a model)
  share                                  a pasteable snapshot
  adopt <url>[@gen] | --paste            fork someone else's canon
  diff --upstream [--propose]            how you have diverged from your seed
  upgrade <gen>                          take a newer generation
  rebase --onto <url>@<gen>              carry your law onto a different base
  merge-driver %O %A %B                  git merge driver; run it for setup

ADJUDICATE                                    (needs an endpoint)
  check \"<proposal>\"                     how a proposal stands (personal: stakes)
  tensions                               where your commitments conflict
  draft --from <paths>                   propose commitments from loose notes
        --from -                         read the document from stdin
        --as <name>                      what to call it in the citations
        --from-git --since 1y            commit bodies as the source
        --dry-run [--json]               propose, write nothing
        --resume                         finish a review, no model call
        --include-ignored                read what .gitignore covers
        --max-chunks <n>                 read at most n passages this run
        --samples <n> --dry-run          read each passage n times (measurement)
        --refold <dir> --k <n>           re-fold those readings, no model call
        --replay <run.json>              re-run a recorded run, no model call
        --replay <run.json> --live-from <stage>
                                         replay above <stage>, call for real from it

CONFIGURE
  config show                            what this canon is configured with
  config set endpoint <url>              any OpenAI-compatible server
  config set model <name>                model name to send (default: local)
  config set extract_model <name>        a different slot for extraction only

FLAGS
  --json          machine-readable on stdout; logs go to stderr
  -m <reason>     the rationale recorded on an act
  --allow-remote  permit an endpoint that is not on this machine

ENVIRONMENT
  CANON_ACTOR      who is acting (default: git user.name, prefixed human:)
  CANON_DIR        use this canon instead of searching upward
  CANON_ENDPOINT   override the configured endpoint for one run
  CANON_MODEL      override the configured model for one run

ON THE PERSONAL PROFILE
  This is a structured journal, not a clinician. It does not diagnose and
  it does not advise. `check` reports which of your commitments have a
  stake in something and which way each pulls; it never returns a verdict
  and never exits 1. A contradiction you are carrying on purpose is a
  first-class state here, and nothing is ever force-resolved.
";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        // `canon help all` is the only way to the long form. Somebody who
        // typed `canon` by accident gets six verbs, not forty-seven.
        let all = args.iter().skip(1).any(|a| a == "all" || a == "--all");
        print!("{}", if all { HELP_ALL } else { HELP });
        std::process::exit(if args.is_empty() { 2 } else { 0 });
    }
    let (cmd, rest) = args.split_first().unwrap();
    let code = match cmd.as_str() {
        "init" => cmds::init(rest),
        "add" => cmds::add(rest),
        "list" => cmds::list(rest),
        "why" => cmds::why(rest),
        "supersede" => cmds::supersede(rest),
        "retract" => cmds::retract(rest),
        "accept" => cmds::accept(rest),
        "dismiss" => cmds::dismiss(rest),
        "undo" => cmds::undo(rest),
        "log" => cmds::log(rest),
        "question" => cmds::question(rest),
        "who" => govern::who(rest),
        "grant" => govern::grant(rest),
        "withdraw" => govern::withdraw(rest),
        "scope" => govern::scoped(rest),
        "policy" => govern::policy(rest),
        "position" => govern::position(rest),
        "decide" => govern::decide(rest),
        "rank" => govern::rank(rest),
        "horizon" => govern::horizon(rest),
        "overdue" => govern::overdue(rest),
        "draw" => draw_cmd::run(rest),
        "silence" => govern::silence(rest),
        "voice" => govern::voice(rest),
        "leave" => govern::leave(rest),
        "replay" => replay::run(rest),
        "open" => cmds::open(rest),
        "mcp" => mcp::serve(),
        "share" => lineage::share(rest),
        "adopt" => lineage::adopt(rest),
        "diff" => lineage::diff(rest),
        "upgrade" => lineage::upgrade(rest),
        "rebase" => rebase::run(rest),
        "merge-driver" => lineage::merge_driver(rest),
        "tensions" => tensions::run(rest),
        "config" => cmds::config(rest),
        "draft" => draft::run(rest),
        "check" => check::run(rest),
        "--version" | "-V" => {
            println!("canon {}", env!("CARGO_PKG_VERSION"));
            0
        }
        other => {
            eprintln!("error: unknown command `{other}`\n");
            print!("{HELP}");
            2
        }
    };
    std::process::exit(code);
}

#[cfg(test)]
mod tests {
    use super::{HELP, HELP_ALL};

    /// **The n+1 test, as a test.**
    ///
    /// If the next person cannot be handed this tool in two sentences it does
    /// not spread, and a help screen re-grows one verb at a time with nobody
    /// noticing. This is the thing that notices.
    #[test]
    fn the_short_help_stays_short_and_stays_plain() {
        let lines = HELP.lines().count();
        assert!(
            lines <= 20,
            "the short help is {lines} lines — it is growing back into the long one"
        );
        let verbs = HELP
            .lines()
            .filter(|l| l.starts_with("  ") && !l.starts_with("    "))
            .count();
        assert!(verbs <= 8, "{verbs} verbs in the short help");

        // Words a housemate has never met. Every one of these is real and
        // useful and belongs in `canon help all`, where somebody has asked.
        for jargon in [
            "standing",
            "subsidiarity",
            "sortition",
            "quorum",
            "horizon",
            "authority",
            "annotation",
            "scope",
        ] {
            assert!(
                !HELP.to_lowercase().contains(jargon),
                "the short help says `{jargon}`"
            );
        }
        // And it names the one door to everything else, or the rest is lost.
        assert!(HELP.contains("canon help all"));
    }

    #[test]
    fn every_verb_the_dispatcher_knows_is_documented_somewhere() {
        // The long help is allowed to be long. It is NOT allowed to be
        // incomplete — a verb nobody can find is a verb nobody uses, and the
        // short list only works if the full one is actually full.
        for verb in [
            "init",
            "add",
            "list",
            "why",
            "supersede",
            "retract",
            "accept",
            "dismiss",
            "undo",
            "log",
            "question",
            "open",
            "who",
            "grant",
            "withdraw",
            "scope",
            "policy",
            "position",
            "decide",
            "rank",
            "horizon",
            "overdue",
            "draw",
            "silence",
            "voice",
            "leave",
            "replay",
            "share",
            "adopt",
            "diff",
            "upgrade",
            "rebase",
            "tensions",
            "config",
            "draft",
            "check",
            "mcp",
        ] {
            assert!(
                HELP_ALL.contains(verb),
                "`{verb}` dispatches but is in no help text"
            );
        }
    }
}
