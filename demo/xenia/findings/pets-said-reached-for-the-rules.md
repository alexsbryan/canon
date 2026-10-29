# Reached for: the rules

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T16-08-41-736Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `34ea068`, the product `a5f82d1`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

the rules. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ canon ratification show / $ can-ef757ca30150 standing (whole canon) (deciding now)

## Who, and how often

- `pets`: 1 of 3 accounts (June run 1)

## What happened

- `pets` run 1, June: passed, did the scene, and said they were done. their last acts: look · look · say “June here. The cat: I won't put a rule in for this — a cat isn't a shelf or a rota, and Sam's allergy isn't something a rule can overrule. My reading: we don't get a cat while Sam lives here and is allergic; if that ever changes, it changes by all three of us agreeing, not by one of us typing it in.

Two things I can point to right now, from `ratification show` and `who house`. Good: rules made under joint:human:June,human:Sam,human:Mira — all three must agree. Bad: the old standing entry still says "deciding now" next to it, and proposals are still "judged under: default". So there are two ratification rules both claiming to be live and I don't know which one counts when someone writes. I'm not going to touch it and guess — someone who knows the tools should retire the standing one properly, and I'd like to see what `help all` says about withdrawing a ratification first.

And my key still isn't bound: `canon review` says anything signed human:June could come from anyone who can write the file. Sam, walk me through `identify` when you have a minute — I'd rather ask than guess.” · tell · tell · tell

## What the people said

- June, `pets` run 1: “A way to see which ratification actually counts. Two entries both say "deciding now" and neither says which one wins, and proposals point at something called "default" I can't find explained. I also couldn't find anything telling me how to retire a rule — I'd have to go looking in help blind, which I won't do with the house's rules. And I reached for somewhere to record "no cat while Sam is allergic, and it's not a rule question" so it stops coming round — the canon holds rules, not reasons we decided not to write one. Last, my key: still nothing that would let me identify myself without being walked through it, so anything signed human:June is still anyone's to write.”

## The product source

- `crates/canon-cli/src/govern.rs:396` holds “deciding now”
- `docs/COOKBOOK.md:494` holds “deciding now”
- `CHANGELOG.md:53` holds “default”
- `Cargo.toml:13` holds “default”
- `Cargo.toml:17` holds “default”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
