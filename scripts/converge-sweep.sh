#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# The convergence arm: does reading each passage N times on the FAST slot
# recover what one reading on the primary recovers?
#
#   ./scripts/converge-sweep.sh [runs] [samples]      default 3 5
#
# ── THE PRE-REGISTRATION ────────────────────────────────────────────────
#
# Written before the arm produced a number, which is the only order in which
# a bar means anything. The incumbent is NOT hardcoded here because it comes
# from the baseline sweep this arm is provisional on — what is fixed in
# advance is the RULE, not the figure.
#
# METRIC. Anchor reachability: of the 11 planted tensions, how many had the
# load-bearing clause of BOTH sides survive extraction. Scored by
# `extraction_coverage` in tests/draft_bar.rs — the same scorer, over the
# same anchors, that scores the incumbent. This arm does not touch tension
# recall: it stops at extraction and `governance_bar` refuses its artifacts.
#
# THE INCUMBENT is the worst-run reachability of the 27B single-reading
# baseline in fixtures/maple-house/runs/qwen-27b-because, scored the same way.
#
# WIN, and the arm is worth carrying further, requires BOTH:
#   1. some k in 1..N reaches the incumbent's worst-run reachability, and
#   2. at that k the surviving candidate count per reading is no more than
#      the incumbent's. Reachability bought by keeping everything is not
#      convergence, it is over-extraction with a fold that never fires — and
#      it lands downstream as decoys in the comparison stage.
#
# KILL, and the fast slot does not read this corpus: no k reaches the
# incumbent. Report the curve and stop; do not tune the prompt and re-run,
# which is fitting the arm to the bar.
#
# NOT MEASURED, and not to be claimed: tension recall and precision, decoy
# behaviour, and any corpus but this one. A win here earns ONE confirmatory
# full-pipeline run at the winning k, scored by `governance_bar`.
#
# ── the instrument ──────────────────────────────────────────────────────
#
# The fold is deterministic code (`draft::converge`), never a model call: a
# stochastic step inside the fold means no point on the curve can be
# attributed to k (§18.4). Each k is scored by REPLAY off the same readings
# — `canon draft --refold` re-folds an artifact without an endpoint — so the
# curve costs N readings once, not N readings per k.
#
# DO NOT REBUILD WHILE THIS RUNS. Set CANON_BAR_BIN to a copy; every run on
# one curve must come from one build or the artifacts are not one instrument.
set -eu

RUNS=${1:-3}
SAMPLES=${2:-5}
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
OUT=${CANON_CONVERGE_OUT:-"$ROOT/fixtures/maple-house/runs/converge-fast-x$SAMPLES"}
DOC=${CANON_BAR_DOC:-"$ROOT/fixtures/maple-house/maple-house.md"}
PROFILE=${CANON_BAR_PROFILE:-house}
ENDPOINT=${CANON_ENDPOINT:-http://localhost:9741/v1}
# The rest of the pipeline never runs in this arm, so `model` only names what
# the artifact records. EXTRACT_MODEL is the slot under test.
MODEL=${CANON_MODEL:-primary}
EXTRACT_MODEL=${CANON_EXTRACT_MODEL:-fast}
BIN=${CANON_BAR_BIN:-"$ROOT/target/debug/canon"}

mkdir -p "$OUT/raw"
{
  echo "commit         $(git -C "$ROOT" rev-parse --short HEAD 2>/dev/null || echo unknown)"
  echo "dirty          $(git -C "$ROOT" status --porcelain 2>/dev/null | wc -l | tr -d ' ') uncommitted file(s)"
  echo "document       $DOC"
  echo "profile        $PROFILE"
  echo "extract model  $EXTRACT_MODEL   <- the slot under test"
  echo "other legs     $MODEL (never called: this arm stops at extract)"
  echo "endpoint       $ENDPOINT"
  echo "binary         $BIN"
  echo "readings       $SAMPLES per passage, $RUNS run(s)"
} > "$OUT/BUILD.txt"
cat "$OUT/BUILD.txt"
echo

i=1
while [ "$i" -le "$RUNS" ]; do
  SCRATCH=$(mktemp -d)
  CANON_DIR="$SCRATCH/.canon"
  export CANON_DIR CANON_ACTOR="bench:converge"
  "$BIN" init --profile "$PROFILE" >/dev/null
  "$BIN" config set endpoint "$ENDPOINT" >/dev/null
  "$BIN" config set model "$MODEL" >/dev/null
  "$BIN" config set extract_model "$EXTRACT_MODEL" >/dev/null

  echo "--- run $i/$RUNS ---"
  START=$(date +%s)
  if "$BIN" draft --dry-run --samples "$SAMPLES" --from "$DOC" >/dev/null; then
    echo "  $(( $(date +%s) - START ))s"
  else
    echo "  FAILED after $(( $(date +%s) - START ))s"
  fi
  for f in "$CANON_DIR/draft-runs/"*.json; do
    [ -e "$f" ] || { echo "  (no artifact: this run produced nothing)"; break; }
    cp "$f" "$OUT/raw/run-$(basename "$f")"
  done
  rm -rf "$SCRATCH"
  i=$((i + 1))
done

echo
echo "refolding $(ls "$OUT/raw" | wc -l | tr -d ' ') reading-set(s) at every k"
k=1
while [ "$k" -le "$SAMPLES" ]; do
  mkdir -p "$OUT/k$k"
  "$BIN" draft --refold "$OUT/raw" --k "$k" --out "$OUT/k$k"
  k=$((k + 1))
done

echo
echo "score each k (the incumbent is the baseline sweep, scored the same way):"
k=1
while [ "$k" -le "$SAMPLES" ]; do
  echo "  CANON_BAR_RUNS=$OUT/k$k cargo test --test draft_bar extraction_coverage -- --ignored --nocapture"
  k=$((k + 1))
done
