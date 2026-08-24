#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build the Des Moines noise corpus from vendored council documents.

The document interleaves Article IV of the Des Moines municipal code as
codified in 2008 with two ordinances that later amended it. That interleaving
is a CONSTRUCTED VIEW — the city never published one document containing both
readings — but nothing in it is written by us, and the labels are not our
opinion either: the council itself said which section each ordinance amends,
and the measure diff between the two readings is mechanical.

Labelling rule, applied by this script and by nothing else:

  a permit SUBJECT present in both the codified article and an amending
  ordinance, whose restatement CHANGES any stated measure (sound level or its
  weighting, distance, hours, counts, days)      -> planted tension
  ...whose stated measures are all identical     -> expected non-tension
  a subject only an ordinance states             -> an addition, not paired
  a base section no ordinance here amends        -> expected non-tension,
                                                    paired for shared vocabulary

The unit is the SUBJECT, not the type letter. This script paired by letter
until 2026-08-22 and it missed a whole supersession: the codified Type "F"
covers the Simon Estes Riverfront Amphitheater AND the Brenton Skating Plaza,
and Ordinance 16,064 keeps Brenton under "F" while moving Simon Estes to a new
Type "N" at 100 dBA -> 100 dBC with the Sunday-to-Thursday close pulled from
11:00 p.m. to 10:00 p.m. Read by letter that is two unrelated permits. The
same split happens to Waterworks Park, which becomes Type "I" (the Lauridsen
Amphitheater) and Type "M" (the park field, with the Zoo parcel).

Run: python3 build.py
"""
import json
import re
from pathlib import Path

HERE = Path(__file__).parent
SRC = HERE / "sources"

ORD_DATES = {"14746": "2008-02-25", "16064": "2021-10-18", "16127": "2022-05-23"}


def load(name):
    t = (SRC / name).read_text(encoding="utf-8", errors="replace")
    t = t.replace("“", '"').replace("”", '"')
    t = t.replace("‘", "'").replace("’", "'").replace("―", '"')
    # Pagination the extractor leaves behind. The form feed is the same class
    # of artifact as the page number and was not being removed: it sits at the
    # head of a page's first line, so `(11) Type "K" permit` began with \f and
    # no line-anchored pattern could see it. Ordinance 16,064's Type "J" ran
    # on into Type "K" for exactly that reason. Every form feed in all four
    # sources is at a line start, so dropping them never joins two lines.
    t = t.replace("\f", "")
    t = re.sub(r"\n\s*Page \d+\s*\n", "\n", t)
    t = re.sub(r"\n{3,}", "\n\n", t)
    return t


def tidy(s):
    """Collapse the wrapping pdftotext leaves, keeping sentences verbatim."""
    s = re.sub(r"[ \t]+", " ", s)
    s = re.sub(r"^ +", "", s, flags=re.M)
    # A section number wrapped mid-token — "section 42-\n257" — normalises to
    # "42- 257" and no quote containing it can ever match its own passage.
    s = re.sub(r"(\d)-\n(\d)", r"\1-\2", s)
    s = re.sub(r"\n{3,}", "\n\n", s)
    return s.strip()


def code_section(text, sec):
    """One `Sec. 42-xxx.` section of the codified article."""
    m = re.search(rf"^Sec\. {re.escape(sec)}\. ?- ?(.+?)$", text, re.M)
    if not m:
        raise SystemExit(f"missing code section {sec}")
    nxt = re.search(r"^Sec\. 42-\d+\. ?-", text[m.end():], re.M)
    body = text[m.end(): m.end() + (nxt.start() if nxt else 4000)]
    return m.group(1).strip().rstrip("."), tidy(body)


# Where a permit subsection ends.
#
# It used to end at `pos + 2200` characters, which is not a boundary any
# document has, and two of the twenty-seven blocks were wrong because of it.
# The codified Type "J" swallowed subsections (f) Commercial advertising and
# (h) Denial or revocation whole — 1,975 characters where the permit is 745 —
# so a rule about advertising was filed under a construction permit, and the
# two readings of "J" no longer matched, which cost the pair its label. The
# Type "Q" of Ordinance 16,127 swallowed the clerk's certification and was
# then cut mid-sentence. Type "G" cleared the cap by 58 characters and was
# correct by luck.
#
# A subsection ends where the document says it does: at the next permit, at a
# sibling of the lettered subsection holding the permit list, at the enacting
# sections an ordinance closes with, at its signature block, or at the next
# section of the code. There is no character cap; a block that finds no
# terminator runs to the end of its source, which is the only case where that
# is the right answer.
BLOCK_END = re.compile(
    r"""^[ \t]*(?:
          \(\d{1,2}\)[ \t]*Type     # the next permit in the list
        | \([a-z]\)[ \t]+\w         # (f) Commercial advertising.
        | Section[ \t]+\d+\.        # Section 2. This ordinance shall...
        | FORM[ \t]+APPROVED         # the signature block
        | Attest:                    # the clerk's certification
        | Sec\.[ \t]*42-\d+\.       # the next section of the code
    )""",
    re.M | re.X,
)


def permit_blocks(text):
    """`(6) Type "F" permit ...` subsections, keyed by letter."""
    marks = [
        (m.start(), m.group(1), m.group(2))
        for m in re.finditer(r'\((\d{1,2})\)\s*Type\s*"?\s*([A-Z])\s*"?\s*[Pp]ermit', text)
    ]
    out = {}
    for pos, num, letter in marks:
        rest = text[pos:]
        # From 1, so the marker opening this block cannot terminate it.
        end = BLOCK_END.search(rest, 1)
        out[letter] = (num, tidy(rest[: end.start() if end else len(rest)]))
    return out


def measures(b):
    """Every quantity a permit subsection states."""
    return {
        "level": sorted({f"{v} dB{u}" for v, u in re.findall(r"(\d{2,3})\s*dB([AC])s?", b)}
                        | {f"{v} decibels" for v in re.findall(r"(\d{2,3})\s*decibels", b)}),
        # The wrap pdftotext leaves puts a newline inside "10:00 p.m.", and a
        # time that differs from itself by a line break is a false label.
        "hours": sorted({re.sub(r"\s+", "", h) for h in re.findall(r"(\d{1,2}:\d{2}\s*[ap]\.\s*m\.)", b)}),
        "feet": sorted({f.replace(",", "") for f in re.findall(r"([\d,]{1,6})\s*feet", b)}),
        "times": sorted({f"{a}/{b2}" for a, b2 in re.findall(r"(\w+)\s+times per (year|calendar year|month)", b)}),
        "days": sorted(set(re.findall(r"(\d{1,3})\s*days", b))),
    }


def diff(a, b):
    return {k: (a[k], b[k]) for k in a if a[k] != b[k]}


def same_words(a, b):
    """Two readings equal but for typography — quote style, dashes, case."""
    f = lambda s: re.sub(r"[^a-z0-9]+", " ", s.lower()).strip()
    return f(a) == f(b)


def states_a_measure(m):
    return any(m.values())


# ── tables, restored from the PDF's own column geometry ─────
#
# `pdftotext -layout` flattens a table into space-separated words and loses
# which row a value belongs to. Table 1 came out with "60" on a line of its
# own and "Residential zones" on the NEXT line, so the limit had no readable
# subject; Tables 2 and 3 came out as "90 24 hours", which is not a sentence
# and cannot be cited.
#
# Every cell below is the council's text, read off the word bounding boxes
# `pdftotext -tsv` reports (code-article-iv.pdf, pages 5-7). Only the
# row/column association is restored, and it is auditable — re-run:
#
#   pdftotext -tsv -f 5 -l 7 sources/code-article-iv.pdf - \
#     | awk -F'\t' '$1==5 {print $7, $8, $12}'
#
# Table 1's label column is a vertically-centred merged cell spanning its two
# value rows, which is why the residential label repeats here.
TABLES = {
    "42-254": {
    "Table 1.": (
        "Table 1. Sound Levels By Receiving Land Use\n\n"
        "| Zoning Category of Receiving Land Use | Time | Sound Level Limit, dBA |\n"
        "| --- | --- | --- |\n"
        "| Residential zones: R1-80 to R-6,R-HD and a residential PUD | 7:00 a.m. to 10:00 p.m. | 60 |\n"
        "| Residential zones: R1-80 to R-6,R-HD and a residential PUD | 10:00 p.m. to 7:00 a.m. | 50 |\n"
        "| Mixed use and commercial zones: PUD to C-4 | At all times | 65 |\n"
        "| Industrial zones: M-1 to M-3 | At all times | 75 |\n"
        "| Noise sensitive area | At all times | 55 |\n"
        "| U-1 floodplain or FW floodway | At all times | 65 |\n"
    ),
    "Table 2.": (
        "Table 2. Continuous Sound Levels Which Pose an Immediate Threat to Health and Welfare "
        "(Measured at 50 Feet)\n\n"
        "| Sound Level Limit (dBA) | Duration |\n"
        "| --- | --- |\n"
        "| 90 | 24 hours |\n| 93 | 12 hours |\n| 96 | 6 hours |\n| 99 | 3 hours |\n"
        "| 102 | 1.5 hours |\n| 105 | 45 minutes |\n| 108 | 22 minutes |\n"
    ),
    "Table 3.": (
        "Table 3. Impulsive Sound Levels Which Pose an Immediate Threat to Health and Welfare "
        "(Measured at 50 Feet)\n\n"
        "| Sound Level Limit (dB) | Number of Repetitions per 24-Hour Period |\n"
        "| --- | --- |\n"
        "| 140 | 1 |\n| 130 | 10 |\n| 120 | 100 |\n"
    ),
    },
    # Page 12: label column at x81-400, "35 mph or Less" at x483, "Over 35
    # mph" at x517. The label wraps around its own numbers in the flattened
    # text ("...10,000 lbs. or / 88 92 / more and any combination...").
    "42-259": {
    "Noise Limit In Relation": (
        "Noise Limit In Relation To Legal Speed Limit\n\n"
        "| Type of Vehicle | 35 mph or Less, dBA | Over 35 mph, dBA |\n"
        "| --- | --- | --- |\n"
        "| Any motor vehicle with a manufacturer's gross vehicle weight rating of 10,000 lbs. "
        "or more and any combination of vehicles towed by such motor vehicle | 88 | 92 |\n"
        "| Any motorcycle | 82 | 86 |\n"
        "| Any other motor vehicle and any combination of motor vehicles towed by such motor "
        "vehicle | 76 | 82 |\n"
    ),
    },
}

# Where each flattened table ends in the -layout text.
TABLE_ENDS = {
    "Table 1.": "(1) For the purposes of this article",
    "Table 2.": "(4) Correction for character of sound",
    "Table 3.": "(O.14,746",
    "Noise Limit In Relation": "This subsection applies to the total noise",
}


def restore_tables(body, sec):
    for marker, rendered in TABLES.get(sec, {}).items():
        start = body.find(marker)
        if start < 0:
            raise SystemExit(f"{marker} not found — the source changed, rebuild the mapping")
        end = body.find(TABLE_ENDS[marker], start)
        if end < 0:
            raise SystemExit(f"end of {marker} not found")
        body = body[:start] + rendered + "\n" + body[end:]
    return body


# ── anchors: what must survive extraction ───────────────────
#
# The smallest phrase each tension turns on. Derived from the SAME measure
# diff that produced the label, never from anything an extractor produced —
# so a tension whose load-bearing clause never became a candidate is reported
# as unreachable rather than scored as a comparison failure.
def alternatives(cat, value):
    """Ways a rule might word one measure. Same rule, different words only."""
    if cat == "level":
        n, unit = value.split(" ", 1)
        u = unit.lower()
        return [f"{n} {u}", f"{n} {u}s", f"{n}{u}", f"{n} {unit}"]
    if cat == "hours":
        m = re.match(r"(\d{1,2}):(\d{2})([ap])\.m\.", value)
        if not m:
            return [value]
        h, mm, ap = m.groups()
        base = [f"{h}:{mm} {ap}.m.", f"{h}:{mm}{ap}m"]
        # The bare hour is the same instant ONLY on the hour: "12 a.m." is not
        # "12:30 a.m.", and letting it satisfy the anchor would score a rule
        # that states a different time as though it carried the right one.
        return base + ([f"{h} {ap}.m.", f"{h}{ap}m"] if mm == "00" else [])
    if cat == "feet":
        return [f"{value} feet", f"{value} ft"]
    if cat == "days":
        return [f"{value} days", f"{value} day"]
    if cat == "times":
        n, per = value.split("/")
        return [f"{n} times per {per}", f"{n} times a {per}"]
    return [value]


def anchor_sides(b, a, d):
    # ONE requirement per side, satisfied by ANY of the measures the council
    # changed. A side is reachable when its rule carries any one of them —
    # requiring all three would report a tension unreachable because the
    # extractor kept the decibel limit and dropped the measuring distance.
    a_alts, b_alts = [], []
    for cat, (old, new) in sorted(d.items()):
        for v in [x for x in old if x not in new]:
            a_alts += alternatives(cat, v)
        for v in [x for x in new if x not in old]:
            b_alts += alternatives(cat, v)
    a_must = [sorted(set(a_alts))] if a_alts else []
    b_must = [sorted(set(b_alts))] if b_alts else []
    # A side with nothing distinctive still has to yield the rule at all.
    if not a_must:
        a_must.append([f'type "{b["letter"]}" permit', f'type {b["letter"]} permit'])
    if not b_must:
        b_must.append([f'type "{a["letter"]}" permit', f'type {a["letter"]} permit'])
    return (
        {"section": b["key"], "must": a_must},
        {"section": a["key"], "must": b_must},
    )


code = load("code-article-iv.txt")
o64 = load("ord-16064.txt")
o127 = load("ord-16127.txt")

base_p = permit_blocks(code)
amd_p = permit_blocks(o64)
q_p = permit_blocks(o127)

# ── what each permit authorises ─────────────────────────────
#
# The council amends a permit by SUBJECT — the venue or the conduct it covers
# — and the type letter is only a slot in a list that gets renumbered. Every
# entry below is the subject the council's own line names, quoted beside it,
# and nothing here is inferred from a permit's body. Read them against
# `des-moines-noise.md`; that is the whole audit.
#
# Two readings state the same rule when their subjects overlap.
SUBJECTS = {
    # Codified article, from Ordinance 14,746 (2008).
    ("base", "A"): {"street-closing"},          # '(1) Type "A" permit.' — street closing, commercial/mixed use
    ("base", "B"): {"residential-park"},        # "Parks located in residential zones"
    ("base", "C"): {"church-school"},           # "Church or school grounds"
    ("base", "D"): {"residential-event"},       # "Residential events"
    ("base", "E"): {"background-commercial"},   # "Background sound equipment"
    ("base", "F"): {"simon-estes", "brenton-plaza"},
    #                                             "Simon Estes Riverfront Amphitheater and Brenton Skating Plaza"
    ("base", "G"): {"special-event"},           # "Special event live performances"
    ("base", "H"): {"farmers-market"},          # "Farmer's Market"
    ("base", "I"): {"waterworks-park"},         # "Waterworks Park"
    ("base", "J"): {"night-construction"},      # "Night construction"
    # Ordinance 16,064 (2021). K through P have no codified counterpart.
    ("16064", "A"): {"street-closing"},         # '(1) Type "A" permit.' — street closing, now incl. industrial
    ("16064", "B"): {"residential-park"},       # "parks located in residential zones"
    ("16064", "C"): {"church-school"},          # "church or school grounds"
    ("16064", "D"): {"residential-event"},      # "residential events"
    ("16064", "E"): {"background-commercial"},  # "background sound equipment"
    ("16064", "F"): {"brenton-plaza"},          # "Brenton Skating Plaza" — Simon Estes moved out, to "N"
    ("16064", "G"): {"special-event"},          # "Special Event Live Performances"
    ("16064", "H"): {"farmers-market"},         # "Farmer's Market"
    ("16064", "I"): {"waterworks-lauridsen"},   # "Waterworks Park - Lauridsen Amphitheater"
    ("16064", "J"): {"night-construction"},     # "night construction"
    ("16064", "K"): {"event-route"},            # "an event whose route is on city streets, trails and/or other public right-of-way"
    ("16064", "L"): {"private-recreation"},     # "a privately held recreational area more than 40 acres in area"
    ("16064", "M"): {"zoo-parcel", "waterworks-field"},
    #                                             "Zoo and Water Works Park" / "the Blank Park Zoo Foundation Parcel and Water Works Park field"
    ("16064", "N"): {"simon-estes"},            # "Simon Estes Riverfront Amphitheater"
    ("16064", "O"): {"riverview-amphitheater"}, # "Riverview Amphitheater"
    ("16064", "P"): {"botanical-center"},       # "Botanical Center"
    # Ordinance 16,127 (2022).
    ("16127", "Q"): {"900-mulberry"},           # "900 Mulberry Street"
}

# A place inside a larger place the codified article covered whole. The 2008
# article permits "Waterworks Park"; the 2021 ordinance addresses two named
# parts of it separately. Both parts are that permit restated, and neither is
# the other.
WITHIN = {
    "waterworks-lauridsen": "waterworks-park",
    "waterworks-field": "waterworks-park",
}


def overlap(a, b):
    """Do two subject sets name any of the same place or conduct?"""
    def same(x, y):
        return x == y or WITHIN.get(x) == y or WITHIN.get(y) == x

    return any(same(x, y) for x in a for y in b)


# ── the document ────────────────────────────────────────────
doc, keys, built = [], {}, []

GENERAL = ["42-251", "42-254", "42-256", "42-257", "42-259", "42-262"]
for sec in GENERAL:
    title, body = code_section(code, sec)
    body = restore_tables(body, sec)
    doc.append(f'# Des Moines Municipal Code, Sec. {sec} — {title}\n\n{body}\n')
    built.append((f"Sec. {sec}", body))
    keys[f"sec:{sec}"] = title

for letter, (num, body) in sorted(base_p.items()):
    sec = f"42-258({num})"
    doc.append(f'# Des Moines Municipal Code, Sec. {sec} — Type "{letter}" permit\n\n{body}\n')
    built.append((f'Sec. {sec} Type "{letter}" permit', body))
    keys[f"sec:{sec}"] = f'Type "{letter}" permit'

for letter, (num, body) in sorted(amd_p.items()):
    sec = f"42-258({num})"
    doc.append(
        f'# Ordinance 16,064, adopted {ORD_DATES["16064"]} — Sec. {sec}, Type "{letter}" permit\n\n{body}\n'
    )
    built.append((f'Ord 16,064 Sec. {sec} Type "{letter}" permit', body))
    keys[f"ord:16064/{sec}"] = f'Type "{letter}" permit'

for letter, (num, body) in sorted(q_p.items()):
    sec = f"42-258({num})"
    doc.append(
        f'# Ordinance 16,127, adopted {ORD_DATES["16127"]} — Sec. {sec}, Type "{letter}" permit\n\n{body}\n'
    )
    built.append((f'Ord 16,127 Sec. {sec} Type "{letter}" permit', body))
    keys[f"ord:16127/{sec}"] = f'Type "{letter}" permit'

# A code section owns its own lettered subsections — Sec. 42-254 really does
# contain "(a) Maximum permissible sound levels." and "(b) Immediate threat."
# A permit is one item of a list and owns nothing below itself.
SECTION_END = re.compile(
    r"^[ \t]*(?:Section[ \t]+\d+\.|FORM[ \t]+APPROVED|Attest:|Sec\.[ \t]*42-\d+\.)", re.M
)

# No section may carry text belonging to another one. This is the check the
# 2,200-character cap needed and did not have: it fires on the exact input
# that was wrong, rather than on a length that happened to look suspicious.
for _head, _body in built:
    _end = BLOCK_END if "permit" in _head else SECTION_END
    _stray = _end.search(_body, 1)
    if _stray:
        _line = _body[_stray.start(): _body.find("\n", _stray.start())]
        raise SystemExit(f"{_head} runs past its own end into: {_line!r}")

(HERE / "des-moines-noise.md").write_text("\n".join(doc), encoding="utf-8")

# ── the truth ───────────────────────────────────────────────
planted, non, unlabelled = [], [], []
anchors = {}
SPLIT = ["train", "dev", "test"]

# Every reading of every permit, keyed the way truth.json keys a side.
READINGS = []
for _src, _blocks in (("base", base_p), ("16064", amd_p), ("16127", q_p)):
    for _letter, (_num, _body) in sorted(_blocks.items()):
        _sec = f"42-258({_num})"
        READINGS.append({
            "src": _src, "letter": _letter, "sec": _sec, "body": _body,
            "key": f"sec:{_sec}" if _src == "base" else f"ord:{_src}/{_sec}",
            "side": {"section": _sec} if _src == "base" else {"ordinance": _src, "section": _sec},
        })

undeclared = [(r["src"], r["letter"]) for r in READINGS if (r["src"], r["letter"]) not in SUBJECTS]
if undeclared:
    raise SystemExit(f"no subject declared for {undeclared} — every reading must be placed")

# An ordinance amends the codified article, so a pair runs base -> later
# reading. Two later readings of one subject would be the ordinances amending
# each other, which none of these do.
restated = [
    (b, a)
    for b in READINGS if b["src"] == "base"
    for a in READINGS if a["src"] != "base"
    if overlap(SUBJECTS[(b["src"], b["letter"])], SUBJECTS[(a["src"], a["letter"])])
]

for i, (b, a) in enumerate(restated):
    d = diff(measures(b["body"]), measures(a["body"]))
    if not d and not (states_a_measure(measures(b["body"])) or states_a_measure(measures(a["body"]))):
        # Neither reading states a measure this script can read, so "no measure
        # changed" is a vacuous test, not a finding. A rule that cannot see a
        # change must not vote that there was none (ARCH_PRINCIPLES §18.3).
        if same_words(b["body"], a["body"]):
            non.append({
                "id": f"N{len(non)+1}", "split": SPLIT[i % 3],
                "a": b["side"], "b": a["side"],
                "why": (f'compatible: the ordinance re-enacts the Type "{b["letter"]}" permit '
                        "word for word, changing only typography."),
            })
        else:
            unlabelled.append((b["letter"], "states no readable measure and the wording changed"))
        continue
    if d:
        changed = "; ".join(
            f"{k} {' / '.join(old) or '(none stated)'} -> {' / '.join(new) or '(none stated)'}"
            for k, (old, new) in sorted(d.items())
        )
        moved = "" if b["letter"] == a["letter"] else f' as the Type "{a["letter"]}" permit'
        planted.append({
            "id": f"T{len(planted)+1}",
            "type": "unmarked_supersession",
            "split": SPLIT[i % 3],
            "a": b["side"], "b": a["side"],
            "why": (f'Ordinance {a["src"][:2]},{a["src"][2:]} restates the Type "{b["letter"]}" '
                    f'permit{moved} with different measures: {changed}.'),
        })
        anchors[planted[-1]["id"]] = list(anchor_sides(b, a, d))
    else:
        non.append({
            "id": f"N{len(non)+1}",
            "split": SPLIT[i % 3],
            "a": b["side"], "b": a["side"],
            "why": f'compatible: the ordinance re-enacts the Type "{b["letter"]}" permit with every stated measure unchanged.',
        })

# A permit is an authorised exception to the general table, not a contradiction
# of it — Sec. 42-254(a) says so in its own first clause.
for letter, (num, _) in sorted(q_p.items()):
    non.append({
        "id": f"N{len(non)+1}",
        "split": "test",
        "a": {"section": "42-254"},
        "b": {"ordinance": "16127", "section": f"42-258({num})"},
        "why": ('decoy: the permit allows levels far above the Table 1 night limit, but Sec. '
                '42-254(a) opens "with the exception of sound levels elsewhere specifically '
                'authorized or allowed in this article" — a permit is that exception.'),
    })

# Base sections no ordinance here amends: heavy shared vocabulary, no conflict.
UNAMENDED = [("42-257", "6"), ("42-259", "7"), ("42-262", "1"), ("42-256", "2")]
for j, (sec, pair_num) in enumerate(UNAMENDED):
    non.append({
        "id": f"N{len(non)+1}",
        "split": SPLIT[j % 3],
        "a": {"section": sec},
        "b": {"ordinance": "16064", "section": f"42-258({pair_num})"},
        "why": (f"decoy: Sec. {sec} and the permit subsection share the article's vocabulary "
                "(sound, music, hours, measurement) but govern different conduct, and no "
                "ordinance in this document amends Sec. " + sec + "."),
    })

truth = {
    "corpus_id": "des-moines-noise",
    "note": (
        "Ground truth for Article IV (Noise Control) of the Des Moines, Iowa municipal code "
        "as codified from Ordinance 14,746 (2008-02-25), interleaved with Ordinances 16,064 "
        "(2021-10-18) and 16,127 (2022-05-23) which amend it. Sections keyed by code section "
        "number, or by amending ordinance plus section. Labels are derived mechanically by "
        "build.py from the council's own amendment pointers and the measure diff between the "
        "two readings — see PROVENANCE.md. Splits: train/dev tunable, test sacred."
    ),
    "schema_version": 2,
    # Still false for the DOCUMENT: nothing here labels a pair between two of
    # the six general sections, or between a general section and a permit. A
    # proposal naming one of those may be a genuine conflict nobody labelled,
    # and precision computed over it would measure the manifest's size rather
    # than the tool's discrimination.
    "exhaustive": False,
    # It IS false for the document and true for a REGION of it, and those are
    # different facts. Inside the permit block every cross-section pair is
    # accounted for: the pairs named above are the readings that restate one
    # another, and every other pair of members is compatible because a permit
    # authorises one venue or one kind of conduct, and two permits with
    # different subjects have nothing to disagree about. That claim rests on
    # the SUBJECTS map at the top of this script — 27 entries, each quoting
    # the council's own subject line — and it is why the map has to be read
    # rather than trusted. See PROVENANCE.md.
    "exhaustive_within": {
        "region": "every permit subsection of Sec. 42-258, in all three readings",
        "members": sorted(r["key"] for r in READINGS),
        "unlabelled_means": "compatible",
        "why": (
            "A permit is one authorisation for one venue or one kind of conduct. Two "
            "readings restate the same rule when their subjects overlap, and every such "
            "pair in this region is labelled above; a pair of members that is not named "
            "is two different authorisations and cannot contradict."
        ),
    },
    "tension_types": ["unmarked_supersession"],
    "planted_tensions": planted,
    "expected_non_tensions": non,
}
(HERE / "truth.json").write_text(json.dumps(truth, indent=2) + "\n", encoding="utf-8")

(HERE / "extraction-anchors.json").write_text(
    json.dumps({
        "note": (
            "What must survive extraction for each planted tension to be FINDABLE AT ALL. "
            "Each anchor is the measure the council changed, taken from the diff between the "
            "codified reading and the amending ordinance — not from anything an extractor "
            "produced. Matching is case-insensitive over collapsed whitespace against any "
            "candidate drawn from that section; each `must` entry is a list of alternatives "
            "satisfied by any one of them. Generated by build.py."
        ),
        "schema_version": 1,
        "anchors": anchors,
    }, indent=2) + "\n",
    encoding="utf-8",
)
print(f"anchors  : {len(anchors)} tensions anchored")

for letter, reason in unlabelled:
    print(f'  UNLABELLED Type "{letter}": {reason} — excluded from truth')
print(f"document : {len(doc)} sections")
print(f"planted  : {len(planted)}")
print(f"non      : {len(non)}")
for t in planted:
    print(f"  {t['id']} [{t['split']:5}] {t['a']['section']:12} vs ord {t['b']['ordinance']} — {t['why'][:96]}")
for n in non:
    print(f"  {n['id']} [{n['split']:5}] {n['a'].get('section'):12} vs {n['b'].get('ordinance')} — {n['why'][:80]}")


# ── the instrument points at something ──────────────────────
#
# A key in the manifest that matches no heading scores nothing and says so
# nowhere. Checked here so the corpus cannot ship pointing at a section that
# does not exist (ARCH_PRINCIPLES §18.1).
def heading_key(h):
    m = re.search(r"Ordinance ([\d,]+),", h)
    sec = re.search(r"Sec\. (\S+?)[,\s—]", h + " ")
    if not sec:
        return None
    if m:
        return f"ord:{m.group(1).replace(',', '')}/{sec.group(1)}"
    return f"sec:{sec.group(1)}"


def side_key(d):
    if "ordinance" in d:
        return f"ord:{d['ordinance']}/{d['section']}"
    return f"sec:{d['section']}"


headings = [h for h in re.findall(r"^# (.+)$", (HERE / "des-moines-noise.md").read_text(), re.M)]
resolved = [heading_key(h) for h in headings]
dupes = {k for k in resolved if resolved.count(k) > 1}
assert not dupes, f"heading keys are not unique: {dupes}"
missing = []
for entry in planted + non:
    for side in (entry["a"], entry["b"]):
        k = side_key(side)
        if k not in resolved:
            missing.append((entry["id"], k))
assert not missing, f"truth keys with no section in the document: {missing}"
print(f"keys     : {len(resolved)} unique headings, every truth key resolves")
