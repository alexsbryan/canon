# A tap changed nothing: the rules, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T02-56-03-707Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `ed1d9ea`, the product `74265cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

the rules, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / CANON_ENDPOINT override the configured endpoint for one run / CANON_MODEL override the configured model for one run

## Who, and how often

- `parties`: 1 of 3 accounts (June run 1)

## What happened

- `parties` run 1, June: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: look · type “canon log --oneline 2>/dev/null | tail -20; echo ---; cat situations.md 2>/dev/null | head -40” into “Command line” · wait · look · look · tap “Command line”

## What the people said

- June, `parties` run 1: “reached for: A way to move the house. That's the whole thing blocking me. The help says CANON_DIR overrides where canon looks, but there's nothing that says "relocate this canon," and there's no command that tells you where the canon thinks its permanent home is — I had to do it sideways with pwd. So we've got a system where writing a rule is one command and putting the rules somewhere that survives a reboot isn't a command at all. Also `canon status` doesn't exist; I tried it first, which is how I learned to start with `list`.

And a way to record "I agree but I won't sign into a temp folder" — there's no state for holding back that isn't just silence. Silence looks like the same thing as not caring.”

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
