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
# house that gave a helper a seat — what it was given, what it
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
  say "Every group forgets why. Rules pile up. Reasons don't. Someone changes"
  say "a rule and nobody strikes the old one. Two years later, nobody can say"
  say "why it's there."
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
say "monthly meetings, appended over six months. 1,728 words."
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
  say "It reads Article I, the passage you just saw. Twenty seconds."
  run $CANON draft --dry-run --max-chunks 1 --from "$DOC"
  hold
  say "Four rules. Every one cites maple-house.md:3-8. That's Article I."
  say "A rule it can't point to in the document is never shown."
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
say "Fifty proposals. It proposes. It does not decide. People go through"
say "them one at a time. There is no accept-all."
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
# `accept` accepts that the two CONTRADICT and carries both — it does not
# accept the ban. The reason says the decision, not the problem, and the
# date is passed as a date so `overdue` can raise it, as act 6's June does.
say "The house can't fix this tonight. It can carry both, say why, and set"
say "a date."
run $CANON accept can-64864f34b1b5 can-ea11bfde216d --revisit 2026-10-01 -m "Both stand for now. Article I was never repealed and the March decision was made; which one holds is for the October meeting."
say "Six months from now, somebody asks why."
run $CANON why can-ea11bfde216d
hold
say "Where it came from. What it's carried against. Who decided, and why."
say "The model proposed. People disposed. The record keeps the reason."
wait_

# ═══════════════════════════════════════════════════════════
# PART TWO IS ABOUT TERMS, NOT CONFLICTS. Part one already showed a
# contradiction found and carried; the room does not need it again. Every
# beat here is a term of membership the room has not seen yet: a seat has a
# room and an end date; a member may speak anywhere and decide only in its
# room; a rule is a proposal until the group's own rule for making rules is
# met; a seat expires with nobody remembering. The hall and the bikes are
# one line of context for the helper's record, not a story of their own.
card "Part two. A member."
say "In part one the agent read, and the people decided."
say "This house went further. It said who is in, who decides what, and for"
say "how long. Then it gave a helper agent a seat."
say "Three questions. What was it given? What did it say? Where did the"
say "people overrule it?"
wait_

DEMO2="$DEMO/fernwood"
$CANON replay fixtures/fernwood-commons --out "$DEMO2/.canon" --profile house >/dev/null
export CANON_DIR="$DEMO2/.canon"

# ── act 6 · the seat ────────────────────────────────────────
# EVENTS FIRST, THEN THE READBACK. Every line in this house is one command
# run by one person or one program under its own name, so the room sees the
# six commands that made the helper's record before `voice` reads them
# back — otherwise `voice` is three sections of ids with no story under
# them. The ids are resolved live from the canon the room is looking at.
# The helper's two moves are different in kind: a position citing a rule is
# READING, which is the seat's job; a dismiss is a RULING, and the hall is
# not the kitchen, so the fold kept the line and applied nothing.
HALL=$($CANON list 2>/dev/null | grep 'stroller through' | grep -o 'can-[0-9a-f]*' | head -1)
BIKES=$($CANON list 2>/dev/null | grep 'Bikes live in the hall' | grep -o 'can-[0-9a-f]*' | head -1)
beat "What the helper was given, said, and had set aside."
say "Every line in this house is one command, run by one person or one"
say "program, under its own name. The helper is a program: it runs the same"
say "commands as everyone, and over MCP it can only read. It runs when the"
say "record changes and on a nightly schedule, with the house's model; that"
say "crontab is the house's, and the record holds only what it wrote."
say "Six commands, five months:"
say ""
say "   2025-09-05  mira    canon grant agent:helper house.kitchen --horizon 2026-01-31"
say "   2025-09-14  dana    canon add \"The hall stays clear enough to get a stroller through.\""
say "                       canon add \"Bikes live in the hall, against the left wall.\""
say "   2025-11-08  theo    canon position \"bikes in the hall\" --toward --citing $BIKES"
say "   2025-11-09  helper  canon position \"bikes in the hall\" --against --citing $HALL"
say "   2025-11-10  helper  canon dismiss $HALL $BIKES -m \"tensions flagged these two. Read both: not a conflict.\""
say "   2026-02-18  dana    canon accept $HALL $BIKES --revisit 2026-06-01"
hold
# IN ORDER: September, the seat and the rules. November, the two days.
# January, the seat lapses. February, Dana rules. The record is read back
# between November and January, and Dana's ruling is told after the
# `overdue` that shows the June date it set.
say "September. Mira gave the helper one room, the kitchen, with an end date."
say "That is a seat. Nine days later Dana wrote the two hall rules."
say ""
say "November. Theo asked, on the record: a position for the bikes. The"
say "record changed, so the helper ran check, the command from act 3, and"
say "filed what it found: the stroller rule pulls against. That is reading,"
say "and it is what the seat is for. A member may speak anywhere."
hold
say "That night its job ran tensions, the command from act 4, which proposes"
say "pairs for a person to rule on. It ruled instead: no conflict. The hall"
say "is not the kitchen. The line is in the record and changed nothing."
say "Speak anywhere. Decide only where it was given a say."
hold
say "One command reads the helper's record back."
run $CANON voice agent:helper
hold
say "January came."
run $CANON overdue
say "The seat lapsed on its date. Nobody had to remember. And in February,"
say "Dana, who has stood in that hall, ruled the same way the helper read it,"
say "with a date to look again: June. June has come and gone, and the record"
say "is still asking. It may even have been right. It was not its call."
wait_

# ── act 7 · a rule is a proposal until the cooks say so ─────
# LIVE, no model. Theo holds the house, not the kitchen. The cooks set how
# kitchen rules are made: both of them, jointly. Theo types a kitchen rule
# and it lands as a proposal. Dana approves; still one short. Sam approves;
# it is a rule. Actors are set per command with CANON_ACTOR.
# THE SEATS FIRST. A seat is one grant line: who holds a room, until when.
# You may grant a room you hold or the room above it; the first grant in an
# empty record needs nobody's leave, which is how a house is founded. The
# room sees the four commands that gave the cooks the kitchen and set its
# rule before Theo writes into it, so `standing` reads as what it is.
beat "A rule is a proposal until the group says so."
say "In part one, anyone could write a rule and it was a rule on arrival."
say "Here, a seat is one line: who holds a room, and until when. You may"
say "grant a room you hold, or the room above it. The first grant in an"
say "empty record needs nobody's leave; that is how a house is founded."
say ""
say "   2025-09-03  mira  canon grant human:mira house -m \"founding member\"     the first line"
say "   2025-09-03  mira  canon grant human:theo house                          and ten more"
say "   2025-09-04  mira  canon grant human:dana house.kitchen -m \"cooks most nights\""
say "                     canon grant human:sam  house.kitchen -m \"cooks most nights\""
say "   2026-03-05  dana  canon ratification set joint:human:dana,human:sam --scope house.kitchen"
say ""
say "Whoever holds a room writes its rules; that is the default, called"
say "standing. The cooks raised the kitchen's bar: both of them, jointly."
hold
say "Theo lives here and holds the house. He does not hold the kitchen."
say "He writes a kitchen rule."
CANON_ACTOR=human:theo run $CANON add "Leftovers are labelled with a name and a date." --scope house.kitchen
say "Written. Visible. Not a rule. Nobody's word was taken from them, and"
say "nobody's rule was written for them."
hold
PROP=$($CANON list 2>/dev/null | grep 'Leftovers are labelled' | grep -o 'can-[0-9a-f]*' | head -1)
CANON_ACTOR=human:dana run $CANON approve "$PROP" -m "we lose a tub of soup a week to this"
hold
CANON_ACTOR=human:sam run $CANON approve "$PROP" -m "yes"
say "Both cooks. Now it is a rule, and the record says who made it one."
hold
# THE THIRD LEVEL. Ostrom separated three: operational rules (what you may
# do), collective-choice rules (how those get made), constitutional rules
# (how the collective-choice rules themselves change). The room has now
# seen two. The third is shown as an attempt: the helper, seat lapsed, tries
# to drop the kitchen back to "whoever holds it writes directly". Changing
# how a scope makes rules takes standing over that scope or the one above
# it (ratify.rs, may_govern), so it is kept on the record and not applied.
say "Ostrom counted three levels of rules. You have now seen two."
say ""
say "   what you may do              leftovers get a name and a date"
say "   how those rules get made     both cooks, jointly"
say "   who may change that          ?"
hold
say "The helper tries."
CANON_ACTOR=agent:helper run $CANON ratification set standing --scope house.kitchen -m "simpler this way"
say "Kept on the record. Not applied. The helper's seat lapsed in January."
hold
# THEO MAY. He holds the house, which covers the kitchen, so he may write
# how the kitchen decides — and the change is judged the way the kitchen
# decides today: both cooks. Two answers side by side: NOT APPLIED for no
# say, PROPOSED for a say without the last word.
say "Theo tries. He holds the house, which covers the kitchen."
CANON_ACTOR=human:theo run $CANON ratification set standing --scope house.kitchen -m "simpler this way"
say "Not applied is for someone with no say. Proposed is for someone with a"
say "say and not the last word. Changing how the kitchen decides is judged"
say "the way the kitchen decides today: both cooks."
say ""
say "   who may change that          the cooks, by the cooks' own rule"
say ""
say "Three levels, and the record answers at all three."
wait_

# THE GARDEN WENT FURTHER. A garden rule is carried twice, and somebody who
# was not there for the first vote has to have joined before the second. It
# is Jefferson's line as a mechanism, and `why` shows the two votes with
# Kit's arrival between them.
# THE FIVE WAYS, in house words, before the garden picks the fifth. Each
# room chooses how a written line becomes a rule; the kitchen chose joint,
# the garden chooses twice. Ola's April approval is on screen because it is
# the line that shows the mechanism: the same two people cannot carry it.
sub "The garden went further."
GARDEN=$($CANON list 2>/dev/null | grep 'Water before nine' | grep -o 'can-[0-9a-f]*' | head -1)
say "Each room picks how a written line becomes a rule. Five ways ship:"
say ""
say "   standing         whoever holds the room writes it; anyone else needs one holder's yes"
say "   joint:a,b        everyone named says yes                                  the kitchen"
say "   threshold:2/1    two holders say yes; one reasoned no stops it"
say "   consent:7d       seven days pass and no holder has objected"
say "   twice:…          carried twice, with days or a new holder in between      the garden"
say ""
say "The default is standing. The kitchen raised it to joint. The garden went"
say "further: a rule is carried twice, and somebody who wasn't in the garden"
say "for the first vote has to have joined before the second."
hold
say "Five commands, one month:"
say ""
say "   2026-04-02  ola   canon ratification set twice:turnover:standing --scope house.garden"
say "   2026-04-03  juno  canon add \"Water before nine or after seven.\" --scope house.garden"
say "   2026-04-10  ola   canon approve $GARDEN                    counts for nothing: same two people"
say "   2026-05-01  mira  canon grant human:kit house.garden"
say "   2026-05-03  ola   canon approve $GARDEN                    the second vote"
run $CANON why "$GARDEN"
hold
say "Juno holds the garden, so her own write was the first vote. Ola's yes a"
say "week later changed nothing: she was there for the first vote too. Then"
say "Kit moved in, and Ola's yes counted."
say "The earth belongs to the living. In the garden a rule doesn't count"
say "until somebody who wasn't there yet says yes too."
wait_

# ── act 8 · what Mira decided not to have ───────────────────
# Wednesday dinners: for two years somebody has just cooked, nobody organised
# it, and when a rotation was proposed Mira said no and wrote down why. That
# is a SILENCE: unwritten on purpose, and the thing groups lose. The laundry
# above it is the sanctions ladder in two lines; say it in two.
beat "What Mira decided NOT to have."
say "Every Wednesday for two years, somebody has cooked dinner for the house."
say "Nobody organised it. A year ago somebody proposed a rotation. Mira said"
say "no, and wrote down why. Her commands, and the two about the laundry:"
say ""
say "   2025-09-20  mira  canon silence \"who cooks on a wednesday\" -m \"it has sorted itself out"
say "                       every week for two years, and a rotation would turn a kindness into a duty\""
say "   2026-02-16  mira  canon decide \"laundry after midnight\" --outcome conflicts --authority ask-one"
say "   2026-02-17  mira  canon decide \"laundry after midnight\" --outcome conflicts --authority ask-panel"
run $CANON voice human:mira
hold
say "Unwritten on purpose. A rotation would turn a kindness into a duty."
say "Groups lose that line, and the same proposal comes back every spring."
say "Here it stays, with Mira's name and her reason."
hold
say "Above it, the laundry. Someone asked to run the machine at 1am, then"
say "asked again. First time, ask one person. Second, ask the house. The"
say "cost of asking went up, counted from what the house had decided."
say "Nobody kept a file on anyone."
wait_

# ── act 9 · Ostrom's eight ──────────────────────────────────
# THE TEST, framed as recognition. The room has just watched this house do
# most of the eight; the legend names each one in the room's words and says
# where they saw it, so the table that follows is a recap they can read,
# not eight new terms. The raw step and its prediction stay off the stage
# (they are in DEMO.md for anyone who asks how the test can fail).
beat "This house, against Ostrom's eight."
say "Ostrom found eight things every group that lasts does. You have just"
say "watched this house do them."
say ""
say "   1  who's in, who decides               the kitchen and its cooks"
say "   2  rules that fit the place            three inherited, five written here"
say "   3  the group changes its own rules     Theo's leftovers, the garden's two votes"
say "   4  a watcher the group can overrule    the helper"
say "   5  consequences that escalate          the laundry"
say "   6  settle a fight cheaply              the bikes"
say "   7  the right to organise               nobody above can take its rules"
say "   8  small groups inside bigger ones     a kitchen inside a house"
hold
say "Now the whole record, a year of this house, rebuilt and checked against"
say "all eight. Every step's answer was written down before the replay ran, so"
say "it can fail. No model."
run $CANON replay fixtures/fernwood-commons --brief
hold
say "Every line under a number is a moment you saw: what was asked, what the"
say "rules said. Two rows say 'left to people': the tool stays out of those."
say "Seventy-one steps, all as expected, in milliseconds."
wait_

# ── act 10 · what if ────────────────────────────────────────
beat "What if we had decided differently?"
say "They're arguing about how they decide. What would the other way have"
say "done to the last year? Every group has this argument."
run $CANON replay fixtures/fernwood-commons --policy default --brief
hold
say "Every decision that would have gone differently, by name, and which way"
say "it moves. Look at the lock. Under the other rule, one person could have"
say "waved through a change nobody could undo."
wait_

# ── act 11 · yours ──────────────────────────────────────────
# The builders' answer, one screen. The transfer study that used to fill
# this act runs as a continuation with SHOW_TRANSFER=1.
beat "Yours."
say "Three commands put an agent in your repository on the same terms as the"
say "people in it:"
say ""
say "   canon init --profile code                 the record lives in your repo"
say "   canon grant agent:yourbot repo.deps --horizon 90"
say "                                             a seat: one scope, ninety days"
say "   canon mcp                                 your agent joins over MCP"
say ""
say "Over MCP it reads: what is in force, why, what is open, how a proposal"
say "stands. Anything it writes, it writes through the same commands as"
say "everyone, as a proposal, under its own name. Its rulings take standing."
say "Its seat expires. Same terms as the people."
wait_

# ── act 11, continued · the transfer study ── OFF BY DEFAULT ─
# Fourteen places from one spine. Whole, behind SHOW_TRANSFER=1: the show
# ran long, and the three commands above are what the room came for.
if [ "${SHOW_TRANSFER:-0}" -eq 1 ]; then
  # The builder's question is "what would I write?" So show the cost of entry
  # before the result: one institution's vocabulary, which is nouns and nothing
  # else, then the grid, then the three commands that put an agent on the same
  # terms in their repo. `canon mcp` is read-only by design — an agent reads
  # what is in force and how a proposal stands; anything it writes, it writes
  # as a proposal under a seat with an end date.
  sub "Not just houses."
  say "Fernwood was one house. Is any of this a house thing? Here is the test."
  say "Take the same year of governance, the same 104 commands in the same"
  say "order, and run it in fourteen places, changing only the names. A"
  say "fishery. A canal. An alpine pasture shared since 1483. A makerspace."
  say "A codebase: nine engineers, one repository, and a CI bot nobody wants"
  say "to own."
  say ""
  say "This is everything the codebase supplies. Names only: who is in, what"
  say "the rooms are called, who watches. No rule, no vote count, no policy"
  say "may appear in here, and the build fails if one does."
  printf '\n\033[2m'; { sed -n '2,4p;10,14p' fixtures/cpr/meridian-monorepo/vocab.json; printf '  "members": [ … nine people … ],\n'; grep '"monitor"' fixtures/cpr/meridian-monorepo/vocab.json; printf '  …\n'; } | sed 's/^/     /'; printf '\033[0m'
  hold
  run cargo test --test transfer_bar -- --nocapture 2>/dev/null
  say "Fourteen rows, one per place. Eight columns, Ostrom's eight, checked the"
  say "way act 9 checked the house. A dot: it held there. Ten real places, all"
  say "eight hold. The four marked ablation we broke on purpose: took away the"
  say "boundary, imposed the rules from outside, removed the watcher, let the"
  say "upstream capture the record. Each fails at exactly the principle we"
  say "broke, and nowhere else. An n is a principle that place says does not"
  say "apply, with a reason. A test that cannot fail is not one."
  say ""
  say "The three lines under it are act 10's what-if, run in all ten places:"
  say "each rule moves the same decisions everywhere. One signature."
  hold
  wait_
fi

# ═══════════════════════════════════════════════════════════
if [ -f "$FOUNDING" ]; then
  card "Part three. A country."
  say "The Constitution still contains the three-fifths clause."
  say "Nobody struck it. Same failure as Article I, two hundred years deep."
  wait_

  # GÖDEL. The other failure, and the one act 7 just answered in a house:
  # the amendment clause governing its own amendment. Morgenstern's memo
  # records the episode and not the argument, so "the best guess" is the
  # honest phrase. The last line is the segue into the agent reading.
  card "\"I have discovered a legal route to a dictatorship.\"" "Kurt Gödel, 1947, on the way to his citizenship hearing"
  say "Einstein spent the drive talking him out of telling the judge. The best"
  say "guess at what he found is Article V: the clause that says how to amend"
  say "the Constitution can be used to amend itself. That is the kitchen's"
  say "third level, at the scale of a country, with nothing above it."
  say ""
  say "He didn't get there from a theorem. He got there by reading the document."
  wait_

  # ── act 13 · read it cold ─────────────────────────────────
  beat "The agent reads the oldest rules we have."
  say "The Declaration. The Articles of Confederation. The Constitution and"
  say "all twenty-seven amendments. 12,672 words. Cold."
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
  say "The fourth is wrong. It proposes things that aren't there."
  say "That is why people review, one at a time. That is why every proposal"
  say "has to quote its passage."
  wait_

  # ── close ─────────────────────────────────────────────────
  card "Was it reading, or remembering?"
  say "Every model has read the Constitution. So we took the nine it found,"
  say "removed the fact each one turns on, and asked again."
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
