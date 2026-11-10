// SPDX-License-Identifier: AGPL-3.0-or-later
//! Generated checks for docs/CONTRACT.md. Inputs shrink with the failing case;
//! proptest records regression seeds and accepts PROPTEST_RNG_SEED for replay.
//! These are distributional checks, not proofs or held-out measurements.

use canon_core::act::{KNOWN_ANNOTATIONS, STRUCTURAL};
use canon_core::allot::{Allocation, Order};
use canon_core::standing::Pull;
use canon_core::{
    sign, Act, ActId, ActKind, Attributes, Authority, Bar, Between, Canon, Governs, Log, Outcome,
    Policy, Position, Proposal, Ratify, Rule, Scope, Standing, Status, Verdict, ANONYMOUS,
};
use canon_readers::{Draws, Pools, Reviews};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::TestCaseError;
use sha2::Digest;

/// References always point to earlier generated acts. Equal timestamps, agent
/// proposals, stale horizons, and references to replaced rules are intentional.
fn histories() -> impl Strategy<Value = Vec<Act>> {
    proptest::collection::vec((0u8..8, any::<usize>(), 0i64..3, any::<bool>()), 1..25).prop_map(
        |steps| {
            let mut acts: Vec<Act> = Vec::new();
            let mut rules: Vec<canon_core::ActId> = Vec::new();
            let mut at = 100;
            for (i, (op, target, tick, human)) in steps.into_iter().enumerate() {
                at += tick;
                let actor = if human { "human:sam" } else { "agent:reader" };
                let kind = if rules.is_empty() || op == 0 {
                    ActKind::Assert {
                        text: format!("agreement {i}"),
                        from: None,
                        source: None,
                    }
                } else {
                    let rule = rules[target % rules.len()].clone();
                    match op {
                        1 => ActKind::Supersede {
                            text: format!("replacement {i}"),
                            old: vec![rule],
                            rationale: "changed circumstances".into(),
                        },
                        2 => ActKind::Retract {
                            target: rule,
                            rationale: "no longer needed".into(),
                        },
                        3 => ActKind::Revert {
                            targets: vec![acts[target % acts.len()].id.clone()],
                            rationale: "correct the record".into(),
                        },
                        4 => ActKind::Question {
                            text: format!("question {i}"),
                            proposal: None,
                        },
                        5 => ActKind::Silence {
                            about: format!("subject {i}"),
                            rationale: "leave room for judgement".into(),
                        },
                        6 => ActKind::Horizon {
                            target: rule,
                            at: at + tick,
                            rationale: "review it".into(),
                        },
                        _ => ActKind::Position {
                            about: rule.as_str().into(),
                            citing: None,
                            pull: if human { Pull::Toward } else { Pull::Against },
                            because: "a stated reason".into(),
                        },
                    }
                };
                let is_rule = matches!(kind, ActKind::Assert { .. } | ActKind::Supersede { .. });
                let act = Act::new(kind, at, actor);
                if is_rule {
                    rules.push(act.id.clone());
                }
                acts.push(act);
            }
            acts
        },
    )
}

/// The areas the wide histories grant, scope and govern: a house, a room
/// inside it, somewhere outside it, and a corner named for a person.
const AREAS: [&str; 4] = ["house", "house.kitchen", "yard", "human:sam.desk"];
/// Who writes them: three people, an agent, and a line left without a name.
const ACTORS: [&str; 5] = [
    "human:sam",
    "human:dana",
    "human:kit",
    "agent:reader",
    ANONYMOUS,
];
/// How a proposal becomes a rule, in the spelling the CLI reads. Parsed, so
/// never a `twice` of a `twice`: see `a_twice_of_twice_rule_refuses_what_is_put_to_it`.
const RATIFY: [&str; 10] = [
    "standing",
    "joint:human:dana,human:sam",
    "threshold:2/1",
    "majority",
    "unanimous",
    "share:60%",
    "share:>2/3",
    "consent:0d",
    "twice:turnover:consent:1d",
    "twice:1d:threshold:1/1",
];

/// One of the areas. The house comes up half the time, so that it gathers
/// enough holders for its gates, draws and pools to be decided by them.
fn area(n: usize) -> Scope {
    Scope::new(AREAS[[0, 1, 0, 2, 0, 3][n % 6]]).unwrap()
}

/// The decision policies a wide history adopts and `replay_deterministic`
/// decides under, as a record writes them. The last carries the v4 knobs.
fn policy_rules() -> Vec<Rule> {
    let ladder = r#""ladder":["act","ask-one","refuse"],"base":{"rule":"default"}"#;
    let written = [
        r#"{"rule":"default"}"#.to_string(),
        r#"{"rule":"consent"}"#.to_string(),
        r#"{"rule":"threshold","against":2}"#.to_string(),
        r#"{"rule":"subsidiarity"}"#.to_string(),
        r#"{"rule":"supermajority","numerator":2,"denominator":3}"#.to_string(),
        format!(r#"{{"rule":"graduated",{ladder}}}"#),
        r#"{"rule":"entrenched","protected":["principle"],"base":{"rule":"consent"}}"#.to_string(),
        r#"{"rule":"cautious","base":{"rule":"threshold","against":1}}"#.to_string(),
        format!(r#"{{"rule":"graduated",{ladder},"key":"cited","repair":true}}"#),
    ];
    let read = |line: &String| serde_json::from_str(line).unwrap();
    written.iter().map(read).collect()
}

/// A key held by a test, signing to PROTOCOL.sshsig the way
/// `ssh-keygen -Y sign` does, so a generated `key` act really binds.
struct Signer(ed25519_compact::KeyPair);

fn put(out: &mut Vec<u8>, field: &[u8]) {
    out.extend_from_slice(&(field.len() as u32).to_be_bytes());
    out.extend_from_slice(field);
}

impl Signer {
    fn of(actor: usize) -> Self {
        let seed = ed25519_compact::Seed::new([actor as u8 + 1; 32]);
        Self(ed25519_compact::KeyPair::from_seed(seed))
    }

    fn blob(&self) -> Vec<u8> {
        let mut blob = Vec::new();
        put(&mut blob, b"ssh-ed25519");
        put(&mut blob, self.0.pk.as_ref());
        blob
    }

    fn public(&self) -> String {
        format!("ssh-ed25519 {} test", sign::base64_encode(&self.blob()))
    }

    /// Sign `act` against `log`, as the CLI does before it appends.
    fn seal(&self, mut act: Act, log: &[Act]) -> Act {
        let message = sign::prepare(&mut act, log);
        let hashed = sha2::Sha512::digest(message.as_bytes());
        let namespace = sign::NAMESPACE.as_bytes();
        let mut signed = b"SSHSIG".to_vec();
        for field in [namespace, b"", b"sha512", &hashed[..]] {
            put(&mut signed, field);
        }
        let mut sig = Vec::new();
        put(&mut sig, b"ssh-ed25519");
        put(&mut sig, self.0.sk.sign(&signed, None).as_ref());
        let mut blob = b"SSHSIG".to_vec();
        blob.extend_from_slice(&1u32.to_be_bytes());
        for field in [&self.blob()[..], namespace, b"", b"sha512", &sig[..]] {
            put(&mut blob, field);
        }
        act.sig = Some(sign::base64_encode(&blob));
        act
    }
}

/// The fields of the envelope every line carries beside its body.
const ENVELOPE: [&str; 7] = ["id", "v", "ts_unix", "actor", "refs", "sig", "op"];

/// THE ONE EXCLUSION the wide histories make, and what it steps around.
///
/// `ActKind::Annotation` can be built in memory with any op name and any
/// body, but it is written as `"op": kind` with its body's fields beside the
/// envelope's. So one whose `kind` is an op this build reads, or whose body
/// has a field named like the envelope's, does not come back from its own
/// rendering as the act that was written: the line is refused, or read as a
/// different act. That breaks `replay_deterministic`, and
/// `an_annotation_under_a_reserved_name_does_not_replay` pins how. Such a
/// name is given an `x-` here; every other name is left as generated.
fn writable_annotation(kind: String, field: String) -> (String, String) {
    let ops = [&STRUCTURAL[..], &KNOWN_ANNOTATIONS[..]].concat();
    let clear = |name: String, taken: &[&str]| match taken.contains(&name.as_str()) {
        true => format!("x-{name}"),
        false => name,
    };
    (clear(kind, &ops), clear(field, &ENVELOPE))
}

/// An earlier act from `pool`, or now and then an id no act has: a truncated
/// record is still a record.
fn pick(pool: &[ActId], n: usize) -> ActId {
    if pool.is_empty() || n.is_multiple_of(6) {
        ActId::from_raw(format!("can-{:012x}", n % 3))
    } else {
        pool[n % pool.len()].clone()
    }
}

/// Histories over every act kind: five actors, grants and withdrawals over
/// nested areas, real ratification and policy rules, draws, pools, bound and
/// contested keys with signed and unsigned acts under them, and annotations
/// this build does not know. A few seconds hold all of it, and now and then
/// an act is dated back to the first of them.
fn wide_histories() -> impl Strategy<Value = Vec<Act>> {
    let step = (
        0u8..40,
        0usize..1000,
        0usize..1000,
        0i64..2,
        0usize..ACTORS.len(),
        0u8..8,
        "[a-z]{1,8}(_[a-z]{1,6})?",
    );
    proptest::collection::vec(step, 1..40).prop_map(|steps| {
        let mut acts: Vec<Act> = Vec::new();
        let (mut rules, mut draws) = (Vec::new(), Vec::<(ActId, i64)>::new());
        let mut seals: Vec<(ActId, i64, usize)> = Vec::new();
        let mut granted: Vec<usize> = Vec::new();
        let mut bound = [false; ACTORS.len()];
        let mut at = 100;
        for (i, (op, x, y, tick, mut who, dial, word)) in steps.into_iter().enumerate() {
            at += tick;
            let mut ts = if dial == 7 { 100 } else { at };
            // Now and then the writer is somebody already granted an area
            // (and whoever seals a secret usually is), so the gates are
            // passed as well as refused.
            let insider = matches!(dial, 2 | 4) || (matches!(op, 21 | 32 | 33) && x % 3 != 0);
            if !granted.is_empty() && insider {
                who = granted[x / 7 % granted.len()];
            }
            // A reveal is usually by somebody who sealed, for the draw they
            // sealed for; otherwise by whoever the step names.
            let sealed = seals.get(x % seals.len().max(1));
            let sealed = sealed.filter(|_| x % 6 != 0 && matches!(op, 22 | 34 | 35));
            who = sealed.map_or(who, |s| s.2);
            let actor = ACTORS[who];
            let ids: Vec<ActId> = acts.iter().map(|a| a.id.clone()).collect();
            let scope = area(x);
            // A draw announced earlier and its boundary, or one nobody announced.
            let known = draws.get(x % draws.len().max(1)).filter(|_| x % 6 != 0);
            let (draw, boundary) = known.cloned().unwrap_or((pick(&[], x), ts));
            let holder = ACTORS[y % ACTORS.len()].to_string();
            let text = format!("agreement {i}");
            let rationale = if y % 5 == 0 { "" } else { "a stated reason" }.to_string();
            // What a position or a decision is about: a rule, or a free key.
            let about = |n: usize| match n {
                0 => "proposal".to_string(),
                _ => pick(&rules, x).as_str().to_string(),
            };
            let kind = match op {
                1 => ActKind::Supersede {
                    text,
                    old: (0..=y % 2).map(|k| pick(&rules, x + k)).collect(),
                    rationale,
                },
                2 => ActKind::Retract {
                    target: pick(&rules, x),
                    rationale,
                },
                3 => ActKind::Revert {
                    targets: vec![pick(&ids, x)],
                    rationale,
                },
                4 => ActKind::Question {
                    text,
                    proposal: (y % 2 == 0).then(|| "may we?".into()),
                },
                5 => ActKind::Silence {
                    about: format!("subject {}", x % 3),
                    rationale: "leave room for judgement".into(),
                },
                6 => ActKind::Horizon {
                    target: pick(&ids, x),
                    at: ts + (y % 5) as i64 - 1,
                    rationale,
                },
                7 | 30 => ActKind::Position {
                    about: about(y % 4),
                    citing: (y % 3 == 0).then(|| pick(&rules, y)),
                    pull: [Pull::Toward, Pull::Against][dial as usize % 2],
                    because: rationale,
                },
                8 => ActKind::Accept {
                    a: pick(&rules, x),
                    b: pick(&rules, y),
                    rationale: "protects both".into(),
                    revisit: [None, Some("2026-12-31"), Some("soon")][y % 3].map(String::from),
                },
                9 => ActKind::Dismiss {
                    a: pick(&rules, x),
                    b: pick(&rules, y),
                    rationale,
                },
                10 => ActKind::Adopt {
                    lineage: "commons".into(),
                    generation: format!("g{}", x % 2),
                    source: (y % 2 == 0).then(|| "https://example.org".into()),
                },
                11 | 25..=27 | 37 | 38 => ActKind::Grant {
                    holder,
                    scope,
                    horizon: (dial % 3 == 0).then(|| ts + (y % 4) as i64),
                    rationale,
                },
                12 => ActKind::Withdraw {
                    holder,
                    scope,
                    rationale,
                },
                13 | 31 => ActKind::Scoped {
                    commitment: pick(&rules, y),
                    scope,
                },
                14 | 39 => ActKind::Policy {
                    text,
                    rule: policy_rules().swap_remove(y % 9),
                    scope: (dial % 2 == 0).then_some(scope),
                },
                15 => ActKind::Ratification {
                    text,
                    rule: Ratify::parse(RATIFY[y % RATIFY.len()]).unwrap(),
                    scope: (dial % 4 != 0).then_some(scope),
                    governs: [None, Some(Governs::Commitments), Some(Governs::All)][x % 3],
                    personal: [None, Some(true), Some(false)][x / 3 % 3],
                },
                16 => ActKind::Allot {
                    text,
                    unit: "slot".into(),
                    units: (0..y % 3).map(|k| format!("slot {k}")).collect(),
                    scope,
                },
                17 => ActKind::Allocation {
                    text,
                    rule: Allocation::Rotation {
                        order: match y % 3 {
                            0 => Order::Holders,
                            1 => Order::Given {
                                actors: vec![holder, actor.into()],
                            },
                            _ => Order::FromDraw { commit: draw },
                        },
                        step: (x % 3) as i64 - 1,
                        per: [0, 1, 86_400][dial as usize % 3],
                    },
                    scope,
                },
                18 => ActKind::Decided {
                    about: about(y % 2),
                    outcome: [Outcome::Supported, Outcome::Conflicts, Outcome::Unaddressed][x % 3],
                    authority: [Authority::Act, Authority::AskPanel, Authority::Refuse][y % 3],
                    rationale,
                    citing: (0..dial as usize % 3)
                        .map(|k| pick(&rules, x + k))
                        .collect(),
                },
                19 => ActKind::Rank {
                    commitment: pick(&rules, x),
                    rank: ["principle", "convention"][y % 2].into(),
                },
                20 | 36 => ActKind::DrawCommit {
                    scope: area(x * (y % 2)),
                    count: [1, 1, 2, 0][y % 4],
                    after_ts: ts + [3, 2, 4, 3, 0, 2, -1, 3][dial as usize],
                    rationale,
                },
                // Sealed before the boundary and opened after it, or (odd dial)
                // whenever the step falls. The digest is of the secret the
                // same actor reveals, or now and then of another: a reveal
                // that does not match is a record too.
                21 | 32 | 33 => {
                    ts = [ts.min(boundary - 1), ts][dial as usize % 2];
                    seals.push((draw.clone(), boundary, who));
                    let secret = format!("secret {}", who + usize::from(y % 4 == 0));
                    ActKind::DrawSecret {
                        commit: draw,
                        digest: canon_core::id::digest_hex(secret.as_bytes()),
                    }
                }
                22 | 34 | 35 => {
                    let (draw, boundary) = sealed.map_or((draw, boundary), |s| (s.0.clone(), s.1));
                    ts = [ts.max(boundary), ts][dial as usize % 2];
                    ActKind::DrawReveal {
                        commit: draw,
                        secret: format!("secret {who}"),
                    }
                }
                // Bound by a real key, vouching for the actor's history or
                // for none of it; a second one contests the name; and an RSA
                // key (or, below, no signature) binds nothing.
                23 => ActKind::Key {
                    keys: vec![match y % 5 {
                        4 => "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQ== x".into(),
                        _ => Signer::of(who).public(),
                    }],
                    prior: acts
                        .iter()
                        .filter(|a| a.actor == actor && y % 2 == 0)
                        .map(sign::digest)
                        .collect(),
                },
                // An op and a field of any name, now and then one this build
                // reads; `writable_annotation` says which names are spared.
                24 => {
                    let named = |taken: &[&str], n: usize| match n % 4 {
                        0 => taken[x % taken.len()].to_string(),
                        _ => word.clone(),
                    };
                    let (kind, field) =
                        writable_annotation(named(&KNOWN_ANNOTATIONS, y), named(&ENVELOPE, x));
                    let body = serde_json::json!({
                        "note": text,
                        "weight": x % 5,
                        field: { "ids": [pick(&ids, y)], "open": dial % 2 == 0 },
                    });
                    ActKind::Annotation {
                        kind,
                        body: body.as_object().unwrap().clone(),
                    }
                }
                _ => ActKind::Assert {
                    text,
                    from: (y % 5 == 0).then(|| pick(&ids, x)),
                    source: (y % 4 == 0).then(|| "notes.md".into()),
                },
            };
            let mut act = Act::new(kind, ts, actor);
            // An actor who has written a key act signs from then on, the key
            // act included, except now and then (carried, not applied).
            bound[who] |= matches!(act.kind, ActKind::Key { .. });
            if bound[who] && dial != 1 {
                act = Signer::of(who).seal(act, &acts);
            }
            match act.kind {
                ActKind::Assert { .. } | ActKind::Supersede { .. } | ActKind::Question { .. } => {
                    rules.push(act.id.clone());
                }
                ActKind::DrawCommit { after_ts, .. } => draws.push((act.id.clone(), after_ts)),
                ActKind::Grant { .. } => granted.push(y % ACTORS.len()),
                _ => {}
            }
            acts.push(act);
        }
        acts
    })
}

/// The moment a wide history is read at: within its own seconds, around a
/// day later (a consent window, a `twice` gap), or long after.
fn wide_now(clock: i64) -> i64 {
    100 + clock * [1, 900, 4000][clock as usize % 3]
}

/// What a caller asks of a state beyond its fields: each commitment's
/// verdict, each draw, each area's pool, and what is overdue.
fn readings(state: &Canon, now: i64) -> String {
    let all = state.commitments().iter();
    let verdicts: Vec<_> = all.map(|c| state.ratify(c, now)).collect();
    let draws: Vec<_> = state.draws().iter().map(|d| state.draw(&d.act)).collect();
    let pools: Vec<_> = (0..6).map(|a| state.pool_at(&area(a), now)).collect();
    format!("{verdicts:?} {draws:?} {pools:?} {:?}", state.overdue(now))
}

type Law = Result<(), TestCaseError>;

fn replays(acts: Vec<Act>, now: i64, pulls: u64) -> Law {
    let log = Log::from_acts(acts);
    let restored = Log::parse(&log.render()).unwrap();
    let state = log.derive_at(now);
    let again = restored.derive_at(now);
    prop_assert_eq!(&state, &again);
    prop_assert_eq!(log.render(), restored.render());
    prop_assert_eq!(readings(&state, now), readings(&again, now));

    let evidence: Vec<_> = state
        .commitments()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let pull = [Pull::Toward, Pull::Against][(pulls.rotate_right(i as u32) & 1) as usize];
            Position::of(c.id().clone(), pull, "generated evidence")
        })
        .collect();
    let (standing, refused) = Standing::cited(&state, "proposal", evidence);
    prop_assert!(refused.is_empty());
    let plain = Attributes {
        now,
        ..Attributes::about("proposal")
    };
    let mut placed = plain
        .clone()
        .in_scope(area(1))
        .by(ACTORS[0])
        .reversible(false);
    placed.amends = state.commitments().first().map(|c| c.id().clone());
    for policy in policy_rules() {
        for attrs in [&plain, &placed] {
            prop_assert_eq!(
                policy.decide(&standing, attrs, &state),
                policy.decide(&standing, attrs, &again)
            );
        }
    }
    Ok(())
}

fn reorders(acts: Vec<Act>, keys: &[u64], now: i64) -> Law {
    let expected = Log::from_acts(acts.clone());
    let mut shuffled: Vec<_> = acts.into_iter().enumerate().collect();
    shuffled.sort_by_key(|(i, _)| (keys[i % keys.len()], *i));
    let actual = Log::from_acts(shuffled.into_iter().map(|(_, act)| act).collect());
    prop_assert_eq!(expected.derive_at(now), actual.derive_at(now));
    prop_assert_eq!(expected.render(), actual.render());
    Ok(())
}

fn merges(acts: Vec<Act>, other: Vec<Act>, membership: &[bool], now: i64) -> Law {
    // Every act reaches at least one branch. Some reach both; one branch
    // also receives exact duplicates. A third history exercises associativity.
    let on_left = |i: usize| membership[i % membership.len()] || i.is_multiple_of(3);
    let on_right = |i: usize| !membership[i % membership.len()] || i.is_multiple_of(3);
    let branch = |keep: &dyn Fn(usize) -> bool, copies: usize| {
        let kept = acts.iter().enumerate().filter(|(i, _)| keep(*i));
        Log::from_acts(kept.flat_map(|(_, a)| vec![a.clone(); copies]).collect())
    };
    let (left, right) = (branch(&on_left, 2), branch(&on_right, 1));
    let third = Log::from_acts(other);
    let expected = Log::from_acts(acts.clone());
    let merged = union(&left, &right);
    prop_assert_eq!(merged.render(), expected.render());
    prop_assert_eq!(merged.derive_at(now), expected.derive_at(now));
    prop_assert_eq!(merged.render(), union(&right, &left).render());
    prop_assert_eq!(merged.render(), union(&merged, &merged).render());
    let first = union(&union(&left, &right), &third);
    let second = union(&left, &union(&right, &third));
    prop_assert_eq!(first.render(), second.render());
    prop_assert_eq!(first.derive_at(now), second.derive_at(now));
    Ok(())
}

#[test]
fn wide_histories_reach_every_act_kind_and_the_gates() {
    // The wide generator is only worth its laws if it gets there: all 25 act
    // kinds, keys that really bind, and the branches a grant opens.
    let mut runner = proptest::test_runner::TestRunner::deterministic();
    let mut kinds = std::collections::HashSet::new();
    let mut reached = [0usize; 12];
    for _ in 0..400 {
        let tree = wide_histories().new_tree(&mut runner).unwrap();
        let log = Log::from_acts(tree.current());
        kinds.extend(log.acts().iter().map(|a| std::mem::discriminant(&a.kind)));
        let now = log.acts().last().unwrap().ts_unix + 2;
        let state = log.derive_at(now);
        let status =
            |want: fn(&Status) -> bool| state.commitments().iter().any(|c| want(c.status()));
        let keys = sign::verify(log.acts()).bindings;
        let checks = [
            !keys.is_empty(),
            keys.values().any(|b| b.contested),
            !state.unverified().is_empty(),
            status(|s| matches!(s, Status::Proposed { .. })),
            status(|s| matches!(s, Status::Refused { .. })),
            !state.ungoverned().is_empty(),
            !state.pending_policies().is_empty(),
            !state.retractions().is_empty(),
            !state.dangling().is_empty(),
            !state.carried().is_empty(),
            state.draws().iter().any(|d| state.draw(&d.act).is_ok()),
            (0..6).any(|a| state.pool_at(&area(a), now).is_ok()),
        ];
        for (n, hit) in reached.iter_mut().zip(checks) {
            *n += usize::from(hit);
        }
    }
    assert_eq!(kinds.len(), 25, "an act kind is never generated");
    assert!(
        reached.iter().all(|n| *n > 0),
        "a branch is never reached: {reached:?}"
    );
}

#[test]
fn an_annotation_under_a_reserved_name_does_not_replay() {
    // A KNOWN GAP in `replay_deterministic`, pinned as it behaves today. It
    // is not intended behaviour. `ActKind`'s `Serialize` writes an
    // annotation's `kind` as its op and its body beside the envelope without
    // refusing a name either already has, so these one-act histories, each
    // built through the public API, do not replay from their own rendering.
    // The wide histories step around them in `writable_annotation`.
    let carried = |kind: &str, body: serde_json::Value| {
        let body = body.as_object().unwrap().clone();
        let kind = kind.to_string();
        Log::from_acts(vec![Act::new(
            ActKind::Annotation { kind, body },
            100,
            "human:sam",
        )])
    };
    let replay = |log: &Log| Log::parse(&log.render());
    let json = |text: &str| serde_json::from_str::<serde_json::Value>(text).unwrap();

    // Named for an op this build reads, with a body that op cannot have:
    // the line it writes is refused.
    assert!(replay(&carried("grant", json("{}"))).is_err());
    // With a body that op can have: read back as that op and applied, where
    // the act that was written was carried and applied nothing.
    let rank = carried(
        "rank",
        json(r#"{"commitment":"can-000000000001","rank":"principle"}"#),
    );
    let back = replay(&rank).unwrap();
    assert!(matches!(back.acts()[0].kind, ActKind::Rank { .. }));
    assert_eq!(rank.derive_at(100).carried().len(), 1);
    assert!(back.derive_at(100).carried().is_empty());
    assert_ne!(rank.derive_at(100), back.derive_at(100));
    // A body field named like the envelope's is written twice and refused...
    assert!(replay(&carried("vote", json(r#"{"actor":"human:dana"}"#))).is_err());
    // ...or, where the envelope's own is absent, read back as the envelope's.
    let signed = carried("vote", json(r#"{"sig":"abc"}"#));
    let back = replay(&signed).unwrap();
    assert_eq!(back.acts()[0].sig.as_deref(), Some("abc"));
    assert_ne!(back.acts(), signed.acts());
}

#[test]
fn a_twice_of_twice_rule_refuses_what_is_put_to_it() {
    // `Ratify::parse` refuses a `twice` of a `twice`, but the enum and a
    // written line both allow one. The fold used to panic on the first
    // proposal judged under it; it now refuses the proposal and says why.
    // The wide histories only adopt rules `Ratify::parse` reads.
    let twice = |each| Ratify::Twice {
        each: Box::new(each),
        between: Between::Turnover,
    };
    let rule = Act::new(
        ActKind::Ratification {
            text: String::new(),
            rule: twice(twice(Ratify::Standing)),
            scope: None,
            governs: None,
            personal: None,
        },
        100,
        "human:sam",
    );
    let proposal = ActKind::Assert {
        text: "a proposed agreement".into(),
        from: None,
        source: None,
    };
    let written = Log::from_acts(vec![rule, Act::new(proposal, 101, "human:sam")]).render();
    let canon = Log::parse(&written).unwrap().derive_at(200);
    let put = canon.commitments().first().expect("the proposal");
    let Verdict::Refused { why, .. } = canon.ratify(put, 200) else {
        panic!("refused, with the reason")
    };
    assert!(why.contains("twice over twice"), "{why}");
}

#[test]
fn generated_laws_are_indexed_by_the_contract() {
    // Function references make a stale/renamed check a compile error; the row
    // check makes removing its guarantee from the index an explicit change.
    let checks: [(&str, fn()); 5] = [
        ("replay_deterministic", replay_deterministic),
        ("arrival_order_independent", arrival_order_independent),
        ("merge_union_laws", merge_union_laws),
        ("agents_cannot_ratify", agents_cannot_ratify),
        ("share_bars_are_their_rules", share_bars_are_their_rules),
    ];
    let contract = include_str!("../../../docs/CONTRACT.md");
    for (name, _) in checks {
        assert!(
            contract.contains(&format!("| `{name}` |")),
            "generated law {name} is missing from docs/CONTRACT.md"
        );
    }
}

/// The moments a house's people come, go and speak, around a proposal written
/// at 100: before it, in its second, after it, and either side of the day a
/// `twice` waits.
const SHARE_TIMES: [i64; 9] = [10, 100, 101, 150, 200, 86_600, 86_700, 90_000, 172_900];

/// A house of up to eight people around one proposal at 100. Each is granted
/// `house` (now and then `garden`, another top-level scope) at a generated
/// moment, some with a term that lapses, some moving out; holders, a
/// stranger and an agent take positions for and against, with and without a
/// reason, often in the same second. The proposer is a holder or the agent,
/// and the proposal is put in `house` or left for the whole canon.
fn share_histories() -> impl Strategy<Value = (Vec<Act>, Act, bool)> {
    let person = (
        0usize..9,
        proptest::option::of(0usize..9),
        proptest::option::of(0usize..9),
        0u8..10,
    );
    let said = (0usize..10, 0usize..9, any::<bool>(), any::<bool>());
    (
        proptest::collection::vec(person, 1..9),
        proptest::collection::vec(said, 0..16),
        0usize..9,
        any::<bool>(),
    )
        .prop_map(|(people, positions, proposer, scoped)| {
            let house = Scope::new("house").unwrap();
            let garden = Scope::new("garden").unwrap();
            let name = |i: usize| format!("human:p{i}");
            let mut acts = Vec::new();
            for (i, (granted, lapses, leaves, seat)) in people.iter().enumerate() {
                let scope = if *seat == 0 { &garden } else { &house };
                acts.push(Act::new(
                    ActKind::Grant {
                        holder: name(i),
                        scope: scope.clone(),
                        horizon: lapses.map(|t| SHARE_TIMES[t]),
                        rationale: String::new(),
                    },
                    SHARE_TIMES[*granted],
                    "human:founder",
                ));
                if let Some(t) = leaves {
                    acts.push(Act::new(
                        ActKind::Withdraw {
                            holder: name(i),
                            scope: scope.clone(),
                            rationale: "moving out".into(),
                        },
                        SHARE_TIMES[*t],
                        name(i),
                    ));
                }
            }
            let speaker = |i: usize| match i {
                i if i < people.len() => name(i),
                8 => "agent:reader".into(),
                _ => "human:stranger".into(),
            };
            let writer = speaker(if proposer < people.len() { proposer } else { 8 });
            let proposal = Act::new(
                ActKind::Assert {
                    text: "a proposed agreement".into(),
                    from: None,
                    source: None,
                },
                100,
                &writer,
            );
            if scoped {
                acts.push(Act::new(
                    ActKind::Scoped {
                        commitment: proposal.id.clone(),
                        scope: house,
                    },
                    100,
                    &writer,
                ));
            }
            for (who, at, toward, reasoned) in positions {
                acts.push(Act::new(
                    ActKind::Position {
                        about: proposal.id.as_str().into(),
                        citing: None,
                        pull: if toward { Pull::Toward } else { Pull::Against },
                        because: if reasoned {
                            format!("{} has a reason", speaker(who))
                        } else {
                            String::new()
                        },
                    },
                    SHARE_TIMES[at],
                    speaker(who),
                ));
            }
            acts.push(proposal.clone());
            (acts, proposal, scoped)
        })
}

fn union(a: &Log, b: &Log) -> Log {
    Log::from_acts(a.acts().iter().chain(b.acts()).cloned().collect())
}

fn grant(holder: &str, scope: &Scope) -> Act {
    Act::new(
        ActKind::Grant {
            holder: holder.into(),
            scope: scope.clone(),
            horizon: None,
            rationale: "founding seats".into(),
        },
        100,
        "human:founder",
    )
}

fn vote(id: &canon_core::ActId, actor: &str, at: i64, pull: Pull) -> Act {
    Act::new(
        ActKind::Position {
            about: id.as_str().into(),
            citing: None,
            pull,
            because: "a reason for this position".into(),
        },
        at,
        actor,
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, .. ProptestConfig::default() })]

    #[test]
    fn replay_deterministic(
        acts in histories(), wide in wide_histories(),
        clock in 0i64..200, pulls in any::<u64>(),
    ) {
        replays(acts, 100 + clock, pulls)?;
        replays(wide, wide_now(clock), pulls)?;
    }

    #[test]
    fn arrival_order_independent(
        acts in histories(), wide in wide_histories(),
        keys in proptest::collection::vec(any::<u64>(), 1..40),
        clock in 0i64..200,
    ) {
        reorders(acts, &keys, 100 + clock)?;
        reorders(wide, &keys, wide_now(clock))?;
    }

    #[test]
    fn merge_union_laws(
        acts in histories(), other in histories(),
        wide in wide_histories(), wide_other in wide_histories(),
        membership in proptest::collection::vec(any::<bool>(), 1..40),
        clock in 0i64..200,
    ) {
        merges(acts, other, &membership, 100 + clock)?;
        merges(wide, wide_other, &membership, wide_now(clock))?;
    }

    #[test]
    fn agents_cannot_ratify(
        suffix in "[a-z]{1,8}",
        ballots in proptest::collection::vec(any::<bool>(), 1..9),
        days in 1u32..4,
    ) {
        let scope = Scope::new("house.kitchen").unwrap();
        let proposer = format!("agent:{suffix}");
        let act = Act::new(ActKind::Assert {
            text: "a proposed agreement".into(), from: None, source: None,
        }, 200, &proposer);
        let proposal = Proposal { id: &act.id, scope: Some(&scope), at: 200, actor: &proposer };
        let now = 200 + i64::from(days) * 86_400 + 20;

        for held in [false, true] {
            let mut base = vec![act.clone(), Act::new(ActKind::Scoped {
                commitment: act.id.clone(), scope: scope.clone(),
            }, 200, &proposer)];
            if held {
                base.extend([grant("human:dana", &scope), grant("human:sam", &scope),
                             grant(&proposer, &scope)]);
            }
            let mut noisy = base.clone();
            for (i, toward) in ballots.iter().enumerate() {
                let actor = format!("agent:{suffix}-{i}");
                if held { noisy.push(grant(&actor, &scope)); }
                noisy.push(vote(&act.id, &actor, 201 + i as i64,
                    if *toward { Pull::Toward } else { Pull::Against }));
            }
            // Self-approval must not count either, even if the agent holds a seat.
            noisy.push(vote(&act.id, &proposer, 210, Pull::Toward));
            let quiet = Log::from_acts(base.clone()).derive_at(now);
            let loud = Log::from_acts(noisy.clone()).derive_at(now);
            prop_assert!(matches!(loud.get(&act.id).unwrap().status(), Status::Proposed { .. }),
                         "an agent's write remains proposed under standing");
            let mut rules = vec![Ratify::Standing];
            if held {
                rules.extend([
                    Ratify::Joint { holders: vec!["human:dana".into(), "human:sam".into()] },
                    Ratify::Threshold { approve: 2, block: 1 },
                    Ratify::Majority,
                    Ratify::Unanimous,
                    Ratify::Share(Bar::new(3, 5, false).unwrap()),
                    Ratify::Consent { days },
                    Ratify::Twice { each: Box::new(Ratify::Threshold { approve: 1, block: 1 }),
                                    between: Between::Days { days } },
                ]);
            }
            for rule in rules {
                prop_assert_eq!(quiet.ratify_under(&rule, &proposal, now),
                                loud.ratify_under(&rule, &proposal, now));
                let mut human = noisy.clone();
                if matches!(rule, Ratify::Consent { .. }) {
                    // Consent can carry by time alone: that is the group's rule,
                    // not an agent's approval. A timely human objection blocks it.
                    prop_assert!(matches!(loud.ratify_under(&rule, &proposal, now), Verdict::Ratified { .. }),
                                 "consent carries after its waiting period");
                    human.push(vote(&act.id, "human:dana", 201, Pull::Against));
                    prop_assert!(matches!(Log::from_acts(human).derive_at(now)
                        .ratify_under(&rule, &proposal, now), Verdict::Refused { .. }),
                        "a timely human objection refuses consent");
                } else {
                    prop_assert!(matches!(loud.ratify_under(&rule, &proposal, now), Verdict::Proposed { .. }),
                                 "agents cannot supply the missing approvals");
                    human.extend([vote(&act.id, "human:dana", 201, Pull::Toward),
                                  vote(&act.id, "human:sam", 202, Pull::Toward),
                                  vote(&act.id, "human:dana", now - 1, Pull::Toward)]);
                    prop_assert!(matches!(Log::from_acts(human).derive_at(now)
                        .ratify_under(&rule, &proposal, now), Verdict::Ratified { .. }),
                        "human holders can still carry the proposal");
                }
            }
        }
    }

    #[test]
    fn share_bars_are_their_rules(history in share_histories()) {
        // `majority` is the share more than half and `unanimous` the share
        // of everyone. Held as a share — as `Bar` builds it, or as a record
        // writes it another way — each is judged as the rule it equals:
        // the same verdict, moment and words, alone and under twice, and
        // the same fold for a house that adopts either.
        let (acts, proposal, scoped) = history;
        let now = 200_000;
        let house = Scope::new("house").unwrap();
        let canon = Log::from_acts(acts.clone()).derive_at(now);
        let p = Proposal {
            id: &proposal.id,
            scope: scoped.then_some(&house),
            at: 100,
            actor: &proposal.actor,
        };
        let written = |line: &str| serde_json::from_str::<Ratify>(line).unwrap();
        let pairs = [
            (Ratify::Majority, Ratify::Share(Bar::MAJORITY)),
            (Ratify::Majority, written(r#"{"rule":"share","numerator":2,"denominator":4,"more_than":true}"#)),
            (Ratify::Unanimous, Ratify::Share(Bar::EVERYONE)),
            (Ratify::Unanimous, written(r#"{"rule":"share","numerator":3,"denominator":3,"more_than":false}"#)),
        ];
        let adopted = |rule: Ratify| {
            let mut acts = acts.clone();
            acts.push(grant("human:founder", &house));
            acts.push(Act::new(ActKind::Ratification {
                text: String::new(), rule, scope: None, governs: None, personal: None,
            }, 50, "human:founder"));
            let canon = Log::from_acts(acts).derive_at(now);
            let c = canon.get(&proposal.id).unwrap();
            (c.status().clone(), canon.ratify(c, now))
        };
        for (named, share) in pairs {
            prop_assert_eq!(named.name(), share.name());
            for between in [None, Some(Between::Days { days: 1 }), Some(Between::Turnover)] {
                let under = |rule: &Ratify| match &between {
                    None => rule.clone(),
                    Some(b) => Ratify::Twice { each: Box::new(rule.clone()), between: b.clone() },
                };
                prop_assert_eq!(canon.ratify_under(&under(&named), &p, now),
                                canon.ratify_under(&under(&share), &p, now));
            }
            prop_assert_eq!(adopted(named), adopted(share));
        }
    }
}
