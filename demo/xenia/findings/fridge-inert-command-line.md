# A tap changed nothing: /, “Command line”

- status: open
- found by: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T01-15-58-238Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `d2df323`, the product `7d1ae8e`
- last seen: the set `/Users/alexsbryan/dev/xenia/runs/2026-10-30T18-38-33-385Z`, 1 run(s) by deepseek-flash, through deepinfra (https://api.deepinfra.com/v1/openai), xenia `87cf6e3`, the product `cd168e3`
- kind: hard (the instrument's): `inert`; never both

## Where

/, the element “Command line”. What was read there first: Command line / Enter / Ctrl-C / Ctrl-D / undo <act-id> [-m "<reason>"] revert an act — yours, or one you hold / question "<text>" record what the canon does not cover

## Who, and how often

- `fridge`: 1 of 3 accounts (Sam run 1)

## What happened

- `fridge` run 1, Sam: passed, did the scene, and said they were done; brought back by a message, then did the scene, and said they were done. inert {"tap":"Command line"}; their last acts: type “pwd; ls; ls dev 2>&1 | head” into “Command line” · look · type “ls -la; echo HOME=$HOME; ls "$HOME"; canon --help 2>&1 | head -40” into “Command line” · type “ls -la” into “Command line” · type “pwd && echo hi” into “Command line” · tap “Command line”

## What the people said

- Sam, `fridge` run 1: “reached for: A shell I could type into, first. Then a way to read the log directly — one command, verbatim acts out, not a summary. I tried `ls -la .canon`, `cat .canon/profile`, a grep for the pmset line in the demo house file; none of it ran, because every keystroke went to the dialog's text box. The thing I actually needed was for the unlabelled prompt to not exist over my command line, or at least to say what it wanted. A permission dialog that won't name its requester is the whole problem in one box.”

## The product source

- `demo/xenia/README.md:146` holds “Command line”
- `demo/xenia/README.md:265` holds “Command line”
- `demo/xenia/README.md:271` holds “Command line”
- `crates/canon-cli/src/sources/tests.rs:441` holds “Ctrl-C”
- `crates/canon-cli/src/ui.rs:145` holds “Ctrl-C”
- `demo/xenia/README.md:249` holds “Ctrl-C”

## To reproduce

```sh
cd /Users/alexsbryan/dev/xenia && node src/run.ts run /Users/alexsbryan/dev/canon/demo/xenia/houses --runs 1 --model deepseek-flash --findings /Users/alexsbryan/dev/canon/demo/xenia/findings --source /Users/alexsbryan/dev/canon
```
