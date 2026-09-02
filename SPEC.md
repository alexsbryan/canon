# The canon act log — format specification

**This specification is released into the public domain (CC0 1.0).**
Implement it freely, on either side, with no obligation to this project.
The reference implementation (`canon-core`) is AGPL-3.0-or-later; the
format is not.

The point of that split: adopting this format must not be a lock-in
decision. A record you cannot leave is not a record you own.

Version: **2**. Status: draft, pre-1.0. Breaking changes bump `v`.

v2 split the `op` namespace so that a community can add a governance move
without every other implementation having to change. See "The acts" below —
the two halves have deliberately opposite rules.

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

### Structural ops — closed, and strict

These four change what is **live**. The set is closed. A reader that meets
an unknown or malformed structural op MUST refuse that line.

| `op` | Fields | Meaning |
|---|---|---|
| `assert` | `text`, `from?`, `source?` | A commitment enters the canon |
| `supersede` | `text`, `old[]`, `rationale?` | Replaces one or more commitments |
| `retract` | `target`, `rationale?` | Withdraws one, no replacement |
| `revert` | `targets[]`, `rationale?` | Tomb-stones prior acts |

Strictness here is not pedantry. A peer that silently skipped a `retract`
it could not parse would fold a commitment back into a canon its holder
had withdrawn, and nothing downstream could detect that.

### Annotations — open, and carried

Everything else is an annotation: a typed statement **about** a commitment
or a pair of them. A reader that meets an annotation `op` it does not
recognise MUST carry the line unchanged and MUST NOT interpret it.

| `op` | Fields | Meaning |
|---|---|---|
| `accept` | `a`, `b`, `rationale`, `revisit?` | A contradiction carried knowingly |
| `dismiss` | `a`, `b`, `rationale?` | Not actually a conflict |
| `question` | `text`, `proposal?` | Something the canon does not cover |
| `adopt` | `lineage`, `generation`, `source?` | Forked from a lineage |
| `position` | `about`, `citing?`, `pull`, `because` | Somebody takes a position |
| `grant` | `holder`, `scope`, `horizon?`, `rationale?` | Standing over a scope |
| `withdraw` | `holder`, `scope`, `rationale?` | Standing given up, or stood down |
| `scoped` | `commitment`, `scope` | A commitment belongs to a scope |
| `policy` | `text`, `rule`, `scope?` | What this canon decides under |
| `ratification` | `text`, `rule`, `scope?` | How a proposal in a scope becomes a rule |
| `decided` | `about`, `outcome`, `authority`, `rationale?` | The group decided something |
| `rank` | `commitment`, `rank` | A principle rather than a convention |
| `horizon` | `target`, `at`, `rationale?` | Look at this again by then |
| `draw_commit` | `scope`, `count`, `after_ts`, `rationale?` | A lot is announced |
| `draw_secret` | `commit`, `digest` | A sealed secret, before the boundary |
| `draw_reveal` | `commit`, `secret` | The secret, after it |
| `allot` | `text`, `unit`, `units[]`, `scope` | What a scope has to share |
| `allocation` | `text`, `rule`, `scope` | How that pool goes round |
| `silence` | `about`, `rationale` | Unwritten on purpose, not by neglect |

`accept.rationale` is **required**: a tolerated contradiction must say
what it protects. Every other rationale is optional, and `dismiss` is
deliberately light ceremony — rejecting detector noise is routine.

`adopt` is an **act**, not repository metadata, so ancestry survives a
file that arrives by paste with no version control attached.

`position.pull` is `toward` or `against`, and `because` is required on
both — a position whose reason a reader cannot check is an assertion.
**The actor is the act's own `actor` field and never a field in the
body.** Two places naming who did something is two answers to one
question the first time they disagree. `citing` present means the
position rests on a commitment this canon holds; absent means it is the
actor's own.

`grant.holder` and `withdraw.holder` are **not** named `actor`. The body
is flattened into the same JSON object as the envelope, which already
carries `actor` — the person doing the granting — and the two are
different people. A body field of that name emits a line with two
`actor` keys that no strict reader can parse back.

`scope` is a **dotted path** — `house.kitchen` — with no empty segments.
A scope covers itself and anything under it, and the boundary is the dot:
`house` covers `house.kitchen` and does NOT cover `household`. A reader
MUST refuse a malformed scope rather than repairing it.

`withdraw` removes grants at or below the named scope. Carving a hole out
of a broader grant is deliberately not expressible: a permission system
with both grants and denials is one where nobody can answer "may they?"
by looking. Re-grant narrower instead.

`policy.rule` is a typed object discriminated on its own `rule` field, and
`policy.text` is the same rule in prose. **Both are required and neither
substitutes for the other**: the prose is what a person reads and
contests, the typed rule is what code reads. An implementation MUST NOT
derive its behaviour from the prose, and MUST NOT show only the typed
form. Nesting is by a `base` field, so a rule may wrap another.

`horizon.at` is Unix **seconds**, and it may target any act — a
commitment, a question, a grant, an accepted contradiction. It is one act
for what people call term limits, sunset clauses, trial periods, revisit
dates and rotation, which are the same shape. The last horizon written
for a target governs, so a date can be moved as well as set.

`accept.revisit` remains a **date string**, because it shipped that way
and this format does not rewrite what is already written. A reader MUST
read both through one calendar, and MUST NOT read an unparseable revisit
as a date — reporting it as unreadable and reading it as epoch zero are
not the same thing, and the second makes it permanently overdue.

`silence.rationale` is **required**, like `accept`'s: something left
unwritten on purpose must say what that protects, or it cannot be told
apart from having been forgotten. A reader MUST match `about` exactly and
MUST NOT match by resemblance — a silence that spread would cover
subjects nobody chose to leave unwritten.

**The draw is a query, not an act.** There is no `draw` op and there must
not be one: given `draw_commit` and the verified secrets, the panel is a
pure function of the log, so every reader computes the same one and nobody
performs it. A reader MUST refuse rather than fall back when the boundary
does not postdate its own `draw_commit`, when no secret verifies, when the
scope has no pool, or when the seats would take the whole pool. The seed
is `sha256` over the commit id and every verified `(actor, secret)` in
sorted order, with `0x1e` between pairs and `0x1f` within one. The
threat model is in `PRIMITIVES.md` under Primitive 9.

`decided` records an **adjudication**, never an observation. It says the
group decided something about a subject; there is no act in this format
that records what a person was seen doing, and adding one would be a
different format.

An annotation the reader *does* recognise is read strictly, for the same
reason as a structural op: a malformed `accept` is a defect in the writer,
not a version this reader is behind.

**Carried is not ignored.** An implementation that carries an
uninterpreted annotation MUST be able to say so — how many, and of what
op — anywhere its answer could have been different had it understood
them. Silently carrying lets a canon answer as though it had read
everything when it had not.

This is what makes the format extensible without a version bump per
governance move. A community that invents `position`, `grant`, `silence`
or `draw` writes them as annotations; every other implementation keeps
reading the log, keeps merging it, and keeps rendering it byte-identically,
while declining to act on what it does not understand.

## Deriving current state

Implementations MUST produce identical state for the same set of acts
regardless of the order they arrive in. Three rules:

**1. Liveness resolves by reference, not by position.** An act is dead
iff some *live, in-seat* `revert` targets it; a `revert` cancelled by
another live `revert` has no effect, so reverting a revert re-applies the
originals. Resolving this by walking a sorted list is incorrect — acts
routinely share a second, and an id tiebreak can order a `revert` ahead
of the act it cancels. In-seat is rule 7: a `revert` of somebody else's
act by a person without standing over it is recorded and has no effect.

**2. Introduce before applying.** Collect every commitment from `assert`
and `supersede` first; only then apply status effects. Same reason.

**3. Report dangling references.** An act naming a commitment absent from
the log is a hole in the record — a truncated file, a hand edit, a
snapshot adopted without its history. Surface it. Do not treat it as a
no-op.

Resulting commitment statuses: `active`, `superseded{by}`, `retracted{at}`,
`proposed{needs}`, `refused{at, by, why}`. The last two come from rule 6.

**4. A conflict carries its disposition.** `accept` and `dismiss` describe
the same underlying thing — two commitments that may not both be honoured —
and derive to one record with one of three dispositions: `open{reason}`
(proposed, never ruled on), `tolerated{rationale, revisit?}`,
`dismissed{rationale?}`. Conflicts are symmetric: `(a,b)` and `(b,a)` are one
conflict.

`reason` and `rationale` are deliberately different words for different
things. A `reason` says why a pair is in tension and belongs to whatever
proposed it. A `rationale` says why a person ruled the way they did.

Implementations MUST NOT derive `open` from the log. A pair nobody ruled on
left no act by definition; `open` exists for surfaces that *propose*
conflicts, so that a proposed and a settled pair share one type.

**5. A question is a commitment-shaped hole.** A `question` act records
something the canon does not cover. It derives to a record with the SAME
three statuses: `active` is open, `superseded{by}` is answered by that
commitment, `retracted{at}` is withdrawn.

Answering a question is superseding it. Withdrawing one is retracting it.
Implementations MUST NOT add a separate answer or close act: the two that
exist already mean the right thing, and a second vocabulary for the same
two transitions is how a format grows a dialect.

A `question` is not an adjudication. An implementation MUST NOT flag one
authored by a non-human actor — noticing a gap decides nothing.

**6. A commitment is a proposal until its scope ratifies it.** Every
commitment introduced by `assert` or `supersede` derives to `proposed`
until the ratification rule of its scope is met, and to `active` once it
is. The rule is the deepest `ratification` act covering the commitment's
scope, else the canon-wide one, else `standing`. Four rules ship:

- `standing` — a holder of the scope writes a rule directly; anyone
  else's write takes one holder's approval; a scope nobody holds is open.
- `joint{holders}` — every named person must approve; one of them
  objecting refuses it.
- `threshold{approve, block}` — this many holders approving carries it,
  this many objecting refuses it.
- `consent{days}` — a rule after this many days unless a holder objects
  with a reason.

Approvals and objections are `position` acts whose `about` is the
commitment's id: `toward` approves, `against` with a non-empty `because`
objects. Only positions from people who hold the scope at its **narrowest
granted level** count; everyone else's are kept and do not count. Positions
from non-human actors never count, and a non-human actor never ratifies its
own write by holding standing. A holder is one whose grant covers the scope
and is held at the time of the position, and who holds it at the deepest
level anyone does — subsidiarity, applied to authorship.

A `supersede` retires its targets only once the new commitment is
`active`. A proposed replacement leaves the rule it would replace standing.

A canon with no `grant` act before a commitment was written has no
holders and no gate: that commitment is `active` on arrival. This is every
canon that predates this rule, and they MUST keep deriving the same way.

**7. Governing takes standing.** A `grant`, `withdraw` of somebody else,
`policy` or `ratification` act whose actor does not hold standing over the
scope it names — or over the scope above it — is recorded and NOT applied.
So is a ruling on the record by somebody without standing over what it
touches: `accept` or `dismiss` over a pair, `retract` of somebody else's
commitment (withdrawing your own is always yours), or `decided` by somebody
with no standing in the canon at all. The act stays in the log, flagged,
and has no effect on the derived state. An agent with a seat over the
kitchen that dismisses a pair of hall rules has spoken, and the record
keeps that it did; nothing changed. Implementations MUST surface such acts rather
than drop them. Only grants made strictly before the act count, for and
against: acts written in the same second cannot govern one another, so a
founder's first twelve grants all take. A canon with no earlier grant is
ungoverned and open; the first grant closes it. Withdrawing your own
standing is always yours to do.

**A `revert` is gated the same way, and this one is load-bearing.** A
tomb-stone is as much a governance move as the act it covers: gating who may
*write* a grant while leaving who may *delete* one open is not a gate,
because a stranger who reverts every grant leaves a canon nobody holds, and
a canon nobody holds is open. Reverting your own act is always yours.
Reverting somebody else's takes standing over what it touched — the scope a
governance act named, the scope of the commitment it introduced, else the
canon — and a `revert` naming several targets applies to all of them or to
none. It is judged against the standing that stood when the revert was
written, so a grant deleted today does not retroactively unseat what was
done under it.

**8. A schedule is a query, not an act.** There is no op that records a turn
taken, and there must not be one. Given a scope's `allot`, the grants held at
a moment, the `allocation` rule and a clock, whose turn it is is a pure
function every reader computes identically — the same rule the draw already
follows. A community running a rotation writes **no per-turn acts at all**.

`allot.units` are NAMED and ordered; a bare count is written out as `1 … n`.
The order carries meaning a count cannot — `gate-1 … gate-11` runs down a
canal — and a reader MUST preserve it. Re-allotting a scope replaces its pool;
`allocation` acts are kept rather than replaced, because a rotation counts its
periods from the moment its rule was adopted, snapped down to a whole multiple
of the period. Both are governance acts under rule 7: an `allot` or an
`allocation` by somebody without standing over the scope is recorded and not
applied.

Who may take a turn is **whoever holds standing covering the scope** — the
first design principle doing a second job, since boundaries decide who may
appropriate and not only who may decide. An `allocation` naming a written
order restricts turns to the actors it names, and a holder the order does not
name MUST be reported rather than dropped.

Neither op changes what is live, so both are annotations: a reader that does
not know them carries them and says so, and no version bump is required.

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

## The snapshot block

A snapshot is what travels when a log does not: readable by a person
scrolling a chat thread, parseable back by a tool.

```
--- canon <lineage> · <profile> · snapshot <YYYY-MM-DD> · <generation>
<text>  (<id>)
<text>  (<id>)
--- <n> live · adopt: canon adopt --paste
```

Each body line is the commitment's text, two spaces, and its id in
parentheses. The id is LAST so the text may contain anything, including
parentheses. Readers MUST tolerate surrounding chatter: a block arrives
with a "here you go" above it and a reply below.

**A snapshot is not a log.** It carries derived current state and drops
supersession history, rationales, and the reasoning behind tolerated
contradictions — the parts that name incidents and people. Enough to
adopt, not enough to audit.

### Generations

The generation is a digest over the snapshot's `(id, text)` pairs, sorted,
joined, hashed with SHA-256, and truncated. Order-independent, so two
people holding the same rules are on the same generation whatever order
their files landed in.

**The text is in the digest and MUST be.** Hashing only the ids looks
sufficient — ids are already content hashes — but the ids in a pasted
block are characters someone can retype. A reader MUST refuse a block
whose declared generation does not match its commitments: it was edited
after it was shared, and adopting it records someone else's name against
words they did not write.

### Adopting

Adopting a snapshot writes one `adopt` act naming lineage, generation and
source, then one `assert` per commitment carrying `from` the upstream id.
That `from` link — not position, and not text matching — is what a later
divergence is computed against. Text matching would call a reworded rule a
different rule, and a canon that arrived by paste has no git history to
fall back on.

## Relationship to the Commonwealth governance oplog

The **envelope is shared** — `id`, `v`, `ts_unix`, `actor`, flattened
body tagged on `op`, content-addressed ids with a tenancy prefix. The
**act vocabulary differs**: a canon commitment carries its text inline,
while a governance rule references an extracted atom in a corpus atlas.

Interoperation is therefore a documented mapping, not identity. Saying so
plainly is better than implying a compatibility that does not hold.
