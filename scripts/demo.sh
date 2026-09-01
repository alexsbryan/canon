#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# The run of show.
#
#   ./scripts/demo.sh              # step through: [enter] between acts, [enter] at each beat
#   ./scripts/demo.sh --auto       # straight through, no clears (what you read back)
#   ./scripts/demo.sh --offline    # no endpoint: skips the live half of act 2 and acts 3-4
#
# THE SHAPE. Before, then after. Act 1 is the document as the house has it.
# Acts 2-4 use a model to turn it into rules, answer a question over them,
# and find where they contradict each other. Then the cable comes out, and
# everything after runs with no model and no network — starting with the
# house doing something about its own contradiction. The reveal lands only
# because the room has just watched the model half work.
#
# NOTHING IS STAGED, AND EVERYTHING SAYS WHAT IT IS. Act 2 reads the first
# passage LIVE before it replays the tape of the whole document, so the room
# watches a tape get cut before it hears the word "recording". Every fixture
# is a ledger written in canon's own verbs. Every tape is a run file `draft`
# wrote about itself. Where a recording is replayed, the script says so and
# the presenter says so out loud.
#
# A REPLAY WRITES NOTHING. `draft --replay` is a measurement (draft.rs:1482),
# so after act 2 the six rules a presenter kept are materialised from
# `fixtures/maple-house/accepted`: each one copied from the tape's own
# candidate set with the citation the model cut. `canon list` shows them on
# stage. Acts 3-5 answer over exactly what the room watched get proposed.
set -uo pipefail
cd "$(dirname "$0")/.."
CANON=${CANON:-./target/debug/canon}
[ -x "$CANON" ] || { echo "build first: cargo build" >&2; exit 1; }

DEMO=${DEMO_DIR:-/tmp/canon-demo}
TAPE=${TAPE:-fixtures/maple-house/runs/demo-tape/run.json}
SEED=${SEED:-fixtures/maple-house/accepted}
DOC=fixtures/maple-house/maple-house.md
FOUNDING=fixtures/founding/runs/pod-27b/sweep/run-1788143950.json
AUTO=0; OFFLINE=0
for a in "$@"; do
  [ "$a" = "--auto" ] && AUTO=1
  [ "$a" = "--offline" ] && OFFLINE=1
done

# ── the stage ───────────────────────────────────────────────
# Stepped mode clears at every act so each one starts at the top of a blank
# projector. --auto never clears. Commands echo as `canon …`, never as the
# path to a debug build, never with an env prefix.
# Wrap canon's own long lines to the projector, not to a default.
[ -t 1 ] && export COLUMNS=${COLUMNS:-$(tput cols 2>/dev/null || echo 100)}
ACT=0
clear_() { [ $AUTO -eq 1 ] || printf '\033[2J\033[H'; }
beat() { ACT=$((ACT + 1)); clear_; printf '\n\033[2mact %s\033[0m\n\033[1m%s\033[0m\n\n' "$ACT" "$1"; }
sub()  { clear_; printf '\n\033[2mact %s, continued\033[0m\n\033[1m%s\033[0m\n\n' "$ACT" "$1"; }
turn() { clear_; printf '\n\n\033[1m%s\033[0m\n\n' "$1"; }
say()  { printf '   %s\n' "$1"; }
run()  {
  local shown="" a
  for a in "$@"; do
    [ "$a" = "$CANON" ] && a=canon
    case "$a" in *" "*) a="\"$a\"";; esac
    shown="$shown${shown:+ }$a"
  done
  printf '\n\033[2m$\033[0m \033[1m%s\033[0m\n\n' "$shown"; "$@"; echo
}
# [enter] between acts; a quiet `…` for a beat WITHIN an act, so the next line
# lands on top of what the room is already looking at.
wait_() { [ $AUTO -eq 1 ] || { printf '\n\033[2m   [enter]\033[0m'; read -r _; }; }
# Either way a blank line is left behind, so beats read as beats.
hold()  { [ $AUTO -eq 1 ] || { printf '\033[2m   …\033[0m'; read -r _; printf '\033[1A\033[2K'; }; echo; }
# A passage, labelled the way canon cites it — file:lines — so when act 2
# prints `maple-house.md:3-8` under a rule the room has already seen it.
passage() { printf '\n   \033[2m%s\033[0m\n' "$1:$2"; sed -n "${2/-/,}p" "$DOC" | sed 's/^/   │ /'; }

rm -rf "$DEMO"; mkdir -p "$DEMO/.canon"
export CANON_DIR="$DEMO/.canon"
$CANON init --profile house >/dev/null
[ -f .canon/config ] && cp .canon/config "$CANON_DIR/config"

# PREFLIGHT. Fail in the green room, never in front of the room.
if [ $OFFLINE -eq 0 ]; then
  ep=${CANON_ENDPOINT:-$(sed -n 's/^endpoint *= *//p' "$CANON_DIR/config" 2>/dev/null)}
  if [ -z "$ep" ] || ! curl -s -m 15 "${ep%/}/models" >/dev/null 2>&1; then
    echo "no endpoint reachable${ep:+ at $ep}."
    echo "the live half of act 2 and acts 3-4 need one. --offline skips just those."
    exit 2
  fi
  echo "endpoint  $ep"
fi

# ── curtain ─────────────────────────────────────────────────
if [ $AUTO -eq 0 ]; then
  clear_; printf '\n\n\n\033[1m   canon\033[0m\n\n'
  say "Your house has rules. They're in two years of chat, a handbook"
  say "nobody opened, and someone's memory."
  wait_
fi

# ── act 1 · before ──────────────────────────────────────────
beat "Where we start."
say "One document. Eleven charter articles, thirteen decisions from"
say "monthly meetings, appended over two years. 1,728 words."
hold
say "Article I, at the top:"
passage maple-house.md 3-5
hold
say "A decision, a hundred lines down:"
passage maple-house.md 111-113
hold
say "Same house. Same document. Nobody wrote down that one replaced the other."
wait_

# ── act 2 · read it ─────────────────────────────────────────
# Live first, then the tape. The room watches a tape get cut — `draft` ends
# with "run recorded at" — before it hears the word "recording".
TAPED=0
beat "Point it at the document. No tidying first."
if [ $OFFLINE -eq 0 ]; then
  say "Live. The model reads Article I, the passage you just saw."
  say "Twenty seconds."
  run $CANON draft --dry-run --max-chunks 1 --from "$DOC"
  hold
  say "Four rules. Every one cites maple-house.md:3-8. That's Article I."
  hold
  say "Last line: 'run recorded at'. Every run writes down every reply."
  say "That file is a tape."
  wait_
fi
if [ -f "$TAPE" ]; then
  if [ $OFFLINE -eq 0 ]; then
    sub "The whole document. Same command, all 24 passages."
    say "A quarter of an hour on this box. So, a recording: the 48 replies"
    say "a real 27B gave to this command yesterday, replayed through the"
    say "same pipeline."
  else
    say "A recording: the 48 replies a real 27B gave to this command,"
    say "replayed through the same pipeline."
  fi
  run $CANON draft --replay "$TAPE"
  TAPED=1
else
  say "Accept the first two guest rules. Then the two late reversals:"
  say "no overnight guests, quiet hours back to 10 PM."
  run $CANON draft --from "$DOC"
fi
hold
say "Every proposal quotes its passage or it isn't shown."
hold
say "One at a time. There is no --accept-all."
hold
# THE ACCEPTS. Six of fifty: four Charter rules and the two decisions that
# reverse them. Copied from the tape's own candidates (0, 1, 2, 4, 26, 29).
# The live-draft branch accepted for real and must not be seeded on top of.
[ $TAPED -eq 1 ] && $CANON replay "$SEED" --out "$CANON_DIR" --profile house >/dev/null
say "Reviewed by hand. Six kept."
run $CANON list
wait_

# ── act 3 · ask it ──────────────────────────────────────────
if [ $OFFLINE -eq 0 ]; then
  # Take one from the room if you like: type it in place of the cousin.
  beat "Ask the house a question."
  say "The house's own rules answer, in the house's own words."
  hold
  run $CANON check "my cousin wants to stay for two weeks"
  hold
  say "Two rules. Article I allows two nights. A decision banned guests."
  say "Nobody withdrew the first. Act 1's pair, found."
  wait_

# ── act 4 · what it disagrees with itself about ─────────────
  beat "Ask it what it disagrees with itself about."
  run $CANON tensions
  hold
  say "Proposed, not ruled. A person accepts or dismisses. Nothing is written."
  wait_
fi

# ── the turn ────────────────────────────────────────────────
if [ $OFFLINE -eq 0 ]; then
  turn "Everything that read went through a model. Now pull the cable out."
else
  ACT=$((ACT + 2))   # acts 3 and 4 skipped; keep DEMO.md's numbering
  turn "Now pull the cable out."
fi
hold
say "Nothing past this point calls a model. Not the ledger, not standing,"
say "not the replay, not the counterfactual. Not the founding documents."
wait_

# ── act 5 · carry it knowingly ──────────────────────────────
# The house does something about its own contradiction, with two verbs and no
# model. Ids are content-addressed and come from the seed, so they are stable.
beat "Carry it knowingly."
say "The house can't fix this tonight. It can write down that it knows."
run $CANON accept can-64864f34b1b5 can-ea11bfde216d -m "Article I was never repealed. Revisit at the October meeting."
hold
say "Six months from now, somebody asks why."
run $CANON why can-ea11bfde216d
hold
say "Where it came from. What it's carried against. Why. Nobody has to remember."
wait_

# ── fernwood ────────────────────────────────────────────────
DEMO2="$DEMO/fernwood"
$CANON replay fixtures/fernwood-commons --out "$DEMO2/.canon" --profile house >/dev/null
export CANON_DIR="$DEMO2/.canon"

beat "A different house, two years in."
say "Thirty acts in its ledger, written with the verbs you just saw:"
say "grant, scope, policy, accept, silence. Then two years of living."
hold
say "A bot said these two rules don't conflict. The house disagreed,"
say "carried the contradiction on purpose, and dated it."
run $CANON why can-5e1a8e880e1d
wait_

beat "What this house decided NOT to have."
say "This is what groups lose. Losing it is why the same proposal"
say "comes back every spring."
hold
run $CANON voice human:mira
wait_

beat "Nobody had to remember."
run $CANON overdue
wait_

beat "Two years of governance, replayed against Ostrom's eight."
say "Standing granted and withdrawn. An objection blocking a thing."
say "A scope handed down. A lot drawn from a seed nobody could steer."
hold
run $CANON replay fixtures/fernwood-commons --brief
wait_

beat "The question every group has, and none can answer."
say "They're arguing about dropping consent. What would that have done"
say "to the last two years?"
hold
run $CANON replay fixtures/fernwood-commons --policy default --brief
hold
say "Every group has had this argument. No group has been able to check."
wait_

# ── act 11 · fourteen commons ───────────────────────────────
# The study is a test, so it runs as one. Fourteen ledgers from one 104-line
# spine of the same verbs; only the nouns change. Four broken on purpose.
beat "Fourteen commons. One spine. Four broken on purpose."
say "A fishery, a makerspace, a monorepo, a mesh of pooled machines."
say "Same 104 lines of verbs. Only the nouns change."
hold
run cargo test --test transfer_bar -- --nocapture 2>/dev/null
hold
say "The four ablations go red exactly where each one predicted."
wait_

# ── one more thing ──────────────────────────────────────────
# Still no model. A TAPE: 850 replies a real 27B gave over the founding
# corpus, replayed through the same pipeline. 2.5 seconds.
if [ -f "$FOUNDING" ]; then
  beat "One more thing. We pointed it at the United States."
  say "The Declaration, the Articles of Confederation, the Constitution,"
  say "all twenty-seven amendments. 12,672 words. Cold."
  hold
  say "Same command as act 2. A rented GPU, an hour and thirty-seven"
  say "minutes, 850 replies from a real 27B. This is that run, replayed."
  say "Still nothing plugged in."
  hold
  FDEMO="$DEMO/founding"; rm -rf "$FDEMO"; mkdir -p "$FDEMO/.canon"
  export CANON_DIR="$FDEMO/.canon"
  $CANON init --profile house >/dev/null
  run $CANON draft --replay "$FOUNDING"
  wait_

  beat "283 contradictions proposed. Four of them."
  say "Read from that run file."
  hold
  run python3 ./scripts/founding-highlights.py "$FOUNDING"
  hold
  say "The fourth is wrong. It proposes things that aren't there."
  say "That's why you review one at a time. That's why every proposal"
  say "has to quote its passage."
  wait_
fi
