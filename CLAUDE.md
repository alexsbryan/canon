# Working in canon

`./scripts/pre-push.sh` is the gate: rustfmt, clippy, the suite (the contract
files included), docs links and the ledger. Run it before you say a change is done.

## The contract

canon promises other implementations and every existing record that it keeps
the laws in [docs/CONTRACT.md](docs/CONTRACT.md). The promise is also written
down in `contract/`: canon-core's public surface (`api.txt`), every verb and
flag (`cli.txt`), and what the fixtures decide and mint (`records/`).
`cargo test` fails when the code and those files part.

- When a test says `contract/<file> is not what the code does`, the change
  you made moves canon's contract. If you did not mean it to, undo that part.
  If you did, stop and tell the user what moves (the lines the test printed);
  `CANON_BLESS=1 cargo test` rewrites the files, and running it is theirs to
  approve. A hook asks them before any edit of `contract/` or a run with
  `CANON_BLESS`.
- A record that reads differently after your change needs a migration or
  reinterpretation test, and a changelog entry under "Before you upgrade".
