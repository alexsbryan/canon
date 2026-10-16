// SPDX-License-Identifier: AGPL-3.0-or-later
//! Signed acts through the real binary and the real `ssh-keygen`.
//!
//! The core's own tests sign with keys they hold in memory. These pin what
//! only the CLI can get wrong: that a write under a bound name is signed or
//! refused and never appended unsigned, that the refusal leaves the file
//! byte-for-byte as it was, and that a signed line checks out with
//! `ssh-keygen -Y verify` and no canon at all — the format promises a reader
//! that much.
//!
//! These need OpenSSH's `ssh-keygen` (8.1 or later) and fail without it,
//! rather than skipping: a gate that passes because it did not run is the
//! thing this repository keeps refusing to ship.

use std::path::{Path, PathBuf};
use std::process::Command;

struct Canon {
    root: PathBuf,
    dir: PathBuf,
}

impl Canon {
    fn new(name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("canon-signing-bar-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let c = Self {
            dir: root.join(".canon"),
            root,
        };
        let (code, out) = c.run("human:alex", None, &["init", "--profile", "house"]);
        assert_eq!(code, 0, "{out}");
        c
    }

    /// An Ed25519 key with no passphrase, outside `~/.ssh`. Its public
    /// half's path.
    fn key(&self, name: &str) -> PathBuf {
        let path = self.root.join(name);
        let ok = Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-C", name, "-f"])
            .arg(&path)
            .status()
            .expect("these tests need ssh-keygen");
        assert!(ok.success());
        path.with_extension("pub")
    }

    /// Put a key where canon looks for it on this machine: `~/.ssh`.
    fn install(&self, public: &Path) {
        let ssh = self.root.join("home/.ssh");
        std::fs::create_dir_all(&ssh).unwrap();
        let name = public.file_stem().unwrap().to_str().unwrap();
        std::fs::copy(public, ssh.join(format!("{name}.pub"))).unwrap();
        std::fs::copy(public.with_extension(""), ssh.join(name)).unwrap();
    }

    /// Run as `actor`, naming `key` in `CANON_SIGNING_KEY` or not. Home is
    /// the test's own and the agent is shut out, so a developer's keys never
    /// leak in.
    fn run(&self, actor: &str, key: Option<&Path>, args: &[&str]) -> (i32, String) {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_canon"));
        cmd.args(args)
            .env("CANON_DIR", &self.dir)
            .env("CANON_ACTOR", actor)
            .env("HOME", self.root.join("home"))
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env_remove("SSH_AUTH_SOCK")
            .env_remove("CANON_SIGNING_KEY")
            .current_dir(&self.root);
        if let Some(k) = key {
            cmd.env("CANON_SIGNING_KEY", k);
        }
        let out = cmd.output().expect("canon runs");
        (
            out.status.code().unwrap_or(-1),
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            ),
        )
    }

    fn acts(&self) -> String {
        std::fs::read_to_string(self.dir.join("acts.jsonl")).unwrap()
    }

    /// A house alex founded and wrote a rule in, then identified in, with
    /// the key in alex's `~/.ssh`.
    fn bound(name: &str) -> (Self, PathBuf) {
        let c = Self::new(name);
        let alex = c.key("alex");
        c.install(&alex);
        for args in [
            &["grant", "human:alex", "house"][..],
            &["add", "Quiet hours are 10pm to 7am.", "--scope", "house"],
        ] {
            let (code, out) = c.run("human:alex", None, args);
            assert_eq!(code, 0, "{out}");
        }
        let (code, out) = c.run("human:alex", None, &["identify", alex.to_str().unwrap()]);
        assert_eq!(code, 0, "{out}");
        // The founding act `init` wrote, the grant and the add.
        assert!(out.contains("vouches for the 4 act(s)"), "{out}");
        (c, alex)
    }

    /// Another machine, or another person's account: nothing in `~/.ssh`.
    fn elsewhere(&self) {
        let _ = std::fs::remove_dir_all(self.root.join("home/.ssh"));
    }
}

#[test]
fn the_env_var_alone_no_longer_writes_as_a_bound_name() {
    let (c, alex) = Canon::bound("env");
    let (code, out) = c.run(
        "human:alex",
        Some(&alex),
        &["add", "Guests may stay two nights."],
    );
    assert_eq!(code, 0, "{out}");
    let last = c.acts().lines().last().unwrap().to_string();
    assert!(
        last.contains(r#""v":3"#) && last.contains(r#""sig":"#),
        "{last}"
    );

    // Something that can append to the file but cannot use alex's key.
    c.elsewhere();
    let before = c.acts();
    let mallory = c.key("mallory");
    for key in [None, Some(mallory.as_path())] {
        let (code, out) = c.run("human:alex", key, &["add", "The agent may merge."]);
        assert_eq!(code, 2, "{out}");
        assert!(out.contains("Nothing was written."), "{out}");
        assert_eq!(
            c.acts(),
            before,
            "a refused write leaves the file as it was"
        );
    }
    // Somebody who never bound a key is untouched.
    let (code, out) = c.run("human:sam", None, &["add", "Bikes live in the shed."]);
    assert_eq!(code, 0, "{out}");
    assert!(!c.acts().lines().last().unwrap().contains(r#""sig":"#));
}

#[test]
fn a_line_written_into_the_file_by_hand_does_not_count_and_says_why() {
    let (c, _) = Canon::bound("by-hand");
    // What anything that can append to the file can do: a grant to itself,
    // under alex's name, with no signature.
    let line = r#"{"id":"can-0000000000aa","v":2,"ts_unix":1900000000,"actor":"human:alex","op":"grant","holder":"agent:mallory","scope":"house"}"#;
    std::fs::write(c.dir.join("acts.jsonl"), format!("{}{line}\n", c.acts())).unwrap();

    let (_, log) = c.run("human:alex", None, &["log"]);
    assert!(
        log.contains("DOES NOT COUNT: not signed, and human:alex has bound a key"),
        "{log}"
    );
    let (_, who) = c.run("human:alex", None, &["who", "house"]);
    assert!(!who.contains("agent:mallory"), "{who}");
    let (_, list) = c.run("human:alex", None, &["list"]);
    assert!(
        list.contains("were not signed by it, and do not count: 1 grant"),
        "{list}"
    );
}

#[test]
fn a_signed_line_verifies_with_ssh_keygen_and_no_canon() {
    let (c, alex) = Canon::bound("by-hand-verify");
    let (code, out) = c.run(
        "human:alex",
        Some(&alex),
        &["add", "Guests may stay two nights."],
    );
    assert_eq!(code, 0, "{out}");

    // SPEC: the canonical bytes are the line's object without `sig`, keys
    // sorted, compact. serde_json's map is sorted in this build.
    let line = c.acts().lines().last().unwrap().to_string();
    let mut value: serde_json::Value = serde_json::from_str(&line).unwrap();
    let sig = value
        .as_object_mut()
        .unwrap()
        .remove("sig")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();
    let message = serde_json::to_string(&value).unwrap();
    let wrapped: Vec<String> = sig
        .as_bytes()
        .chunks(70)
        .map(|c| String::from_utf8(c.to_vec()).unwrap())
        .collect();
    let armoured = format!(
        "-----BEGIN SSH SIGNATURE-----\n{}\n-----END SSH SIGNATURE-----\n",
        wrapped.join("\n")
    );
    let sig_file = c.root.join("line.sig");
    std::fs::write(&sig_file, armoured).unwrap();
    let allowed = c.root.join("allowed_signers");
    let public = std::fs::read_to_string(&alex).unwrap();
    std::fs::write(
        &allowed,
        format!("human:alex namespaces=\"canon\" {public}"),
    )
    .unwrap();

    let verify = |message: &str| {
        let mut child = Command::new("ssh-keygen")
            .args(["-Y", "verify", "-n", "canon", "-I", "human:alex", "-f"])
            .arg(&allowed)
            .arg("-s")
            .arg(&sig_file)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(message.as_bytes())
            .unwrap();
        child.wait().unwrap().success()
    };
    assert!(
        verify(&message),
        "ssh-keygen rejects what canon signed: {message}"
    );
    assert!(!verify(&message.replace("two nights", "ten nights")));
}

#[test]
fn identify_founds_a_canon_and_needs_nothing_configured() {
    // The founding order: init, identify, then everything signed with the
    // key canon finds in ~/.ssh.
    let c = Canon::new("founding");
    let alex = c.key("alex");
    c.install(&alex);
    let (code, out) = c.run("human:alex", None, &["identify", alex.to_str().unwrap()]);
    assert_eq!(code, 0, "{out}");
    assert!(
        !c.dir.join("config").exists(),
        "nothing is remembered in the canon"
    );
    for args in [
        &["grant", "human:alex", "house"][..],
        &["add", "Quiet hours are 10pm to 7am.", "--scope", "house"],
    ] {
        let (code, out) = c.run("human:alex", None, args);
        assert_eq!(code, 0, "{out}");
    }
    // Everything after the founding act, which predates the key and counts
    // through its `prior`.
    assert!(c.acts().lines().skip(1).all(|l| l.contains(r#""sig":"#)));
    let (_, list) = c.run("human:alex", None, &["list"]);
    assert!(
        list.contains("Quiet hours") && !list.contains("do not count"),
        "{list}"
    );
    let (code, out) = c.run("human:alex", None, &["identify"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("human:alex is bound to SHA256:"), "{out}");
    assert!(out.contains("you sign here with"), "{out}");
}

#[test]
fn on_another_machine_the_key_is_found_where_ssh_keeps_it() {
    let (c, alex) = Canon::bound("second-machine");
    // A fresh clone on a machine without the key.
    c.elsewhere();
    let (code, out) = c.run("human:alex", None, &["add", "Guests may stay two nights."]);
    assert_eq!(code, 2, "{out}");
    assert!(out.contains("ssh-add"), "{out}");
    let (code, out) = c.run("human:alex", None, &["identify"]);
    assert_eq!(code, 2, "{out}");
    // The key arrives in ~/.ssh — copied, or a FIDO key's handle — and that
    // is all it takes.
    c.install(&alex);
    let (code, out) = c.run("human:alex", None, &["add", "Guests may stay two nights."]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn identify_refuses_what_it_cannot_do_honestly() {
    let (c, alex) = Canon::bound("refusals");
    let backup = c.key("backup");
    // A name binds once.
    let (code, out) = c.run(
        "human:alex",
        None,
        &["identify", alex.to_str().unwrap(), backup.to_str().unwrap()],
    );
    assert_eq!(code, 2, "{out}");
    // Without a key named there is nothing to bind, and what ~/.ssh holds
    // is offered rather than chosen.
    let (code, out) = c.run("human:sam", None, &["identify"]);
    assert_eq!(code, 2, "{out}");
    assert!(
        out.contains("which key?") && out.contains("alex.pub"),
        "{out}"
    );
    // And binding warns when nothing needs a touch.
    let (code, out) = c.run("human:sam", Some(&backup), &["identify"]);
    assert_eq!(code, 0, "{out}");
    assert!(out.contains("None of these keys needs a touch"), "{out}");
}
