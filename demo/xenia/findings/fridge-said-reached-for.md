# Reached for: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: soft (the person's): what they said when asked "reached for"; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / noticing." / $ canon add "The bathroom is cleaned on a rota: one person each week, named here, covering shower, sink, toilet and floor; anyone who can't take their week swaps it here, not only in the group chat."

## Who, and how often

- `fridge`: 2 of 3 accounts (Sam run 1, Mira run 1)

## What happened

- `fridge` run 1, Sam: passed, did the scene, and said they were done. their last acts: look · say “June — leftovers. The line I'd draw: anything with a name or a date on it is theirs; unlabelled stuff in the shared shelf is fair game, and if it's been there past a week it's fair game anyway. Labels are cheap, and "I didn't know it was yours" stops being an argument.

Same for the shopping: if you buy it, write it here — what, how much. Capsule: whoever shops logs it, so nobody's guessing who paid.

Before I write either, though, I owe you all a correction. I've been treating canon check as recall. It isn't: the same noise question got a rule name days ago, the bathroom question got a rule name plus a model's paraphrase of why. That's deepseek-flash on localhost deciding, not canon remembering. The rules and the acts are what's actually on record. So check is a second opinion, not proof — and if check tells you ALREADY COVERED, read the rule it points at.

Also still unexplained: my first canon add echoed and wrote nothing; the identical command worked the second time. If you ever see add return no rule id, re-run it and don't assume it landed.

Say whether rota/kitty for the shop or just the log, and I'll write the pair.” · tell · tell · tell
- `fridge` run 1, Mira: passed, did the scene, and said they were done. their last acts: look · say “Mira here. Two things — and one of them I need help with.

1. My canon is still in the temp folder. Sam added can-8f5fcc8a03dc as a rule my house counts as real, so it looks like the house home got settled somewhere and I missed the path. Sam: give me the exact init line for the permanent location and I'll run it here; I won't guess at one.

2. I asked the canon who pays for the shared shop. Answer: nothing we've decided bears on it — "THIS NEEDS A NEW RULE". So it's for one of us to write, and June's leftovers are the same gap: what's shared, what's labelled. Both fit one rule if we want.

I'm not writing either one alone.” · tell · tell · tell

## What the people said

- Sam, `fridge` run 1: “Three things.

A verb that tells me where the canon actually lives — one command that says "your rules are in this directory, and here's whether that directory is temp." I had to infer that from pwd and a path under /var/folders.

A way to move or relocate the canon, or at least an export. The help gave me adopt, upgrade, rebase --onto, share — all lineage. So the only protection was copying, and I had to work out that share wouldn't run until there were rules (it refuses with grants only, "no live commitments"). That surprised me at the worst moment.

And confirmation that a write landed. canon add echoed and wrote nothing the first time; I only knew because can-8f5fcc8a03dc came back on the second run. I wanted an explicit "not written" there, not silence.”
- Mira, `fridge` run 1: “A way to move my canon out of /var/folders without knowing where the house lives. And the path Sam settled on — everyone else is on that home and I'm not; I had to ask rather than find it.”

## The product source

- `crates/canon-cli/src/lineage.rs:66` holds “no live commitments”
- `crates/canon-cli/src/lineage.rs:333` holds “no live commitments”
- `crates/canon-cli/src/guard.rs:672` holds “not written”
- `crates/canon-cli/src/review/tests.rs:413` holds “not written”
- `crates/canon-cli/src/review/tests.rs:416` holds “not written”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
