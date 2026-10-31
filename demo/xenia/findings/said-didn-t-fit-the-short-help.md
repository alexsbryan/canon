# Didn't fit together: the short help

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "didn't fit"; never both

## Where

the short help. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $ canon --help

## Who, and how often

- `mira-explore`: 1 of 1 accounts (Mira run 1)

## What happened

- `mira-explore` run 1, Mira: passed, explored, and said they were done. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- Mira, `mira-explore` run 1: “Three things didn't fit.
1. The help says nobody writes rules one at a time, and `draft` is meant to read a folder of notes. But what I found was entered by hand: every act in the log is by Sam, all dated today. I don't know whether it was drafted from our notes or typed in.
2. The log has things that aren't rules: "silence" on "a chore rotation", "ratify standing", and an open question about the plants. The help doesn't explain them in what I read, and I skimmed some of it. `canon help all` mentions "who decides what", "what has gone stale" and "drawing lots", but I didn't get to those sections.
3. `canon why` on the quiet hours rule says only "written by Sam, in force". It has no reason, so not every rule explains itself. Only the one that changed does.”

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
