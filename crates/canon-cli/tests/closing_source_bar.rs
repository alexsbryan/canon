// SPDX-License-Identifier: AGPL-3.0-or-later
//! Which withdrawal closed a question, from what the reader already says.
//!
//! `Status::Retracted` keeps a time and nothing else, and the fold's own
//! account of which acts are live is private. So "who closed this, and why"
//! has to be joined from the outside: the record's `retract` acts against the
//! question's status, `retractions()`, `ungoverned()` and `unverified()`.
//!
//! This file is that join and the histories it is good for. It names a
//! source only when one candidate is left and the status confirms it. Two
//! candidates in the same second stay two candidates: the public output does
//! not say which the fold applied, and guessing by order would be inventing
//! an answer.

use canon_core::{Act, ActId, ActKind, Canon, Log, Pull, Scope, Status, Verdict};

/// What can be said about the withdrawal behind a question's status.
#[derive(Debug, PartialEq, Eq)]
enum Closing {
    /// The question is not withdrawn.
    NotClosed,
    /// One withdrawal fits, and the status confirms it: its actor and reason.
    Sole(String, String),
    /// More than one fits. Each is shown; none is named the one.
    Among(Vec<String>),
    /// Withdrawn, and no withdrawal in this record fits.
    Unknown,
}

fn closing(log: &Log, canon: &Canon, q: &ActId) -> Closing {
    let Some(Status::Retracted { at }) = canon.question(q).map(|q| &q.status) else {
        return Closing::NotClosed;
    };
    let set_aside = |id: &ActId| {
        canon.unverified().iter().any(|(x, _)| x == id)
            || canon.ungoverned().iter().any(|(x, _)| x == id)
    };
    // A withdrawal put to the area counts from its approval, not its writing.
    let put = |id: &ActId| canon.retractions().iter().find(|r| r.act == *id);
    let fits: Vec<&Act> = log
        .acts()
        .iter()
        .filter(|a| matches!(&a.kind, ActKind::Retract { target, .. } if target == q))
        .filter(|a| !set_aside(&a.id))
        .filter(|a| match put(&a.id).map(|r| &r.verdict) {
            Some(Verdict::Ratified { since, .. }) => since == at,
            Some(_) => false,
            None => a.ts_unix == *at,
        })
        .collect();
    match fits[..] {
        [] => Closing::Unknown,
        [a] => {
            let ActKind::Retract { rationale, .. } = &a.kind else {
                unreachable!()
            };
            Closing::Sole(a.actor.clone(), rationale.clone())
        }
        _ => Closing::Among(fits.iter().map(|a| a.actor.clone()).collect()),
    }
}

fn scope(s: &str) -> Scope {
    Scope::new(s).expect("a scope")
}

fn grant(holder: &str, s: &str, ts: i64) -> Act {
    let kind = ActKind::Grant {
        holder: holder.into(),
        scope: scope(s),
        horizon: None,
        rationale: String::new(),
    };
    Act::new(kind, ts, "human:sam")
}

fn ask(ts: i64, actor: &str) -> Act {
    let kind = ActKind::Question {
        text: "Why is the inlet label absent?".into(),
        proposal: None,
    };
    Act::new(kind, ts, actor)
}

fn scoped(q: &ActId, s: &str, ts: i64) -> Act {
    let kind = ActKind::Scoped {
        commitment: q.clone(),
        scope: scope(s),
    };
    Act::new(kind, ts, "human:sam")
}

fn close(q: &ActId, ts: i64, actor: &str, why: &str) -> Act {
    let kind = ActKind::Retract {
        target: q.clone(),
        rationale: why.into(),
    };
    Act::new(kind, ts, actor)
}

fn undo(target: &ActId, ts: i64, actor: &str) -> Act {
    let kind = ActKind::Revert {
        targets: vec![target.clone()],
        rationale: "resume".into(),
    };
    Act::new(kind, ts, actor)
}

fn approve(target: &ActId, ts: i64, actor: &str) -> Act {
    let kind = ActKind::Position {
        about: target.to_string(),
        citing: None,
        pull: Pull::Toward,
        because: "yes".into(),
    };
    Act::new(kind, ts, actor)
}

/// Sam and Theo are in charge of the canal; Ola asked, in the canal's area.
fn canal() -> (Vec<Act>, ActId) {
    let q = ask(100, "human:ola");
    let id = q.id.clone();
    let acts = vec![
        grant("human:sam", "canal", 10),
        grant("human:theo", "canal", 20),
        q,
        scoped(&id, "canal", 100),
    ];
    (acts, id)
}

fn read(acts: Vec<Act>, q: &ActId) -> Closing {
    let log = Log::from_acts(acts);
    closing(&log, &log.derive(), q)
}

fn sole(actor: &str, why: &str) -> Closing {
    Closing::Sole(actor.into(), why.into())
}

#[test]
fn one_withdrawal_is_named_whoever_had_the_right_to_write_it() {
    let (acts, q) = canal();
    assert_eq!(read(acts.clone(), &q), Closing::NotClosed);

    // The asker's own.
    let mut own = acts.clone();
    own.push(close(&q, 200, "human:ola", "answered at the meeting"));
    assert_eq!(read(own, &q), sole("human:ola", "answered at the meeting"));

    // Somebody in charge of the area, after the asker has gone.
    let mut held = acts.clone();
    held.push(close(&q, 200, "human:theo", "follow-up is with the DCO"));
    assert_eq!(
        read(held, &q),
        sole("human:theo", "follow-up is with the DCO")
    );

    // An area nobody is in charge of: open, and it takes.
    let asked = ask(100, "human:ola");
    let open = vec![asked.clone(), close(&asked.id, 200, "human:new", "stale")];
    assert_eq!(read(open, &asked.id), sole("human:new", "stale"));
}

#[test]
fn a_withdrawal_without_the_right_closes_nothing_until_it_is_approved() {
    let (mut acts, q) = canal();
    let asked = close(&q, 200, "human:new", "I think this is done");
    acts.push(asked.clone());
    assert_eq!(read(acts.clone(), &q), Closing::NotClosed);

    // Approved by somebody who could have written it: closed from then, and
    // the reason shown is the one the proposer gave.
    acts.push(approve(&asked.id, 300, "human:theo"));
    let log = Log::from_acts(acts);
    let canon = log.derive();
    assert_eq!(
        canon.question(&q).unwrap().status,
        Status::Retracted { at: 300 }
    );
    assert_eq!(
        closing(&log, &canon, &q),
        sole("human:new", "I think this is done")
    );
}

#[test]
fn an_anonymous_question_is_closed_by_standing_and_never_by_being_anonymous_too() {
    // `withdraw --ask` writes the question as `anonymous`. A second
    // anonymous writer shares the name and so reads as the asker: the
    // existing rule, and no proof of anything. It is pinned here so the
    // product says "anonymous", not "the asker".
    let q = ask(100, canon_core::ANONYMOUS);
    let mut acts = vec![
        grant("human:sam", "canal", 10),
        q.clone(),
        scoped(&q.id, "canal", 100),
    ];
    let mut by_name = acts.clone();
    by_name.push(close(&q.id, 200, canon_core::ANONYMOUS, "no longer needed"));
    assert_eq!(
        read(by_name, &q.id),
        sole(canon_core::ANONYMOUS, "no longer needed")
    );
    // A named replacement with no seat proposes; Sam, who has one, closes.
    let mut lacking = acts.clone();
    lacking.push(close(&q.id, 200, "human:new", "handing over"));
    assert_eq!(read(lacking, &q.id), Closing::NotClosed);
    acts.push(close(&q.id, 200, "human:sam", "follow-up is with the DCO"));
    assert_eq!(
        read(acts, &q.id),
        sole("human:sam", "follow-up is with the DCO")
    );
}

#[test]
fn an_undone_withdrawal_reopens_and_an_undone_undo_closes_again() {
    let (mut acts, q) = canal();
    let closed = close(&q, 200, "human:theo", "stop tracking");
    let undone = undo(&closed.id, 300, "human:theo");
    acts.extend([closed, undone.clone()]);
    assert_eq!(read(acts.clone(), &q), Closing::NotClosed);
    acts.push(undo(&undone.id, 400, "human:theo"));
    assert_eq!(read(acts, &q), sole("human:theo", "stop tracking"));
}

#[test]
fn of_two_withdrawals_the_latest_raw_reason_is_not_the_answer() {
    // Theo closes, Sam closes later, Sam's is undone. The newest `retract`
    // line in the record is Sam's; the withdrawal in force is Theo's.
    let (mut acts, q) = canal();
    let first = close(&q, 200, "human:theo", "stop tracking");
    let second = close(&q, 300, "human:sam", "duplicate");
    acts.extend([first, second.clone()]);
    assert_eq!(read(acts.clone(), &q), sole("human:sam", "duplicate"));
    acts.push(undo(&second.id, 400, "human:sam"));
    assert_eq!(read(acts.clone(), &q), sole("human:theo", "stop tracking"));

    // And an undo by somebody with no right to it changes nothing: the fold
    // refuses it, and the join never had to ask why.
    let (mut acts, q) = canal();
    let only = close(&q, 200, "human:theo", "stop tracking");
    acts.extend([only.clone(), undo(&only.id, 300, "human:stranger")]);
    assert_eq!(read(acts, &q), sole("human:theo", "stop tracking"));
}

#[test]
fn two_withdrawals_in_one_second_are_shown_as_two() {
    // Both applied, both dated the status's second. Nothing public says
    // which the fold wrote last, so neither is named.
    let (mut acts, q) = canal();
    let a = close(&q, 200, "human:theo", "stop tracking");
    let b = close(&q, 200, "human:sam", "duplicate");
    acts.extend([a.clone(), b.clone()]);
    let Closing::Among(who) = read(acts.clone(), &q) else {
        panic!("two candidates")
    };
    assert_eq!(who.len(), 2);

    // THE LIMIT. Undo one of the two in that second: the question is still
    // withdrawn, by the other, and the join cannot say so. The undone
    // `retract` is still a line in the record, still verified, still dated
    // the status's second, and whether the undo was admitted is the fold's
    // private answer. Both stay candidates.
    acts.push(undo(&b.id, 300, "human:sam"));
    let Closing::Among(who) = read(acts, &q) else {
        panic!("still two candidates, though one is undone")
    };
    assert_eq!(who.len(), 2);
}

#[test]
fn an_undone_withdrawal_in_another_second_is_a_wrong_candidate_never_a_wrong_answer() {
    // The same limit, from the other side. Theo's closure is undone; a
    // proposal by a newcomer is approved in the SAME second Theo wrote his.
    // Two acts fit the status's time and one of them is dead.
    let (mut acts, q) = canal();
    let dead = close(&q, 300, "human:theo", "stop tracking");
    let put = close(&q, 200, "human:new", "I think this is done");
    acts.extend([dead.clone(), put.clone()]);
    acts.push(undo(&dead.id, 350, "human:theo"));
    acts.push(approve(&put.id, 300, "human:sam"));
    let log = Log::from_acts(acts);
    let canon = log.derive();
    assert_eq!(
        canon.question(&q).unwrap().status,
        Status::Retracted { at: 300 }
    );
    // It over-reports candidates; it does not name the dead one.
    assert!(matches!(closing(&log, &canon, &q), Closing::Among(w) if w.len() == 2));
}
