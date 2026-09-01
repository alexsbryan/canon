# The run of show

`./scripts/demo.sh` — [enter] between acts, `…` at each beat within one.
`--auto` runs straight through with no clears. `--offline` skips the live
half of act 2 and acts 3–4, the only parts that need an endpoint.

**The shape.** Before, then after. Act 1 is the document as the house has it.
Acts 2–4 use a model to turn it into rules, answer a question over them, and
find where they contradict each other. Then the cable comes out, and the first
thing the house does without a model is act on its own contradiction. The
reveal lands because the room has just watched the model half work.

| act | beat | command | model |
|---|---|---|---|
| 1 | **before** — one document; Article I and a decision 100 lines down say opposite things | two passages of `maple-house.md`, labelled as canon cites them | no |
| 2 | the model reads Article I live; four rules cite lines 3–8 | `canon draft --dry-run --max-chunks 1 --from …` | **yes** |
| 2, cont. | the whole document, from the tape of the same command; then the six kept | `canon draft --replay <tape>` · `canon list` | taped |
| 3 | ask the house a question; its own rules answer, and find act 1's pair | `canon check "…"` | **yes** |
| 4 | ask what it disagrees with itself about | `canon tensions` | **yes** |
| — | **the turn** — "everything that read went through a model. Now pull the cable out." | | |
| 5 | carry it knowingly; six months later, ask why | `canon accept a b -m "…"` · `canon why b` | no |
| 6 | a different house, two years in; a bot overruled and dated | `canon why can-5e1a8e880e1d` | no |
| 7 | what the house decided **not** to have | `canon voice human:mira` | no |
| 8 | nobody had to remember | `canon overdue` | no |
| 9 | two years replayed against Ostrom's eight, 11 ms | `canon replay fixtures/fernwood-commons --brief` | no |
| 10 | what dropping consent would have done | `… --policy default --brief` | no |
| 11 | fourteen commons, one spine, four broken on purpose | `cargo test --test transfer_bar -- --nocapture` | no |
| 12 | **one more thing** — the founding documents, 12,672 words, cold | `canon draft --replay <run>` | taped |
| 13 | 283 contradictions proposed. Four of them, one wrong | a reader over the run file | no |

**What is on the screen is for the room.** Every line `demo.sh` prints is
audience-facing. Presenter directions live here, not on the projector.

## Say that the replays are recordings

Act 2 (the whole document), act 12 and act 13 replay tapes: real replies a
real 27B gave, run back through the same pipeline. Not a mock, not a mockup,
not a live call. The screen says "a recording" each time; say it too. The
live read of Article I that opens act 2 is what makes the word land: the room
has just watched a tape get cut.

## The lines that carry it

- **Act 1** — read both passages aloud. Then: *"Nobody wrote down that one
  replaced the other."* Let it sit.
- **Act 2** — while the model reads, twenty seconds: *"It's reading Article
  I now."* When the rules land: *"Every one cites lines 3 to 8. You just read
  them."*
- **Act 3** — take a question from the room if you want one; type it in place
  of the cousin. The cousin is the safe one: it lands on act 1's pair.
- **Act 4** — *"Proposed, not ruled. Nothing is written until a person says
  so."*
- **Act 5** — the first thing without a model is the house acting. *"It can't
  fix this tonight. It can write down that it knows."*
- **Act 7** — the silence is the emotional beat. Hold it.
- **Act 10** — hold the silence after the number. *"Every group has had this
  argument. No group has ever been able to check."*
- **Act 13** — read the first pair aloud and stop. All men are created equal,
  against three fifths of all other Persons. Then the fourth, which is wrong:
  *"It proposes things that are not there. That is why review is one at a
  time."*
- **The close** — volunteer the number that hurts. *"We asked whether it was
  reading or remembering. We removed the fact each contradiction turns on and
  asked again. Five dropped. Four didn't. We published that too."*

## Before you present

```sh
cargo build
cargo test --test transfer_bar         # act 11 runs this; build it first
./scripts/demo.sh --offline --auto     # everything but the live parts, no endpoint
./scripts/demo.sh --auto               # the whole thing, under a minute
```

**Size the terminal before the curtain.** canon wraps its own long lines to
`COLUMNS`, with continuation lines under the text, and `demo.sh` exports that
from `tput cols`. Around 100 columns at a projector-sized font reads best;
narrower still works, it just wraps more.

`demo.sh` preflights the endpoint and refuses in the green room rather than
failing in front of the room. **Only the live half of act 2 and acts 3 and 4 need it**, about twenty-five
seconds between them. Everything else, including the rest of act 2 and both
founding acts, runs with nothing plugged in.

**Act 2 generates first, then replays.** A replay alone looks staged: a path
under `fixtures/` and the word "recording". So with an endpoint the model
reads Article I live, `--max-chunks 1`, about twenty seconds on the MoE, and
four rules come out citing `maple-house.md:3-8`, the passage act 1 showed.
`draft` ends with "run recorded at": the room has just watched a tape get cut.
The whole document is 24 passages and a quarter of an hour, so the rest is the
same command's run from 2026-08-31, replayed: every reply a real 27B gave, run
through the same pipeline. Not a mock — real output, no live risk, **0.03s**. A tape is what `draft` writes about itself; [cutting one is two
verbs](#cutting-a-tape-is-two-verbs). A tape only replays
against the build it was cut on; a stale one refuses loudly rather than
answering from the wrong recording, so re-cut it whenever the call sequence
changes.

**A replay writes nothing, so the accepts come from a seed.** `draft --replay`
sets `dry_run` because a replay is a measurement, which used to leave acts 3
and 4 with an empty canon — `check` printing "no live rules", `tensions`
printing "fewer than two", and neither one ever reaching the endpoint the
preflight had just insisted on. `fixtures/maple-house/accepted` closes that:
six rules a presenter accepted going through act 2's proposals by hand, each
one **copied from the tape's own candidate set** with the citation the model
cut for it. `demo.sh` materializes it into the demo canon right after act 2,
so acts 3–5 answer over exactly what the room just watched get proposed.

The six are chosen because the two acts want different things:

- **Act 3** needs the early guest rules — the two-night limit is what "can my
  cousin stay two weeks" lands on. It comes back with *two* conflicts, since
  the house later banned overnight guests outright and never withdrew the
  Charter article.
- **Act 4** needs the *reversals*. Maple house contradicts itself late in the
  document — overnight guests forbidden outright (`maple-house.md:109-113`),
  quiet hours moved back to 10 PM (`93-97`) — and those are the pairs
  `tensions` finds. A seed of only the early Charter rules gives act 4 **"no
  tensions found"**, which is a dead beat.

If anybody asks whether those rules were really accepted, `canon list` shows
them with their sources, and the tape shows them being proposed. Say it out
loud rather than letting it look like sleight of hand.

**Editing the seed.** Re-cut the tape and the seed together — a rule in the
seed that the tape no longer proposes is exactly the sleight of hand above.
The candidate indices it draws from are 0, 1, 2, 4, 26 and 29.

## Parked

- **The 27B is gone from the mesh.** Act 2's tape was cut on
  `Qwen3.8-27B-UD-Q6_K_XL` (48 calls,
  `fixtures/maple-house/runs/demo-tape/run.json`, replays in 0.03s), but
  `primary` now resolves to `Qwen3.6-35B-A3B-UD-MTP-IQ4_NL` and the 27B is no
  longer loaded. Act 2's tape is 27B output; acts 3 and 4 answer live on the
  MoE. Don't claim one model for both halves of the demo.

- ~~`check` and `tensions` have never been run live for the stage.~~ Run
  2026-09-01 on the MoE; both read at size. `check` lands on two conflicts
  with the amend/carry commands under each; `tensions` finds both reversals.

## How the primitives come together on stage

Nothing on the stage is a mock, and almost nothing is a script. The demo is
canon's own verbs run in order over three fixtures, and the fixtures are
canon's own ledgers. This section says which primitive each act is made of,
and how to make each artifact again.

### The acts, as verbs

| act | what the room sees | canon verbs | primitives |
|---|---|---|---|
| 1 | one document, two passages 100 lines apart | none yet: the source, labelled `file:lines` the way canon cites | — |
| 2 | Article I read live, four rules cite it; then 46 from the tape | `draft --dry-run --max-chunks 1 --from`, then `draft --replay <tape>` | resolvers: text in, cited evidence out, never a verdict |
| 3 | the house's rules answer a proposal | `check "…"` | positions with a source, a pull and a reason; a verdict must cite something the canon holds |
| 4 | the house disagrees with itself | `tensions` | positions again; every pair is a *proposal* until `accept` or `dismiss` |
| 5 | the house carries its own contradiction, and can say why | `accept a b -m`, `why b` | the ledger: append-only, reasoned, with the citation `draft` cut still attached |
| 6 | a bot said no conflict; another house carried it anyway | `why <id>` | the ledger again: `assert`, `dismiss`, `accept`, `horizon`, in order |
| 7 | what the house decided not to have | `voice <actor>` | standing queries; `silence` as data |
| 8 | nobody had to remember | `overdue` | horizons on grants and on carried contradictions |
| 9 | two years replayed, Ostrom's eight | `replay --brief` | `Log → Canon → policy → Decision`, pure |
| 10 | what a different rule would have done | `replay --policy default --brief` | policy as a pure function; decided twice, diffed |
| 11 | fourteen commons, one spine | `cargo test --test transfer_bar` | the same nine primitives; only the nouns change |
| 12 | the founding documents, cold | `draft --replay <run>` | resolvers, at 12,672 words |
| 13 | 283 tensions, four read aloud | a reader over the run file | — |

### The fixtures are ledgers, and the ledgers are verbs

**Fernwood Commons** (acts 6–10) is a `.canon/acts.jsonl` written in seed
dialect: 8 `assert`, 15 `grant`, 4 `policy set`, 2 `scope`, 1 `adopt`,
1 `silence`. The scenario then does what a house does over two years, in
canon's verbs: `position` (an agent's objection), `dismiss`, `accept` with a
`horizon`, `policy set` on the kitchen's own scope, `decide` twice, and a
`draw commit` / `draw seal` / `draw open` sequence for a panel. `expected.json`
is what those verbs must produce. No model ever touched the file.

```sh
canon replay fixtures/fernwood-commons            # 42 step(s), all as expected
canon replay fixtures/fernwood-commons --out /tmp/fw/.canon
CANON_DIR=/tmp/fw/.canon canon log                # read the ledger the acts made
```

**The CPR study** (act 11) is the same ledger fourteen times. The spine is
104 lines of those same verbs with the nouns left blank; a `vocab.json` fills
in actors, scopes, commitments and proposals and may not name a rule, a
policy, an authority or an outcome. The one generator in this repo,
`scripts/cpr-build.py`, exists to make that refusal checkable and to
*predict* `expected.json` in a second implementation, so `replay` is a
differential test rather than a recording.

```sh
python3 scripts/cpr-build.py --all --pin-draw     # regenerate; git diff is empty
cargo test --test transfer_bar                    # the study as a bar, ~3 s
```

**Maple House** (acts 1–4) is the one fixture that is a *document* rather than
a ledger, because acts 2–4 are about turning a document into one. Written by
hand as fiction for Commonwealth's tension bench, vendored with its sha256
pinned in [PROVENANCE.md](./fixtures/maple-house/PROVENANCE.md). Eleven
tensions planted, seven decoys, all labelled.

### Cutting a tape is two verbs

A tape is what `draft` writes about itself. Every `draft` run leaves
`.canon/draft-runs/<timestamp>.json`: endpoint, alias, served model, chunks,
every call and reply, every candidate, every drop and why. Act 2's tape is
that file from one run:

```sh
canon init --profile house
canon draft --dry-run --from fixtures/maple-house/maple-house.md
cp .canon/draft-runs/<timestamp>.json fixtures/maple-house/runs/demo-tape/run.json
```

Cut 2026-08-31 against `localhost:9841/v1`, alias `primary`, served by
`Qwen3.8-27B-UD-Q6_K_XL`. 48 calls. `--replay` runs the pipeline over the
recorded replies and refuses if the build asks for a call the tape lacks.
(`scripts/record-demo-tape.sh` is those three lines with a scratch canon.)

The **founding run** (acts 12–13) is the same two verbs over
`fixtures/founding/founding.md`, completed 2026-08-31 on a rented A6000
carrying the same 27B: 850 calls, 676 of 676 comparison passes, 1h37m. The
corpus itself is rebuilt from vendored National Archives and Avalon HTML by
`fixtures/founding/build.py`, no network, no model; the eleven supersessions in
its `truth.json` are parsed from the Archives' own notes. Act 13's reader,
`scripts/founding-highlights.py`, opens that run file and prints four chosen
pairs, the fourth chosen because it is wrong.

### The accepts are what `draft` writes when you say yes

`draft --replay` is a measurement and writes nothing. So the six rules acts 3–5
answer over live in `fixtures/maple-house/accepted/` as a seed: six
`assert` acts, each the text and citation of a candidate from the tape
(indices 0, 1, 2, 4, 26, 29), which is exactly what pressing accept during
`draft` would have written. `demo.sh` materializes them with

```sh
canon replay fixtures/maple-house/accepted --out <dir>/.canon --profile house
```

and `canon list` shows them with their sources if anyone asks.

### Tolerance

| kind | acts | reproduces |
|---|---|---|
| **ledger verbs** | 1, 5–11 | byte for byte, any machine, no endpoint |
| **taped** | 2 (the rest), 12, 13 | byte for byte from the tape; re-cutting gives a different candidate set |
| **live** | 2 (first passage), 3, 4 | wording varies; the shape holds: four guest rules citing 3-8, two conflicts on the cousin, the two reversals as tensions |

The live half of act 2 and acts 3–4 need an OpenAI-compatible endpoint that can enforce a JSON schema
(`canon config set endpoint <url>`, or `CANON_ENDPOINT`). Rust 1.95 is pinned
in `rust-toolchain.toml`; Python is stdlib only. To put a number on your own
model rather than trust ours, `./scripts/draft-bar.sh 3` runs three tapes of
Maple House and `./scripts/score-bar.sh maple-house` scores them against
`truth.json`, naming the model that produced each.
