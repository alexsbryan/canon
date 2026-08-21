#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Produce draft runs for the Maple House bar, then score them.
#
#   ./scripts/draft-bar.sh [runs]        default 3
#
# Each run is a `canon draft --dry-run` over the vendored fixture against a
# real endpoint. Nothing is written to any canon you own: each run gets its
# own throwaway .canon under a temp directory, and only the run artifact is
# kept.
#
# DO NOT REBUILD WHILE A SWEEP IS RUNNING. The runs invoke
# target/debug/canon, so a `cargo build` in another terminal swaps the
# binary mid-sweep and the artifacts are no longer one instrument (§18.4).
# Set CANON_BAR_BIN to a copy of the binary if you need to keep working.
#
# REPEATS ARE THE POINT. One run is an anecdote; the spread between runs over
# the same document is the noise floor every published number has to clear
# (§18.5). The scorer refuses fewer than three.
set -eu

# --runs-only produces artifacts and stops. Used when the bar runs in the
# background: `cargo test` here would fight a foreground build for the target
# directory lock, and a blocked build reads as a hung bar.
RUNS_ONLY=0
[ "${2:-}" = "--runs-only" ] && RUNS_ONLY=1
[ "${1:-}" = "--runs-only" ] && { RUNS_ONLY=1; set -- 3; }

RUNS=${1:-3}
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
# One directory per model: mixing two models' artifacts into one score
# produces a mean about nothing.
OUT=${CANON_BAR_OUT:-"$ROOT/fixtures/maple-house/runs"}
DOC="$ROOT/fixtures/maple-house/maple-house.md"
ENDPOINT=${CANON_ENDPOINT:-http://localhost:9741/v1}
MODEL=${CANON_MODEL:-primary}

if [ "$RUNS_ONLY" -eq 0 ]; then
  cargo build --manifest-path "$ROOT/Cargo.toml" 2>&1 | grep -E '^error' && exit 1
fi
# CANON_BAR_BIN pins the binary for the whole sweep. Set it to a COPY when
# you intend to keep working in the tree: every run must come from one build
# or the artifacts are not one instrument (§18.4).
BIN=${CANON_BAR_BIN:-"$ROOT/target/debug/canon"}

mkdir -p "$OUT"
echo "endpoint  $ENDPOINT (model $MODEL)"
echo "document  $DOC"
echo "runs      $RUNS -> $OUT"
echo

i=1
while [ "$i" -le "$RUNS" ]; do
  SCRATCH=$(mktemp -d)
  CANON_DIR="$SCRATCH/.canon"
  export CANON_DIR CANON_ACTOR="bench:draft-bar"
  "$BIN" init >/dev/null
  "$BIN" config set endpoint "$ENDPOINT" >/dev/null
  "$BIN" config set model "$MODEL" >/dev/null

  echo "--- run $i/$RUNS ---"
  START=$(date +%s)
  # A run that fails is reported and the bar continues: losing runs 2 and 3
  # because run 1 died leaves no measurement at all, and a missing run is
  # visible in the scorer's count.
  if "$BIN" draft --dry-run --from "$DOC" >/dev/null; then
    echo "  $(( $(date +%s) - START ))s"
  else
    echo "  FAILED after $(( $(date +%s) - START ))s (exit $?)"
  fi

  # Name the artifact by run ordinal AND its own timestamp, so a re-run
  # appends rather than silently replacing evidence behind a published number.
  for f in "$CANON_DIR/draft-runs/"*.json; do
    cp "$f" "$OUT/run-$(basename "$f")"
  done
  rm -rf "$SCRATCH"
  i=$((i + 1))
done

echo
if [ "$RUNS_ONLY" -eq 1 ]; then
  echo "$(ls "$OUT" | wc -l | tr -d ' ') artifact(s) in $OUT"
  echo "score them: cargo test --test draft_bar -- --ignored --nocapture"
  exit 0
fi
echo "scoring $(ls "$OUT" | wc -l | tr -d ' ') artifact(s)"
cd "$ROOT" && cargo test --test draft_bar -- --ignored --nocapture
