"""Offline mutations only: no test image, kernel observation or external action."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_inline as subject

CHECKS = subject.ROOT / subject.OUTPUT
if not (CHECKS / "candidate").is_dir():
    CHECKS = subject.ROOT / "target/sample-boot-inline-checks"


class InlineReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = subject.analyze(checks=CHECKS)
        cls.text = (CHECKS / "candidate/194-controls.log").read_text(encoding="utf-8-sig")
        cls.reference = (CHECKS / "reference/194-controls.log").read_text(encoding="utf-8-sig")

    def test_first_cohort_and_predeclared_rejection(self):
        self.assertEqual(len(self.facts["input_packets"]), 15)
        self.assertEqual(self.facts["input_members_verified"], 856)
        self.assertEqual(self.facts["source_canaries"], 46)
        self.assertEqual(self.facts["decision"], "reject-for-adoption")
        self.assertTrue(self.facts["first_canary_refusal_preserved"])
        for lane in self.facts["lanes"]:
            totals = lane["candidate"]["totals"]
            self.assertEqual(totals["reference"], subject.counts(2048, 0, 0, 324864, 2048, 324864))
            self.assertEqual(totals["candidate"], subject.counts(768, 0, 0, 288000, 768, 288000))

    def test_counts_shape_ownership_seed_and_passing_selection_refuse_drift(self):
        for before, after in [
            ('"heap_capacity":37', '"heap_capacity":0'), ('"input_capacity":256', '"input_capacity":36'),
            ('"samples_per_arm":256', '"samples_per_arm":255'), ('"candidate_size":56', '"candidate_size":40'),
            ('"generic":246', '"generic":245'), ('0x074b0a19', '0x074b0a18'),
            ('"same_first_error_and_events":true', '"same_first_error_and_events":false'),
            ('"allocations":256', '"allocations":256.0'), ('"zeroed":0', '"zeroed":false'),
            ('12 passed; 0 failed', '11 passed; 1 failed')]:
            self.assertIn(before, self.text)
            with self.subTest(before=before), self.assertRaises(ValueError):
                subject.candidate_facts(self.text.replace(before, after))

    def test_reference_only_preflight_has_exact_shape(self):
        facts = subject.reference_facts(self.reference)
        self.assertEqual(len(facts["rows"]["PREFLIGHT"]), 7)
        for before, after in [('"point_size":40', '"point_size":56'), ('"clone_calls":256', '"clone_calls":255'),
                              ('7 filtered out', '0 filtered out')]:
            with self.assertRaises(ValueError):
                subject.reference_facts(self.reference.replace(before, after))

    def test_missing_duplicate_unknown_and_reordered_intervals_refuse(self):
        lines = self.text.splitlines()
        i = next(i for i, line in enumerate(lines) if 'HC074-BOOT-INLINE-GETTERS {"sample":1' in line)
        for text in ("\n".join(lines[:i] + lines[i+1:]), self.text + "\n" + lines[i],
                     self.text + "\nHC074-BOOT-INLINE-OTHER {}", self.text.replace('"sample":1', '"sample":0')):
            with self.assertRaises(ValueError):
                subject.candidate_facts(text)

    def test_timer_protocol_rejects_invalid_types_and_work_or_order_drift(self):
        rows = subject.candidate_facts(self.text)["rows"]["GETTERS"]
        for field, value in (("iterations", 262143), ("warmup", 0), ("checksum", 0),
                             ("order", "candidate-first"), ("allocator_capture_active", True),
                             ("reference_wall_ns", False), ("candidate_wall_ns", -1), ("candidate_wall_ns", 1.5)):
            changed = copy.deepcopy(rows)
            changed[0][field] = value
            with self.subTest(field=field, value=value), self.assertRaises(ValueError):
                subject.intervals(changed, True)

    def test_receipt_hash_phase_scope_and_first_failure_are_not_relabelled(self):
        row = subject.load(CHECKS / "candidate/194-result.json")
        for field, value in (("phase", "reference"), ("cohort", "second"), ("server", True),
                             ("runtime_candidate_implemented", True), ("original_owner_profiles", 1)):
            changed = copy.deepcopy(row)
            changed[field] = value
            with patch.object(subject, "load", return_value=changed), self.assertRaises(ValueError):
                subject.receipt(CHECKS / "candidate", "194", "candidate")
        with patch.object(subject, "sha", return_value="0"*64), self.assertRaises(ValueError):
            subject.receipt(CHECKS / "candidate", "194", "candidate")
        with self.assertRaises(ValueError):
            subject.receipt(CHECKS / "candidate", "host", "candidate")

    def test_windows_first_refusals_and_closed_gates_stay_unchanged(self):
        self.assertEqual(self.facts["original_windows_seconds"], [300, 60, 10])
        self.assertEqual([row["age_ns"] for row in self.facts["first_refusals"]], [10042390247, 10010835739])
        self.assertEqual(self.facts["measurement_retries"], 0)
        self.assertEqual(self.facts["original_owner_profiles"], 0)
        for field in ("runtime_candidate_implemented", "full_path_speedup_proven", "original_material_gate_passed",
                      "full_sigkill_matrix", "clocks_changed", "services_changed", "server", "product"):
            self.assertIs(self.facts[field], False)

    def test_separate_exclusive_publication_and_source_pins(self):
        with tempfile.TemporaryDirectory() as directory:
            file = Path(directory) / "facts.json"
            subject.publish(file, dict(first=True))
            with self.assertRaises(FileExistsError):
                subject.publish(file, dict(first=False))
            file.write_text("drift")
            with self.assertRaises(ValueError):
                subject.source_pins(directory, {"facts.json": "0"*64})
        subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        with self.assertRaises(ValueError):
            subject.require_output(subject.ROOT / subject.BASE / "sample-boot-baseline-20261022/facts.json")
        with patch.object(Path, "is_symlink", return_value=True), self.assertRaises(ValueError):
            subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")


if __name__ == "__main__":
    unittest.main()
