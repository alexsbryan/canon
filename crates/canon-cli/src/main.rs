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
mod explain;
mod govern;
mod lineage;
mod locate;
mod mcp;
mod model;
mod profile;
mod quantify;
mod rebase;
mod store;
mod subject;
mod tensions;
#[cfg(test)]
mod testing;

const HELP: &str = "\
canon — a body of commitments, and what was decided about them

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

CONFIGURE
  config show                            what this canon is configured with
  config set endpoint <url>              any OpenAI-compatible server
  config set model <name>                model name to send (default: local)

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
        print!("{HELP}");
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
