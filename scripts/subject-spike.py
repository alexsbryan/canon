#!/usr/bin/env python3
"""Trial a source-backed topic hint on recorded draft candidates, without a model.

The hint is NOT an inferred subject or an act: every rule still needs a person
to confirm what it is about. Run against a draft artifact and the same exported
sources; no files are changed. --jsonl prints the full candidate contract.

    python3 scripts/subject-spike.py <draft-run.json> <source-root>
"""

import argparse
import hashlib
import json
import re
from collections import Counter, defaultdict
from pathlib import Path


SPAN = re.compile(r"^(\d+)-(\d+)$")
DEICTIC = re.compile(r"\b(it|this|that|these|those|neither|either|they|them)\b", re.I)
EVIDENCE_CHARS = 320  # extra context per card, excluding the quote and title


class Sources:
    def __init__(self, root):
        self.root = root.resolve()
        self.by_name = defaultdict(list)
        for path in root.rglob("*.md"):
            if path.is_file() and path.resolve().is_relative_to(self.root):
                self.by_name[path.name].append(path.resolve())

    def locate(self, name):
        if not name or Path(name).is_absolute() or ".." in Path(name).parts:
            return None, "unsafe source path"
        exact = (self.root / name).resolve()
        if not exact.is_relative_to(self.root):
            return None, "source escapes root"
        if exact.is_file():
            return exact, None
        if "/" in name:
            return None, "source file absent"
        matches = self.by_name[name]
        if len(matches) != 1:
            return None, "source name ambiguous" if matches else "source file absent"
        return matches[0], None


def heading(lines, stop):
    """Nearest actual Markdown heading at or before the cited passage."""
    for index in range(min(stop, len(lines)) - 1, -1, -1):
        raw = lines[index].strip()
        if raw.startswith("# "):
            return index + 1, raw[2:].strip()
    return None


def pointer(root, path, line, text):
    return {
        "text": text,
        "source": f"{path.relative_to(root)}:{line}",
        "digest": hashlib.sha256(text.encode("utf-8")).hexdigest()[:12],
    }


def nearby_evidence(lines, start, end, quote, root, path, strategy="strict"):
    """Cut a bounded adjacent excerpt. It is evidence to read, not a subject."""
    passage = "\n".join(lines[start - 1:end])
    if passage.count(quote) != 1:
        return None, "quote repeated inside cited span"
    before, after = passage.split(quote, 1)
    line = start + before.count("\n")
    prefix = before.rsplit("\n", 1)[-1]
    if prefix.strip():
        index = line - 1
        left, right, direction = 0, len(prefix.rstrip()), "tail"
        method = "same_line_prefix"
    elif after.split("\n", 1)[0].strip():
        index = line + quote.count("\n") - 1
        suffix = after.split("\n", 1)[0]
        left, right, direction = len(lines[index]) - len(suffix), len(lines[index]), "head"
        method = "same_line_suffix"
    elif line > 1 and lines[line - 2].strip():
        index = line - 2
        left, right, direction = 0, len(lines[index].rstrip()), "tail"
        method = "adjacent_line"
    else:
        # A section break can cross into a different rule. Offer the preceding
        # paragraph only as a separate, explicitly weaker experiment.
        index = line - 2
        if strategy != "extended" or index < 0 or lines[index].strip():
            return None, "no adjacent source context"
        index -= 1  # cross at most one blank line; never skip another heading
        if index < 0 or not lines[index].strip() or lines[index].lstrip().startswith("#"):
            return None, "no adjacent source context"
        left, right, direction = 0, len(lines[index].rstrip()), "tail"
        method = "preceding_paragraph"

    if direction == "tail":
        first, last = max(left, right - EVIDENCE_CHARS), right
    else:
        first, last = left, min(right, left + EVIDENCE_CHARS)
    excerpt = lines[index][first:last]
    col = first + len(excerpt) - len(excerpt.lstrip()) + 1  # codepoint, 1-based
    text = excerpt.strip()
    if not text or lines[index][col - 1:col - 1 + len(text)] != text:
        return None, "adjacent context could not be cut"
    return {
        "text": text,
        "source": f"{path.relative_to(root)}:{index + 1}@{col}-{col + len(text) - 1}",
        "digest": hashlib.sha256(text.encode("utf-8")).hexdigest()[:12],
        "method": method,
        "clipped": first > left or last < right,
    }, None


def card(candidate, sources, strategy="strict"):
    result = {
        "text": candidate["text"],
        "kind": candidate.get("kind", "rule"),
        "citation": candidate.get("source"),
        "quote": candidate.get("quote"),
        "topic_hint": None,
        "local_heading": None,
        "referent_evidence": None,
        "signals": [],
        "review": "blocked",
    }

    citation = candidate.get("source", "")
    name, sep, span = citation.rpartition(":")
    coords = SPAN.fullmatch(span) if sep else None
    if not coords:
        result["blocked_by"] = "citation has no line span"
        return result
    start, end = map(int, coords.groups())
    path, problem = sources.locate(name)
    if problem:
        result["blocked_by"] = problem
        return result
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeError):
        result["blocked_by"] = "source unreadable"
        return result
    if start < 1 or end < start or end > len(lines):
        result["blocked_by"] = "citation outside source"
        return result
    quote = result["quote"]
    if not quote or quote not in "\n".join(lines[start - 1:end]):
        result["blocked_by"] = "quote no longer in cited source span"
        return result

    result["review"] = "human_confirm_subject"
    evidence, reason = nearby_evidence(lines, start, end, quote, sources.root, path, strategy)
    result["referent_evidence"] = evidence
    if reason:
        result["signals"].append("no_unique_local_evidence")
    first = heading(lines, 1)
    near = heading(lines, start)
    if first:
        result["topic_hint"] = pointer(sources.root, path, *first)
        if first[1].endswith(("…", "...")):
            result["signals"].append("truncated_topic_hint")
    else:
        result["signals"].append("no_source_topic_hint")
    if near and near != first:
        result["local_heading"] = pointer(sources.root, path, *near)
    if DEICTIC.search(candidate["text"]):
        result["signals"].append("possible_unresolved_reference")
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("source_root", type=Path)
    parser.add_argument("--jsonl", action="store_true", help="one review card per rule")
    parser.add_argument("--query", help="show cards whose text contains this substring")
    parser.add_argument("--strategy", choices=("strict", "extended"), default="strict")
    parser.add_argument("--method", choices=("same_line_prefix", "same_line_suffix", "adjacent_line", "preceding_paragraph", "none"),
                        help="inspect a particular local-context path")
    parser.add_argument("--max-examples", type=int, default=8)
    args = parser.parse_args()
    if not args.source_root.is_dir():
        parser.error("source_root must be a directory")
    if args.max_examples < 1:
        parser.error("--max-examples must be positive")
    run = json.loads(args.run.read_text())
    sources = Sources(args.source_root)
    rules = [c for c in run["candidates"] if c.get("kind", "rule") == "rule"]
    cards = [card(c, sources, args.strategy) for c in rules]
    if args.jsonl:
        for item in cards:
            if not args.query or args.query.lower() in item["text"].lower():
                print(json.dumps(item, ensure_ascii=False))
        return
    signals = Counter(flag for item in cards for flag in item["signals"])
    blocked = Counter(item.get("blocked_by") for item in cards if item["review"] == "blocked")
    print(f"run={args.run} model={run.get('served_model') or run.get('model')} strategy={args.strategy}")
    print(f"rules={len(cards)} verified={len(cards) - sum(blocked.values())} blocked={dict(blocked)}")
    print(f"source_topic_hint={sum(item['topic_hint'] is not None for item in cards)} signals={dict(signals)}")
    methods = Counter(item["referent_evidence"]["method"] for item in cards if item["referent_evidence"])
    clipped = sum(bool(item["referent_evidence"] and item["referent_evidence"]["clipped"]) for item in cards)
    print(f"local_evidence={dict(methods)} clipped={clipped} unavailable={len(cards) - sum(methods.values())}")
    flagged = [item for item in cards if "possible_unresolved_reference" in item["signals"]]
    print(f"flagged_with_local_evidence={sum(item['referent_evidence'] is not None for item in flagged)}/{len(flagged)}")
    if not args.query:
        missing = [item["text"] for item in flagged if item["referent_evidence"] is None]
        for text in missing[:3]:
            print(f"  flagged without local evidence: {text[:150]}")
    print("Signals are advisory, not a completeness check; every verified card still needs human subject confirmation.")
    if args.query:
        visible = [item for item in cards if args.query.lower() in item["text"].lower()]
    elif args.method:
        visible = [item for item in cards if (item["referent_evidence"] or {}).get("method", "none") == args.method]
    else:
        visible = [item for item in cards if "possible_unresolved_reference" in item["signals"]]
    visible = visible[:args.max_examples]
    for item in visible:
        print(f"\n  rule: {item['text']}")
        hint = item["topic_hint"]
        print(f"  topic hint: {hint['text'] if hint else '(none)'}")
        print(f"  topic from: {hint['source'] if hint else '(none)'}")
        evidence = item["referent_evidence"]
        print(f"  local evidence: {evidence['text'] if evidence else '(none)'}")
        print(f"  evidence from: {evidence['source'] if evidence else '(none)'}")
        print(f"  quote: {(item['quote'] or '')[:240]}")
        print(f"  passage: {item['citation']}  [{item['review']}]")


if __name__ == "__main__":
    main()
