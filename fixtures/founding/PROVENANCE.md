# The founding documents — where this corpus came from

The Declaration of Independence, the Articles of Confederation, and the
Constitution of the United States with all twenty-seven amendments. Every word
is the enacting body's.

| Document | Source | Adopted | sha256 (HTML) |
|---|---|---|---|
| Declaration of Independence | `archives.gov/founding-docs/declaration-transcript` | 1776-07-04 | `6a7237a6…46b2c52c` |
| Articles of Confederation | `avalon.law.yale.edu/18th_century/artconf.asp` | ratified 1781-03-01 | `61039252…311971f9` |
| Constitution | `archives.gov/founding-docs/constitution-transcript` | 1787-09-17 | `67f506d7…7ed40163` |
| Bill of Rights (Amendments I–X) | `archives.gov/founding-docs/bill-of-rights-transcript` | ratified 1791-12-15 | `804228c4…4c0736d2` |
| Amendments XI–XXVII | `archives.gov/founding-docs/amendments-11-27` | 1795–1992 | `a991eed9…6d08c7b5` |

Retrieved 2026-08-25. `sources/fetch.sh` re-downloads and prints the hashes;
`build.py` rebuilds `founding.md` and `truth.json` from the vendored HTML with
no network and no model.

**Licence.** The founding documents are edicts of government and carry no
copyright. The National Archives transcriptions preserve the original spelling
and punctuation of the parchment; the Avalon Project's Articles text is
likewise a transcription of the ratified instrument. The HTML is vendored so a
standalone `git clone` of `canon` can rebuild the corpus.

## What was constructed, and what was not

**The grouping is ours. The text is not.** `build.py` decides which words sit
under which heading, and every grouping rule keys on a marker the document
prints for itself: the parchment's `Article. I.` and `Section. 1.`, Avalon's
bare roman numerals, the Archives' `AMENDMENT XIV`. Nothing is paraphrased,
reordered, or summarised.

**Three kinds of editorial matter were dropped**, all of them the publisher's
rather than the enacting body's: the Archives' `Passed by Congress … Ratified …`
datelines, its `Note:` annotations, and each page's navigation and footer. The
notes are dropped from the text a reader is asked to extract commitments from
and read separately as ground truth — see below. Dropping the site chrome is
not cosmetic: the first build produced a second, empty Amendment XXVII made of
`Contact Us` and `En Español`, because the Archives repeats every amendment
heading in an "On This Page" list *below* the transcript.

## The ground truth comes in two halves, and only one of them is ours

**Half one is DERIVED and nobody planted it.** The National Archives prints a
note under each amendment naming the article it modified or superseded —
*"Article I, section 2, of the Constitution was modified by section 2 of the
14th amendment."* `build.py` parses those nine notes out of the same vendored
HTML the corpus is built from, and **refuses to emit anything at all if a note
does not parse**, rather than silently scoring against eight. They yield ten
pairs; an eleventh comes from the 21st Amendment's own text repealing the 18th.
Anyone can check all eleven against archives.gov in a browser.

These are `unmarked_supersession`, and they are the same defect the Des Moines
corpus was built to expose: a later decision replaced an earlier rule and
nobody struck the earlier one. The Constitution still contains the three-fifths
clause, the original electoral procedure, legislature-appointed senators, and
Prohibition. A document that cannot say what it currently says is exactly the
problem `canon` exists for, and the United States has been running one for two
hundred and thirty-eight years.

**One resolution inside half one is ours**, and it is 6 lines in `build.py`
under `OPERATIVE`. Where a note names an amendment but not one of its sections
— *"superseded by the 13th amendment"* — the key has to name the section that
carries the operative language, or it names no passage in the corpus and the
tension is unfindable by construction rather than by the model's failure. Each
entry quotes the words it was read off.

**Half two is AUTHORED and marked as such.** Six `principle_vs_rule` entries,
each pairing a stated principle with a specific rule that cannot be honoured
alongside it. The rule applied: a pair goes in only when **both sides are
quoted text in this corpus**. No entry rests on a document that was not
vendored, and none rests on what anyone later said the words meant. Every quote
is checked verbatim against the corpus on every build — thirteen passages, and
the build fails if one has drifted.

One of the six is marked `contested`: whether the Necessary and Proper Clause
and the Tenth Amendment are in tension is the oldest argument in American
constitutional law, and this file is not going to settle it. It is labelled
because a reader deserves to know which entries are plain on the face of the
text and which are a reading.

**The decoys are chosen to be hard.** Four of the six are near-identical
wording applied to different subjects — the ex post facto prohibition stated
once against Congress and once against the States, and the same enforcement
sentence carried by four different amendments. That is precisely the shape that
folded a real Des Moines rule out of existence, and a detector that flags it
here is telling us something we need to know.

## What this corpus does not contain

The ratification debates. The Federalist Papers are the most-cited interpretive
source for this text and they are **not** part of the canon, because a canon is
what a body committed to and the Federalist is the argument that produced it —
the class of thing a snapshot drops by design. Retrieving it as interpretive
evidence is a corpus question, not a canon question, and it belongs on the
other side of that seam.
