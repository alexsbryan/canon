#!/usr/bin/env bash
# release-notes.sh — print one release's section of CHANGELOG.md.
#
#   ./scripts/release-notes.sh            what is waiting under Unreleased
#   ./scripts/release-notes.sh 0.2.0      the notes a v0.2.0 tag publishes
#
# release.yml runs this twice: once before building, so a tag whose version
# has no section fails before it spends a build, and once to publish the
# section as the GitHub Release's notes. A missing or empty section exits 1;
# a release with no notes is the ritual skipped, not a release with nothing
# to say.
set -euo pipefail
cd "$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

version="${1:-Unreleased}"
notes="$(awk -v v="$version" '
  /^## / { inside = ($2 == v) ; if (inside) { found = 1 ; next } }
  inside
  END { exit found ? 0 : 1 }
' CHANGELOG.md)" || {
  echo "CHANGELOG.md has no \`## $version\` section — rename \`## Unreleased\` to it before tagging" >&2
  exit 1
}
# Trim the blank lines either side of the section.
notes="$(printf '%s\n' "$notes" | sed -e '/./,$!d' | sed -e ':a' -e '/^\n*$/{$d;N;ba' -e '}')"
if [ -z "$notes" ]; then
  echo "CHANGELOG.md's \`## $version\` section is empty" >&2
  exit 1
fi
printf '%s\n' "$notes"
