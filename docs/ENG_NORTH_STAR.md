# canon: engineering north star

**Draft for the steward, not approved.** 2026-10-19.

canon is event sourcing for a group's rules, with authority derived rather
than stored. The record is an append-only log of content-addressed acts that
merges by set union. What is in force, and who may decide it, is
`fold(log, now, policy)`, a pure `no_std` function, so any history replays
under any other policy. Language models are untrusted parsers behind one
port: they return positions, code cuts the quote, and people ratify. The
shape fits any problem where who decided what, under which rule, must be
recomputable by anyone holding a copy, with no server to trust.

## Invariants

Each is a claim the compiler refuses to let code break. *State* says
whether that refusal exists yet, and what holds the behaviour until it does.

1. **Only the fold says what is in force, and it counts only people.**
   Outside `canon-core`, `Canon` and `Commitment` are read through methods
   and have no constructor, so `Log::derive` is the only way to get one, and
   `Verdict`'s variants are `non_exhaustive`, so one can be matched and never
   built. Every count that ratifies holds a `Person`, whose one constructor,
   `Person::of`, takes only a `human:` actor and lives in a module of its
   own, so not even the rest of the core can mint one.
   *State:* held since 2026-10-19, by `compile_fail` doctests on each type
   and watched red from the CLI (setting `Commitment::status` is E0616). The
   generated law `agents_cannot_ratify` still checks the counting itself.

2. **A model is quarantined behind one port.** `client_for` is the only
   constructor of `Client`, and a host off this machine needs a `Remote`
   token that only `--allow-remote` mints. The crate that calls a model
   cannot name the record's writer; a reply reaches the record only through
   `review`, one act per keystroke.
   *State:* not held. `Client::new` and `store::append` are `pub` in one
   crate. `locality_is_decided_conservatively` holds the locality check
   (watched red); only convention keeps `draft`'s review the one path from a
   reply to the record.

3. **Every quote is a slice of its passage.** A quote canon shows is a
   `Quote`, and both of its constructors take the passage: `locate::cite`
   cuts it at the positions a model returned, and `Quote::within` finds a
   quote read back from a run file. A candidate holding a `Quote` has no
   `Deserialize`, so a run file is read only through `DraftRun::bound`, which
   finds each quote in its own recorded chunk or refuses the file.
   *State:* held since 2026-10-19 for the quote (watched red: minting a
   `Quote` is E0423, reading a run without binding it is E0277), and every
   recorded run in `fixtures/` binds. Not held: that the passage handed in is
   the document read, since a `Chunk` is plain data; `Offered::at` returns an
   unclamped `Option<usize>` that a caller could ignore; and the number guard
   is held by unit tests (watched red).

4. **Same acts, same canon.** `canon-core` is `#![no_std]` and
   `forbid(unsafe_code)` with a closed dependency list, so a clock, a file, a
   socket, a model or a `HashMap` is a compile error there, and time is a
   parameter. Outside the core the fold takes only a `Log`, whose
   constructors sort by `(ts_unix, id)` and dedupe by content-addressed id.
   Only `store` can name the ledger file, so every write to it is a function
   there: `append`, `create`, which never truncates, and `materialize`, which
   writes a whole replayed canon for `replay --out`. Ops are a closed enum
   matched exhaustively, so a new op does not compile until every site
   decides it.
   *State:* mostly held. `no_std` (watched red: a clock read in the fold is
   E0433), the dependency list (`scripts/core-boundary.sh`), the fold behind
   `Log` (a `compile_fail` doctest) and the ledger's name (watched red:
   naming it from another module is E0603) are held. Not held: `Act` and
   `ActId::from_raw` let an id differ from its content's digest, the merge
   driver writes the path git hands it, and only the fold's dispatch matches
   ops exhaustively. The laws `replay_deterministic`,
   `arrival_order_independent` and `merge_union_laws` hold the behaviour
   (watched red).

They are ranked in that order. The first is what canon is for. The second
keeps an untrusted reader from becoming an author or a leak. The third lets
anyone with the document check a reading. The fourth makes the rest
replayable, and decides where code goes: inside `canon-core` if it changes
what is in force, outside if it touches a clock, a file, the network or a
model. The ranking has decided one change: on 2026-10-03 an agent stopped
minting rules in scopes nobody holds, and the first outranked the fourth.

## What the compiler cannot hold

These hold over the record as written. The fold trusts each act's
`ts_unix`, its `actor` and the file's bytes, so whoever can append writes
the canon. Defenses live outside the fold and are opt-in: `canon identify`
for signed acts, and `canon witness` and `canon guard git` against git.
Model accuracy is measured (`scripts/draft-bar.sh`), never gated.
[The contract](./CONTRACT.md#what-the-checks-assume) states the rest.

## Before any change

Ask whether it is mechanism or policy. Policy, such as a threshold, who
decides or what silence means, is an act in the record, never a default in
code. Mechanism changes [the format](./SPEC.md) and
[the contract](./CONTRACT.md), and a new op needs two institutions in
different domains and a written, failed attempt to compose it
([`adequacy_bar.rs`](../crates/canon-cli/tests/adequacy_bar.rs)). Anything
else lands outside `canon-core` and may not loosen the four.
