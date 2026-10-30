# Too many words before acting: /

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-29T00-38-52-588Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `e095e4e`, the product `ab59c50`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- kind: hard (the instrument's): `wordy`; never both

## Where

/. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $

## Who, and how often

- `setup`: 1 of 3 accounts (June run 1)

## What happened

- `setup` run 1, June: passed, did the scene, and said they were done. wordy {"words":206,"budget":200}; their last acts: wait · say “I'm June. So: canon grant human:june house.

Two things before we build on it: that /var/folders/.../house/.canon path bothers me too — temp dirs can get cleaned out, and if that's where our rules live they could vanish. Can you check where the canon actually sits (canon help all mentions forking/merging; there may be a "where am I" way)? If it's really under /var/folders, maybe we should init in the same folder as situations.md instead. Your call — but I'd rather sort that now than write 14 rules into a temp folder.

Meanwhile I'll read canon help all so I understand granting, drawing lots, and "who decides what" before I hold a say.” · type “canon help all
” into “command line” · scroll · wait · type “q
” into “pager”

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
