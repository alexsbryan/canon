// SPDX-License-Identifier: AGPL-3.0-or-later
//! A draw nobody can steer.
//!
//! Selection by lot is the one entry in `PRIMITIVES.md` that was not designed
//! in advance — it fell out of the adequacy test, because every other
//! technology of political economy decomposed into the primitives already here
//! and sortition did not. It needs randomness, and randomness is what a
//! content-addressed, replayable ledger cannot casually have: a draw nobody can
//! reproduce is a draw nobody can audit, and a draw seeded by whoever called it
//! is not a draw.
//!
//! **The threat model is in `PRIMITIVES.md` under Primitive 9, it was written
//! before this file, and it changed the design.** The sketch that document
//! carried seeded the draw from the first act after a boundary not authored by
//! the drawer. That does not survive: an act's id is a hash of its own body, so
//! whoever writes it can try bodies until the shuffle favours them, and hashing
//! is cheap. What is here instead is commit-reveal across the pool.
//!
//! **The draw is a query, not an act.** Nobody performs it, so there is nothing
//! to perform badly. Given the log, every replayer computes the same panel, and
//! the drawer's only move is the commit — made before any secret exists.
//!
//! One residual, named rather than hidden: the last revealer can compare the
//! panel that results from revealing with the one that results from silence,
//! and choose. One bit, once, at the cost of their own seat. That is the
//! standard result for commit-reveal without an external beacon, and it does
//! not close with a ledger alone.

use serde::{Deserialize, Serialize};

use crate::fold::Canon;
use crate::id::{digest_hex, ActId};
use crate::scope::Scope;

/// A draw somebody announced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committed {
    pub act: ActId,
    pub scope: Scope,
    pub count: usize,
    pub after_ts: i64,
    /// When the announcement was written. The boundary must postdate it.
    pub at: i64,
    pub drawer: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub rationale: String,
}

/// A digest somebody published before the boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sealed {
    pub commit: ActId,
    pub actor: String,
    pub digest: String,
    pub at: i64,
}

/// A secret somebody published after it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Opened {
    pub commit: ActId,
    pub actor: String,
    pub secret: String,
    pub at: i64,
}

/// The panel, and everything needed to check it by hand.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Drawn {
    pub commit: ActId,
    /// Who was selected, in draw order.
    pub seats: Vec<String>,
    /// Who was eligible, after exclusions. Sorted.
    pub pool: Vec<String>,
    /// The seed, in hex, so a person can recompute it.
    pub seed: String,
    /// Whose secrets went into the seed. Sorted.
    pub contributed: Vec<String>,
    /// Who sealed a secret and never opened it. **Excluded from the pool** —
    /// withholding costs you your seat, which is what bounds the one
    /// influence this scheme leaves open.
    pub withheld: Vec<String>,
}

/// Why a draw refuses.
///
/// **Every one of these refuses rather than falling back.** A lottery that
/// quietly degrades is worse than no lottery, because it launders a chosen
/// panel as a fair one (§18.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", rename_all = "snake_case")]
pub enum DrawError {
    NoSuchDraw {
        commit: ActId,
    },
    /// The boundary did not postdate its own announcement, so the drawer
    /// chose a moment they could already see.
    BoundaryNotInFuture {
        after_ts: i64,
        committed_at: i64,
    },
    /// Nobody opened a secret. There is no seed, and there is no default one.
    NothingRevealed,
    /// Nobody holds standing over the pool's scope.
    EmptyPool {
        scope: Scope,
    },
    /// A draw that selects everyone is not a draw.
    PoolTooSmall {
        pool: usize,
        count: usize,
    },
    NoSeats,
}

impl std::fmt::Display for DrawError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoSuchDraw { commit } => write!(f, "no draw announced as {commit}"),
            Self::BoundaryNotInFuture {
                after_ts,
                committed_at,
            } => write!(
                f,
                "the boundary ({after_ts}) does not postdate the announcement ({committed_at}) — \
                 a draw whose moment was chosen after the fact is not a draw"
            ),
            Self::NothingRevealed => write!(
                f,
                "nobody opened a secret, so there is no seed — and there is no default seed"
            ),
            Self::EmptyPool { scope } => {
                write!(
                    f,
                    "nobody holds standing over `{scope}`, so there is no pool"
                )
            }
            Self::PoolTooSmall { pool, count } => write!(
                f,
                "{count} seat(s) from a pool of {pool} would select everyone, which is not a draw"
            ),
            Self::NoSeats => write!(f, "a draw for zero seats selects nobody"),
        }
    }
}

impl std::error::Error for DrawError {}

impl Canon {
    /// The draw announced by this act, computed from the log.
    ///
    /// Pure, and that is the entire security argument: two people running this
    /// on the same file get the same panel, and neither of them can change it.
    pub fn draw(&self, commit: &ActId) -> Result<Drawn, DrawError> {
        let announced = self
            .draws
            .iter()
            .find(|d| &d.act == commit)
            .ok_or_else(|| DrawError::NoSuchDraw {
                commit: commit.clone(),
            })?;
        if announced.after_ts <= announced.at {
            return Err(DrawError::BoundaryNotInFuture {
                after_ts: announced.after_ts,
                committed_at: announced.at,
            });
        }
        if announced.count == 0 {
            return Err(DrawError::NoSeats);
        }

        // The pool is frozen at the BOUNDARY, not at the moment somebody asks.
        // Standing granted after it does not join; standing that lapsed before
        // it does not count.
        let mut eligible: Vec<String> = self
            .who_decides(&announced.scope, announced.after_ts)
            .iter()
            .map(|g| g.actor.clone())
            .collect();
        eligible.sort_unstable();
        eligible.dedup();
        if eligible.is_empty() {
            return Err(DrawError::EmptyPool {
                scope: announced.scope.clone(),
            });
        }

        let mut contributed: Vec<(String, String)> = Vec::new();
        let mut withheld: Vec<String> = Vec::new();
        for actor in &eligible {
            // First seal per actor. A second would let somebody publish
            // several digests and open whichever flatters them.
            let Some(sealed) = self
                .sealed
                .iter()
                .find(|s| &s.commit == commit && &s.actor == actor)
            else {
                continue;
            };
            // A seal written at or after the boundary is not a commitment to
            // anything — by then the other secrets are being opened.
            if sealed.at >= announced.after_ts {
                continue;
            }
            let opened = self
                .opened
                .iter()
                .find(|o| &o.commit == commit && &o.actor == actor)
                // Opened BEFORE the boundary leaks it to anyone who has not
                // sealed yet, so it does not count either.
                .filter(|o| o.at >= announced.after_ts)
                .filter(|o| digest_hex(o.secret.as_bytes()) == sealed.digest);
            match opened {
                Some(o) => contributed.push((actor.clone(), o.secret.clone())),
                None => withheld.push(actor.clone()),
            }
        }

        if contributed.is_empty() {
            return Err(DrawError::NothingRevealed);
        }

        // Sealing and then staying silent costs you your seat. That is what
        // bounds the last revealer's one bit of influence.
        let mut pool: Vec<String> = eligible
            .into_iter()
            .filter(|a| !withheld.contains(a))
            .collect();
        if announced.count >= pool.len() {
            return Err(DrawError::PoolTooSmall {
                pool: pool.len(),
                count: announced.count,
            });
        }

        // Canonical, so two machines seed identically. Sorted by actor, with
        // separators that cannot appear in either field's rendering, so
        // `("ab", "c")` and `("a", "bc")` cannot collide.
        contributed.sort();
        let mut material = Vec::new();
        material.extend_from_slice(commit.as_str().as_bytes());
        for (actor, secret) in &contributed {
            material.push(0x1e);
            material.extend_from_slice(actor.as_bytes());
            material.push(0x1f);
            material.extend_from_slice(secret.as_bytes());
        }
        let seed = digest_hex(&material);

        shuffle(&seed, &mut pool);
        let seats = pool.iter().take(announced.count).cloned().collect();
        let mut sorted_pool = pool;
        sorted_pool.sort_unstable();
        Ok(Drawn {
            commit: commit.clone(),
            seats,
            pool: sorted_pool,
            seed,
            contributed: contributed.into_iter().map(|(a, _)| a).collect(),
            withheld,
        })
    }

    /// Draws announced but not yet drawable, and why.
    pub fn draws_announced(&self) -> &[Committed] {
        &self.draws
    }
}

/// A seed-keyed Fisher-Yates.
///
/// The stream is `sha256(seed || counter)`, taken eight bytes at a time —
/// no extra dependency, and reproducible by anyone with a hash function.
/// Rejection sampling rather than a modulo, because a biased lottery is a
/// steerable one and the bias would be invisible.
fn shuffle(seed: &str, items: &mut [String]) {
    let mut stream = Stream::new(seed);
    for i in (1..items.len()).rev() {
        let j = stream.below(i as u64 + 1) as usize;
        items.swap(i, j);
    }
}

struct Stream {
    seed: String,
    counter: u64,
    buffer: Vec<u8>,
}

impl Stream {
    fn new(seed: &str) -> Self {
        Self {
            seed: seed.to_string(),
            counter: 0,
            buffer: Vec::new(),
        }
    }

    fn next_u64(&mut self) -> u64 {
        if self.buffer.len() < 8 {
            let block = digest_hex(format!("{}|{}", self.seed, self.counter).as_bytes());
            self.counter += 1;
            self.buffer.extend(block.into_bytes());
        }
        let bytes: Vec<u8> = self.buffer.drain(..8).collect();
        u64::from_be_bytes(bytes.try_into().expect("eight bytes"))
    }

    /// Uniform in `0..n`. Rejection sampling: the largest multiple of `n`
    /// that fits in a u64 is the acceptance window, and anything above it is
    /// discarded rather than folded down.
    fn below(&mut self, n: u64) -> u64 {
        debug_assert!(n > 0);
        let limit = u64::MAX - (u64::MAX % n) - 1;
        loop {
            let v = self.next_u64();
            if v <= limit {
                return v % n;
            }
        }
    }
}

#[cfg(test)]
mod tests;
