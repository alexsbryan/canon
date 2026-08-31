# The run of show

`./scripts/demo.sh` — `--offline` skips the model beats, `--auto` runs straight
through, otherwise [enter] steps.

**The shape.** Acts 1–3 use a model, because that is what the tool does when
you point it at a mess you already have. Act 4 unplugs, and everything after
runs with no model and no network — which lands as a *reveal* rather than as a
constraint, and only because the room has just watched the model half work.

| act | beat | command | model |
|---|---|---|---|
| 1 | point it at the folder; rules fall out quoting their source | `canon draft --replay <tape>` | **no — taped** |
| 2 | take a proposal from the room; the house's own rules answer | `canon check "…"` | yes |
| 3 | ask what it disagrees with itself about | `canon tensions` | yes |
| — | **the turn** — "everything so far used a model. Now pull the cable out." | | |
| 4 | a bot said no conflict; the house overruled it and dated the disagreement | `canon why can-5e1a8e880e1d` | no |
| 5 | what the house decided **not** to have | `canon voice human:mira` | no |
| 6 | nobody had to remember | `canon overdue` | no |
| 7 | two years replayed, 55 ms | `canon replay fixtures/fernwood-commons` | no |
| 8 | what dropping consent would have done | `… --policy default --brief` | no |
| 9 | ten commons, one 104-line spine, four broken on purpose | `./scripts/cpr-sweep.sh` | no |
| 10 | **one more thing** — the founding documents, 12,672 words, cold | `canon draft --replay <sweep>` | no |
| 11 | 283 contradictions proposed. Four of them, one of them wrong | `scripts/founding-highlights.py` | no |

**Acts 10 and 11 are live, not slides.** They replay a TAPE — 850 replies a
real 27B gave over the founding corpus during the completed sweep, run back
through the same pipeline (citation cutting, the guards, the fold). Real
output, no live risk, **2.5 seconds**, still nothing plugged in.

The contamination number is the closing line and belongs beside them —
[DEMO_PLAN.md](./DEMO_PLAN.md) Phase 1c.

## Say that acts 1, 10 and 11 are recordings

They replay a tape — real replies a real 27B gave, run back through the same
pipeline. That is not a mock and it is not a mockup, but it is also not a live
call, and presenting it as one would discredit every honest number in the rest
of the show. `demo.sh` prints the disclosure; read it out.

## The lines that carry it

- **Act 1** — *"Every proposal quotes the passage it came from, or it isn't
  shown. You go through them one at a time. There is no `--accept-all`."*
- **Act 5** — the silence is the emotional beat. *"This is what groups lose,
  and losing it is why the same proposal comes back every spring."*
- **Act 8** — hold the silence after the number. *"Every group has had this
  argument. No group has ever been able to check."*
- **Act 11** — read the first pair aloud and stop talking. All men are created
  equal, against three fifths of all other Persons. Then the fourth pair,
  which is wrong: *"It proposes things that are not there. That is why review
  is one at a time."*
- **The close** — volunteer the number that hurts. *"We asked whether it was
  reading or remembering. We removed the fact each contradiction turns on and
  asked again. Five dropped. Four didn't. We published that too."*

## Before you present

```sh
cargo build
./scripts/demo.sh --offline --auto     # acts 4-9, no endpoint needed
```

`demo.sh` preflights the endpoint and refuses in the green room rather than
failing in front of the room. Only acts 2 and 3 need it — there is no tape for
`check` or `tensions`. Everything else, including both founding acts, runs
with nothing plugged in.

**Act 1 can run from a tape**, which is every reply a real endpoint gave,
recorded, replayed through the same pipeline. Not a mock — real output, no
live risk. Cut one with `./scripts/record-demo-tape.sh`. A tape only replays
against the build it was cut on; a stale one refuses loudly rather than
answering from the wrong recording, so re-cut it whenever the call sequence
changes.

## Parked

- **Nothing.** Act 1's tape is cut (48 calls, `Qwen3.8-27B-UD-Q6_K_XL`,
  `fixtures/maple-house/runs/demo-tape/run.json`, replays in 0.03s). Acts
  10-11 got theirs from the completed founding sweep. Only acts 2 and 3 —
  `check` and `tensions` — still need a live endpoint, and `--offline` skips
  exactly those two.

  Nobody has yet looked at whether `check` and `tensions` READ well at
  projector size. That is the one unrehearsed thing left.
- **`check` and `tensions` have never been run live for the stage.** They work;
  nobody has looked at whether they *read* well at size.
