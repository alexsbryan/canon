# The CPR transfer study

**Can you take an arbitrary common-pool resource and get Ostrom governance out
of it without designing anything?**

Fourteen institutions, one spine. Ten are resources; four are ablations of
those ten, each removing one use of one primitive and naming, in advance,
which principles it expects to lose.

```sh
./scripts/cpr-sweep.sh          # the whole study, no endpoint, ~3 seconds
cargo test --test transfer_bar  # the same thing as a bar
```

| institution | what the commons is | people | levels | monitor | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `harbourside-makerspace` | shared tools and consumables | 10 | 2 | bot | · | · | · | · | · | · | · | · |
| `crosswalk-coliving` | a building, its commons and its roof | 11 | 3 | bot | · | · | · | · | · | · | · | · |
| `meridian-monorepo` | a codebase several teams live in | 9 | 3 | bot | · | · | · | · | · | · | · | · |
| `commonwealth-mesh` | pooled machines holding one model | 10 | 2 | bot | · | · | · | · | · | · | · | · |
| `northgate-buildfarm` | shared CI capacity | 24 | 2 | bot | · | · | · | · | · | · | · | · |
| `parkside-allotments` | a community garden and its standpipe | 11 | 2 | person | · | · | · | · | · | · | · | · |
| `tidepool-forum` | a moderated forum's attention | 11 | 3 | person | · | · | · | · | · | · | · | · |
| `torbel-alpine` | a Swiss village's summer pasture · *founded* | 6 | 2 | person | · | **n** | · | · | · | · | **n** | · |
| `valencia-huerta` | a gravity irrigation canal | 11 | 2 | bot | · | · | · | · | · | · | · | · |
| `alanya-fishery` | named inshore fishing sites | 11 | 2 | bot | · | · | · | · | · | · | · | · |
| `harbourside-no-boundary` | *ablation:* nobody holds the laser | | | | **x** | · | · | · | · | · | · | **x** |
| `meridian-imposed-rules` | *ablation:* the rule is set elsewhere | | | | · | · | **x** | · | · | · | · | · |
| `northgate-unwatched` | *ablation:* the monitor never lapses | | | | · | · | · | **x** | · | · | · | · |
| `crosswalk-upstream-capture` | *ablation:* upstream holds a seat | | | | · | · | · | · | · | · | **x** | · |

`·` holds · `x` does not · `n` declared inapplicable, with a reason the bar
makes the fixture prove. The eight criteria are in
[STUDY.md](../../STUDY.md#the-eight-criteria).

**The ten differ on purpose.** 6 to 24 people, two holders of the inner
boundary or five, two levels of nesting or three, monitored by a bot or by one
of the community's own people, forked from an upstream or founded outright.
The bar refuses a set whose institutions are not each structurally distinct —
ten vocabularies over one shape would be one institution in ten coats of
paint.

## What is in a fixture

Only `vocab.json` is written. Everything else is generated from it and the
shared spine, and the build refuses a vocabulary that tries to set a rule.

```
_spine/seed.jsonl.tmpl       the only place mechanism lives — 104 lines,
_spine/scenario.jsonl.tmpl   shared by all fourteen institutions
<name>/vocab.json            107-149 lines of nouns and shape. THIS is what
                             a new CPR costs. An ablation costs 8.
<name>/acts.jsonl            generated
<name>/scenario.jsonl        generated
<name>/expected.json         generated — PREDICTED, not recorded
```

`expected.json` is predicted by `scripts/cpr-build.py`, in Python, from the
policy semantics. `canon replay` is therefore a differential test between two
implementations of what should happen, rather than a recording of what did.
The one exception is a draw's `seats` and `seed`, which are hashes of the log
and are recorded under `--pin-draw`.

## Adding one

Copy a `vocab.json`, rename the nouns, and run the build. There is no step
where you choose a policy, an authority or an outcome — the spine already did,
identically, for everybody.

```sh
cp -r fixtures/cpr/harbourside-makerspace fixtures/cpr/your-commons
$EDITOR fixtures/cpr/your-commons/vocab.json      # nouns only
rm fixtures/cpr/your-commons/{acts.jsonl,scenario.jsonl,expected.json}
python3 scripts/cpr-build.py fixtures/cpr/your-commons --pin-draw
cargo test --test transfer_bar
```

If the eight do not clear for your commons, that is a finding about the
primitives and belongs in `PRIMITIVES.md`, not a vocabulary to reword.

[PROVENANCE.md](./PROVENANCE.md) says what these institutions are and,
more importantly, what they are not.
