import importlib.util
from pathlib import Path
import unittest


spec = importlib.util.spec_from_file_location("contract_check", Path(__file__).parents[1] / "contract-check.py")
contract = importlib.util.module_from_spec(spec)
spec.loader.exec_module(contract)


class Classification(unittest.TestCase):
    def decide(self, new_surface=None, new_laws=None, checks=True, observations=True, consumers=True):
        return contract.classification({"Order": "shape"}, new_surface or {"Order": "shape"},
            {"identity": "stable IDs"}, new_laws or {"identity": "stable IDs"},
            checks, observations, consumers)[0]

    def test_same_checked_contract_is_preserved(self):
        self.assertEqual(self.decide(), "preserves_checked_contract")

    def test_new_surface_with_existing_promises_intact_extends(self):
        self.assertEqual(self.decide({"Order": "shape", "query": "signature"}), "extends_checked_contract")

    def test_changed_record_results_are_an_observed_break(self):
        self.assertEqual(self.decide(observations=False), "breaks_checked_contract")

    def test_old_consumers_rejected_is_an_observed_break(self):
        self.assertEqual(self.decide(consumers=False), "breaks_checked_contract")

    def test_baseline_law_counterexample_is_an_observed_break(self):
        result, evidence = contract.classification({"Order": "shape"}, {"Order": "shape"},
            {"identity": "stable"}, {"identity": "stable"}, False, True, True, ["identity"])
        self.assertEqual(result, "breaks_checked_contract")
        self.assertEqual(evidence["violated_laws"], ["identity"])

    def test_missing_public_item_or_law_is_a_break(self):
        result, _ = contract.classification({"Order": "shape"}, {}, {"identity": "stable"}, {}, True, True, True)
        self.assertEqual(result, "breaks_checked_contract")

    def test_changed_words_or_signature_are_not_guessed_compatible(self):
        self.assertEqual(self.decide({"Order": "different shape"}), "not_established")
        self.assertEqual(self.decide(new_laws={"identity": "different promise"}), "not_established")

    def test_unavailable_evidence_does_not_pass(self):
        self.assertEqual(self.decide(checks=False), "not_established")

    def test_new_law_without_baseline_owned_checks_is_not_established(self):
        self.assertEqual(self.decide(new_laws={"identity": "stable IDs", "new_law": "a new promise"}), "not_established")

    def test_candidate_cannot_rewrite_the_format_specification_and_claim_preservation(self):
        result, evidence = contract.classification({"Order": "shape"}, {"Order": "shape"},
            {"identity": "stable"}, {"identity": "stable"}, True, True, True,
            specification_unchanged=False)
        self.assertEqual(result, "not_established")
        self.assertTrue(evidence["specification_changed"])

    def test_duplicate_law_ids_are_refused(self):
        with self.assertRaises(ValueError):
            contract.laws("| `identity` | stable | test |\n| `identity` | different | test |")

    def test_dev_dependencies_come_from_the_baseline(self):
        candidate = '[package]\nname="x"\n[dev-dependencies]\nproptest="99"\n[features]\none=[]\n'
        baseline = '[package]\nname="x"\n[dev-dependencies]\nproptest="1"\n'
        merged = contract.replace_dev_dependencies(candidate, baseline)
        self.assertIn('proptest="1"', merged)
        self.assertNotIn('proptest="99"', merged)
        self.assertIn('[features]\none=[]', merged)


if __name__ == "__main__":
    unittest.main()
