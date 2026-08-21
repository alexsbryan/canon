# The canon act log — format specification

**This specification is released into the public domain (CC0 1.0).**
Implement it freely, on either side, with no obligation to this project.
The reference implementation (`canon-core`) is AGPL-3.0-or-later; the
format is not.

The point of that split: adopting this format must not be a lock-in
decision. A record you cannot leave is not a record you own.

Version: **1**. Status: draft, pre-1.0. Breaking changes bump `v`.

## The file

One file, `.canon/acts.jsonl`. Append-only. One JSON object per line,
UTF-8, newline-terminated. Blank lines are skipped.

## The envelope

Every line carries four envelope fields plus a **flattened** act body, so
`op` sits beside `actor` rather than nested under it.

| Field | Type | Meaning |
|---|---|---|
| `id` | string | Content-addressed act id (below) |
| `v` | integer | Format version. Always written |
| `ts_unix` | integer | When the act happened, Unix **seconds** |
| `actor` | string | Who performed it. `human:<name>` for a person |
| `op` | string | Discriminates the act body |

```json
{"id":"can-4c1a9f0e2b71","v":1,"ts_unix":1787341438,"actor":"human:dana","op":"assert","text":"Quiet hours run 11pm to 7am."}
```

A reader encountering a `v` **greater** than it understands MUST refuse
that line rather than interpreting it partially. Silent partial reads
corrupt the derived state in ways nothing downstream can detect.

## Ids

`can-` followed by the first 12 hex characters of
`SHA-256("can" | "|" | ts_unix | "|" | actor | "|" | body)`, where `body`
is the act body serialized as JSON in field-declaration order.

Two consequences the format depends on:

- **Union merge is exact.** The same act written on two machines yields
  the same id, so deduplication is equality rather than heuristics.
- **Replay is stable.** Ids do not encode position, so appending,
  sorting and merging never renumber anything.

Two byte-identical acts by the same actor in the same second collide by
design. Appending happens in real time, so this does not arise in
practice; it is a documented bound, not an accident.

## The acts

A **commitment** is introduced by `assert` or `supersede`. Its identity is
the id of the act that introduced it.

| `op` | Fields | Meaning |
|---|---|---|
| `assert` | `text`, `from?`, `source?` | A commitment enters the canon |
| `supersede` | `text`, `old[]`, `rationale?` | Replaces one or more commitments |
| `retract` | `target`, `rationale?` | Withdraws one, no replacement |
| `accept` | `a`, `b`, `rationale`, `revisit?` | A contradiction carried knowingly |
| `dismiss` | `a`, `b`, `rationale?` | Not actually a conflict |
| `revert` | `targets[]`, `rationale?` | Tomb-stones prior acts |
| `adopt` | `lineage`, `generation`, `source?` | Forked from a lineage |

`accept.rationale` is **required**: a tolerated contradiction must say
what it protects. Every other rationale is optional, and `dismiss` is
deliberately light ceremony — rejecting detector noise is routine.

`adopt` is an **act**, not repository metadata, so ancestry survives a
file that arrives by paste with no version control attached.

## Deriving current state

Implementations MUST produce identical state for the same set of acts
regardless of the order they arrive in. Three rules:

**1. Liveness resolves by reference, not by position.** An act is dead
iff some *live* `revert` targets it; a `revert` cancelled by another live
`revert` has no effect, so reverting a revert re-applies the originals.
Resolving this by walking a sorted list is incorrect — acts routinely
share a second, and an id tiebreak can order a `revert` ahead of the act
it cancels.

**2. Introduce before applying.** Collect every commitment from `assert`
and `supersede` first; only then apply status effects. Same reason.

**3. Report dangling references.** An act naming a commitment absent from
the log is a hole in the record — a truncated file, a hand edit, a
snapshot adopted without its history. Surface it. Do not treat it as a
no-op.

Resulting statuses: `active`, `superseded{by}`, `retracted{at}`.

## Canonical ordering

For storage and rendering, sort by `(ts_unix, id)` and deduplicate by
`id`. This makes two machines render byte-identical files after a merge.
Within a single second the resulting order is arbitrary to a reader; that
is accepted, because the derivation above does not depend on it.

## Attribution

Adjudications — everything except `assert` and `adopt` — are expected to
carry an actor beginning `human:`. Implementations MUST surface acts that
do not, rather than dropping or rejecting them. Extraction and drafting
are machine work and are not flagged.

This is how *agents propose, humans dispose* becomes a property of the
record rather than a rule someone has to remember.

## Merging

Union both sides, deduplicate by `id`, sort canonically. Under git:

```
# .gitattributes
acts.jsonl merge=canon
```

## Relationship to the Commonwealth governance oplog

The **envelope is shared** — `id`, `v`, `ts_unix`, `actor`, flattened
body tagged on `op`, content-addressed ids with a tenancy prefix. The
**act vocabulary differs**: a canon commitment carries its text inline,
while a governance rule references an extracted atom in a corpus atlas.

Interoperation is therefore a documented mapping, not identity. Saying so
plainly is better than implying a compatibility that does not hold.
