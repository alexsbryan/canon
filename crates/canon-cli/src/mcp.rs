// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon mcp` — the agent surface, over stdio JSON-RPC 2.0.
//!
//! The problem this exists for: an agent about to make a consequential choice
//! does not know the house rules, and the prevailing fix — pasting them into
//! the prompt — saturates. This lets an agent ask instead.
//!
//! **Every tool here is a read.** There is no tool that writes an act: not
//! permission-gated, absent. Amending a canon requires the CLI, run by a
//! person. That is how *agents propose, humans dispose* becomes a property of
//! the build rather than a rule someone has to remember, and
//! [`tools::READ_ONLY_TOOLS`] is pinned by a test so adding a write tool
//! fails the suite.
//!
//! Hand-rolled over `serde_json` rather than pulling an MCP crate: the wire
//! surface is five methods, and a dependency here would be most of the
//! tool's dependency tree.

use std::collections::BTreeMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::store;

/// Protocol revision advertised when a client asks for one we do not know.
/// A client's own version is echoed when we recognise it, per MCP's
/// negotiation rule.
const PREFERRED_PROTOCOL: &str = "2025-06-18";
const SUPPORTED_PROTOCOLS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

pub mod tools {
    /// The complete tool surface. **Reads only.**
    ///
    /// Pinned by `the_surface_is_read_only`. A tool that mutates a canon
    /// belongs in the CLI, where a person runs it.
    pub const READ_ONLY_TOOLS: &[&str] = &["canon_list", "canon_why", "canon_open", "canon_check"];
}

// ── transport ───────────────────────────────────────────────

/// Serve on stdin/stdout until EOF. Returns a process exit code.
pub fn serve() -> i32 {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("canon mcp: reading stdin: {e}");
                return 1;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(req) => handle(&req),
            // Parse errors carry a null id: we could not read one.
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(resp) = response {
            if writeln!(stdout, "{resp}").is_err() || stdout.flush().is_err() {
                return 1;
            }
        }
    }
    0
}

/// Dispatch one message. `None` for a notification, which takes no reply.
fn handle(req: &Value) -> Option<Value> {
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let id = req.get("id").cloned();

    // A message with no `id` is a notification: act on it, answer nothing.
    let id = id?;

    Some(match method {
        "initialize" => {
            let asked = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str);
            let version = match asked {
                Some(v) if SUPPORTED_PROTOCOLS.contains(&v) => v,
                _ => PREFERRED_PROTOCOL,
            };
            ok(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": { "tools": {} },
                    "serverInfo": {
                        "name": "canon",
                        "version": env!("CARGO_PKG_VERSION"),
                    },
                }),
            )
        }
        "ping" => ok(id, json!({})),
        "tools/list" => ok(id, json!({ "tools": tool_descriptors() })),
        "tools/call" => {
            let name = req
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = req
                .get("params")
                .and_then(|p| p.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));
            ok(id, call(name, &args))
        }
        other => error(id, -32601, &format!("unknown method `{other}`")),
    })
}

fn ok(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// The tool-result envelope MCP clients expect.
fn content(text: impl Into<String>, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text.into() }],
        "isError": is_error,
    })
}

// ── tools ───────────────────────────────────────────────────

fn tool_descriptors() -> Vec<Value> {
    vec![
        json!({
            "name": "canon_list",
            "description": "Commitments currently in force. Small canons list in full; large \
                            canons show counts by scope first. Filter by scope or text, or page \
                            through results; use canon_why(id) to open a rule's history.",
            "inputSchema": { "type": "object", "properties": {
                "scope": { "type": "string", "description": "Room to consult, including its parent scopes and unscoped rules." },
                "query": { "type": "string", "description": "Case-insensitive substring of the rule text." },
                "limit": { "type": "integer", "minimum": 1, "maximum": 100 },
                "offset": { "type": "integer", "minimum": 0 }
            }, "additionalProperties": false },
        }),
        json!({
            "name": "canon_why",
            "description": "A commitment's history: what it replaced, when, the reason given, \
                            and any contradiction it is knowingly carried against. Use it to \
                            explain a rule rather than merely cite it.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Commitment id, or a unique prefix." }
                },
                "required": ["id"],
                "additionalProperties": false,
            },
        }),
        json!({
            "name": "canon_open",
            "description": "What this canon does not cover: questions recorded and not yet \
                            answered. Read it before assuming silence means permission.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
        }),
        json!({
            "name": "canon_check",
            "description": "How a proposal stands against the commitments in force, with the \
                            commitments it cites. Costs one model call, so reach for it on a \
                            consequential choice; for a small canon, canon_list is usually \
                            enough. Returns stakes rather than a verdict on a personal canon.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "proposal": {
                        "type": "string",
                        "description": "The proposal to judge, as you would say it out loud.",
                    }
                },
                "required": ["proposal"],
                "additionalProperties": false,
            },
        }),
    ]
}

fn call(name: &str, args: &Value) -> Value {
    // Name and arguments are checked BEFORE a canon is located. Neither needs
    // one, and an agent that mistyped should get a reply it can act on even
    // where there is no canon to read — "no tool named X" is more useful than
    // "no canon found" when the problem is the tool name.
    let arg = match name {
        "canon_list" | "canon_open" => None,
        "canon_why" => match args.get("id").and_then(Value::as_str) {
            Some(v) => Some(v),
            None => return content("canon_why requires an `id` argument.", true),
        },
        "canon_check" => match args.get("proposal").and_then(Value::as_str) {
            Some(v) if !v.trim().is_empty() => Some(v),
            _ => return content("canon_check requires a `proposal` argument.", true),
        },
        // Unknown tool is NOT a hard error: an agent that mistypes should be
        // able to recover from the reply rather than treat the surface as
        // broken.
        other => {
            return content(
                format!(
                    "no tool named `{other}`. Available: {}.",
                    tools::READ_ONLY_TOOLS.join(", ")
                ),
                false,
            )
        }
    };
    let dir = match locate() {
        Ok(d) => d,
        Err(e) => return content(e, true),
    };
    let rendered = match name {
        "canon_list" => render_list(&dir, args),
        "canon_open" => render_open(&dir),
        "canon_why" => render_why(&dir, arg.unwrap_or_default()),
        "canon_check" => render_check(&dir, arg.unwrap_or_default()),
        _ => unreachable!("validated above"),
    };
    match rendered {
        Ok(s) => content(s, false),
        Err(e) => content(e, true),
    }
}

fn locate() -> Result<PathBuf, String> {
    if let Ok(d) = std::env::var("CANON_DIR") {
        return Ok(PathBuf::from(d));
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    store::locate(&cwd)
        .ok_or_else(|| "no canon found. Run `canon init` first, or set CANON_DIR.".to_string())
}

fn render_list(dir: &Path, args: &Value) -> Result<String, String> {
    let canon = store::read(dir)?.derive();
    list_of(&canon, args)
}

fn list_of(canon: &canon_core::Canon, args: &Value) -> Result<String, String> {
    let obj = args
        .as_object()
        .ok_or("canon_list arguments must be an object")?;
    if let Some(key) = obj
        .keys()
        .find(|k| !["scope", "query", "limit", "offset"].contains(&k.as_str()))
    {
        return Err(format!("canon_list does not take `{key}`"));
    }
    let scope = match obj.get("scope") {
        None => None,
        Some(Value::String(raw)) => Some(
            canon_core::Scope::new(raw)
                .ok_or_else(|| format!("`{raw}` is not a scope: use a dotted path"))?,
        ),
        _ => return Err("canon_list scope must be a string".into()),
    };
    let query = match obj.get("query") {
        None => None,
        Some(Value::String(raw)) => Some(raw.to_lowercase()),
        _ => return Err("canon_list query must be a string".into()),
    };
    let limit = match obj.get("limit") {
        None => 50,
        Some(v) => match v.as_u64() {
            Some(n @ 1..=100) => n as usize,
            _ => return Err("canon_list limit must be between 1 and 100".into()),
        },
    };
    let offset = match obj.get("offset") {
        None => 0,
        Some(v) => v
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .ok_or("canon_list offset must be a nonnegative integer")?,
    };
    let live: Vec<_> = canon.active().collect();
    if live.is_empty() {
        return Ok("This canon has no commitments yet.".into());
    }
    if live.len() > 50 && obj.is_empty() {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for c in &live {
            let key = canon
                .scope_of(&c.id)
                .map(|s| s.as_str())
                .unwrap_or("(unscoped)");
            *counts.entry(key.to_string()).or_default() += 1;
        }
        let mut out = format!("{} in force. By scope:\n", live.len());
        let omitted = counts.len().saturating_sub(30);
        for (scope, n) in counts.into_iter().take(30) {
            out.push_str(&format!("  {scope}  {n}\n"));
        }
        if omitted > 0 {
            out.push_str(&format!("  …and {omitted} more scope(s)\n"));
        }
        out.push_str("Use canon_list(scope, query, limit, offset) to browse rules, and canon_why(id) for history.");
        return Ok(out);
    }
    let filtered: Vec<_> = live
        .into_iter()
        .filter(|c| {
            scope
                .as_ref()
                .is_none_or(|s| canon.scope_of(&c.id).is_none_or(|here| here.covers(s)))
                && query
                    .as_ref()
                    .is_none_or(|q| c.text.to_lowercase().contains(q))
        })
        .collect();
    if filtered.is_empty() {
        let mut filters = Vec::new();
        if let Some(s) = &scope {
            filters.push(format!("scope `{s}`"));
        }
        if let Some(q) = &query {
            filters.push(format!("query `{q}`"));
        }
        return Ok(format!("No commitments match {}.", filters.join(" and ")));
    }
    if offset >= filtered.len() {
        return Ok(format!(
            "Offset {offset} is past the {} matching commitment(s). Start again at offset 0.",
            filtered.len()
        ));
    }
    let mut out = String::new();
    for c in filtered.iter().skip(offset).take(limit) {
        out.push_str(&format!("{}  {}\n", c.id, c.text));
    }
    if obj.is_empty() {
        out.push_str(&format!("\n{} in force.", filtered.len()));
    } else {
        out.push_str(&format!(
            "\nShowing {}-{} of {} matching commitment(s).",
            offset.min(filtered.len()) + usize::from(offset < filtered.len()),
            offset.saturating_add(limit).min(filtered.len()),
            filtered.len()
        ));
        if offset.saturating_add(limit) < filtered.len() {
            out.push_str(&format!(" Next: offset {}.", offset + limit));
        }
    }

    let carried = canon.tolerated().count();
    if carried > 0 {
        out.push_str(&format!(
            "\n{carried} contradiction(s) are carried knowingly — `canon_why` on either side \
             gives the reason."
        ));
    }
    // A hole in the record is louder than a missing feature (§18.3).
    if !canon.dangling.is_empty() {
        out.push_str(&format!(
            "\nWARNING: {} act(s) reference a commitment absent from this log.",
            canon.dangling.len()
        ));
    }
    Ok(out)
}

fn render_open(dir: &Path) -> Result<String, String> {
    let canon = store::read(dir)?.derive();
    let open: Vec<_> = canon.open().collect();
    if open.is_empty() {
        return Ok(
            "No open questions. Note that this means nobody has RECORDED a gap — it is not a \
             claim that the canon covers everything."
                .into(),
        );
    }
    let mut out = String::new();
    for q in &open {
        out.push_str(&format!("{}  {}\n", q.id, q.text));
        if let Some(p) = &q.proposal {
            out.push_str(&format!("    surfaced by: \"{p}\"\n"));
        }
    }
    out.push_str(&format!("\n{} open question(s).", open.len()));
    Ok(out)
}

/// The one tool here that costs a model call.
///
/// Still a read: it writes no act. An agent that concludes something should
/// be recorded says so in chat, as a command a person can run.
fn render_check(dir: &Path, proposal: &str) -> Result<String, String> {
    let canon = store::read(dir)?.derive();
    if canon.active().next().is_none() {
        return Err(
            "This canon has no commitments yet, so there is nothing to check against.".into(),
        );
    }
    let profile = crate::profile::Profile::load(dir)?;
    let client = crate::model::client_for(dir, false).map_err(|e| e.to_string())?;
    let (standing, refused) =
        crate::check::assess(&client, &canon, proposal).map_err(|e| e.to_string())?;
    // The canon's own rule, so an agent is told what the community decided
    // rather than what shipped.
    let attrs = canon_core::Attributes::about(proposal).at(store::now());
    let decision = canon_core::Policy::decide(canon.policy_for(None), &standing, &attrs, &canon);
    let silence = canon.silence_about(proposal);
    let mut out = crate::check::render(profile, &canon, &standing, &decision, silence);
    // Refusals travel to the agent too. A shorter answer with no explanation
    // is indistinguishable from a canon that had less to say.
    if !refused.is_empty() {
        out.push_str(&format!(
            "\n({} uncitable position(s) were refused and are not shown.)\n",
            refused.len()
        ));
    }
    Ok(out)
}

fn render_why(dir: &Path, needle: &str) -> Result<String, String> {
    let log = store::read(dir)?;
    let canon = log.derive();
    let id = crate::explain::resolve_any(&canon, needle)?;
    Ok(crate::explain::explain(&log, &canon, &id)?.render(""))
}

#[cfg(test)]
mod tests;
