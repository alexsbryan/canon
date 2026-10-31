# A tap changed nothing: the rules, “Command line”

- status: open, not seen since the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-28T18-55-28-602Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `fc36366`, the product `963e1cd`
- kind: hard (the instrument's): `inert`; never both

## Where

the rules, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / $ / $ canon --help

## Who, and how often

- `june-explore`: 1 of 1 accounts (June run 1)

## What happened

- `june-explore` run 1, June: passed, explored, and said they were done. inert {"tap":"Command line"}; their last acts: type “canon --help” into “Command line” · tap “Enter” · tap “Command line” · type “canon list” into “Command line” · tap “Enter” · tap “Command line”

## What the people said

- June, `june-explore` run 1: “didn't fit: The help text itself pointed to `canon check --about "a chore rotation"`, but when I typed exactly that, the tool rejected it and told me the usage is `canon check "<proposal>"`. So the tool's own advice didn't match the tool. Once I dropped `--about`, it accepted the command — but then said it couldn't judge, because no model endpoint is configured. So the one part meant to reassure you that a silence won't be mistaken for a gap doesn't actually run here yet.”
- June, `june-explore` run 1: “not as expected: Typing `canon check --about "..."` — the very form the tool's own message suggested — errored with a usage complaint. And `canon check "a chore rotation"` didn't return a verdict either; it just said no endpoint was configured and told me to run a config set command. I didn't run that, because changing the house's configuration isn't my decision to make.”
- June, `june-explore` run 1: “where they stopped: I stopped at the config. The only way to make `check` actually work would have been to point it at a model server, and that's a change to the house's setup — Sam's to decide, not mine, and I'd rather ask than guess. I'd also finished what Sam asked for: I looked at every read-only part of canon, formed a view of what it's for, and sent him that answer in the house chat.”

## The product source

- `demo/xenia/README.md:101` holds “Command line”
- `demo/xenia/findings/findings.json:8` holds “Command line”
- `demo/xenia/findings/findings.json:13` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:86` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node /Users/alexsbryan/dev/xenia/src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/scenes --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon --debug
```
