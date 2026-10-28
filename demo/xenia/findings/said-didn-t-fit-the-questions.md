# Didn't fit together: the questions

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: soft (the person's): what they said when asked "didn't fit"; never both

## Where

the questions. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / can-b21ecb905c35 2026-10-28 human:Sam silence "a chore rotation" / can-cae07a1249c4 2026-10-28 human:Sam assert Guests may stay up to three nights.

## Who, and how often

- `mira-explore`: 1 of 1 accounts (Mira run 1)

## What happened

- `mira-explore` run 1, Mira: passed, explored, and said they were done. their last acts: say “Had a proper play with canon. I only read — didn't change anything, and I wouldn't have without asking first.

What it is, as I understand it: a record of the house's rules where every change keeps its reason attached. Not a list of rules — a list of decisions: who decided, when, why, and what it replaced. Three rules are live (cooking/washing up, quiet hours 10pm–7am, guests up to three nights unless the house chat says otherwise). Seven acts in the log, all yours from 28 Oct.

The bit I actually wanted works. `canon why can-ad6f90907d37` tells me the guests rule changed because your cousin stayed two weeks and nobody knew how to raise it. I didn't have to ask you. And `canon why` on the old version of the rule shows it was replaced, by what, and in force after.

The other thing I like: it records the *absence* of a rule as a decision, not a gap. There's a "silence" on a chore rotation — we decided in March not to have one, because it would turn a kindness into a duty. That's in there with the reason. So next time someone says "should we do a rota?", the answer is findable instead of starting the argument again.

And it's honest about what it doesn't cover: there's an open question about who waters the plants when everyone's away at once. It shows up under `canon open` as one open item, and `why` on it tells you the command to answer it.

Three things I noticed, in case they're useful:

1. `why` on the silence suggests `canon check --about "a chore rotation"`. That verb doesn't exist — it errors with `usage: canon check "<proposal>"`. So that line of the help is stale. Worth fixing, because a tool whose job is "don't ask Sam" shouldn't send you to a command that doesn't run.

2. `check` and `draft` need a model and there's no endpoint configured here, so they just say `cannot judge`. That's fine by me — the read-only half (list, log, why, open) needs no model at all, and that's the half I want. I didn't set an endpoint; that's your setup, not mine to change.

3. The type/Enter thing caught me out — worth knowing if I ever paste you a line.

One thing to flag: when I looked at the covenant, "Sam's cousin" is named only as a relationship and the reason is written for the house to read. Fine as far as it goes, but bear in mind anyone I show canon to can read why each rule exists, not just what it is. Some reasons are more comfortable to write down in that voice than others.

Seems like a good thing to have. Can I ask it a question I'd rather not put to you in the group?” · tell · tell · tell · tell · tell

## What the people said

- Mira, `mira-explore` run 1: “Two things. The tool contradicts itself: `why` on a silence tells you to run `canon check --about "...")`, but `check` doesn't take that — it errors and says `usage: canon check "<proposal>"`. So the built-in advice sends you to a command that won't run. And the deeper half of the tool (check, draft) needs a model endpoint, which isn't set up here, so those just say "cannot judge" — while the plain read-only half (list, log, why, open) works fine with no model at all.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
