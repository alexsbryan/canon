# Try rules that belong to the group

The optional institution reader keeps an adopted rule under your group's
agreed procedure, including when its original author asks to remove it.

This is an experimental, source-built preview. Everyday `canon` commands
still use their existing interpretation. You can try this reader separately
against a record; it reads the files without changing them.

## Try a house's decisions

You need a checkout of canon, the Rust toolchain, and Python 3. Run these
commands from the checkout's root. [Contributing](./CONTRIBUTING.md#getting-set-up)
covers getting the source and toolchain. No model or endpoint is needed.

```sh
cargo test -p canon-institution
python3 examples/institution-house/walkthrough.py
```

```text
2020-01-03  proposed   Helper proposes quiet hours.
2020-01-04  proposed   Mira agrees; Sam has not agreed yet.
2020-01-05  active     Sam agrees; the rule takes effect.
2020-01-06  active     Helper asks to remove it; the rule stays.
2020-01-07  active     Mira agrees to removal; it still needs Sam.
2020-01-08  retracted  Sam agrees to removal; the rule ends.
Six views checked. The example record and starting agreement are unchanged.
```

Mira and Sam share a house. They start with a seat each and a procedure:
both must agree to changes to the house's rules. Helper, an agent, drafts
“Quiet hours are 10pm to 7am.” It becomes a house rule when Sam adds the
second approval on January 5.

On January 6, Helper asks to remove it. The house rule stays in force until
Mira and Sam both agree to removal. Writing the original proposal gave Helper
no continuing power to remove the house's adopted rule alone.

That is what *institution* means here: an agreed way to decide who can change
what. The record keeps the requests and approvals; the reader works out which
changes took effect, and when.

The [example](../examples/institution-house/README.md) is invented and uses
unsigned names. Its script checks the actual reader's answers at six dates,
including the two effective times, and checks that both input files stay
unchanged. It reads the same complete record each time; later approvals do
not count in earlier views.

## Look at one decision

```sh
./target/debug/canon-institution \
  --arrangements examples/institution-house/starting-agreement.json \
  --log examples/institution-house/record/acts.jsonl \
  --at 1578268800
```

This is January 6, 2020, at midnight UTC. The quiet-hours entry,
`can-b540f8eee068`, is `active` in `state.commitments`. Helper's removal
request, `can-05109bf7ce1c`, has this decision in `admissions`:

```json
{
  "source": "can-05109bf7ce1c",
  "status": "proposed",
  "effective_at": null,
  "requirements": [
    {
      "scope": "house",
      "rule": {"rule": "joint", "holders": ["human:Mira", "human:Sam"]},
      "verdict": {
        "verdict": "proposed",
        "needs": "approval from human:Mira, human:Sam"
      }
    }
  ]
}
```

Approvals name the particular request. Approving quiet hours is different
from approving their removal. `scope` names the room this decision affects;
`joint` means all the named people must agree.

Try the same command with `--at 1578441600`, January 8. The removal is now
`admitted`, with that timestamp as its `effective_at`. The quiet-hours entry
is `retracted`; its wording and history remain in the record.

`--at` takes Unix seconds, not a date string. These examples use midnight
UTC. Keep the arguments in the order shown: `--arrangements`, `--log`, `--at`.

### Reading the rest of the answer

| Field | What it tells you |
|---|---|
| `interpretation` | Which reader produced this view: `shared-assent-v2`. |
| `at` | The time you asked about. |
| `admissions` | Decisions on requests considered under this agreement: `proposed`, `admitted`, or `refused`, with their requirements. |
| `effective_at` in a decision | When an admitted change took effect; `null` if it has not. |
| `state.commitments` | Adopted rules, including ones later withdrawn or replaced. Check each entry's status. |
| `pending` | Rule proposals that have not been admitted, with their status. Other requests, such as removals, appear in `admissions`. |
| `effects` | Source entries applied in this view, each with its effective time. |
| `source_sha256`, `arrangements_sha256` | Fingerprints of the interpreted record and starting agreement. Keep both inputs to reproduce a result. |

A rule keeps its original `asserted_at`: January 3 in this example. Its
admission's `effective_at` is January 5. Those answer different questions:
when someone wrote it, and when the house agreed to it.

Exit status `0` means the reader completed. Read the individual decisions:
a completed reading can contain pending or refused requests. Exit status `2`
means it could not judge; the reason is printed on standard error.

## The starting agreement

The reader takes two inputs: your record and an explicit statement of what
the group accepts as already in place. In this example, that statement is
[starting-agreement.json](../examples/institution-house/starting-agreement.json):

```json
{
  "boundary": 1577923200,
  "effects": [
    {"source": "can-73fc0e52ac35", "effective_at": 1577836800},
    {"source": "can-84f756e2064f", "effective_at": 1577836800},
    {"source": "can-59115926b1a5", "effective_at": 1577923200}
  ],
  "pending": []
}
```

`boundary` is January 2, when this reading starts. Each `source` names an
entry in the [record](../examples/institution-house/record/acts.jsonl): Mira's
seat, Sam's seat, and their shared decision procedure. `effective_at` says
when that starting fact took effect. `pending` lists any existing proposals
still awaiting a decision at the boundary; this house has none yet.

Listing those entries means “we accept these as our starting facts.” The
reader checks that the references are usable; the group supplies the agreement
behind them. An empty agreement cannot establish who gets the first say.
Keep this file with the record, and ask for a view at or after its boundary.

## Trying it with your own record

Make a separate copy of your record and write a starting-agreement file that
reflects what your group actually accepts. Use full source IDs from that
record, and distinguish adopted facts from proposals still waiting. This
preview has no automatic conversion from an existing canon into that agreement.

Pass those two paths to the reader in the command above. Re-running a view
writes neither input and does not switch the interpretation used by `canon
list`, the browser UI, or MCP. The diagnostic executable is built from source;
the normal installer installs `canon`.

The preview handles explicitly named rooms where people already hold seats.
When a change affects several rooms, every required room's
procedure must pass. Changes to the decision procedure use the old procedure;
already-filed requests keep their original requirements. The
[technical guide](../crates/canon-institution/README.md) describes the supported
profile and its limits, including transfers and unsupported changes.

Names and signatures have the same trust boundary as the underlying record.
The [identity guide](./GUIDE.md#who-gets-a-say) explains binding a key to a name.
This reader can check recorded authority and assent; the people using it still
have to agree that the arrangements are theirs.

For integration details, use the
[library guide](../crates/canon-institution/README.md). Its temporal API is
`interpret`; the runner's `--arrangements` selects it. The older `project` /
`--basis` interface is the fixed-basis `shared-assent-v1` compatibility reader.
The [boundary design](./INSTITUTION_BOUNDARY.md) explains how the optional
interpretation sits above canon's record mechanisms.
