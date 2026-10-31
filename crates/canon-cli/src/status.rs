// SPDX-License-Identifier: AGPL-3.0-or-later
//! A bare `canon` inside a canon: where you are, and what is waiting on you.
//!
//! **Each section is the verb it ends with, asked the same way.** What waits
//! for your yes or no is `review`'s queue, past its date is `overdue`'s, the
//! questions are `open`'s and the writes nobody looked at are the ones
//! `list` warns about. A second reading of "what is waiting for me" could
//! disagree with `review`, and the page would send a person to a verb that
//! says otherwise.

use std::collections::BTreeSet;

use crate::cmds::{load, or_fail};
use crate::{explain::one_line, govern::say_due, profile::Profile, review, store, wrap};

pub fn page() -> i32 {
    let (d, log, st) = or_fail!(load());
    let profile = or_fail!(Profile::load(&d));
    let (me, now) = (store::actor(), store::now());
    // The areas `who` would name you in charge of: held now, each once.
    let yours = |g: &&canon_core::Grant| g.actor == me && g.held_at(now);
    let held = st.grants().iter().filter(yours);
    let areas = Vec::from_iter(held.map(|g| g.scope.as_str()).collect::<BTreeSet<_>>());
    let whose = match profile {
        Profile::Personal => "your own".to_string(),
        _ => format!("this {}'s", profile.scope()),
    };
    let (noun, nouns, n) = (profile.noun(), profile.nouns(), st.active().count());
    print!("{whose} {nouns}: {n} in force.");
    if !areas.is_empty() {
        print!(" You are in charge of {}.", areas.join(", "));
    }
    println!();
    // Yours is the fold's answer: whether your yes or no would move it.
    let mut waiting = review::queue(&log, &st, &me, now);
    waiting.retain(|i| i.yours);
    if !waiting.is_empty() {
        println!("\nwaiting for your yes or no:");
        for i in &waiting {
            let text = format!("{}: {}", i.kind.noun(), one_line(&i.text));
            println!("{}", wrap::hang(&format!("  {}  ", i.id), &text));
            println!("{}", wrap::hang("      needs ", &i.needs));
        }
        println!("  canon review");
    }
    let due = st.overdue(now);
    if !due.is_empty() {
        println!("\npast its review date: {}", due.len());
        due.iter().take(3).for_each(|o| say_due(&st, o, now));
        println!("  canon overdue");
    }
    let unseen = st.unattended().len() + st.ungoverned().len();
    let open = st.open().count();
    if open > 0 {
        println!("\n{open} open question(s) — canon open");
    }
    // The two kinds of write `list` warns about: an agent's ruling, and a
    // change to who decides from someone not in charge of it.
    if unseen > 0 {
        println!("\n{unseen} written by an agent or by someone not in charge, waiting for a person — canon log");
    }
    if waiting.is_empty() && due.is_empty() && open + unseen == 0 {
        println!("\nnothing is waiting for you. Write one down: canon add \"<a {noun}>\"");
    }
    println!("\n`canon help` for everything else.");
    0
}
