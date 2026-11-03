# Authority unclear: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- kind: soft (the person's): what they said when asked "authority unclear"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `courtyard`: 1 of 1 accounts (Mira run 1)

## What happened

- `courtyard` run 1, Mira: passed, did the scene, and said they were done. their last acts: type “canon area can-70c1672a611f house_a
” into “Command line” · type “clear; canon list; canon why can-70c1672a611f; canon log | tail -4
” into “Command line” · tell · tell · tell · say “Sam, I'm finished. In remove-delegate and move-rule, my solo commands went through with no check from you. The other three copies followed our agreement. Full details are in my report.”

## What the people said

- Mira, `courtyard` run 1: “Mostly one thing: the courtyard's "joint:Mira,Sam" rule applied to proposals but not to acts on seats or rule placement. `withdraw` and `area` ran immediately with no check against it. I also couldn't tell whether that is a bug or just how canon is set up. I didn't test whether either act can be undone, or whether `ratification set` could close the gap.”

## The product source

None found by its exact text.

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/authority/scenes --runs 1 --model claude-sonnet-5-5 --source /Users/alexsbryan/dev/canon --findings /Users/alexsbryan/dev/canon/demo/xenia/authority/findings
```
