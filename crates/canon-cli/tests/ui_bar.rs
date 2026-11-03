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
    // HTTP completion is Content-Length, not a clean TCP EOF. Linux can reset
    // a closing socket after a complete response; incomplete bodies still fail.
    let mut s = BufReader::new(s);
    let mut out = String::new();
    loop {
        let mut line = String::new();
        assert!(
            s.read_line(&mut line).unwrap() > 0,
            "incomplete headers for {request}"
        );
        out.push_str(&line);
        if line == "\r\n" {
            break;
        }
    }
    let length: usize = out
        .lines()
        .find_map(|l| l.strip_prefix("Content-Length: "))
        .unwrap()
        .parse()
        .unwrap();
    let mut body = vec![0; length];
    s.read_exact(&mut body)
        .unwrap_or_else(|e| panic!("incomplete response for {request}: {out}: {e}"));
    out.push_str(std::str::from_utf8(&body).unwrap());
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
    assert!(canon(&["init", "--profile", "house", "--open"])
        .status
        .success());
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

#[test]
fn a_person_can_identify_and_then_write_signed_from_the_page() {
    let root = std::env::temp_dir().join(format!("canon-ui-identify-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    std::fs::create_dir_all(home.join(".ssh")).unwrap();
    assert!(Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-C", "alex", "-f"])
        .arg(home.join(".ssh/id_ed25519"))
        .status()
        .expect("this test needs ssh-keygen")
        .success());
    let env = |c: &mut Command| {
        c.env("CANON_DIR", root.join(".canon"))
            .env("CANON_ACTOR", "human:alex")
            .env("HOME", &home)
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("CANON_SIGNING_KEY")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1");
    };
    let mut ui = Command::new(env!("CARGO_BIN_EXE_canon"));
    env(&mut ui);
    let mut child = ui
        .arg("ui")
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
        .unwrap();
    let (host, rest) = url.split_once('/').unwrap();
    let port: u16 = host.strip_prefix("127.0.0.1:").unwrap().parse().unwrap();
    let token = rest.split("t=").nth(1).unwrap().to_string();
    let first = exchange(
        port,
        &format!("GET /{rest} HTTP/1.1\r\nHost: {host}\r\n\r\n"),
    );
    let cookie = first
        .lines()
        .find_map(|l| l.strip_prefix("Set-Cookie: "))
        .and_then(|c| c.split(';').next())
        .unwrap()
        .to_string();
    let get = |path: &str| {
        exchange(
            port,
            &format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nCookie: {cookie}\r\n\r\n"),
        )
    };
    let post = |path: &str, body: &str| {
        exchange(
            port,
            &format!(
                "POST {path} HTTP/1.1\r\nHost: {host}\r\nOrigin: http://{host}\r\nCookie: {cookie}\r\n\
                 Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}",
                body.len()
            ),
        )
    };

    assert!(get("/").contains("Create a canon"));
    assert!(post("/setup", &format!("token={token}&profile=house")).starts_with("HTTP/1.1 303"));
    let offered = get("/identify");
    let fingerprint = offered
        .split("value=\"SHA256:")
        .nth(1)
        .and_then(|s| s.split('"').next())
        .map(|f| format!("SHA256:{f}"))
        .expect("the key in ~/.ssh is offered");
    let enc = |s: &str| {
        s.replace('+', "%2B")
            .replace('/', "%2F")
            .replace('=', "%3D")
            .replace(':', "%3A")
    };
    let done = post(
        "/identify",
        &format!("token={token}&key={}", enc(&fingerprint)),
    );
    assert!(done.contains("You are identified with"), "{done}");
    let wrote = post(
        "/silence",
        &format!("token={token}&about=guests&because=We+trust+each+other"),
    );
    assert!(wrote.starts_with("HTTP/1.1 303"), "{wrote}");
    let record = std::fs::read_to_string(root.join(".canon/acts.jsonl")).unwrap();
    // The procedure and founding seat predate identification. The key and
    // subsequent silence are signed, including after browser-first setup.
    assert_eq!(record.lines().count(), 4, "{record}");
    assert!(
        record.lines().skip(2).all(|l| l.contains("\"sig\":")),
        "both signed: {record}"
    );
}

#[test]
fn an_empty_launch_directory_reaches_setup_and_founds_the_same_record_as_init() {
    let root = std::env::temp_dir().join(format!("canon-ui-first-run-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("home")).unwrap();
    for profile in ["personal", "house", "code"] {
        let browser = root.join(profile).join("browser");
        let terminal = root.join(profile).join("terminal");
        std::fs::create_dir_all(&browser).unwrap();
        std::fs::create_dir_all(&terminal).unwrap();
        let command = || {
            let mut c = Command::new(env!("CARGO_BIN_EXE_canon"));
            c.env_remove("CANON_DIR")
                .env("HOME", root.join("home"))
                .env("CANON_ACTOR", "human:alex");
            c
        };
        let mut child = command()
            .arg("ui")
            .current_dir(&browser)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let _page = Page(child);
        let url = BufReader::new(stdout)
            .lines()
            .map_while(Result::ok)
            .find_map(|l| l.trim().strip_prefix("http://").map(str::to_string))
            .expect("setup prints a browser URL");
        let (host, rest) = url.split_once('/').unwrap();
        let port: u16 = host.strip_prefix("127.0.0.1:").unwrap().parse().unwrap();
        let token = rest.strip_prefix("?t=").unwrap();
        let first = exchange(
            port,
            &format!("GET /{rest} HTTP/1.1\r\nHost: {host}\r\n\r\n"),
        );
        let cookie = first
            .lines()
            .find_map(|l| l.strip_prefix("Set-Cookie: "))
            .unwrap()
            .split(';')
            .next()
            .unwrap();
        let setup = exchange(
            port,
            &format!("GET / HTTP/1.1\r\nHost: {host}\r\nCookie: {cookie}\r\n\r\n"),
        );
        assert!(setup.contains("Create a canon"));
        assert!(!browser.join(".canon").exists(), "GET never initializes");
        let body = format!("token={token}&profile={profile}");
        let made = exchange(port, &format!("POST /setup HTTP/1.1\r\nHost: {host}\r\nCookie: {cookie}\r\nOrigin: http://{host}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{body}", body.len()));
        assert!(made.starts_with("HTTP/1.1 303"), "{made}");
        assert!(command()
            .args(["init", "--profile", profile])
            .current_dir(&terminal)
            .output()
            .unwrap()
            .status
            .success());
        let read = |dir: &std::path::Path| {
            let raw = std::fs::read_to_string(dir.join(".canon/acts.jsonl")).unwrap();
            canon_core::Log::parse(&raw).unwrap();
            // Compare append order: canonical ID order can change when the
            // browser and CLI creations land in different Unix seconds.
            raw.lines()
                .map(|l| serde_json::from_str::<canon_core::Act>(l).unwrap())
                .collect::<Vec<_>>()
        };
        let a = read(&browser);
        let b = read(&terminal);
        assert_eq!(a.len(), b.len());
        for (a, b) in a.iter().zip(&b) {
            assert_eq!(a.kind, b.kind);
            assert_eq!(a.actor, b.actor);
            assert_eq!(a.v, b.v);
            assert_eq!(
                a.id,
                canon_core::Act::new(a.kind.clone(), a.ts_unix, &a.actor).id
            );
        }
        assert_eq!(
            std::fs::read(browser.join(".canon/profile")).unwrap(),
            std::fs::read(terminal.join(".canon/profile")).unwrap()
        );
    }
}
