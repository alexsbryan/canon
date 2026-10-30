# A tap changed nothing: the record, “Command line”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `inert`; never both

## Where

the record, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / can-bc0a076ce5a0 2026-10-30 human:Sam assert Guests stay three nights at most unless the house agrees otherwise, and the agreement is written here, not only in the group chat. / can-5210e4aa330b 2026-10-30 human:Sam assert Quiet hours: 10pm to 8am, no loud sound that travels off the property; log complaints here.

## Who, and how often

- `debrief`: 1 of 3 accounts (Mira run 1)

## What happened

- `debrief` run 1, Mira: passed, did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: look · type “who
” into “Command line” · type “canon who
” into “Command line” · type “canon policy show
” into “Command line” · type “canon log
” into “Command line” · tap “Command line”

## What the people said

- Mira, `debrief` run 1: “reached for: A permanent home. Ten rules now live in /private/var/folders/.../T/canon-house-l5gwOk/house, and `canon config show` only ever tells me endpoint and model — never the path. There's no "where am I" verb. Two directories with a .canon in them look identical from inside, and one gets swept. I've asked Sam for the init line every day since day 3 and it hasn't come, so I've never written a rule.

Also missing: a way for me to say "I'm not asserting this, I'm asking" that survives someone else writing over it. My question stayed open, which is right, but it only stayed open because Sam's rule didn't happen to close it — not because anything protected it.”

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
