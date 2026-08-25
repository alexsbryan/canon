// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon tensions` — where your own commitments pull against each other.
//!
//! One call, every live commitment in context. That is affordable because a
//! canon is small by design: thirty commitments is under two thousand tokens,
//! so all-pairs comparison needs no retrieval and no candidate enumerator.
//! Past roughly a hundred it stops being affordable, and that limit is stated
//! in the spec rather than discovered by a user.
//!
//! **What comes back is a proposal, not a ruling.** Every pair is minted as
//! [`Disposition::Open`] and nothing is written to the log: a tension becomes
//! a fact about the canon only when a person runs `accept` or `dismiss`.
//! Pairs already ruled on are filtered out through [`Canon::is_settled`], so
//! a decision someone already made is never re-served as news.

use canon_core::{ActId, Canon, Commitment, Conflict, Disposition};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::model::{self, Client, ModelError};

const SYSTEM: &str = "\
You compare normative commitments and find genuine tensions.

A tension is a pair that cannot both be fully honoured in some realistic \
situation. The reason must name that situation.

Rules:
- Precision over recall. Report a pair only if you can name the situation.
- Commitments about different subjects are not a tension.
- A general rule and a specific case of it are not a tension.
- One commitment being harder to keep than another is not a tension.
- Report each pair once.
- Write the reason as one or two sentences describing the situation. Quote or \
paraphrase a commitment; never refer to one by its number.";

#[derive(Debug, Deserialize)]
struct Found {
    #[serde(default)]
    tensions: Vec<Pair>,
}

#[derive(Debug, Deserialize)]
struct Pair {
    a: crate::model::Pos,
    b: crate::model::Pos,
    #[serde(default)]
    reason: String,
}

fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "tensions": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "a": { "type": "integer", "description": "number of the first commitment" },
                        "b": { "type": "integer", "description": "number of the second commitment" },
                        "reason": {
                            "type": "string",
                            "description": "the situation in which both cannot be honoured",
                        },
                    },
                    "required": ["a", "b", "reason"],
                    "additionalProperties": false,
                },
            },
        },
        "required": ["tensions"],
        "additionalProperties": false,
    })
}

/// A pair the model proposed, still in terms of the list it was given.
///
/// Indices, not ids: `draft` compares candidate texts that have no id yet,
/// and `tensions` compares commitments that do. One engine, and the caller
/// attaches identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Proposed {
    pub a: usize,
    pub b: usize,
    pub reason: String,
}

/// How many commitments go into one comparison.
///
/// **Measured, not chosen.** One call over sixty commitments asks a model to
/// weigh 1,770 pairs in a single pass, and it does not: replaying the same
/// sixty extracted rules through only this step found 1 of 11 planted
/// tensions in one call, 3 of 11 in blocks of twenty, and 5 of 11 in blocks
/// of twelve — with zero false positives on the seven labelled compatible
/// pairs at every size. Recall was the casualty of the batch size, and
/// discrimination never was.
///
/// Twenty-four rather than the twelve that measurement named, because the
/// twelve was a BLOCK size under the old block-pairwise scheme and a block
/// was compared against another block: 300 of that scheme's 325 passes on a
/// 289-commitment canon held twenty-four commitments, not twelve. This is the
/// pass size that was actually measured; it is now stated directly instead of
/// emerging from two blocks being concatenated.
const BATCH: usize = 24;

/// How many times every pair is examined, in different company each time.
///
/// **The number this replaced was a lottery, not a policy.** Block-pairwise
/// comparison gave a pair one look if it straddled two blocks and `k` looks
/// if it sat inside one — measured on a 289-commitment canon: 40,032 pairs
/// (96.2%) got exactly ONE look and 1,584 pairs (3.8%) got TWENTY-FIVE, with
/// nothing in between. Which side a pair landed on was decided by whether the
/// two rules happened to be adjacent in the document.
///
/// That distribution is also what the two-arrangement union was buying back:
/// a second arrangement promotes a different 3.8% out of the one-look
/// majority, which is why its hits were disjoint from the first's. Asking for
/// the looks directly is the same purchase made on purpose, spread evenly,
/// and it costs fewer calls than the union did — 488 passes against 650 at
/// n=289.
const LOOKS: usize = 2;

/// The comparison schedule: which commitments are weighed together, and how
/// often each pair comes up.
///
/// A **covering design** — the classical object for "every pair in at least
/// one block". Built greedily: seed each block with the commitment in the
/// most still-unexamined pairs, then grow it by whichever commitment adds the
/// most unexamined pairs to what is already there. Deterministic throughout,
/// ties to the lower position, so two runs over one document produce one
/// schedule and a noise floor stays a property of the model.
///
/// **The guarantee is the one block-pairwise made, and it is stronger:**
/// every unordered pair appears in at least [`LOOKS`] returned sets rather
/// than in at least one. Asserted rather than argued — see the tests.
///
/// Greedy rather than optimal, and the gap is known: the Schönheim bound puts
/// a perfect covering of 289 commitments in blocks of 24 at 157 passes and
/// this finds 267. Closing that is a better construction, not a different
/// contract, and nothing above here would change.
///
/// **What the blocks are NOT ordered by, and why it is not coming back.**
/// Clustering near-twins so a contradiction is weighed against the rule it
/// contradicts was measured on 2026-08-24, one variable on one pinned binary:
/// recall 0.55 -> 0.39, decoys 0/7 -> 2/7, reachability 10/11 -> 9/11. A
/// comparison pass is GENERATIVE, and a block of near-twins starves it of the
/// contrast that makes a conflict stand out — narrowing helps a scorer judging
/// one pair in isolation and hurts a pass where the block's diversity IS the
/// signal. The knob is deleted, not defaulted off.
fn schedule(n: usize, looks: usize) -> Vec<Vec<usize>> {
    if n < 2 || looks == 0 {
        return Vec::new();
    }
    // Looks still owed to each unordered pair. `n` is bounded by what a canon
    // is (the spec names a ceiling), so a dense triangle is the cheap choice.
    let idx = |a: usize, b: usize| a * n + b;
    let mut owed = vec![0u32; n * n];
    for a in 0..n {
        for b in a + 1..n {
            owed[idx(a, b)] = looks as u32;
        }
    }
    let owes = |owed: &[u32], a: usize, b: usize| {
        if a == b {
            0
        } else if a < b {
            owed[idx(a, b)]
        } else {
            owed[idx(b, a)]
        }
    };

    let mut out: Vec<Vec<usize>> = Vec::new();
    loop {
        // How many still-owed pairs each commitment sits in.
        let mut deficit = vec![0usize; n];
        for a in 0..n {
            for b in a + 1..n {
                if owed[idx(a, b)] > 0 {
                    deficit[a] += 1;
                    deficit[b] += 1;
                }
            }
        }
        let Some(seed) = (0..n).max_by_key(|i| (deficit[*i], std::cmp::Reverse(*i))) else {
            break;
        };
        if deficit[seed] == 0 {
            break;
        }
        let mut block = vec![seed];
        while block.len() < BATCH.min(n) {
            let best = (0..n).filter(|i| !block.contains(i)).max_by_key(|i| {
                let gain = block.iter().filter(|j| owes(&owed, *i, **j) > 0).count();
                (gain, std::cmp::Reverse(*i))
            });
            let Some(next) = best else { break };
            // A block that can add nothing new is finished. Padding it out
            // would buy re-examinations nobody asked for, which is the very
            // thing this replaced.
            if block.iter().all(|j| owes(&owed, next, *j) == 0) {
                break;
            }
            block.push(next);
        }
        if block.len() < 2 {
            break;
        }
        for x in 0..block.len() {
            for y in x + 1..block.len() {
                let (a, b) = (block[x].min(block[y]), block[x].max(block[y]));
                owed[idx(a, b)] = owed[idx(a, b)].saturating_sub(1);
            }
        }
        block.sort_unstable();
        out.push(block);
    }
    out
}

/// The engine: find tensions among these texts, answering in list positions.
///
/// At or below [`BATCH`] one pass already holds every pair, and asking for a
/// second look at the same crowd is asking the same question again rather
/// than a different one. Such a run costs exactly one call.
///
/// Above it the passes come from [`schedule`]: a covering design in which
/// every unordered pair appears in at least [`LOOKS`] passes, in different
/// company each time. A cover and not a partition — no pair is split across
/// passes and none goes unexamined. The cost stays quadratic in the number of
/// commitments, which is the reason the spec names a ceiling on how large a
/// canon this tool serves.
pub fn detect_over(client: &Client, texts: &[&str]) -> Result<Compared, ModelError> {
    if texts.len() < 2 {
        return Ok(Compared::default());
    }
    if texts.len() <= BATCH {
        // One pass already holds every pair. Asking for a second look at the
        // same crowd is asking the same question again, not a different one.
        let pairs = one_pass(client, texts, &(0..texts.len()).collect::<Vec<_>>())?;
        return Ok(Compared {
            pairs,
            passes: 1,
            unread: Vec::new(),
            schedule: Schedule {
                passes: 1,
                batch: texts.len(),
                looks: 1,
            },
        });
    }

    let sets = schedule(texts.len(), LOOKS);
    let passes = sets.len();
    eprintln!(
        "{} commitments is past what one comparison holds — {passes} passes over blocks of \
         {BATCH}, every pair weighed {LOOKS} times in different company",
        texts.len()
    );

    let mut out: Vec<Proposed> = Vec::new();
    let mut unread: Vec<String> = Vec::new();
    let mut last_err: Option<ModelError> = None;
    for (i, idx) in sets.iter().enumerate() {
        let done = i + 1;
        eprint!("\rcomparing {done}/{passes}…");
        let _ = std::io::Write::flush(&mut std::io::stderr());
        // One pass failing is not the canon's. The map step has always
        // recorded a chunk it could not read and kept going; this step threw
        // away every other comparison for one refusal, and on a 34-section
        // ordinance that cost a whole run twenty passes in, thirty-three
        // minutes deep, with the two runs behind it never starting. What a
        // refusal costs instead is COVERAGE, which is recorded and reported
        // rather than absorbed (§18.3).
        let got = match one_pass(client, texts, idx) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("\nwarning: comparison {done}/{passes} produced no answer: {e}");
                // The positions, not just the pass number. A comparison that
                // fails once and cannot be reproduced is a mystery for as
                // long as its INPUT is unrecoverable — and one of these
                // failed on a Des Moines sweep in a way no synthetic pass of
                // the same size and shape would repeat. With the positions
                // recorded, the artifact's own `kept` and `candidates` give
                // the exact texts back, and the next occurrence is a bug
                // someone can drive rather than a story about one.
                unread.push(format!(
                    "pass {done}/{passes} over kept positions {idx:?}: {e}"
                ));
                last_err = Some(e);
                continue;
            }
        };
        for p in got {
            // The same pair can surface in more than one pass — it is weighed
            // LOOKS times by construction. First reason wins; a pair is one
            // tension however often it is noticed.
            if !out.iter().any(|q| is_same(q, &p)) {
                out.push(p);
            }
        }
    }
    // Nothing was compared at all. A run reporting zero tensions because zero
    // comparisons happened is the failure §18.3 names, so the real error from
    // the last attempt is returned rather than an empty result.
    if let (true, Some(e)) = (unread.len() == passes, last_err) {
        return Err(e);
    }
    eprintln!(
        "\r{}/{passes} passes done, {} pair(s) proposed",
        passes - unread.len(),
        out.len()
    );
    if !unread.is_empty() {
        // Loud, because every tension number from this run is a number about
        // a fraction of the pairs.
        eprintln!(
            "WARNING: {} of {passes} comparison(s) went unread — this run weighed {:.0}% of the pairs",
            unread.len(),
            100.0 * (passes - unread.len()) as f64 / passes as f64
        );
    }

    Ok(Compared {
        pairs: out,
        passes,
        unread,
        schedule: Schedule {
            passes,
            batch: BATCH,
            looks: LOOKS,
        },
    })
}

/// The shape of the comparison a run actually ran.
///
/// In the artifact because a recall number is a number about a schedule: a
/// reader who cannot see how many times each pair was weighed cannot compare
/// two runs, and `looks` is the knob most likely to move the number.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schedule {
    pub passes: usize,
    /// How many commitments one comparison held.
    pub batch: usize,
    /// How many times every pair was examined.
    pub looks: usize,
}

/// What a comparison run weighed, and what it could not.
///
/// `passes` is what was attempted and `unread` what came back unusable, so a
/// caller can say what fraction of the pair space a number covers instead of
/// implying all of it.
#[derive(Debug, Default)]
pub struct Compared {
    pub pairs: Vec<Proposed>,
    pub passes: usize,
    pub unread: Vec<String>,
    pub schedule: Schedule,
}

fn is_same(a: &Proposed, b: &Proposed) -> bool {
    (a.a, a.b) == (b.a, b.b) || (a.a, a.b) == (b.b, b.a)
}

/// One comparison over the texts at `idx`, answering in GLOBAL positions.
fn one_pass(client: &Client, texts: &[&str], idx: &[usize]) -> Result<Vec<Proposed>, ModelError> {
    if idx.len() < 2 {
        return Ok(Vec::new());
    }
    let mut user = String::from("Commitments:\n");
    for (i, g) in idx.iter().enumerate() {
        user.push_str(&format!("{}. {}\n", i + 1, texts[*g]));
    }
    user.push_str("\nReturn every pair in tension, with the situation that forces the choice.");

    let found: Found = client.complete_json(SYSTEM, &user, "tensions", &schema())?;

    let mut out: Vec<Proposed> = Vec::new();
    for p in found.tensions {
        let in_range = |n: usize| n >= 1 && n <= idx.len();
        // Zero is not a position the model was offered, so a sentinel that
        // is not a position at all lands on the same refusal as one that is
        // simply wrong — and the warning below still names what was said.
        let (pa, pb) = (p.a.get().unwrap_or(0), p.b.get().unwrap_or(0));
        if !in_range(pa) || !in_range(pb) {
            eprintln!(
                "\nwarning: dropped a proposed tension naming commitment {} and {} — only 1..{} were offered",
                p.a,
                p.b,
                idx.len()
            );
            continue;
        }
        // Back to positions in the caller's list, which is what identity is
        // attached to. A pass never returns its own numbering.
        let (a, b) = (idx[pa - 1], idx[pb - 1]);
        if a == b {
            eprintln!("\nwarning: dropped a proposed tension of a commitment with itself");
            continue;
        }
        let proposed = Proposed {
            a,
            b,
            reason: p.reason.trim().to_string(),
        };
        if !out.iter().any(|q| is_same(q, &proposed)) {
            out.push(proposed);
        }
    }
    Ok(out)
}

/// Find tensions among commitments that already have ids.
pub fn detect(client: &Client, commitments: &[&Commitment]) -> Result<Vec<Conflict>, ModelError> {
    let texts: Vec<&str> = commitments.iter().map(|c| c.text.as_str()).collect();
    Ok(detect_over(client, &texts)?
        .pairs
        .into_iter()
        .map(|p| Conflict {
            a: commitments[p.a].id.clone(),
            b: commitments[p.b].id.clone(),
            // `at: 0` because nothing happened: no act was written, so there
            // is no moment to record. The fold never mints this.
            at: 0,
            disposition: Disposition::Open { reason: p.reason },
        })
        .collect())
}

/// Drop pairs someone already ruled on. Returns `(open, settled_count)`.
pub fn unsettled(canon: &Canon, found: Vec<Conflict>) -> (Vec<Conflict>, usize) {
    let before = found.len();
    let open: Vec<Conflict> = found
        .into_iter()
        .filter(|c| !canon.is_settled(&c.a, &c.b))
        .collect();
    let settled = before - open.len();
    (open, settled)
}

/// The shared rendering — `tensions` and the end of `draft` show the same
/// thing, so they call the same code rather than drifting into two versions
/// of it the way `why` once did.
pub fn render(canon: &Canon, open: &[Conflict], settled: usize) -> String {
    let mut out = String::new();
    if open.is_empty() {
        out.push_str("no tensions found.\n");
        if settled > 0 {
            out.push_str(&format!(
                "  {settled} pair(s) were already ruled on and are not re-shown — `canon list --json` for them.\n"
            ));
        }
        return out;
    }
    out.push_str(&format!(
        "{} tension(s) — proposed, and nobody has ruled on them yet.\n\n",
        open.len()
    ));
    let text = |id: &ActId| {
        canon
            .get(id)
            .map(|c| c.text.clone())
            .unwrap_or_else(|| "(not in this canon)".into())
    };
    for c in open {
        out.push_str(&format!("  {}  {}\n", c.a, text(&c.a)));
        out.push_str(&format!("  {}  {}\n", c.b, text(&c.b)));
        if let Disposition::Open { reason } = &c.disposition {
            if !reason.is_empty() {
                out.push_str(&format!("  why: {reason}\n"));
            }
        }
        out.push_str(&format!(
            "  carry it knowingly:  canon accept {} {} -m \"<what this protects>\"\n",
            c.a, c.b
        ));
        out.push_str(&format!(
            "  or it is not one:    canon dismiss {} {}\n\n",
            c.a, c.b
        ));
    }
    if settled > 0 {
        out.push_str(&format!(
            "{settled} further pair(s) were already ruled on and are not re-shown.\n"
        ));
    }
    out
}

pub fn run(args: &[String]) -> i32 {
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return crate::cmds::fail(e),
    };
    let live: Vec<&Commitment> = canon.active().collect();
    let json = crate::cmds::has(args, "--json");
    if live.len() < 2 {
        if json {
            println!("[]");
        } else {
            let profile = crate::profile::Profile::load(&dir).unwrap_or_default();
            println!(
                "fewer than two live {} — nothing to compare.",
                profile.nouns()
            );
        }
        return 0;
    }
    let client = match model::client_for(&dir, crate::cmds::has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!(
        "comparing {} commitments on {}",
        live.len(),
        client.describe()
    );
    let found = match detect(&client, &live) {
        Ok(f) => f,
        Err(e) => return model::report(e),
    };
    let (open, settled) = unsettled(&canon, found);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&open).unwrap_or_else(|_| "[]".into())
        );
    } else {
        print!("{}", render(&canon, &open, settled));
    }
    // Exit 0 even when tensions are found: this is a report, not an
    // adjudication. Exit 1 belongs to `check`, which rules on a proposal —
    // and the personal profile must never emit it at all.
    0
}

#[cfg(test)]
mod tests;
