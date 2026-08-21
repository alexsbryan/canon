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

## Not the corpus

`recipe.toml` is deliberately not vendored. It configures an ingest pipeline
this tool does not have and would only invite someone to try running it.
