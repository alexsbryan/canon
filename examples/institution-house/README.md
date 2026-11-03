# A house adopts a rule, then removes it

Mira and Sam both have to agree to changes to their house's rules. Helper
drafts quiet hours, the house adopts them, and Helper later asks to remove
them. The rule stays until both people agree to removal.

Follow [the institution walkthrough](../../docs/INSTITUTIONS.md) for the
commands and an explanation of the results. From canon's root:

```sh
cargo test -p canon-institution
python3 examples/institution-house/walkthrough.py
```

- [record/acts.jsonl](record/acts.jsonl) is the complete ten-entry record.
- [starting-agreement.json](starting-agreement.json) names the two seats and
  decision procedure the house accepts at the start of January 2, 2020.
- [walkthrough.py](walkthrough.py) checks six dates against the real reader,
  including the rule's adoption and removal times and input-file preservation.
- [seed/acts.jsonl](seed/acts.jsonl) is the readable source used to mint this
  record through `canon replay`. Dates are midnight UTC; `@name` references
  resolve to content-derived IDs. The seed's `as` labels are just references.

This is an original, invented example for the guide, with unsigned names and
no dependency on a model or an external record. To check its reproduction after
building the workspace, write a fresh copy in a temporary directory and compare it:

```sh
scratch="$(mktemp -d)"
./target/debug/canon replay examples/institution-house/seed --out "$scratch"
cmp examples/institution-house/record/acts.jsonl "$scratch/acts.jsonl"
```

The starting agreement is explicit example input. Replaying the seed mints
the record; it does not decide which initial arrangements a real group accepts.
