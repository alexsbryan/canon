#!/bin/sh
# SPDX-License-Identifier: AGPL-3.0-or-later
# Re-download the council documents this corpus is built from, and re-extract
# their text. Checksums are in ../PROVENANCE.md; a mismatch means the city
# republished a file and the corpus must be rebuilt, not silently trusted.
set -eu
cd "$(dirname "$0")"
base=https://councildocs.dsm.city/ordinances
fetch() { curl -fsSL --max-time 60 -o "$2" "$1"; echo "  $2  $(wc -c <"$2" | tr -d ' ') bytes"; }
fetch "$base/14746.pdf"  ord-14746.pdf
fetch "$base/16,064.pdf" ord-16064.pdf
fetch "$base/16,127.pdf" ord-16127.pdf
fetch "https://www.nonoise.org/regulation/ordinance/Des%20Moines,%20Iowa.pdf" code-article-iv.pdf
for f in *.pdf; do pdftotext -layout "$f" "${f%.pdf}.txt"; done
shasum -a 256 *.pdf
