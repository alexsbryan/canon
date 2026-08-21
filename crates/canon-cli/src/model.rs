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

#[derive(Debug)]
pub struct Client {
    agent: ureq::Agent,
    endpoint: String,
    model: String,
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
            endpoint,
            // Most local servers serve one model and ignore this field, but
            // the OpenAI schema requires it, so something must be sent.
            model: cfg.model.clone().unwrap_or_else(|| "local".to_string()),
        })
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
        let refusal = match self.post(&rung1) {
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
        decode(&self.post(&rung2)?)
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

    /// POST once; return the assistant's content string.
    fn post(&self, body: &Value) -> Result<String, ModelError> {
        let url = format!("{}/chat/completions", self.endpoint);
        let response = match self
            .agent
            .post(&url)
            .set("content-type", "application/json")
            // `send_string` rather than `send_json`: the reply is parsed by
            // hand anyway, so ureq's `json` feature would buy nothing.
            .send_string(&body.to_string())
        {
            Ok(r) => r,
            Err(ureq::Error::Status(status, r)) => {
                let detail = r.into_string().unwrap_or_else(|e| e.to_string());
                return Err(ModelError::Refused {
                    status,
                    detail: cap(detail.trim()),
                });
            }
            Err(ureq::Error::Transport(t)) => {
                return Err(ModelError::Transport {
                    url,
                    detail: t.to_string(),
                })
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

/// Decode the assistant's content into JSON.
///
/// Tolerates a fenced code block, because `json_object` mode on several
/// servers wraps the object in one. That is decoding a known transport
/// wrapper around the same JSON — not parsing prose, and not a third rung.
fn decode(content: &str) -> Result<Value, ModelError> {
    let mut s = content.trim();
    if let Some(rest) = s.strip_prefix("```") {
        let rest = rest.strip_prefix("json").unwrap_or(rest);
        s = rest.trim().trim_end_matches("```").trim();
    }
    serde_json::from_str(s).map_err(|e| ModelError::Malformed {
        detail: e.to_string(),
        raw: cap(content),
    })
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
