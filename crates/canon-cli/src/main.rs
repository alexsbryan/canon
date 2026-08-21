// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon` — hold a body of commitments, record what was decided about them,
//! and check proposals against them.
//!
//! Exit codes are part of the contract, because CI and agents read them:
//!   0 supported / ok · 1 conflicts · 2 unaddressed or usage · 3 cannot judge

mod cmds;
mod store;

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
  log                                    the raw acts
  share                                  a pasteable snapshot

ADJUDICATE                                    (needs an endpoint)
  check \"<proposal>\"                     supported / conflicts / unaddressed
  tensions                               where your commitments conflict
  draft --from <paths>                   propose commitments from loose notes

FLAGS
  --json        machine-readable on stdout; logs go to stderr
  -m <reason>   the rationale recorded on an act

ENVIRONMENT
  CANON_ACTOR   who is acting (default: git user.name, prefixed human:)
  CANON_DIR     use this canon instead of searching upward
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
        "share" => cmds::share(rest),
        "check" | "tensions" | "draft" => cmds::needs_model(cmd),
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
