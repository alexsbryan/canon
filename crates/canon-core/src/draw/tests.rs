// SPDX-License-Identifier: AGPL-3.0-or-later
//! One test per row of the threat model in `PRIMITIVES.md` under Primitive 9.
//!
//! Each guard in `Canon::draw` was removed, one at a time, and the test named
//! in that table watched to fail. A gate nobody has seen fail is not a gate
//! (§18.1), and for a lottery that is not a stylistic point: a draw that looks
//! fair and is not launders a chosen panel as a drawn one.

use super::*;
use crate::act::{Act, ActKind};
use crate::log::Log;

const BOUNDARY: i64 = 1_000;

fn house() -> Scope {
    Scope::new("house").unwrap()
}

fn grant(actor: &str, at: i64) -> Act {
    Act::new(
        ActKind::Grant {
            holder: actor.into(),
            scope: house(),
            horizon: None,
            rationale: String::new(),
        },
        at,
        "human:alex",
    )
}

fn announce(count: usize, after_ts: i64, at: i64) -> Act {
    Act::new(
        ActKind::DrawCommit {
            scope: house(),
            count,
            after_ts,
            rationale: "kitchen panel".into(),
        },
        at,
        "human:alex",
    )
}

fn seal(commit: &ActId, actor: &str, secret: &str, at: i64) -> Act {
    Act::new(
        ActKind::DrawSecret {
            commit: commit.clone(),
            digest: crate::id::digest_hex(secret.as_bytes()),
        },
        at,
        actor,
    )
}

fn open(commit: &ActId, actor: &str, secret: &str, at: i64) -> Act {
    Act::new(
        ActKind::DrawReveal {
            commit: commit.clone(),
            secret: secret.into(),
        },
        at,
        actor,
    )
}

/// Twelve householders, a three-seat panel, everyone plays straight.
fn straight() -> (Vec<Act>, ActId) {
    let people: Vec<String> = (0..12).map(|i| format!("human:p{i:02}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(3, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("secret-{i}"), 300 + i as i64));
        acts.push(open(
            &id,
            p,
            &format!("secret-{i}"),
            BOUNDARY + 1 + i as i64,
        ));
    }
    (acts, id)
}

#[test]
fn a_draw_selects_from_the_pool_and_shows_its_working() {
    let (acts, id) = straight();
    let drawn = Log::from_acts(acts).derive().draw(&id).expect("draws");
    assert_eq!(drawn.seats.len(), 3);
    assert_eq!(drawn.pool.len(), 12);
    assert_eq!(drawn.contributed.len(), 12);
    assert!(drawn.withheld.is_empty());
    assert_eq!(drawn.seed.len(), 64, "checkable by hand");
    for seat in &drawn.seats {
        assert!(drawn.pool.contains(seat));
    }
    let mut unique = drawn.seats.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), 3, "nobody gets two seats");
}

// ── (a) grinding the boundary ───────────────────────────────

#[test]
fn a_boundary_in_the_past_is_refused() {
    // The drawer picks a moment whose consequences they can already see.
    // There is nothing to grind toward if the boundary must postdate its own
    // announcement, and everything to grind toward if it need not.
    let mut acts: Vec<Act> = (0..5).map(|i| grant(&format!("human:p{i}"), 100)).collect();
    let commit = announce(2, 300, 400);
    let id = commit.id.clone();
    acts.push(commit);
    let err = Log::from_acts(acts).derive().draw(&id).unwrap_err();
    assert!(
        matches!(err, DrawError::BoundaryNotInFuture { .. }),
        "{err:?}"
    );
}

#[test]
fn a_boundary_exactly_at_the_announcement_is_refused_too() {
    // The off-by-one that would reopen (a): "at the same second" is a moment
    // the drawer was standing in.
    let mut acts: Vec<Act> = (0..5).map(|i| grant(&format!("human:p{i}"), 100)).collect();
    let commit = announce(2, 400, 400);
    let id = commit.id.clone();
    acts.push(commit);
    assert!(matches!(
        Log::from_acts(acts).derive().draw(&id).unwrap_err(),
        DrawError::BoundaryNotInFuture { .. }
    ));
}

// ── (b) the drawer seeds their own draw ─────────────────────

#[test]
fn the_drawer_has_no_move_after_committing() {
    // There is no seed ACT, so there is nothing for the drawer to author.
    // Asserted the only way it can be: the drawer writes as many acts as they
    // like after the boundary, and the panel does not move.
    let (acts, id) = straight();
    let before = Log::from_acts(acts.clone()).derive().draw(&id).unwrap();
    let mut noisy = acts;
    for i in 0..50 {
        noisy.push(Act::new(
            ActKind::Assert {
                text: format!("drawer noise {i}"),
                from: None,
                source: None,
            },
            BOUNDARY + 1,
            "human:alex",
        ));
    }
    let after = Log::from_acts(noisy).derive().draw(&id).unwrap();
    assert_eq!(before.seats, after.seats, "the drawer moved the panel");
    assert_eq!(before.seed, after.seed);
}

// ── (c) pool churn ──────────────────────────────────────────

#[test]
fn standing_granted_after_the_boundary_does_not_join_the_pool() {
    let (mut acts, id) = straight();
    acts.push(grant("human:latecomer", BOUNDARY + 500));
    let drawn = Log::from_acts(acts).derive().draw(&id).unwrap();
    assert_eq!(drawn.pool.len(), 12);
    assert!(!drawn.pool.contains(&"human:latecomer".to_string()));
}

#[test]
fn withdrawing_after_the_boundary_does_not_shrink_a_frozen_pool() {
    // Pool churn from the other direction, and the one that nearly got
    // through: the fold used to DELETE a withdrawn grant, so somebody
    // stepping back today would silently remove themselves from a pool that
    // was frozen last week — changing everyone else's odds after the fact.
    // Grants are closed with a date now, not deleted.
    let (mut acts, id) = straight();
    let before = Log::from_acts(acts.clone()).derive().draw(&id).unwrap();
    acts.push(Act::new(
        ActKind::Withdraw {
            holder: "human:p05".into(),
            scope: house(),
            rationale: "moving out".into(),
        },
        BOUNDARY + 900,
        "human:p05",
    ));
    let after = Log::from_acts(acts).derive().draw(&id).unwrap();
    assert_eq!(before.pool, after.pool, "the frozen pool moved");
    assert_eq!(before.seats, after.seats);
}

#[test]
fn standing_that_lapsed_before_the_boundary_is_not_in_the_pool() {
    let (mut acts, id) = straight();
    acts.push(Act::new(
        ActKind::Grant {
            holder: "human:leaving".into(),
            scope: house(),
            horizon: Some(BOUNDARY - 1),
            rationale: String::new(),
        },
        100,
        "human:alex",
    ));
    let drawn = Log::from_acts(acts).derive().draw(&id).unwrap();
    assert!(!drawn.pool.contains(&"human:leaving".to_string()));
}

// ── (d) the empty window ────────────────────────────────────

#[test]
fn a_draw_with_nothing_revealed_refuses_rather_than_falling_back() {
    // The failure that matters most. A default seed here would be a panel
    // somebody could compute in advance and present as drawn.
    let people: Vec<String> = (0..6).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(2, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("s{i}"), 300));
    }
    let err = Log::from_acts(acts).derive().draw(&id).unwrap_err();
    assert!(matches!(err, DrawError::NothingRevealed), "{err:?}");
}

#[test]
fn a_scope_nobody_holds_has_no_pool_and_refuses() {
    let commit = announce(2, BOUNDARY, 200);
    let id = commit.id.clone();
    let err = Log::from_acts(vec![commit]).derive().draw(&id).unwrap_err();
    assert!(matches!(err, DrawError::EmptyPool { .. }), "{err:?}");
}

// ── (e) grinding the secret ─────────────────────────────────

#[test]
fn a_revealed_secret_that_does_not_match_its_digest_is_refused() {
    // The whole point of the seal. Without the check, everyone simply picks
    // their secret after seeing the others'.
    let people: Vec<String> = (0..6).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(2, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("s{i}"), 300));
        // p0 opens something else entirely.
        let opened = if i == 0 {
            "a-much-better-secret".to_string()
        } else {
            format!("s{i}")
        };
        acts.push(open(&id, p, &opened, BOUNDARY + 1));
    }
    let drawn = Log::from_acts(acts).derive().draw(&id).unwrap();
    assert_eq!(drawn.withheld, vec!["human:p0".to_string()]);
    assert!(!drawn.contributed.contains(&"human:p0".to_string()));
    assert!(
        !drawn.pool.contains(&"human:p0".to_string()),
        "and it costs them their seat"
    );
}

#[test]
fn a_secret_opened_before_the_boundary_does_not_count() {
    // Opening early hands the secret to anyone who has not sealed yet, which
    // is the same attack from the other end.
    let people: Vec<String> = (0..6).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(2, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("s{i}"), 300));
        let when = if i == 0 { 400 } else { BOUNDARY + 1 };
        acts.push(open(&id, p, &format!("s{i}"), when));
    }
    let drawn = Log::from_acts(acts).derive().draw(&id).unwrap();
    assert_eq!(drawn.withheld, vec!["human:p0".to_string()]);
}

#[test]
fn a_secret_sealed_after_the_boundary_is_not_a_commitment() {
    let people: Vec<String> = (0..6).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(2, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        let sealed_at = if i == 0 { BOUNDARY + 1 } else { 300 };
        acts.push(seal(&id, p, &format!("s{i}"), sealed_at));
        acts.push(open(&id, p, &format!("s{i}"), BOUNDARY + 2));
    }
    let drawn = Log::from_acts(acts).derive().draw(&id).unwrap();
    assert!(!drawn.contributed.contains(&"human:p0".to_string()));
    // Not withheld either — they never validly sealed, so there is nothing
    // to have withheld, and they keep their seat in the pool.
    assert!(drawn.withheld.is_empty());
    assert!(drawn.pool.contains(&"human:p0".to_string()));
}

// ── (f) several digests, open the flattering one ────────────

#[test]
fn a_second_secret_from_the_same_actor_is_ignored() {
    // Sealing twice and opening whichever helps is grinding with extra steps.
    let people: Vec<String> = (0..6).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(2, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("s{i}"), 300 + i as i64));
        acts.push(open(&id, p, &format!("s{i}"), BOUNDARY + 1));
    }
    // p0 seals a SECOND digest later and opens that one instead.
    acts.push(seal(&id, "human:p0", "the-good-one", 500));
    let canon = Log::from_acts(acts).derive();
    assert_eq!(
        canon
            .sealed
            .iter()
            .filter(|s| s.actor == "human:p0")
            .count(),
        1,
        "the fold keeps the first seal only"
    );
    let drawn = canon.draw(&id).unwrap();
    assert!(
        drawn.contributed.contains(&"human:p0".to_string()),
        "their FIRST secret still counts"
    );
}

// ── (g) two replayers ───────────────────────────────────────

#[test]
fn two_replayers_draw_the_same_panel() {
    let (acts, id) = straight();
    let a = Log::from_acts(acts.clone()).derive().draw(&id).unwrap();
    // The same acts, arriving in the opposite order — which is what a merge
    // between two machines actually looks like.
    let mut reversed = acts;
    reversed.reverse();
    let b = Log::from_acts(reversed).derive().draw(&id).unwrap();
    assert_eq!(a, b);
}

#[test]
fn the_panel_moves_when_any_secret_moves() {
    // The other half of determinism: a draw that ignored its seed would also
    // be perfectly reproducible, and useless.
    let (acts, id) = straight();
    let base = Log::from_acts(acts.clone()).derive().draw(&id).unwrap();
    let mut changed: Vec<Act> = acts
        .into_iter()
        .filter(|a| {
            !matches!(
                &a.kind,
                ActKind::DrawSecret { .. } | ActKind::DrawReveal { .. }
            ) || a.actor != "human:p00"
        })
        .collect();
    changed.push(seal(&id, "human:p00", "different", 300));
    changed.push(open(&id, "human:p00", "different", BOUNDARY + 1));
    let moved = Log::from_acts(changed).derive().draw(&id).unwrap();
    assert_ne!(
        base.seed, moved.seed,
        "one secret changed and the seed did not"
    );
}

// ── (h) more seats than people ──────────────────────────────

#[test]
fn drawing_more_seats_than_the_pool_holds_refuses() {
    let people: Vec<String> = (0..3).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(5, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("s{i}"), 300));
        acts.push(open(&id, p, &format!("s{i}"), BOUNDARY + 1));
    }
    assert!(matches!(
        Log::from_acts(acts).derive().draw(&id).unwrap_err(),
        DrawError::PoolTooSmall { .. }
    ));
}

#[test]
fn drawing_exactly_the_pool_refuses_because_that_is_not_a_draw() {
    // Three seats from three people selects everyone and would present an
    // unselected group as a drawn one.
    let people: Vec<String> = (0..3).map(|i| format!("human:p{i}")).collect();
    let mut acts: Vec<Act> = people.iter().map(|p| grant(p, 100)).collect();
    let commit = announce(3, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    for (i, p) in people.iter().enumerate() {
        acts.push(seal(&id, p, &format!("s{i}"), 300));
        acts.push(open(&id, p, &format!("s{i}"), BOUNDARY + 1));
    }
    assert!(matches!(
        Log::from_acts(acts).derive().draw(&id).unwrap_err(),
        DrawError::PoolTooSmall { .. }
    ));
}

#[test]
fn a_draw_for_nobody_refuses() {
    let (mut acts, _) = straight();
    let commit = announce(0, BOUNDARY, 200);
    let id = commit.id.clone();
    acts.push(commit);
    assert!(matches!(
        Log::from_acts(acts).derive().draw(&id).unwrap_err(),
        DrawError::NoSeats
    ));
}

#[test]
fn a_draw_nobody_announced_refuses() {
    let (acts, _) = straight();
    let ghost = ActId::from_raw("can-000000000000");
    assert!(matches!(
        Log::from_acts(acts).derive().draw(&ghost).unwrap_err(),
        DrawError::NoSuchDraw { .. }
    ));
}

// ── the shuffle itself ──────────────────────────────────────

#[test]
fn the_shuffle_reaches_every_position_and_does_not_favour_one() {
    // Not a statistics suite — a smoke test that the Fisher-Yates is not
    // accidentally an identity or a rotation, which is what a biased draw
    // would look like from outside.
    let mut counts = std::collections::BTreeMap::new();
    for round in 0..400 {
        let mut items: Vec<String> = (0..8).map(|i| format!("p{i}")).collect();
        super::shuffle(
            &crate::id::digest_hex(format!("seed{round}").as_bytes()),
            &mut items,
        );
        *counts.entry(items[0].clone()).or_insert(0usize) += 1;
    }
    assert_eq!(counts.len(), 8, "someone can never come first: {counts:?}");
    for (who, n) in &counts {
        assert!(
            (15..=100).contains(n),
            "`{who}` came first {n} times in 400, which is not a shuffle"
        );
    }
}
