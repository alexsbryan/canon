# A tap changed nothing: /, “canon help all”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “canon help all”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / noticing." / $ canon add "The bathroom is cleaned on a rota: one person each week, named here, covering shower, sink, toilet and floor; anyone who can't take their week swaps it here, not only in the group chat."

## Who, and how often

- `fridge`: 1 of 3 accounts (June run 1)

## What happened

- `fridge` run 1, June: passed, did the scene, and said they were done. inert {"tap":"canon help all"}; their last acts: look · tap “Command line” · type “canon help all
” into “the input at the bottom of the terminal” · look · type “canon help all
” into “Enter” · tap “canon help all”

## What the people said

- June, `fridge` run 1: “reached for: A field to type commands into. The terminal shows "Command line / Enter / Ctrl-C / Ctrl-D" at the top but I couldn't find a place to type — I tried the label, "Enter", even "Command", and nothing took. I only got `canon who` and `canon list` through by typing into "Command" — and both were guesses at the right name. Also: no clear "where does this canon actually live / can I move it" command. pwd told me the folder was wrong but not how to fix it, and the leftovers question had nowhere to go but talk.”

## The product source

- `crates/canon-cli/src/cli/tests.rs:266` holds “canon help all”
- `crates/canon-cli/src/cli.rs:95` holds “canon help all”
- `crates/canon-cli/src/cli.rs:145` holds “canon help all”
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
