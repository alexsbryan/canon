// SPDX-License-Identifier: AGPL-3.0-or-later
//! Signed acts. Each attack the design names has a test here that writes it
//! and watches it not count.

use super::*;
use crate::{Log, Status};
use alloc::vec;

// ── signing, the way `ssh-keygen -Y sign` does it ───────────

/// A key held by a test. Real signatures, built to PROTOCOL.sshsig by hand
/// so the tests can sign whatever they write; the OpenSSH vectors below are
/// what keep this builder honest.
pub(crate) struct Signer {
    pair: ed25519_compact::KeyPair,
    /// `Some(flags)` for a FIDO key, signing with those flags.
    fido: Option<u8>,
}

impl Signer {
    pub(crate) fn new(seed: u8) -> Self {
        Self {
            pair: ed25519_compact::KeyPair::from_seed(ed25519_compact::Seed::new([seed; 32])),
            fido: None,
        }
    }

    fn fido(seed: u8, flags: u8) -> Self {
        Self {
            fido: Some(flags),
            ..Self::new(seed)
        }
    }

    fn blob(&self) -> Vec<u8> {
        let mut b = Vec::new();
        match self.fido {
            None => put_string(&mut b, ED25519.as_bytes()),
            Some(_) => put_string(&mut b, SK_ED25519.as_bytes()),
        }
        put_string(&mut b, self.pair.pk.as_ref());
        if self.fido.is_some() {
            put_string(&mut b, b"ssh:");
        }
        b
    }

    pub(crate) fn public(&self) -> String {
        let kind = if self.fido.is_some() {
            SK_ED25519
        } else {
            ED25519
        };
        format!("{kind} {} test", base64_encode(&self.blob()))
    }

    pub(crate) fn sign(&self, message: &[u8]) -> String {
        let mut signed = MAGIC.to_vec();
        for field in [
            NAMESPACE.as_bytes(),
            b"",
            b"sha512",
            &Sha512::digest(message),
        ] {
            put_string(&mut signed, field);
        }
        let mut sig = Vec::new();
        match self.fido {
            None => {
                put_string(&mut sig, ED25519.as_bytes());
                put_string(&mut sig, self.pair.sk.sign(&signed, None).as_ref());
            }
            Some(flags) => {
                let mut over = Sha256::digest(b"ssh:").to_vec();
                over.push(flags);
                over.extend_from_slice(&7u32.to_be_bytes());
                over.extend_from_slice(&Sha256::digest(&signed));
                put_string(&mut sig, SK_ED25519.as_bytes());
                put_string(&mut sig, self.pair.sk.sign(&over, None).as_ref());
                sig.push(flags);
                sig.extend_from_slice(&7u32.to_be_bytes());
            }
        }
        let mut blob = MAGIC.to_vec();
        blob.extend_from_slice(&1u32.to_be_bytes());
        for field in [&self.blob()[..], NAMESPACE.as_bytes(), b"", b"sha512", &sig] {
            put_string(&mut blob, field);
        }
        base64_encode(&blob)
    }

    /// Sign `act` against `log`, as the CLI does before it appends.
    pub(crate) fn seal(&self, mut act: Act, log: &[Act]) -> Act {
        let message = prepare(&mut act, log);
        act.sig = Some(self.sign(message.as_bytes()));
        act
    }
}

fn assert_by(text: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Assert {
            text: text.into(),
            from: None,
            source: None,
        },
        ts,
        actor,
    )
}

fn grant(holder: &str, scope: &str, ts: i64, actor: &str) -> Act {
    Act::new(
        ActKind::Grant {
            holder: holder.into(),
            scope: crate::Scope::new(scope).unwrap(),
            horizon: None,
            rationale: String::new(),
        },
        ts,
        actor,
    )
}

/// A first link for `actor`, vouching for everything they wrote in `log`.
pub(crate) fn bind(actor: &str, ts: i64, log: &[Act], keys: &[&Signer]) -> Act {
    let prior = log
        .iter()
        .filter(|a| a.actor == actor)
        .map(digest)
        .collect();
    let act = Act::new(
        ActKind::Key {
            keys: keys.iter().map(|k| k.public()).collect(),
            prior,
        },
        ts,
        actor,
    );
    keys[0].seal(act, log)
}

fn out(st: &crate::Canon) -> Vec<&ActId> {
    st.unverified.iter().map(|(id, _)| id).collect()
}

fn holds(st: &crate::Canon, actor: &str) -> bool {
    st.grants.iter().any(|g| g.actor == actor)
}

// ── OpenSSH's own vectors ───────────────────────────────────
//
// From openssh-portable regress/unittests/sshsig/testdata (BSD licence),
// namespace `unittest`. Both verify with `ssh-keygen -Y verify`; the FIDO
// one is a real authenticator's signature, which no test here could make.

const SIGNED_DATA: &[u8] = b"This is a test, this is only a test";
const ED25519_PUB: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIIlizSIN3DQV78VPNjVvvwjgf+P5HqbPYCYu3BOMdjAE ED25519 test key";
const ED25519_SIG: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAADMAAAALc3NoLWVkMjU1MTkAAAAgiWLNIg3cNBXvxU82NW+/COB/4/
keps9gJi7cE4x2MAQAAAAIdW5pdHRlc3QAAAAAAAAABnNoYTUxMgAAAFMAAAALc3NoLWVk
MjU1MTkAAABAihQsbUzuNEFflk5Tw1+H9aLS7tZQk0RG8KW1DtOmDYYnWe3D3UKiG3fcJa
DNg4vBWp1j1gLRiBMOF+gwYNegDg==
-----END SSH SIGNATURE-----";
const SK_PUB: &str = "sk-ssh-ed25519@openssh.com AAAAGnNrLXNzaC1lZDI1NTE5QG9wZW5zc2guY29tAAAAIJsaDYXQYruc6bilCYDIK4YSOeG+zmrRO2M9t03//7LHAAAABHNzaDo= ED25519-SK test key";
const SK_SIG: &str = "-----BEGIN SSH SIGNATURE-----
U1NIU0lHAAAAAQAAAEoAAAAac2stc3NoLWVkMjU1MTlAb3BlbnNzaC5jb20AAAAgmxoNhd
Biu5zpuKUJgMgrhhI54b7OatE7Yz23Tf//sscAAAAEc3NoOgAAAAh1bml0dGVzdAAAAAAA
AAAGc2hhNTEyAAAAZwAAABpzay1zc2gtZWQyNTUxOUBvcGVuc3NoLmNvbQAAAEAi+7eTjW
/+LQ2M+sCD+KFtH1n7VFFJon/SZFsxODyV8cWTlFKj617Ys1Ur5TV6uaEXQhck8rBA2oQI
HTPANLIPARI0Vng=
-----END SSH SIGNATURE-----";

#[test]
fn openssh_signatures_verify_and_nothing_near_them_does() {
    for (public, sig) in [(ED25519_PUB, ED25519_SIG), (SK_PUB, SK_SIG)] {
        let key = Key::parse(public).unwrap();
        assert_eq!(
            check(&key, "unittest", SIGNED_DATA, sig),
            Ok(()),
            "{public}"
        );
        assert!(check(
            &key,
            "unittest",
            b"This is a test, this is only a tesT",
            sig
        )
        .is_err());
        assert!(
            check(&key, "git", SIGNED_DATA, sig).is_err(),
            "another namespace"
        );
        assert!(
            !verifies(&key, SIGNED_DATA, sig),
            "canon's namespace is not `unittest`"
        );
    }
    // One key's signature is not another's.
    let other = Key::parse(SK_PUB).unwrap();
    assert!(check(&other, "unittest", SIGNED_DATA, ED25519_SIG).is_err());
}

#[test]
fn fingerprints_are_the_ones_ssh_keygen_prints() {
    assert_eq!(
        Key::parse(ED25519_PUB).unwrap().fingerprint(),
        "SHA256:ZxTZQ/K7JZpz0mHqWqdmYCUcq3MAtnu9TYA66TCBkxM"
    );
    let sk = Key::parse(SK_PUB).unwrap();
    assert_eq!(
        sk.fingerprint(),
        "SHA256:pbI9wlyrFSsUbp+GTiIAOqNMvoajbF2tHsPMECyrMYU"
    );
    assert!(sk.touch_required());
    assert_eq!(
        Key::parse(&sk.openssh()).unwrap(),
        sk,
        "the comment is dropped, nothing else"
    );
}

#[test]
fn only_ed25519_keys_bind() {
    let rsa = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQ== x";
    assert!(Key::parse(rsa).unwrap_err().contains("ed25519"));
    assert!(Key::parse("ssh-ed25519 !!!").is_err());
    assert!(Key::parse("").is_err());
}

#[test]
fn base64_round_trips_and_refuses_what_is_not_base64() {
    for n in 0..40u8 {
        let bytes: Vec<u8> = (0..n).collect();
        assert_eq!(base64_decode(&base64_encode(&bytes)).unwrap(), bytes);
    }
    assert!(base64_decode("a").is_none(), "a lone sixth of a byte");
    assert!(base64_decode("ab!=").is_none());
}

#[test]
fn a_test_signature_is_a_real_one() {
    // The builder above, checked against the same verifier as OpenSSH's
    // vectors — and FIDO without a touch refused.
    for signer in [Signer::new(1), Signer::fido(2, USER_PRESENT)] {
        let key = Key::parse(&signer.public()).unwrap();
        assert!(verifies(&key, b"hello", &signer.sign(b"hello")));
        assert!(!verifies(&key, b"hullo", &signer.sign(b"hello")));
    }
    let untouched = Signer::fido(3, 0);
    let key = Key::parse(&untouched.public()).unwrap();
    assert!(!verifies(&key, b"hello", &untouched.sign(b"hello")));
}

// ── what is signed ──────────────────────────────────────────

#[test]
fn canonical_bytes_are_sorted_compact_and_without_the_signature() {
    let mut act = assert_by("tabs\tand \u{1}, \"quotes\", é and 🦀", 7, "human:alex");
    act.id = ActId::from_raw("can-000000000000");
    act.sig = Some("ignored".into());
    assert_eq!(
        canonical(&act),
        "{\"actor\":\"human:alex\",\"id\":\"can-000000000000\",\"op\":\"assert\",\
         \"text\":\"tabs\\tand \\u0001, \\\"quotes\\\", é and 🦀\",\"ts_unix\":7,\"v\":2}"
    );
}

#[test]
fn a_signed_act_survives_render_parse_and_a_merge() {
    let alex = Signer::new(1);
    let a = assert_by("quiet hours at 11", 100, "human:alex");
    let key = bind("human:alex", 200, core::slice::from_ref(&a), &[&alex]);
    let log = vec![a.clone(), key.clone()];
    let signed = alex.seal(assert_by("guests two nights", 300, "human:alex"), &log);
    assert_eq!(
        signed.v,
        crate::SIGNED_VERSION,
        "signing alone stays readable by v3"
    );
    assert_eq!(
        a.v,
        crate::UNSIGNED_VERSION,
        "an unsigned line stays readable by v2"
    );

    let one = Log::from_acts(vec![a, key, signed.clone()]);
    // Through text, the way the merge driver unions two sides.
    let again = Log::parse(&Log::from_acts(vec![signed]).render()).unwrap();
    let merged = Log::from_acts(one.acts().iter().chain(again.acts()).cloned().collect());
    let merged = Log::parse(&merged.render()).unwrap();
    assert_eq!(merged.render(), one.render());
    let st = merged.derive();
    assert!(st.unverified.is_empty(), "{:?}", st.unverified);
    assert_eq!(st.active().count(), 2);
}

#[test]
fn a_version_past_signing_is_still_refused() {
    let line = r#"{"id":"can-deadbeef0000","v":5,"ts_unix":1,"actor":"human:x","op":"assert","text":"hi"}"#;
    assert!(matches!(
        Log::parse(line),
        Err(crate::ParseError::UnknownVersion { found: 5, .. })
    ));
}

// ── what counts ─────────────────────────────────────────────

#[test]
fn a_canon_nobody_signs_in_folds_exactly_as_before() {
    let a = assert_by("quiet hours at 11", 100, "human:alex");
    let b = grant("human:alex", "house", 50, "human:alex");
    let st = Log::from_acts(vec![a, b]).derive();
    assert!(st.unverified.is_empty());
    assert_eq!(verify(&[]), Verified::default());
}

#[test]
fn the_env_var_no_longer_makes_you_alex() {
    // Alex founds the house, binds a key, and keeps working signed. Then
    // something that can append to the file writes `human:alex` on a grant
    // to itself and on a rule — `CANON_ACTOR=human:alex`, in effect.
    let alex = Signer::new(1);
    let founding = grant("human:alex", "house", 100, "human:alex");
    let key = bind(
        "human:alex",
        110,
        core::slice::from_ref(&founding),
        &[&alex],
    );
    let mut log = vec![founding, key];
    let rule = alex.seal(assert_by("quiet hours at 11", 120, "human:alex"), &log);
    log.push(rule.clone());

    let forged_grant = grant("agent:mallory", "house", 130, "human:alex");
    let forged_rule = assert_by("the agent may merge", 140, "human:alex");
    log.push(forged_grant.clone());
    log.push(forged_rule.clone());

    let st = Log::from_acts(log).derive();
    assert!(holds(&st, "human:alex"));
    assert!(
        !holds(&st, "agent:mallory"),
        "a forged grant is not applied"
    );
    assert!(st.get(&forged_rule.id).is_none(), "the fold never saw it");
    assert!(matches!(st.get(&rule.id).unwrap().status, Status::Active));
    let mut forged = vec![&forged_grant.id, &forged_rule.id];
    forged.sort();
    assert_eq!(out(&st), forged);
    assert!(st
        .unverified
        .iter()
        .all(|(_, why)| why.contains("not signed")));
}

#[test]
fn a_forgery_dated_before_the_binding_does_not_count_either() {
    // The attack that sank "a binding protects what comes after it".
    let alex = Signer::new(1);
    let history = assert_by("quiet hours at 11", 100, "human:alex");
    let key = bind("human:alex", 500, core::slice::from_ref(&history), &[&alex]);
    let backdated = assert_by("the agent may merge", 101, "human:alex");

    let st = Log::from_acts(vec![history.clone(), key, backdated.clone()]).derive();
    assert!(
        matches!(st.get(&history.id).unwrap().status, Status::Active),
        "vouched history counts"
    );
    assert_eq!(out(&st), vec![&backdated.id]);
}

#[test]
fn someone_else_signing_as_you_does_not_count() {
    let alex = Signer::new(1);
    let mallory = Signer::new(9);
    let a = assert_by("quiet hours at 11", 100, "human:alex");
    let key = bind("human:alex", 110, core::slice::from_ref(&a), &[&alex]);
    let log = vec![a, key];
    let forged = mallory.seal(assert_by("the agent may merge", 120, "human:alex"), &log);

    let st = Log::from_acts([log, vec![forged.clone()]].concat()).derive();
    assert_eq!(out(&st), vec![&forged.id]);
    assert!(st.unverified[0].1.contains("not signed by a key bound"));
}

#[test]
fn an_approval_cannot_be_moved_onto_other_text_behind_the_same_id() {
    // An agent proposes; alex approves, signed. The agent then swaps in a
    // different proposal under the same id — 48 bits, so findable. The
    // swap is written by hand here; the effect is the same.
    let alex = Signer::new(1);
    let founding = grant("human:alex", "house", 100, "human:alex");
    let key = bind(
        "human:alex",
        110,
        core::slice::from_ref(&founding),
        &[&alex],
    );
    let proposal = assert_by("quiet hours at 11", 120, "agent:euphemia");
    let mut log = vec![founding, key, proposal.clone()];
    let scoped = Act::new(
        ActKind::Scoped {
            commitment: proposal.id.clone(),
            scope: crate::Scope::new("house").unwrap(),
        },
        121,
        "agent:euphemia",
    );
    log.push(scoped);
    let approve = alex.seal(
        Act::new(
            ActKind::Position {
                about: proposal.id.as_str().into(),
                citing: None,
                pull: crate::Pull::Toward,
                because: String::new(),
            },
            130,
            "human:alex",
        ),
        &log,
    );
    assert!(
        approve.refs.contains_key(&proposal.id),
        "the referent is pinned"
    );
    log.push(approve.clone());
    let honest = Log::from_acts(log.clone()).derive();
    assert!(honest.unverified.is_empty(), "{:?}", honest.unverified);
    assert!(matches!(
        honest.get(&proposal.id).unwrap().status,
        Status::Active
    ));

    let mut swapped = assert_by("the agent may merge", 120, "agent:euphemia");
    swapped.id = proposal.id.clone();
    let log: Vec<Act> = log
        .into_iter()
        .map(|a| {
            if a.id == proposal.id {
                swapped.clone()
            } else {
                a
            }
        })
        .collect();
    let st = Log::from_acts(log).derive();
    assert_eq!(out(&st), vec![&approve.id]);
    assert!(st.unverified[0]
        .1
        .contains("has changed since it was signed"));
    assert!(!matches!(
        st.get(&proposal.id).unwrap().status,
        Status::Active
    ));
}

#[test]
fn a_signed_line_pasted_into_another_canon_does_not_verify_there() {
    let alex = Signer::new(1);
    let here = assert_by("quiet hours at 11", 100, "human:alex");
    let key = bind("human:alex", 110, core::slice::from_ref(&here), &[&alex]);
    let signed = alex.seal(
        assert_by("guests two nights", 120, "human:alex"),
        &[here.clone(), key.clone()],
    );

    // Elsewhere alex has bound the same key, honestly.
    let there = assert_by("bikes in the hall", 90, "human:alex");
    let key_there = bind("human:alex", 95, core::slice::from_ref(&there), &[&alex]);
    let st = Log::from_acts(vec![there, key_there, signed.clone()]).derive();
    assert_eq!(out(&st), vec![&signed.id]);
    assert!(st.unverified[0].1.contains("different record"));
}

#[test]
fn a_key_act_that_cannot_bind_changes_nothing_else() {
    let a = assert_by("quiet hours at 11", 100, "human:alex");
    let rsa = Act::new(
        ActKind::Key {
            keys: vec!["ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQ== x".into()],
            prior: vec![],
        },
        110,
        "human:alex",
    );
    let later = assert_by("guests two nights", 120, "human:alex");
    let st = Log::from_acts(vec![a, rsa.clone(), later.clone()]).derive();
    assert_eq!(out(&st), vec![&rsa.id]);
    assert!(
        matches!(st.get(&later.id).unwrap().status, Status::Active),
        "the name is not bound"
    );
}

#[test]
fn a_squat_on_an_unbound_name_is_visible_and_does_not_reach_anyone_else() {
    // Mallory binds `human:sam` before sam does, vouching for nothing.
    // Sam's history stops counting and the record says why; alex is
    // untouched. The defence is binding before your first act; this test
    // pins that the damage is loud and stays on the name.
    let mallory = Signer::new(9);
    let sam = assert_by("bikes in the hall", 100, "human:sam");
    let alex = assert_by("quiet hours at 11", 100, "human:alex");
    let squat = mallory.seal(
        Act::new(
            ActKind::Key {
                keys: vec![mallory.public()],
                prior: vec![],
            },
            110,
            "human:sam",
        ),
        &[sam.clone(), alex.clone()],
    );
    let st = Log::from_acts(vec![sam.clone(), alex.clone(), squat]).derive();
    assert_eq!(out(&st), vec![&sam.id]);
    assert!(matches!(st.get(&alex.id).unwrap().status, Status::Active));
}

#[test]
fn a_canon_can_be_founded_by_binding_a_key_first() {
    // `canon init`, then `canon identify`: the binding is the canon's first
    // act and has nothing else to name. Everything after it names it.
    let alex = Signer::new(1);
    let key = bind("human:alex", 100, &[], &[&alex]);
    assert!(key.refs.is_empty());
    let founding = alex.seal(
        grant("human:alex", "house", 110, "human:alex"),
        core::slice::from_ref(&key),
    );
    let st = Log::from_acts(vec![key.clone(), founding]).derive();
    assert!(st.unverified.is_empty(), "{:?}", st.unverified);
    assert!(holds(&st, "human:alex"));

    // Same second, and the grant's id sorts first: no ordering decides
    // whether a binding holds.
    let mut same = grant("human:alex", "house", 100, "human:alex");
    same = alex.seal(same, core::slice::from_ref(&key));
    let st = Log::from_acts(vec![key.clone(), same]).derive();
    assert!(st.unverified.is_empty(), "{:?}", st.unverified);
}

#[test]
fn a_second_binding_contests_the_name_instead_of_replacing_the_first() {
    // Alex binds; a thief who can write the file binds alex's name to their
    // own key, dated earlier to look like the original. Neither wins.
    let alex = Signer::new(1);
    let thief = Signer::new(9);
    let history = assert_by("quiet hours at 11", 100, "human:alex");
    let mine = bind("human:alex", 200, core::slice::from_ref(&history), &[&alex]);
    let theirs = bind(
        "human:alex",
        150,
        core::slice::from_ref(&history),
        &[&thief],
    );
    let log = vec![history.clone(), mine.clone(), theirs.clone()];
    let signed = alex.seal(assert_by("guests two nights", 300, "human:alex"), &log);
    let forged = thief.seal(assert_by("the agent may merge", 301, "human:alex"), &log);

    let all = [log, vec![signed.clone(), forged.clone()]].concat();
    let checked = verify(Log::from_acts(all.clone()).acts());
    let b = &checked.bindings["human:alex"];
    assert!(b.contested && b.keys.is_empty() && b.acts.len() == 2);
    let st = Log::from_acts(all).derive();
    assert!(
        matches!(st.get(&history.id).unwrap().status, Status::Active),
        "both vouch for it"
    );
    let mut lost = vec![&mine.id, &theirs.id, &signed.id, &forged.id];
    lost.sort();
    assert_eq!(out(&st), lost);
    assert!(st
        .unverified
        .iter()
        .all(|(_, why)| why.contains("contested")));
}

#[test]
fn a_backup_key_bound_alongside_signs_as_well() {
    let laptop = Signer::new(1);
    let backup = Signer::fido(2, USER_PRESENT);
    let history = assert_by("quiet hours at 11", 100, "human:alex");
    let key = bind(
        "human:alex",
        110,
        core::slice::from_ref(&history),
        &[&laptop, &backup],
    );
    let log = vec![history, key];
    let a = laptop.seal(assert_by("one", 120, "human:alex"), &log);
    let b = backup.seal(assert_by("two", 130, "human:alex"), &log);
    let st = Log::from_acts([log, vec![a, b]].concat()).derive();
    assert!(st.unverified.is_empty(), "{:?}", st.unverified);
    assert_eq!(st.active().count(), 3);
}

#[test]
fn a_binding_pasted_from_another_canon_never_lets_anyone_sign_as_you() {
    // The residual, pinned. Alex's binding from elsewhere, pasted in by
    // someone who can write this file, carries only alex's own key: alex's
    // history here stops counting — loudly — and nothing the paster signs
    // ever counts as alex.
    let alex = Signer::new(1);
    let mallory = Signer::new(9);
    let elsewhere = bind("human:alex", 10, &[], &[&alex]);
    let here = assert_by("quiet hours at 11", 100, "human:alex");
    let forged = mallory.seal(
        assert_by("the agent may merge", 110, "human:alex"),
        core::slice::from_ref(&here),
    );
    let st = Log::from_acts(vec![elsewhere, here.clone(), forged.clone()]).derive();
    let mut lost = vec![&here.id, &forged.id];
    lost.sort();
    assert_eq!(out(&st), lost);
}
