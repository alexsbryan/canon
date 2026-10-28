# Didn't do what they expected: a rule written or changed

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "not as expected"; never both

## Where

a rule written or changed. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / `draft` and `check` need a model; everything else folds a text file. / A canon on the `personal` profile never returns a verdict and never exits 1.

## Who, and how often

- `sam-explore`: 1 of 1 accounts (Sam run 1)

## What happened

- `sam-explore` run 1, Sam: passed, explored, and said they were done. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- Sam, `sam-explore` run 1: “Two things. First, I typed `help` and got the bash builtin list, not canon's help. I had to guess `canon --help`. That worked, but nothing told me to try it. Second, `canon why` on the silence id errored even though the log had just printed that id. I didn't try `check`, `add`, `draft` or `canon help all`, so I can't say how they behave. `draft` and `check` need a model, and I didn't want to change Mira's canon.”

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
