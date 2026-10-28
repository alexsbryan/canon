# Didn't fit together: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: soft (the person's): what they said when asked "didn't fit"; never both

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

- June, `june-explore` run 1: “The help text itself pointed to `canon check --about "a chore rotation"`, but when I typed exactly that, the tool rejected it and told me the usage is `canon check "<proposal>"`. So the tool's own advice didn't match the tool. Once I dropped `--about`, it accepted the command — but then said it couldn't judge, because no model endpoint is configured. So the one part meant to reassure you that a silence won't be mistaken for a gap doesn't actually run here yet.”

## The product source

- `demo/xenia/findings/findings.json:236` holds “a chore rotation”
- `demo/xenia/findings/findings.json:329` holds “a chore rotation”
- `demo/xenia/findings/said-didn-t-fit-the-short-help.md:24` holds “a chore rotation”
- `crates/canon-cli/src/check.rs:530` holds “<proposal>”
- `crates/canon-cli/src/main.rs:55` holds “<proposal>”
- `crates/canon-cli/src/main.rs:167` holds “<proposal>”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
