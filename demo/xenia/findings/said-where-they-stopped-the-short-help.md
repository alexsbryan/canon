# Where they stopped: the short help

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T16-03-37-837Z`, 1 run(s) by claude-sonnet-5-5, as claude -p, xenia `d7600a5`, the product `0f15ca8`
- kind: soft (the person's): what they said when asked "where they stopped"; never both

## Where

the short help. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $ canon --help

## Who, and how often

- `mira-explore`: 1 of 1 accounts (Mira run 1)

## What happened

- `mira-explore` run 1, Mira: passed, explored, and said they were done. their last acts: tap “Enter” · tell · tell · tell · tell · tell

## What the people said

- Mira, `mira-explore` run 1: “I stopped after `canon why` on the guests rule. That answered what I wanted: what is live, and why one rule changed. I didn't run `add`, `check`, `draft` or `init`. They would change the record, or they need a model endpoint that I don't know is set up. I'd rather ask you than guess in the house's shared record. I also didn't read the sections of `canon help all` on who decides, staleness and drawing lots. I'd read those before using `add`.”

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
