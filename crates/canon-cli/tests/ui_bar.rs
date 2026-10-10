// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon ui` over a real socket: it listens on this machine only, the
//! printed address lets a browser in, and an idle connection — which every
//! browser opens ahead of time — does not stop the page answering.
//!
//! The door's other checks are unit tests on `ui::respond`; this pins the
//! part only a socket can get wrong.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

struct Page(Child);

impl Drop for Page {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn exchange(port: u16, request: &str) -> String {
    let mut s = TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.write_all(request.as_bytes()).unwrap();
    let mut out = String::new();
    s.read_to_string(&mut out).unwrap();
    out
}

#[test]
fn the_page_lets_in_its_own_address_and_is_not_held_by_an_idle_socket() {
    let root = std::env::temp_dir().join(format!("canon-ui-bar-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let canon = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_canon"))
            .args(args)
            .env("CANON_DIR", root.join(".canon"))
            .env("CANON_ACTOR", "human:alex")
            .current_dir(&root)
            .output()
            .unwrap()
    };
    assert!(canon(&["init", "--profile", "house"]).status.success());
    assert!(canon(&["add", "Quiet hours are 10pm to 7am."])
        .status
        .success());

    let mut child = Command::new(env!("CARGO_BIN_EXE_canon"))
        .arg("ui")
        .env("CANON_DIR", root.join(".canon"))
        .env("CANON_ACTOR", "human:alex")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let _page = Page(child);
    let url = BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .find_map(|l| l.trim().strip_prefix("http://").map(str::to_string))
        .expect("the page prints its address");
    let (host, rest) = url.split_once('/').unwrap();
    let port: u16 = host
        .strip_prefix("127.0.0.1:")
        .expect("loopback only")
        .parse()
        .unwrap();

    // A browser's speculative connection: opened, and never used.
    let _idle = TcpStream::connect(("127.0.0.1", port)).unwrap();

    let first = exchange(
        port,
        &format!("GET /{rest} HTTP/1.1\r\nHost: {host}\r\n\r\n"),
    );
    assert!(first.starts_with("HTTP/1.1 303"), "{first}");
    let cookie = first
        .lines()
        .find_map(|l| l.strip_prefix("Set-Cookie: "))
        .and_then(|c| c.split(';').next())
        .expect("a cookie for the token")
        .to_string();
    let page = exchange(
        port,
        &format!("GET / HTTP/1.1\r\nHost: {host}\r\nCookie: {cookie}\r\n\r\n"),
    );
    assert!(page.starts_with("HTTP/1.1 200"), "{page}");
    assert!(page.contains("Content-Security-Policy: default-src 'none'"));
    assert!(page.contains("Quiet hours are 10pm to 7am."));
    let refused = exchange(port, &format!("GET / HTTP/1.1\r\nHost: {host}\r\n\r\n"));
    assert!(refused.starts_with("HTTP/1.1 403"), "{refused}");
}
