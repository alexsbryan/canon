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
