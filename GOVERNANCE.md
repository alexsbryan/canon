# Governance

One steward today. That's an awkward sentence to write under a tool whose
whole claim is that a group's rules can live in software as mechanism — so
rather than only assert it, this project records it.

```sh
canon who canon
#   human:alex  over canon
#   1 with standing, narrowest first
```

`.canon/acts.jsonl` is committed. This project's rules are in it, in the
same format, folded by the same code, with no special case for being the
project's own. If this document and the canon disagree, believe the canon:
it's the one with a reason attached to every line and a history you can
`git log`.

## What's in there

```sh
canon list                # what's in force, and what's proposed
canon open                # what nobody has decided
canon ratification show   # how a proposal becomes a rule, per scope
canon why <id>            # where any one of them came from
```

Five scopes, because the answer differs by area: `canon.format` (the record
format), `canon.core` (the fold), `canon.cli` (the tool's behaviour),
`canon.fixtures`, and `canon.docs`.

**Three silences are recorded**, which is the part most projects lose. A
plugin system for ops, a stale-issue bot, and an ingest-accuracy gate in
CI: each was decided against, each carries the reason, and `canon check`
will say "decided against" rather than "gap". Losing those is why the same
proposal comes back every spring.

**Three questions are open**, and the first one is the real one: *nobody has
decided what earns a grant.* A second person holding standing over a scope
is the next actual governance event here, and there is no rule for it yet.
I'd rather have that written down as an unanswered question than improvise
an answer the first time it matters.

## The one rule that binds me

`canon.docs` is under `consent:14d`. A documentation rule that no holder
objects to within fourteen days is a rule — which, with one holder, means
the cost of an unanswered proposal falls on me rather than on the person
who wrote it.

It binds my own writes too, and you can see that right now: two rules I
added on 2026-09-02 read `PROPOSED, not yet a rule` in `canon list`, waiting
out my own window. That is either the most convincing thing in this
repository or the most self-indulgent, and I've decided to find out.

Everything else is `standing` — what shipped as the default. Holders write
the rules of a scope; anyone else proposes, and one holder's approval makes
it a rule. An agent may propose and object under any of them and **cannot
mint a rule**, even where it holds standing.

## Where this is going

The people under a rule should, in time, be the ones making it. Right now
they aren't, and I'd rather name the phase than let it read as a
personality.

Steering closely now is how I earn the right to stop. You don't hand a
newborn project a finished constitution and wish it luck; you keep it
coherent long enough for real rules to form, write them down, and then live
under them. A constitution worth the name binds the person who wrote it —
which is what `consent:14d` is a small, literal down payment on.

The way in is ordinary: show up, send a fixture or a bug or a number
measured on your own endpoint, and the answer to *what earns a grant* gets
written by the first case that forces it rather than by me guessing in
advance. When it's answered it'll be answered in the canon, with the reason,
and `canon why` will say what it replaced.

## If it goes somewhere you can't follow

The door is unlocked, and unusually so for a project like this.

The tooling is AGPL-3.0-or-later. **The record format is CC0** — SPEC.md
owes this project nothing and neither does an independent implementation of
it. And a canon forks by design: `canon adopt` takes someone else's, your
local commitments survive an `upgrade`, and `canon diff --upstream` shows
how you've diverged. Ostrom's seventh principle is that the right to
organise isn't undermined from outside, and here that's an affordance the
tool provides rather than a promise I'm making.

A record you cannot leave is not a record you own. That applies to this one.

## Honestly

It's early, one group has used this and it's mine, and the thing I most
want is for someone to point it at their own mess and tell me what it got
wrong. Be patient, be constructive, and when it actually works, demand the
best.

This will change. A constitution you can't amend isn't one, it's a mood.
Call it a first draft — from someone whose whole job is to make the next
drafts need him less.
