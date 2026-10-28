# Didn't do what they expected: the rules

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: soft (the person's): what they said when asked "not as expected"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / superseded by "Guests may stay up to three nights; longer needs a house chat." can-3a12920028cf / reason it was replaced: Sam's cousin stayed two weeks and nobody knew how to raise it.

## Who, and how often

- `sam-explore`: 1 of 1 accounts (Sam run 1)

## What happened

- `sam-explore` run 1, Sam: passed, explored, and said they were done. their last acts: say “Played with it. Short version: it's a ledger of rules where every rule carries its own act and reason — that's the good part, and it's genuinely good.

What it does well: `why <id>` tells you what a rule replaced, by whom, and why, in plain language, and it goes both directions (the old rule names its successor). A `silence` is a first-class thing: "a chore rotation" isn't a gap, it's a decision not to have one, with the March reason attached — and `check --about` is supposed to say "unwritten on purpose" instead of calling it missing. `log` is the raw acts; `undo` is logged and itself revertible. When it can't judge it says so and exits 3 instead of guessing. That's the "show me the act and the reason" behavior I want.

What I don't trust: `canon who` says nobody holds this canon, it's open — and yet `canon add "Guests may stay a month"` printed **"in force"** and went straight into `list`, no ratification, no reason, alongside "three nights." The help says a proposal becomes a rule when its scope ratifies it; nothing ratified that. The tool that refuses to *judge* quietly will still *decide* quietly. Worse, `check` couldn't run at all (no endpoint configured), so the three-nights-vs-a-month contradiction was never looked at — and `add` didn't care.

Two more small ones: `config show` printed "nothing configured" and exited 0 when the very next thing you'd want (`check`) then fails with exit 3 — it should have said so up front. And the `--help` pager was sticky enough that commands I typed queued behind it instead of running; annoying in practice.

I undid my probe (`undo can-fdcc0d222a1c -m ...`); house is back to 3 rules.

So: what's it for? It's a memory for decisions, not a rulebook — it remembers *why* and *who*, and what you chose not to write down, so six months from now nobody re-litigates the chore rotation by accident. The thing I'd fix before you rely on it: it shouldn't accept a rule "in force" when no one holds the canon, and it should refuse to accept anything while `check` is unable to run. Want me to set an endpoint so `check` can actually work?” · tell · tell · tell · tell · tell

## What the people said

- Sam, `sam-explore` run 1: “I typed `canon why ...` and `canon log`, but the help pager was still open and just swallowed them — the line sat there unrun until I pressed Enter. And `canon add` "Guests may stay a month": I expected it to be a proposal waiting for someone, but it went live immediately.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
