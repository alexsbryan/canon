# canon

A decision log that knows what it currently says.

Hold a body of commitments. Record what was decided about them, and why.
Ask whether a proposal sits with or against them.

Works for one person, one codebase, or one household.

```sh
canon init
canon add "Survey what exists and prove it cannot serve before building."
canon add "Ship the smallest thing that closes the issue."
canon list
```

Later, when something changes:

```sh
canon supersede can-4f19 "Prefer extending an existing helper." -m "PR #612 discussion"
canon why can-9b02          # what this replaced, when, and the reason given
```

And when two commitments genuinely conflict and you are keeping both:

```sh
canon accept can-a81 can-3d2 -m "reliability is how I earn the autonomy, for now"
```

Nothing is ever force-resolved and nothing is destroyed. A contradiction
you are carrying on purpose is a first-class state, not a bug to be
cleaned up.

## Why it is shaped like this

**One file.** Everything lives in `.canon/acts.jsonl`, append-only. It
diffs, so git gives it history for free. It greps. You own it — leaving
is deleting a directory.

**Current state is derived, never stored.** What is live, what replaced
what, which contradictions you are carrying: all of it is a pure fold
over the log. `canon-core` has no filesystem or network dependency at
all, which is enforced by its dependency list rather than by discipline.

**Nothing is destroyed.** Every act is revertible, including a revert.

**Most of it needs no model.** `add`, `list`, `why`, `supersede`,
`retract`, `accept`, `dismiss`, `undo`, `log` and `share` are the fold.
Only `check`, `tensions` and `draft` call one, and they take any
OpenAI-compatible endpoint.

## Status

Early. The record verbs work and are tested; `check`, `tensions` and
`draft` are not implemented yet and exit `3` (*cannot judge*) rather than
guessing.

## Format

[SPEC.md](./SPEC.md) — released **CC0**, public domain, so adopting the
format is not a lock-in decision. The tooling here is AGPL-3.0-or-later;
the record format belongs to nobody.

Larger tools read the same file.

## Exit codes

`0` supported · `1` conflicts · `2` unaddressed, or a usage error ·
`3` cannot judge

`--json` puts data on stdout and logs on stderr, so this drops into CI
and agent tooling without a wrapper.

## Build

```sh
cargo build
cargo test
```

## License

AGPL-3.0-or-later. The format specification is CC0.
