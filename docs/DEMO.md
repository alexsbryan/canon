# Automation for the People — the run of show

`./scripts/demo.sh` — [enter] between acts, `…` at each beat within one.
Fifty-two pauses end to end, with the transfer study off. The copy is held to [VOICE.md](./VOICE.md):
a line lands on the same screen as the output it describes, nothing
pre-explains a command, and nothing is there to impress.
`--auto` runs straight through with no clears. `--offline` skips the live
half of act 2 and acts 3–4, the only parts that need an endpoint.

**One question, three times.** On what terms can an agent be a member of a
group? A house; a house that gave a helper a seat; a country.
Every part turns on the same moment: a group changed a rule and nobody struck
the old one or wrote down why. Jefferson names the failure. Ostrom is the
hero: she showed ordinary people can govern what they share, and found the
eight things groups that last all do. Those eight are the bar.

**Each part asks something the last one could not.** Part one: can it read,
and who decides. Part two: on what terms does it hold a seat. Part three: does
the reading hold at the scale of a country, and was it reading at all. Part
two never re-tells a contradiction; part one did that. Every beat in it is a
term of membership the room has not seen yet.

**What is on the screen is for the room.** Every line `demo.sh` prints is
audience-facing and written for a smart high-school crowd: no jargon, plain
words. Presenter notes live here.

## Cold open

Four cards, one line each. Say the rest.

1. *Automation for the People.*
2. *"The earth belongs to the living."* Jefferson to Madison, 1789. He meant
   that no generation should be bound by rules it can't see and can't
   revise. Then he helped write a document that still contains the
   three-fifths clause. Under it, in the room's words: every group forgets
   why. Rules pile up. Reasons don't.
3. *Ostrom's eight.* Elinor Ostrom spent a career on ordinary people who
   shared a pasture, a canal, a fishing ground, and kept it going for
   centuries with no king and no market. Eight things they all did. Say two
   or three in plain words: everyone knows who's in; the people who live
   under a rule can change it; whoever watches can be overruled by the
   people they watch.
4. *We're about to add a new kind of member to our groups. On what terms?*

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
| 6 | six commands over five months, on screen as the people and the program typed them: a seat with a room and an end date; Theo's ask; the helper's reading, citing a rule; its ruling outside its seat, kept and not applied; Dana's ruling. Then the readback, then January | `canon voice agent:helper` · `canon overdue` | no |
| 7 | **live, no model:** a non-cook writes a kitchen rule; it lands as a proposal; both cooks approve; it is a rule. Then Ostrom's three levels: the helper tries to change how the kitchen makes rules, kept and not applied; Theo tries, and it is a proposal for the cooks | `canon add … --scope house.kitchen` · `canon approve` ×2 · `canon ratification set …` ×2 | no |
| 7, cont. | the garden went further: a rule carried twice, with somebody new between the votes | `canon why <the watering rule>` | no |
| 8 | Wednesday dinners, unwritten on purpose; the laundry in two lines | `canon voice human:mira` | no |
| 9 | the eight in the room's words, each tied to a beat they saw; then the whole record, a year of the house, checked against all eight in milliseconds | `canon replay fixtures/fernwood-commons --brief` | no |
| 10 | what if we had decided differently | `… --policy default --brief` | no |
| 11 | yours: the three commands that put an agent in your repo on the same terms | none; three lines of screen text | no |
| 11, cont. | **off by default** (`SHOW_TRANSFER=1`): the transfer study, fourteen places from one spine | `cargo test --test transfer_bar -- --nocapture` | no |
| | **Part three. A country.** | | |
| — | **Gödel card** — Article V can amend itself: the third level with nothing above it. He found it by reading | | |
| 12 | the agent reads the founding documents cold | `canon draft --replay <run>` | recorded |
| 13 | 283 contradictions proposed. Four of them, one wrong | a reader over the run file | no |
| — | **close** — reading, or remembering? Five and four. Under the record, not above it. | | |
| — | **curtain call** — a cat crosses the screen trailing a rainbow, and leaves Ostrom's line: *a resource arrangement that works in practice can work in theory.* | | |

## Say that the replays are recordings

The second half of act 2 and act 12 replay recordings: real replies a real
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
  fix this tonight. It can carry both, say why, and set a date."* `accept`
  accepts that the two rules contradict; it does not accept the ban, and the
  reason says so. The room reads the verb the other way, so say "carry"
  aloud. The date goes in as `--revisit`, and the `why` that follows ends on
  *look again by 2026-10-01*.
- **Part two card** — say what changes. *"In part one the agent read, and
  the people decided. This house went further: it said who is in, who
  decides what, and for how long. Then it gave a helper a seat."* Then the
  three questions, and let them be the spine of the part: what was it given,
  what did it say, where did the people overrule it. Do not re-tell the hall
  as a conflict story. The room just watched a contradiction get found and
  carried; another one reads as the same act again.
- **Act 6** — events first, readback last. The first screen is the five
  commands that made the helper's record, with the real ids from the
  canon on the projector, and one sentence for builders before it: *the
  helper is a program; it runs the same commands as everyone, with its own
  name; over MCP it can only read.* Walk the timeline in order, by month.
  September: *"one room, the kitchen, given by Mira, with an end date. Nine
  days later Dana wrote the two hall rules."* The wires, said once
  before the timeline: *"it runs when the record changes and on a nightly
  schedule, with the house's model; that crontab is the house's, and the
  record holds only what it wrote."* Both of the helper's verbs, `check`
  and `tensions`, call a model, and it did that in November with the cable
  in. Nothing that runs on stage in part two does; the turn after act 4
  stays true. Nov 9 needed no judgment from the helper, since `check`
  handed it the reading; Nov 10 is the model's own view of two rules,
  written as a ruling, which is the step the fold catches. November has a cause on screen now: *"Theo asked, on the record.
  The record changed, so the helper ran `check`, the command from act 3, and
  filed what it found: the stroller rule pulls against. That is reading."*
  Then the turn: *"that night its job ran `tensions`, the command from act
  4, which proposes pairs for a person to rule on. It ruled instead."* Both
  triggers are verbs the room watched in part one, and the one thing the
  record cannot show, the cron line, is named as a cron line. Only then `voice`, and
  the screen is recognition: the seat, lapsed; the position; the ruling,
  not applied. Then `overdue`, which shows January and June, and the close
  in order: *"January came. The seat lapsed on its date. Nobody had to
  remember. And in February Dana, who has stood in that hall, ruled the same
  way the helper read it, with a date to look again: June. June has come
  and gone, and the record is still asking. It may even have been right. It
  was not its call."* The `overdue` screen shows both dates as overdue; say
  so rather than let the room notice. The helper reads; it does not measure.
  Its ruling came from reading two rules side by side, and Dana's is the one
  grounded in the actual hall. That last pair is the thesis of part two: the failure is
  the seat, not the answer. Dana's ruling is told last because it happened
  last; the earlier draft said February before January. One thing to have
  ready: the grant's horizon prints as January 31, so say January 31.
- **Act 7** — the live beat, and the one thing part one could not do. Open
  on the contrast: *"In part one anyone could write a rule and it was a
  rule on arrival."* Then the seats, on screen before anyone writes into
  the kitchen: what a seat is in one sentence (*"one line: who holds a room,
  and until when. You may grant a room you hold, or the room above it. The
  first grant in an empty record needs nobody's leave"*), then the four
  commands: Mira's founding grant to herself, Theo's, the two cooks', and
  Dana setting `joint`. Say the founding line plainly if asked who granted
  Mira: she did, on day one, and it is in the record like any other line.
  Then `standing` has a meaning when it comes up: holders write, everyone
  else proposes, and the cooks raised the kitchen above it. Theo types a kitchen rule and the screen says
  PROPOSED. *"Nobody's word was taken from them, and nobody's rule was
  written for them."* Dana approves, still one short. Sam approves.
  *"Both cooks. Now it's a rule, and the record says who made it one."*
  Then the ladder. Ostrom counted three levels of rules, and the room has
  now seen two: what you may do (the leftovers), and how those rules get
  made (both cooks). The third is who may change *that*. Show it as an
  attempt, not a lecture: *"The helper tries."* It sets the kitchen back to
  "whoever holds it writes directly", and the screen says NOT APPLIED, with
  the reason. Then Theo tries the same thing. He holds the house, so he
  may; the screen says PROPOSED, needs approval from dana, sam. *"Not
  applied is for someone with no say. Proposed is for someone with a say
  and not the last word. Changing how the kitchen decides is judged the way
  the kitchen decides today."* The line: *"Three levels, and the record
  answers at all three."* This is the one beat that says the system extends
  upward, so do not rush it.
- **Act 7, continued** — the garden, in two screens. First the five ways a
  room can turn a line into a rule, in house words, with the kitchen and
  the garden marked on the rows they chose; this is the only place the
  show explains ratification's options, so give it its pause. Then the
  five commands: Ola setting `twice`, Juno writing her rule, Ola's April
  approval that counts for nothing because she was there for the first
  vote, Mira granting Kit the garden, Ola's approval that counts. Point at
  the grant, because the newcomer's arrival is an ordinary act. Then `why`
  on Juno's rule, and read the status line: Juno's own write was the first
  vote, Kit joined, Ola's yes was the second. The line
  is Jefferson's from the cold open, landed: *"The earth belongs to the
  living. In the garden a rule doesn't count until somebody who wasn't
  there yet says yes too."* Do not explain turnover further; the sentence
  is the mechanism.
- **Act 8** — the Wednesday story, three sentences, then Mira's three
  commands on screen before `voice`: `canon silence` with her reason, and
  the two `canon decide` lines about the laundry. The verb `silence` is the
  beat; let the room read it as the thing she typed. Then the screen, then
  the line: *"A rotation would turn a kindness into a duty."* Hold that; it
  is the emotional beat. The laundry gets two
  sentences, no more: *"Asked once, ask one person. Asked again, ask the
  house. Counted from what the house decided, never from watching
  anyone."* Row 5 of the next act shows the ladder itself.
- **Act 9** — the table is a recap, not new information, and the legend is
  what makes it one. Read the legend as recognition: *"Row 4 is the helper.
  Row 3 is Theo's leftovers. Row 5 is the laundry. Row 6 is the bikes."*
  Then one sentence on the test before the command: *"Every step's answer
  was written down before the replay ran, so it can fail."* Then the table.
  Two rows say *left to people*: *"the tool stays out of those. A table
  where all eight say built in is the tell that somebody stretched a
  definition."* Close on the last line: fifty-six steps, all as expected,
  eight milliseconds. Do not put the raw step or the prediction on the
  screen; if anyone asks how the test can fail, [the answer is
  below](#if-someone-asks).
- **Act 10** — the brief form groups what moved into EASIER and HARDER, two
  lines a decision. Point at the front-door lock under EASIER: *"one person
  could have waved through a change nobody could undo."* End there. The
  claim that nobody has been able to check this before was cut: the screen
  makes it, and saying it is selling.
- **Act 11** — one screen, the three commands, read slowly. `canon mcp` is
  the line that lands: *"your agent joins over MCP, and reads what is in
  force and how a proposal stands. Anything it writes is a proposal. Its
  seat expires. Same terms as the people."*
- **Act 11, continued** — the transfer study, off by default since the show
  ran long; `SHOW_TRANSFER=1` brings it back whole. When it runs: everyone in the room is asking
  "how do I use this." It opens with the question the room has: *"Fernwood
  was one house. Is any of this a house thing?"* Then the test, plainly:
  the same 104 commands, fourteen places, only the names change. Then the
  cost of entry: the codebase's file is names only, and the build fails if
  a rule or a vote count appears in it. Then the grid, read as a grid:
  fourteen rows, eight columns that are Ostrom's eight, a dot where it held,
  ten real places all holding, four we broke on purpose each failing only at
  the principle we broke, and `n` where a place says a principle does not
  apply, with a reason. Then the three commands.
  Read them slowly. `canon mcp` is the line that lands: *"your agent joins
  over MCP, and reads what is in force and how a proposal stands. Anything
  it writes is a proposal. Its seat expires. Same terms as the people."*
- **Part three card** — Jefferson again, without saying his name. *"The
  Constitution still contains the three-fifths clause. Nobody struck it."*
- **Gödel card** — the other failure, and the one act 7 answered in a
  house. Read the quote, then: *"Einstein spent the drive talking him out
  of telling the judge. The best guess at what he found is Article V: the
  clause that says how to amend the Constitution can be used to amend
  itself. That is the kitchen's third level, at the scale of a country,
  with nothing above it."* Say "best guess": Morgenstern's memorandum
  records the episode, not the argument. The last line is the segue into
  act 12: *"He didn't get there from a theorem. He got there by reading the
  document."* If anyone asks whether this is Gödel's incompleteness
  theorem: no, and the demo doesn't claim it. Incompleteness is about what
  a system cannot reach; this is about a system working perfectly and
  producing a catastrophe. He found it the way `draft` finds things, by
  reading.
- **Act 13** — read the first pair aloud and stop. All men are created equal,
  against three fifths of all other Persons. Then the fourth, which is wrong.
- **The close** — volunteer the number that hurts. Five dropped, four didn't.
  *"You should ask that of every agent."* Then the last card: under the
  record, not above it.
- **Curtain call** — press enter once more and say nothing. The cat crosses,
  the rainbow stays, and the card reads Ostrom's answer to the theorists who
  told her self-governing commons could not work: *"A resource arrangement
  that works in practice can work in theory."* Let the room read the last
  line, *"So can a demo,"* and take the laugh. Skipped in `--auto`.

## If someone asks

Three things came off the stage after the first audience, because each one
raised more questions than it answered. They are still true and still one
command away.

**Who did decide the hall?** Act 6 says Dana did; the screen shows the
helper's call marked *not applied*. To show the human decision beside it:

```sh
canon why <the hall rule>       # canon list | grep -i 'hall stays clear'
```

Two adjacent lines: the helper's call, *not applied: outside their standing*,
and Dana's *carried against … there is nowhere else for the bikes*, with a
revisit date. This used to open part two, and it reads as act 5 again.

**How can the Ostrom test fail?** The fixture is two files: a script of the
house's year, seventy-one steps in canon's own verbs, and a prediction for
every step written from Ostrom's principle before the replay ran. One step and
its prediction:

```sh
grep '"bikes-against-the-hall"' fixtures/fernwood-commons/scenario.jsonl | python3 -m json.tool
grep -A5 '"bikes-against-the-hall"' fixtures/fernwood-commons/expected.json
```

Replay rebuilds the history and checks every prediction; a step that lands
somewhere else fails the run. Act 11's four ablations are that failure, on
purpose. Raw JSON on the projector is what made this a question rather than
an answer.

**Can't it just edit the file?** Yes, and so can a person. The record is a
text file, and the fold takes the time and the name on each line as given;
two hand-written lines dated before the founding would unseat a house with
every rule satisfied. Nothing inside the format can tell. Where the ledger
lives in git, git can, and `canon guard git` has it: every commit that
touched the ledger added lines and nothing else, none dated before what it
already held. It is opt-in, it prints what it cannot see, and a canon with
no guard says it trusts its log. One command shows the shape:

```sh
canon witness            # every commit that touched the ledger, checked against its parent
```

The demo does not put this on stage because the evening's question is on
what terms an agent is a member, and this is about the floor under
everybody. It is the first question a sharp room asks after "under the
record", so have the answer.

**What stops a holder loosening their own rule?** Nothing stops them
proposing it; the proposal is judged under the rule it would replace, by
the same people. Under `joint` both cooks have to agree to loosen `joint`.
The one remaining gap, named in
[PRIMITIVES.md](./PRIMITIVES.md#ratification--the-collective-choice-level):
an author can still `undo` their own rule change, because undoing your own
act is always yours.

**The laundry ladder.** Act 8 gives it two sentences and row 5 of act 9's
table shows the three rungs: ask one person, ask the group, not under this
policy. It is counted from Mira's two recorded decisions, never from anyone
watching anyone. A different subject, the drying rack, starts at the bottom
again.

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
| 6 | the helper's record, then January | `voice agent:helper`, `overdue` | standing with a horizon; rulings take standing: `dismiss` without it is recorded and not applied; horizons on grants and on carried contradictions |
| 7 | a non-cook's kitchen rule becomes a proposal and then a rule; the helper and then Theo try to change how the kitchen makes rules; the garden's rule carried twice | `add --scope`, `approve`, `ratification set`, `why` | **ratification**: a write is a proposal until the scope's rule is met; changing the rule takes standing over the scope or the one above (`may_govern`) and is then itself a proposal under the rule it replaces; `twice` waits for turnover |
| 8 | what Mira decided not to have | `voice human:mira` | standing queries; `silence` as data |
| 9 | a year of the house against Ostrom's eight | `replay --brief` | `Log → Canon → policy → Decision`, pure |
| 10 | what a different rule would have done | `replay --policy default --brief` | policy as a pure function; decided twice, diffed |
| 11 | fourteen commons, one spine | `cargo test --test transfer_bar` | the same nine primitives; only the nouns change |
| 12 | the founding documents, cold | `draft --replay <run>` | resolvers, at 12,672 words |
| 13 | 283 tensions, four read aloud | a reader over the run file | — |

### The fixtures are ledgers, and the ledgers are verbs

**Fernwood Commons** (acts 6–10) is a `.canon/acts.jsonl` written in seed
dialect: 8 `assert`, 15 `grant`, 4 `policy set`, 2 `scope`, 1 `adopt`,
1 `silence`; the scenario adds three more `grant`s for the garden. The scenario then does what a house does over a year, in
canon's verbs: `position` (an agent's objection), `dismiss`, `accept` with a
`horizon`, `policy set` on the kitchen's own scope, `decide` twice, a
`draw commit` / `draw seal` / `draw open` sequence for a panel, and at the end
`ratification set` on the kitchen, a non-cook's `add` that stays a proposal
until both cooks `approve`, the helper's proposal that a cook refuses, and
the garden's `ratification set twice:turnover:standing` with a rule carried
once, waiting, and carried again after Kit is granted the garden.
`expected.json` is what those verbs must produce. No model ever touched it.

```sh
canon replay fixtures/fernwood-commons            # 71 step(s), all as expected
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
pinned in [PROVENANCE.md](../fixtures/maple-house/PROVENANCE.md). Eleven
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
`assert` acts, each the text and citation of a candidate from the recording
(indices 0, 1, 2, 4, 26, 29), which is exactly what pressing accept during
`draft` would have written. `demo.sh` materializes them with

```sh
canon replay fixtures/maple-house/accepted --out <dir>/.canon --profile house
```

and `canon why` on any one of the six shows the passage it was cut from
(`list` shows the text only).

### Tolerance

| kind | acts | reproduces |
|---|---|---|
| **ledger verbs** | 1, 5–11 | byte for byte, any machine, no endpoint |
| **recorded** | 2 (the rest), 12 | byte for byte from the recording; re-cutting gives a different candidate set |
| **live** | 2 (first passage), 3, 4 | wording varies; the shape holds: four guest rules citing 3-8, two conflicts on the cousin, the two reversals as tensions |

The live half of act 2 and acts 3–4 need an OpenAI-compatible endpoint that can enforce a JSON schema
(`canon config set endpoint <url>`, or `CANON_ENDPOINT`). Rust 1.95 is pinned
in `rust-toolchain.toml`; Python is stdlib only. To put a number on your own
model rather than trust ours, `./scripts/draft-bar.sh 3` runs three recordings of
Maple House and `./scripts/score-bar.sh maple-house` scores them against
`truth.json`, naming the model that produced each.
