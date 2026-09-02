#!/usr/bin/env bash
# pre-push.sh — the gate, run before code leaves your machine.
#
# ## It runs everything, every time
#
# There is no scoping here, no "only if .rs changed", and no tier that gets
# skipped on a doc-only push. The whole set costs about ten seconds warm.
#
# That is worth saying out loud because it is the luxury of a small tree, and
# it will not survive growth. A gate with a budget has to decide what to drop
# and then gets it wrong occasionally; a gate that runs in ten seconds never
# has to decide. If this script ever creeps past a minute, the answer is to
# make the suite faster or to scope it — not to let people start reaching for
# --no-verify, because a gate that is routinely bypassed protects nothing.
#
# ## It is the primary gate; CI confirms it
#
# CI runs this identical set on a clean checkout. That matters for two things
# a local run cannot do — catching what your machine had lying around, and
# gating contributions from machines nobody controls — but by the time CI
# speaks, the code has already left. This is the one that stops it.
#
# Installed by scripts/install-git-hooks.sh, which points core.hooksPath at
# .githooks/ so the gate is a reviewed file in the tree rather than something
# each person copies into their own .git/hooks/. Runnable by hand:
#
#   ./scripts/pre-push.sh
#
set -uo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

start=$SECONDS
failed=()

# Run one gate, keep going on failure, and report every failure at the end.
# Stopping at the first one turns a single push into four round trips.
gate() {
    local name="$1"; shift
    printf '  %-28s' "$name"
    local out
    if out=$("$@" 2>&1); then
        printf 'ok\n'
    else
        printf 'FAILED\n'
        failed+=("$name")
        printf '%s\n' "$out" | sed 's/^/      /'
    fi
}

echo "pre-push:"
gate "rustfmt"          cargo fmt --all --check
gate "clippy"           cargo clippy --workspace --all-targets -- -D warnings
gate "tests"            cargo test --workspace
gate "docs links"       ./scripts/docs-gate.sh

elapsed=$(( SECONDS - start ))

if [ ${#failed[@]} -ne 0 ]; then
    echo
    echo "pre-push: ${#failed[@]} gate(s) failed in ${elapsed}s — ${failed[*]}"
    echo
    echo "  rustfmt      cargo fmt --all"
    echo "  clippy       cargo clippy --workspace --all-targets --fix"
    echo "  tests        cargo test --workspace -- --nocapture"
    echo "  docs links   ./scripts/docs-gate.sh   (names the file and line)"
    echo
    echo "  Genuinely stuck and need the push? git push --no-verify — then say"
    echo "  so on the PR, so a red CI run is expected rather than a surprise."
    exit 1
fi

echo "pre-push: green in ${elapsed}s"
