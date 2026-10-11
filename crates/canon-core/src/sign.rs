// SPDX-License-Identifier: AGPL-3.0-or-later
//! Signed acts: who wrote a line, checked rather than taken as given.
//!
//! Without this the fold believes the `actor` string, so anyone who can
//! append to the file can ratify as anyone — `CANON_ACTOR=human:alex` is
//! enough. A person can opt out of that by binding an SSH key to their name
//! with a [`ActKind::Key`] act. From then on an act under that name is given
//! to the fold only if a bound key signed it, or the binding vouched for it
//! by digest. Everything else under the name is
//! **carried and not applied**, and listed in [`crate::Canon::unverified`].
//! A name that never binds folds exactly as it always did.
//!
//! **Content, not time.** Nothing here reads `ts_unix`. A rule like "the
//! binding protects acts after it" is beaten by a forgery dated a second
//! before the binding, and the fold has no clock to tell an honest offline
//! append from a backdated one. So the key act vouches for the actor's
//! history by digest, and anything else under the name that its keys did
//! not sign does not count.
//!
//! **One binding per name.** A second key act for the same name does not
//! replace the first; it contests the name, and then only what every binding
//! vouches for still counts. Losing every key means a new name. Bind a
//! backup key alongside the first if that would hurt.
//!
//! **What a signature covers** is [`canonical`]: the act's JSON object
//! without `sig`, keys sorted, compact. That includes `refs`, which pins
//! every act this one names — and the canon's earliest act — by full SHA-256,
//! because an id is 48 bits and two proposals sharing one take seconds to
//! find. A signed act must name at least one act that is present, so a line
//! pasted into an unrelated canon does not verify there. A key act is the
//! exception, because it may found the canon and have nothing to name.
//!
//! **Signatures are SSHSIG** (OpenSSH's `PROTOCOL.sshsig`), namespace
//! `canon`, from `ssh-ed25519` or `sk-ssh-ed25519@openssh.com` keys. Canon
//! never holds a private key: the CLI asks `ssh-keygen -Y sign`, and anyone
//! can check an act without canon by asking `ssh-keygen -Y verify`. A FIDO
//! signature counts only with its user-presence flag set, so "a person
//! touched the key" is something every reader can check, not just the
//! signer's machine.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use sha2::{Digest, Sha256, Sha512};

use crate::act::{Act, ActKind, FORMAT_VERSION};
use crate::id::ActId;

/// The SSHSIG namespace. A git commit signature is in `git`, so neither can
/// be replayed as the other.
pub const NAMESPACE: &str = "canon";

const MAGIC: &[u8] = b"SSHSIG";
const ED25519: &str = "ssh-ed25519";
const SK_ED25519: &str = "sk-ssh-ed25519@openssh.com";
/// FIDO user-presence: somebody touched the key for this signature.
const USER_PRESENT: u8 = 0x01;

// ── what is signed ──────────────────────────────────────────

/// The bytes a signature covers: the act's JSON object without `sig`, keys
/// sorted bytewise, no whitespace.
///
/// Rendered from the parsed act, not copied from the raw line, because the
/// git merge driver re-renders every line it unions and a signature has to
/// survive a merge. For every op this build knows, parse then render gives
/// these bytes back. A line carrying a field this build does not know loses
/// it here and stops verifying, which fails closed: it does not count.
pub fn canonical(act: &Act) -> String {
    // `serde_json::Map` is a `BTreeMap` in this build (`preserve_order` is
    // refused by the core boundary), so the keys come out sorted.
    let mut value = serde_json::to_value(act).expect("an act serializes");
    if let serde_json::Value::Object(map) = &mut value {
        map.remove("sig");
    }
    serde_json::to_string(&value).expect("a value serializes")
}

/// The full SHA-256 of an act's canonical bytes, in hex. What `refs` and
/// `prior` pin acts by.
pub fn digest(act: &Act) -> String {
    hex(&Sha256::digest(canonical(act).as_bytes()))
}

/// Make `act` ready to sign against `log`: the v3 version it will be written
/// at, and its `refs`. Returns the message to sign.
///
/// `refs` names the log's earliest act, which is what makes a signed line
/// pasted into an unrelated canon fail, and every act whose id appears
/// anywhere in this act's body. Scanning the body rather than listing fields
/// per op means an op added later is pinned without anybody remembering to.
pub fn prepare(act: &mut Act, log: &[Act]) -> String {
    act.v = FORMAT_VERSION;
    let by_id: BTreeMap<&ActId, &Act> = log.iter().map(|a| (&a.id, a)).collect();
    let mut named = BTreeSet::new();
    if let Some(anchor) = log
        .iter()
        .min_by(|a, b| (a.ts_unix, &a.id).cmp(&(b.ts_unix, &b.id)))
    {
        named.insert(anchor.id.clone());
    }
    let body = serde_json::to_value(&act.kind).expect("an act serializes");
    strings(&body, &mut |s| {
        let id = ActId::from_raw(s);
        if by_id.contains_key(&id) {
            named.insert(id);
        }
    });
    act.refs = named
        .into_iter()
        .filter_map(|id| by_id.get(&id).map(|a| (id.clone(), digest(a))))
        .collect();
    canonical(act)
}

fn strings(v: &serde_json::Value, f: &mut impl FnMut(&str)) {
    match v {
        serde_json::Value::String(s) => f(s),
        serde_json::Value::Array(xs) => xs.iter().for_each(|x| strings(x, f)),
        serde_json::Value::Object(m) => m.values().for_each(|x| strings(x, f)),
        _ => {}
    }
}

// ── keys ────────────────────────────────────────────────────

/// A public key, as an OpenSSH `authorized_keys` line names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    blob: Vec<u8>,
    point: [u8; 32],
    /// The FIDO application (`ssh:` by default). `None` for a plain key.
    application: Option<Vec<u8>>,
}

impl Key {
    /// Read `<type> <base64> [comment]`. Only Ed25519 and FIDO Ed25519 are
    /// accepted: one curve means one verifier to get right, and both are
    /// what `ssh-keygen -t ed25519[-sk]` makes.
    pub fn parse(line: &str) -> Result<Self, String> {
        let mut words = line.split_whitespace();
        let kind = words.next().ok_or("an empty key")?;
        if kind != ED25519 && kind != SK_ED25519 {
            return Err(format!(
                "`{kind}` keys are not accepted — use `ssh-keygen -t ed25519-sk` (a key you touch) or `-t ed25519`"
            ));
        }
        let blob = base64_decode(words.next().ok_or("a key with no body")?)
            .ok_or("a key body that is not base64")?;
        let mut r = Reader(&blob);
        if r.string()? != kind.as_bytes() {
            return Err("a key whose body names a different type".into());
        }
        let point: [u8; 32] = r
            .string()?
            .try_into()
            .map_err(|_| "an Ed25519 key that is not 32 bytes")?;
        let application = if kind == SK_ED25519 {
            Some(r.string()?.to_vec())
        } else {
            None
        };
        r.end()?;
        Ok(Self {
            blob,
            point,
            application,
        })
    }

    /// `SHA256:…`, the fingerprint `ssh-keygen -l` prints, so a person can
    /// compare what canon says with what their own tools say.
    pub fn fingerprint(&self) -> String {
        format!(
            "SHA256:{}",
            base64_encode(&Sha256::digest(&self.blob)).trim_end_matches('=')
        )
    }

    /// Does a signature need somebody's touch? True for a FIDO key, whose
    /// signatures carry a presence flag every reader checks.
    pub fn touch_required(&self) -> bool {
        self.application.is_some()
    }

    /// The key as `<type> <base64>`, comment dropped: what a key act records.
    pub fn openssh(&self) -> String {
        let kind = if self.touch_required() {
            SK_ED25519
        } else {
            ED25519
        };
        format!("{kind} {}", base64_encode(&self.blob))
    }

    fn kind(&self) -> &'static str {
        if self.touch_required() {
            SK_ED25519
        } else {
            ED25519
        }
    }
}

/// Does `sig` (an SSHSIG blob, armoured or bare base64) sign `message` under
/// `key` in canon's namespace?
pub fn verifies(key: &Key, message: &[u8], sig: &str) -> bool {
    check(key, NAMESPACE, message, sig).is_ok()
}

fn check(key: &Key, namespace: &str, message: &[u8], sig: &str) -> Result<(), String> {
    let body: String = sig
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .flat_map(|l| l.chars())
        .filter(|c| !c.is_whitespace())
        .collect();
    let blob = base64_decode(&body).ok_or("a signature that is not base64")?;
    let mut r = Reader(&blob);
    if r.take(MAGIC.len())? != MAGIC || r.u32()? != 1 {
        return Err("not an SSHSIG v1 signature".into());
    }
    if r.string()? != key.blob.as_slice() {
        return Err("signed by a different key".into());
    }
    if r.string()? != namespace.as_bytes() {
        return Err("signed for a different namespace".into());
    }
    let reserved = r.string()?;
    let hash_alg = r.string()?;
    let hashed: Vec<u8> = match hash_alg {
        b"sha512" => Sha512::digest(message).to_vec(),
        b"sha256" => Sha256::digest(message).to_vec(),
        _ => return Err("an unknown SSHSIG hash".into()),
    };
    let mut signed = MAGIC.to_vec();
    for field in [namespace.as_bytes(), reserved, hash_alg, &hashed] {
        put_string(&mut signed, field);
    }

    let mut s = Reader(r.string()?);
    r.end()?;
    if s.string()? != key.kind().as_bytes() {
        return Err("a signature of a different key type".into());
    }
    let raw = ed25519_compact::Signature::from_slice(s.string()?)
        .map_err(|_| "a malformed Ed25519 signature")?;
    let point = ed25519_compact::PublicKey::from_slice(&key.point)
        .map_err(|_| "a malformed Ed25519 key")?;
    let over = match &key.application {
        None => signed,
        // PROTOCOL.u2f: the authenticator signs the application's hash, its
        // flags and counter, and the hash of the message.
        Some(application) => {
            let flags = s.byte()?;
            let counter = s.u32()?;
            if flags & USER_PRESENT == 0 {
                return Err("a FIDO signature made without a touch".into());
            }
            let mut over = Sha256::digest(application).to_vec();
            over.push(flags);
            over.extend_from_slice(&counter.to_be_bytes());
            over.extend_from_slice(&Sha256::digest(&signed));
            over
        }
    };
    s.end()?;
    point
        .verify(over, &raw)
        .map_err(|_| "a signature that does not verify".to_string())
}

// ── bindings ────────────────────────────────────────────────

/// One actor's bound keys, as the log establishes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    pub actor: String,
    /// The keys that sign for this name. Empty when contested.
    pub keys: Vec<Key>,
    /// The key acts that bind this name. More than one is a contest.
    pub acts: Vec<ActId>,
    /// Two key acts bind this name: somebody else holds a key that signs
    /// for it, and nothing signed under it counts.
    pub contested: bool,
}

/// What [`verify`] found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Verified {
    /// Every name a key act binds, by name.
    pub bindings: BTreeMap<String, Binding>,
    /// Acts under a bound name that do not count, with why. Carried and not
    /// applied: kept in the log, never given to the fold.
    pub unverified: Vec<(ActId, String)>,
}

/// Which acts count, given the key acts in `acts`.
///
/// A log with no key act returns at once and costs nothing, which is every
/// canon written before signing existed.
pub fn verify(acts: &[Act]) -> Verified {
    let mut out = Verified::default();
    if !acts.iter().any(|a| matches!(a.kind, ActKind::Key { .. })) {
        return out;
    }
    let digests: BTreeMap<&ActId, String> = acts.iter().map(|a| (&a.id, digest(a))).collect();
    let signed_by = |act: &Act, keys: &[Key]| -> Result<(), String> {
        let sig = act.sig.as_deref().ok_or("not signed")?;
        let mut present = false;
        for (id, want) in &act.refs {
            if let Some(have) = digests.get(id) {
                if have != want {
                    return Err(format!(
                        "{id}, which it names, has changed since it was signed"
                    ));
                }
                present = true;
            }
        }
        // A key act need not name anything present: it may found the canon,
        // with nothing yet to name. Pasted into another canon it carries
        // only its owner's own key, so it can stop their history counting
        // there — what deleting their lines would do — and never lets
        // anyone sign as them. Anything else signed must name this record.
        if !present && !matches!(act.kind, ActKind::Key { .. }) {
            return Err("signed against a different record".into());
        }
        let message = canonical(act);
        if keys.iter().any(|k| verifies(k, message.as_bytes(), sig)) {
            Ok(())
        } else {
            Err("not signed by a key bound to this name".into())
        }
    };

    let mut by_actor: BTreeMap<&str, Vec<&Act>> = BTreeMap::new();
    for a in acts {
        by_actor.entry(a.actor.as_str()).or_default().push(a);
    }

    for (actor, mine) in by_actor {
        // A key act binds when it signs with one of its own keys.
        let mut binding: Vec<(&Act, Vec<Key>, &[String])> = Vec::new();
        let mut broken: Vec<(ActId, String)> = Vec::new();
        for a in &mine {
            let ActKind::Key { keys, prior } = &a.kind else {
                continue;
            };
            let parsed = keys
                .iter()
                .map(|k| Key::parse(k))
                .collect::<Result<Vec<_>, _>>()
                .and_then(|ks| {
                    if ks.is_empty() {
                        Err("it names no key".into())
                    } else {
                        Ok(ks)
                    }
                });
            match parsed.and_then(|ks| signed_by(a, &ks).map(|()| ks)) {
                Ok(ks) => binding.push((a, ks, prior)),
                Err(e) => broken.push((a.id.clone(), format!("a key act that does not bind: {e}"))),
            }
        }
        out.unverified.extend(broken);
        if binding.is_empty() {
            // Nothing binds this name, so nothing about it changes.
            continue;
        }

        // Two bindings mean somebody else holds a key for this name. Neither
        // wins — "the earlier" would be decided by a timestamp the loser can
        // write — and only what every one of them vouches for still counts.
        let contested = binding.len() > 1;
        let mut vouched: BTreeSet<&str> = binding[0].2.iter().map(String::as_str).collect();
        for (_, _, prior) in &binding[1..] {
            let theirs: BTreeSet<&str> = prior.iter().map(String::as_str).collect();
            vouched.retain(|d| theirs.contains(d));
        }
        let contest = || format!("{actor} is contested: two key acts bind it");
        for a in &mine {
            let why = if matches!(a.kind, ActKind::Key { .. }) {
                if !contested || !binding.iter().any(|(b, _, _)| b.id == a.id) {
                    continue; // the binding itself, or already listed as broken
                }
                contest()
            } else if vouched.contains(digests[&a.id].as_str()) {
                continue;
            } else if contested {
                contest()
            } else {
                match signed_by(a, &binding[0].1) {
                    Ok(()) => continue,
                    Err(_) if a.sig.is_none() => format!("not signed, and {actor} has bound a key"),
                    Err(e) => e,
                }
            };
            out.unverified.push((a.id.clone(), why));
        }
        out.bindings.insert(
            actor.to_string(),
            Binding {
                actor: actor.to_string(),
                keys: if contested {
                    Vec::new()
                } else {
                    binding[0].1.clone()
                },
                acts: binding.iter().map(|(a, _, _)| a.id.clone()).collect(),
                contested,
            },
        );
    }
    out.unverified.sort();
    out
}

// ── wire format and base64 ──────────────────────────────────

struct Reader<'a>(&'a [u8]);

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.0.len() < n {
            return Err("a truncated SSH field".into());
        }
        let (head, rest) = self.0.split_at(n);
        self.0 = rest;
        Ok(head)
    }
    fn byte(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn string(&mut self) -> Result<&'a [u8], String> {
        let n = self.u32()? as usize;
        self.take(n)
    }
    fn end(&self) -> Result<(), String> {
        if self.0.is_empty() {
            Ok(())
        } else {
            Err("trailing bytes in an SSH field".into())
        }
    }
}

fn put_string(out: &mut Vec<u8>, field: &[u8]) {
    out.extend_from_slice(&(field.len() as u32).to_be_bytes());
    out.extend_from_slice(field);
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 with padding, as OpenSSH writes it.
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (u32::from(*b) << (16 - 8 * i)));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Standard base64; padding optional. `None` for anything else.
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let text = text.trim_end_matches('=');
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    let (mut acc, mut bits) = (0u32, 0u32);
    for c in text.bytes() {
        let v = ALPHABET.iter().position(|a| *a == c)? as u32;
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    // Leftover bits must be zero padding, and a lone sixth of a byte is not
    // base64 at all.
    (bits < 6 && acc == 0).then_some(out)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
pub(crate) mod tests;
