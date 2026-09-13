# The pieces and the verbs

canon keeps one notebook for a group's rules: what you decided, what replaced
it and why, who gets a say about what, and what you left alone on purpose.
This page is that notebook in plain words — the ten ideas underneath it, and
the verbs you type. Every example here is real and runnable; `canon help all`
has the complete list with every flag.

The tool has two halves. The **record half** never calls a model and never
needs one: adding a rule, changing a rule, deciding who decides — all plain
writes to a file you own. The **reading half** — pointing it at your old notes
and letting it propose rules it finds — calls a language model, on your machine
unless you say otherwise, and everything it proposes waits for a person. You
review one at a time, and there is no accept-all.

## The ten ideas

Underneath, canon is ten small ideas. You never need their formal names to use
the tool, but the output sometimes uses them, so here is each one in house
words. The formal argument for why these ten and no more is in
[PRIMITIVES.md](./PRIMITIVES.md) — a design document you never have to read to
run the tool.

1. **The notebook never erases.** Fixing a rule adds a correction; the old
   wording stays, marked replaced, next to the reason. Undo works the same way
   — it's a visible act, not a deletion.
2. **Four moves.** Something enters, something replaces something, something is
   withdrawn, something is undone. That is all that can happen to a rule.
3. **Notes about rules.** A question the rules don't answer. A contradiction
   you carry on purpose, with a date to look again. A decision to write
   nothing. A seat handed to someone. These are entries too.
4. **A reader, not a judge.** The part that reads your own documents turns
   text into checkable facts — what number was said, what thing was named —
   and plain code compares the facts. A model helps read. It never decides.
5. **Pulling, with receipts.** Support or objection names what it pulls from —
   a rule you hold, or yourself — and states a reason. An objection without a
   reason is not an objection here.
6. **Rooms.** Authority is held over a room — `house`, `house.kitchen` — and
   rooms nest. A seat is granted for a while; when it ends it ends in the
   record, not in someone's memory. (The tool calls a seat *standing* and a
   room *scope*.)
7. **How a room decides.** Each room can say how its own rules get made: a
   seatholder writes, named people must all agree, so many for and so many
   against, or nobody objects within so many days. Changing that rule is
   itself judged under the rule it replaces.
8. **Two questions it can always answer.** What have we decided that
   contradicts itself? And what has gone stale past its revisit date?
9. **A hat nobody can rig.** Picking names by lot, where every participant
   seals a secret before a deadline and the pick is computed from all of
   them. Nobody can steer it — including whoever called the draw — and anyone
   can recompute it from the record.
10. **Whose turn.** For things a group shares — the car, the good room, the
    build machine — the rotation is computed, not staffed. There is no
    bookkeeping to do and nothing recorded about what anybody did.

## The verbs

Type `canon` with no arguments and you get the seven everyday verbs; `canon
help all` lists everything. Grouped here by what you're doing. Anything marked
*model* calls a language model; everything else works offline.

### Start

| What it does | Example |
|---|---|
| Start a notebook here — for a house, yourself, or a codebase | `canon init --profile house` |
| Point it at a model endpoint for the reading half | `canon config set endpoint http://localhost:9741/v1` |

### Day to day

| What it does | Example |
|---|---|
| Propose a rule; it's a proposal until its room ratifies it | `canon add "Quiet hours run 11pm to 7am." --scope house.quiet` |
| What's in force now, and what's still proposed | `canon list` |
| Where a rule came from, and what it replaced | `canon why can-ffc1` |
| Replace a rule, keeping the reason | `canon supersede can-ffc1 "Guests may stay up to three nights." -m "Sam's cousin stayed two weeks and nobody knew how to raise it."` |
| Withdraw a rule with no replacement | `canon retract can-ffc1 -m "Nobody was using it."` |
| Undo an entry — yours, or one you hold a seat over | `canon undo can-0ea2 -m "That was the wrong room."` |
| Every entry, in order | `canon log` |

### When the rules run out

| What it does | Example |
|---|---|
| Record a gap the rules don't cover | `canon question "Who looks after the allotment?"` |
| The open questions | `canon open` |
| Record that something is unwritten on purpose | `canon silence "party politics" -m "We decided not to have a rule about this."` |
| Carry two conflicting rules knowingly, with a look-again date | `canon accept can-ffc1 can-9b31 -m "We know; revisit in spring." --revisit 2027-03-01` |
| Rule out an apparent conflict | `canon dismiss can-ffc1 can-9b31 -m "One is about guests, one about pets."` |
| Put a review date on any entry | `canon horizon can-ffc1 2027-03-01` |
| What has gone past its date | `canon overdue` |

### Who gets a say

| What it does | Example |
|---|---|
| Who may decide this room, and under what rule | `canon who house.kitchen` |
| Give someone a seat, optionally with a term | `canon grant human:dana house.kitchen --horizon 90` |
| Step down, or stand someone down | `canon withdraw human:dana house.kitchen -m "Term is up."` |
| Put a rule in a room | `canon scope can-ffc1 house.kitchen` |
| Say how a room's rules get made | `canon ratification set joint:human:dana,human:sam --scope house.kitchen` |
| Show or change how proposals are judged | `canon policy set consent` |
| Approve a proposal | `canon approve can-9b31 -m "Yes — and the rota moves too."` |
| Object to one; the reason is required | `canon object can-9b31 -m "The pan rule already covers this."` |
| Pull for or against something, citing what backs you | `canon position "can-9b31" --against -m "It doubles the tool budget."` |
| Record what the group decided | `canon decide "guest room request" --outcome supported --authority act -m "Dana ruled; the guest stays to Friday."` |
| Mark a rule as a principle, not a convention | `canon rank can-ffc1 principle` |
| What someone raised, and what came of it | `canon voice human:dana` |
| Leave a room, and leave a question behind | `canon leave house.kitchen -m "Who waters the plants while I'm gone?"` |

### Sharing a thing

| What it does | Example |
|---|---|
| Name what a room shares | `canon allot house.tools --named drill,ladder` |
| Say how it goes round — whose turn, how long | `canon allocation set rotation --scope house.tools --order holders --per 7` |
| Whose turn it is right now | `canon pool house.tools --at 2026-09-14` |
| Announce a draw for a future date | `canon draw commit house.tools 1 --after 2026-10-01` |
| Seal your secret before the date, then open it | `canon draw seal draw-1` |
| The panel, recomputed from the record | `canon draw show draw-1` |

### Looking back

| What it does | Example |
|---|---|
| Re-decide your own history under a different rule | `canon replay --policy consent --brief` |
| Where your commitments conflict *model* | `canon tensions` |
| How a proposal stands with the rules you have *model* | `canon check "Guests may stay a week."` |
| The ledger checked against git — append-only, not backdated | `canon witness` |
| What is guarding the ledger, and what each guard cannot see | `canon guard show` |
| How you've diverged from a canon you forked from | `canon diff --upstream` |

### Reading your old notes

One verb, `draft`, and it's the model half. Point it at any folder of text —
chat exports, org files, a `NOTES` file — and it proposes rules one at a time,
each quoting the passage it came from:

```sh
canon draft --from ~/house-notes        # a folder, read recursively
canon draft --from-git --since 1y       # or your commit messages
```

A small local model proposes worse rules and misses more conflicts — the size
of the model is what moves quality here. Measure yours with
`./scripts/draft-bar.sh 3`.

### Someone else's canon

A canon is a file; you can fork one, carry your own rules onto a different
base, and diff the two. `canon share` prints a pasteable snapshot, `canon
adopt --paste` forks one, and `canon rebase --onto <url>` carries your rules
onto a different base (*model*). `canon mcp` serves the same verbs to an agent
over MCP.

## Where to go next

- [Getting started](./GETTING_STARTED.md) — a house's first hour, step by step.
- [Cookbook](./COOKBOOK.md) — the questions groups actually ask, answered with
  real commands.
- [PRIMITIVES.md](./PRIMITIVES.md) — why these ten ideas and no more.
- [SPEC.md](./SPEC.md) — the file format itself, if you want to read the raw
  notebook.
