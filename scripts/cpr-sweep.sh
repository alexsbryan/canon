#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# The CPR transfer study, end to end. No endpoint, no model, no network.
#
#   ./scripts/cpr-sweep.sh
#
# Three questions, in order:
#   1. does every institution replay as its fixture predicted?
#   2. what does the same counterfactual do to each of them?
#   3. what did adding an institution actually cost?
set -uo pipefail
cd "$(dirname "$0")/.."
CANON=${CANON:-./target/debug/canon}
[ -x "$CANON" ] || { echo "build first: cargo build" >&2; exit 1; }

dirs=(); for d in fixtures/cpr/*/; do [ -f "$d/vocab.json" ] && dirs+=("$d"); done
resources=(); ablations=()
for d in "${dirs[@]}"; do
  if grep -q '"predicted_to_fail"' "$d/vocab.json"; then ablations+=("$d"); else resources+=("$d"); fi
done

echo "── 1. replay ──────────────────────────────────────────────"
fail=0
for d in "${dirs[@]}"; do
  line=$("$CANON" replay "$d" 2>&1 | tail -1)
  printf '  %-28s %s\n' "$(basename "$d")" "$line"
  [[ "$line" == *"all as expected"* ]] || fail=1
done

echo
echo "── 2. the counterfactual ──────────────────────────────────"
echo "  What a different rule would have done. A signature is the SET of"
echo "  (step, field) pairs that came out differently — so one signature"
echo "  across every institution means the divergence is the policy's and"
echo "  not the domain's."
for pol in default consent subsidiarity; do
  sigs=$(for d in "${resources[@]}"; do
    "$CANON" replay "$d" --policy "$pol" 2>&1 >/dev/null \
      | grep -oE '^[a-z0-9-]+\.[a-z]+' | sort | shasum | cut -c1-12
  done | sort -u)
  # Decisions, not fields: `outcome`, `authority`, `because` and `rule` all
  # move together for one decision, and counting them separately doubles it.
  n=$("$CANON" replay "${resources[0]}" --policy "$pol" --brief 2>/dev/null \
        | grep -oE '[0-9]+ of [0-9]+ decision' | head -1)
  count=$(echo "$sigs" | wc -l | tr -d ' ')
  printf '  --policy %-13s %-14s change  %s signature(s) over %s institutions\n' \
    "$pol" "$n" "$count" "${#resources[@]}"
  [ "$count" = 1 ] || fail=1
done

echo
echo "── 3. what an institution cost ────────────────────────────"
spine=$(cat fixtures/cpr/_spine/*.tmpl | grep -cvE '^\s*(//.*)?$')
printf '  shared spine (mechanism, written once)  %4s lines\n' "$spine"
total=0
for d in "${resources[@]}" "${ablations[@]}"; do
  n=$(grep -cvE '^\s*$' "$d/vocab.json"); total=$((total+n))
  printf '  %-38s %4s lines of vocabulary\n' "$(basename "$d")" "$n"
done
printf '  %-38s %4s lines, and no mechanism\n' "TOTAL, ${#dirs[@]} institutions" "$total"

echo
[ $fail -eq 0 ] && echo "the study clears." || echo "the study did NOT clear."
exit $fail
