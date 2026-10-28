# Didn't fit together: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "didn't fit"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `sam-explore`: 1 of 1 accounts (Sam run 1)

## What happened

- `sam-explore` run 1, Sam: passed, explored, and said they were done. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- Sam, `sam-explore` run 1: “The silence act. `canon log` lists can-175d61adcad6 as `silence "a chore rotation"`, but `canon why can-175d61adcad6` returns "error: no record matching". Every other id in the log worked with `why`. Also, `canon list` only shows asserted rules. Silence and question acts don't appear there, so nothing in `list` says the chore rotation was silenced. The top-level help doesn't mention silence either, so I couldn't tell what it does. Smaller point: the log shows the original guest rule (can-7feb9586c3af) as asserted. `list` correctly omits it, but `why` on the new rule is the only place that says it was replaced.”

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
