#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Score a corpus's draft runs against ITS OWN manifest.
#
#   ./scripts/score-bar.sh <corpus> [runs-dir]
#   ./scripts/score-bar.sh maple-house
#   ./scripts/score-bar.sh des-moines-noise /tmp/sweep/qwen-27b
#
# The manifest, the anchors and the runs all come from one corpus because
# this script derives them from one name. Scoring one corpus's runs against
# another's truth produces a number about nothing, and nothing downstream
# would catch it (ARCH_PRINCIPLES §10.6, §18.3).
set -eu

CORPUS=${1:-}
[ -n "$CORPUS" ] || { echo "usage: $0 <corpus> [runs-dir]"; exit 2; }

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DIR="$ROOT/fixtures/$CORPUS"
[ -d "$DIR" ] || {
  echo "no corpus '$CORPUS'. Available:"
  for d in "$ROOT"/fixtures/*/; do [ -f "$d/truth.json" ] && echo "  $(basename "$d")"; done
  exit 2
}

RUNS=${2:-}
if [ -z "$RUNS" ]; then
  # One directory is one instrument; if there are several, say so rather than
  # picking one and reporting a number the caller did not choose.
  found=$(find "$DIR/runs" -mindepth 1 -maxdepth 1 -type d 2>/dev/null | sort)
  n=$(printf '%s\n' "$found" | grep -c . || true)
  [ "$n" -eq 1 ] || { echo "$n run directories under $DIR/runs — name one:"; printf '  %s\n' $found; exit 2; }
  RUNS=$found
fi

echo "corpus  $CORPUS"
echo "runs    $RUNS"
echo
CANON_BAR_TRUTH="$DIR/truth.json" \
CANON_BAR_ANCHORS="$DIR/extraction-anchors.json" \
CANON_BAR_RUNS="$RUNS" \
  cargo test --manifest-path "$ROOT/Cargo.toml" --test draft_bar -- --ignored --nocapture
