"""The review-card spike must not invent or misattribute provenance."""

import runpy
import tempfile
import unittest
from pathlib import Path


module = runpy.run_path(str(Path(__file__).with_name("subject-spike.py")))
Sources, card = module["Sources"], module["card"]


class SubjectSpikeTest(unittest.TestCase):
    def test_an_inline_referent_is_cut_from_the_source_not_rewritten(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            body = "The cache already exists. Do not use this for a replay."
            (root / "note.md").write_text(f"# Replay cache\n\n{body}\n")
            item = card(
                {"text": "Do not use this for a replay.", "source": "note.md:3-3",
                 "quote": "Do not use this for a replay."},
                Sources(root),
            )
            evidence = item["referent_evidence"]
            self.assertEqual(evidence["method"], "same_line_prefix")
            self.assertEqual(evidence["text"], "The cache already exists.")
            self.assertEqual(evidence["source"], "note.md:3@1-25")
            self.assertEqual(body[:25], evidence["text"])

    def test_only_a_verifiable_source_hint_can_reach_review(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "note.md").write_text("# Replay cache\n\nDo not use this for a replay.\n")
            sources = Sources(root)
            candidate = {
                "text": "Do not use this for a replay.",
                "source": "note.md:3-3",
                "quote": "Do not use this for a replay.",
            }
            good = card(candidate, sources)
            self.assertEqual(good["review"], "human_confirm_subject")
            self.assertEqual(good["topic_hint"]["source"], "note.md:1")
            self.assertIn("possible_unresolved_reference", good["signals"])

            forged = card({**candidate, "quote": "Not what the source says."}, sources)
            self.assertEqual(forged["review"], "blocked")
            self.assertEqual(forged["blocked_by"], "quote no longer in cited source span")

            escaped = card({**candidate, "source": "../note.md:3-3"}, sources)
            self.assertEqual(escaped["review"], "blocked")
            self.assertEqual(escaped["blocked_by"], "unsafe source path")

    def test_an_adjacent_line_can_help_but_a_repeated_quote_cannot_be_located(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "note.md").write_text(
                "# FaithBench\n\nFaithBench recalibration was falsified.\n"
                "Do not re-open it as a threshold question.\n"
            )
            sources = Sources(root)
            candidate = {"text": "Do not re-open it as a threshold question.",
                         "source": "note.md:4-4", "quote": "Do not re-open it as a threshold question."}
            item = card(candidate, sources)
            self.assertEqual(item["referent_evidence"]["method"], "adjacent_line")
            self.assertEqual(item["referent_evidence"]["text"], "FaithBench recalibration was falsified.")

            (root / "repeated.md").write_text("# Topic\n\nAgain. Again.\n")
            ambiguous = card({"text": "Again.", "quote": "Again.",
                              "source": "repeated.md:3-3"}, Sources(root))
            self.assertIsNone(ambiguous["referent_evidence"])
            self.assertIn("no_unique_local_evidence", ambiguous["signals"])

    def test_same_line_suffix_is_cited_and_paragraph_crossing_is_opt_in(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "suffix.md").write_text(
                "# Topic\n\nDo not re-open it. The FaithBench hypothesis failed.\n"
            )
            suffix = card({"text": "Do not re-open it.", "quote": "Do not re-open it.",
                           "source": "suffix.md:3-3"}, Sources(root))
            self.assertEqual(suffix["referent_evidence"]["method"], "same_line_suffix")
            self.assertEqual(suffix["referent_evidence"]["text"], "The FaithBench hypothesis failed.")

            (root / "paragraph.md").write_text(
                "# Topic\n\nThe FaithBench hypothesis failed.\n\nDo not re-open it.\n"
            )
            proposal = {"text": "Do not re-open it.", "quote": "Do not re-open it.",
                        "source": "paragraph.md:5-5"}
            self.assertIsNone(card(proposal, Sources(root))["referent_evidence"])
            weaker = card(proposal, Sources(root), "extended")["referent_evidence"]
            self.assertEqual(weaker["method"], "preceding_paragraph")
            self.assertEqual(weaker["source"], "paragraph.md:3@1-33")

    def test_a_flat_name_that_resolves_to_two_sources_is_blocked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for folder in ("attempt", "memory"):
                (root / folder).mkdir()
                (root / folder / "same.md").write_text("# Different topic\n\nA rule exists.\n")
            result = card(
                {"text": "A rule exists.", "source": "same.md:3-3", "quote": "A rule exists."},
                Sources(root),
            )
            self.assertEqual(result["review"], "blocked")
            self.assertEqual(result["blocked_by"], "source name ambiguous")


if __name__ == "__main__":
    unittest.main()
