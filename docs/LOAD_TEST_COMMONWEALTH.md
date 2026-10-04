# Load test: Commonwealth's notes into canon

*Internal. What broke or chafed while moving one large, real, agent-written body of guidance into canon, kept here so it can be fixed in one pass. Started 2026-09-13 against the installed debug build of 2b9ed42 (only test files differ from v0.1.0), macOS arm64, endpoint `http://localhost:9741/v1` serving `Qwen3.6-35B-A3B-UD-MTP-IQ4_NL`.*

commonwealth-ai keeps about 12,000 notes, most written by agent sessions. Somewhere between 700 and 1,500 of them record guidance: invariants, things tried and why they failed, the operator's conventions. The goal is canon as the record for those, one rule plus its history, which agents ask before they act. The ground rule for fixes: prove an ergonomic on the Commonwealth side first, and promote into canon what holds in general form.

Each entry says what happened, where it lives, what it costs this use case, and one disposition. **fix** means canon should change. **prove** means build it on the Commonwealth side first and promote it if it generalizes. **document** means it is by design and a user should meet the reason where they meet the behaviour.

### 1. A setup script's grant does not govern the acts right after it

Grant yourself a scope, set `standing`, and write an agent's acts in the same second: the agent's add goes straight into force, its self-grant applies, and its change to the scope's ratification rule applies. The same acts a second later come back as a proposal and three NOT APPLIED lines. `Canon::may_govern` counts only grants made strictly before an act, so a canon whose grants all share that second is still bootstrapping (`crates/canon-core/src/ratify.rs:364-377`). That is deliberate and tested (`simultaneous_founding_grants_do_not_lock_the_founder_out`), but a founding script or an import runs exactly this way, gets notebook rules, and nothing on screen says why. It cost this load test one wrong conclusion. **document**, and **fix** the output: when an act is judged as bootstrap because no grant predates it, say so on the line that reports it.

### 2. A refused governance act leads with a success line

`CANON_ACTOR=agent:claude canon grant agent:claude repo.defaults` in a governed canon prints `agent:claude holds repo.defaults with no end` and only then `NOT APPLIED: agent:claude granted agent:claude standing over repo.defaults without holding it`. `ratification set` and `retract` have the same shape. The exit code is 1, which is right; the first line says the opposite. An agent or a script reading the headline takes the wrong lesson. **fix**: lead with NOT APPLIED.

### 3. An agent running the CLI writes as the person by default

The actor is `CANON_ACTOR`, else git's `user.name` prefixed `human:` (`crates/canon-cli/src/store.rs:132-142`). Every agent session on a developer machine shares that git identity, so `canon approve` from an agent is a person's approval unless the harness sets the variable. The MCP surface being read-only does not reach the CLI. Separately, `is_human` is a prefix check (`crates/canon-core/src/ratify.rs:259`), so a label is spoofable, which the README already says. **prove** on our side: every harness sets `CANON_ACTOR=agent:<harness>` and a tool hook refuses a `human:` actor and the adjudication verbs. Candidate **fix** once proven: refuse adjudication acts when stdin is not a terminal and `CANON_ACTOR` is unset, instead of defaulting to a person.

### 4. The agent surface assumes a small canon

`canon_list` takes no arguments and returns everything in force (`crates/canon-cli/src/mcp.rs:138-221`). `canon check` numbers every active commitment into one prompt (`crates/canon-cli/src/check.rs:97-107`). This use case lands near a thousand commitments, many of them written from paragraph-length incident notes, so list is a context dump and check is one prompt over the whole canon. That is the configuration measured at 1 of 11 planted tensions against 5 of 11 in blocks of twelve (`docs/CANON_CLI.md` in commonwealth-ai). A silence matches its subject exactly, by design, so "has anyone tried this" also needs lookup. **prove**: retrieval by text, symbol and file stays on the Commonwealth side and returns canon ids. Promote a scope or text filter on `canon_list` if it earns it.

### 5. A first rule cannot say why it exists

`assert` carries `text`, `from` and `source` (`docs/SPEC.md`, the structural ops table). Only `supersede` has a rationale, and `why` prints a reason only for a commitment born from one (`crates/canon-cli/src/explain.rs:142-156`). Guidance written after an incident is mostly its reason: "every prompt goes through the evidence bound" is a rule, and the 1.3 MB request that blew past it is why anyone should keep it. **prove**: hold the history on the Commonwealth side keyed by canon id. Candidate promotion: an optional rationale on `assert`, shown by `why`.

### 6. The passage behind a drafted rule stays on one machine

The verbatim quote lives in the run file (`Candidate.quote`, `crates/canon-cli/src/draft.rs:232-252`), and `.canon/.gitignore` keeps `draft-runs/` out of git. The accepted act keeps only `source` as `path:first-last` (`draft.rs:2384-2388`). On any other clone `why` points at a span of a file that may not exist, and the quote that justified the rule is gone. For a house reading its own folder that is fine; for a record other machines and agents rely on, the evidence does not travel. **prove**: commit the exported sources beside the canon on our side. Promote a quote on the assert if the history view needs it.

### 7. `add` cannot record where a rule came from

`add` takes `--scope` and nothing else (`crates/canon-cli/src/cli.rs:38`). Only the draft review writes `source`, so a proposal written by an agent, or an import that needs no model, cannot carry provenance. **prove**, then promote `--source` if the import path needs it.

### 8. `draft --resume` only resumes the newest run

`resume` sorts the run files and takes the last (`crates/canon-cli/src/draft.rs:1705-1706`). An import large enough to split across runs strands every run but the newest, even with candidates left in it. **fix**: resume every run with candidates remaining, oldest first, or take the run to resume as an argument.

### 9. A rule applies to one room only

A commitment has one scope, and the last `scoped` act wins (`crates/canon-core/src/tests.rs:1002`). 361 of the 440 invariants being imported name more than one symbol or file. **prove**: anchors stay on the Commonwealth side as index data keyed by canon id.

### 10. `who` reports the policy next to a new ratification rule

Right after `ratification set standing --scope repo`, `canon who repo.defaults` ends `decided under: default`. That line is the decision policy, not how rules get made, and beside a rule you just set it reads as though the rule did not take. **fix**: label the line, or print both.

## From the draft pilot

Pilot 1, 2026-09-13: 20 notes (8 invariants, 6 attempts, 6 migrated memories, 81,519 characters) became 56 chunks and 72 candidates — 59 rules, 12 records, 1 question — plus 13 dropped for their citation. 311 seconds wall on the endpoint above, about 5.6 seconds a chunk including the support and dedupe passes. One run, with another session sharing the daemon and its load unmeasured. The split of rules below is one reader's judgement, not a labelled bar.

Pilot 2, same day, same 20 notes and model: each note's own first line became its heading and the `**Applies to:**` boilerplate was removed. 55 chunks, 49 candidates to review (48 rules, 1 question) plus 11 records, 9 dropped, 243 seconds. On the same reading, about 26 stand alone (27 before), about 13 lose their subject (14), and about 10 describe rather than guide (18). One run each, so the deltas are a direction, not a measurement.

### 11. The width check drops a note's headline rule

Of the 13 dropped, 9 cited between 5 and 8 sentences (`crates/canon-cli/src/locate.rs:100`) and one cited 11 characters (`locate.rs:105`). One of them was the note's own headline, "build_self_manifest must derive provider.name via resolve_primary_model_name(provider), not by size_gb", cited across the eight sentences that explain it. The check is right that a wide span evidences the passage rather than the rule. The cost is that a dense note loses the one sentence a person would have picked. With the note's title as its heading (pilot 2) the same headline survived and drops fell from 13 to 9, so part of this is source shape. **fix** candidate: before dropping, ask once for the narrowest span that states the rule.

### 12. Extracted rules lose their subject

About 14 of the 59 rules cannot be read on their own: "Do not re-open it as a threshold question.", "RE-MINTING IS OWED, together with the quiet run.", "Neither end of this curve is a product." The nearest heading goes to the model as context, but these exported notes had no heading that named a subject. Pilot 2 gave every note a heading that names it, and the same three sentences came back word for word; about 13 of 49 still cannot stand alone. Heading context is not enough, so this is not a source-shape problem. Pilot 2 also copied markup into a rule — `**build_self_manifest must derive…` — which belongs in the quote and not in the text a person accepts. **fix**: write each rule so it reads without its passage, measured against the same notes before and after; strip inline emphasis from the rule text, never from the quote.

### 13. Descriptions of code come out as standards

About 18 of the 59 describe how something is built or where a project stands rather than what to do: "`sovereign-desktop/src-tauri/src/state.rs` registers 11 recipe-author tools…", "Demo config `SOVEREIGN_TITLE_EXPAND=1 SOVEREIGN_DECOMP_DECAY=0.6`." A record catches what happened; a present-tense description fits no kind and lands as the default, a rule (`crates/canon-cli/src/draft.rs:112-115`), and the code profile's voice — "a standard the code is held to" — leans the same way. Pilot 2's headings took this from about 18 to about 10, and 8 of those 10 come from one 27,034-character project log that a source filter removes on our side. **prove**: filter sources to guidance on our side. If descriptions still leak at full scale, **fix** the extraction instructions so a description is a record.

### 14. One long source becomes many candidates

Chunks target 1,500 characters (`draft.rs:68`). One migrated project memory ran to 14 chunks and produced 16 candidates, and the pilot's 20 notes put 60 in front of a reviewer. For discrete sources — a note, a decision, an issue — review cost follows length rather than the number of decisions in it. **prove**: shape and filter sources on our side, and measure candidates per note at full scale before anything changes here.

### 15. Progress lines write terminal control codes into a redirected log

`draft` and `tensions` print `\r\x1b[K` before every progress line (`draft.rs:2025`, `draft.rs:2031`, `draft.rs:2070`, `crates/canon-cli/src/tensions.rs:270`, `tensions.rs:314`) whether or not stderr is a terminal, so a background run's log is a single line of escape codes. `draft` already asks whether stdin is a terminal before prompting (`draft.rs:1593`). **fix**: plain lines when stderr is not a terminal.

## From founding the Commonwealth canon

### 16. A founding grant can name someone other than you, silently

With `CANON_ACTOR` unset the CLI acts as git's `user.name` prefixed `human:`, so an operator named "Alex Bryan" in git acts as `human:Alex Bryan` (`crates/canon-cli/src/store.rs:132-142`). A founding `canon grant human:alex commonwealth` then grants a holder who never writes: every later act from the operator is a non-holder's proposal waiting on `human:alex`, and neither the grant nor the next add says so. The Commonwealth founding (`can-630986e03a26`) avoided it only because the advice named the git spelling. **fix**: name the granting actor on the grant line, and when a person grants a different `human:` holder while holding nothing, say they will not hold the scope.

### 17. `canon init` ignores less than canon's own repository does

The `.gitignore` that `canon init` writes into a fresh `.canon/` lists `seen` and `draft-runs/`. This repository's own `.canon/.gitignore` also lists `config`, with the reason: `canon draft` on a fresh clone should say "no endpoint configured", not fail against a port on somebody else's laptop. So a canon founded with the tool commits one machine's endpoint and model name on its first commit unless someone notices. **fix**: `init` writes `config` into the ignore list, one list shared with this repository's own.

## From loading the Commonwealth canon

### 18. `--resume` can open a run that is still being drafted

A draft checkpoints to `draft-runs/<at>.partial.json` while it works (`crates/canon-cli/src/draft.rs:454`), and `--resume` takes every file whose extension is `json`, sorts by name and opens the last (`draft.rs:1697-1706`). `x.partial.json` has extension `json`, and a run started later sorts later, so while an 80-minute import is running, `--resume` offers its half-finished checkpoint and hides the finished run you meant to review. Found planning the Commonwealth load: the founding-charter run finished first and a 1,088-chunk notes run was about to start in the same canon; the workaround was drafting the notes against a staging canon and moving the finished file in. **fix**: `--resume` skips `*.partial.json`, and says it did when one is present ("a run is still being drafted"). Entry 8's resume-every-run design should inherit the same filter.

### 19. Nothing tells a harness which verbs decide

Entry 3's prove step is built for one harness. commonwealth-ai's `.claude/settings.json` sets `CANON_ACTOR=agent:claude-code`, and `.claude/hooks/canon-actor-guard.py` refuses three things before the call runs: a `canon` verb that is neither a read nor a proposal, a proposal whose resolved actor is not `agent:`, and a hand edit of `acts.jsonl` or `draft-runs/*.json`. Its suite has 47 cases, each shown able to fail against an always-allow and an always-refuse stub. To do this the hook keeps its own list of 15 read verbs, 5 `show` subcommands and 4 proposing verbs, and refuses everything else. canon has that knowledge and does not publish it. `canon help all` groups verbs by whether they need a model, so `approve` sits under RECORD next to `list`. The only read-only classification in the code pins the MCP surface (`crates/canon-cli/src/mcp.rs:35`, `the_surface_is_read_only`), not the CLI. Every harness that wants the same guard has to write the list again, and every copy goes stale the day canon adds a verb. The hook fails closed, so a new read verb is refused until someone edits it. The pi and opencode adapters in commonwealth-ai have no blocking tool hook yet, so they only get the variable once someone wires it. **fix**: canon publishes each verb's effect (reads, proposes, decides) in one table that the CLI dispatch and the MCP pin both use, readable as `canon help --json`, so a guard reads the class instead of copying it.

### 20. A draft run does not record the canon that drafted it

A run file records `endpoint`, `model` and `served_model`, and nothing about the canon binary (keys read from the charter run `draft-runs/1789319124.json`, schema `canon-draft-run/v1`). On this machine `canon` on PATH is the canon working tree's debug build, so a rebuild there changes what the next command runs. The notes draft started at 1789318705 on the build then installed, the binary was rebuilt 102 seconds later, and the working tree carries uncommitted changes to `draft.rs`. The review of that run with `--resume`, or a `--replay` of it, will run a different build, and nothing in the file or on screen says so. A candidate count that moves between the draft and a replay then cannot be attributed to the model or to canon. **fix**: record the build in the run (version, git revision and a dirty flag, captured at build time), and have `--resume` and `--replay` name it when it differs from the running build.

### 21. One runaway reading call ends a four-hour import, and nothing can pick it up

The notes draft read all 1,168 passages in 3h25m (1,745 candidates, 170 dropped for a bad citation), then failed 28 minutes into `support`. One quantities call generated until the daemon's 300-second MTP deadline (`n_generated=17221`) and came back as HTTP 503. canon correctly told that apart from a busy host and did not retry it. But `quantify_pairs` propagates the first error (`read_block(client, &flat)?`), so one bad batch out of 294 failed the whole stage, where extraction tolerates a bad reply (`draft.rs:2178`). Line numbers here are at 2b9ed42.

Nothing bounds that call. The request sets `temperature: 0.0` and no `max_tokens` (`model.rs`, `request`), and the quantities schema puts no `maxItems` on either array and no `maxLength` on any string (`quantify.rs`, `schema`).

**Correction**: this entry first said twelve batches were replayed exactly as canon sends them and none ran away. They were not exact. The replays were built in Python, which keeps a schema's keys in the order written, and canon does not: its `serde_json` is built without `preserve_order` (no `indexmap` among its dependencies in `Cargo.lock`), so `json!` sends every object with its keys sorted. Sovereign parses the schema with key order kept and compiles it with llguidance, which emits an object's properties in that order (llguidance 1.7.6, `json/compiler.rs:417`), so the replays asked for a different grammar. Sent with sorted keys, batches 119-123 give the failed run's exact lengths (340, 335, 762, 293 and 524 tokens) and batch 124 runs to a 2,560-token cap. Sent in written order they give 653, 345, 1,037, 296 and 619, and batch 124 finishes in 301. `canon draft --continue` on the failed run, sending canon's own requests, matched it on all 74 calls before batch 124 and ran away on batch 124 again.

The cause is that order. Sorted, a quantity's properties are `canonical, of, unit, value`, the reverse of the order `schema()` writes and `SYSTEM` teaches, so the model must say what a quantity measures and its unit before it gives the number. Rule 9 of batch 124 describes `passages_used + passages_dropped` and states no number. The model opened a quantity, wrote `"of": "total passages"` and `"unit": ""`, and reached the required `value` with nothing to put in it. Inside that string it copied the end of the prompt and then its own reply, escaped, as a verbatim 1,012-character cycle, and never closed it. The two replies are identical up to that first key; in written order the model writes `"value": "+"` and closes the object. The same inversion is in every schema canon sends (line numbers at f65bd38): `check.rs:76` and `rebase.rs:73` put `because` first, and extraction (`draft.rs:800`) asks for `because` before `kind` and `text`.

The larger cost is recovery. A real run records no tape (`c.recording()` only `if dry_run`, `draft.rs:2052`), so `--replay … --live-from support` refuses it (`draft.rs:1437`). Every passage was also marked read as it was extracted (`draft.rs:2159`), so a re-run in the same canon reads nothing (`draft.rs:1979`). The 1,745 candidates sit in the failed run file with no verb that continues from them, and the only way forward is another 3.5 hours of extraction.

**fix**: send each schema in the order it is written, without turning on `preserve_order` (`canon-core/src/act.rs:355` depends on sorted keys so that an adopted log re-renders to identical bytes on two machines), for example by keeping each schema as written JSON and sending it as a `RawValue`, with a test on the bytes a request puts on the wire. Bound what a reading can write: `maxItems` on the arrays and `maxLength` on the strings make this runaway impossible, where a `max_tokens` scaled to the batch only stops it. A failed reading batch becomes unread and is reported, rather than failing the stage. And a failed run can continue from its recorded candidates, either because every run keeps its tape or because `--live-from <stage>` accepts the recorded output of the stages above it.

### 22. A number written two ways is refused, and dedupe regroups when four candidates change

Rerunning support on the same 1,745 candidates with schemas sent in written order (988b5b1) changed 4 of the 6 number-check refusals. The 4 refused only under the sorted grammar were wrong: three readings carried prose as the value (`once per process and only for graph_unavailable; …`), and the fourth refused `16,384 tokens` against a citation stating `16,384 context`. Of the 4 refused only by the fixed run, 2 rules add a number their citation lacks (`one comment per call`; `p90` in `p90 2316ms`), and 2 state the citation's number in other words: `48GB VRAM` against `48GB`, `approximately 11-20` against `~11-20`.

Dedupe reads every candidate in one call (`draft.rs`, `dedupe`). The two runs' candidate sets differed by those four rules, and 7 of the first run's 32 duplicate groups survived into the second run's 47. Each run folds real duplicates the other missed (three wordings of one rule about joining shell snippets; two statements of how `provider.name` is derived), and at least one new fold merges claims about two different components (`pi-agent-core`, `pi-coding-agent`). A grouping that moves this much under a four-item change is not a stable reading of the set.

**fix**: compare a quantity with its qualifier and restated unit set aside (`approximately` and `~`; `GB VRAM` and `GB`). Dedupe in windows small enough that a change in one leaves the other windows' groups alone, and report how many groups a continued run shares with the run it continued.

### 23. A proposal cannot carry its citation or its reason, so a delegated review has no path through canon

The operator reviewed 153 candidates of the charter run by hand (66 accepted, 1 rejected, 86 skipped) and delegated the notes run, 1,485 offerable candidates, to an agent to propose verdicts on. The agent has no act that holds a proposal worth ratifying. `canon add` writes an `Assert` with `source: None` (`cmds.rs:351-357`) and takes no flag but `--scope` (`cli.rs:38`), where the review path writes the same act with `source: Some(c.source)` (`draft.rs:2674-2678`). A proposal an agent files for a drafted candidate drops the citation that is the rule's history, and has nowhere to put the one-line reason the operator would ratify it on. `draft --resume` keeps the source, and it is a person's act by design. `--accept-all` is refused on purpose (`draft.rs:2016`), so what is left is a script that answers `--resume`'s prompts by candidate text from a verdict file: the bulk acceptance canon refuses, rebuilt outside it. Line numbers at 33965b4.

**fix**: let a proposal carry what the review would have written, for example `add "<text>" --source <cite> -m "<why>"`, or `draft --propose <verdicts.jsonl>` filing each candidate as a proposal from the acting agent with its source and reason. `approve` then ratifies, and `list` groups proposals by the reason they were filed under, so a person ratifies a group they have read rather than an import.

### 24. An edit keeps the model's kind, so a rule extracted as a question can only be recorded as a question

`review` takes the act from the candidate's kind and only the text from the answer (`draft.rs:2673-2691`): `[e]dit` on a `question` writes `ActKind::Question` with the edited text. The notes run phrased stated rules as questions. "What is the condition under which a data file authored FOR an instrument is landed?" cites a passage that reads "THE RULE: a data file authored FOR an instrument is not landed until that instrument has read it". Accepting that candidate records an answered question as open, editing it does the same, and rejecting it loses the rule unless someone retypes it with `add`, which drops the source (entry 23). In the notes run, 1 of the first 31 question candidates reviewed was an open question. Line numbers at 33856c6.

**fix**: let `[e]dit` change the kind, or check each question against its own passage and offer one the passage answers as the rule it states.

## From founding the Ersilia canon by adoption

*2026-10-03. Ersilia (`~/dev/ersilia/.canon`) was founded on the code profile, given the operator's standing over `arch`, `procedure` and `default`, and forked from commonwealth-ai@881e16 with `share | adopt --paste`; an agent filed 27 Ersilia proposals and 7 questions, and the operator ratified by script. `canon` on PATH is the working tree's debug build (`~/.local/bin/canon` links to `target/debug/canon`), at 158f623 with uncommitted changes to `draft.rs`; line numbers at 158f623.*

Entries 1, 3, 12 and 23 recurred and are not repeated. Entry 3: the agent set `CANON_ACTOR=agent:claude` by hand on every call. Entry 12: two fragments came down through adoption ("If it isn't, the friction is a bug…" and "It must be specific and actionable by someone who did not see the incident."). Entry 23: the operator's 27 approvals went into a generated script of `canon approve` lines.

### 25. Every verb refuses `--help`

`canon init --help` prints "`canon init` has no `--help` — it takes `--profile`", and `approve`, `list`, `retract` and `adopt` answer the same way (the verb table, `crates/canon-cli/src/cli.rs:74`). The refusal names a flag but not what the verb does or writes, so learning what `init` puts in a repository meant initialising a scratch directory first. **fix**: `<verb> --help` prints that verb's line from `help all` and its flags.

### 26. A fresh canon makes an agent's adds law

On a fresh code-profile canon with no grant, `CANON_ACTOR=agent:claude canon add "…" --scope procedure` printed `in force`: nobody held the scope, so it was open (`crates/canon-core/src/ratify.rs:557`). The only signal is the `list` warning. The first probe rule written while founding Ersilia's canon became law before any person had written anything; the founding had to grant the operator both scopes before a single proposal. This is not entry 1 (grants in the same second): here no grant exists at all. **fix**: on the code and house profiles, an agent's act in an open scope is a proposal; failing that, the add line says "in force only because nobody holds `<scope>`".

### 27. The root scope is `default` in one place and "this canon" in another

An agent's unscoped proposal reads "needs approval from one person who holds this canon" (`crates/canon-core/src/ratify.rs:317`), and `canon grant <actor> default` is how a person comes to hold it (`crates/canon-core/src/policy.rs:359`). Nothing on screen connects the two; `default` turned up only in `canon who default`'s hint. **fix**: one name, printed in both places, for example "holds the root scope (`default`)".

**Correction**: `default` is not a root scope, and `policy.rs:359` is the name of the shipped *policy*, not a scope. The canon has no scope of its own: an unscoped proposal is ratified by whoever holds the widest scopes anyone holds (`holders_at`, `ratify.rs`), which in Ersilia were `arch` and `procedure` from the founding grants, so the operator already held the canon and the later `grant … default` added a third top-level scope that changed nothing. What misled was `canon who default` answering "grant it" for a name nothing used.

### 28. Adoption is all or nothing, and retracting an inherited rule means mapping ids by hand

Ersilia inherits the Commonwealth canon's general rules and not its tooling, so 17 of the 66 had to go: `svrn` and `sovereign` commands, the work atlas, `DEFAULTS_LEDGER.md`, a near-duplicate and a fragment. `adopt` takes `--paste` or a URL and nothing else (`cli.rs:74`), and each adopted commitment gets a new id, with the upstream id only in `list --json`'s `from` field, so the retraction script resolved each upstream id through `jq` before it could retract. `diff --upstream` then reported `RETRACTED (17) · ADDED (27) · UNTOUCHED (49)` cleanly: the lineage record works, and reaching it took a script. **fix**: `adopt --except <upstream-id>…`, recorded as retractions with their reasons so `diff --upstream` reads the same, and let `retract` and `why` accept an upstream id that resolves through `from`.

### 29. An agent can propose a rule but not the retraction of one

An agent's `add` in a held scope becomes a proposal the holder approves. An agent's `retract` prints `NOT APPLIED … on the record as can-eae1b008d37e; somebody with standing has to do it` (`crates/canon-cli/src/cmds.rs:231`), and `canon approve can-eae1b008d37e` answers `no commitment matching`. The record keeps the agent's retraction and its reason, and the holder still has to retype both; the 17 retractions in entry 28 had to be written into the operator's own script for that reason. **fix**: a NOT APPLIED retraction becomes a proposal the holder can approve or object to, as an add does.

### 30. A fully ratified canon still warns that its rules were not authored by a person

After the operator approved all 27 proposals, `canon list` printed `warning: 31 adjudication(s) were not authored by a person:` followed by 31 ids on one line (`crates/canon-cli/src/cmds.rs:519`, `crates/canon-core/src/fold.rs:207`). They are the 27 proposals, each approved by a person, and the founding grants and ratification rules the agent wrote on the operator's instruction while the scopes were open. Reporting authorship is right; on a canon with nothing left to ratify the line reads as a defect, and 31 ids inline cannot be read. **fix**: do not count a proposal a person approved, and report the rest grouped by kind (grants, ratification rules, questions) with a count, the ids behind `--json`.

### 31. Nothing renders the canon for a reader without the CLI

Ersilia's build loop runs agents in harnesses without canon, so the rules in force have to be a file in the repository. `share` is an adoption snapshot, not a reader's document, and `list --json` (`crates/canon-cli/src/cmds.rs:448`) carries neither scope nor rank on a commitment: scopes are a separate list of pairs, and rank exists only as `op: rank` acts in `acts.jsonl`. Ersilia's `scripts/principles.sh` therefore folds the log itself with `jq` to render `PRINCIPLES.md`, grouped by scope with principles first and the inherited rules last, plus a `--check` for a stale render. Every canon read by agents outside the CLI will write that script again. **fix**: carry `scope` and `rank` on each commitment in `list --json`, and add a render (`canon render --markdown`, or `share --markdown`) with a `--check` that fails when a committed render is stale.
