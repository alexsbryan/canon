# A tap changed nothing: /, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / CONFIGURE / config show what this canon is configured with

## Who, and how often

- `guests`: 1 of 3 accounts (Sam run 1)

## What happened

- `guests` run 1, Sam: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: look · type “pwd; ls -la; ls ..” into “Command line” · type “pwd; ls -la” into “Command line” · tap “Command line”

## What the people said

- Sam, `guests` run 1: “reached for: A way to copy or move the canon — no export, no move, no "where am I" verb. Relocation is only expressible as share/adopt/upgrade/rebase lineage, and share refuses outright while the canon holds only grants and no rules, so the one safety net I could plan for the temp folder doesn't exist yet exactly when I need it. I also wanted a plain view of who holds what: grants don't appear in canon list, only as a line of output when granted.”

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
