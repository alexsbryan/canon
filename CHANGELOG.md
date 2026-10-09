# Changelog

What changed in each release of canon, newest first. A change you would notice
lands here in the commit that makes it, under Unreleased. Cutting a release
renames that heading to the version, and its section becomes the release notes
([Releasing](./docs/CONTRIBUTING.md#releasing)).

## Unreleased

- `CANON_API_KEY` is sent as a bearer token, so canon can use a hosted
  endpoint that requires a key. It's read from the environment only; `config
  show` says whether it's set without printing it.
- Claude works: an endpoint at `api.anthropic.com` is spoken to in
  Anthropic's own API, which enforces the schema. Anthropic's "overloaded"
  (529) is waited out like a 429.
- The core compiles with `no_std` + `alloc`. Local and CI gates check its
  isolated runtime dependencies and features, alongside the workspace checks.
- [The contract](./docs/CONTRACT.md) names the guarantees, executable checks,
  and assumptions. Generated replay, merge, and ratification checks shrink
  failures and preserve regression seeds.
- Compatibility is computed against a pinned git baseline using its tests,
  fixtures, expectations, generated old consumers, and old-record comparisons.
  CI runs the evaluator from the PR base branch. Reports distinguish preservation,
  extension, observed breaks, and evidence that is not established; contributors
  link the report rather than choose a classification.

## 0.2.0 — 2026-10-03

### Before you upgrade

Two changes alter what an existing canon says. A canon where only people
wrote rules, and nobody retracted a rule they had no seat over, reads the same
as before.

- An agent's write into a scope nobody holds is a proposal now, not a rule.
  Any person's `canon approve <id>` makes it one. Rules an agent wrote before
  anyone was granted anything will show in `canon list` as proposed.
- A retraction by someone without a seat over the rule is a proposal now,
  where it used to be refused. It waits for approval from someone who could
  have retracted the rule themselves, so retractions refused before will show
  in `canon list` as waiting.

### Founding and governing a canon

- `canon <verb> --help` and `canon help <verb>` say what a verb does and what
  it takes. `canon init --help` says what it writes.
- `canon who` with no scope says who holds the canon as a whole, and a
  proposal waiting on them names the scopes that count: "holds this canon by
  holding arch or procedure". There is no scope named for the canon itself.
- `canon who <scope>` for a scope nothing uses says so and lists the scopes in
  use, rather than only suggesting you grant it.
- `canon adopt --paste --except <id>,… -m "<why>"` leaves rules behind as you
  fork. They are recorded as retracted, with your reason.
- `canon retract`, `canon why` and `canon approve` take the id a rule had
  upstream, as the pasted block printed it.
- `canon approve` works on a proposed change to how a scope decides. It used
  to answer its own hint with "no commitment matching".
- A refused act leads with NOT APPLIED and names the act. An act that took
  only because nobody held the scope yet says `applied while open`.
- `canon grant` names who granted, and says when you seat someone else while
  holding nothing yourself.
- `canon who <scope>` names both rules: how rules are made there, and how
  proposals are judged.
- `canon list` counts acts with no person behind them by kind, instead of
  printing every id on one line, and no longer counts an agent placing its own
  proposal in a scope.
- `canon init` adds `config` to `.canon/.gitignore`, so a new canon does not
  commit one machine's endpoint.

### Reading a canon without canon

- `canon list --markdown` renders what is in force as a document, grouped by
  scope with ranked rules first. `--out <file>` writes it, and `--check` exits
  1 when that file is stale. It carries no date, so a committed copy goes
  stale only when the canon changes.
- `canon list --json` carries `scope` and `rank` on each commitment.
- Over MCP, `canon_list` shows counts by scope first once a canon passes fifty
  rules, and takes `scope`, `query`, `limit` and `offset`.

### Drafting

- `canon draft --continue <run.json>` finishes a run that failed after
  extraction, from its recorded candidates, without reading a passage again.
- The number check can no longer run until the endpoint times out. Schemas go
  to the model in the order they are written; sent sorted, the model could
  fill a field with a copy of the prompt. Each reading is also capped, a block
  that fails is retried one pair at a time, and pairs it could not read are
  listed as `support_unchecked` for review.
- `canon draft --resume` offers every finished run with candidates left,
  oldest first.
- Markdown emphasis comes out of a candidate's text, and code symbols stay
  whole.
- Progress overwrites one line in a terminal and prints plain lines to a log.
- A chat export with timestamps, or a git commit, records in the run when each
  passage was said. Accepting a rule still writes its act at review time.

## 0.1.0 — 2026-09-08

The first release: one `canon` binary for macOS, Linux and Windows, and
`install.sh` to put it on your PATH.
