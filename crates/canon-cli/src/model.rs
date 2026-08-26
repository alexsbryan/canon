// SPDX-License-Identifier: AGPL-3.0-or-later
//! The endpoint client — the only place `canon` talks to a model.
//!
//! One transport (`ureq`, blocking, no async runtime) and one shape of call:
//! an OpenAI-compatible `/chat/completions` that must come back as JSON
//! matching a schema the caller supplies.
//!
//! **The structured-output ladder, and why it has exactly two rungs.**
//! Endpoints differ in what they accept. `json_schema` is the rung that makes
//! the reply parseable by construction; `json_object` is the rung almost
//! everything supports, at the cost of stating the schema in the prompt. The
//! ladder climbs down ONCE, and it says so on stderr when it does — a
//! substitution the user is not told about is the failure mode §18.3 exists
//! to prevent. There is no third rung: **prose parsing is not a fallback.**
//! An endpoint that will not answer in JSON produces exit 3 — "cannot judge",
//! which is a real verdict — rather than a plausible answer scraped out of a
//! paragraph.

use std::path::Path;
use std::time::Duration;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::config::Config;

/// How long to wait for a completion. Local models on modest hardware take
/// tens of seconds for a tensions call over thirty commitments; the default
/// exists so a slow answer is not mistaken for a hung one.
const READ_TIMEOUT_SECS: u64 = 600;
const CONNECT_TIMEOUT_SECS: u64 = 10;

/// Endpoint error bodies can be enormous (some servers echo the request).
/// Keep enough to name the refusal, not enough to bury it.
const DETAIL_CAP: usize = 600;

#[derive(Debug)]
pub enum ModelError {
    /// No endpoint configured. Reported, never guessed at.
    NoEndpoint,
    /// The config file itself could not be read.
    Config(String),
    /// A non-local endpoint where the caller requires a local one.
    Remote {
        host: String,
        endpoint: String,
    },
    Transport {
        url: String,
        detail: String,
    },
    /// The endpoint answered, and the answer was a refusal.
    Refused {
        status: u16,
        detail: String,
    },
    /// The endpoint answered, and the answer was not the shape we asked for.
    Malformed {
        detail: String,
        raw: String,
    },
}

impl std::fmt::Display for ModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEndpoint => write!(
                f,
                "no endpoint configured — `canon config set endpoint http://localhost:8080/v1`\n  \
                 any OpenAI-compatible server will do (llama.cpp, vllm, a sovereign daemon at :9741/v1)"
            ),
            Self::Config(e) => write!(f, "{e}"),
            Self::Remote { host, endpoint } => write!(
                f,
                "`{endpoint}` is not local (host `{host}`), and this command sends your own text.\n  \
                 pass --allow-remote to send it anyway, or point the endpoint at a local model"
            ),
            Self::Transport { url, detail } => {
                write!(f, "could not reach {url}: {detail}")
            }
            Self::Refused { status, detail } => {
                write!(f, "the endpoint refused (HTTP {status}): {detail}")
            }
            Self::Malformed { detail, raw } => write!(
                f,
                "the endpoint answered, but not in the shape asked for: {detail}\n  \
                 raw reply: {raw}"
            ),
        }
    }
}

impl std::error::Error for ModelError {}

impl ModelError {
    /// Exit code contract: 3 is "cannot judge", a first-class verdict.
    /// A remote endpoint held back by policy is exit 2 — the tool *could*
    /// judge; the user has to say they want the text to leave the machine.
    pub fn exit_code(&self) -> i32 {
        match self {
            Self::Remote { .. } | Self::Config(_) => 2,
            _ => 3,
        }
    }
}

/// The one way a command acquires a model.
///
/// Every model-using verb goes through here, so the locality rule is decided
/// once rather than re-implemented per verb (§10.6). `canon` handles text its
/// user did not choose to publish — a journal, a house chat, a private
/// repository — so a non-local endpoint is refused unless the user says
/// otherwise on the command line.
pub fn client_for(dir: &Path, allow_remote: bool) -> Result<Client, ModelError> {
    let cfg = Config::load(dir).map_err(ModelError::Config)?;
    let client = Client::new(&cfg)?;
    if !allow_remote {
        client.require_local()?;
    }
    Ok(client)
}

/// The client the EXTRACT leg should use: `extract_model` when set, otherwise
/// the one every leg shares.
///
/// Takes an already-acquired client so the locality rule is decided once, in
/// `client_for`, and cannot be bypassed by asking for a leg — a per-leg slot
/// is a routing choice, never a privacy one.
pub fn extract_client(dir: &Path, base: &Client) -> Result<Client, ModelError> {
    let cfg = Config::load(dir).map_err(ModelError::Config)?;
    Ok(match cfg.extract_model.as_deref().map(str::trim) {
        Some(m) if !m.is_empty() => base.with_model(m),
        _ => base.with_model(base.model()),
    })
}

/// Print a model failure and return the exit code it maps to.
///
/// Exit 3 reads as "cannot judge", which is a verdict; the other codes are
/// ordinary errors. Saying "cannot judge" for a bad config file would be a
/// small lie about which of the two happened.
pub fn report(e: ModelError) -> i32 {
    let code = e.exit_code();
    if code == 3 {
        eprintln!("cannot judge: {e}");
    } else {
        eprintln!("error: {e}");
    }
    code
}

/// One exchange with the endpoint: the path it went to, and the raw reply.
///
/// The REPLY is what is worth keeping. Everything canon does after a call —
/// cutting a citation from the passage the model pointed at, refusing a
/// silence with no reason, refusing a rule that states a number its citation
/// does not, folding duplicates, thresholding a convergence — is pure code
/// over this string. Recording it makes every one of those testable against
/// real model output at zero cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TapeEntry {
    /// Endpoint path, e.g. `chat/completions` or `embeddings`. Checked on
    /// replay: a tape played against a build that calls a different endpoint
    /// in a different order is not evidence about that build.
    pub path: String,
    /// Which stage asked — the schema name the call already carries
    /// (`commitments`, `quantities`, `groups`, `tensions`, `bearings`,
    /// `changes`), or `embeddings`. No new vocabulary: every call site was
    /// already naming its stage, this just keeps the name.
    ///
    /// Defaulted, so a tape recorded before stages were labelled still plays
    /// end to end — it simply cannot be cut at a stage.
    #[serde(default)]
    pub stage: String,
    pub raw: String,
    /// The status this reply came back with. 200 is an answer.
    ///
    /// **A tape that records only successes cannot be replayed faithfully.**
    /// A failed call used to leave no entry at all, so every later call
    /// popped the reply meant for its predecessor. The founding run of
    /// 2026-08-26 lost chunk 1 to a backend error; replaying its tape read
    /// all 103 remaining passages against the wrong reply and produced 256
    /// candidates instead of 342, with 88 dropped for citations pointing at
    /// somebody else's passage. Nothing said so until a stage label happened
    /// to mismatch on the very last call — and had the gap been in the LAST
    /// chunk, nothing would have said so at all.
    #[serde(default = "ok_status", skip_serializing_if = "is_ok_status")]
    pub status: u16,
}

fn ok_status() -> u16 {
    200
}

fn is_ok_status(s: &u16) -> bool {
    *s == 200
}

/// A run's exchanges, being written or being played back.
///
/// **Why this is at the transport seam and not per stage.** Every stage
/// reaches the endpoint through [`Client::post_json`], so one seam records
/// all of them and no stage has to know it is being taped (§10.6). It is also
/// the reason a replay costs nothing: the expensive half of a run is the
/// model, and a tape has already paid for it.
#[derive(Debug)]
pub enum Tape {
    Record(std::cell::RefCell<Vec<TapeEntry>>),
    /// Playing back, optionally only up to a stage.
    ///
    /// `live_from` is what makes an arm on a LATE stage cheap. The stages
    /// above it come off the tape; from that stage on the calls are real. A
    /// comparison arm is 10 of ~36 calls, so it costs about five minutes
    /// instead of twenty — and unlike a full replay it CAN judge a changed
    /// prompt, because the changed stage is actually run.
    Play {
        entries: std::cell::RefCell<std::collections::VecDeque<TapeEntry>>,
        live_from: Option<String>,
        live: std::cell::Cell<bool>,
    },
}

impl Tape {
    pub fn record() -> Self {
        Tape::Record(std::cell::RefCell::new(Vec::new()))
    }

    pub fn play(entries: Vec<TapeEntry>, live_from: Option<String>) -> Self {
        Tape::Play {
            entries: std::cell::RefCell::new(entries.into()),
            live_from,
            live: std::cell::Cell::new(false),
        }
    }

    /// What was recorded, in order.
    pub fn entries(&self) -> Vec<TapeEntry> {
        match self {
            Tape::Record(v) => v.borrow().clone(),
            Tape::Play { entries, .. } => entries.borrow().iter().cloned().collect(),
        }
    }
}

#[derive(Debug)]
pub struct Client {
    agent: ureq::Agent,
    endpoint: String,
    model: String,
    /// Shared, not owned: `with_model` hands a leg its own client, and a leg
    /// with its own tape would drop its calls from the run's recording — the
    /// extract leg, which is 24 of ~36 calls, first.
    tape: Option<std::rc::Rc<Tape>>,
    /// Consecutive calls that have waited out the whole backpressure budget.
    ///
    /// Shared for the same reason the tape is: this counts how the HOST is
    /// behaving, and a leg with its own counter would never reach the
    /// give-up threshold no matter how long the endpoint stayed down.
    busy_streak: std::rc::Rc<std::cell::Cell<usize>>,
}

// ── backpressure ────────────────────────────────────────────

/// What one call will wait out before giving up, and the ceiling on a single
/// sleep between attempts.
///
/// Sized from the failure they exist for. On 2026-08-26 the daemon shed load
/// for about eight minutes while a founding sweep was in its comparison
/// stage. Nothing waited — every refusal returned instantly — so 588 of 690
/// passes burned in that window and the run reported 15% coverage.
const BACKPRESSURE_BUDGET: Duration = Duration::from_secs(300);
const BACKPRESSURE_CAP: Duration = Duration::from_secs(30);

/// Consecutive calls allowed to exhaust the budget before the run gives up.
///
/// Retrying without this trades one failure for a worse one: 690 passes each
/// waiting out five minutes is two and a half days of looking busy. Three
/// exhausted calls is a host that is not returning on the timescale this run
/// cares about, so the stage abandons — which writes the artifact and says
/// why, instead of hanging.
const BACKPRESSURE_GIVE_UP_AFTER: usize = 3;

/// Is this refusal the host asking us to come back, or declining outright?
///
/// The distinction is whether waiting can possibly help. A 429 or a 503 says
/// "over capacity right now"; a 400 or a 404 will say the same thing however
/// long we wait, and retrying one burns a call to learn nothing (§18.3).
pub fn is_backpressure(status: u16, detail: &str) -> bool {
    // 429 is the protocol's own unambiguous "come back later".
    if status == 429 {
        return true;
    }
    // 503 is NOT unambiguous, and treating it as if it were broke two
    // existing tests the first time this was written. This daemon answers
    // 503 both for "the queue is full and your request never ran" and for
    // "your request ran and blew its 300s inference deadline". Retrying the
    // second spends another 300 seconds arriving in the same place, on a
    // request already known to be too expensive. Only the body separates
    // them, so the body is what decides.
    //
    // Never a 200 either: there the "refusal" is the MODEL declining, and no
    // amount of waiting changes its mind.
    status != 200 && (detail.contains("host busy") || detail.contains("queue position"))
}

/// How long to wait before retry `attempt` (1-based), given what this call
/// has already waited. `None` means the budget is spent.
///
/// Doubling from two seconds and capped, so a spike that clears in ten
/// seconds costs ten seconds rather than a fixed penalty, and a long one
/// settles into a steady poll instead of sleeping for minutes at a stretch.
pub fn backoff(attempt: usize, waited: Duration) -> Option<Duration> {
    if attempt == 0 || waited >= BACKPRESSURE_BUDGET {
        return None;
    }
    let secs = 2u64
        .saturating_pow(u32::try_from(attempt.min(16)).unwrap_or(16))
        .min(BACKPRESSURE_CAP.as_secs());
    let d = Duration::from_secs(secs).min(BACKPRESSURE_BUDGET - waited);
    if d.is_zero() {
        None
    } else {
        Some(d)
    }
}

impl Client {
    /// Build a client from config. `Err(NoEndpoint)` when none is set —
    /// absence is reported rather than defaulted to somebody's localhost.
    pub fn new(cfg: &Config) -> Result<Self, ModelError> {
        let endpoint = cfg
            .endpoint
            .as_deref()
            .map(|e| e.trim_end_matches('/').to_string())
            .filter(|e| !e.is_empty())
            .ok_or(ModelError::NoEndpoint)?;
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(CONNECT_TIMEOUT_SECS))
            .timeout_read(Duration::from_secs(READ_TIMEOUT_SECS))
            .build();
        Ok(Self {
            agent,
            busy_streak: Default::default(),
            endpoint,
            // Most local servers serve one model and ignore this field, but
            // the OpenAI schema requires it, so something must be sent.
            model: cfg.model.clone().unwrap_or_else(|| "local".to_string()),
            // No default. An embedding model is a second model the user may
            // not have, and guessing a name produces a 404 thirty seconds
            // into a run instead of a clear "ordering unavailable".
            tape: None,
        })
    }

    /// The endpoint as configured. Recorded in a draft run, so a published
    /// number always says which server produced it.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// The model name sent with each request. Recorded alongside the
    /// endpoint: a quality number that does not say which model produced it
    /// cannot be compared with anything, including itself next month.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The same client pointed at a different slot.
    ///
    /// The endpoint, the agent and the locality decision were already made and
    /// none of them changes with the model name — only which slot serves the
    /// call does. `client_for` remains the one place a client is ACQUIRED
    /// (§10.6); this narrows an acquired one to a leg.
    pub fn with_model(&self, model: &str) -> Self {
        Self {
            agent: self.agent.clone(),
            endpoint: self.endpoint.clone(),
            model: model.to_string(),
            // One tape per RUN, shared by every leg's client.
            tape: self.tape.clone(),
            busy_streak: self.busy_streak.clone(),
        }
    }

    /// The host, parsed without a URL crate — this is the only URL question
    /// the tool asks, and it does not justify a dependency.
    pub fn host(&self) -> String {
        let rest = self
            .endpoint
            .split_once("://")
            .map(|(_, r)| r)
            .unwrap_or(&self.endpoint);
        let authority = rest.split(['/', '?', '#']).next().unwrap_or(rest);
        let authority = authority
            .rsplit_once('@')
            .map(|(_, h)| h)
            .unwrap_or(authority);
        // `[::1]:8080` — the brackets are what make a v6 literal parseable.
        if let Some(v6) = authority.strip_prefix('[') {
            return v6.split(']').next().unwrap_or(v6).to_string();
        }
        authority
            .rsplit_once(':')
            .map(|(h, _)| h)
            .unwrap_or(authority)
            .to_string()
    }

    /// Is this endpoint on this machine?
    ///
    /// Conservative on purpose: anything not obviously loopback counts as
    /// remote, so a misconfiguration errs toward keeping a journal at home.
    pub fn is_local(&self) -> bool {
        let host = self.host();
        host == "localhost"
            || host.ends_with(".localhost")
            || host == "::1"
            || host == "0.0.0.0"
            || host
                .split_once('.')
                .is_some_and(|(first, _)| first == "127" && host.split('.').count() == 4)
    }

    /// Refuse unless the endpoint is on this machine. Callers that handle
    /// someone's own text (`draft`) gate on this; `--allow-remote` skips it.
    pub fn require_local(&self) -> Result<(), ModelError> {
        if self.is_local() {
            return Ok(());
        }
        Err(ModelError::Remote {
            host: self.host(),
            endpoint: self.endpoint.clone(),
        })
    }

    /// What to print when naming which endpoint did the work. Every command
    /// that calls a model says this, because "your journal never left your
    /// machine" is only worth anything if the tool tells you.
    pub fn describe(&self) -> String {
        format!(
            "{} ({})",
            self.endpoint,
            if self.is_local() { "local" } else { "REMOTE" }
        )
    }

    /// One call, answered as `T`, or a named failure.
    pub fn complete_json<T: DeserializeOwned>(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<T, ModelError> {
        let value = self.complete_value(system, user, schema_name, schema)?;
        serde_json::from_value(value.clone()).map_err(|e| ModelError::Malformed {
            detail: e.to_string(),
            raw: cap(&value.to_string()),
        })
    }

    fn complete_value(
        &self,
        system: &str,
        user: &str,
        schema_name: &str,
        schema: &Value,
    ) -> Result<Value, ModelError> {
        // Rung 1 — the endpoint enforces the shape.
        let rung1 = self.request(
            system,
            user,
            json!({
                "type": "json_schema",
                "json_schema": { "name": schema_name, "strict": true, "schema": schema },
            }),
        );
        let refusal = match self.post(schema_name, &rung1) {
            Ok(text) => return decode(&text),
            Err(e @ ModelError::Refused { .. }) if is_schema_refusal(&e) => e,
            Err(e) => return Err(e),
        };

        // Rung 2 — we enforce the shape, and we SAY we are doing it.
        eprintln!("note: this endpoint refused structured output by schema — {refusal}");
        eprintln!(
            "      retrying once with `json_object`, schema stated in the prompt instead. \
             Naming the substitution rather than making it silently."
        );
        let stated = format!(
            "{user}\n\nReply with one JSON object and nothing else. It must match this schema \
             exactly:\n{}",
            serde_json::to_string(schema).unwrap_or_default()
        );
        let rung2 = self.request(system, &stated, json!({ "type": "json_object" }));
        // No rung 3. A refusal here is a refusal, and prose is never parsed.
        decode(&self.post(schema_name, &rung2)?)
    }

    fn request(&self, system: &str, user: &str, response_format: Value) -> Value {
        json!({
            "model": self.model,
            // Adjudication wants the same answer twice, so temperature is
            // pinned rather than left to the server's default.
            "temperature": 0.0,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user },
            ],
            "response_format": response_format,
        })
    }
    /// POST once to a path under the endpoint; return `(parsed, raw)`.
    ///
    /// The transport half, shared by completions and embeddings so there is
    /// one place that knows how this server reports a refusal (§10.6).
    /// Attach a fresh recording tape to this client and every leg made from it.
    pub fn recording(mut self) -> Self {
        self.tape = Some(std::rc::Rc::new(Tape::record()));
        self
    }

    /// A client that answers from a recording and never reaches the network.
    ///
    /// No endpoint is required and no locality check applies, because no call
    /// leaves the process. The endpoint string is carried only so the artifact
    /// a replay writes still says which server produced the evidence.
    pub fn replaying(endpoint: &str, model: &str, entries: Vec<TapeEntry>) -> Self {
        Self {
            agent: ureq::AgentBuilder::new().build(),
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            tape: Some(std::rc::Rc::new(Tape::play(entries, None))),
            busy_streak: Default::default(),
        }
    }

    /// Play a tape on a LIVE client, going real from `live_from` onward.
    ///
    /// The client keeps its configured endpoint because the calls from the cut
    /// stage on are genuine. This is the loop for iterating on a late stage:
    /// the comparison stage is 10 of ~36 calls, so an arm on it costs the 10.
    pub fn playing(mut self, entries: Vec<TapeEntry>, live_from: Option<String>) -> Self {
        self.tape = Some(std::rc::Rc::new(Tape::play(entries, live_from)));
        self
    }

    /// A leg's view of this client: same endpoint, same model, SAME tape.
    ///
    /// `with_model` is for pointing a leg at a different slot; this is for a
    /// leg that shares everything. Both share the tape, because a run has one.
    pub fn for_leg(&self) -> Self {
        self.with_model(&self.model)
    }

    /// What this run has recorded so far, in order. Empty when not recording.
    pub fn tape(&self) -> Vec<TapeEntry> {
        self.tape.as_ref().map(|t| t.entries()).unwrap_or_default()
    }

    fn post_json(
        &self,
        path: &str,
        stage: &str,
        body: &Value,
    ) -> Result<(Value, String), ModelError> {
        // A tape being PLAYED answers here, before any transport exists.
        //
        // The path is checked rather than assumed. A build that calls a
        // different endpoint, or the same ones in a different order, is not
        // the build this tape recorded, and a number scored from it would be
        // about neither — so it refuses instead of substituting (§18.3).
        if let Some(Tape::Play {
            entries,
            live_from,
            live,
        }) = self.tape.as_deref()
        {
            // Once the cut stage is reached the tape is done and every call
            // from here is real — including the rest of THIS stage's calls.
            if !live.get() && live_from.as_deref() == Some(stage) {
                live.set(true);
            }
            if !live.get() {
                let entry =
                    entries
                        .borrow_mut()
                        .pop_front()
                        .ok_or_else(|| ModelError::Malformed {
                            detail: format!(
                                "the tape is exhausted: this build asked for a `{path}` call \
                                 the recording does not have. A change to the CALL SEQUENCE \
                                 cannot be judged by replay — re-run it live, or cut the tape \
                                 above the stage you changed with `--live-from`."
                            ),
                            raw: String::new(),
                        })?;
                if entry.path != path {
                    return Err(ModelError::Malformed {
                        detail: format!(
                            "tape out of step: this build asked for `{path}` where the \
                             recording has `{}`. Replay judges pure code over recorded \
                             model output; the calls themselves must match.",
                            entry.path
                        ),
                        raw: String::new(),
                    });
                }
                // A stage label only checks when the recording HAS one: tapes
                // written before stages were labelled carry `""`, and refusing
                // those would retire evidence that is still perfectly good for
                // a full replay.
                if !entry.stage.is_empty() && entry.stage != stage {
                    return Err(ModelError::Malformed {
                        detail: format!(
                            "tape out of step: the `{stage}` stage asked, but the recording \
                             has a `{}` call here.",
                            entry.stage
                        ),
                        raw: String::new(),
                    });
                }
                // A recorded refusal is replayed AS a refusal. Anything else
                // would make the replay a different run from the one taped.
                if entry.status != 200 {
                    return Err(ModelError::Refused {
                        status: entry.status,
                        detail: cap(&entry.raw),
                    });
                }
                let parsed: Value =
                    serde_json::from_str(&entry.raw).map_err(|e| ModelError::Malformed {
                        detail: format!("recorded reply is not JSON: {e}"),
                        raw: cap(&entry.raw),
                    })?;
                return Ok((parsed, entry.raw));
            }
        }
        let url = format!("{}/{path}", self.endpoint);
        let mut waited = Duration::ZERO;
        let mut attempt = 0usize;
        let response = loop {
            match self
                .agent
                .post(&url)
                .set("content-type", "application/json")
                // `send_string` rather than `send_json`: the reply is parsed by
                // hand anyway, so ureq's `json` feature would buy nothing.
                .send_string(&body.to_string())
            {
                Ok(r) => {
                    self.busy_streak.set(0);
                    break r;
                }
                Err(ureq::Error::Status(status, r)) => {
                    let detail = cap(r.into_string().unwrap_or_else(|e| e.to_string()).trim());
                    if !is_backpressure(status, &detail) {
                        self.tape_entry(path, stage, &detail, status);
                        return Err(ModelError::Refused { status, detail });
                    }
                    attempt += 1;
                    let Some(d) = backoff(attempt, waited) else {
                        let streak = self.busy_streak.get() + 1;
                        self.busy_streak.set(streak);
                        self.tape_entry(path, stage, &detail, status);
                        if streak >= BACKPRESSURE_GIVE_UP_AFTER {
                            return Err(ModelError::Refused {
                                status,
                                detail: format!(
                                    "the host has been busy for {streak} calls running, each \
                                     waiting out {}s. Giving up rather than turning an outage \
                                     into a hang. Last reply: {detail}",
                                    BACKPRESSURE_BUDGET.as_secs()
                                ),
                            });
                        }
                        return Err(ModelError::Refused { status, detail });
                    };
                    // Loud. A run that silently takes an hour longer than it
                    // should is a run nobody can account for afterwards.
                    eprintln!(
                        "\n  host busy (HTTP {status}) — waiting {}s, then retry {attempt}",
                        d.as_secs()
                    );
                    std::thread::sleep(d);
                    waited += d;
                }
                Err(ureq::Error::Transport(t)) => {
                    return Err(ModelError::Transport {
                        url: url.clone(),
                        detail: t.to_string(),
                    })
                }
            }
        };
        let raw = response.into_string().map_err(|e| ModelError::Transport {
            url: url.clone(),
            detail: e.to_string(),
        })?;
        let parsed: Value = serde_json::from_str(&raw).map_err(|e| ModelError::Malformed {
            detail: format!("response body is not JSON: {e}"),
            raw: cap(&raw),
        })?;
        // Recorded AFTER the transport succeeded and BEFORE anything
        // interprets it: what goes on the tape is what the server said, not
        // what this build made of it.
        self.tape_entry(path, stage, &raw, 200);
        Ok((parsed, raw))
    }

    /// Put one exchange on the tape, answer or refusal alike.
    ///
    /// Every call that reached the endpoint gets an entry, because the tape's
    /// job is to let a replay make the SAME sequence of calls. Skipping the
    /// failures is what let a single lost chunk shift every later reply by
    /// one — see [`TapeEntry::status`].
    fn tape_entry(&self, path: &str, stage: &str, raw: &str, status: u16) {
        if let Some(Tape::Record(v)) = self.tape.as_deref() {
            v.borrow_mut().push(TapeEntry {
                path: path.to_string(),
                stage: stage.to_string(),
                raw: raw.to_string(),
                status,
            });
        }
    }

    /// POST once; return the assistant's content string.
    fn post(&self, stage: &str, body: &Value) -> Result<String, ModelError> {
        let (parsed, raw) = self.post_json("chat/completions", stage, body)?;
        let message = parsed
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"));
        // A model that declines is a refusal, not a malformed reply — the
        // distinction is what lets the caller print WHY it cannot judge.
        if let Some(why) = message
            .and_then(|m| m.get("refusal"))
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            return Err(ModelError::Refused {
                status: 200,
                detail: format!("the model declined: {}", cap(why)),
            });
        }
        message
            .and_then(|m| m.get("content"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| ModelError::Malformed {
                detail: "no `choices[0].message.content` in the response".into(),
                raw: cap(&raw),
            })
    }
}

/// Was this 4xx specifically about the structured-output request?
///
/// Narrow on purpose. A 400 for a bad model name must NOT climb down the
/// ladder — retrying would burn a second call and report the wrong cause.
fn is_schema_refusal(e: &ModelError) -> bool {
    let ModelError::Refused { status, detail } = e else {
        return false;
    };
    if !(400..500).contains(status) {
        return false;
    }
    let d = detail.to_ascii_lowercase();
    d.contains("json_schema") || d.contains("response_format") || d.contains("schema")
}

/// A position a model answered with, before anything trusts it.
///
/// Every one of these indexes something the model was SHOWN — a sentence
/// marker, a rule's number in the list it was given, a commitment's place in
/// the offered set. Each reader already refuses one that is out of range,
/// because a position naming something that was not offered is not an answer.
///
/// **What none of them survived was a position that would not deserialize.**
/// A model with no way to say "none of them" reaches for a sentinel: a
/// Qwen3-family 4B answers `same_as: -1` for a rule that duplicates nothing,
/// and a `usize` field turns that into an error that kills the whole call —
/// twenty-six candidates thrown away because one of them was unique. That is
/// the same failure §18.3 names, arriving through the type system: a partial
/// answer reported as no answer.
///
/// So the wire type is signed and [`Pos::get`] is the only way to an index.
/// It answers `None` for anything that is not a position, which lands on the
/// refusal each reader already has. Artifact structs canon writes itself
/// stay `usize` — nothing there came off a wire.
/// `Default` is zero, which is deliberately NOT a position: the markers a
/// model is shown are 1-based, so a field the answer omitted is refused by
/// the same range check that refuses one it got wrong.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
pub struct Pos(i64);

impl Pos {
    /// The position, or `None` when the answer was not one.
    pub fn get(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

impl std::fmt::Display for Pos {
    /// Prints what the model actually said, so a warning about a position
    /// out of range can name the sentinel rather than a substitute for it.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The reasoning channels a server may leave sitting in the content.
///
/// A closed set of transport wrappers, so it is a list rather than a pattern
/// (§2.1). Widening it to "anything in angle brackets" would start eating
/// document text.
const REASONING: &[&str] = &["think", "thinking", "reasoning"];

/// Decode the assistant's content into JSON.
///
/// Tolerates a fenced code block, because `json_object` mode on several
/// servers wraps the object in one. That is decoding a known transport
/// wrapper around the same JSON — not parsing prose, and not a third rung.
///
/// Tolerates a reasoning wrapper for the same reason. A reasoning model
/// behind an OpenAI-shaped endpoint may deliver its thinking in `content`
/// instead of a field of its own, and then the answer arrives correct and
/// unparseable. Measured on a Qwen3-family 4B on this endpoint: extraction
/// succeeded on all 24 passages of the Maple House fixture and the run died
/// at the reduce step, three times identically, because the reply was the
/// right JSON followed by a stray `</think>`.
fn decode(content: &str) -> Result<Value, ModelError> {
    let s = unfence(content.trim());
    let first = match serde_json::from_str(s) {
        Ok(v) => return Ok(v),
        Err(e) => e,
    };
    for reading in readings(s) {
        if let Ok(v) = serde_json::from_str(unfence(reading.trim())) {
            reasoning_channel_notice();
            return Ok(v);
        }
    }
    Err(ModelError::Malformed {
        detail: first.to_string(),
        raw: cap(content),
    })
}

fn unfence(s: &str) -> &str {
    let Some(rest) = s.strip_prefix("```") else {
        return s;
    };
    let rest = rest.strip_prefix("json").unwrap_or(rest);
    rest.trim().trim_end_matches("```").trim()
}

/// Every reading of `s` with a reasoning wrapper taken off, best first.
///
/// Two shapes turn up, both from one cause — the endpoint puts reasoning in
/// the content rather than in a field of its own:
///
/// ```text
/// <think>…</think>{"a":1}    a complete block, then the answer
/// {"a":1}</think>            the server consumed the OPEN tag and the model
///                            closed after answering
/// ```
///
/// Each reading is TRIED, never chosen: whichever one parses is the answer,
/// and a reply where none parses is malformed and says so with the raw text.
/// That is what keeps this from being a guess about where the answer is.
fn readings(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for tag in REASONING {
        let (open, close) = (format!("<{tag}>"), format!("</{tag}>"));
        let Some(i) = s.rfind(&close) else { continue };
        out.push(&s[i + close.len()..]);
        let before = &s[..i];
        out.push(match before.rfind(&open) {
            Some(j) => &before[j + open.len()..],
            None => before,
        });
    }
    out
}

/// Said once per process, not once per call.
///
/// It is a property of the endpoint, so twenty-four passages would print it
/// twenty-four times and teach the person to skip the warnings. Silence
/// would be worse: a reply being unwrapped is a decision, and a decision
/// nobody can see is not finished (§9.1).
fn reasoning_channel_notice() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        eprintln!(
            "note: this endpoint returns reasoning inside the reply. \
             canon is reading the answer out of it."
        );
    });
}

fn cap(s: &str) -> String {
    if s.chars().count() <= DETAIL_CAP {
        return s.to_string();
    }
    let head: String = s.chars().take(DETAIL_CAP).collect();
    format!("{head}… ({} chars total)", s.chars().count())
}

#[cfg(test)]
mod tests;
