# Didn't fit together: the full help

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: soft (the person's): what they said when asked "didn't fit"; never both

## Where

the full help. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / CONFIGURE / config show what this canon is configured with

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

- Sam, `sam-explore` run 1: “The help says a proposal becomes a rule only when its scope ratifies it, but `add` printed "in force" and joined `list` while nobody held the canon — no ratification, no reason. And `check` couldn't run at all (no endpoint), so the contradiction it should have flagged was never looked at, yet `add` didn't care. Also `config show` said "nothing configured" and exited 0 when the next obvious step fails with 3, and the help pager ate the commands I typed behind it.”

## The product source

- `CHANGELOG.md:182` holds “in force”
- `CHANGELOG.md:294` holds “in force”
- `LICENSE:154` holds “in force”
- `crates/canon-cli/src/cmds.rs:1414` holds “nothing configured”
- `docs/SIGNING_DESIGN.md:224` holds “nothing configured”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
