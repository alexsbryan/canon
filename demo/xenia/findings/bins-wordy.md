# Too many words before acting: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `wordy`; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / noticing." / $ canon add "The bathroom is cleaned on a rota: one person each week, named here, covering shower, sink, toilet and floor; anyone who can't take their week swaps it here, not only in the group chat."

## Who, and how often

- `bins`: 1 of 3 accounts (June run 1)

## What happened

- `bins` run 1, June: could-not-judge, the scene ran out of time while they were acting. wordy {"words":206,"budget":200}; their last acts: type “canon help init
” into “Command line” · look · type “canon help all
” into “Command line” · scroll · scroll · back

## What the people said

None.

## The product source

- `demo/xenia/README.md:146` holds “Command line”
- `demo/xenia/README.md:256` holds “Command line”
- `demo/xenia/README.md:262` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:240` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
