#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# The run of show.
#
#   ./scripts/demo.sh              # step through with [enter]
#   ./scripts/demo.sh --auto       # straight through
#   ./scripts/demo.sh --offline    # skip the three model beats
#
# THE SHAPE. Acts 1-3 use a model, because that is what the tool does when you
# point it at a mess you already have. Act 4 unplugs, and everything after it
# runs with no model and no network — which lands as a REVEAL rather than as a
# constraint, and only because the room has just watched the model half work.
#
# Act 1 can run from a TAPE — every reply a real endpoint gave, recorded, so
# the ingest is real output with no live risk. `scripts/record-demo-tape.sh`
# cuts one. A tape only replays against the build it was cut on: the call
# sequence is checked, and a stale tape refuses rather than answering from the
# wrong recording. Acts 2 and 3 have no tape and are always live.
set -uo pipefail
cd "$(dirname "$0")/.."
CANON=${CANON:-./target/debug/canon}
[ -x "$CANON" ] || { echo "build first: cargo build" >&2; exit 1; }

DEMO=${DEMO_DIR:-/tmp/canon-demo}
TAPE=${TAPE:-fixtures/maple-house/runs/demo-tape/run.json}
DOC=fixtures/maple-house/maple-house.md
AUTO=0; OFFLINE=0
for a in "$@"; do
  [ "$a" = "--auto" ] && AUTO=1
  [ "$a" = "--offline" ] && OFFLINE=1
done

beat() { printf '\n\033[1m── %s\033[0m\n\n' "$1"; }
say()  { printf '   %s\n' "$1"; }
run()  { printf '\n\033[2m$ %s\033[0m\n' "$*"; "$@"; }
wait_() { [ $AUTO -eq 1 ] || { printf '\n\033[2m   [enter]\033[0m'; read -r _; }; }

rm -rf "$DEMO"; mkdir -p "$DEMO/.canon"
export CANON_DIR="$DEMO/.canon"
$CANON init --profile house >/dev/null
# A scratch canon has no config, so the model beats resolve their endpoint
# from the environment or from the canon you already have here. Carry it over
# rather than discovering on stage that acts 2 and 3 have nowhere to call.
[ -f .canon/config ] && cp .canon/config "$CANON_DIR/config"

# PREFLIGHT. Fail in the green room, never in front of the room. Acts 2 and 3
# are live by design — there is no tape for `check` or `tensions` — so if
# there is no endpoint the honest thing is to say so now and offer --offline.
if [ $OFFLINE -eq 0 ]; then
  ep=${CANON_ENDPOINT:-$(sed -n 's/^endpoint *= *//p' "$CANON_DIR/config" 2>/dev/null)}
  if [ -z "$ep" ] || ! curl -s -m 5 "${ep%/}/models" >/dev/null 2>&1; then
    echo "no endpoint reachable${ep:+ at $ep}."
    echo "acts 2-3 (check, tensions) need one — they have no tape."
    echo "run with --offline to skip just those two; every other act is taped."
    exit 2
  fi
  echo "endpoint  $ep"
fi

# Act 1 needs no endpoint once its tape is cut — the tape IS the model's
# output, replayed through the same pipeline. Only acts 2 and 3 are live.
if [ -f "$TAPE" ] || [ $OFFLINE -eq 0 ]; then
  beat "A house. Two years of decisions, in documents nobody has read."
  say "Point it at the folder. No tidying first."
  if [ -f "$TAPE" ]; then
    say "(This is a RECORDING — every reply a real 27B gave over these"
    say " documents, replayed through the same pipeline. Say so out loud.)"
    run $CANON draft --replay "$TAPE"
  else
    run $CANON draft --from "$DOC"
  fi
  say ""
  say "Every proposal quotes the passage it came from, or it isn't shown."
  say "You go through them one at a time. There is no --accept-all."
  wait_
fi

if [ $OFFLINE -eq 0 ]; then
  beat "Now ask the house a question. Somebody give me one."
  say "This is the house's own rules answering, in the house's own words."
  run $CANON check "my cousin wants to stay for two weeks"
  wait_

  beat "And ask it what it disagrees with itself about."
  run $CANON tensions
  wait_
fi

# ── the turn ────────────────────────────────────────────────
if [ $OFFLINE -eq 0 ]; then
  beat "Everything so far went through a model. Now pull the cable out."
else
  beat "Now pull the network cable out."
fi
say "Nothing past this point calls one. Not the ledger, not standing,"
say "not scopes, not the replay, not the counterfactual — and not the"
say "founding documents at the end."
wait_

DEMO2="$DEMO/fernwood"
$CANON replay fixtures/fernwood-commons --out "$DEMO2/.canon" --profile house >/dev/null
export CANON_DIR="$DEMO2/.canon"

beat "A different house, and a rule with a history."
say "A bot looked at the hall and said these two do not conflict."
say "The house disagreed, carried the contradiction on purpose, and dated it."
run $CANON why can-5e1a8e880e1d
wait_

beat "What this house decided NOT to have."
say "This is what groups lose, and losing it is why the same proposal"
say "comes back every spring."
run $CANON voice human:mira
wait_

beat "And nobody had to remember."
run $CANON overdue
wait_

beat "Two years of governance, replayed. Fifty-five milliseconds."
run $CANON replay fixtures/fernwood-commons
wait_

beat "The question every group has, and none can answer."
say "They are arguing about dropping consent. What would that have done"
say "to the last two years?"
run $CANON replay fixtures/fernwood-commons --policy default --brief
wait_

beat "Ten commons. One spine of 104 lines. Four broken on purpose."
run ./scripts/cpr-sweep.sh
wait_

# ── one more thing ──────────────────────────────────────────
# Still no model. This is a TAPE: 850 replies a real 27B gave over the
# founding corpus, replayed through the same pipeline — citation cutting,
# the guards, the fold. Real output, no live risk, 2.5 seconds.
FOUNDING=fixtures/founding/runs/pod-27b/sweep/run-1788143950.json
if [ -f "$FOUNDING" ]; then
  beat "One more thing. We pointed it at the United States."
  say "The Declaration, the Articles of Confederation, the Constitution"
  say "with all twenty-seven amendments. 12,672 words. Cold."
  say ""
  say "Still nothing plugged in — this is a recording of what a real model"
  say "said, run back through the same pipeline."
  FDEMO="$DEMO/founding"; rm -rf "$FDEMO"; mkdir -p "$FDEMO/.canon"
  CANON_DIR="$FDEMO/.canon" $CANON init --profile house >/dev/null
  run env CANON_DIR="$FDEMO/.canon" $CANON draft --replay "$FOUNDING"
  wait_

  beat "283 contradictions proposed. Four of them."
  run python3 ./scripts/founding-highlights.py "$FOUNDING"
  say ""
  say "The fourth one is wrong. It proposes things that are not there —"
  say "which is why you review one at a time, and why every proposal has"
  say "to quote the passage it came from."
fi
