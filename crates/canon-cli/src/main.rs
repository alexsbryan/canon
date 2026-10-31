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
// The integration tests' list of jargon, read by the help test too.
#[cfg(test)]
#[path = "../tests/jargon/mod.rs"]
mod jargon;
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
        "areas",
        "AREAS\n\n  An area is a dotted path: `house`, `house.quiet`, `repo.defaults`.\n  Whoever is in charge of `house` is in charge of `house.quiet` and all\n  under it; whoever is in charge of `house.quiet` has only the quiet corner.\n  The whole canon's area is the profile's own: `house`, `repo` or `personal`.\n\n  `canon who` says who is in charge of the canon; `canon give <actor> <area>`\n  puts someone in charge of one; `canon help give` has the flags.\n",
    ),
];

/// Everything. Reached by `canon help all`, never by accident.
const HELP_ALL: &str = "\
canon — every verb. The short list is `canon --help`.

USAGE
  canon <command> [args]

RECORD                                        (no model needed)
  init [--profile personal|code|house]  start a canon here, with you in charge of it
       [--plain]                         without the v4 knobs (changes to the rulings
                                         that wait, personal areas), for older readers
       [--open]                          the old shared start: any write takes
                                         until someone is in charge
  identify [<key.pub> [<backup.pub>…]]   from here on, only your SSH key writes as you;
                                         again, who you are here and what does not count
  add \"<text>\" [--area a]               propose a rule; it is one once its area passes it
  review [--area a] [--json]             what is waiting on you, an area at a time; offers
                                         you charge of an area nobody is in charge of yet.
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
  amend <id> \"<text>\" -m \"<reason>\"      change a rule's words, and say why
  repeal <id> -m \"<reason>\"              take a rule away, no replacement; an id from the
                                         canon you copied names a rule you got from it.
                                         Not in charge? It goes to whoever is, as a proposal
  accept <a> <b> -m \"<reason>\"           keep two rules that conflict, on purpose
  dismiss <a> <b> [-m \"<reason>\"]        not a real conflict
  undo <act-id> [-m \"<reason>\"]          take back an act — yours, or one in your area
  question \"<text>\"                      record what the canon does not cover
  open                                   the open questions
  log                                    the raw acts
  mcp                                    serve the agent surface on stdio

GOVERN                                        (no model needed)
  who [<area>]                           who's in charge here, and how rules pass;
                                         no area: who's in charge of the whole canon
  give <actor> <area> [--until <d>]      put someone in charge of an area, until <d>
  withdraw <actor> <area>                step back from an area, or take someone off it
           [--ask \"<question>\"]         yours: leave the question, without your name
  area <id> <area>                       put a rule in an area
  rulings show | set <rule> [-m \"...\"]   how actions are judged here
  ratification show | set <rule>         how rules pass here, area by area: `standing`
                                         (whoever is in charge says yes), `joint:a,b`,
                                         `threshold:n/m` (n yes pass it, m no stop it),
                                         `majority`, `unanimous`, `share:60%` or
                                         `share:2/3` (that share of those in charge),
                                         `consent:Nd` (passes unless someone objects
                                         within N days), `twice:turnover:<rule>` or
                                         `twice:Nd:<rule>` (must pass twice)
       [--governs all|rules]             whether a change to the rulings waits too
       [--personal on|off]               canon-wide: a person's own area is theirs
  allot <area> --named a,b,c             what this commons has to share
  allocation set rotation --area s       how it goes round: --step, --per, --order
                                         or --from-draw <id> for a lot nobody steers
  pool <area> [--at <date>]              whose turn it is. `who` says who decides,
                                         `pool` who gets what
  decide \"<about>\" allow|deny -m \"<why>\" record what the group decided
         --outcome <o> --authority <a>   the long form, as `check --json` says it
         [--citing <id>,…]               the rules it turned on; a ladder counts by them
  rank <id> principle                    constitutional: harder to change than a rule
                                         ranked `convention`
  revisit <act-id> <YYYY-MM-DD>          set a review date: look at this again by then
  overdue                                what is past its review date
  draw commit <area> <seats> --after     announce a lot nobody can steer
  draw seal | open <draw-id>             your secret, before and after
  draw show <draw-id>                    the panel, recomputed from the log
  silence \"<subject>\" -m \"<why>\"         left unwritten on purpose, not by neglect
  witness [--base <ref>] [--gate]        the ledger checked against git: append-only,
                                         not backdated. Opt in; see `guard`
  guard show | git [--apply]             what defends this ledger from outside the
                                         fold, and what each guard cannot see
  voice [<actor>]                        what someone raised, and what came of it
  second <question-id>                   add your weight to a question, without your name
  replay [<dir>] [--rulings <rule>]      re-decide this canon\'s own record;
                                         --rulings asks what another rule
                                         would have done to it
         [--scenario <f>]                ask your own questions instead
         [--write-scenario <f>]          write out the derived ones to edit
         [--expect <f>]                  check the answers against this file

COPYING                     (git optional; only adopt --rebase needs a model)
  share                                  a pasteable snapshot
  adopt <url>[@gen] | --paste            copy another group's rules
        [--except <id>,… -m \"<why>\"]       repeal these as they arrive
        <url>@<gen> --rebase             move a canon copied from one group's rules
                                         onto another's, keeping its record
              [--carry <id> -m \"<why>\"]     carry a conflict anyway, as your ruling
              [--dry-run]                print the reading, write nothing
  diff --upstream [--propose]            how you differ from the rules you copied
  upgrade <gen> | --paste                take their updates
  merge-driver %O %A %B                  git merge driver; run it for setup

RULING                                        (needs an endpoint)
  check \"<proposal>\"                     how a proposal stands (personal: stakes)
  conflicts                              rules of yours that conflict
  draft --from <paths>                   propose rules from loose notes
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
  it does not advise. `check` reports which things you hold yourself to have a
  stake in something and which way each pulls; it never returns a verdict
  and never exits 1. A contradiction you are carrying on purpose is a
  first-class state here, and nothing is ever force-resolved.
";

fn main() {
    use std::io::IsTerminal;
    // An old spelling is made new before anything reads it, and said once on
    // stderr to a person at a terminal; a script parsing stdout reads on.
    let typed: Vec<String> = std::env::args().skip(1).collect();
    let (args, notes) = cli::resolve(&typed, std::io::stderr().is_terminal());
    notes.iter().for_each(|n| eprintln!("{n}"));
    if args.is_empty() || args[0] == "-h" || args[0] == "--help" || args[0] == "help" {
        // `canon help <verb>` is the same answer as `canon <verb> --help`.
        if let Some(verb) = args.get(1).and_then(|v| cli::help_for(v, HELP_ALL)) {
            print!("{verb}");
            std::process::exit(0);
        }
        // `scopes` is what the topic was called before it was `areas`.
        let topic = args.get(1).map(|t| if t == "scopes" { "areas" } else { t });
        if let Some((_, text)) = topic.and_then(|t| TOPICS.iter().find(|(n, _)| *n == t)) {
            print!("{text}");
            std::process::exit(0);
        }
        // Any word canon uses, from its glossary: `canon help in charge`.
        let said = cli::define(&args[1..].join(" "));
        if !said.is_empty() {
            println!("{}", said.join("\n"));
            std::process::exit(0);
        }
        // A topic nobody has used to print the short help, so a reader asking
        // about profiles learned nothing and was not told so.
        if let Some(t) = topic.filter(|t| *t != "all" && *t != "--all" && !t.starts_with('-')) {
            eprintln!(
                "no such topic or word `{t}` — canon help all lists the verbs, and `canon help profile | areas` the rest."
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
    let code = match cli::verb(cmd) {
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
    use crate::jargon;

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

        // **Past the short help, plain is the bar too.** A student reads the
        // long help and each verb's own as closely as the first screen, so the
        // words the record keeps for itself stay out of all three.
        let mut said = jargon::found(HELP);
        said.extend(jargon::found(HELP_ALL));
        for (verb, ..) in crate::cli::VERBS {
            let help = crate::cli::help_for(verb, HELP_ALL).unwrap();
            said.extend(
                jargon::found(&help)
                    .into_iter()
                    .map(|w| format!("{verb}: {w}")),
            );
        }
        said.dedup();
        assert!(
            said.is_empty(),
            "{} jargon:\n{}",
            said.len(),
            said.join("\n")
        );
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
