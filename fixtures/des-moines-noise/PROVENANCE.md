# Des Moines noise control — where this corpus came from

Article IV (Noise Control) of the Des Moines, Iowa municipal code, interleaved
with two ordinances that later amended it. Every word is the city council's.

| Document | Source | Adopted | sha256 (PDF) |
|---|---|---|---|
| Ordinance 14,746 — enacts Art. IV | `councildocs.dsm.city/ordinances/14746.pdf` | 2008-02-25 (Roll Call 08-329) | `e51a4999…a221a3c57` |
| Article IV as codified | `nonoise.org/regulation/ordinance/Des Moines, Iowa.pdf` | — | `cea206d6…cb19eb96c` |
| Ordinance 16,064 — amends §§ 42-246, 42-258 | `councildocs.dsm.city/ordinances/16,064.pdf` | 2021-10-18 (Roll Call 21-1620) | `cf8841a6…666ee79e8` |
| Ordinance 16,127 — amends §§ 42-246, 42-258 | `councildocs.dsm.city/ordinances/16,127.pdf` | 2022-05-23 (Roll Call 22-0805) | `6aedbd39…f548da940` |

Retrieved 2026-08-22. `sources/fetch.sh` re-downloads and re-extracts;
`build.py` rebuilds `des-moines-noise.md` and `truth.json` from the vendored
text with no network and no model.

**Licence.** Municipal ordinances are edicts of government and carry no
copyright. The PDFs are vendored so a standalone `git clone` of `canon` can
rebuild the corpus, which is the property this repository is built around.

## What was constructed, and what was not

The city never published one document containing both readings. Interleaving
the codified article with the ordinances that amend it is **our construction**,
and it is the point: it reproduces the shape a real body of commitments takes
when a decision changes a rule and nobody goes back to strike the old one.

What is *not* ours: the text and the dates.

The **pairing is ours in one respect**, and this file said otherwise until
2026-08-22. The council numbers its permits `(1)`…`(17)` and lettered `A`…`Q`,
and where a letter keeps its subject the pairing is the council's. But
Ordinance 16,064 *splits* two of them, and no line in it says so. The codified
Type "F" covers the Simon Estes Riverfront Amphitheater **and** the Brenton
Skating Plaza; the ordinance keeps Brenton under "F" and re-enacts Simon Estes
as a new Type "N". The codified Type "I" covers Waterworks Park; the ordinance
addresses the Lauridsen Amphitheater as "I" and the park field as "M". Reading
those pairs off the subject lines is **our reading of the council's words**,
recorded in the `SUBJECTS` map at the top of `build.py` with each entry quoting
the line it came from. It is 27 lines and it is meant to be read.

## Tables were re-rendered, and only their association was restored

`pdftotext -layout` flattens a table into space-separated words and loses
which row a value belongs to. Table 1 emerged with `60` alone on one line and
`Residential zones:` on the next, so the limit had no readable subject. Tables
2 and 3 emerged as `90 24 hours`, which is not a sentence and cannot be cited
— the first run against the flattened text lost 11 candidates to "quote too
short to be evidence", every one of them a table row.

Four tables are therefore re-rendered as markdown by `build.py`: Table 1
(sound levels by receiving land use), Tables 2 and 3 (levels posing an
immediate threat), and the vehicle table in Sec. 42-259. **Every cell is the
council's own text.** Only the row-and-column association is restored, read
off the word bounding boxes `pdftotext -tsv` reports, and it is auditable:

```sh
pdftotext -tsv sources/code-article-iv.pdf - | awk -F'\t' '{print $2, $7, $8, $12}'
```

Table 1's label column is a vertically centred merged cell spanning its two
value rows, which is why the residential label repeats in the rendering.

One more artifact is repaired: a section number wrapped mid-token
(`section 42-` / `257`) normalises to `42- 257`, and no quote containing it
could ever match its own passage. `section 42- 258 (e)` is left as it stands —
that space is in the ordinance itself.

## Where a subsection ends

`permit_blocks` used to end a permit at `pos + 2200` characters, which is not
a boundary any document has, and two of the twenty-seven blocks were wrong:

- The codified Type **"J"** swallowed subsections `(f) Commercial advertising`
  and `(h) Denial or revocation` whole — 1,975 characters where the permit is
  740 — so rules about advertising and permit revocation were filed under a
  night-construction permit, and the two readings of "J" no longer matched
  each other, which cost that pair its label.
- Type **"Q"** of Ordinance 16,127 swallowed the clerk's certification and was
  then cut mid-sentence.
- Type **"G"** cleared the cap by 58 characters and was correct by luck.

A block now ends where the document says it does: at the next permit, at a
sibling of the lettered subsection holding the permit list, at the enacting
`Section N.` an ordinance closes with, at its signature block, or at the next
section of the code. There is no character cap. `build.py` then **asserts**
that no emitted section contains a marker belonging to another one, so this
cannot come back quietly.

One artifact was hiding the boundary: `pdftotext` writes a form feed at the
head of each page's first line, and `(11) Type "K" permit` began with one, so
no line-anchored pattern could see it. `load()` strips form feeds now, next to
the `Page N` lines it already stripped.

## The labelling rule

Applied by `build.py`, by nothing else, and auditable by re-reading it. **The
unit is the SUBJECT, not the type letter** — see the split above:

- A permit **subject** stated in **both** the codified article and an amending
  ordinance, whose restatement **changes any measure** it states — sound level
  or its weighting, distance, hours, counts, days — is a **planted tension**
  (`unmarked_supersession`).
- Whose measures are **all identical**, and whose wording matches but for
  typography, is an **expected non-tension**: the ordinance re-enacted it.
- A subject stated only by an ordinance: an **addition**, not paired.
- A base section no ordinance here amends: an **expected non-tension**, paired
  for its heavy shared vocabulary.

Pairing by letter cost the manifest two tensions — the F→N and I→M splits —
and both are the corpus's own `unmarked_supersession` pattern. It also left
Type "J" unlabelled, but that was the block boundary rather than the rule.

`build.py` asserts that every key in the manifest resolves to exactly one
heading in the document. A manifest pointing at a section that does not exist
scores nothing and says so nowhere (§18.1). It also asserts that every reading
has a declared subject, so a permit added to a source cannot slip through
unpaired.

## What the manifest is complete about

`exhaustive` is **false** for the document and that is not going to change:
nothing here labels a pair between two of the six general sections, or between
a general section and a permit. 177 of the 528 cross-section pairs are in that
condition.

`exhaustive_within` names where it **is** complete — the 27 permit
subsections, 351 pairs. Twelve are labelled above; the other 339 are
compatible by the rule that makes the region exhaustive: *a permit authorises
one venue or one kind of conduct, and two permits with different subjects have
nothing to disagree about*. That claim is only as good as the `SUBJECTS` map,
which is why the map quotes the council for every entry.

The bar divides precision by the pairs inside that region and **reports how
many proposals landed outside it**. Outside a complete region an unlabelled
proposal and a wrong one are the same observation, and dividing by them would
measure the manifest's size rather than the tool (§18.3).

## What this corpus is weak at

- **Six of the eleven tensions are the same pattern** — a level restated in
  dB(C) where the article said dB(A), the number unchanged. That is a real
  substantive change, since the weighting curves differ, but it means the
  corpus tests one narrow discrimination six times (Types A, B, C, D, E, H).
  The five varied ones are F, F→N, G, I and I→M, which move a distance, a
  clock time or a day count as well as the level.
- **It is 3.4× the size of Maple House** (35KB, 33 sections) and past the
  ~60-commitment ceiling the spec names for `tensions`. Expect the comparison
  stage to cost substantially more than one call per block.
- **Register.** This is drafted legal prose, not a household charter. Poor
  extraction here may be register rather than capability — which is worth
  knowing, since an ordinance is a document real people actually hold.

## Holdout status

**Every number read before 2026-08-22 is void, and the reason is in this
file.** The corpus those runs scored had a night-construction permit carrying
two foreign subsections, a permit carrying a clerk's certification and cut
mid-sentence, and a manifest missing two of its eleven supersessions. A number
taken against a wrong answer key is not a smaller number, it is not a number.
The runs are not kept.

What was read, and is retained only as a record of what was inspected:
reachability on the old corpus reported 3 of 9, and the per-tension breakdown
named individual pairs across all three splits — T3 (test) and T8 (dev) among
them. That is inspection, and it is recorded here rather than left unsaid.
**T-ids have since been reassigned** (the manifest went from 9 planted to 11),
so those identifiers no longer name the same pairs.

What it does NOT license: the mechanism those pairs failed by is visible in
T1 and T4, both `train`, and any fix must be justified from those. The
diagnosis is mechanical and identical across the six — a dB(C) reading
folded into its dB(A) predecessor because `measure.rs` reads no decibel unit
— so nothing about the `test` pairs informs it. `test`-split
pairs are sacred: read the aggregate, never the instance. The first time a
test-split miss is opened, this file must say so — Maple House's numbers are
labelled train-contaminated for exactly that reason, and the whole value of a
second corpus is that it starts clean.
