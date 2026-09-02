# Cookbook

The questions groups actually ask, and the acts that answer them.

Everything here is **model-free** except the two recipes marked *(needs a
model)*. In a house, one person runs the model half and everybody else needs
nothing.

The examples are one small house, built up as it goes: Mira, Sam and Dana hold
the house; Priya holds the kitchen; Ola lives there and holds nothing yet.

---

## First, the shape

Three different things, and keeping them apart is what makes the rest work.

| | what it is | example |
|---|---|---|
| **a commitment** | the rule itself — the thing you cite | *"Bikes go against the left wall."* |
| **a position** | citing it, for or against, with a reason | citing that rule against a bike rack |
| **a policy** | what the group does when citations conflict or run out | *one reasoned objection stops it* |

You **cite a commitment** to justify an action. **Policy** is the layer above:
it never justifies anything, it decides what happens when two justifications
collide or nobody can find one. That split is why a group can change how it
decides without rewriting what it decided.

---

## Setting up

### "Who actually decides this?"

Give people standing over a scope, then ask.

```sh
canon grant human:mira house
canon grant human:sam house
canon grant human:dana house
canon grant human:priya house.kitchen

canon who house.kitchen
```

```text
human:priya  over house.kitchen
human:dana  over house
human:mira  over house
human:sam  over house

4 with standing, narrowest first
decided under: cautious/consent
```

Narrowest first, and that ordering *is* subsidiarity. A scope is a dotted
path: `house` covers `house.kitchen` and does not cover `household`. Nobody
has to be asked who decides — it's a query.

### "How do we stop one person quietly rewriting the rules?"

Say what makes a proposal into a rule. Four rules ship: `standing` (whoever
holds the scope writes directly — the default), `joint:a,b` (everyone named),
`threshold:n/m` (n approving carries, m objecting stops), `consent:Nd` (N
days' silence carries it).

```sh
canon policy set consent --cautious \
  -m "One reasoned objection stops a thing, and what cannot be undone is not decided by silence."
canon ratification set threshold:2/1 --scope house \
  -m "A house rule takes two of us; one reasoned objection stops it."
```

Now a write is a proposal until the rule is met, and the command says so:

```sh
canon add "Bikes go against the left wall of the hall." --scope house
```

```text
can-294920c3d916  Bikes go against the left wall of the hall.
  PROPOSED, not yet a rule — needs 1 more approval(s) from people who hold house
  approve it:  canon approve can-294920c3d916
  object:      canon object can-294920c3d916 -m "<why>"
```

Dana wrote it and holds the house, so she counts as one of the two. Sam
seconds it:

```sh
canon approve can-294920c3d916 -m "the hall is impassable"
```

```text
can-294920c3d916  approved by human:sam
  in force
```

Ola, who holds nothing, needs both:

```text
can-6c338fed1276  Guests may stay up to three nights.
  PROPOSED, not yet a rule — needs 2 more approval(s) from people who hold house
```

Her proposal is on the record, visible, and not in force. Nothing was thrown
away and nothing was waved through.

---

## Day to day

### "Does this clash with something we've already decided?" *(needs a model)*

```sh
canon check "put a bike rack in the hall"
```

```text
THIS NEEDS AN AMENDMENT
  it runs against a rule the house already has:

  can-294920c3d916  "Bikes go against the left wall of the hall."
                    asserted 2026-09-02, in force, never superseded
                    because: a rack against that wall is the width of a pram
  amend it:  canon supersede can-294920c3d916 "<the new rule>" -m "<why>"
  or carry both knowingly:  canon accept can-294920c3d916 <other> -m "<what this protects>"
```

The model's only job is finding **which rules bear on it and which way they
pull**. The verdict itself is code, so it's reproducible from the evidence
forever, and you can argue with the evidence without having to trust the
verdict.

Every citation names a rule the canon actually holds — anything citing a rule
that isn't there is dropped, and the drop is reported. **You cannot justify an
action by a rule nobody wrote.**

To put a position on the record yourself, model or no model:

```sh
canon position "a bike rack in the hall" --citing can-294920c3d916 --against \
  -m "the rack is the width of a pram"
```

The reason is required in both directions. A position whose reason nobody can
check is an assertion, and this whole tool exists to replace assertions with
citations.

### "Nothing we've decided covers this"

This is the one canon has a real opinion about.

**An unaddressed proposal is not an approved one.** Under the shipped default
it routes to *ask one person with standing* and never to *act* — neither a
conflict nor a silence authorises anything by itself. Most systems treat "no
rule against it" as permission; this one refuses to.

With a model, `canon check` says so plainly:

```text
THIS NEEDS A NEW RULE
  nothing the house has decided bears on it.

  write one:  canon add "<the rule>"
  or record the gap for the next meeting:  canon question "put a sauna in the garden"
```

Three moves follow.

**Record the gap so it survives the conversation.**

```sh
canon question "Who looks after the plants when everyone travels at once?"
```

```text
can-65840b5de0cc  ? Who looks after the plants when everyone travels at once?
  answer it:  canon supersede can-65840b5de0cc "<the rule>" -m "<reason>"
```

**Answering a question is superseding it.** There is deliberately no separate
`answer` verb — the two transitions that exist already mean the right thing,
and the result is that the new rule carries the gap it closed:

```sh
canon supersede can-65840b5de0cc \
  "Whoever is away longest waters the plants; say so in the group chat before you go." \
  -m "Three plants died in August and nobody had been asked."

canon why can-33be28b8ccee
```

```text
can-33be28b8ccee  Whoever is away longest waters the plants; say so in the group chat before you go.
  asserted 2026-09-02 by human:mira
  reason for the change: Three plants died in August and nobody had been asked.
  replaced can-65840b5de0cc: the question "Who looks after the plants when everyone travels at once?"
  status: in force
```

**Say when you decided *not* to have a rule.** A gap and a deliberate silence
are different facts, and only one of them wants filling:

```sh
canon silence "a chore rotation" \
  -m "We decided in March not to have one — it would turn a kindness into a duty."
```

```text
can-a42afdeb46c1  unwritten on purpose: "a chore rotation"
  We decided in March not to have one — it would turn a kindness into a duty.
  `canon check --about "a chore rotation"` will say so rather than call it a gap
```

`check` on that subject now reports `UNWRITTEN ON PURPOSE` with the reason,
who decided it and when — instead of prompting for a new rule, which is how a
tool turns a working unwritten practice into a rota nobody wanted.

**Let the stakes set the ceremony.** `--cautious` wraps any rule: what is
*irreversible* and *unaddressed* is refused rather than permitted by silence.
So a novel reversible thing gets act-and-notify, and a novel irreversible one
stops until a person looks. That is the actual answer to reasoning about a
situation no rule covers — not deriving a verdict, but routing it by what it
would cost to be wrong.

### "We keep having the same argument"

```sh
canon policy set default --graduated ask-one,ask-panel,refuse
```

First time about a subject, ask one person. Second time, ask the group. Third,
refuse. Repetition is the signal that something needs a rule, and the ladder
makes the signal automatic instead of relying on somebody noticing.

### "Why is this rule like this?"

Never edit a rule. Supersede it, and the reason is kept:

```sh
canon supersede can-294920c3d916 \
  "Bikes go against the left wall of the hall, and the shed once it is built." \
  -m "The rack argument in September; the wall alone was never going to hold six bikes."

canon why <the new id>
```

A year later *why is the bike rule like this?* has an answer and nobody had to
remember it. The replacement is a proposal like any other, so under
`threshold:2/1` it takes a second holder before it retires the rule it would
replace — a proposed replacement leaves the old rule standing. The `-m` reason is not politeness: a rule with no reason
can't be revisited well, because you can't tell whether the thing it protected
against still exists.

### "Two of our rules contradict"

Finding them needs a model:

```sh
canon tensions          # (needs a model)
```

Deciding what to do about one does not. A contradiction you are carrying on
purpose is something to record, not a bug to clean up:

```sh
canon accept can-294920c3d916 can-8514 -m "nowhere else for the bikes until the shed is built"
canon dismiss can-1234 can-5678 -m "these are about different halls"
```

`accept` requires its reason — a tolerated contradiction has to say what it
protects, or nobody can tell later whether it still does.

---

## Changing how you decide

### "What would a different rule have done to us?"

The question every group has before changing how it decides, and normally
unanswerable. Here it takes no setup and no files:

```sh
canon replay                                              # your own record, re-decided
canon replay --policy threshold --objections 2 --brief
```

```text
Under `threshold` instead of the rules this canon adopted, 1 of 3 decisions land somewhere else.
1 would be easier to do.

  EASIER
    a bike rack in the hall
        not under this policy → act

  the reason on each side: the same command without --brief
```

The questions come from what your canon already holds — every subject somebody
took a position on, everything the group decided — so the answer is about
decisions the house really had. The whole policy vocabulary can be forced, so
the rule you are actually weighing is one you can ask about.

Want to ask better questions than the record can know about?

```sh
canon replay --write-scenario questions.jsonl   # the derived ones, to edit
canon replay --scenario questions.jsonl --policy consent --brief
```

### "Someone changed something they had no say over"

```sh
canon ratification set standing --scope house -m "let anyone write"    # run by Ola
```

```text
can-aa41b198d998  standing
  let anyone write
  how rules are made in house and everything under it
  NOT APPLIED: human:ola set how house makes rules without holding it
  it is on the record; somebody with standing has to do it.
```

Exit code 1, and `canon list` footers how many such acts are sitting there.
The act is kept — the house should see that it was tried — and it changed
nothing. The same gate covers grants, policies, rulings, retracting somebody
else's rule, and `undo` of somebody else's act. Withdrawing your own is always
yours.

---

## People, dates and machines

### "Standing shouldn't be forever"

```sh
canon grant human:priya house.kitchen --horizon 2027-01-01
canon horizon can-294920c3d916 2026-03-01 -m "check the hall still works in winter"
canon overdue
```

```text
2026-03-01  can-294920c3d916  Bikes go against the left wall of the hall.
          check the hall still works in winter

1 overdue
```

One act covers what people call term limits, sunset clauses, trial periods,
revisit dates and rotation — they are the same shape. Nobody has to remember
to take a seat away.

When someone steps back, take the thing they know with them:

```sh
canon leave house.kitchen -m "nobody has ever written down which pans are shared"
```

```text
human:priya no longer holds house.kitchen
can-fab1fa47a5db  ? nobody has ever written down which pans are shared
  recorded without your name.
  it sits next to your withdrawal in the log, so treat the anonymity as thin.
```

Their standing ends and the thing they know outlives the seat — with the tool
saying plainly how thin the anonymity is, rather than promising more than a
log can give.

### "An agent is acting in our community"

```sh
canon mcp        # the agent surface, on stdio
```

An agent gets `canon_list`, `canon_open`, `canon_why` and `canon_check` — all
read-only. So it can look up what bears on a thing and cite it as
justification, which is exactly what a monitor is for.

What it cannot do is decide. **An agent may propose and may object; it never
mints a rule** — not even its own proposal, not even holding standing. Any
adjudication written by a non-person is surfaced by name rather than hidden:

```text
warning: 1 adjudication(s) were not authored by a person: can-a976a31ea826
```

That is Ostrom's fourth principle stated as a type: the monitor is answerable
to the people, never the reverse. A house can change it, and the record will
show that it did.

### "Whose turn is it?"

A commons usually has something to share out — sites, headgates, machine
slots. Say what there is, say how it goes round, and the schedule computes
itself. This is Alanya's inshore fishery, which has run this way since the
seventies:

```sh
canon allot fishery.sites --unit site \
  --named kizilburun,incekum,karaburun,mahmutlar,konakli,payallar,turkler,okurcalar,avsallar,demirtas,kargicak \
  -m "The written list of spots, and the order runs west to east along the coast."

canon allocation set rotation --scope fishery.sites --step 1 --per 1d \
  --order human:kemal,human:ayla,human:bora,… \
  -m "From September each boat moves one site east a day."

canon pool fishery.sites
```

```text
fishery.sites — turn 0 under rotation:1/86400s

  avsallar    human:halim
  demirtas    human:irem
  incekum     human:ayla
  karaburun   human:bora
  kizilburun  human:kemal
  …
```

And it moves on its own. One boat, five days:

```text
2026-09-02  kizilburun
2026-09-03  incekum
2026-09-04  karaburun
2026-09-05  mahmutlar
2026-09-06  konakli
```

**Nobody writes a turn down.** The schedule is a query, like the draw —
given the pool, who holds standing, the rule and the date, every reader
computes the same answer. A rotation costs no daily bookkeeping at all.

Four things worth knowing:

- **The sign of `--step` is the direction.** Alanya rotates east from
  September and west from January: same rule, different sign.
- **`--from-draw <id>` orders by a lot nobody can steer**, reusing a draw's
  verified seed. That's Alanya's September assignment.
- **Who may take a turn is who holds standing over the scope** — boundaries
  decide who may appropriate, not just who may decide.
- **A `--order` roster restricts turns to the people it names**, and anyone
  holding the scope who isn't on it is reported as idle rather than dropped.
  That's how you keep a monitor out of the rotation without taking away the
  standing it watches with.

`canon pool --at 2027-01-14` asks about any date. `canon who` is authority;
`canon pool` is appropriation.

### "Who's on the panel?"

A lot nobody can steer — announced first, seeded from secrets sealed before
the boundary and opened after:

```sh
canon draw commit house 3 --after 2027-03-01
```

```text
can-cfbfb942625e  3 seat(s) from house, after 2027-03-01
  everyone in the pool seals a secret BEFORE 2027-03-01:
    canon draw seal can-cfbfb942625e
  and opens it after:
    canon draw open can-cfbfb942625e
```

`canon draw show <draw-id>` then prints the panel. The draw is a query, not
an act: given the commit and the verified secrets, every reader computes the
same panel and nobody performs it. It refuses rather than falls back — if the
boundary is in the past, if no secret verifies, or if the seats would take the
whole pool, it says so.

---

## Three things that surprise people

**Unaddressed is not approval.** A proposal nothing bears on comes back as a
finding, not a green light. If you want silence to permit things, adopt a
policy that says so — and the record will show you chose it.

**A scoped ratification rule doesn't cover an unscoped rule.** `ratification
set threshold:2/1 --scope house` governs `house` and everything under it. A
commitment written with no `--scope` isn't under it and falls back to the
canon-wide rule. Either scope your rules, or set the ratification rule with no
`--scope` so it covers everything.

**Acts written in the same second cannot govern each other.** That is what
lets a founder write twelve grants in one sitting without the first one to
sort locking out the other eleven. It is also a documented loophole, written
down in `SPEC.md` rather than hidden.

---

## Where this fits

The three recipes above map onto the three levels Ostrom found in commons that
lasted: the rules (`add`), the rules for making rules (`grant`,
`ratification set`, `policy set`), and who may change *those* (the same acts,
aimed one scope up).

- [Getting started](./GETTING_STARTED.md) — a house's first hour, and the tier table.
- [SPEC.md](./SPEC.md) — the file format, and every rule the fold applies.
- [PRIMITIVES.md](./PRIMITIVES.md) — why each of these is a primitive rather than a feature.
