# A tap changed nothing: /, “cat house/situations.md”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “cat house/situations.md”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `setup`: 1 of 3 accounts (Sam run 1)

## What happened

- `setup` run 1, Sam: could-not-judge, the scene ran out of time while they were acting. inert {"tap":"cat house/situations.md"}; their last acts: say “(1/4) canon — the rules you already have, and what was decided about them. Commands: init [--profile personal|code|house] start one in this directory; draft --from <folder> read what you already have; add "<text>" [-m <why>] [--cite <where>] write one down; check "<proposal>" does it clash with one?; list; why <id>; log the raw acts, oldest first.” · say “(2/4) draft/check need a model (canon config set endpoint <url>; a model does the reading). config show / config set endpoint|model|extract_model. Flags: --json, -m <reason>, --allow-remote (endpoint must be on this machine otherwise).” · say “(3/4) ENV: CANON_ACTOR (who is acting; default git user.name prefixed human:; a name bound to a key also needs that key to sign), CANON_SIGNING_KEY, CANON_DIR, CANON_ENDPOINT, CANON_MODEL, CANON_API_KEY.” · say “(4/4) Personal profile note: canon is a structured journal, not a clinician; check never returns a verdict; a deliberate contradiction is a first-class state, never force-resolved. That's the paste. Waiting on the situations.md list before init.” · type “cat house/situations.md” into “Command line” · tap “cat house/situations.md”

## What the people said

None.

## The product source

- `demo/xenia/README.md:143` holds “Command line”
- `demo/xenia/README.md:203` holds “Command line”
- `demo/xenia/README.md:209` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:187` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
