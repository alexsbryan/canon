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
/// discrimination never was. The spec claimed this regime held to about a
/// hundred commitments; it does not, and that claim has been corrected.
const BATCH: usize = 12;

/// Cut an order into blocks and pair every block with itself and each other.
///
/// Separated from [`detect_over`] because the property that matters is about
/// this function alone and nothing else: **every unordered pair of positions
/// appears in at least one returned set, for any order given.** That is what
/// lets `similarity_order` rearrange the list freely — it moves pairs between
/// passes and can never drop one — and it is asserted rather than argued.
///
/// Not a partition, and the difference is the point. A cross pass over blocks
/// `x` and `y` shows the model all of `x`, all of `y`, and everything
/// between, so a WITHIN-block pair is examined again in every cross pass its
/// block takes part in — `k` times over, against `k`-1 for a pair that
/// straddles two blocks. Its extra looks are the good ones, too: a self pass
/// weighs it against `C(BATCH,2)` others where a cross pass weighs it against
/// `C(2*BATCH,2)`. `detect_over` folds the repeats, so the cost is calls
/// rather than duplicate tensions — and the asymmetry is precisely why
/// `similarity_order` is worth its one embedding call, since it decides which
/// pairs get the many good looks and which get the one crowded one.
fn passes_over(order: &[usize]) -> Vec<Vec<usize>> {
    let blocks: Vec<&[usize]> = order.chunks(BATCH).collect();
    // A pass over fewer than two texts compares nothing. It cannot fail and
    // cannot find anything, so counting it would pad the coverage this run
    // reports — the last block is a single commitment whenever the count is
    // one past a multiple of BATCH.
    (0..blocks.len())
        .flat_map(|x| (x..blocks.len()).map(move |y| (x, y)))
        .map(|(x, y)| {
            if x == y {
                blocks[x].to_vec()
            } else {
                blocks[x].iter().chain(blocks[y]).copied().collect()
            }
        })
        .filter(|idx| idx.len() >= 2)
        .collect()
}

/// How one run lays the list out before [`passes_over`] cuts it into blocks.
///
/// Every arrangement is a PERMUTATION, and `passes_over` covers every
/// unordered pair whatever the order — so an arrangement decides only WHO a
/// pair is weighed against, never WHICH pairs are weighed. That is what makes
/// several of them a union rather than a gamble: each one is a complete
/// comparison on its own, so a second arrangement can only add.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrangement {
    /// The list as the caller gave it — document order, or the similarity
    /// chain of [`similarity_order`] when an embedding model is configured.
    Given,
    /// The block matrix read down its columns rather than across its rows.
    ///
    /// Commitments that shared a block in the given order land in different
    /// ones here, and the company they keep instead is drawn one from each
    /// old block. Where the list fills at least [`BATCH`] blocks the
    /// separation is total: no two that shared a block share one again.
    Transposed,
}

impl Arrangement {
    /// The name a run reports this arrangement under. One spelling, in the
    /// artifact and on the terminal both.
    pub fn name(self) -> &'static str {
        match self {
            Arrangement::Given => "given",
            Arrangement::Transposed => "transposed",
        }
    }

    /// Permute a base order into this arrangement.
    ///
    /// A bijection in both arms, which is the whole contract: `passes_over`
    /// guarantees coverage for any permutation, and guarantees nothing at all
    /// for a list that gained or lost an entry.
    fn apply(self, base: &[usize]) -> Vec<usize> {
        match self {
            Arrangement::Given => base.to_vec(),
            Arrangement::Transposed => {
                let rows = base.len().div_ceil(BATCH);
                (0..BATCH)
                    // Down each column, taking one entry from every block
                    // before moving on. The last row is short whenever the
                    // count is not a multiple of BATCH, and `get` drops those
                    // holes rather than inventing positions for them.
                    .flat_map(|col| (0..rows).map(move |row| row * BATCH + col))
                    .filter_map(|i| base.get(i).copied())
                    .collect()
            }
        }
    }
}

/// The arrangements one comparison run folds into a single answer.
///
/// **Measured on two corpora, and what generalises is the DISJOINTNESS.** Two
/// arrangements of the same list propose overlapping but different tension
/// sets, so their union exceeds either alone: on the Maple House charter 6 of
/// 11 planted tensions became 9 of 11 with decoys unchanged at 0, and on the
/// Des Moines noise ordinance 5 of 11 became 7 of 11 with decoys unchanged at
/// 2. Neither arrangement beat the other by more than a single tension on its
/// own, and the precision half of the Maple House result did NOT reproduce on
/// Des Moines — so what this buys is recall, at `k(k+1)/2` calls per
/// arrangement, and nothing else is claimed for it.
///
/// Those two numbers come from unioning the PROPOSALS of two runs, one
/// arrangement each, on a tape that held extraction identical. This code is
/// the mechanism they argue for and not the thing they measured — the bar in
/// `tests/draft_bar.rs` is what measures it, over three live runs per corpus.
///
/// Whether a third arrangement keeps adding is unmeasured. The number that
/// answers it is `added` in [`ByArrangement`], which every run reports.
const ARRANGEMENTS: &[Arrangement] = &[Arrangement::Given, Arrangement::Transposed];

/// Order commitments so that near-twins share a block.
///
/// **Coverage is unaffected, by construction.** Block-pairwise compares every
/// block with itself and with every other one, so each unordered pair lands in
/// exactly one pass whatever order the list is in. Reordering MOVES a pair
/// between passes; it can never drop one. What it changes is how much company
/// a pair keeps: a pair inside one block is weighed against `C(12,2)` = 66
/// others, and a pair split across two blocks against `C(24,2)` = 276.
///
/// Measured on the Des Moines corpus: this moves 6 of 11 planted pairs out of
/// a cross pass and into a self pass, with no coverage change and no extra
/// model call. It also speaks to the only pass that ever failed there — pass
/// 16 of 28, the cross pass holding one block of Type "G" permit rules
/// against the block restating the same rules in an amending ordinance. It
/// ran 3,094 tokens into a 300s deadline and died, in all three runs, while
/// the mean pass that finished emitted 234. The same 24 texts through a
/// smaller model finished in 29s and proposed 2 pairs, so the input is not
/// inherently pathological — facing one block of near-twins against another
/// is, and that is the arrangement this removes.
///
/// A greedy nearest-neighbour chain rather than a clustering: deterministic,
/// single-pass, and it needs neither a `k` nor a threshold. Ties go to the
/// lower position, so two runs over one document order identically.
fn similarity_order(client: &Client, texts: &[&str]) -> Vec<usize> {
    let n = texts.len();
    let document_order = || (0..n).collect::<Vec<usize>>();
    let vectors = match client.embed(texts) {
        Ok(v) if v.len() == n => v,
        // Every one of these keeps the run going in document order, which is
        // what it always did — but says so, because a quieter comparison is
        // not the same comparison (§18.3).
        Ok(v) => {
            eprintln!(
                "\nnote: comparing in document order — the endpoint returned {} vector(s) for {n} commitments",
                v.len()
            );
            return document_order();
        }
        Err(ModelError::NoEmbedModel) => {
            eprintln!(
                "\nnote: comparing in document order — `canon config set embed_model <name>` \
                 groups near-duplicates into one comparison instead of splitting them across two"
            );
            return document_order();
        }
        Err(e) => {
            eprintln!("\nnote: comparing in document order — embedding failed: {e}");
            return document_order();
        }
    };

    let norms: Vec<f32> = vectors
        .iter()
        .map(|v| v.iter().map(|x| x * x).sum::<f32>().sqrt())
        .collect();
    // A zero vector has no direction, so it is similar to nothing rather than
    // to everything — which is what dividing by its norm would produce.
    let cosine = |i: usize, j: usize| -> f32 {
        if norms[i] == 0.0 || norms[j] == 0.0 {
            return 0.0;
        }
        let dot: f32 = vectors[i].iter().zip(&vectors[j]).map(|(a, b)| a * b).sum();
        dot / (norms[i] * norms[j])
    };

    let mut order = Vec::with_capacity(n);
    let mut placed = vec![false; n];
    order.push(0);
    placed[0] = true;
    for _ in 1..n {
        let last = *order.last().expect("just pushed");
        let next = (0..n)
            .filter(|k| !placed[*k])
            .max_by(|a, b| {
                cosine(last, *a)
                    .total_cmp(&cosine(last, *b))
                    // Equal similarity goes to the lower position, so the
                    // order is a function of the document and nothing else.
                    .then(b.cmp(a))
            })
            .expect("one unplaced remains");
        placed[next] = true;
        order.push(next);
    }
    eprintln!("\nordered {n} commitments by similarity so near-duplicates share a comparison");
    order
}

/// The engine: find tensions among these texts, answering in list positions.
///
/// Above [`BATCH`] the comparison is **block-pairwise**: the list is cut into
/// blocks and every block is compared with itself and with every other block,
/// so no pair goes unexamined — see [`passes_over`] for why that is a cover
/// and not a partition. That costs `k(k+1)/2` calls for `k` blocks —
/// quadratic, and the reason the spec names a ceiling on how large a canon
/// this tool serves.
///
/// And it runs that whole comparison once per entry in [`ARRANGEMENTS`],
/// folding the results into one set. Each arrangement examines every pair, so
/// this is a union of complete comparisons rather than a sampling of a
/// partial one: what the second arrangement changes is the company each pair
/// keeps, and pairs a crowded pass talked past are noticed in a quieter one.
/// The cost is the call count multiplied by the number of arrangements, and
/// [`Compared::arrangements`] carries what each of them was worth.
pub fn detect_over(client: &Client, texts: &[&str]) -> Result<Compared, ModelError> {
    if texts.len() < 2 {
        return Ok(Compared::default());
    }
    if texts.len() <= BATCH {
        // One pass already holds every pair, so every arrangement of it is
        // the same comparison run again. A union of one is that one.
        let pairs = one_pass(client, texts, &(0..texts.len()).collect::<Vec<_>>())?;
        let found = pairs.len();
        return Ok(Compared {
            pairs,
            passes: 1,
            unread: Vec::new(),
            arrangements: vec![ByArrangement {
                arrangement: Arrangement::Given.name().to_string(),
                passes: 1,
                unread: 0,
                proposed: found,
                added: found,
            }],
        });
    }

    // The base order is chosen ONCE and every arrangement is a permutation of
    // it, so `similarity_order`'s one embedding call is not paid per
    // arrangement — and the escape hatch it is behind still decides what the
    // first arrangement looks like, exactly as it did before the union.
    let base = similarity_order(client, texts);
    let plan: Vec<(Arrangement, Vec<Vec<usize>>)> = ARRANGEMENTS
        .iter()
        .map(|a| (*a, passes_over(&a.apply(&base))))
        .collect();
    let passes: usize = plan.iter().map(|(_, sets)| sets.len()).sum();
    eprintln!(
        "{} commitments is past what one comparison holds — {passes} passes over blocks of \
         {BATCH}, {} arrangement(s) of the same list",
        texts.len(),
        plan.len()
    );

    let mut out: Vec<Proposed> = Vec::new();
    let mut unread: Vec<String> = Vec::new();
    let mut arrangements: Vec<ByArrangement> = Vec::new();
    let mut last_err: Option<ModelError> = None;
    let mut done = 0usize;
    for (arrangement, sets) in &plan {
        // Folded within the arrangement first, so `proposed` counts pairs and
        // not sightings: a within-block pair is shown to the model in several
        // passes of the same arrangement by construction.
        let mut mine: Vec<Proposed> = Vec::new();
        let mut mine_unread = 0usize;
        for (i, idx) in sets.iter().enumerate() {
            done += 1;
            eprint!("\rcomparing {done}/{passes} ({})…", arrangement.name());
            let _ = std::io::Write::flush(&mut std::io::stderr());
            // One pass failing is not the canon's. The map step has always
            // recorded a chunk it could not read and kept going; this step
            // threw away every other comparison for one refusal, and on a
            // 34-section ordinance that cost a whole run twenty passes in,
            // thirty-three minutes deep, with the two runs behind it never
            // starting. What a refusal costs instead is COVERAGE, which is
            // recorded and reported rather than absorbed (§18.3).
            let got = match one_pass(client, texts, idx) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("\nwarning: comparison {done}/{passes} produced no answer: {e}");
                    // The arrangement and the positions, not just the pass
                    // number. A comparison that fails once and cannot be
                    // reproduced is a mystery for as long as its INPUT is
                    // unrecoverable — and one of these failed on a Des Moines
                    // sweep in a way no synthetic pass of the same size and
                    // shape would repeat. With the arrangement and positions
                    // recorded, the artifact's own `kept` and `candidates`
                    // give the exact texts back, and the next occurrence is a
                    // bug someone can drive rather than a story about one.
                    unread.push(format!(
                        "pass {}/{} of the {} arrangement over kept positions {idx:?}: {e}",
                        i + 1,
                        sets.len(),
                        arrangement.name()
                    ));
                    mine_unread += 1;
                    last_err = Some(e);
                    continue;
                }
            };
            for p in got {
                // The same pair can surface in more than one pass. First
                // reason wins; a pair is one tension however often it is
                // noticed.
                if !mine.iter().any(|q| is_same(q, &p)) {
                    mine.push(p);
                }
            }
        }
        // And the fold across arrangements is the union itself. `added` is
        // the only number that can retire an arrangement: an arrangement that
        // adds nothing over several runs is pure cost, and this is where a
        // reader sees that without instrumenting anything.
        let mut report = ByArrangement {
            arrangement: arrangement.name().to_string(),
            passes: sets.len(),
            unread: mine_unread,
            proposed: mine.len(),
            added: 0,
        };
        for p in mine {
            if !out.iter().any(|q| is_same(q, &p)) {
                report.added += 1;
                out.push(p);
            }
        }
        arrangements.push(report);
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
    if arrangements.len() > 1 {
        for r in &arrangements {
            eprintln!(
                "  {:<11} {} pass(es), {} pair(s), {} no earlier arrangement had",
                r.arrangement, r.passes, r.proposed, r.added
            );
        }
    }
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
        arrangements,
    })
}

/// What one arrangement of the list was worth.
///
/// `added` is the load-bearing column: pairs this arrangement proposed that no
/// earlier one had. A union whose second arrangement adds nothing is paying
/// double for one comparison, and that has to be visible in the run itself
/// rather than reconstructed by whoever wonders.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ByArrangement {
    pub arrangement: String,
    pub passes: usize,
    pub unread: usize,
    /// Distinct pairs this arrangement proposed, its own repeats folded.
    pub proposed: usize,
    /// How many of those were new to the union at that point.
    pub added: usize,
}

/// What a comparison run weighed, and what it could not.
///
/// `passes` is what was attempted across every arrangement and `unread` what
/// came back unusable, so a caller can say what fraction of the pair space a
/// number covers instead of implying all of it.
#[derive(Debug, Default)]
pub struct Compared {
    pub pairs: Vec<Proposed>,
    pub passes: usize,
    pub unread: Vec<String>,
    /// One entry per arrangement, in the order they ran.
    pub arrangements: Vec<ByArrangement>,
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
