# Automation for the People — the run of show

`./scripts/demo.sh` — [enter] between acts, `…` at each beat within one.
`--auto` runs straight through with no clears. `--offline` skips the live
half of act 2 and acts 3–4, the only parts that need an endpoint.

**One question, three times.** On what terms can an agent be a member of a
group? A house; a house that gave a helper a seat for two years; a country.
Every part turns on the same moment: a group changed a rule and nobody struck
the old one or wrote down why. Jefferson names the failure. Ostrom is the
hero: she showed ordinary people can govern what they share, and found the
eight things groups that last all do. Those eight are the bar.

**What is on the screen is for the room.** Every line `demo.sh` prints is
audience-facing and written for a smart high-school crowd: no jargon, plain
words. Presenter notes live here.

## Cold open

Five cards, one line each. Say the rest.

1. *Automation for the People.*
2. *"The earth belongs to the living."* Jefferson to Madison, 1789. He meant
   that no generation should be bound by rules it can't see and can't
   revise. Then he helped write a document that still contains the
   three-fifths clause.
3. *Every group forgets why.* Your house, your club, your team. Rules pile
   up. Reasons don't.
4. *Ostrom's eight.* Elinor Ostrom spent a career on ordinary people who
   shared a pasture, a canal, a fishing ground, and kept it going for
   centuries with no king and no market. Eight things they all did. Say two
   or three in plain words: everyone knows who's in; the people who live
   under a rule can change it; whoever watches can be overruled by the
   people they watch.
5. *We're about to add a new kind of member to our groups. On what terms?*

## The three parts

| act | beat | command | model |
|---|---|---|---|
| | **Part one. A house.** | | |
| 1 | **before** — one document; Article I and a decision 100 lines down say opposite things | two passages of `maple-house.md`, labelled as canon cites them | no |
| 2 | the agent reads Article I live; four rules cite lines 3–8 | `canon draft --dry-run --max-chunks 1 --from …` | **yes** |
| 2, cont. | the whole document from the recording of the same command; six kept by hand | `canon draft --replay <run>` · `canon list` | recorded |
| 3 | ask the house a question; its own rules answer, and find act 1's pair | `canon check "…"` | **yes** |
| 4 | ask where the house disagrees with itself | `canon tensions` | **yes** |
| — | **the turn** — everything that read went through a model. Pull the cable out. | | |
| 5 | the people decide: carry it knowingly, then ask why | `canon accept a b -m "…"` · `canon why b` | no |
| | **Part two. A member.** | | |
| 6 | two rules, one hall: the stroller and the bikes; the helper called no conflict from a kitchen seat, and it never took effect; Dana decided | `canon why <the hall rule>` | no |
| 7 | the helper's whole record: a seat with an end date, an objection, a call outside its seat, a proposal refused | `canon voice agent:helper` | no |
| 7, cont. | **live, no model:** a non-cook writes a kitchen rule; it lands as a proposal; both cooks approve; it is a rule | `canon add … --scope house.kitchen` · `canon approve` ×2 | no |
| 8 | nobody had to remember | `canon overdue` | no |
| 9 | Wednesday dinners, unwritten on purpose; the laundry asked three times, and what each ask cost | `canon voice human:mira` | no |
| 10 | two years against Ostrom's eight, in milliseconds | `canon replay fixtures/fernwood-commons --brief` | no |
| 11 | what if we had decided differently | `… --policy default --brief` | no |
| 12 | not just houses, yours: one institution's vocabulary is nouns only; the grid; then the three commands that put an agent in your repo on the same terms | `cargo test --test transfer_bar -- --nocapture` | no |
| | **Part three. A country.** | | |
| 13 | the agent reads the founding documents cold | `canon draft --replay <run>` | recorded |
| 14 | 283 contradictions proposed. Four of them, one wrong | a reader over the run file | no |
| — | **close** — reading, or remembering? Five and four. Under the record, not above it. | | |
| — | **curtain call** — a cat crosses the screen trailing a rainbow, and leaves Ostrom's line: *a resource arrangement that works in practice can work in theory.* | | |

## Say that the replays are recordings

The second half of act 2 and act 13 replay recordings: real replies a real
model gave, run back through the same steps. The screen says "a recording"
each time; say it too. The live read of Article I that opens act 2 is what
makes the word land: the room has just watched a recording get made.

## The lines that carry it

- **Act 1** — read both passages aloud. *"Nobody wrote down that one
  replaced the other."* Let it sit.
- **Act 2** — while it reads, twenty seconds: *"It's reading Article I now."*
  When the rules land: *"Every one cites lines 3 to 8. You just read them."*
  Then the first term of membership: *it proposes, it does not decide.*
- **Act 3** — take a question from the room if you want one; type it in place
  of the cousin. The cousin is the safe one: it lands on act 1's pair.
- **Act 5** — the first thing without a model is people deciding. *"It can't
  fix this tonight. It can write down that it knows."*
- **Part two card** — name the boundary. Part one was a house that had not
  said who is in; anyone could write and everything was a rule. *"This one
  drew a boundary."* Everything in part two follows from that.
- **Act 6** — set up the people before the bot. *"Dana wrote both rules the
  same day. A parent and the cyclists, one corridor."* Then the helper's
  call, from a kitchen seat, about the hall. *"The record kept what it said,
  and it never took effect."* Then Dana's decision. The line: *"A member may
  speak anywhere. It decides only where it was given a say."*
- **Act 7** — the thesis on one screen. A seat, given by Mira, that ended in
  January. An objection, citing a rule. A call, overruled. A proposal,
  refused by a cook with a reason. *"That's a member."* Then the live beat:
  Theo types a kitchen rule and the screen says PROPOSED. *"Nobody's word
  has been taken away from them, and nobody's rule has been written for
  them."* Dana approves, still one short. Sam approves. *"Both cooks. Now
  it's a rule, and the record says who made it one."*
- **Act 9** — two stories, told before the command. Wednesday dinners: two
  years of someone just cooking, then a rotation proposed, then Mira's no with
  her reason. *"A rotation would turn a kindness into a duty."* Hold that; it is
  the emotional beat. Then the laundry, which is Ostrom's graduated
  sanctions in one record: ask one, ask the house, no. *"Nobody kept a file
  on anyone."*
- **Act 10** — this is where a room decides whether it's real. Say the eight
  in plain words. Then show the two files: one raw step from the script of
  the house's two years, and the prediction written for it before the replay
  ran. *"Replay rebuilds the whole history from the record and checks every
  prediction."* The table prints every scene under its principle: what was
  asked, what the rules said. *Built in* means the tool does it; *left to
  people* means it stays out of the way. Close: *"It can fail, and act 12
  shows it failing."*
- **Act 11** — the brief form groups what moved into EASIER and HARDER, two
  lines a decision. Point at the front-door lock under EASIER: *"one person
  could have waved through a change nobody could undo."* Then hold the
  silence. *"Every group has had this argument. No group has ever been able
  to check."*
- **Act 12** — this is the builders' act; everyone in the room is asking
  "how do I use this." Answer in order: the cost of entry (the codebase's
  vocabulary, nouns and nothing else, *"the same 104 lines run all
  fourteen"*), the grid (*"four broken on purpose, that's how you know the
  test is real"*), then the three commands. Read them slowly. `canon mcp` is
  the line that lands: *"your agent joins over MCP, and reads what is in
  force and how a proposal stands. Anything it writes is a proposal. Its seat
  expires. Same terms as the people."*
- **Part three card** — Jefferson again, without saying his name. *"The
  Constitution still contains the three-fifths clause. Nobody struck it."*
- **Act 14** — read the first pair aloud and stop. All men are created equal,
  against three fifths of all other Persons. Then the fourth, which is wrong.
- **The close** — volunteer the number that hurts. Five dropped, four didn't.
  *"You should ask that of every agent."* Then the last card: under the
  record, not above it.
- **Curtain call** — press enter once more and say nothing. The cat crosses,
  the rainbow stays, and the card reads Ostrom's answer to the theorists who
  told her self-governing commons could not work: *"A resource arrangement
  that works in practice can work in theory."* Let the room read the last
  line, *"So can a demo,"* and take the laugh. Skipped in `--auto`.

## Before you present

```sh
cargo build
cargo test --test transfer_bar         # act 12 runs this; build it first
./scripts/demo.sh --offline --auto     # everything but the live parts, no endpoint
./scripts/demo.sh --auto               # the whole thing, under a minute
```

**Size the terminal before the curtain.** canon wraps its own long lines to
`COLUMNS`, with continuation lines under the text, and `demo.sh` exports that
from `tput cols`. Around 100 columns at a projector-sized font reads best;
narrower still works, it just wraps more.

**Nothing else on the host.** A busy mesh answers `503 host busy`, and canon
waits and retries rather than failing: the live read in act 2 then sits on a
spinner for minutes. Before the curtain, stop builds, tests and sweeps on the
machine that serves the endpoint, and run act 2 once as a warm-up.

`demo.sh` preflights the endpoint and refuses in the green room rather than
failing in front of the room. **Only the live half of act 2 and acts 3 and 4 need it**, about twenty-five
seconds between them. Everything else, including the rest of act 2 and both
founding acts, runs with nothing plugged in.

**Act 2 generates first, then replays.** A replay alone looks staged: a path
under `fixtures/` and the word "recording". So with an endpoint the model
reads Article I live, `--max-chunks 1`, about twenty seconds on the MoE, and
four rules come out citing `maple-house.md:3-8`, the passage act 1 showed.
`draft` ends with "run recorded at": the room has just watched a recording get made.
The whole document is 24 passages and a quarter of an hour, so the rest is the
same command's run from 2026-08-31, replayed: every reply a real 27B gave, run
through the same pipeline. Not a mock — real output, no live risk, **0.03s**. A recording is what `draft` writes about itself; [making one is two
verbs](#making-a-recording-is-two-verbs). A recording only replays
against the build it was cut on; a stale one refuses loudly rather than
answering from the wrong recording, so re-cut it whenever the call sequence
changes.

**A replay writes nothing, so the accepts come from a seed.** `draft --replay`
sets `dry_run` because a replay is a measurement, which used to leave acts 3
and 4 with an empty canon — `check` printing "no live rules", `tensions`
printing "fewer than two", and neither one ever reaching the endpoint the
preflight had just insisted on. `fixtures/maple-house/accepted` closes that:
six rules a presenter accepted going through act 2's proposals by hand, each
one **copied from the recording's own candidate set** with the citation the model
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
them with their sources, and the recording shows them being proposed. Say it out
loud rather than letting it look like sleight of hand.

**Editing the seed.** Re-cut the recording and the seed together — a rule in the
seed that the recording no longer proposes is exactly the sleight of hand above.
The candidate indices it draws from are 0, 1, 2, 4, 26 and 29.

## Parked

- **The 27B is gone from the mesh.** Act 2's recording was cut on
  `Qwen3.8-27B-UD-Q6_K_XL` (48 calls,
  `fixtures/maple-house/runs/demo-tape/run.json`, replays in 0.03s), but
  `primary` now resolves to `Qwen3.6-35B-A3B-UD-MTP-IQ4_NL` and the 27B is no
  longer loaded. Act 2's recording is 27B output; acts 3 and 4 answer live on the
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
| 2 | Article I read live, four rules cite it; then 50 from the recording; six kept | `draft --dry-run --max-chunks 1 --from`, `draft --replay`, `list` | resolvers: text in, cited evidence out, never a verdict |
| 3 | the house's rules answer a proposal | `check "…"` | positions with a source, a pull and a reason; a verdict must cite something the canon holds |
| 4 | the house disagrees with itself | `tensions` | positions again; every pair is a *proposal* until `accept` or `dismiss` |
| 5 | the people carry the contradiction, and can say why | `accept a b -m`, `why b` | the ledger: append-only, reasoned, the citation `draft` cut still attached |
| 6 | the hall and the bikes; a bot ruled from outside its seat and the ruling did not apply; the house decided | `why <id>` | rulings take standing: `dismiss` without it is recorded and not applied; `accept` by a holder is |
| 7 | the helper's record, then a non-cook's kitchen rule becomes a proposal and then a rule | `voice agent:helper`, `add --scope`, `approve` | standing with a horizon; **ratification**: a write is a proposal until the scope's rule is met |
| 8 | nobody had to remember | `overdue` | horizons on grants and on carried contradictions |
| 9 | what Mira decided not to have | `voice human:mira` | standing queries; `silence` as data |
| 10 | two years against Ostrom's eight | `replay --brief` | `Log → Canon → policy → Decision`, pure |
| 11 | what a different rule would have done | `replay --policy default --brief` | policy as a pure function; decided twice, diffed |
| 12 | fourteen commons, one spine | `cargo test --test transfer_bar` | the same nine primitives; only the nouns change |
| 13 | the founding documents, cold | `draft --replay <run>` | resolvers, at 12,672 words |
| 14 | 283 tensions, four read aloud | a reader over the run file | — |

### The fixtures are ledgers, and the ledgers are verbs

**Fernwood Commons** (acts 6–11) is a `.canon/acts.jsonl` written in seed
dialect: 8 `assert`, 15 `grant`, 4 `policy set`, 2 `scope`, 1 `adopt`,
1 `silence`. The scenario then does what a house does over two years, in
canon's verbs: `position` (an agent's objection), `dismiss`, `accept` with a
`horizon`, `policy set` on the kitchen's own scope, `decide` twice, a
`draw commit` / `draw seal` / `draw open` sequence for a panel, and at the end
`ratification set` on the kitchen, a non-cook's `add` that stays a proposal
until both cooks `approve`, and the helper's proposal that a cook refuses.
`expected.json` is what those verbs must produce. No model ever touched it.

```sh
canon replay fixtures/fernwood-commons            # 56 step(s), all as expected
canon replay fixtures/fernwood-commons --out /tmp/fw/.canon
CANON_DIR=/tmp/fw/.canon canon log                # read the ledger the acts made
```

**The CPR study** (act 12) is the same ledger fourteen times. The spine is
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

### Making a recording is two verbs

A recording is what `draft` writes about itself. Every `draft` run leaves
`.canon/draft-runs/<timestamp>.json`: endpoint, alias, served model, chunks,
every call and reply, every candidate, every drop and why. Act 2's recording is
that file from one run:

```sh
canon init --profile house
canon draft --dry-run --from fixtures/maple-house/maple-house.md
cp .canon/draft-runs/<timestamp>.json fixtures/maple-house/runs/demo-tape/run.json
```

Cut 2026-08-31 against `localhost:9841/v1`, alias `primary`, served by
`Qwen3.8-27B-UD-Q6_K_XL`. 48 calls. `--replay` runs the pipeline over the
recorded replies and refuses if the build asks for a call the recording lacks.
(`scripts/record-demo-tape.sh` is those three lines with a scratch canon.)

The **founding run** (acts 13–14) is the same two verbs over
`fixtures/founding/founding.md`, completed 2026-08-31 on a rented A6000
carrying the same 27B: 850 calls, 676 of 676 comparison passes, 1h37m. The
corpus itself is rebuilt from vendored National Archives and Avalon HTML by
`fixtures/founding/build.py`, no network, no model; the eleven supersessions in
its `truth.json` are parsed from the Archives' own notes. Act 14's reader,
`scripts/founding-highlights.py`, opens that run file and prints four chosen
pairs, the fourth chosen because it is wrong.

### The accepts are what `draft` writes when you say yes

`draft --replay` is a measurement and writes nothing. So the six rules acts 3–5
answer over live in `fixtures/maple-house/accepted/` as a seed: six
`assert` acts, each the text and citation of a candidate from the recording
(indices 0, 1, 2, 4, 26, 29), which is exactly what pressing accept during
`draft` would have written. `demo.sh` materializes them with

```sh
canon replay fixtures/maple-house/accepted --out <dir>/.canon --profile house
```

and `canon list` shows them with their sources if anyone asks.

### Tolerance

| kind | acts | reproduces |
|---|---|---|
| **ledger verbs** | 1, 5–12 | byte for byte, any machine, no endpoint |
| **recorded** | 2 (the rest), 13 | byte for byte from the recording; re-cutting gives a different candidate set |
| **live** | 2 (first passage), 3, 4 | wording varies; the shape holds: four guest rules citing 3-8, two conflicts on the cousin, the two reversals as tensions |

The live half of act 2 and acts 3–4 need an OpenAI-compatible endpoint that can enforce a JSON schema
(`canon config set endpoint <url>`, or `CANON_ENDPOINT`). Rust 1.95 is pinned
in `rust-toolchain.toml`; Python is stdlib only. To put a number on your own
model rather than trust ours, `./scripts/draft-bar.sh 3` runs three recordings of
Maple House and `./scripts/score-bar.sh maple-house` scores them against
`truth.json`, naming the model that produced each.
