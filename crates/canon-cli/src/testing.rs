// SPDX-License-Identifier: AGPL-3.0-or-later
//! A mock OpenAI-compatible endpoint, shared by every test that needs one.
//!
//! One implementation rather than one per test module: two mocks drift, and a
//! test asserting against a drifted mock proves nothing about the wire.
//!
//! Pattern borrowed from `sovereign-cli-llm`'s `egress_reds.rs` — a
//! one-connection-per-response local listener recording request bodies —
//! reimplemented over `std::net` because `canon` has no async runtime and is
//! not getting one.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use crate::config::{ApiKey, Config};
use crate::model::Client;

pub struct Mock {
    pub base: String,
    recorded: Arc<Mutex<Vec<Value>>>,
    raw: Arc<Mutex<Vec<String>>>,
    heads: Arc<Mutex<Vec<String>>>,
}

impl Mock {
    /// Serve exactly `script.len()` requests, in order, then stop listening.
    pub fn spawn(script: Vec<(u16, String)>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
        let port = listener.local_addr().unwrap().port();
        let recorded: Arc<Mutex<Vec<Value>>> = Arc::new(Mutex::new(Vec::new()));
        let raw: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let heads: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let rec = Arc::clone(&recorded);
        let raw_rec = Arc::clone(&raw);
        let heads_rec = Arc::clone(&heads);
        std::thread::spawn(move || {
            for (status, body) in script {
                let Ok((mut sock, _)) = listener.accept() else {
                    return;
                };
                let mut buf: Vec<u8> = Vec::new();
                let mut tmp = [0u8; 4096];
                let request_body = loop {
                    if let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let header = String::from_utf8_lossy(&buf[..pos]).to_string();
                        heads_rec.lock().unwrap().push(header.clone());
                        let len = header
                            .lines()
                            .find_map(|l| {
                                let (k, v) = l.split_once(':')?;
                                k.eq_ignore_ascii_case("content-length")
                                    .then(|| v.trim().parse::<usize>().ok())
                                    .flatten()
                            })
                            .unwrap_or(0);
                        let start = pos + 4;
                        while buf.len() < start + len {
                            match sock.read(&mut tmp) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => buf.extend_from_slice(&tmp[..n]),
                            }
                        }
                        break String::from_utf8_lossy(&buf[start..(start + len).min(buf.len())])
                            .to_string();
                    }
                    match sock.read(&mut tmp) {
                        Ok(0) | Err(_) => break String::new(),
                        Ok(n) => buf.extend_from_slice(&tmp[..n]),
                    }
                };
                if let Ok(v) = serde_json::from_str::<Value>(&request_body) {
                    rec.lock().unwrap().push(v);
                }
                raw_rec.lock().unwrap().push(request_body);
                let reason = if (200..300).contains(&status) {
                    "OK"
                } else {
                    "Bad Request"
                };
                let resp = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\n\
                     content-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = sock.write_all(resp.as_bytes());
                let _ = sock.flush();
            }
        });
        Self {
            base: format!("http://127.0.0.1:{port}/v1"),
            recorded,
            raw,
            heads,
        }
    }

    pub fn client(&self) -> Client {
        Client::new(&Config {
            endpoint: Some(self.base.clone()),
            ..Config::default()
        })
        .expect("client")
    }

    /// A client that sends `key` as its bearer token.
    pub fn client_with_key(&self, key: &str) -> Client {
        self.client().with_api_key(ApiKey::new(key))
    }

    /// The request line and headers of each request, in order.
    pub fn request_heads(&self) -> Vec<String> {
        self.heads.lock().unwrap().clone()
    }

    /// The request bodies the mock received, in order.
    pub fn requests(&self) -> Vec<Value> {
        self.recorded.lock().unwrap().clone()
    }

    /// The request bodies exactly as they arrived, in order.
    ///
    /// [`Mock::requests`] parses them, and a parsed `Value` sorts its keys in
    /// this build, so a test about the order of what went over the wire has
    /// to read these instead.
    pub fn raw_requests(&self) -> Vec<String> {
        self.raw.lock().unwrap().clone()
    }
}

/// An OpenAI-shaped completion carrying `content`.
pub fn completion(content: &str) -> String {
    json!({ "choices": [{ "message": { "role": "assistant", "content": content } }] }).to_string()
}
