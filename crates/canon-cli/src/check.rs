// SPDX-License-Identifier: AGPL-3.0-or-later
//! `canon check` — the adjudication, and the three ways of saying it.
//!
//! One model call. Every live commitment goes into context and the answer
//! comes back as bearings: which commitments touch this proposal, which way
//! each pulls, and why. [`Standing::cited`] then refuses any position naming a
//! commitment this canon does not have or giving no reason, so a rendered
//! conflict always points at a rule a person can read.
//!
//! **The profiles are not cosmetic.** `code` renders a verdict because a
//! codebase wants one and CI reads exit codes. `house` renders which ACT the
//! proposal needs, because a household's output is an agenda. `personal`
//! renders stakes and **never a verdict, never exit 1** — a tool that ruled
//! on someone's inner life would do harm the codebase profile cannot, and
//! that is enforced here by [`exit_code`] and pinned by a test.

use canon_core::{
    Attributes, Authority, Canon, Commitment, Decision, Disposition, Outcome, Policy, Position,
    Pull, Scope, Standing, Status,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::model::{self, Client, ModelError, Schema};
use crate::profile::Profile;
use crate::resolver::Offered;
use crate::store;

const SYSTEM: &str = "\
You judge how a proposal sits against a body of commitments.

For every commitment that bears on the proposal, return:
- commitment: its number
- pull: \"against\" if acting on the proposal would break or strain it, \
\"toward\" if the proposal serves it
- because: one sentence naming what in the proposal touches what in the \
commitment

Rules:
- Only commitments that genuinely bear on the proposal. Most will not.
- Never cite a commitment you cannot give a reason for.
- If nothing bears on the proposal, return an empty list. That is an answer.
- Judge the proposal as written. Do not improve it.";

/// The model's answer shape.
///
/// **Still called `bearings` on the wire, deliberately.** A `Position` may be
/// sourced from a commitment or from an actor; this reader only ever produces
/// the first kind, because reading a canon is the only thing it does. Renaming
/// the schema key would change the prompt, and the prompt is measured. The two
/// names are not one concept spelled twice: a bearing is what a commitment has
/// on a proposal, and it is a strict subset of what a position can be.
#[derive(Debug, Deserialize)]
struct Judged {
    #[serde(default)]
    bearings: Vec<JudgedOne>,
}

#[derive(Debug, Deserialize)]
struct JudgedOne {
    commitment: crate::model::Pos,
    #[serde(default)]
    pull: String,
    #[serde(default)]
    because: String,
}

/// Written in sorted order, the order this stage was decoded in before
/// [`Schema`] existed and so the order it was measured in. Moving a property
/// is a measured change, not a tidy-up.
pub(crate) const SCHEMA: Schema = Schema::new(
    r#"{
  "type": "object",
  "properties": {
    "bearings": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "because": { "type": "string" },
          "commitment": { "type": "integer", "description": "number of the commitment" },
          "pull": { "type": "string", "enum": ["toward", "against"] }
        },
        "required": ["commitment", "pull", "because"],
        "additionalProperties": false
      }
    }
  },
  "required": ["bearings"],
  "additionalProperties": false
}"#,
);

/// Ask how this proposal stands. Returns the standing and anything refused.
///
/// The standing is what the canon says AND what people said about `about`
/// with `canon object --about`: a live check that saw only the rules would
/// leave every rule that counts people deciding nothing outside `replay`.
pub fn assess(
    client: &Client,
    canon: &Canon,
    proposal: &str,
    about: &str,
) -> Result<(Standing, Vec<Position>), ModelError> {
    let mut positions = canon.said_about(about);
    let live: Vec<&Commitment> = canon.active().collect();
    // The same numbering discipline as every other reader here, from the one
    // place that owns it — see `crate::resolver`. `Offered::at` never clamps:
    // clamping attributes a conflict to whichever rule happens to be last.
    let offered = Offered::new(
        live.iter().map(|c| c.text().to_string()).collect(),
        "commitment",
    );
    if offered.is_empty() {
        return Ok(Standing::cited(canon, proposal, positions));
    }
    let user = format!(
        "Commitments:\n{}\nProposal:\n{proposal}\n",
        offered.numbered()
    );
    let judged: Judged = client.complete_json(SYSTEM, &user, "bearings", &SCHEMA)?;
    for b in judged.bearings {
        let Some(c) = b
            .commitment
            .get()
            .and_then(|n| offered.at(n))
            .and_then(|i| live.get(i))
        else {
            continue;
        };
        let pull = match b.pull.trim().to_ascii_lowercase().as_str() {
            "against" => Pull::Against,
            "toward" | "towards" | "for" => Pull::Toward,
            other => {
                eprintln!("warning: dropped a position with an unreadable pull `{other}`");
                continue;
            }
        };
        positions.push(Position::of(c.id().clone(), pull, b.because.trim()));
    }
    // Counted, not merely warned about: "the reader cited past the end nine
    // times" is a measurement about this pass, not noise to swallow.
    if offered.refused() > 0 {
        eprintln!(
            "warning: {} reading(s) named a commitment that was not offered",
            offered.refused()
        );
    }
    Ok(Standing::cited(canon, proposal, positions))
}

/// The exit-code contract, decided in ONE place.
///
/// **It reports the OUTCOME, not the authority.** CI and agents already read
/// these three codes, and a policy that refuses is louder on stdout than a
/// changed exit code would be useful — moving the contract onto the authority
/// ladder would silently redefine what `canon check` means for every existing
/// caller.
///
/// `Personal` returns 0 whatever the outcome. That is the invariant, not a
/// default: a personal canon reports stakes, and an exit 1 is a machine
/// saying a life choice failed a check.
pub fn exit_code(profile: Profile, outcome: Outcome) -> i32 {
    match (profile, outcome) {
        (Profile::Personal, _) => 0,
        (_, Outcome::Supported) => 0,
        (_, Outcome::Conflicts) => 1,
        (_, Outcome::Unaddressed) => 2,
    }
}

fn cite(canon: &Canon, b: &Position, indent: &str) -> String {
    // A position an ACTOR took cites no commitment, so there is no rule to
    // quote — but there is a person and a reason, and dropping them would be
    // the renderer quietly showing less than the canon holds. It gets the
    // same two lines in the same shape: who, which way, and why.
    let Some(c) = b.commitment().and_then(|id| canon.get(id)) else {
        let Some(actor) = b.actor() else {
            return String::new();
        };
        let way = match b.pull {
            Pull::Against => "objects",
            Pull::Toward => "supports",
        };
        return format!(
            "{indent}{actor}  {way}\n{}\n",
            crate::wrap::hang(
                &format!("{indent}{}because: ", " ".repeat(actor.len() + 2)),
                &b.because
            ),
        );
    };
    let status = match &c.status() {
        Status::Active => "in force, never amended".to_string(),
        Status::Superseded { by } => format!("amended by {by}"),
        Status::Retracted { at } => format!("repealed {}", store::ymd(*at)),
        Status::Proposed { needs } => format!("proposed, not yet a rule — needs {needs}"),
        Status::Refused { at, by, .. } => format!(
            "refused{}",
            crate::explain::refused_by(by, Some(*at), |b| b)
        ),
    };
    let pad = " ".repeat(c.id().as_str().len() + 2);
    format!(
        "{}\n{indent}{pad}asserted {}, {status}\n{}\n",
        crate::wrap::hang(
            &format!("{indent}{}  \"", c.id()),
            &format!("{}\"", c.text())
        ),
        store::ymd(c.asserted_at()),
        crate::wrap::hang(&format!("{indent}{pad}because: "), &b.because),
    )
}

/// Contradictions among the cited commitments that someone already chose to
/// carry. Without this the personal profile re-litigates a decision its user
/// already made, which is the one thing it must not do.
fn carried(canon: &Canon, standing: &Standing) -> Vec<String> {
    let mut out = Vec::new();
    for c in canon.tolerated() {
        let both = standing
            .cited_commitments()
            .any(|p| p.commitment() == Some(&c.a))
            && standing
                .cited_commitments()
                .any(|p| p.commitment() == Some(&c.b));
        if !both {
            continue;
        }
        if let Disposition::Tolerated { rationale, revisit } = &c.disposition {
            let when = store::ymd(c.at);
            out.push(match revisit {
                Some(r) => format!("accepted {when}: \"{rationale}\" (revisit by {r})"),
                None => format!("accepted {when}: \"{rationale}\""),
            });
        }
    }
    out
}

/// The words a profile uses. **Voice, not policy.**
///
/// This table is the whole of what used to be three renderers. `code` renders
/// a verdict because a codebase wants one and CI reads exit codes; `house`
/// renders which ACT the proposal needs, because a household's output is an
/// agenda. Those are two vocabularies for one decision, and once the decision
/// moved into [`canon_core::policy`] there was nothing left in the second
/// renderer except the words — so it became a table and the branch it lived
/// in went away.
///
/// The `personal` profile is deliberately NOT in this table. It renders no
/// verdict at all, which is not a third vocabulary but a different shape, and
/// forcing it through a verdict renderer to save a function is how the
/// invariant that protects it would eventually be lost.
struct Voice {
    conflict: &'static str,
    conflict_lead: &'static str,
    supported: &'static str,
    supported_lead: &'static str,
    unaddressed: &'static str,
    unaddressed_lead: &'static str,
    also: &'static str,
    /// After each opposing citation, name the acts that would settle it.
    offer_acts: bool,
    /// What to do about a gap. `{p}` is the proposal.
    gap: &'static str,
}

const CODE: Voice = Voice {
    conflict: "CONFLICT",
    conflict_lead: "",
    supported: "SUPPORTED",
    supported_lead: "",
    unaddressed: "UNADDRESSED",
    unaddressed_lead: "  nothing in this canon bears on this proposal.",
    also: "\nalso bears on it, in favour:\n",
    offer_acts: false,
    gap: "  record the gap:  canon question \"{p}\"\n",
};

const HOUSE: Voice = Voice {
    conflict: "THIS NEEDS AN AMENDMENT",
    conflict_lead: "  it runs against a rule the house already has:\n",
    supported: "ALREADY COVERED",
    supported_lead: "  no act needed; the house has already decided this:\n",
    unaddressed: "THIS NEEDS A NEW RULE",
    unaddressed_lead: "  nothing the house has decided bears on it.\n",
    also: "\nalso bears on it, in favour:\n",
    offer_acts: true,
    gap: "  write one:  canon add \"<the rule>\"\n  \
          or record the gap for the next meeting:  canon question \"{p}\"\n",
};

pub fn render(
    profile: Profile,
    canon: &Canon,
    standing: &Standing,
    decision: &Decision,
    attrs: &Attributes,
) -> String {
    match profile {
        // Never a verdict, and never routed through one.
        Profile::Personal => render_stakes(canon, standing),
        Profile::Code => render_verdict(&CODE, canon, standing, decision, attrs),
        Profile::House => render_verdict(&HOUSE, canon, standing, decision, attrs),
    }
}

/// A verdict's first lines: what it is, then its lead when the voice has one.
fn head(out: &mut String, verdict: &str, lead: &str) {
    out.push_str(&format!("{verdict}\n{lead}"));
    if !lead.is_empty() {
        out.push('\n');
    }
}

/// The one place an outcome becomes words.
fn render_verdict(
    v: &Voice,
    canon: &Canon,
    standing: &Standing,
    decision: &Decision,
    attrs: &Attributes,
) -> String {
    let mut out = String::new();
    match decision.outcome {
        Outcome::Conflicts => {
            head(&mut out, v.conflict, v.conflict_lead);
            for b in standing.against() {
                out.push_str(&cite(canon, b, "  "));
                if v.offer_acts {
                    out.push_str(&settle(canon, b));
                }
            }
            let toward: Vec<&Position> = standing.toward().collect();
            if !toward.is_empty() {
                out.push_str(v.also);
                for b in toward {
                    out.push_str(&cite(canon, b, "  "));
                }
            }
        }
        Outcome::Supported => {
            head(&mut out, v.supported, v.supported_lead);
            for b in standing.toward() {
                out.push_str(&cite(canon, b, "  "));
            }
        }
        Outcome::Unaddressed => {
            // Not approval. The canon is silent, and silence is reported as
            // silence — whatever the policy then permits.
            // A subject somebody left unwritten ON PURPOSE is not a gap, and
            // prompting for a new rule here is how a tool turns a working
            // unwritten practice into a rota nobody wanted.
            if let Some(s) = canon.silence_about(&attrs.about) {
                out.push_str("UNWRITTEN ON PURPOSE\n");
                out.push_str(&format!(
                    "  \"{}\" is left unwritten deliberately.\n",
                    s.about
                ));
                out.push_str(&format!("  {}\n", s.rationale));
                out.push_str(&format!(
                    "  decided {} by {} — and revisitable like anything else:\n    canon undo {}\n",
                    store::ymd(s.at),
                    s.actor,
                    s.act
                ));
            } else {
                head(&mut out, v.unaddressed, v.unaddressed_lead);
                out.push_str(&v.gap.replace("{p}", &standing.proposal));
            }
        }
    }
    out.push_str(&next_step(canon, decision, attrs));
    // Exactly one trailing newline. The per-citation affordances end with a
    // blank line so they separate from each other, which used to be followed
    // by the authority line and now sometimes ends the output.
    format!("{}\n", out.trim_end())
}

/// Whom to ask, when the answer is to ask: the people holding the narrowest
/// held scope that covers the action, or the canon as a whole when it names
/// none. Empty when nobody holds anything, and then any person can answer.
///
/// `None` when the answer is not to ask. An "ask" that names nobody sends a
/// person looking for whoever informally decides, which is the gatekeeper
/// `canon who` exists to make unnecessary.
pub fn ask<'a>(canon: &'a Canon, decision: &Decision, attrs: &Attributes) -> Option<Vec<&'a str>> {
    matches!(decision.authority, Authority::AskOne | Authority::AskPanel).then(|| {
        let held = canon.holders_at(attrs.scope.as_ref(), attrs.now);
        held.into_iter().collect()
    })
}

/// What you may do now, said every time: go ahead, or whom to ask.
///
/// **The rule that said so prints only once this community adopted one.**
/// Under the shipped default the authority is a pure function of the
/// outcome, so naming the rule restates the verdict in vocabulary a reader
/// has never met: a fresh house canon once told housemates to "ask one
/// person with standing" when nobody had been granted standing. The step
/// itself is not vocabulary — "ask human:alex first" is an instruction — so
/// it always prints. The moment somebody runs `canon policy set`, the rule
/// is the thing the group decided, and it prints even when it agrees,
/// because a rule invisible whenever it agrees is one nobody notices they
/// are governed by. `--json` carries both either way.
///
/// An ask ends with the command the person asked runs to answer, so their
/// ruling is a `decided` act a graduated rule can count, not a word in a
/// hallway nobody can find again.
fn next_step(canon: &Canon, decision: &Decision, attrs: &Attributes) -> String {
    let asked = ask(canon, decision, attrs);
    let record = match asked {
        Some(_) => format!(
            "  they answer:  canon decide \"{}\" allow|deny -m \"<why>\"\n",
            attrs.about
        ),
        None => String::new(),
    };
    let what = match (asked, decision.authority) {
        (Some(p), _) if p.is_empty() => {
            "ask a person first: nobody is in charge of this, so anyone can answer".into()
        }
        (Some(p), Authority::AskOne) if p.len() > 1 => format!("ask one of {} first", p.join(", ")),
        (Some(p), _) => format!("ask {} first", p.join(", ")),
        (None, Authority::Act) => "go ahead".into(),
        (None, a) => a.prose().to_string(),
    };
    match canon.policies().is_empty() {
        true => format!("\n{what}\n{record}"),
        false => format!("\n{what}\n  {}\n{record}", decision.because),
    }
}

/// The acts that would settle an opposing citation. A household's agenda.
fn settle(canon: &Canon, b: &Position) -> String {
    let Some(c) = b.commitment().and_then(|id| canon.get(id)) else {
        return String::new();
    };
    format!(
        "  amend it:  canon amend {} \"<the new rule>\" -m \"<why>\"\n  \
         or carry both knowingly:  canon accept {} <other> -m \"<what this protects>\"\n\n",
        c.id(),
        c.id()
    )
}

fn render_stakes(canon: &Canon, standing: &Standing) -> String {
    // Never a verdict. No CONFLICT, no SUPPORTED, no ruling — what has a
    // stake in this, and which way each pulls.
    let mut out = String::new();
    if standing.positions.is_empty() {
        out.push_str("  NOTHING WITH A STAKE\n");
        out.push_str("    none of your commitments speak to this either way.\n");
        return out;
    }
    out.push_str("  STAKE\n");
    let width = standing
        .positions
        .iter()
        .filter_map(|b| b.commitment().and_then(|id| canon.get(id)))
        .map(|c| c.text().chars().count())
        .max()
        .unwrap_or(0)
        .min(52);
    for b in &standing.positions {
        let Some(c) = b.commitment().and_then(|id| canon.get(id)) else {
            continue;
        };
        let arrow = match b.pull {
            Pull::Against => "pulls against",
            Pull::Toward => "pulls toward",
        };
        let quoted = format!("\"{}\"", c.text());
        out.push_str(&format!(
            "    {}  {:<width$}  <- {arrow}\n",
            c.id(),
            quoted,
            width = width + 2
        ));
        // Aligned under the commitment text, not under its id: the reason
        // belongs to the words, and a reader scans the left edge of the text.
        out.push_str(&format!(
            "    {}  {}\n",
            " ".repeat(c.id().as_str().len()),
            b.because
        ));
    }
    for line in carried(canon, standing) {
        out.push_str(&format!("    {line}\n"));
    }
    out
}

/// The `--json` payload.
///
/// The personal profile's payload carries NO `outcome` field. An outcome is a
/// verdict whichever way it is serialized, and "never renders a verdict" has
/// to hold for the machine-readable surface too or it does not hold.
pub fn payload(
    profile: Profile,
    canon: &Canon,
    standing: &Standing,
    decision: &Decision,
    attrs: &Attributes,
) -> Value {
    let mut v = json!({
        "proposal": standing.proposal,
        "profile": profile.as_str(),
        "positions": standing.positions,
    });
    if let Some(s) = canon.silence_about(&attrs.about) {
        v["silence"] = serde_json::to_value(s).unwrap_or(Value::Null);
    }
    if profile != Profile::Personal {
        v["outcome"] = serde_json::to_value(decision.outcome).unwrap_or(Value::Null);
        v["authority"] = serde_json::to_value(decision.authority).unwrap_or(Value::Null);
        v["because"] = json!(decision.because);
        if let Some(people) = ask(canon, decision, attrs) {
            v["ask"] = json!(people);
        }
    }
    v
}

/// What the proposal is, beyond its words. The MCP tool asks through this
/// too, so an agent and a person asking about the same action in the same
/// room are judged under the same rule.
///
/// Effect classification is the caller's job — here, the person typing the
/// command. `canon` never infers that a thing is irreversible, because a
/// library that guessed would be wrong in exactly the cases the guess
/// matters.
pub fn attributes(args: &[String], proposal: &str) -> Result<Attributes, String> {
    let mut attrs = Attributes::about(crate::cmds::flag(args, "--about").unwrap_or(proposal))
        .by(crate::store::actor())
        .at(crate::store::now());
    if let Some(raw) = crate::cmds::flag(args, "--area") {
        let scope = Scope::new(raw).ok_or_else(|| {
            format!("`{raw}` is not an area: a dotted path like `house.kitchen`, no empty parts")
        })?;
        attrs = attrs.in_scope(scope);
    }
    if crate::cmds::has(args, "--irreversible") {
        attrs = attrs.reversible(false);
    }
    if let Some(id) = crate::cmds::flag(args, "--amends") {
        attrs = attrs.amending(canon_core::ActId::from_raw(id));
    }
    Ok(attrs)
}

pub fn run(args: &[String]) -> i32 {
    // `--about` without a proposal is the command `canon silence` prints
    // (DESIGN: a silence is not a gap), so the subject is the proposal.
    let Some(proposal) = crate::cmds::positionals(args)
        .first()
        .copied()
        .or_else(|| crate::cmds::flag(args, "--about"))
    else {
        return crate::cmds::fail("usage: canon check \"<proposal>\"");
    };
    let (dir, _, canon) = match crate::cmds::load() {
        Ok(v) => v,
        Err(e) => return crate::cmds::fail(e),
    };
    let profile = match Profile::load(&dir) {
        Ok(p) => p,
        Err(e) => return crate::cmds::fail(e),
    };
    if canon.active().next().is_none() {
        eprintln!(
            "this canon has no live {} — nothing to check against.\n  canon add \"<the first one>\"",
            profile.nouns()
        );
        return 2;
    }
    // Before the endpoint, not after: a mistyped scope should cost nothing.
    let attrs = match attributes(args, proposal) {
        Ok(a) => a,
        Err(e) => return crate::cmds::fail(e),
    };
    let client = match model::client_for(&dir, crate::cmds::has(args, "--allow-remote")) {
        Ok(c) => c,
        Err(e) => return model::report(e),
    };
    eprintln!("checking on {}", client.describe());
    let (standing, refused) = match assess(&client, &canon, proposal, &attrs.about) {
        Ok(v) => v,
        Err(e) => return model::report(e),
    };
    // Refusals are named, not swallowed: a shorter answer with no explanation
    // is indistinguishable from a canon that had less to say.
    for b in &refused {
        // The reason has to match what was actually checked. A drop message
        // built from a different field than the comparison is how a real
        // mismatch renders as nothing and hides itself — the support step
        // shipped exactly that defect once.
        let (what, missing) = match &b.source {
            canon_core::Source::Commitment(id) => {
                (id.to_string(), "not a commitment in this canon")
            }
            canon_core::Source::Actor(a) => (format!("`{a}`"), "no actor named"),
        };
        let why = match b.because.trim() {
            "" => "no reason given",
            _ => missing,
        };
        eprintln!("warning: refused an uncitable position on {what} — {why}");
    }
    // The canon's own rule, from the canon, for this scope. A canon that
    // adopted nothing decides by what shipped — and says so.
    let rule = canon.policy_for(attrs.scope.as_ref()).clone();
    let decision = rule.decide(&standing, &attrs, &canon);
    if crate::cmds::has(args, "--json") {
        let v = payload(profile, &canon, &standing, &decision, &attrs);
        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
    } else {
        print!("{}", render(profile, &canon, &standing, &decision, &attrs));
        if let Some(note) = crate::cmds::carried_note(&canon) {
            eprintln!("\n{note}");
        }
    }
    exit_code(profile, decision.outcome)
}

#[cfg(test)]
mod tests;
