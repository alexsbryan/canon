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
    pub const READ_ONLY_TOOLS: &[&str] = &["canon_list", "canon_why"];
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
            "description": "The commitments currently in force. For a small canon this is the \
                            whole integration: read them once at the start of a task and reason \
                            over them directly.",
            "inputSchema": { "type": "object", "properties": {}, "additionalProperties": false },
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
    ]
}

fn call(name: &str, args: &Value) -> Value {
    let dir = match locate() {
        Ok(d) => d,
        Err(e) => return content(e, true),
    };
    match name {
        "canon_list" => match render_list(&dir) {
            Ok(s) => content(s, false),
            Err(e) => content(e, true),
        },
        "canon_why" => {
            let Some(id) = args.get("id").and_then(Value::as_str) else {
                return content("canon_why requires an `id` argument.", true);
            };
            match render_why(&dir, id) {
                Ok(s) => content(s, false),
                Err(e) => content(e, true),
            }
        }
        // Unknown tool is NOT a hard error: an agent that mistypes should be
        // able to recover from the reply rather than treat the surface as
        // broken.
        other => content(
            format!(
                "no tool named `{other}`. Available: {}.",
                tools::READ_ONLY_TOOLS.join(", ")
            ),
            false,
        ),
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

fn render_list(dir: &Path) -> Result<String, String> {
    let canon = store::read(dir)?.derive();
    let live: Vec<_> = canon.active().collect();
    if live.is_empty() {
        return Ok("This canon has no commitments yet.".into());
    }
    let mut out = String::new();
    for c in &live {
        out.push_str(&format!("{}  {}\n", c.id, c.text));
    }
    out.push_str(&format!("\n{} in force.", live.len()));

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

fn render_why(dir: &Path, needle: &str) -> Result<String, String> {
    let log = store::read(dir)?;
    let canon = log.derive();
    let id = crate::explain::resolve(&canon, needle)?;
    Ok(crate::explain::explain(&log, &canon, &id)?.render(""))
}

#[cfg(test)]
mod tests;
