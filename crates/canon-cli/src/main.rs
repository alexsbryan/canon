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
mod cli;
mod cmds;
mod config;
mod draft;
mod draw_cmd;
mod explain;
mod govern;
mod guard;
mod identify;
mod lineage;
mod locate;
mod mcp;
mod model;
mod profile;
mod quantify;
mod rebase;
mod render;
mod replay;
mod resolver;
mod review;
mod seen;
mod similar;
mod sources;
mod store;
mod subject;
mod tensions;
mod term;
#[cfg(test)]
mod testing;
mod ui;
mod wrap;

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
  add \"<text>\" [-m <why>] [--cite <where>] write one down, with why it is one and where it came from
  check \"<proposal>\"                     does it clash with one? A model does the
                                         reading: canon config set endpoint <url>
  list                                   what is live now
  why <id>                               what replaced what, when, and why; a silence too
  log                                    the raw acts, oldest first

Nobody writes one of these a rule at a time. `draft` reads a folder — any text
in it — or a pipe: `cat whatever | canon draft --from - --dry-run --json`.

`draft`/`check` need a model (`canon config set endpoint <url>`); the rest folds a text file.

  canon help all    who decides what, how you decide, what has gone stale,
                    forking and merging, drawing lots, the agent surface
";

/// The two things a reader asks before their first write: which profile
/// they founded, and what a scope is. `canon help profile` used to print
/// the short help; a topic nobody has says so now.
const TOPICS: &[(&str, &str)] = &[
    (
        "profile",
        "THE THREE PROFILES\n\n  personal  a person's own journal: what has a stake, and which\n            way it pulls; never a verdict, never exit 1.\n  code      a codebase's principles: verdict-shaped, because CI reads\n            exit codes.\n  house     a household's rules: which act a proposal needs.\n\n  `canon init` defaults to personal; `canon init --profile <p>` picks one.\n  The profile changes the words and the verdicts, not the record.\n",
    ),
    (
        "scopes",
        "SCOPES\n\n  A scope is a dotted path: `house`, `house.quiet`, `repo.defaults`.\n  A grant over `house` covers `house.quiet` and everything under it; a\n  grant over `house.quiet` covers only the quiet corner. The whole canon's\n  scope is the profile's own: `house`, `repo` or `personal`.\n\n  `canon who` names the scopes that hold the canon; `canon grant <actor>\n  <scope>` gives standing over one; `canon help grant` has the flags.\n",
    ),
];

/// Everything. Reached by `canon help all`, never by accident.
const HELP_ALL: &str = "\
canon — every verb. The short list is `canon --help`.

USAGE
  canon <command> [args]

RECORD                                        (no model needed)
  init [--profile personal|code|house]  start a canon here: its founder holds it
       [--plain]                         without the v4 knobs (policy changes that
                                         wait, personal scopes), for older readers
       [--open]                          the old shared start: any write takes
                                         until someone holds it
  identify [<key.pub> [<backup.pub>…]]   from here on, only your SSH key writes as you;
                                         again, who you are here and what does not count
  add \"<text>\" [--scope s]              propose a commitment; a rule when its scope ratifies it
  review [--scope s] [--json]            what is waiting on you, a scope at a time; offers
                                         you standing over a scope nobody holds yet.
                                         A person's verb: it needs a terminal and a
                                         human: actor; --json only reads
  ui                                     canon as a page on this machine: decide, propose,
                                         change or take away a rule, leave something
                                         unwritten, identify yourself
  approve <id> [-m \"<why>\"]             approve a proposal, by name
          --about \"<subject>\"           or support an action [--citing <id>]
  object <id> -m \"<why>\"                object to one; the reason is required
         --about \"<subject>\"            or object to an action; `check` counts it
                                         (what `position` did)
  list                                   what is live now, and what is proposed
       --markdown [--out <f> [--check]]  the same, as a document for readers without
                                         canon; --check fails when <f> is stale
  why <id>                               what this replaced, when, and why; a silence or question shows its act
  supersede <id> \"<text>\" -m \"<reason>\"  replace a commitment
  retract <id> -m \"<reason>\"             withdraw one, no replacement; an upstream id
                                         names what was inherited from it. Without
                                         standing, a proposal for a holder to approve
  accept <a> <b> -m \"<reason>\"           carry a contradiction knowingly
  dismiss <a> <b> [-m \"<reason>\"]        not actually a conflict
  undo <act-id> [-m \"<reason>\"]          revert an act — yours, or one you hold
  question \"<text>\"                      record what the canon does not cover
  open                                   the open questions
  log                                    the raw acts
  mcp                                    serve the agent surface on stdio

GOVERN                                        (no model needed)
  who [<scope>]                          who may decide this, and under what;
                                         no scope: who holds the canon as a whole
  grant <actor> <scope> [--horizon <d>]  give someone standing
  withdraw <actor> <scope>               step back from a scope, or stand down
           [--ask \"<question>\"]         yours: leave the question, without your name
  scope <id> <scope>                     put a commitment in a scope
  policy show | set <rule> [-m \"...\"]    what this canon decides under
  ratification show | set <rule>         how a proposal becomes a rule, per scope:
                                         standing, joint:a,b, threshold:n/m (n approve,
                                         m objecting stop it), majority, unanimous,
                                         share:60% or share:2/3 (that share of who holds
                                         it), consent:Nd, twice:turnover:<rule>
                                         or twice:Nd:<rule>
       [--governs all|commitments]       whether a change of policy waits too
       [--personal on|off]               canon-wide: a person's own scope is theirs
  allot <scope> --named a,b,c            what this commons has to share
  allocation set rotation --scope s      how it goes round: --step, --per, --order
                                         or --from-draw <id> for a lot nobody steers
  pool <scope> [--at <date>]             whose turn it is. `who` is authority,
                                         `pool` is appropriation
  decide \"<about>\" --outcome --authority record what the group decided
         [--citing <id>,…]               the rules it turned on; a ladder counts by them
  rank <id> <rank>                       a principle, not a convention
  horizon <act-id> <YYYY-MM-DD>          look at this again by then
  overdue                                what has gone past its date
  draw commit <scope> <seats> --after    announce a lot nobody can steer
  draw seal | open <draw-id>             your secret, before and after
  draw show <draw-id>                    the panel, recomputed from the log
  silence \"<subject>\" -m \"<why>\"         unwritten on purpose, not by neglect
  witness [--base <ref>] [--gate]        the ledger checked against git: append-only,
                                         not backdated. Opt in; see `guard`
  guard show | git [--apply]             what defends this ledger from outside the
                                         fold, and what each guard cannot see
  voice [<actor>]                        what someone raised, and what came of it
  second <question-id>                   add your weight to a question, without your name
  replay [<dir>] [--policy <rule>]       re-decide this canon\'s own record;
                                         --policy asks what another rule
                                         would have done to it
         [--scenario <f>]                ask your own questions instead
         [--write-scenario <f>]          write out the derived ones to edit
         [--expect <f>]                  check the answers against this file

LINEAGE                     (git optional; only adopt --rebase needs a model)
  share                                  a pasteable snapshot
  adopt <url>[@gen] | --paste            fork someone else's canon
        [--except <id>,… -m \"<why>\"]       retract these as they arrive
        <url>@<gen> --rebase             move a canon that descends from one
                                         lineage onto another, keeping its record
              [--carry <id> -m \"<why>\"]     carry a conflict anyway, as your ruling
              [--dry-run]                print the reading, write nothing
  diff --upstream [--propose]            how you have diverged from your seed
  upgrade <gen> | --paste                take a newer generation
  merge-driver %O %A %B                  git merge driver; run it for setup

ADJUDICATE                                    (needs an endpoint)
  check \"<proposal>\"                     how a proposal stands (personal: stakes)
  tensions                               where your commitments conflict
  draft --from <paths>                   propose commitments from loose notes
        --from -                         read the document from stdin
        --as <name>                      what to call it in the citations
        --from-git --since 1y            commit bodies as the source
        --dry-run [--json]               propose, write nothing
        --resume [run]                   finish a review, no model call — every run with
                                         candidates left, oldest first, or the one named
        --include-ignored                read what .gitignore covers
        --max-chunks <n>                 read at most n passages this run
        --yes                            do not ask before a large run
        --continue <run.json>            finish a run that failed after extraction, from
                                         its recorded candidates — no passage read again
        --replay <run.json>              re-run a recorded run, no model call

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
  CANON_ACTOR      who is acting (default: git user.name, prefixed human:);
                   a name that has bound a key also needs that key to sign
  CANON_SIGNING_KEY  the key you sign with, when it is not in ~/.ssh
                   or your ssh-agent
  CANON_DIR        use this canon instead of searching upward
  CANON_ENDPOINT   override the configured endpoint for one run
  CANON_MODEL      override the configured model for one run
  CANON_API_KEY    sent as a bearer token, for an endpoint that needs one

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
        // `canon help <verb>` is the same answer as `canon <verb> --help`.
        if let Some(verb) = args.get(1).and_then(|v| cli::help_for(v, HELP_ALL)) {
            print!("{verb}");
            std::process::exit(0);
        }
        let topic = args.get(1).map(String::as_str);
        if let Some((_, text)) = topic.and_then(|t| TOPICS.iter().find(|(n, _)| *n == t)) {
            print!("{text}");
            std::process::exit(0);
        }
        // A topic nobody has used to print the short help, so a reader asking
        // about profiles learned nothing and was not told so.
        if let Some(t) = topic.filter(|t| *t != "all" && *t != "--all" && !t.starts_with('-')) {
            eprintln!(
                "no such topic `{t}` — canon help all lists the verbs, and `canon help profile | scopes` the rest."
            );
            std::process::exit(1);
        }
        // `canon help all` is the only way to the long form. Somebody who
        // typed `canon` by accident gets seven verbs, not forty-nine.
        let all = args.iter().skip(1).any(|a| a == "all" || a == "--all");
        print!("{}", if all { HELP_ALL } else { HELP });
        std::process::exit(if args.is_empty() { 2 } else { 0 });
    }
    let (cmd, rest) = args.split_first().unwrap();
    // Before the parser, which would refuse it, and before dispatch, which
    // would act on the rest of the line.
    if cli::wants_help(rest) {
        if let Some(verb) = cli::help_for(cmd, HELP_ALL) {
            print!("{verb}");
            std::process::exit(0);
        }
    }
    // **Before dispatch, because after it the act is written.** An argument
    // nobody recognised used to be passed over in silence — including
    // `--dry-runn`, which made the flag meaning "write nothing" fail open.
    let rest = &match cli::check(cmd, rest) {
        Ok(checked) => checked,
        // Through `fail`, so a refused argument reads like every other thing
        // this tool declines to do, and exits the same way.
        Err(report) => std::process::exit(cmds::fail(report)),
    }[..];
    let code = match cli::VERBS.iter().find(|(v, ..)| v == cmd) {
        Some((.., run)) => run(rest),
        None if cmd == "--version" || cmd == "-V" => {
            println!("canon {}", env!("CARGO_PKG_VERSION"));
            0
        }
        None => {
            eprintln!("error: unknown command `{cmd}`\n");
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
        for (verb, ..) in crate::cli::VERBS {
            assert!(
                HELP_ALL.contains(verb),
                "`{verb}` dispatches but is in no help text"
            );
        }
    }
}
