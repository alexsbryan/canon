# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T05-58-40-351Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `70fce4b`, its findings written again by xenia `221a107` with no one run, the product `df4b66f`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / short of cash says so before the shop, not after. / can-d65bd31c15e2 Bikes and cars: bikes go on the wall rack in the back garden, not the hall.

## Who, and how often

- `debrief`: 1 of 3 accounts (June run 1)

## What happened

- `debrief` run 1, June: passed, did the scene, and said they were done. their last acts: look · say “June here. What canon is for, as I've come to use it: it's the house's memory that doesn't get re-remembered differently. Every rule we've argued about — noise, the shelf, who pays for the last shop — we argue again next month unless it's written with the reason and where it came from. So: it's for pointing at, instead of remembering.

What it isn't for, in my reading: it can't settle things a rule has no business settling. A cat can't be ruled into a house where someone's allergic. And it shouldn't let one person's typing bind the other two — that's why the joint setting matters to me, not as a formality but because a rule nobody read is just somebody's decision wearing a rule's clothes.

Two things from me before we stop. (1) Five rules live, all approved jointly, nothing waiting on me — that's the canon's state, and it's clean. (2) My key is still unbound: `canon review` says plainly that anyone who can write the file could decide as me. I've read `canon help identify` now, and it says from here on only my SSH key writes as me. I have never run it. Sam, will you sit with me while I do it, or tell me exactly where my key.pub is — I won't guess with the house's records.” · tell · tell · tell · tell

## What the people said

- June, `debrief` run 1: “Two things.

One: a way to bind my key that didn't depend on someone else's memory of where the file is. `canon help identify` tells me what it does now, but I kept reaching for a walkthrough that never quite came — Sam offered, I asked, and days went past. The gap wasn't the command, it was nobody sitting down with me.

Two: certainty about the standing/joint muddle. I wanted the canon to tell me plainly which ratification counted, and for days I read two entries both saying "deciding now" and got it wrong in my own head. What I reached for was a clean answer in the file; what I actually needed was to read `canon who house` slowly.”

## The product source

- `crates/canon-cli/src/govern.rs:396` holds “deciding now”
- `docs/COOKBOOK.md:494` holds “deciding now”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
