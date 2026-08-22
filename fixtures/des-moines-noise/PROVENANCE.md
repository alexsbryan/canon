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

What is *not* ours: the text, the dates, and the pairing. The council said
which section each ordinance amends. We did not choose which rules relate to
which.

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

## The labelling rule

Applied by `build.py`, by nothing else, and auditable by re-reading it:

- A permit type stated in **both** the codified article and an amending
  ordinance, whose restatement **changes any measure** it states — sound level
  or its weighting, distance, hours, counts, days — is a **planted tension**
  (`unmarked_supersession`).
- Whose measures are **all identical**, and whose wording matches but for
  typography, is an **expected non-tension**: the ordinance re-enacted it.
- Stated only by an ordinance: an **addition**, not paired.
- A base section no ordinance here amends: an **expected non-tension**, paired
  for its heavy shared vocabulary.

**Where the rule refuses to vote.** Type "J" (night construction) states no
measure this script can read, and its wording changed. "No measure changed" is
then a vacuous test rather than a finding, so Type J is **excluded from the
manifest** rather than labelled — a check that cannot see a change must not
report that there was none (`ARCH_PRINCIPLES` §18.3). `build.py` prints it on
every run.

`build.py` also asserts that every key in the manifest resolves to exactly one
heading in the document. A manifest pointing at a section that does not exist
scores nothing and says so nowhere (§18.1).

## What this corpus is weak at

- **Six of the nine tensions are the same pattern** — a level restated in
  dB(C) where the article said dB(A), the number unchanged. That is a real
  substantive change, since the weighting curves differ, but it means the
  corpus tests one narrow discrimination six times. Types F, G and I are the
  varied ones (level *and* distance, level *and* days, level *and* both).
- **It is 3.8× the size of Maple House** (38KB, 33 sections) and past the
  ~60-commitment ceiling the spec names for `tensions`. Expect the comparison
  stage to cost substantially more than one call per block.
- **Register.** This is drafted legal prose, not a household charter. Poor
  extraction here may be register rather than capability — which is worth
  knowing, since an ordinance is a document real people actually hold.

## Holdout status

**Reachability read 2026-08-22; recall and precision not yet read.** The
first scored run reported 3 of 9 tensions reachable, and the per-tension
breakdown names individual pairs across all three splits — T3 (test) and T8
(dev) among them. That is inspection, and it is recorded here rather than
left unsaid.

What it does NOT license: the mechanism those pairs failed by is visible in
T1 and T4, both `train`, and any fix must be justified from those. The
diagnosis is mechanical and identical across the six — a dB(C) reading
folded into its dB(A) predecessor because `measure.rs` reads no decibel unit
— so nothing about the `test` pairs informs it. `test`-split
pairs are sacred: read the aggregate, never the instance. The first time a
test-split miss is opened, this file must say so — Maple House's numbers are
labelled train-contaminated for exactly that reason, and the whole value of a
second corpus is that it starts clean.
