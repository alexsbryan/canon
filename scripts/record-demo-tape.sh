#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Cut a tape for act 1 of the demo: every reply the endpoint gives, recorded,
# so the ingest can be shown as real output with no live risk.
#
#   ./scripts/record-demo-tape.sh
#
# A TAPE IS NOT A MOCK. It is what a real model actually said, and the replay
# runs the same pipeline over it — citation cutting, the guards, the fold. It
# refuses if this build asks for a call the recording does not have, so a tape
# cut against an older binary fails loudly instead of answering from the wrong
# recording. Re-cut it whenever the call sequence changes.
set -euo pipefail
cd "$(dirname "$0")/.."
CANON=${CANON:-./target/debug/canon}
OUT=fixtures/maple-house/runs/demo-tape
DOC=fixtures/maple-house/maple-house.md
SCRATCH=$(mktemp -d)
trap 'rm -rf "$SCRATCH"' EXIT

mkdir -p "$SCRATCH/.canon"
CANON_DIR="$SCRATCH/.canon" $CANON init --profile house >/dev/null
echo "recording against the configured endpoint — this pays for the run once"
CANON_DIR="$SCRATCH/.canon" $CANON draft --dry-run --from "$DOC" >/dev/null

src=$(ls -t "$SCRATCH"/.canon/draft-runs/*.json | head -1)
mkdir -p "$OUT"
cp "$src" "$OUT/run.json"
python3 - "$OUT/run.json" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
print(f"  tape       {len(d.get('tape', []))} recorded call(s)")
print(f"  model      {d.get('served_model') or d.get('model')}")
print(f"  candidates {len(d.get('candidates', []))}")
print(f"  written to {sys.argv[1]}")
PY
