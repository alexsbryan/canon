// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon identify` — from here on, only your key writes as you.
//!
//! Without it, `CANON_ACTOR=human:you` is you. Run once, it binds your SSH
//! key to your name, and from then on the fold counts an act under that name
//! only if your key signed it. Run again, it says who you are here. That is
//! the whole verb: a name binds once, and a lost key means a new name. See
//! `canon_core::sign` for the rules and `docs/SIGNING_DESIGN.md` for why.
//!
//! **Nothing to configure.** The log says which keys are bound to your name,
//! so canon finds the one to sign with the way SSH would: its public half in
//! `~/.ssh`, else your ssh-agent. `CANON_SIGNING_KEY` names a key kept
//! anywhere else. A key belongs to a person, not to a canon directory —
//! several people can share a folder, and a key one of them named there
//! must never become another's default.
//!
//! **Canon never holds a private key.** It hands `ssh-keygen -Y sign` the
//! bytes to sign and keeps what comes back.
//!
//! **A write that would not count is never written.** If your name is bound
//! and no key bound to it can sign here, the write is refused, the way an
//! approval that holds no standing is.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use canon_core::sign::{self, Binding, Key};
use canon_core::{Act, ActKind, Log};

use crate::cmds::{fail, load, positionals};
use crate::store;

/// Names a key kept somewhere canon would not look.
pub const ENV: &str = "CANON_SIGNING_KEY";

/// The key this person signs with.
struct Signing {
    /// What `ssh-keygen -f` is given: a public key (it finds the private
    /// half beside it, or in the agent) or a private key.
    file: PathBuf,
    key: Key,
    /// A public key written out so `ssh-keygen` can ask the agent for its
    /// private half; ours to remove.
    temporary: bool,
}

impl Drop for Signing {
    fn drop(&mut self) {
        if self.temporary {
            let _ = std::fs::remove_file(&self.file);
        }
    }
}

/// A key from a path, public or private half.
fn load_key(raw: &str) -> Result<Signing, String> {
    let file = match raw.strip_prefix("~/") {
        Some(rest) => store::home()
            .ok_or("`~` in a key path, and no home directory")?
            .join(rest),
        None => PathBuf::from(raw),
    };
    let public = if file.extension().is_some_and(|e| e == "pub") {
        file.clone()
    } else {
        let mut p = file.clone().into_os_string();
        p.push(".pub");
        PathBuf::from(p)
    };
    let text = std::fs::read_to_string(&public)
        .map_err(|e| format!("reading {}: {e}", public.display()))?;
    let key = Key::parse(text.lines().next().unwrap_or(""))
        .map_err(|e| format!("{}: {e}", public.display()))?;
    Ok(Signing {
        file,
        key,
        temporary: false,
    })
}

/// Every Ed25519 public key in `~/.ssh`, readable or not skipped quietly.
fn in_dot_ssh() -> Vec<Signing> {
    let Some(dir) = store::home().map(|h| h.join(".ssh")) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<Signing> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "pub"))
        .filter_map(|p| load_key(&p.display().to_string()).ok())
        .collect();
    found.sort_by(|a, b| a.file.cmp(&b.file));
    found
}

/// The keys the ssh-agent holds, as `ssh-add -L` lists them. None when there
/// is no agent.
fn in_agent() -> Vec<Key> {
    Command::new("ssh-add")
        .arg("-L")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| Key::parse(l).ok())
                .collect()
        })
        .unwrap_or_default()
}

/// The key to sign with for a name bound to `keys`.
fn signer(keys: &[Key]) -> Result<Signing, String> {
    if let Some(raw) = std::env::var(ENV).ok().filter(|v| !v.trim().is_empty()) {
        let s = load_key(raw.trim())?;
        return if keys.contains(&s.key) {
            Ok(s)
        } else {
            Err(format!(
                "{ENV} names {}, which is not a key bound to this name ({})",
                s.key.fingerprint(),
                fingerprints(keys)
            ))
        };
    }
    if let Some(s) = in_dot_ssh().into_iter().find(|s| keys.contains(&s.key)) {
        return Ok(s);
    }
    // Not on disk where we look, so in the agent or nowhere. `ssh-keygen -f`
    // with only a public key asks the agent for the private half.
    let Some(key) = in_agent().into_iter().find(|k| keys.contains(k)) else {
        return Err(format!(
            "no key bound to it ({}) is in ~/.ssh or your ssh-agent here — `ssh-add` it, or \
             set {ENV} to where it is",
            fingerprints(keys)
        ));
    };
    let file = std::env::temp_dir().join(format!("canon-signing-{}.pub", std::process::id()));
    std::fs::write(&file, format!("{}\n", key.openssh()))
        .map_err(|e| format!("writing {}: {e}", file.display()))?;
    Ok(Signing {
        file,
        key,
        temporary: true,
    })
}

fn fingerprints(keys: &[Key]) -> String {
    keys.iter()
        .map(|k| {
            format!(
                "{}{}",
                k.fingerprint(),
                if k.touch_required() { " (touch)" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// Ask `ssh-keygen` to sign `message`. The SSHSIG blob, armour off.
fn ssh_sign(signing: &Signing, message: &[u8]) -> Result<String, String> {
    if signing.key.touch_required() {
        // `-q` keeps ssh-keygen's own chatter off the screen, its touch
        // prompt included, so this one says it instead.
        eprintln!(
            "touch your security key to sign ({})",
            signing.key.fingerprint()
        );
    }
    let mut child = Command::new("ssh-keygen")
        .args(["-q", "-Y", "sign", "-n", sign::NAMESPACE, "-f"])
        .arg(&signing.file)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("running ssh-keygen: {e} — signing needs OpenSSH 8.1 or later"))?;
    child
        .stdin
        .take()
        .expect("piped")
        .write_all(message)
        .map_err(|e| format!("writing to ssh-keygen: {e}"))?;
    let out = child
        .wait_with_output()
        .map_err(|e| format!("waiting for ssh-keygen: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "ssh-keygen could not sign with {}: {}",
            signing.file.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let armoured =
        String::from_utf8(out.stdout).map_err(|_| "ssh-keygen wrote something that is not text")?;
    Ok(armoured
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect())
}

/// Sign `act` against `log`, and check it the way every reader will before
/// it is anywhere near the record.
fn seal(log: &Log, mut act: Act, signing: &Signing) -> Result<Act, String> {
    let message = sign::prepare(&mut act, log.acts());
    act.sig = Some(ssh_sign(signing, message.as_bytes())?);
    let mut with = log.acts().to_vec();
    with.push(act.clone());
    if let Some((_, why)) = sign::verify(&with)
        .unverified
        .into_iter()
        .find(|(id, _)| *id == act.id)
    {
        return Err(format!("signed, but it would not count: {why}"));
    }
    Ok(act)
}

/// Every write goes through here. An actor with no key passes straight
/// through; a bound one is signed or refused.
pub fn sign_if_bound(log: &Log, act: Act) -> Result<Act, String> {
    let checked = sign::verify(log.acts());
    let Some(bound) = checked.bindings.get(&act.actor) else {
        return Ok(act);
    };
    let nothing = |e: String| format!("{e}\nNothing was written.");
    let signing = usable(bound).map_err(nothing)?;
    seal(log, act, &signing).map_err(nothing)
}

/// A key that may sign for `bound`, or why there is none.
fn usable(bound: &Binding) -> Result<Signing, String> {
    if bound.contested {
        return Err(format!(
            "{} is contested: two key acts bind it, so somebody else holds a key that signs \
             for it and nothing more written under it can count. Write under a new name and \
             ask a holder to grant it.",
            bound.actor
        ));
    }
    signer(&bound.keys).map_err(|e| format!("{} has bound a key, and {e}", bound.actor))
}

// ── the verb ────────────────────────────────────────────────

pub fn run(args: &[String]) -> i32 {
    identify(&positionals(args)).unwrap_or_else(fail)
}

fn identify(pos: &[&str]) -> Result<i32, String> {
    let (d, log, _) = load()?;
    let actor = store::actor();
    let checked = sign::verify(log.acts());
    match checked.bindings.get(&actor) {
        None => bind(&d, &log, &actor, pos),
        Some(bound) => {
            if !pos.is_empty() {
                return Err(format!(
                    "{actor} is already bound, and a name binds once — a new key means a new name"
                ));
            }
            Ok(show(&log, &checked, bound))
        }
    }
}

fn bind(d: &Path, log: &Log, actor: &str, pos: &[&str]) -> Result<i32, String> {
    let first = match pos.first() {
        Some(p) => load_key(p)?,
        None => match std::env::var(ENV).ok().filter(|v| !v.trim().is_empty()) {
            Some(raw) => load_key(raw.trim())?,
            None => {
                let here: Vec<String> = in_dot_ssh()
                    .iter()
                    .map(|s| format!("  {}  {}", s.file.display(), s.key.fingerprint()))
                    .collect();
                return Err(format!(
                    "which key? `canon identify ~/.ssh/<key>.pub`. A key made with `ssh-keygen \
                     -t ed25519-sk` needs your touch to sign, so nothing running as you can sign \
                     without you.{}",
                    if here.is_empty() {
                        String::new()
                    } else {
                        format!("\nIn ~/.ssh:\n{}", here.join("\n"))
                    }
                ));
            }
        },
    };
    let mut extras = Vec::new();
    for extra in pos.iter().skip(1) {
        extras.push(load_key(extra)?.key.clone());
    }
    let (keys, vouched) = bind_with(d, log, actor, &first, &extras)?;
    println!("{actor} is bound to {}", fingerprints(&keys));
    if vouched > 0 {
        println!("  and vouches for the {vouched} act(s) already written under it");
    }
    println!(
        "From here on, only these keys can write as {actor}. Tell the group the fingerprints \
         some other way than this file: anyone who can write to it could have bound the name \
         instead, and comparing is how you would know. A name binds once, so losing every key \
         means a new name."
    );
    if !keys.iter().any(Key::touch_required) {
        println!(
            "None of these keys needs a touch, so anything that can use them without asking — \
             an agent running as you, a loaded ssh-agent — can sign as you. A key made with \
             `ssh-keygen -t ed25519-sk`, or `ssh-add -c`, closes that."
        );
    }
    Ok(0)
}

/// Bind `signing` (and `extras`) to `actor`: the key act, vouching for
/// everything already written under the name, signed and appended. The keys
/// bound and how many acts were vouched for. The one way both the terminal
/// and the page identify.
fn bind_with(
    d: &Path,
    log: &Log,
    actor: &str,
    signing: &Signing,
    extras: &[Key],
) -> Result<(Vec<Key>, usize), String> {
    let mut keys = vec![signing.key.clone()];
    for k in extras {
        if !keys.contains(k) {
            keys.push(k.clone());
        }
    }
    let prior: Vec<String> = log
        .acts()
        .iter()
        .filter(|a| a.actor == actor)
        .map(sign::digest)
        .collect();
    let vouched = prior.len();
    let act = Act::new(
        ActKind::Key {
            keys: keys.iter().map(Key::openssh).collect(),
            prior,
        },
        store::now(),
        actor,
    );
    let act = seal(log, act, signing)?;
    store::append(d, &act)?;
    Ok((keys, vouched))
}

/// A key this person could identify with, as the page offers it.
pub struct Offer {
    pub fingerprint: String,
    /// Where it was found: a file in `~/.ssh`, or the ssh-agent.
    pub found: String,
    pub touch: bool,
}

/// The keys this person could identify with: Ed25519 public keys in
/// `~/.ssh`, then whatever else the ssh-agent holds. Offered by
/// fingerprint, so the page never takes a path from a form.
pub fn offers() -> Vec<Offer> {
    let mut out: Vec<Offer> = in_dot_ssh()
        .iter()
        .map(|s| Offer {
            fingerprint: s.key.fingerprint(),
            found: s.file.display().to_string(),
            touch: s.key.touch_required(),
        })
        .collect();
    for k in in_agent() {
        if !out.iter().any(|o| o.fingerprint == k.fingerprint()) {
            out.push(Offer {
                fingerprint: k.fingerprint(),
                found: "your ssh-agent".into(),
                touch: k.touch_required(),
            });
        }
    }
    out
}

/// Identify `actor` with the offered key whose fingerprint is
/// `fingerprint`. What the page says afterwards.
pub fn identify_by_fingerprint(d: &Path, actor: &str, fingerprint: &str) -> Result<String, String> {
    let log = store::read(d)?;
    if sign::verify(log.acts()).bindings.contains_key(actor) {
        return Err(format!(
            "{actor} is already identified, and a name binds once"
        ));
    }
    let signing = match in_dot_ssh()
        .into_iter()
        .find(|s| s.key.fingerprint() == fingerprint)
    {
        Some(s) => s,
        None => {
            let key = in_agent()
                .into_iter()
                .find(|k| k.fingerprint() == fingerprint)
                .ok_or("that key is no longer in ~/.ssh or your ssh-agent")?;
            let file =
                std::env::temp_dir().join(format!("canon-signing-{}.pub", std::process::id()));
            std::fs::write(&file, format!("{}\n", key.openssh()))
                .map_err(|e| format!("writing {}: {e}", file.display()))?;
            Signing {
                file,
                key,
                temporary: true,
            }
        }
    };
    let (keys, vouched) = bind_with(d, &log, actor, &signing, &[])?;
    let mut said = format!(
        "You are identified with {}{}. Tell the group this fingerprint some other way than this file.",
        fingerprints(&keys),
        if vouched > 0 {
            format!(", vouching for the {vouched} acts already written under your name")
        } else {
            String::new()
        }
    );
    if !keys.iter().any(Key::touch_required) {
        said.push_str(
            " This key needs no touch, so a program running as you could sign with it; \
             a key made with ssh-keygen -t ed25519-sk closes that.",
        );
    }
    Ok(said)
}

fn show(log: &Log, checked: &sign::Verified, bound: &Binding) -> i32 {
    let actor = &bound.actor;
    if bound.contested {
        println!("{actor} is contested: two key acts bind it, and nothing signed under it counts");
    } else {
        println!("{actor} is bound to {}", fingerprints(&bound.keys));
    }
    let lost = checked
        .unverified
        .iter()
        .filter(|(id, _)| log.acts().iter().any(|a| a.id == *id && a.actor == *actor))
        .count();
    if lost > 0 {
        println!("  {lost} act(s) under this name do not count — `canon log` marks them");
    }
    let others = checked.bindings.len() - 1;
    if others > 0 {
        println!("  {others} other name(s) here have bound a key");
    }
    if bound.contested {
        return 2;
    }
    match signer(&bound.keys) {
        Ok(s) if s.temporary => {
            println!(
                "you sign here with {} from your ssh-agent",
                s.key.fingerprint()
            );
            0
        }
        Ok(s) => {
            println!("you sign here with {}", s.file.display());
            0
        }
        Err(e) => {
            println!("{actor} has bound a key, and {e}");
            2
        }
    }
}
