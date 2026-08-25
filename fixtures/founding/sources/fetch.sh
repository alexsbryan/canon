#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Re-download the founding transcripts this corpus is built from.
#
#   sh fixtures/founding/sources/fetch.sh && python3 fixtures/founding/build.py
#
# The HTML is vendored so a standalone `git clone` of canon can rebuild the
# corpus with no network, which is the property this repository is built
# around. This script exists so the vendoring can be AUDITED: re-run it and
# `git diff` is either empty or a change the publisher made.
set -eu
cd "$(dirname "$0")"
fetch() { curl -fsS -m 60 -o "$2" "$1" && echo "  $2  $(wc -c < "$2") bytes"; }

echo "National Archives — founding documents"
for d in declaration-transcript constitution-transcript bill-of-rights-transcript amendments-11-27; do
  fetch "https://www.archives.gov/founding-docs/$d" "$d.html"
done
echo "Avalon Project, Yale Law School"
fetch "https://avalon.law.yale.edu/18th_century/artconf.asp" "articles-of-confederation.html"

echo
echo "sha256:"
shasum -a 256 ./*.html 2>/dev/null || sha256sum ./*.html
