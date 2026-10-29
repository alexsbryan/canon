# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / [--governs all|commitments] whether a change of policy waits too / [--personal on|off] canon-wide: a person's own scope is theirs

## Who, and how often

- `bikes`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `bikes` run 1, Sam: failed, ran out of patience. their last acts: type “canon log
” into “Command line” · scroll · type “canon approve can-d65bd31c15e2 -m "Mira's wording matches the hall and the drive; approved as written. Blocked car asks in chat before the mover needs out."
” into “Command line” · tell · tell · tell
- `bikes` run 1, Mira: passed, did the scene, and said they were done. their last acts: type “canon help add
” into “Command line” · type “canon add "Bikes and cars: bikes go on the wall rack in the back garden, not the hall. Whoever's car is in the drive moves it if someone needs to get out; the car that is blocked gets asked first in the house chat. Nothing is left in the hall overnight." --scope house -m "hall full of bikes, second car blocks the drive" -m "Sam asked"
” into “Command line” · say “Mira here. Sam — bikes and the blocked drive: I've drafted it, and this time it's a proposal, not a rule. `can-d65bd31c15e2`: bikes on the wall rack in the back garden, nothing left in the hall overnight, and whoever's car is in the drive moves it when someone needs out — ask in the chat first. Needs June and you to approve (`canon approve can-d65bd31c15e2`) or object with a reason. Change the wording if it doesn't fit the drive.” · tell · tell · tell

## What the people said

- Sam, `bikes` run 1: “A way to confirm what I'd just done. After typing `approve` I wanted the act's own receipt — the new id, judged under which rule, in force when — and instead the screen stayed on my old `log` output. `canon why <id>` would have done it if I'd had the new id, but nothing told me what it was. Closest thing was re-running `list` and eyeballing "1 proposed" becoming "4 rules live," which is the kind of quiet confirmation I don't trust.”
- Mira, `bikes` run 1: “A way to tell Sam what the proposal *is* without him opening a terminal. Canon gives me the id and the approve/object lines, but it doesn't put the wording anywhere the chat can see it — I had to retype it myself and trust I got it right. And the terminal wasn't broken this time, but I'm never sure it won't be, so I check `canon list` before I trust anything.”

## The product source

- `docs/COOKBOOK.md:464` holds “1 proposed”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
