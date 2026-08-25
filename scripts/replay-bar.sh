#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Score the current build against RECORDED runs. No endpoint, no model, no wait.
#
#   ./scripts/replay-bar.sh [corpus] [taped-runs-dir]
#   ./scripts/replay-bar.sh maple-house
#
# This is the fast loop. Every arm on 2026-08-24 cost about an hour because it
# re-ran the whole pipeline — 24 of ~36 calls are extraction, and no arm that
# day changed extraction. A taped run carries every reply the endpoint gave, so
# the pure code below those calls can be re-scored in seconds.
#
# WHAT IT JUDGES: anything downstream of a model call — the citation cut, the
# silence guard, the quantity guard, the fold, the convergence threshold, the
# tension rendering, the scorer itself.
#
# WHAT IT CANNOT JUDGE, and refuses rather than fudging: a change to the CALLS.
# A different prompt, schema, chunking or pass count makes the recording the
# wrong evidence, and the tape says so and exits 3 ("cannot judge"). Re-record
# with ./scripts/draft-bar.sh when that happens — a prompt change always needs
# live runs.
set -eu

CORPUS=${1:-maple-house}
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
SRC=${2:-"$ROOT/fixtures/$CORPUS/runs/qwen-27b-taped"}
[ -d "$SRC" ] || { echo "no taped runs at $SRC"; echo "record some: ./scripts/draft-bar.sh 3"; exit 2; }

cargo build --manifest-path "$ROOT/Cargo.toml" 2>&1 | grep -E '^error' && exit 1
BIN=${CANON_BAR_BIN:-"$ROOT/target/debug/canon"}
OUT=$(mktemp -d)/replayed
mkdir -p "$OUT"

echo "replaying $CORPUS from $SRC"
echo "build     $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown), \
$(git -C "$ROOT" status --porcelain 2>/dev/null | wc -l | tr -d ' ') uncommitted file(s)"
echo

n=0; skipped=0
for f in "$SRC"/*.json; do
  [ -e "$f" ] || break
  # A run with no tape cannot be replayed. Named, never silently skipped: a
  # bar scored over two of three runs while claiming three is the failure the
  # zero-test exit exists to stop.
  if ! grep -q '"tape"' "$f"; then
    echo "  SKIPPED (no tape) $(basename "$f")"
    skipped=$((skipped + 1)); continue
  fi
  SCRATCH=$(mktemp -d)
  CANON_DIR="$SCRATCH/.canon"; export CANON_DIR CANON_ACTOR="bench:replay"
  "$BIN" init --profile house >/dev/null
  if "$BIN" draft --replay "$f" >/dev/null 2>&1; then
    for r in "$CANON_DIR/draft-runs/"*.json; do
      [ -e "$r" ] || break
      cp "$r" "$OUT/$(basename "$f")"
    done
    n=$((n + 1))
  else
    code=$?
    echo "  REFUSED (exit $code) $(basename "$f") — the tape and this build disagree;"
    echo "    re-record with ./scripts/draft-bar.sh if you changed a prompt, schema or chunking"
  fi
  rm -rf "$SCRATCH"
done

echo
if [ "$n" -eq 0 ]; then
  echo "nothing replayed ($skipped without a tape). A bar over zero runs is not a result."
  exit 4
fi
echo "$n run(s) replayed$([ "$skipped" -gt 0 ] && echo ", $skipped skipped")  ->  $OUT"
echo
cd "$ROOT"
CANON_BAR_RUNS="$OUT" cargo test --test draft_bar -- --ignored --nocapture
