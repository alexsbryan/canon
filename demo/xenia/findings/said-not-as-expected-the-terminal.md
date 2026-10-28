# Didn't do what they expected: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: soft (the person's): what they said when asked "not as expected"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `june-explore`: 1 of 1 accounts (June run 1)

## What happened

- `june-explore` run 1, June: passed, explored, and said they were done. their last acts: say “Sam — June here. I've had a careful play with canon, only reading (help, list, log, why, and a check that just reported it couldn't judge). I didn't write or change anything.

What I think it's for: it keeps the house's rules in one place, and — the really good part — it keeps the *story* of them, not just the current wording. For each rule I can see who wrote it and when, what it replaced and *why*. Your guest rule shows it replaced the earlier "up to three nights" because your cousin stayed two weeks and nobody knew how to raise it. That's the sort of thing I'd otherwise have to guess at.

Two bits that struck me as especially thoughtful:
- The "silence" on a chore rotation. It's written down that you *decided against* one, in March, because it would turn a kindness into a duty. So a future look won't see a missing rule and assume a gap — it'll see a decision.
- The plants question — "who looks after the plants when everyone travels at once?" — is logged as asked but not answered. That looks like a real gap to me, and now that it's recorded it won't get lost.

So my honest reading: it's for making decisions rememberable — what was decided, by whom, why, and what was deliberately left undone. It's less a rulebook than a memory that doesn't flatter you.

One practical note: `check` and `draft` need a model behind them and there isn't one set up yet, so `check` just says it can't judge. Everything else works fine offline. I left the config alone since that's yours to decide.

I'm still new here and careful with anything that writes. Happy to go further if you'd like, but only if you ask.” · tell · tell · tell · tell · tell

## What the people said

- June, `june-explore` run 1: “Typing `canon check --about "..."` — the very form the tool's own message suggested — errored with a usage complaint. And `canon check "a chore rotation"` didn't return a verdict either; it just said no endpoint was configured and told me to run a config set command. I didn't run that, because changing the house's configuration isn't my decision to make.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
