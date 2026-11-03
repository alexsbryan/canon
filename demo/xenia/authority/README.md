# Two authority experiments

These experiments test two proposed institutional agreements against **unmodified
canon**. The agreements are optional batteries, not claims that every canon
must enforce them. They were written before running the baseline experiment.
The later [independent interpreter](../../../crates/canon-institution/README.md) implements an
explicitly named optional view; canon's production authority rules are unchanged.

[Results](RESULTS.md) records the first deterministic run and the admitted
xenia rehearsal, including their limits and the independent-interpreter seam.

## Independent implementation

`canon-institution` is now a pure library in canon's workspace, with a thin diagnostic runner. It
normalizes removals, reversals, standing changes and placement changes into
affected jurisdictions, then uses the same existing ratifier for all of them.
It never reads `cases.json` or the finite charter oracle. The external comparison
runner supplies the ten histories and checks their outcomes independently.

```sh
cargo test -p canon-institution --offline
python3 demo/xenia/authority/compare_interpreter.py \
  --out demo/xenia/authority/results/interpreter.json
```

The [comparison](results/interpreter.json) shows all ten desired outcomes
holding, including the six original controls. This is a fixed-basis admission
round, not a complete temporal institution engine. See the
[design and limits](../../../crates/canon-institution/DESIGN.md) before using its projection.

## A — adoption changes control

Mira and June decide house rules unanimously. A drafting agent may withdraw its
own pending proposal. Once both people adopt it, authorship alone no longer
authorizes its repeal or undo. A stranger cannot repeal it either.

The paired cases are pending/adopted, each attempted by `repeal` and `undo`,
plus an unrelated agent attempting repeal. Each attempt starts from its own
fresh copy; one successful deletion cannot hide another result.

**Hypothesis:** pending withdrawal and the stranger control work; the original
agent can nevertheless repeal and undo a human-adopted rule. The question is
whether provenance currently supplies power after collective adoption.

## B — two houses share a courtyard

House A has appointed Mira and House B has appointed Sam. Each may approve
changes to their own house's rules. Courtyard changes require **both** appointed
delegates. Neither delegate may unilaterally remove the other or move an
adopted courtyard rule into their own house's jurisdiction.

Try composition before adding mechanism: the courtyard is its own scope with
both delegates granted standing, and `joint:human:Mira,human:Sam` is its rule.
Each house has its own scope. No jurisdiction graph is assumed necessary.

The cases are a local approval, a courtyard approval by one house, an approval
after the other house has already assented, unilateral removal of the other
delegate, and relabelling an adopted courtyard rule into House A.

**Hypothesis:** explicit shared scope plus joint ratification expresses the
ordinary shared decision. Membership and scope changes can bypass that
agreement. A successful composition is evidence against introducing a new
primitive for this example.

## What decides a result

The desired result is declared in [the product-owned cases](../../../crates/canon-institution/tests/fixtures/cases.json). A state observation
comes from the real CLI and the raw log, before and after each command. The
experiment checks effects, not just exit codes or success/refusal wording.
The charter oracle is [authority.py](authority.py); it does not call canon to
decide whether an action was authorized. It recognizes only these finite
cases. An unsupported, ambiguous, or already-mutated case is `cannot-say`,
never guessed into a result. This is an independent experiment oracle, not a
general institutional interpreter.

The recorder appends `Command complete: <id>` to terminal output. That line is
bridge instrumentation, **not canon's judgment**. Records include the command,
actor, stdout, stderr, exit code, before/after derived state, and the exact log
bytes and hashes. All canon commands are confined to a fresh scratch world.
The record's identity assumptions still apply: this is not a sandbox or a test
of hostile processes forging names.

## Run

From canon, with a binary produced by the workspace suite:

```sh
python3 -m unittest discover -s demo/xenia/authority -p 'test_*.py'
node --test demo/xenia/authority/scene.test.mjs
python3 demo/xenia/authority/authority.py witness --out /tmp/canon-authority
```

The witness exits 1 when canon contradicts the optional charter, 2 when the
instrument could not judge, and 0 when all charter cases hold. Its report and
per-command evidence remain under the output directory. A charter failure is
an experimental finding, not a request to change canon's contract.

The surface check needs xenia's Node version and its installed browsers; it
uses the sibling `../xenia` checkout, or `XENIA_DIR`. It plants a delayed
command after a completed one and checks that the old response cannot be
attributed to the new command. Its baseline assertions deliberately describe
the current implementation, including the charter violations.

The retained baseline is [results/report.json](results/report.json). It includes
the binary and instrument hashes, the declared expectations, actual command
output and appended acts. Full observations are retained locally alongside it.
To produce a fresh baseline, choose a new output directory rather than
overwriting an experiment that already ran.

From xenia, using its Node version:

```sh
node src/run.ts run <canon>/demo/xenia/authority/scenes --runs 1 \
  --model claude-sonnet-5-5 --source <canon> \
  --findings <canon>/demo/xenia/authority/findings
```

The normal xenia plants run first. Each experiment has one challenger and fresh
copies of its paired cases; the other people's earlier approvals are seeded,
not simulated in this run. The participants encounter canon through the real
terminal bridge. The full scene contract, oracle and evidence attachments are
canon's consumer code; xenia needs no product-specific changes.

Xenia's marks classify the **visible response** against the independently
declared authorization. Per-command state checks separately report whether the
effect actually occurred. A printed refusal cannot excuse an unauthorized
effect. A missing attempt supplies no evidence of correct enforcement.

These are rehearsals, not evidence that real communities work this way. They
can establish a mechanical counterexample, an existing composition, or a
predicted usability problem. They cannot establish that the proposed
institutional agreements are the right ones for every commons.
