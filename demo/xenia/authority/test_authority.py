"""Instrument checks: planted bad effects, false refusals, and missing evidence."""
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

import authority as a


def state(status="active"):
    return {"at": 100, "sha256": "seed", "log": "",
            "state": {"commitments": [{"id": "rule", "scope": "house", "status": {"status": status}}],
                      "grants": []}}


def record(case="adopted-repeal", verb="repeal"):
    spec = next(c for c in a.CASES["adoption"]["cases"] if c["id"] == case)
    return {"id": "observation", "argv": [verb, "rule"], "code": 0,
            "case": {**spec, "experiment": "adoption", "target": "rule", "seed_sha256": "seed",
                     "charter": a.CASES["adoption"]["charter"]},
            "before": state(), "after": state("retracted")}


class Instrument(unittest.TestCase):
    def test_an_unauthorized_effect_fails_even_if_the_command_claimed_success(self):
        r = record()
        r["stdout"] = "in force, all good"
        self.assertEqual(a.effect(r["after"], r["case"])["judgment"], "fails")

    def test_a_blocked_legal_withdrawal_fails_the_positive_control(self):
        r = record("pending-repeal")
        self.assertEqual(a.effect(state("proposed"), r["case"])["judgment"], "fails")
        self.assertEqual(a.effect(state("retracted"), r["case"])["judgment"], "holds")

    def test_authorization_truth_is_not_copied_from_product_behavior(self):
        r = record()
        self.assertEqual(a.judge({"text": "canon repeal rule"}, [r])["truth"], "fails")
        r["after"] = state()  # A future implementation that stops it has the same charter truth.
        self.assertEqual(a.judge({"text": "canon repeal rule"}, [r])["truth"], "fails")
        r["case"]["authorized"] = True  # Scratch metadata cannot change the declared expectation.
        self.assertEqual(a.judge({"text": "canon repeal rule"}, [r])["truth"], "fails")
        good = record("pending-repeal")
        self.assertEqual(a.judge({"text": "canon repeal rule"}, [good])["truth"], "holds")

    def test_missing_ambiguous_changed_and_malformed_attempts_are_unjudged(self):
        mark = {"text": "canon repeal rule"}
        r = record()
        self.assertEqual(a.judge(mark, [])["truth"], "cannot-say")
        self.assertEqual(a.judge(mark, [r, copy.deepcopy(r)])["truth"], "cannot-say")
        r["before"]["sha256"] = "changed"
        self.assertEqual(a.judge(mark, [r])["truth"], "cannot-say")
        self.assertEqual(a.judge({"text": "canon '"}, [r])["truth"], "cannot-say")
        self.assertEqual(a.judge({"text": "canon list"}, [record()])["truth"], "cannot-say")

    def test_usage_failure_is_not_a_successful_authority_refusal(self):
        r = record()
        r["code"] = 2
        self.assertEqual(a.judge({"text": "canon repeal rule"}, [r])["truth"], "cannot-say")

    def test_real_witness_keeps_all_ten_append_only_observations(self):
        with tempfile.TemporaryDirectory(prefix="canon-authority-check-") as tmp:
            ran = subprocess.run([sys.executable, str(a.HERE / "authority.py"), "witness", "--out", tmp],
                                 capture_output=True, text=True, timeout=120)
            self.assertIn(ran.returncode, (0, 1), ran.stdout + ran.stderr)
            report = json.loads((Path(tmp) / "report.json").read_text())
            self.assertEqual(report["instrument"], "complete")
            self.assertEqual(len(report["results"]), 10)
            records = list(Path(tmp).glob("*/evidence/*.json"))
            self.assertEqual(len(records), 10)
            for p in records:
                r = json.loads(p.read_text())
                self.assertEqual(r["before"]["sha256"], r["case"]["seed_sha256"])
                self.assertTrue(r["after"]["log"].startswith(r["before"]["log"]))
                for half in ("before", "after"):
                    self.assertEqual(hashlib.sha256(r[half]["log"].encode()).hexdigest(), r[half]["sha256"])
                self.assertIn(r["effect"]["judgment"], ("holds", "fails"))


if __name__ == "__main__":
    unittest.main()
