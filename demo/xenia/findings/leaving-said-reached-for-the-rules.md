# Reached for: the rules

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ can-16df375f7a6b Spare room: it is an office on its claimed days, and anyone else who needs it books it in the house chat the day before, saying which hours. Whoever booked it has it for those hours; whoever wants it outside the office days and hasn't booked gets it only if nobody has booked that time. If two people want the same hours, the one who asked first has it and the other says so in the chat. Office days are fixed and whoever holds them says in the chat when they change. / PROPOSED, not yet a rule — needs approval from human:June, human:Sam

## Who, and how often

- `leaving`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `leaving` run 1, Sam: passed, did the scene, and said they were done. their last acts: scroll · type “canon list
” into “Command line” · say “Sam here, correcting myself rather than leaving it in the air: June's approval did land — can-ea37cbb42955, "human:June position toward can-16df375f7a6b", log now 21 acts. So I was wrong when I said June never approved; my log read was the stale one, at 20. The spare room is live by joint: all three of us voted, and `canon list` shows it. Nothing bypassed joint — I jumped at a mismatch that was my own reading, and I'd rather say that than let a false alarm stand.

Still true and still unwritten: the spare room rule never states which days the office days are, so Mira should amend that. Bikes/cars still names two movers. And notice/deposit is still a gap — my shape stands: a month's notice in the chat, deposit back within two weeks of the room being cleared, and any deduction named with a receipt in the chat before money is held.” · tell · tell · tell
- `leaving` run 1, Mira: passed, did the scene, and said they were done. their last acts: type “canon list
” into “Command line” · scroll · say “Correction to my last: I read `canon list` properly now — the spare room (can-16df375f7a6b) is live, 5 rules, not waiting on you. Good.

So the count is: guests, bathroom rota, kitty, bikes/cars, spare room. Moving out is the gap — notice and deposit, nothing there. Sam, give me the shape you want and I'll draft it for June and you to approve.” · tell · tell · tell

## What the people said

- Sam, `leaving` run 1: “A way to see the current act count without a fresh read — something in the prompt telling me the log grew under me. I read the log, got 20, and treated that as the state; when June's vote arrived I had no signal it had, so I reported a gap that had already closed. A watermark, or "canon log" printing "new acts since your last read", would have saved the false alarm.

Also a per-rule trail in one place: `canon list` saying "5 rules live" but not which positions carried each. I had to walk the log by hand to prove all three voted. And a way to see a rule is live *and* which of its clauses are unresolved — the office days and the two-mover sentence are known defects in text that reads as settled.”
- Mira, `leaving` run 1: “A way to list just the pending proposals. `canon list` mixes live rules and PROPOSED ones in the same scroll, so I read a stale PROPOSED line and told the house my spare room was still waiting when it had already gone in. I'd want `canon list --proposed` or a status column, plain. And a blank-rule answer for deposit/notice — the canon can hold "these people owe each other money" only as prose.”

## The product source

- `crates/canon-cli/src/cmds.rs:851` holds “canon log”
- `crates/canon-cli/src/cmds.rs:871` holds “canon log”
- `crates/canon-cli/src/cmds.rs:878` holds “canon log”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
