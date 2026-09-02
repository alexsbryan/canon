#!/usr/bin/env bash
# install-git-hooks.sh — point this clone's hooks at the version-controlled
# .githooks/ directory.
#
# Why core.hooksPath rather than copying files into .git/hooks/: hooks under
# .git/ are per-clone, invisible to review, and drift silently between
# machines. This repository treats the pre-push gate as the primary
# correctness gate, so it needs to be a reviewed, shared artifact — one
# `git pull` updates the gate for everyone.
#
# Idempotent. Safe to re-run.
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if ! git rev-parse --git-dir >/dev/null 2>&1; then
    echo "install-git-hooks: not inside a git repository — nothing to do" >&2
    exit 0
fi

current="$(git config --local --get core.hooksPath || true)"

# Respect someone who has deliberately pointed hooksPath somewhere else (a
# personal hook manager, a monorepo wrapper). Tell them what they are missing
# rather than silently overwriting their setup.
if [[ -n "$current" && "$current" != ".githooks" ]]; then
    echo "install-git-hooks: core.hooksPath is already '$current' — leaving it alone." >&2
    echo "                   To adopt the repository's shared gate, run:" >&2
    echo "                     git config core.hooksPath .githooks" >&2
    exit 0
fi

chmod +x .githooks/* scripts/pre-push.sh scripts/docs-gate.sh 2>/dev/null || true
git config core.hooksPath .githooks

echo "install-git-hooks: core.hooksPath -> .githooks"
echo
echo "  The pre-push gate now runs on every push: rustfmt, clippy, the whole"
echo "  suite, and a link check over the documents. About ten seconds."
echo
echo "  Run it by hand any time:  ./scripts/pre-push.sh"
