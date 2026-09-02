#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Automation for the People — the run of show.
#
#   ./scripts/demo.sh              # step through: [enter] between acts, [enter] at each beat
#   ./scripts/demo.sh --auto       # straight through, no clears (what you read back)
#   ./scripts/demo.sh --offline    # no endpoint: skips the live half of act 2 and acts 3-4
#
# ONE QUESTION, THREE TIMES. On what terms can an agent be a member of a
# group? Part one: a house, and the agent's first job — read what the house
# already wrote and make it legible, proposing and never ruling. Part two: a
# house that gave a helper a seat for two years — what it was given, what it
# said, where the people overruled it, and how the whole thing measures
# against Ostrom's eight. Part three: a country, and the same failure two
# hundred years deep. Every part turns on the same moment: a group changed a
# rule and nobody struck the old one or wrote down why.
#
# NOTHING IS STAGED, AND THE SCREEN SAYS WHAT IT IS. Act 2 reads one passage
# live before it replays the recording of the whole document, so the room
# watches a recording get made before it hears the word. Every house here is
# a record written in canon's own verbs. Every recording is a run file
# `draft` wrote about itself. Everything the screen prints is for the room;
# presenter notes are in DEMO.md.
#
# A REPLAY WRITES NOTHING (draft.rs:1482), so after act 2 the six rules a
# presenter kept are materialised from `fixtures/maple-house/accepted` — each
# copied from the recording's own candidates with the citation the model cut —
# and shown with `canon list`.
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
[ -t 1 ] && export COLUMNS=${COLUMNS:-$(tput cols 2>/dev/null || echo 100)}
ACT=0
clear_() { [ $AUTO -eq 1 ] || printf '\033[2J\033[H'; }
beat()  { ACT=$((ACT + 1)); clear_; printf '\n\033[2mact %s\033[0m\n\033[1m%s\033[0m\n\n' "$ACT" "$1"; }
sub()   { clear_; printf '\n\033[2mact %s, continued\033[0m\n\033[1m%s\033[0m\n\n' "$ACT" "$1"; }
# A card: one line, big, nothing else on the screen. For the quotes and the
# part titles. `$2` is a dim attribution.
card()  { clear_; printf '\n\n\n\n\033[1m   %s\033[0m\n' "$1"; [ -n "${2:-}" ] && printf '\n   \033[2m%s\033[0m\n' "$2"; echo; }
say()   { printf '   %s\n' "$1"; }
run()   {
  local shown="" a
  for a in "$@"; do
    [ "$a" = "$CANON" ] && a=canon
    case "$a" in *" "*) a="\"$a\"";; esac
    shown="$shown${shown:+ }$a"
  done
  # Who is typing, when it is not the presenter: `theo $ canon add …`.
  local who="${CANON_ACTOR:-}"; who="${who#human:}"
  printf '\n\033[2m%s$\033[0m \033[1m%s\033[0m\n\n' "${who:+$who }" "$shown"; "$@"; echo
}
# Both pauses read the TERMINAL, not stdin, and first discard anything typed
# while output was scrolling — a stray key pressed during a long print must
# not be taken as the press that continues the show, or land in the shell.
pause_() { while read -r -t 0.05 -n 1000 _ </dev/tty 2>/dev/null; do :; done; read -r _ </dev/tty; }
wait_() { [ $AUTO -eq 1 ] || { printf '\n\033[2m   [enter]\033[0m'; pause_; }; }
hold()  { [ $AUTO -eq 1 ] || { printf '\033[2m   …\033[0m'; pause_; printf '\033[1A\033[2K'; }; echo; }
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

# ── cold open ───────────────────────────────────────────────
if [ $AUTO -eq 0 ]; then
  card "Automation for the People"
  wait_
  card "\"The earth belongs to the living.\"" "Thomas Jefferson to James Madison, 1789"
  wait_
  card "Every group forgets why."
  say "Rules pile up. Reasons don't. Someone changes a rule and nobody"
  say "strikes the old one. Two years later, nobody can say why it's there."
  wait_
  card "Ostrom's eight." "Elinor Ostrom, Governing the Commons, 1990"
  say "She studied ordinary people who shared a pasture, a canal, a fishing"
  say "ground, and kept it going for centuries. No king, no market."
  say "She found eight things they all did. Those are the bar tonight."
  wait_
  card "We're about to add a new kind of member to our groups." "On what terms?"
  wait_
fi

# ═══════════════════════════════════════════════════════════
card "Part one. A house."
wait_

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

# ── act 2 · the agent reads ─────────────────────────────────
TAPED=0
beat "The agent's first job: read what the house already wrote."
if [ $OFFLINE -eq 0 ]; then
  say "Live. It reads Article I, the passage you just saw. Twenty seconds."
  run $CANON draft --dry-run --max-chunks 1 --from "$DOC"
  hold
  say "Four rules. Every one cites maple-house.md:3-8. That's Article I."
  say "A rule it can't point to in the document is never shown."
  hold
  say "Last line: 'run recorded at'. Every reply it got is written down."
  say "That file is a recording."
  wait_
fi
if [ -f "$TAPE" ]; then
  if [ $OFFLINE -eq 0 ]; then
    sub "The whole document. Same command, all 24 passages."
    say "A quarter of an hour on this machine. So, a recording: the 48 replies"
    say "a real model gave to this command yesterday, run back through the"
    say "same steps."
  else
    say "A recording: the 48 replies a real model gave to this command,"
    say "run back through the same steps."
  fi
  run $CANON draft --replay "$TAPE"
  TAPED=1
else
  say "Accept the first two guest rules. Then the two late reversals:"
  say "no overnight guests, quiet hours back to 10 PM."
  run $CANON draft --from "$DOC"
fi
hold
say "Fifty proposals. It proposes. It does not decide."
hold
say "People go through them one at a time. There is no accept-all."
hold
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
  say "Nobody withdrew the first. Act 1's pair, found in a second."
  wait_

# ── act 4 · where it disagrees with itself ──────────────────
  beat "Ask it where the house disagrees with itself."
  run $CANON tensions
  hold
  say "Proposed, not ruled. A person accepts or dismisses. Nothing is written"
  say "until someone does."
  wait_
fi

# ── the turn ────────────────────────────────────────────────
if [ $OFFLINE -eq 0 ]; then
  card "Everything that read went through a model." "Now pull the cable out."
else
  ACT=$((ACT + 2))   # acts 3 and 4 skipped; keep DEMO.md's numbering
  card "Now pull the cable out."
fi
say "Nothing past this point calls a model. Not the record, not who decides,"
say "not the replay, not the what-if. Not the founding documents."
wait_

# ── act 5 · the people decide ───────────────────────────────
beat "The people decide."
say "The house can't fix this tonight. It can write down that it knows."
run $CANON accept can-64864f34b1b5 can-ea11bfde216d -m "Article I was never repealed. Revisit at the October meeting."
hold
say "Six months from now, somebody asks why."
run $CANON why can-ea11bfde216d
hold
say "Where it came from. What it's carried against. Who decided, and why."
say "The model proposed. People disposed. The record keeps the reason."
wait_

# ═══════════════════════════════════════════════════════════
card "Part two. A member."
say "A different house. This one drew a boundary: it said who is in, and"
say "who decides what. Two years in, it gave a helper agent a seat."
wait_

DEMO2="$DEMO/fernwood"
$CANON replay fixtures/fernwood-commons --out "$DEMO2/.canon" --profile house >/dev/null
export CANON_DIR="$DEMO2/.canon"
# Ids are content-addressed, so a wording edit to the fixture moves them.
# Look the hall rule up by its text rather than pinning an id.
HALL=$($CANON list 2>/dev/null | grep -i 'hall stays clear' | grep -o 'can-[0-9a-f]*' | head -1)

# ── act 6 · the hall, the bikes, and the helper ──────────────
# The human story first, or the bot's record means nothing. Dana wrote two
# rules the same day: the hall stays clear for a stroller, and the bikes
# live in the hall. A parent and the cyclists, one corridor. The helper
# looked at the two rules and said they don't conflict. The people living
# with a stroller and three bikes knew better.
beat "Two rules, one hall."
say "Dana wrote both, the same day. The hall stays clear enough for a"
say "stroller. And the bikes live in the hall, against the left wall."
say "A parent and the cyclists. One corridor."
hold
say "The house had given a helper agent a say over the kitchen, with an"
say "end date. The helper looked at the two hall rules and called it:"
say "no conflict."
hold
say "The hall is not the kitchen. The record kept what the helper said,"
say "and it never took effect. The people with the stroller and the three"
say "bikes decided for themselves."
run $CANON why "$HALL"
hold
say "The helper's call, by name and date, marked as outside its seat."
say "Dana's decision, by name and date, with a reason and a revisit date."
say "A member may speak anywhere. It decides only where it was given a say."
wait_

# ── act 7 · the helper's whole record ──────────────────────
beat "Everything the helper was given, said, and had set aside."
run $CANON voice agent:helper
hold
say "A seat, given by a person, that ended in January. One objection,"
say "citing a rule. One call outside its seat, kept and not applied. One"
say "proposal, refused by a cook with a reason. That is a member."
wait_

# ── act 7, continued · a rule is a proposal until the cooks say so ──
# LIVE, no model. Theo holds the house, not the kitchen. The cooks set how
# kitchen rules are made: both of them, jointly. Theo types a kitchen rule
# and it lands as a proposal. Dana approves; still one short. Sam approves;
# it is a rule. Actors are set per command with CANON_ACTOR.
sub "Now somebody who is not a cook writes a kitchen rule."
say "The cooks decided how kitchen rules get made: both of them, jointly."
say "Theo lives here and holds the house. He does not hold the kitchen."
hold
CANON_ACTOR=human:theo run $CANON add "Leftovers are labelled with a name and a date." --scope house.kitchen
hold
say "Written. Visible. Not a rule. Nobody's word has been taken away from"
say "them, and nobody's rule has been written for them."
hold
PROP=$($CANON list 2>/dev/null | grep 'Leftovers are labelled' | grep -o 'can-[0-9a-f]*' | head -1)
CANON_ACTOR=human:dana run $CANON approve "$PROP" -m "we lose a tub of soup a week to this"
hold
CANON_ACTOR=human:sam run $CANON approve "$PROP" -m "yes"
hold
say "Both cooks. Now it is a rule, and the record says who made it one."
wait_

# ── act 8 · nobody had to remember ──────────────────────────
beat "Nobody had to remember."
run $CANON overdue
hold
say "The helper's seat lapsed in January. The bikes come up again in June."
say "Nobody carries this in their head."
wait_

# ── act 9 · what Mira decided not to have ───────────────────
# Two human stories are in Mira's record, and neither reads without setup.
# Wednesday dinners: for two years somebody has just cooked, nobody organised
# it, and when a rotation was proposed Mira said no and wrote down why. That is a
# SILENCE: unwritten on purpose, not by neglect, and the thing groups lose.
# The laundry: the same request three times, and each time it cost more to
# ask, counted from decisions the house made, never from watching anyone.
beat "What Mira decided NOT to have."
say "Every Wednesday for two years, somebody has cooked dinner for the house."
say "Nobody organised it. Nobody was ever asked to."
hold
say "Last spring somebody proposed a rotation. Mira said no, and wrote down why."
hold
run $CANON voice human:mira
hold
say "Unwritten on purpose. A rotation would turn a kindness into a duty. That is"
say "the line groups lose, and losing it is why the same proposal comes back"
say "every spring. Here it stays, with Mira's name and her reason."
hold
say "And the laundry, above it. Someone asked to run the machine at 1am."
say "Then again. First time: ask one person. Second: ask the whole house."
say "The third time, the house's own rule said no before anyone had to."
say "The cost of asking went up each time, counted from what the house had"
say "decided. Nobody kept a file on anyone."
wait_

# ── act 10 · Ostrom's eight ─────────────────────────────────
# THE SAUCE, or this looks like vaporware. The fixture is two files: a script
# of the house's two years, fifty-six steps in canon's own verbs, and a file
# of predicted outcomes written from Ostrom's principles before the replay
# ran. Show one raw step and its prediction, THEN replay. The table's rows
# are backed by scenes, and `--brief` now prints them.
beat "Two years of this house, against Ostrom's eight."
say "Who's in and who decides. Rules the group can change itself. A monitor"
say "the group can overrule. Consequences that escalate. Cheap ways to settle"
say "a fight. The right to organise. Small groups inside bigger ones."
hold
say "What gets replayed: a script of the house's two years. Fifty-six steps."
say "Who was given what. Who objected. What someone proposed. One of them,"
say "as written:"
printf '\n\033[2m'; grep '"bikes-against-the-hall"' fixtures/fernwood-commons/scenario.jsonl | python3 -m json.tool --indent 2 | sed 's/^/     /'; printf '\033[0m'
hold
say "Beside it, a prediction for every step, written before the replay"
say "existed, from what Ostrom's principle says should happen:"
printf '\n\033[2m'; grep -A5 '"bikes-against-the-hall"' fixtures/fernwood-commons/expected.json | sed 's/^/   /'; printf '\033[0m'
hold
say "Replay rebuilds the whole history from the record, step by step, and"
say "checks every prediction. No model. Watch the clock."
run $CANON replay fixtures/fernwood-commons --brief
hold
say "Every line under a principle is a scene: what was asked, what the rules"
say "said. 'All as expected' means all fifty-six matched a prediction made"
say "before the run. That is the test. It can fail, and act 12 shows it failing."
wait_

# ── act 11 · what if ────────────────────────────────────────
beat "What if we had decided differently?"
say "They're arguing about how they decide. What would the other way have"
say "done to the last two years? Every group has this argument."
hold
run $CANON replay fixtures/fernwood-commons --policy default --brief
hold
say "Every decision that would have gone differently, by name, and which way"
say "it moves. Look at the lock. Under the other rule, one person could have"
say "waved through a change nobody could undo."
hold
say "No group has ever been able to check this before."
wait_

# ── act 12 · not just houses. yours. ────────────────────────
# The builder's question is "what would I write?" So show the cost of entry
# before the result: one institution's vocabulary, which is nouns and nothing
# else, then the grid, then the three commands that put an agent on the same
# terms in their repo. `canon mcp` is read-only by design — an agent reads
# what is in force and how a proposal stands; anything it writes, it writes
# as a proposal under a seat with an end date.
beat "Not just houses. Yours."
say "A fishery. A canal. An alpine pasture shared since 1483. A makerspace."
say "A codebase: nine engineers, one repository, and a CI bot nobody wants"
say "to own."
hold
say "What it costs to bring one in. The codebase, as written. Nouns only:"
say "who, what is shared, who watches. There are no rules of the game in"
say "here. The same 104 lines run all fourteen."
printf '\n\033[2m'; { sed -n '2,4p;10,14p' fixtures/cpr/meridian-monorepo/vocab.json; printf '  "members": [ … nine people … ],\n'; grep '"monitor"' fixtures/cpr/meridian-monorepo/vocab.json; printf '  …\n'; } | sed 's/^/     /'; printf '\033[0m'
hold
run cargo test --test transfer_bar -- --nocapture 2>/dev/null
hold
say "Ostrom's eight hold in all ten. Four we broke on purpose, and each one"
say "fails exactly where we said it would. That's how you know the test is real."
hold
say "So, your project. Three commands:"
say ""
say "   canon init --profile code                 the record lives in your repo"
say "   canon grant agent:yourbot repo.deps --horizon 90"
say "                                             a seat: one scope, ninety days"
say "   canon mcp                                 your agent joins over MCP"
say ""
say "Over MCP it reads: what is in force, why, what is open, how a proposal"
say "stands. Anything it writes, it writes as a proposal. Its rulings take"
say "standing. Its seat expires. Same terms as the people."
wait_

# ═══════════════════════════════════════════════════════════
if [ -f "$FOUNDING" ]; then
  card "Part three. A country."
  say "The Constitution still contains the three-fifths clause."
  say "Nobody struck it. Same failure as Article I, two hundred years deep."
  wait_

  # ── act 13 · read it cold ─────────────────────────────────
  beat "The agent reads the oldest rules we have."
  say "The Declaration. The Articles of Confederation. The Constitution and"
  say "all twenty-seven amendments. 12,672 words. Cold."
  hold
  say "Same command as act 2. A rented GPU, an hour and thirty-seven minutes,"
  say "850 replies from a real model. This is that run, replayed."
  hold
  FDEMO="$DEMO/founding"; rm -rf "$FDEMO"; mkdir -p "$FDEMO/.canon"
  export CANON_DIR="$FDEMO/.canon"
  $CANON init --profile house >/dev/null
  run $CANON draft --replay "$FOUNDING"
  wait_

  # ── act 14 · four of 283 ──────────────────────────────────
  beat "283 contradictions proposed. Four of them."
  run python3 ./scripts/founding-highlights.py "$FOUNDING"
  hold
  say "The fourth is wrong. It proposes things that aren't there."
  say "That is why people review, one at a time. That is why every proposal"
  say "has to quote its passage."
  wait_

  # ── close ─────────────────────────────────────────────────
  card "Was it reading, or remembering?"
  say "Every model has read the Constitution. So we took the nine it found,"
  say "removed the fact each one turns on, and asked again."
  hold
  say "Five dropped. Four didn't."
  say "We published both numbers. You should ask that of every agent."
  wait_
fi

card "Automation for the people." "The agent sits under the record, not above it."
say "Jefferson wanted the living to be able to see their rules and revise them."
say "Ostrom showed ordinary people can govern what they share."
say "An agent can be a member on the same terms as anyone: it cites or it's"
say "silent, it proposes and people decide, its seat is given, bounded, and"
say "written down. Under the record. Not above it."
wait_

# ── curtain call ────────────────────────────────────────────
# End on a laugh. A cat flies across the terminal trailing a rainbow, and
# leaves behind the line Ostrom is said to have given a room of theorists
# who told her that self-governing commons could not work. Skipped in --auto:
# a rainbow in a log file is just escape codes.
nyan() {
  local w=${COLUMNS:-100} i c
  local cols=$((w - 14))
  local -a cat=( ' ,------,   ' ' |   /\_/\  ' '~|__( ^ .^) ' '  ""  ""    ' )
  local -a hue=(196 208 226 46 33 129)
  clear_; printf '\033[?25l'
  for ((i = 0; i <= cols; i++)); do
    printf '\033[H\n\n\n\n\n'
    for ((c = 0; c < 4; c++)); do
      printf '\033[38;5;%sm%*s\033[0m' "${hue[$(( (c + i / 2) % 6 ))]}" "$i" "" | tr ' ' '='
      printf '%s\n' "${cat[c]}"
    done
    sleep 0.02
  done
  printf '\033[?25h'
  sleep 0.6
}
if [ $AUTO -eq 0 ]; then
  nyan
  card "\"A resource arrangement that works in practice can work in theory.\"" "Elinor Ostrom"
  say "So can a demo."
fi
