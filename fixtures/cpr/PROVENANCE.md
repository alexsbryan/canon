# Where these institutions come from, and what is not claimed

Fourteen institutions live here. None of them is a transcript. Every one is a
**vocabulary** — actors, scopes, commitments, proposals, and the shape of the
commons — written by hand and fed to one shared spine.

## What is claimed

That the eight design principles can be composed out of canon's nine
primitives for an arbitrary common-pool resource, with **no mechanism written
per resource**; that the resulting governance behaves the same way in ten
unrelated domains; and that the instrument measuring this can go red, because
four single-variable ablations make it go red exactly where each predicted.

That claim is about the primitives, and it is checked by `cargo test --test
transfer_bar` in about three seconds with no endpoint. It is a claim about ten
shapes, not about every commons that could exist: I chose the ten, and I chose
the axes they vary along. `STUDY.md` has the list of what the spine still
fixes.

## What is NOT claimed

**That canon read any of these institutions' real documents.** It did not.
Nobody pointed `canon draft` at a Valais village archive. The vocabularies are
written; the governance is derived. Whether an Ostrom-conformant canon falls
out of a real community's actual mess is a different question, it needs a
model, and it is [STUDY.md](../../STUDY.md) leg 2 — which has produced no
number yet and says so.

**That the three historical fixtures are history.** `torbel-alpine`,
`valencia-huerta` and `alanya-fishery` are stylised from Elinor Ostrom's
published accounts of three long-enduring commons in *Governing the Commons*
(Cambridge, 1990) — the Törbel alpine grazing and forest commons of the Valais
(ch. 3), the huerta irrigation communities of eastern Spain (ch. 3), and the
Alanya inshore fishery of southern Turkey (ch. 1). The rules they carry are
recognisable — cattle wintered on your own hay, water taken in the order of
the ditch, a boat fishing the site it drew — but the names, dates, quantities
and the prose are invented. They are **models of documented institutions, not
sources**. Read Ostrom for the institutions.

They are here for one reason: the eight principles were derived from cases
like these and not from a makerspace, so an instrument that works on a
makerspace and fails on an alpine pasture would be measuring the wrong thing.

**That the four ablations are those historical failures.** Each ablation names
an analogue in Ostrom's chapter 5 on fragile and failed CPR institutions. The
analogue explains why the failure mode is worth modelling; it is not a claim
that the fixture reconstructs the case. What each ablation actually is, is one
line of vocabulary removing one use of one primitive.

## The seven modern institutions

`harbourside-makerspace`, `crosswalk-coliving`, `meridian-monorepo`,
`commonwealth-mesh`, `northgate-buildfarm`, `parkside-allotments` and
`tidepool-forum` are invented outright, and no real organisation is meant.
They exist because they are where the argument actually points: a repository
and a shared house are common-pool resources, and this study is mostly about
them.

`commonwealth-mesh` models the resource that
[Commonwealth](https://github.com/alexsbryan/commonwealth-ai) pools — machines
lent to a mesh by people who know each other. That it needed no new primitive
is the point of including it.

## Reproducing

Every file in a fixture directory except `vocab.json` is generated:

```sh
python3 scripts/cpr-build.py --all --pin-draw
```

The build is deterministic. `--pin-draw` records the two values a predictor
cannot know — a draw's `seats` and `seed`, which are hashes of the log — and
nothing else in `expected.json` is recorded rather than predicted.
