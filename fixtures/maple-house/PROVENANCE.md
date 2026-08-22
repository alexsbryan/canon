# Maple House — where this fixture came from

`maple-house.md` and `truth.json` were written for the governance-tension
bench in the Commonwealth monorepo and are vendored here so `canon`'s
published numbers are reproducible from this repository alone.

| | |
|---|---|
| Upstream path | `sovereign-recipes/maple-house/` in `commonwealth-ai` |
| Last upstream change to these two files | `4d96eeaf5a2b2779f349a50b717ee1a814f335ab` (2026-06-18) |
| Copied at upstream HEAD | `fc02ebf1` |
| `maple-house.md` | sha256 `2d0c374965f6864b51cdd7557b76bd497bec7e8c4b754180bbdeff9d64aed56b` |
| `truth.json` | sha256 `2422496003341a7bb3c32b1f4d16e825c37298aac288916b476a2c863444151f` |

Copied, not depended on. A path dependency across repositories would break a
standalone `git clone` of `canon`, which is the property the whole tool is
built around. If upstream revises the manifest, re-copy and update this file —
the two must not drift silently, because a number scored against a drifted
truth is a number about nothing.

## What is in it

A fictional twelve-person co-op: eleven charter articles and thirteen dated
house decisions, written so that the conflicts between them are known in
advance. `truth.json` labels every one.

- **11 planted tensions** across four types — `direct_contradiction`,
  `unmarked_supersession`, `charter_conflict`, `scope_overlap`.
- **7 expected non-tensions** — pairs that share vocabulary, or refine each
  other, and are NOT in conflict. `D1` is an outright decoy: guests and
  overnight in both, one about parking.
- **Splits** — `train` (tunable), `dev` (tunable), `test` (sacred).

Pairs are keyed by section: a charter article numeral, or a decision date.
Both are unique within the document.

## `extraction-anchors.json` was written here, not vendored

`maple-house.md` and `truth.json` come from upstream. The anchors file does
not: it is this repository's own instrument, and it answers a question
`truth.json` does not ask.

`truth.json` labels which SECTION PAIRS are in tension. That scores the
comparison step and says nothing about the step before it. If the clause a
tension turns on never survives extraction, no amount of comparison can find
it, and a recall number computed over that candidate set is a statement
about extraction wearing a comparison's clothes.

So each anchor names the smallest phrase a tension depends on, taken from
`truth.json`'s own `why` field and the source passage. `T5` reads *"Charter
sets quiet hours at 11 PM; the decision silently moves the weeknight start to
10 PM"* — so Article II must yield a rule containing `11:00 pm` and the
2026-02-10 decision one containing `10:00 pm`, or T5 is unreachable before
the comparison starts. Which is exactly what was happening.

The `fidelity` section is the other half: measures the source states that a
rule must not silently change. It exists because a candidate read *"at least
three hours in advance"* while its own verbatim quote said *"three days
ahead"* — the citation check passed it, because a citation proves the quote
is real and not that the rule matches it.

Both were written before the extraction prompt was touched.

## Not the corpus

`recipe.toml` is deliberately not vendored. It configures an ingest pipeline
this tool does not have and would only invite someone to try running it.
