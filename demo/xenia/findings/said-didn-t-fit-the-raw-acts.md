# Didn't fit together: the raw acts

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "didn't fit"; never both

## Where

the raw acts. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / why <id> what replaced what, when, and why / log the raw acts, oldest first

## Who, and how often

- `june-explore`: 1 of 1 accounts (June run 1)

## What happened

- `june-explore` run 1, June: passed, explored, and said they were done. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- June, `june-explore` run 1: “A few things didn't fit. 1) The log shows "assert: Guests may stay up to three nights" and then a later "supersede" with the longer wording, so I wasn't sure whether the first was replaced or still stands. List shows only the new one, so I assume it was replaced. I didn't run `why` to check. 2) Every act is by Sam and every date is today, so it looks like one person wrote the house's rules in a single sitting. I couldn't tell whether the housemates agreed to them, and "ratify standing" didn't tell me who ratified what. 3) "A chore rotation" was silenced, but I don't know whether the house decided that or Sam did. 4) The help says a `personal` canon never returns a verdict, but I don't know which profile this one uses. 5) I don't know what `draft` and `check` need from a model, or where it would run.”

## The product source

- `demo/xenia/world/scene.mjs:45` holds “Command line”
- `demo/xenia/world/scene.mjs:47` holds “Command line”
- `demo/xenia/world/scene.mjs:61` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:73` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model claude-sonnet-5-5 --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
