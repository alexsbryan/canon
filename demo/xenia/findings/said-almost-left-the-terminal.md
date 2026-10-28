# Almost left: the terminal

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "almost left"; never both

## Where

the terminal. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `june-join`: 1 of 1 accounts (June run 1)
- `sam-join`: 1 of 1 accounts (Sam run 1)

## What happened

- `june-join` run 1, June: failed, ran out of patience. their last acts: tap “Enter” · tell · tell · tell · tell · tell
- `sam-join` run 1, Sam: failed, ran out of patience. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- June, `june-join` run 1: “Yes. When the screen filled with bash help text and I couldn't scroll back to or read the end of the file, I nearly stopped there. I kept going because I wanted to read the adopt line before running anything. In the end I couldn't read it.”
- Sam, `sam-join` run 1: “Yes, near the end. The screen is tiny, and the long help output scrolled past with "There's more you skimmed past." I couldn't read all of it. Then the session ended after my init command, so I never saw its output.”

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
