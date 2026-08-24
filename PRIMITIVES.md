# The primitives — what a governance library must expose, and what it must not

**Status: design, ahead of implementation.** Each primitive below is marked
`BUILT`, `PARTIAL` or `NOT BUILT`. A document that reads like a description of
working software while describing an intention is the same defect as a green
test that never ran, so the marks are load-bearing and must be kept honest.

Companion to `SPEC.md`, which fixes the wire format. This fixes the *shape* —
which questions the library answers and which it refuses to answer on a
community's behalf.

## The thesis

A governance tool cannot make people good, brave, or fair. What it can do is
change relative costs. Every primitive here earns its place by making some
prosocial act cheaper — raising a concern, citing a rule, admitting a
contradiction, changing your mind in public — or some antisocial act more
expensive: acting without grounds, deciding quietly, re-litigating a settled
thing, folding your proposal into someone else's approval.

That framing is the acceptance test for anything proposed for this library.
"Does it make the better path easier and the worse path harder?" If a feature
does neither, it belongs in a caller, not here.

The second constraint is that this is a library for **configuring** governance,
not a governance system with settings. The line between the two is the line
between mechanism and policy, and most of what people call governance turns out
to be policy over a very small mechanical core.

## The line

**Mechanism** is what must be true for any policy to be trustworthy: that the
record cannot be quietly rewritten, that a justification names something real,
that absence is reported rather than defaulted. A community may not configure
these away, because configuring them away doesn't produce different governance,
it produces the appearance of governance.

**Policy** is every question with a defensible range of answers: how many
objections make a conflict, who may decide what, whether questions may be
anonymous, what happens when nothing bears on a proposal. These are not our
call. Communities differ, and a library that answers them has become a product.

Everything below is sorted into one or the other, and the sorting is the
substance of this document.

---

## Primitive 1 — The ledger

`BUILT` (`canon-core`: `act.rs`, `log.rs`, `fold.rs`, `id.rs`)

Append-only. Content-addressed ids derived from `(ts_unix, actor, body)`.
Duplicate acts collapse, so union merge is exact rather than heuristic. The
fold is independent of arrival order, so two branches merge identically either
way. Revert tombstones an act *and its effects*, and reverting a revert
reapplies the original. A format version greater than the reader understands is
refused rather than partially interpreted.

**The failure it prevents:** a record that can be edited after the fact is not a
record, it is a claim. Every accountability property downstream — that a
decision once made stays made, that you can see who decided, that nobody can
quietly relitigate — rests here and nowhere else.

**It has no opinion about governance and must not acquire one.** This layer is
what makes a house canon and a codebase canon the same object.

## Primitive 2 — Four structural ops, closed and strict

`BUILT` (`ActKind::{Assert, Supersede, Retract, Revert}`)

Something enters, something replaces something, something leaves, something is
undone. These change what is *live*, so an unknown one must be refused rather
than skipped: a peer silently dropping your retraction is a correctness
failure, not a compatibility inconvenience.

Closed set, therefore an enum.

## Primitive 3 — Open annotations, forward-compatible

`BUILT` in format v2 (`act.rs`: `STRUCTURAL`, `KNOWN_ANNOTATIONS`,
`ActKind::Annotation`; `fold.rs`: `Canon::carried`)

**Corrected from "registry".** What this needed was not a registration API but
a namespace split with a carried fallback: known ops are read strictly, and an
op from a build ahead of this one is preserved verbatim and left uninterpreted.
Interpretation is added where a kind is understood, which is the policy layer,
not a table of constructors.

Everything that is not structural is a typed statement *about* a commitment or
a pair of them. Accepting a contradiction. Dismissing a detector's false
positive. Recording a gap. Recording ancestry. Seconding a question without
attribution. Granting scope. Marking a trial period. Declaring that something is
deliberately left unwritten.

Unknown annotation kinds must be **carried but not interpreted** — the opposite
rule from Primitive 2.

**Why this changed:** the note on `Question` in `act.rs` explained it was added
inside v1 "because an unknown `op` is refused rather than skipped, which makes
every new act kind a breaking change." That rule is right for structural ops and
much too strict for annotations. Under it, a community could not add a
governance move without forking the format — which meant the library shipped one
opinion about what moves exist.

**Carried is not ignored**, and that is the §4.3 deviation this repays.
`Canon::carried` records every uninterpreted annotation by op, and `canon log`
and `canon list` report them. Extensibility that answers as though it had read
everything is worse than refusing to read.

## Primitive 4 — Resolvers: text in, typed evidence out, never a verdict

`PARTIAL` (`locate`, `quantify`, `subject` all implement this contract
independently; it is a convention, not a trait)

A resolver reads open text and returns typed structure. Code compares the
structure and decides. `locate` returns a *position* and code cuts the quote.
`quantify` returns quantities and code compares canonical forms. `subject`
returns a *partition* and code compares integers.

Three modules derived this independently, each after a failure that came from
letting a model hold the decision. It should be a trait with one rule, not a
pattern each new module rediscovers at its own cost.

**The failure it prevents:** a model asked to *guarantee* something will
sometimes fail to, plausibly and without saying so. Structure it produces can be
checked; a verdict it produces cannot.

Configurable: which resolvers run, which model, whether any run at all.
Not configurable: that code holds the decision.

## Primitive 5 — Positions: cited evidence with a source, a direction and a reason

`BUILT` (`standing.rs`: `Position { source, pull, because }`,
`Source::{Commitment, Actor}`, `Standing::cited`; `act.rs`: the `position` op)

A position names something real — a commitment the canon holds, or an actor —
says which way it pulls, and says why. `Standing::cited` drops any bearing citing a commitment that does
not exist, and **returns what it refused** rather than silently rendering a
shorter answer.

**The failure it prevents:** an agent — or a person — writing its own permission
slip. "This is supported by our hospitality principle" is an assertion. A
bearing that survives `cited` is a citation. That difference is the whole
distance between a governance system and a rubber stamp.

**Requiring `because` is not decoration.** Forcing reasons into public form
constrains people to arguments framed as group-interested, and that constraint
shapes outcomes even when the reason is insincere — what Elster called the
civilizing force of hypocrisy.

**Two source kinds, and the split earns more than voting.** A bearing runs from
a commitment; a vote, objection, second or delegation runs from an *actor*.
Modelling only the first was what made every voting technology look like new
mechanism.

It also decides, structurally, what an agent may do. Citing a commitment is a
READING — an agent's whole job — and is not an adjudication. Taking your own
position is a STANCE, and under a consent policy one reasoned objection blocks,
so an agent that may object may veto. `fold.rs` therefore flags a
machine-authored `Position { citing: None }` as unattended and a
`Position { citing: Some(_) }` as ordinary work. "Agents draft, ask and cite;
they do not adjudicate" stopped being a sentence to remember and became a
consequence of the type (§7).

The act carries no actor field: for an actor-sourced position the source IS the
act's own `actor`. Two places naming who did something is the duplicated decider
that diverges quietly (§10.6).

## Primitive 6 — Boundaries: who holds standing, over what

`BUILT` (`scope.rs`: `Scope`, `Grant`; `act.rs`: the `grant`, `withdraw` and
`scoped` ops; `fold.rs`: `Canon::{who_decides, standing_of, scope_of}`)

Ostrom's first design principle, from the study of common-pool-resource
institutions that endured for centuries, is clearly defined boundaries: who
holds rights, and over which resource. Not her eighth. Her first.

We initially filed this as a gap to close later. That was wrong. Systems
without it do not endure, and every richer policy — subsidiarity, sortition,
scoped authority, cohort re-ratification — is unstateable without it.

Modelled as annotations (Primitive 3) consumed by policy (Primitive 7), not as
new structural ops. A scope grant is a typed statement in the ledger like any
other, which means it is itself citable, contestable and revertible.

A scope is a dotted path, so `house` covers `house.kitchen` — **nesting is
Ostrom's eighth principle for free**, and a policy preferring the deepest grant
is subsidiarity with no extra machinery. `covers` requires the boundary to be a
dot: `house` does not cover `household`, and a bare prefix test would hand
someone authority over a scope that merely spells similarly.

Standing is **held, not remembered**: a lapsed grant stays in the record as a
fact and stops answering `who_decides`. Rotation is therefore the default shape
rather than a reform somebody has to propose against an incumbent.

**Withdrawal removes grants at or below the named scope, and carving a hole out
of a broader grant is deliberately not expressible.** Stepping back from
`house.kitchen` while holding `house` would need a negative grant, and a
permission system with both grants and denials is one where nobody can answer
"may they?" by looking. Re-grant narrower instead.

## Primitive 7 — Policy: a pure function from evidence to outcome and authority

`BUILT` (`policy.rs`: `Rule`, `Authority`, `Attributes`, `Decision`, the
`Policy` trait; `act.rs`: the `policy`, `decided` and `rank` ops;
`fold.rs`: `Canon::{policy_for, prior_decisions, rank_of}`)

Ships as one enum rather than one struct per policy. The set this library
ships is closed and has to serialize — a policy lives in the ledger, not in a
config file — so it is an enum (§2.1); `Policy` stays a trait so a caller can
add one without forking. `Rule::{Graduated, Entrenched, Cautious}` wrap another
rule rather than replacing it, and the wrapping can only make an answer
stricter: `Authority` is ordered and a modifier takes the max. A wrapper that
could soften what it wraps would make entrenchment a way to weaken a rule.

`Standing::outcome()` delegates to `policy::default_outcome` and does not
re-implement it. That is the §10.6 risk this primitive carried, and it is
pinned by a table test that asserts both callers give the same answer for
every shape of evidence the old inline rule could see.

```
(bearings, proposal attributes, actor standing) -> outcome, authority
```

Today `Outcome` is derived one way: supported when bearings exist and none pull
against, conflicts when any do, unaddressed when none bear at all. That is *a*
policy, and it is currently welded in.

Making it a function turns every governance question we have discussed into
configuration rather than a feature request: two objections to conflict; a
principle outranking a rule; irreversible effects requiring a human act
regardless of outcome; anonymous questions with attributed adjudication; the
kitchen group deciding kitchen rules.

**Authority must be graduated, not binary.** Ostrom's consistent finding is that
durable commons use mild-first escalating responses, and that both zero
enforcement and harsh first-strike enforcement fail. So the output is a ladder —
act; act and notify; ask one person; ask a panel; refuse — not a boolean.

**A single policy per canon is the wrong shape.** Buchanan and Tullock showed
the optimal decision rule minimizes the sum of external costs (decisions imposed
on you) and decision costs (the effort of participating), and that ratio differs
by decision type. Hence *proposal attributes*: a proposal is a string plus
attributes, so a policy can say "irreversible and unaddressed means refuse and
escalate" without the library ever learning what a door is. Effect
classification is the caller's job; the attribute is the interface.

## Primitive 8 — Two standing queries

`BUILT` (`tensions`; `horizon.rs`: `Canon::{overdue, unreadable_dates}`, the
`horizon` op, `Due::{Horizon, Revisit, Standing}`)

The generalization held: one query returns a lapsed term limit, a trial period
and a revisit date, and a test asserts all three come back from a single call.
Three queries would have meant three mechanisms.

The clock is passed in, never read. `overdue(now)` is pure, which is what
`canon replay` depends on completely.

**What contradicts what.** Governance by accretion means commitments accumulate
one decision at a time and begin to conflict. Without this the canon silently
becomes incoherent while still reading as authoritative.

**What has gone stale.** Any annotation may carry a horizon, and one query
answers what is overdue. The comment on `Accept.revisit` — "a date that has
passed is not noise; it is the signal" — is exactly right and is currently
applied to one act kind. Generalize it. Systems like this rot because things
accumulate and nothing closes; a closure query is the cheapest available
defense, and it is the difference between deferring and burying.

---

## Primitive 9 — A fair draw

`BUILT` (`draw.rs`: `Canon::draw`, `Drawn`, `DrawError`; the `draw_commit`,
`draw_secret` and `draw_reveal` ops)

**The threat model came first, and it changed the design.** What is written
below was enumerated before any code, and the sketch this document carried —
"a draw act names the pool, the count, and a seed source that already existed
and was authored by someone other than the drawer" — did not survive it.

Selection of people by lot, in a way nobody could steer.

This is the only entry on this list that was not designed in advance. It fell
out of the adequacy test below: every other technology of political economy we
tried decomposed into Primitives 1-8, and sortition did not. It needs randomness,
and randomness is exactly what a content-addressed, replayable ledger cannot
casually have — a draw nobody can reproduce is a draw nobody can audit, and a
draw seeded by whoever calls it is not a draw.

What is required is a seed **nobody chose after seeing the pool**.

Getting this wrong is not a small defect. Sortition is the answer this document
leans on for two separate problems — Freeman's entrenchment, and the minimal
governance ask — and a steerable lottery is worse than no lottery, because it
launders a chosen panel as a fair one.

### The threat model, written before the code

The design under attack, restated so the attacks have something to bite:

1. `draw_commit { scope, count, after_ts }` names the pool by scope, how many
   seats, and a boundary that must be **strictly in the future** when it is
   written.
2. Anyone in the pool writes `draw_secret { commit, digest }` **before** the
   boundary. The digest is `sha256` of a secret they keep.
3. After the boundary, they write `draw_reveal { commit, secret }`.
4. **The draw is a query, not an act.** Seed is `sha256` over the commit id and
   every verified `(actor, secret)` in sorted order; selection is a
   seed-keyed Fisher-Yates over the frozen pool. Anyone replaying the log
   computes the same panel.

Point 4 is what the threat model forced. The original sketch seeded the draw
from **the first act after the boundary not authored by the drawer**, and that
does not survive attack (e) below: an act's id is a hash of its own body, so
whoever writes it can try bodies until the shuffle favours them. Hashing is
cheap and a pool of twenty needs a few hundred attempts. The race to be first
is not a defence — most people write acts rarely, and an attacker writes at
`after_ts + 1`.

| # | Attack | Closed by | Test |
|---|---|---|---|
| a | Grind `after_ts` onto a favourable seed | the boundary must postdate its own commit act, and no secret is revealed when it is chosen — there is nothing yet to grind toward | `a_boundary_in_the_past_is_refused` |
| b | The drawer seeds their own draw | there is no seed act; the drawer's only move is the commit, and a drawer in the pool contributes one secret like anyone else | `the_drawer_has_no_move_after_committing` |
| c | Pool churn between commit and draw | the pool is frozen at `after_ts` — grants live then, not now | `standing_granted_after_the_boundary_does_not_join_the_pool` |
| d | Empty window — nobody revealed | **refuse.** A draw with no verified secret is not a draw with a default seed | `a_draw_with_nothing_revealed_refuses_rather_than_falling_back` |
| e | Grind the secret | a digest is committed before the boundary and checked on reveal; a mismatch excludes that actor | `a_revealed_secret_that_does_not_match_its_digest_is_refused` |
| f | Commit several digests, reveal the flattering one | **first digest per actor wins**; a later one is not read | `a_second_secret_from_the_same_actor_is_ignored` |
| g | Two replayers disagree | the seed and the shuffle are pure functions of the log | `two_replayers_draw_the_same_panel` |
| h | Draw more seats than there are people | **refuse.** A draw that selects everyone is not a draw, and shipping one would launder an unselected group as a chosen one | `drawing_more_seats_than_the_pool_holds_refuses` |

### The residual, named

**The last revealer has one bit.** Whoever reveals last can compute the panel
that results from revealing and compare it with the panel that results from
staying silent, then choose. They cannot grind — their secret is committed —
so the influence is one bit, exercised once, and it costs them their own seat:
an actor who committed and did not reveal is excluded from the pool.

This is the standard result for commit-reveal without an external beacon and it
does not close with a ledger alone. It is bounded, it is visible in the log —
a commit with no reveal is a fact anyone can see — and it is the price of
having no network dependency. A community that wants it closed needs a public
randomness beacon, which is a different tool.

**The draw is only as fair as the pool, and the pool is a scope.** Standing
granted before the boundary counts, which is deliberate: excluding somebody who
legitimately joined last week would be the worse failure. It does mean the
question "who may be drawn" is answered by grants, under whatever policy
governs them — which is where it belongs, and where it is visible.

## Does the set actually span? An adequacy test

The claim this document makes is that governance is policy over a small
mechanical core. That claim is falsifiable, and this is the test: **take the
known technologies of political economy and try to express each as a composition
of the primitives.** Anything that requires new mechanism is evidence the set is
incomplete.

The point is not novelty. These techniques are old, studied, and mostly
well-understood; what has been missing is a substrate where adopting one is
configuration rather than a rebuild. A community should be able to layer
sortition onto what it already does the way one adds a dependency.

| Technology | Composition | Verdict |
|---|---|---|
| Consent, not consensus | policy over positions where `Against` requires a reason | spans |
| Majority / supermajority / unanimity | policy counting actor-sourced positions | spans\* |
| Quorum, thresholds | policy over position count and standing | spans |
| Veto and minority protection | policy | spans |
| Subsidiarity | nested scopes + policy routing to the lowest competent one | spans |
| Delegation / liquid democracy | annotation (A delegates scope S to B, with a horizon); policy resolves the chain | spans |
| Term limits, rotation | scope grant carrying a horizon + the staleness query | spans |
| Sunset clauses, trial periods | annotation carrying a horizon + the staleness query | spans |
| Recall, impeachment | retract a scope grant | spans |
| Appeal, escalation | policy returning a higher scope in the authority ladder | spans |
| Entrenchment (constitution harder to amend than statute) | rank as an annotation; policy reads it | spans |
| Per-actor budgets, quadratic voting | fold over actor-sourced annotations | spans |
| Precedent, distinguishing, overruling | the core: assert, supersede, tensions, the fold guards | spans |
| Cohort ratification | `Adopt` + generation + scope | spans |
| Deliberative minipublics | fair draw + scope grant with horizon + a question batch | needs Primitive 9 |
| Sortition | a draw nobody can steer | **does not span** |
| Graduated sanctions | authority ladder + a count of prior decisions | spans, with a caveat below |
| Futarchy, staked prediction | positions carrying transferable stakes | out of scope — no value transfer here, and we are not adding it |

\* Spans only once positions can be actor-sourced — see the first finding.

Three findings, and they changed the primitives rather than decorating them.

**Positions have two source kinds, and only one exists.** A `Bearing` runs from
a *commitment* to a proposal. A vote, an objection, a second, a delegation all
run from an *actor* to a proposal. Those are different relations, and today only
the first is modelled — which is why the entire voting family looked like it
needed new mechanism. Generalize `Bearing` into a position whose source is
either a commitment or an actor, keep `because` required on anything that pulls
against, and eight rows of that table become policy rather than features. This
is the highest-leverage change on the list and it is small.

**Graduated sanctions collide with the line we do not cross.** Ostrom's fifth
principle needs to know that this is the third occurrence, and counting
occurrences by person is precisely the surveillance file this document forbids.
The resolution is a real distinction rather than a compromise: **the ladder
counts prior decisions, not prior observations.** "The house asked Dana to stop
doing X" is an adjudication, attributed to whoever decided it, and belongs in
the record. "Dana ran the washing machine at 1am" is an observation about a
person and does not. A community that has never decided anything has no ladder
to climb, which is the correct behaviour.

**One generalization pays for five technologies.** Term limits, sunset clauses,
trial periods, revisit dates and rotation are all the same shape: an annotation
carrying a horizon, plus one query for what is overdue. That is the strongest
evidence in this document that Primitive 8 is a primitive and not a feature.

## The motions people already make

The other reason to prefer a small core: **the primitives should name what
people already do, not ask them to do something new.** Communities already ask
questions, already decide, already object with reasons, already defer things,
already delegate, already withdraw, and already record some of it — in minutes,
a spreadsheet, a chat thread, a lease.

Every primitive here corresponds to a motion that exists organically:

- Asking → `Question`
- Deciding → `Assert`, `Supersede`, `Retract`
- Objecting, with a reason → a position pulling against
- Agreeing to disagree → `Accept`
- Handing something to someone → a scope grant
- Stepping back → a scoped withdrawal
- Changing your mind → `Supersede` with a rationale
- Undoing → `Revert`

Nothing on that list is a new behaviour. The library's contribution is that
these motions become *records with a shape*, so a policy can be written over
them — and so a community that wants sortition, or consent, or subsidiarity, can
adopt it without changing how anyone behaves day to day.

That is the whole design intent: the technologies of political economy are old
and mostly sound, and the barrier to adopting them has never been that nobody
understood them. It is that each one has meant building an institution around
it. If the records people already keep carry enough structure, adopting one
becomes a configuration change.

## What is policy, and therefore not ours

Stated explicitly so that adding any of these to the core is recognizable as a
mistake rather than as progress:

- How many bearings against constitutes a conflict.
- Whether silence is consent. (We ship an opinion — see below — we do not
  enforce one.)
- Who may perform which act, and over what scope.
- Which act kinds may be unattributed.
- What outcome grants what authority.
- Quorum, thresholds, revisit intervals, escalation ladders.
- Whether a class of proposal needs a human regardless of outcome.
- How an answer is phrased.

That last one needs untangling before anything else is built on it. Profiles
today conflate **voice** (how an answer reads) with **policy** (what the answer
is). Split them, or every future policy knob arrives welded to a rendering
concern.

## Defaults are governance

We intend to ship strong opinions, loosely held. The behavioral literature is
unkind to the second half: defaults are extraordinarily sticky, and most
adopters never change them. Whatever ships as default *is* the governance for
nearly everyone, and calling it loosely held describes our intentions rather
than the outcome.

The mitigation is recursive and cheap, because the machinery already exists:
**the default policy is itself commitments in the canon.** Not a config file
beside it. Then how a community governs is subject to `check`, to tension
detection, to `supersede` with a rationale, and to a visible diff against the
lineage it was forked from. A default you can run `canon why` against is
genuinely loosely held. One living in a TOML file is not.

The default policy we intend to ship is **consent, not consensus**: silence is
consent, and one reasoned objection blocks. Taken from sociocratic practice,
where an objection must be argued as harm to the group's aims rather than stated
as a preference — which is close to what `Bearing { pull: Against, because }`
already is. For a group prone to non-confrontation this is the right default:
the passive are not required to act, and one person with an actual reason can
still stop a thing.

## What this makes cheaper, and what it makes dearer

Cheaper:

- **Raising a concern without accusing anyone.** A `Question` records a gap in
  the canon, not a complaint about a person. "Is it okay to run the washing
  machine after midnight" is answerable without anyone having been wrong.
- **Finding out you are offside, in private, before proposing.** Most conflict
  avoided is conflict that never needed to happen.
- **Holding a contradiction knowingly.** `Accept` with a required rationale and
  a revisit date gives a third outcome that is neither winning nor losing, which
  changes the risk calculus of raising anything at all.
- **Changing your mind.** `Supersede` carries a rationale and leaves the old
  commitment visible as superseded rather than erased.

Dearer:

- **Acting without grounds.** `Unaddressed` is not an approval, and the doc
  comment says so. A proposal engineered to cause harm is, almost by
  construction, one no commitment supports — so it lands in the one outcome that
  cannot authorize anything.
- **Deciding quietly.** Adjudication is expected to be human-authored, and
  `unattended` surfaces what was not.
- **Re-litigating a settled thing.** The ledger remembers so that nobody has to
  re-fight, and changing a decision costs a rationale and leaves a trace.
- **Laundering a proposal through someone else's approval.** Near-identical
  proposals fold, which is what keeps the review burden survivable under volume
  — and the fold guards are what stop that from being weaponized.

## Voice must be cheaper than exit

Hirschman's claim is that members respond to decline through exit or voice, and
that available exit reduces investment in voice. A community with high turnover
has cheap exit, and it is tempting to read that as a reason to doubt whether
voice is the operative response at all.

That is the wrong conclusion, and it is not testable the way it sounds: you
cannot learn whether voice would work with better tools by measuring voice under
worse ones. **Voice has to be the response. Making it cheaper than exit is the
design target, and it is the only lever we hold.**

The two are not costly on the same axis, which is what makes the target
tractable. Exit is expensive in logistics — a new place, a deposit, a move — and
**cheap in exposure**: nobody confronts you, you simply go. Voice is nearly free
in logistics and **expensive in exposure**. So convenience is not the binding
constraint and optimizing it moves nothing. Every affordance that works here
reduces *exposure*:

- A `Question` records a gap rather than accusing a person.
- `check` runs privately, so you can discover you are offside before proposing
  and adjust without anyone knowing you considered it.
- Unattributed asking, with attributed adjudication.
- Unattributed seconds, so you learn you are not alone without going first.
- `Accept` as a real outcome, so raising something is not a bet you can only
  lose.

Three consequences follow that are not obvious from the primitives alone.

**Loyalty is manufacturable, and the ledger is the mechanism.** Hirschman's
third term is less sentiment than the expectation that voice will work. That
expectation is built from evidence, and the log already holds it: a member
should be able to see voice's track record — questions asked, which became
commitments, what changed. Not a vanity metric. It is what makes the next
person's decision to speak rational.

**Exit is silent by default, and the silence is the damage.** The person leaves
and the reason leaves with them. An offboarding that mints an unattributed
`Question` converts a departure into a governance signal, which is the cheapest
available salvage of an exit nobody prevented.

**Exit is gradual before it is total.** People withdraw from the common table,
stop hosting, stop coming to meetings. Those are exits from *scopes*, and once
boundaries (Primitive 6) exist, scoped withdrawal becomes a recordable move
rather than an invisible one — which surfaces the pre-exit signal without
demanding a confrontation from someone already disengaging.

The limit is real and worth stating: we work only on voice's side. Lowering
exit's cost is not ours to do, and raising it — making a community harder to
leave — is not governance. It is a trap, and it produces the compliance that
looks like consent, which is the failure this whole document is written against.

## The line we do not cross

**No person-attributed observations in the canon, ever.** Acts record who
*decided*. The content must never record what a named member *did*. The moment
the log holds "Dana ran the washing machine at 1am" this has become a
surveillance file, and the first time a `why` output appears in an argument as a
gotcha, people stop asking questions and every property above inverts.

Incidents inform which commitments get minted. They are not stored as evidence
against anyone.

## The tensions, and the floor under each

A tool cannot determine how it is used. It can make one direction cheaper than
the other, and it can refuse to supply the thing an abuse would need. That is
the whole of what follows: for each unresolved tension, what the tool cannot do,
and the floor it holds anyway.

None of these are closed. A design that claims to have solved the problems
political theory has argued over for a century is not loosely held.

### Legibility versus métis

Scott's case is that formalizing local practice destroys the tacit, adaptive
knowledge that made it work. High modernism's failure needed three ingredients:
a totalizing map, authority to impose it, and contempt for local knowledge as
unscientific. Michael Polanyi's tacit knowing and Hayek's dispersed local
knowledge describe the same substrate from other directions — and Hayek's point
is the useful one here, that a mechanism can *coordinate* on local knowledge
without *extracting* it.

**What the tool cannot do.** It cannot stop a community from trying to write
everything down, and it cannot supply the judgment that unwritten practice
carries.

**The floor.**

*It does not aim at total coverage.* `Unaddressed` is a healthy state with a
floor rather than a defect with a target, and nothing here should optimize canon
size, coverage, or the number of resolved tensions. A canon that answers
everything has replaced judgment with lookup.

*Standards are first-class, not failed rules.* Kaplow's distinction is the
useful frame: rules are specified in advance and cheap to apply but fit badly;
standards are specified at the point of application and fit well but cost when
applied. Métis lives in standards — "guests are your responsibility", "leave it
better than you found it" — and their underspecification is the point. The
resolvers must never pressure a commitment toward being sharpened into a
measurable rule. A system that quantifies will drift toward the quantifiable
unless told not to.

*Deliberate silence is recordable.* A community can assert that it has decided
**not** to write a rule about something. That converts silence-by-neglect into
silence-by-choice, and it protects the unwritten from being read as a gap
awaiting closure. Costs nothing under Primitive 3, and it is the most direct
thing this document can do for métis.

*The canon is never the sole input.* The authority ladder can return "ask a
person" as an outcome, which keeps human judgment inside the loop by design
rather than by omission.

**What remains human.** Whether the community has the restraint to leave things
unwritten. No mechanism supplies that.

### Minimal structure versus hidden power

Freeman's argument is that groups refusing explicit structure do not become
unstructured — they develop informal, unaccountable elites, and the absence of
written rules protects incumbents. Michels' iron law is the pessimistic baseline:
organizations tend toward oligarchy regardless of intent.

The mechanism of informal power is worth stating precisely, because it is what
determines whether a tool can touch it at all: **informal power runs on private
knowledge of the rules and the process.** Who decides. What was decided before.
What is pending. Why things are done this way. An informal gatekeeper is someone
who knows the process and can dispense that knowledge selectively. Freeman's own
prescriptions — rotation, delegation by explicit procedure, distribution of
authority, and *diffusion of information* — attack exactly that.

**What the tool cannot do.** Power will operate outside any record. Friendships,
side conversations, and who was in the kitchen at the time are not addressable
by software and never will be.

**The floor.**

*Everything that governs is in the ledger and readable by the governed* —
including the policy itself. You cannot be the gatekeeper of "how things work
here" when how things work is queryable.

*"Who decides this?" must be answerable without asking a person.* If finding out
who has authority requires knowing whom to ask, that person is the informal
elite. This is derivable from Primitives 6 and 7 but is called out because it
must exist as a surface, not merely be implied.

*Decisions made by nobody are surfaced.* `unattended` is the drift detector, and
drift is informal power's favourite instrument — the decision that simply
happened, which no one can be asked to justify because no one made it.

*Authority expires.* Scope grants carry horizons, so rotation is the default
shape rather than a reform someone has to propose against an incumbent.

**What remains human.** Whether anyone looks. A readable record that nobody
reads protects no one, and the literature offers no mechanism for curiosity.

### Monitors accountable to whom

Ostrom's fourth principle requires monitors drawn from, or accountable to, the
people governed. An agent fleet paid by hosting fees and judged on task
completion is neither. This is the ordinary principal-agent problem with the
usual asymmetry: the agent has better information and different incentives.

**What the tool cannot do.** It cannot align an agent's incentives with a
community's interests, and it cannot make a community capable of evaluating work
it does not understand.

**The floor.**

*Discretion is bounded structurally, not by instruction.* An agent cannot act on
`Unaddressed`, so its authority is a function of the canon's coverage rather
than of its own confidence. This is the single most important line, because it
is the one an eager agent cannot argue its way past.

*Agents may draft, ask and cite; they may not adjudicate.* Today this is
reported after the fact. It should be gateable: policy must be able to require a
human act for a class of decision, and the shipped default should require one
for adjudication.

*The monitor's record is auditable by the governed.* Every agent action is
attributable and replayable, and an agent cannot quietly rewrite what it did.
That is Ostrom's answer — appropriators monitoring the monitor — and the ledger
already provides it.

*The agent's job is itself a scope grant with a horizon.* This closes the loop
neatly: an agent fleet's authority is granted, scoped, expiring, and renewed by
humans, and its case for renewal is the auditable record of what it decided,
cited and refused. "Keeping the job" stops being a motivation the agent holds
privately and becomes a decision the community makes on evidence.

**What remains human.** Someone has to actually review the record before
renewing. The tool can make the evidence cheap to obtain; it cannot make anyone
weigh it.

### Enforcement

Not a tension so much as an undecided question, kept here so it is not mistaken
for an oversight. The adequacy test showed a graduated ladder is modellable
without a surveillance file, since it counts prior *decisions* rather than prior
*observations*. What nobody has decided is whether this library should carry an
enforcement ladder at all, or only ever advise while humans act. Ostrom found
that both zero enforcement and harsh first-strike enforcement fail; that is an
argument for the middle, not for us picking it on a community's behalf.

## What we are borrowing, and from whom

Not a bibliography. Each of these changed something above.

- **Elinor Ostrom**, *Governing the Commons* (1990). The eight design principles
  for enduring common-pool-resource institutions. Promoted boundaries to
  Primitive 6, made authority graduated in Primitive 7, and supplied two of the
  open tensions.
- **Albert Hirschman**, *Exit, Voice, and Loyalty* (1970). The design target:
  voice cheaper than exit, on the exposure axis rather than the convenience one.
  Loyalty as the expectation that voice will work, and therefore as something a
  visible track record can build.
- **Jo Freeman**, "The Tyranny of Structurelessness" (1972). Why "minimal" is
  not automatically good, and why rotation matters.
- **Timur Kuran**, *Private Truths, Public Lies* (1995). Preference
  falsification: why people do not know whether they are alone, and why
  revealing the distribution must gate attention rather than outcome — a
  cascade is not deliberation.
- **James C. Scott**, *Seeing Like a State* (1998). Legibility versus métis; the
  floor under the unaddressed rate, and the reason coverage is never a goal.
- **Michael Polanyi** on tacit knowing, and **Hayek**, "The Use of Knowledge in
  Society" (1945). The same substrate from two directions, and Hayek's useful
  half: a mechanism can coordinate on local knowledge without extracting it.
- **Louis Kaplow**, "Rules versus Standards" (1992). Why an underspecified
  commitment is a standard rather than a failed rule, and why the resolvers must
  not sharpen one into the other.
- **Robert Michels**, the iron law of oligarchy (1911). The pessimistic baseline
  under Freeman: drift toward oligarchy is the default, so rotation has to be
  the default shape rather than a reform proposed against an incumbent.
- **Buchanan and Tullock**, *The Calculus of Consent* (1962). Why one policy per
  canon is the wrong shape, and hence proposal attributes.
- **Chantal Mouffe**, agonistic pluralism. Why `Accept` is a first-class outcome
  rather than a failure to decide.
- **Sociocracy** (Endenburg and successors). Consent rather than consensus, and
  the reasoned objection as the unit of dissent.
- **Jon Elster**, on the civilizing force of hypocrisy. Why `because` is
  required.
- **The common law.** The closest structural relative: cases become holdings,
  holdings become precedent, precedent is distinguished on material facts. Two
  borrowings outstanding — `ratio decidendi` versus `obiter dicta`, so that not
  every word of a recorded decision binds equally; and the recognition that
  `subject.rs` is performing *distinguishing*, which is the same operation
  lawyers have done for centuries and not merely an analogy to it.
- **David Hume**, "Of the Original Contract" (1748). Staying is not consenting
  when leaving is costly — which is why a cohort that inherited a canon it never
  agreed to should get an explicit moment to adopt or contest it.

## Where to start

Two changes, in order, both small.

Generalize `Bearing` so a position may be sourced from an actor as well as from
a commitment. Eight rows of the adequacy table turn from mechanism into policy
the moment this lands.

Then extract the outcome computation out of `standing.rs` into a named policy
with today's behaviour as `Default`. That unlocks Primitives 6 and 7, and with
them everything in the table except the fair draw.

The fair draw wants its own design pass. Do not ship a lottery before it is
auditable.
