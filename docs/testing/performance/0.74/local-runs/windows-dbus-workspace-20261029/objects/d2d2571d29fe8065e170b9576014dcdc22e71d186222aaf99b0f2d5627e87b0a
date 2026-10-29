"""Offline mutations only; no baseline or live test is invoked."""
import copy
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review as subject

CHECKS=subject.ROOT / subject.OUTPUT / "first"
if not CHECKS.is_dir():
    CHECKS=subject.ROOT / "target/proc-document-baseline-checks/first"


class ReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze(checks=CHECKS)
        cls.text=(CHECKS / "194-controls.log").read_text(encoding="utf-8-sig")

    def test_exact_first_two_windows_lanes_and_old_seals(self):
        self.assertEqual(len(self.facts["input_packets"]),13)
        self.assertEqual(self.facts["input_members_verified"],770)
        self.assertEqual([r["compiler"] for r in self.facts["lanes"]],["1.94.0","1.88.0"])
        self.assertEqual(self.facts["source_canaries"],40)
        self.assertEqual(self.facts["allocation_baseline_runs"],2)

    def test_main_and_stage_are_not_pooled(self):
        row=self.facts["lanes"][0]["rows"]
        self.assertEqual(row["MAIN"]["instances"],256)
        self.assertEqual(row["MAIN"]["counts"]["requested_bytes"],16777472)
        self.assertEqual(row["STAGES"]["created"]["requested_bytes"],65537)
        self.assertEqual(row["STAGES"]["reused"],subject.counts())
        self.assertTrue(row["STAGES"]["not_pooled"])

    def test_changed_counter_corpus_seed_error_and_gate_refuse(self):
        changes=[('"instances":256','"instances":257'),('"zeroed":256','"zeroed":false'),
                 ('"requested_bytes":16777472','"requested_bytes":16777472.0'),
                 ('"reads_per_instance":4','"reads_per_instance":3'),
                 ('"lengths":[16,37,4096,128]','"lengths":[16,37,128,4096]'),
                 ('"timing_claim":false','"timing_claim":true'),('"server":false','"server":true'),
                 ('"retry":false','"retry":true'),('0x074d5a18','0x074d5a19'),
                 ('"not_pooled":true','"not_pooled":false'),('7 passed; 0 failed','6 passed; 1 failed')]
        for before,after in changes:
            self.assertIn(before,self.text)
            with self.subTest(before=before),self.assertRaises(ValueError):
                subject.log_facts(self.text.replace(before,after))

    def test_missing_duplicate_unknown_and_partial_controls_refuse(self):
        line=next(line for line in self.text.splitlines() if "HC074-PROC-DOCUMENT-MAIN " in line)
        for text in (self.text.replace(line,""),self.text+"\n"+line,
                     self.text+"\nHC074-PROC-DOCUMENT-UNKNOWN {}",
                     self.text.replace("test counter::counter_zeroed_reallocation_and_drop_forwarding ... ok","")):
            with self.assertRaises(ValueError):subject.log_facts(text)

    def test_no_promotion_or_changed_original_refusals(self):
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([row["age_ns"] for row in self.facts["first_refusals"]],[10042390247,10010835739])
        self.assertEqual(self.facts["original_owner_profiles"],0)
        for field in ("runtime_candidate_implemented","full_path_speedup_proven","original_material_gate_passed",
                      "full_sigkill_matrix","clocks_changed","services_changed","server","product"):
            self.assertIs(self.facts[field],False)
        self.assertTrue(all(self.facts["interpretation"].values()))

    def test_receipt_wrong_exit_names_hash_or_platform_refuses(self):
        original=subject.load(CHECKS / "194-result.json")
        for change in (lambda r:r.update(platform="gnu-linux"),lambda r:r.update(failure="first failure"),
                       lambda r:r.update(original_owner_profiles=True),lambda r:r.update(cohort="retry"),
                       lambda r:r["steps"][0].update(exit=True),
                       lambda r:r["steps"][0].update(exit=False),
                       lambda r:r["steps"][0].update(log="../other.log"),
                       lambda r:r["steps"][0].update(sha256="0"*64)):
            row=copy.deepcopy(original)
            change(row)
            with patch.object(subject,"load",return_value=row),self.assertRaises(ValueError):subject.receipt(CHECKS,"194")

    def test_source_drift_refuses_except_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            file=root / "one.rs"
            pin=hashlib.sha256(b"fresh();\n").hexdigest()
            with patch.dict(subject.PROBE_PINS,{"one.rs":pin},clear=True):
                file.write_bytes(b"fresh();\r\n")
                subject.source_pins(root)
                file.write_bytes(b"cached();\n")
                with self.assertRaisesRegex(ValueError,"Probe source drift"):subject.source_pins(root)

    def test_separate_output_and_exclusive_publication(self):
        subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        for name in ("reserved-reconciliation-20261022","held-material-live-20261021"):
            with self.assertRaises(ValueError):subject.require_output(subject.ROOT / subject.BASE / name / "facts.json")
        with tempfile.TemporaryDirectory() as directory:
            file=Path(directory)/"facts.json"
            subject.prior.prior.rounds.prior.publish(file,dict(first=True))
            with self.assertRaises(FileExistsError):subject.prior.prior.rounds.prior.publish(file,dict(first=False))
            self.assertEqual(subject.load(file),dict(first=True))


if __name__ == "__main__":unittest.main(verbosity=2)
