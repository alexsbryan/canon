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
#
# COST. The comparison stage runs once per arrangement and folds the results,
# so a sweep is `arrangements * k(k+1)/2` calls per run for k blocks of
# twelve: 20 passes per run on Maple House and 72 on Des Moines at two
# arrangements. Budget roughly double what a single-arrangement sweep took,
# and read the scorer's `arrangements` table to see what the second one
# bought — an arrangement whose `added` column stays empty is pure cost.
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
# One document per sweep. CANON_BAR_DOC points the bar at a different corpus;
# CANON_BAR_PROFILE picks the voice extraction writes in, which is a property
# of the document and not of the tool — a municipal code is a body governing
# itself, so it reads as `house`, not `personal`.
DOC=${CANON_BAR_DOC:-"$ROOT/fixtures/maple-house/maple-house.md"}
PROFILE=${CANON_BAR_PROFILE:-house}
ENDPOINT=${CANON_ENDPOINT:-http://localhost:9741/v1}
MODEL=${CANON_MODEL:-primary}
# Similarity ordering, and it defaults OFF because it was MEASURED and it lost.
#
# `similarity_order` puts near-twins in one comparison block, on the theory
# that a contradiction should be weighed against the rule it contradicts. The
# key had never been set here, so it had never been in a scored run. It was,
# on 2026-08-24, one variable against runs/qwen-27b-because on the same pinned
# binary: recall 0.55 -> 0.39, decoys 0/7 -> 2/7, reachability 10/11 -> 9/11.
# Arm runs 2 and 3 had extraction accounting identical to the incumbent, which
# isolates the loss to ordering alone.
#
# WHY, and it is worth knowing before anyone tries this again: the comparison
# stage is a GENERATIVE pass over a block of 12. Clustering near-twins starves
# it of contrast — twelve rules that all look alike is a harder discrimination
# than one quiet-hours rule among eleven unrelated ones, where the conflict
# stands out. Narrowing helps a scorer judging one pair in isolation. It hurts
# a generative pass, where the block's diversity IS the signal.
#
# `CANON_BAR_EMBED=embed` sets it. It no longer reproduces that arm on its
# own: ordering picks the BASE order, and every arrangement is a permutation
# of that base, so this now swaps the first arrangement of a union rather than
# replacing the whole comparison. The refutation above stands for what it
# tested; it says nothing about similarity ordering as a union member.
EMBED_MODEL=${CANON_BAR_EMBED-}

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
# Stamp the evidence with the build that produced it. A quality number that
# cannot say which commit it describes cannot be compared with anything.
mkdir -p "$OUT"
{
  echo "commit    $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  echo "dirty     $(git -C "$ROOT" status --porcelain 2>/dev/null | wc -l | tr -d ' ') uncommitted file(s)"
  echo "document  $DOC"
  echo "profile   $PROFILE"
  echo "model     $MODEL"
  echo "ordering  ${EMBED_MODEL:-document order (no embed_model)}"
  echo "endpoint  $ENDPOINT"
  echo "binary    $BIN"
} > "$OUT/BUILD.txt"
echo

i=1
while [ "$i" -le "$RUNS" ]; do
  SCRATCH=$(mktemp -d)
  CANON_DIR="$SCRATCH/.canon"
  export CANON_DIR CANON_ACTOR="bench:draft-bar"
  # The fixture is a house charter, so the canon is a house canon: the
  # profile decides the voice extraction writes in, and a personal-profile
  # run over a charter produces "I observe quiet hours" instead of a rule.
  "$BIN" init --profile "$PROFILE" >/dev/null
  "$BIN" config set endpoint "$ENDPOINT" >/dev/null
  "$BIN" config set model "$MODEL" >/dev/null
  # An `if`, not `[ ... ] && ...`: under `set -e` a false test makes the
  # compound return 1 and kills the sweep, which is the same shape as the `cp`
  # failure the header above records costing runs 2 and 3.
  if [ -n "$EMBED_MODEL" ]; then
    "$BIN" config set embed_model "$EMBED_MODEL" >/dev/null
  fi

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
  # An unmatched glob is a literal path here, and `cp` failing under `set -e`
  # aborted the whole sweep — so one failed run cost the two behind it, which
  # is precisely what the comment above says this loop avoids. Observed on a
  # Des Moines sweep: run 1 died in the comparison stage and runs 2 and 3
  # never started, leaving nothing to score.
  for f in "$CANON_DIR/draft-runs/"*.json; do
    [ -e "$f" ] || { echo "  (no artifact: this run produced nothing)"; break; }
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
