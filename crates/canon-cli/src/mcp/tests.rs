// SPDX-License-Identifier: AGPL-3.0-or-later
//! The wire contract, and the one structural invariant this module exists to
//! hold: nothing here can write.

use super::*;

fn req(body: Value) -> Option<Value> {
    handle(&body)
}

// ── the invariant ───────────────────────────────────────────

#[test]
fn the_surface_is_read_only() {
    // The guard that makes "agents propose, humans dispose" structural. If a
    // tool that writes an act is ever added, this fails — deliberately
    // annoying to update, because the update IS the decision.
    let names: Vec<String> = tool_descriptors()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(
        names,
        tools::READ_ONLY_TOOLS,
        "the advertised tools must be exactly the declared read-only set"
    );

    for forbidden in [
        "add",
        "assert",
        "supersede",
        "retract",
        "accept",
        "dismiss",
        "undo",
        "revert",
        "init",
        "adopt",
        "write",
        "set",
        "delete",
        "create",
    ] {
        assert!(
            !names.iter().any(|n| n.contains(forbidden)),
            "`{forbidden}` must not appear in an MCP tool name — writes belong to the CLI"
        );
    }
}

#[test]
fn every_advertised_tool_declares_a_schema() {
    for t in tool_descriptors() {
        assert!(t["name"].is_string());
        assert!(
            t["description"].is_string(),
            "an agent picks tools by description"
        );
        assert_eq!(t["inputSchema"]["type"], "object");
        assert_eq!(
            t["inputSchema"]["additionalProperties"], false,
            "unknown arguments must be rejected, not ignored"
        );
    }
}

// ── the handshake ───────────────────────────────────────────

#[test]
fn initialize_echoes_a_protocol_it_supports() {
    let r = req(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2024-11-05" }
    }))
    .expect("a request gets a reply");
    assert_eq!(r["result"]["protocolVersion"], "2024-11-05");
    assert_eq!(r["result"]["serverInfo"]["name"], "canon");
    assert!(r["result"]["capabilities"]["tools"].is_object());
}

#[test]
fn initialize_falls_back_when_asked_for_an_unknown_protocol() {
    let r = req(json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "1999-01-01" }
    }))
    .unwrap();
    assert_eq!(r["result"]["protocolVersion"], PREFERRED_PROTOCOL);
}

#[test]
fn a_notification_gets_no_reply() {
    // No `id` means no response. Answering one corrupts the stream.
    assert!(req(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
}

#[test]
fn ping_answers() {
    let r = req(json!({ "jsonrpc": "2.0", "id": "abc", "method": "ping" })).unwrap();
    assert_eq!(r["id"], "abc");
    assert!(r["error"].is_null());
}

#[test]
fn a_string_id_survives_the_round_trip() {
    // JSON-RPC ids may be strings or numbers; echoing the wrong type loses
    // the client's correlation.
    let r = req(json!({ "jsonrpc": "2.0", "id": "req-7", "method": "tools/list" })).unwrap();
    assert_eq!(r["id"], "req-7");
}

#[test]
fn an_unknown_method_is_a_jsonrpc_error() {
    let r = req(json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/destroy" })).unwrap();
    assert_eq!(r["error"]["code"], -32601);
}

// ── tools/call ──────────────────────────────────────────────

#[test]
fn an_unknown_tool_is_recoverable_not_fatal() {
    // isError:false so an agent that mistyped can read the reply and retry
    // rather than treating the whole surface as broken. No canon is set up
    // here on purpose: the tool NAME is checked before a canon is located, so
    // the reply names the real problem instead of "no canon found".
    let r = req(json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "canon_delete_everything", "arguments": {} }
    }))
    .unwrap();
    assert_eq!(r["result"]["isError"], false);
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(
        text.contains("canon_list"),
        "the reply names what is available"
    );
}

#[test]
fn canon_why_without_an_id_is_an_error_the_agent_can_read() {
    let r = req(json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/call",
        "params": { "name": "canon_why", "arguments": {} }
    }))
    .unwrap();
    assert_eq!(r["result"]["isError"], true, "a bad argument IS an error");
}
