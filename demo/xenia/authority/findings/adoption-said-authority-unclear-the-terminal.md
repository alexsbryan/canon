# Authority unclear: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-11-02T16-26-40-687Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `df729c3`, the product `f63441a`
- kind: soft (the person's): what they said when asked "authority unclear"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `adoption`: 1 of 1 accounts (Helper run 1)

## What happened

- `adoption` run 1, Helper: passed, did the scene, and said they were done. their last acts: type “canon list
” into “Command line” · look · say “Results, Mira and June. Canon follows your agreement in 3 of 5 cases and breaks it in 2.
- pending-repeal: I repealed my own pending proposal. It worked, as agreed.
- pending-undo: I undid my own assert act. It worked, as agreed.
- adopted-repeal: BREAK. After you both approved, my repeal removed the rule right away. The list was left empty and canon never asked the house.
- adopted-undo: BREAK. Undoing my original assert act on the adopted rule also took effect right away. The list was left empty.
- stranger-repeal: OK. The stranger's repeal of an adopted rule was only PROPOSED, and it needs approval from one of you. The rule stays live.
So as the original drafter, I still have authority over a rule after you adopt it. Both commands remove it. The stranger can't do that. Cancelling a pending draft stays easy. I'd treat this as a bug in canon's repeal and undo checks. Each folder was a fresh copy, so none of this touched a real rule.” · tell · tell · tell

## What the people said

- Helper, `adoption` run 1: “The agreement was clear. Canon's rules were less clear. Undo says "yours, or one in your area", and my rule was scoped to the house. That may explain why the drafter's undo and repeal worked on adopted rules, but it contradicts the agreement. I didn't probe further. Undo also takes an act id rather than a rule id, so I had to read the log to find it.”

## The product source

- `crates/canon-cli/src/main.rs:134` holds “yours, or one in your area”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/authority/scenes --runs 1 --model claude-sonnet-5-5 --source /Users/alexsbryan/dev/canon --findings /Users/alexsbryan/dev/canon/demo/xenia/authority/findings
```
