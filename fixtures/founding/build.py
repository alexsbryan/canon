#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Rebuild founding.md from the vendored transcripts in sources/.

No network and no model. Every word in the output is the enacting body's;
this file decides only which words are grouped under which heading, and
each grouping rule below quotes the marker it keys on.

    python3 fixtures/founding/build.py
"""
import html
import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
SRC = HERE / "sources"


def lines(name):
    """Vendored HTML -> the transcript's own lines, tags and chrome removed."""
    s = (SRC / f"{name}.html").read_text(encoding="utf-8", errors="replace")
    s = re.sub(r"(?is)<(script|style)[^>]*>.*?</\1>", " ", s)
    s = re.sub(r"(?i)<br\s*/?>", "\n", s)
    s = re.sub(r"(?i)</(p|div|h[1-6]|li|tr)>", "\n", s)
    s = re.sub(r"<[^>]+>", "", s)
    s = html.unescape(s).replace("\xa0", " ")
    out = [re.sub(r"[ \t]+", " ", l).strip() for l in s.split("\n")]
    # The Archives pages repeat every amendment heading in an "On This Page"
    # nav BELOW the transcript, and close with a site footer. Both parse as
    # document structure and neither is the document: the run picked up a
    # second, empty Amendment XXVII made of "Contact Us" and "En Espanol".
    # The transcript ends at the first "Back to ... Page" link.
    stop = next(
        (n for n, l in enumerate(out) if l.startswith("Back to ") and l.endswith("Page")),
        len(out),
    )
    return out[:stop]


def between(ls, start_pred, end_pred):
    i = next(n for n, l in enumerate(ls) if start_pred(l))
    j = next((n for n, l in enumerate(ls[i + 1 :], i + 1) if end_pred(l)), len(ls))
    return [l for l in ls[i:j] if l]


# ── the Declaration ─────────────────────────────────────────

def declaration():
    ls = lines("declaration-transcript")
    i = next(n for n, l in enumerate(ls) if "When in the Course" in l)
    # The transcript is one line per paragraph. The famous second paragraph
    # runs from "We hold these truths" to the end of the indictment's
    # preamble; the grievances are the run of lines opening "He has"/"He is"/
    # "For ", and the conclusion opens "We, therefore".
    body = [l for l in ls[i:] if l]
    end = next(n for n, l in enumerate(body) if l.startswith("We, therefore"))
    truths = body[1]
    griev = [l for l in body[2:end] if l]
    out = []
    out.append(("Declaration of Independence, Exordium", body[0]))
    out.append(("Declaration of Independence, Self-Evident Truths", truths))
    out.append(("Declaration of Independence, Grievances", "\n".join(griev)))
    # The conclusion paragraph alone. What follows it is the signature block,
    # grouped by colony — "Georgia", "North Carolina" — which is a list of
    # names and not a commitment anyone can extract.
    out.append(("Declaration of Independence, Conclusion", body[end]))
    return out


# ── the Articles of Confederation ───────────────────────────

ROMAN = r"(?:I|II|III|IV|V|VI|VII|VIII|IX|X|XI|XII|XIII)"

# Links the Archives prints between the transcript and its footer. Each is a
# whole line and none of them is a sentence, which is what makes them safe to
# match exactly rather than by prefix.
NAV = {
    "Amendments 11-27",
    "The U.S. Bill of Rights",
    "The Bill of Rights Transcript",
    "On This Page",
    "Constitution of the United States",
}


def articles_of_confederation():
    ls = lines("articles-of-confederation")
    # Avalon prints each article as a bare roman numeral on its own line,
    # e.g. "II." followed by the article's text.
    marks = [
        (n, re.fullmatch(rf"({ROMAN})\.", l).group(1))
        for n, l in enumerate(ls)
        if re.fullmatch(rf"{ROMAN}\.", l)
    ]
    out = []
    for k, (n, num) in enumerate(marks):
        end = marks[k + 1][0] if k + 1 < len(marks) else len(ls)
        text = [l for l in ls[n + 1 : end] if l]
        # Avalon closes the transcript with "Source:" and then its whole site
        # nav — century ranges, collection names. Left in, that landed
        # "4000bce - 399" inside Article XIII, which is one of the two
        # passages the unauthorised-founding pair rests on.
        cut = next((k for k, l in enumerate(text) if l.startswith("Source:")), len(text))
        text = [l for l in text[:cut] if not l.startswith("Avalon")]
        if text:
            out.append((f"Articles of Confederation, Article {num}", "\n".join(text)))
    return out


# ── the Constitution and its amendments ─────────────────────

def constitution():
    ls = lines("constitution-transcript")
    i = next(n for n, l in enumerate(ls) if l.startswith("We the People"))
    body = [l for l in ls[i:] if l]
    out = [("U.S. Constitution, Preamble", body[0])]
    art = None
    sec = None
    buf = []

    def flush():
        if art and buf:
            head = f"U.S. Constitution, Article {art}"
            if sec:
                head += f", Section {sec}"
            out.append((head, "\n".join(buf)))

    for l in body[1:]:
        # The parchment writes "Article. I." and "Section. 1." with points.
        m = re.fullmatch(rf"Article\.?\s*({ROMAN})\.?", l)
        if m:
            flush()
            art, sec, buf = m.group(1), None, []
            continue
        m = re.fullmatch(r"Section\.?\s*(\d+)\.?", l)
        if m:
            flush()
            sec, buf = m.group(1), []
            continue
        if l.startswith("Attest") or l.startswith("Go. Washington"):
            break
        if art:
            buf.append(l)
    flush()
    return out


def amendments():
    out = []
    for name, pat in (
        ("bill-of-rights-transcript", r"Amendment ([IVXL]+)"),
        ("amendments-11-27", r"AMENDMENT ([IVXL]+)"),
    ):
        ls = lines(name)
        marks = [
            (n, re.fullmatch(pat, l).group(1))
            for n, l in enumerate(ls)
            if re.fullmatch(pat, l)
        ]
        for k, (n, num) in enumerate(marks):
            end = marks[k + 1][0] if k + 1 < len(marks) else len(ls)
            body = [l for l in ls[n + 1 : end] if l]
            # The Archives prefixes each amendment with its passage and
            # ratification dates and, where one exists, a note naming the
            # article it modified. Both are the Archives' editorial matter,
            # not the amendment, so they are dropped from the text a reader
            # is asked to extract commitments from. `truth.py` reads the
            # notes straight out of the same file.
            # Cross-page links sit BETWEEN the last amendment and the
            # "Back to …" sentinel, so truncating at the sentinel is not
            # enough: "Amendments 11-27" landed inside the Tenth Amendment.
            nav = next(
                (k for k, l in enumerate(body) if l in NAV),
                len(body),
            )
            body = body[:nav]
            body = [
                l for l in body
                if not l.startswith("Note:")
                and not l.startswith("Passed by Congress")
                and not l.startswith("Originally proposed")
                and not l.startswith("Ratified")
                and "The 18th amendment" not in l
            ]
            if not body:
                continue
            # Amendments XIII, XIV, XV, XVIII, XX, XXI, XXII, XXV, XXVI and
            # XXVII carry numbered sections of their own.
            secs, cur, cbuf = [], None, []
            for l in body:
                m = re.fullmatch(r"Section\.?\s*(\d+)\.?", l)
                if m:
                    if cbuf:
                        secs.append((cur, cbuf))
                    cur, cbuf = m.group(1), []
                    continue
                cbuf.append(l)
            if cbuf:
                secs.append((cur, cbuf))
            for s, text in secs:
                head = f"U.S. Constitution, Amendment {num}"
                if s:
                    head += f", Section {s}"
                out.append((head, "\n".join(text)))
    return out


# ── section keys ────────────────────────────────────────────
#
# The key a heading scores under. Two levels, like the Des Moines corpus's
# `ord:<n>/<sec>`: the outer level is the enacting INSTRUMENT, because this
# corpus interleaves four of them and "Article II" names a different rule in
# each. The inner level is the citation the instrument uses for itself.

INSTRUMENTS = {
    "Declaration of Independence": "declaration",
    "Articles of Confederation": "articles",
    "U.S. Constitution": "constitution",
}


def key_of(heading):
    doc, _, rest = heading.partition(", ")
    slug = INSTRUMENTS.get(doc)
    if slug is None:
        return None
    m = re.fullmatch(rf"Article ({ROMAN})(?:, Section (\d+))?", rest)
    if m:
        return f"{slug}:{m.group(1)}" + (f".{m.group(2)}" if m.group(2) else "")
    m = re.fullmatch(r"Amendment ([IVXL]+)(?:, Section (\d+))?", rest)
    if m:
        return f"{slug}:amend.{m.group(1)}" + (f".{m.group(2)}" if m.group(2) else "")
    return f"{slug}:{rest.lower().replace(' ', '-')}"


ARABIC = {
    1: "I", 2: "II", 3: "III", 4: "IV", 5: "V", 6: "VI", 7: "VII", 8: "VIII",
    9: "IX", 10: "X", 11: "XI", 12: "XII", 13: "XIII", 14: "XIV", 15: "XV",
    16: "XVI", 17: "XVII", 18: "XVIII", 19: "XIX", 20: "XX", 21: "XXI",
    22: "XXII", 23: "XXIII", 24: "XXIV", 25: "XXV", 26: "XXVI", 27: "XXVII",
}

# Where a note names an amendment and not one of its sections, this says
# WHICH section carries the language that does the superseding, and quotes
# the words it was read off. That resolution is OURS, not the Archives'.
# Without it the pair names a key no passage in the corpus has, and the
# tension is unfindable by construction rather than by the model's failure.
OPERATIVE = {
    "XIII": ("1", "Neither slavery nor involuntary servitude"),
    "XIV": ("2", "Representatives shall be apportioned among the several States"),
    "XX": ("2", "The Congress shall assemble at least once in every year"),
    "XXI": ("1", "The eighteenth article of amendment to the Constitution"),
    "XXV": ("1", "In case of the removal of the President from office"),
    "XXVI": ("1", "The right of citizens of the United States, who are eighteen years"),
}


def amendment_key(num, section=None):
    """`XIII` -> `constitution:amend.XIII.1`, via OPERATIVE when unsectioned."""
    if section is None and num in OPERATIVE:
        section = OPERATIVE[num][0]
    return f"constitution:amend.{num}" + (f".{section}" if section else "")


# ── part A: the supersessions, read out of the Archives' own notes ──
#
# Every entry here is DERIVED. The National Archives prefixes each amendment
# with a note naming the article it modified or superseded; this reads those
# nine notes out of the same vendored HTML the corpus is built from and
# refuses to emit anything it cannot parse. Nobody planted these, and anyone
# can check them against archives.gov in a browser.

NOTE_LEFT = re.compile(
    r"(?:Article ([IVXL]+), section (\d+)"
    r"|Amendment (\d+), section (\d+)"
    # The 20th's second claim names a whole amendment and no section of it.
    # Anchored on "portion of the" so the amendment on the RIGHT of a
    # one-claim sentence can never be read as the thing being superseded.
    r"|portion of the (\d+)(?:st|nd|rd|th) amendment)",
    re.I,
)
# What did the superseding. A note's tail after "by" carries an optional
# section and an optional amendment; "by section 3." alone means section 3 of
# the amendment the note is printed under. At least one must be present, and a
# tail with neither is refused rather than guessed at.
NOTE_TAIL = re.compile(r"(?:superseded|modified|affected) by (.+?)\.?$", re.I)


def superseded_by(sentence, printed_under):
    m = NOTE_TAIL.search(sentence)
    if not m:
        return None
    tail = m.group(1)
    sec = re.search(r"section (\d+)", tail, re.I)
    amd = re.search(r"amendment (\d+)|the (\d+)(?:st|nd|rd|th) amendment", tail, re.I)
    if not sec and not amd:
        return None
    num = printed_under
    if amd:
        num = ARABIC[int(amd.group(1) or amd.group(2))]
    return amendment_key(num, sec.group(1) if sec else None)


def archives_notes():
    """(amendment numeral, note sentence) for every note the Archives prints."""
    ls = lines("amendments-11-27")
    out, cur = [], None
    for l in ls:
        m = re.fullmatch(r"AMENDMENT ([IVXL]+)", l)
        if m:
            cur = m.group(1)
        elif l.startswith("Note:") and cur:
            out.append((cur, l))
    return out


def supersessions():
    found = []
    for num, note in archives_notes():
        # A note may carry two claims: the 20th's names both Article I
        # section 4 and a portion of the 12th amendment.
        for sentence in re.split(r"(?<=\.)\s+(?=In addition)", note):
            left = NOTE_LEFT.search(sentence)
            b = superseded_by(sentence, num)
            if not left or not b:
                raise SystemExit(f"unparsed Archives note: {sentence!r}")
            art, sec, amd, asec, whole = left.groups()
            if art:
                a = f"constitution:{art}.{sec}"
            elif amd:
                a = amendment_key(ARABIC[int(amd)], asec)
            else:
                a = amendment_key(ARABIC[int(whole)])
            verb = re.search(r"(superseded|modified|affected)", sentence, re.I).group(1).lower()
            found.append({
                "type": "unmarked_supersession",
                "a": a,
                "b": b,
                "why": sentence.replace("Note: ", "").strip(),
                "source": "National Archives, Constitution of the United States: Amendments 11-27",
            })
    return found


# ── part B: tensions between a stated principle and a specific rule ──
#
# **These are OURS, and this is the line where that starts.** Part A is read
# out of the Archives' own notes; every entry below is our reading of two
# passages, and it is recorded here with both passages quoted so a reader can
# disagree with the reading rather than with a number. Where the contradiction
# is argued rather than plain on the face of the text, `contested` says so.
#
# The rule applied: a pair goes in only when both sides are QUOTED TEXT in
# this corpus. No entry rests on a document we did not vendor, and none rests
# on what anyone later said the words meant.

AUTHORED = [
    dict(
        id="P1", a="declaration:self-evident-truths", b="constitution:I.2",
        quote_a="all men are created equal … endowed by their Creator with certain unalienable Rights … Life, Liberty and the pursuit of Happiness",
        quote_b="adding to the whole Number of free Persons … three fifths of all other Persons",
        why="A universal claim of equality, and an apportionment rule that counts some persons as three fifths of one.",
        contested=False,
    ),
    dict(
        id="P2", a="declaration:self-evident-truths", b="constitution:IV.2",
        quote_a="unalienable Rights … Life, Liberty and the pursuit of Happiness",
        quote_b="No Person held to Service or Labour in one State … shall, in Consequence of any Law or Regulation therein, be discharged from such Service or Labour, but shall be delivered up",
        why="An inalienable right to liberty, and a duty on every State to return a person escaping from service.",
        contested=False,
    ),
    dict(
        id="P3", a="declaration:self-evident-truths", b="constitution:I.9",
        quote_a="unalienable Rights … Life, Liberty and the pursuit of Happiness",
        quote_b="The Migration or Importation of such Persons as any of the States now existing shall think proper to admit, shall not be prohibited by the Congress prior to the Year one thousand eight hundred and eight",
        why="The same right to liberty, and a clause forbidding Congress to end the slave trade for twenty years.",
        contested=False,
    ),
    dict(
        id="P4", a="articles:II", b="constitution:VI",
        quote_a="Each state retains its sovereignty, freedom, and independence, and every power, jurisdiction, and right, which is not by this Confederation expressly delegated",
        quote_b="This Constitution, and the Laws of the United States … shall be the supreme Law of the Land; and the Judges in every State shall be bound thereby, any Thing in the Constitution or Laws of any State to the Contrary notwithstanding",
        why="Retained state sovereignty, and a supremacy clause binding state judges against their own state's law.",
        contested=False,
    ),
    dict(
        id="P5", a="articles:XIII", b="constitution:VII",
        quote_a="nor shall any alteration at any time hereafter be made in any of them; unless such alteration be agreed to in a Congress of the United States, and be afterwards confirmed by the legislatures of every State",
        quote_b="The Ratification of the Conventions of nine States, shall be sufficient for the Establishment of this Constitution between the States so ratifying the Same",
        why="Alteration required every legislature; the replacement declared itself established on nine conventions. The founding act is not authorised by the instrument it replaces.",
        contested=False,
    ),
    dict(
        id="P6", a="constitution:I.8", b="constitution:amend.X",
        quote_a="To make all Laws which shall be necessary and proper for carrying into Execution the foregoing Powers",
        quote_b="The powers not delegated to the United States by the Constitution, nor prohibited by it to the States, are reserved to the States respectively, or to the people",
        why="A power to make whatever laws are necessary and proper, and a reservation of everything not delegated. Which one bounds the other is the oldest argument in American constitutional law.",
        contested=True,
    ),
]

# Pairs a reader might flag and that are NOT tensions. Chosen to be hard: four
# of the six are near-identical wording applied to different subjects, which is
# the shape that folded a real Des Moines rule out of existence.
DECOYS = [
    dict(id="N1", a="constitution:I.9", b="constitution:I.10",
         why="compatible: both forbid bills of attainder and ex post facto laws, one to the Congress and one to the States. Same limit, different party bound."),
    dict(id="N2", a="constitution:amend.XIII.2", b="constitution:amend.XV.2",
         why="compatible: two amendments carrying the same enforcement sentence for different articles."),
    dict(id="N3", a="constitution:amend.XIV.5", b="constitution:amend.XXVI.2",
         why="compatible: the same enforcement sentence again, in two more amendments."),
    dict(id="N4", a="articles:IV", b="constitution:IV.1",
         why="compatible: full faith and credit is restated from the Articles into the Constitution. A rule and its re-enactment are not a tension."),
    dict(id="N5", a="constitution:amend.IV", b="constitution:amend.V",
         why="compatible: unreasonable searches and compelled self-incrimination are different protections, neither limiting the other."),
    dict(id="N6", a="constitution:I.8", b="constitution:I.10",
         why="compatible: Congress may coin money and the States may not. An allocation, not a conflict."),
]

# The 21st repeals the 18th in its own words rather than in an Archives note,
# so it is read out of the amendment's text and not out of the annotation.
REPEAL = dict(
    id="S11", a="constitution:amend.XVIII.1", b="constitution:amend.XXI.1",
    quote="The eighteenth article of amendment to the Constitution of the United States is hereby repealed.",
)


def emit_truth(sections):
    have = {key_of(h) for h, _ in sections}
    have.discard(None)
    planted, seen = [], set()

    def add(entry, kind, split):
        if entry["a"] not in have or entry["b"] not in have:
            raise SystemExit(f"{entry.get('id','?')}: key not in corpus: {entry['a']} / {entry['b']}")
        pair = tuple(sorted((entry["a"], entry["b"])))
        if pair in seen:
            raise SystemExit(f"duplicate pair {pair}")
        seen.add(pair)
        out = {"id": entry.get("id") or f"S{len(planted) + 1}", "type": kind, "split": split}
        out.update({k: v for k, v in entry.items() if k not in ("id", "type", "split")})
        planted.append(out)

    for t in supersessions():
        add(t, "unmarked_supersession", "train")
    add({**REPEAL, "why": "The 21st repeals the 18th in its own words, and neither is struck from the document.",
         "source": "U.S. Constitution, Amendment XXI, Section 1"}, "unmarked_supersession", "train")
    for t in AUTHORED:
        add(t, "principle_vs_rule", "test")

    members = sorted({k for t in planted for k in (t["a"], t["b"])}
                     | {k for d in DECOYS for k in (d["a"], d["b"])})
    for d in DECOYS:
        if d["a"] not in have or d["b"] not in have:
            raise SystemExit(f"{d['id']}: key not in corpus")

    return {
        "corpus_id": "founding",
        "note": (
            "Ground truth for the founding documents of the United States: the Declaration of "
            "Independence, the Articles of Confederation, and the Constitution with all twenty-seven "
            "amendments, as transcribed by the National Archives and the Avalon Project. Every "
            "`unmarked_supersession` entry is DERIVED, not authored: the Archives prints a note under "
            "each amendment naming the article it modified or superseded, and build.py parses those "
            "notes out of the same vendored HTML the corpus is built from, refusing anything it cannot "
            "parse. The `principle_vs_rule` entries are OURS and each one quotes both passages so the "
            "reading can be argued with. Nothing here rests on a document that is not vendored."
        ),
        "schema_version": 2,
        # The enacting instruments this corpus interleaves. The scorer reads
        # this rather than carrying the vocabulary in code: "Article II" names
        # a different rule in each of them, and no parser can tell which
        # without being told which documents are in play.
        "instruments": INSTRUMENTS,
        "exhaustive": False,
        "exhaustive_within": {
            "region": (
                "every pair drawn from the sections the Archives annotates as superseded or "
                "modified, the amendments that did so, and the passages named in the authored "
                "entries and decoys below"
            ),
            "members": members,
        },
        "tension_types": ["unmarked_supersession", "principle_vs_rule"],
        "planted_tensions": [
            {k: v for k, v in t.items() if k != "contested"} | (
                {"contested": True} if t.get("contested") else {}
            ) for t in planted
        ],
        "expected_non_tensions": [dict(d, split="test") for d in DECOYS],
    }


# ── what must survive extraction ────────────────────────────
#
# For each planted tension, the phrase on each side that CARRIES it. A reading
# that lost the phrase cannot find the tension by any amount of comparison, and
# a comparison-stage number computed over such a candidate set is measuring the
# wrong stage. Every phrase is checked verbatim against the section it names on
# every build, so an anchor cannot quietly stop naming anything.
#
# Chosen as the shortest fragment that is DISTINCTIVE to the passage: what the
# amendment changed, or the clause the principle collides with.
#
# **Short on purpose, because the haystack is the extractor's PARAPHRASE and
# not the source.** An anchor spanning 18th-century punctuation reports a real
# extraction as a miss: the parchment says "Life, Liberty and the pursuit of
# Happiness" and the first smoke run came back "Life, Liberty, and the pursuit
# of Happiness", one Oxford comma away from scoring the Declaration as unread.
# Every phrase here is a semantic invariant — a name, a measure, a verb the
# rule turns on — rather than a span whose wording a reader could restyle.

ANCHOR = {
    "S1":  ("Citizens of another State", "shall not be construed to extend to any suit"),
    "S2":  ("vote by Ballot for two Persons", "vote by ballot for President and Vice-President"),
    "S3":  ("No Person held to Service or Labour", "Neither slavery nor involuntary servitude"),
    "S4":  ("three fifths of all other Persons", "counting the whole number of persons"),
    "S5":  ("Capitation", "taxes on incomes"),
    "S6":  ("chosen by the Legislature thereof", "elected by the people thereof"),
    "S7":  ("first Monday in December", "day of January"),
    "S8":  ("shall choose immediately", "Vice President elect shall become President"),
    "S9":  ("devolve on the Vice President", "Vice President shall become President"),
    "S10": ("the right to vote at any election", "eighteen years of age"),
    "S11": ("intoxicating liquors", "is hereby repealed"),
    "P1":  ("all men are created equal", "three fifths of all other Persons"),
    "P2":  ("pursuit of Happiness", "shall be delivered up"),
    "P3":  ("pursuit of Happiness", "Migration or Importation"),
    "P4":  ("retains its sovereignty", "supreme Law of the Land"),
    "P5":  ("legislatures of every State", "Conventions of nine States"),
    "P6":  ("necessary and proper", "reserved to the States"),
}


def emit_anchors(sections, truth):
    text = {key_of(h): " ".join(t.split()) for h, t in sections}
    out, missing = {}, []
    for t in truth["planted_tensions"]:
        pair = ANCHOR.get(t["id"])
        if not pair:
            missing.append(f"{t['id']}: no anchor")
            continue
        entry = []
        for side, phrase in zip(("a", "b"), pair):
            if phrase.lower() not in text.get(t[side], "").lower():
                missing.append(f"{t['id']} {side} ({t[side]}): {phrase!r}")
            entry.append({"section": t[side], "must": [[phrase]]})
        out[t["id"]] = entry
    if missing:
        raise SystemExit("anchors do not name text in the corpus:\n  " + "\n  ".join(missing))
    return {
        "note": (
            "What must survive extraction for each planted tension to be FINDABLE AT ALL. Each "
            "anchor is the phrase that carries the tension on its side — what the amendment "
            "changed, or the clause the principle collides with — taken from the documents and "
            "not from anything an extractor produced. Matching is case-insensitive over collapsed "
            "whitespace against any candidate drawn from that section. build.py refuses to write "
            "this file if a phrase does not appear in the section it names."
        ),
        "schema_version": 1,
        "anchors": out,
    }


def verify_quotes(sections, truth):
    """Every quoted fragment must appear VERBATIM in the section it names.

    Run on every build, because a manifest whose quotes have drifted from the
    corpus still reads as authoritative. The ellipsis is ours; each side of it
    is the document's own words and is checked as such.
    """
    text = {key_of(h): " ".join(t.split()) for h, t in sections}
    missing = []
    for t in truth["planted_tensions"]:
        for side in ("a", "b"):
            q = t.get(f"quote_{side}") or (t.get("quote") if side == "b" else None)
            for frag in [" ".join(f.split()) for f in (q or "").split("…") if f.strip()]:
                if frag not in text.get(t[side], ""):
                    missing.append(f"{t['id']} {side} ({t[side]}): {frag[:60]!r}")
    if missing:
        raise SystemExit("quoted text is not in the corpus:\n  " + "\n  ".join(missing))
    return sum(
        1 for t in truth["planted_tensions"]
        for s_ in ("a", "b")
        if t.get(f"quote_{s_}") or (t.get("quote") if s_ == "b" else None)
    )


def main():
    sections = declaration() + articles_of_confederation() + constitution() + amendments()
    doc = [
        "<!-- Rebuilt by fixtures/founding/build.py from sources/. Do not edit by hand. -->",
        "",
    ]
    for head, text in sections:
        doc.append(f"# {head}")
        doc.append("")
        doc.append(text)
        doc.append("")
    (HERE / "founding.md").write_text("\n".join(doc), encoding="utf-8")
    truth = emit_truth(sections)
    quoted = verify_quotes(sections, truth)
    (HERE / "truth.json").write_text(json.dumps(truth, indent=2) + "\n", encoding="utf-8")
    anchors = emit_anchors(sections, truth)
    (HERE / "extraction-anchors.json").write_text(
        json.dumps(anchors, indent=2) + "\n", encoding="utf-8"
    )
    print(
        f"{len(truth['planted_tensions'])} planted "
        f"({sum(1 for t in truth['planted_tensions'] if t['type'] == 'unmarked_supersession')} derived, "
        f"{sum(1 for t in truth['planted_tensions'] if t['type'] == 'principle_vs_rule')} authored), "
        f"{len(truth['expected_non_tensions'])} decoys, "
        f"region {len(truth['exhaustive_within']['members'])} members, "
        f"{quoted} quoted passages verified verbatim"
    )
    words = sum(len(t.split()) for _, t in sections)
    print(f"{len(sections)} sections, {words} words -> {HERE / 'founding.md'}")
    for head, text in sections[:3] + sections[-2:]:
        print(f"  {head:58} {len(text.split()):5} words")
    return 0


if __name__ == "__main__":
    sys.exit(main())
