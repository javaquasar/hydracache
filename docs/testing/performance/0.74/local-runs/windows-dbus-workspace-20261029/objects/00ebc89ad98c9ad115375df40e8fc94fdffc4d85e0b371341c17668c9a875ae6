"""Pure reader mutations only; no boot probe or live observation is invoked."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_boot as subject

CHECKS=subject.ROOT / subject.OUTPUT / "first"
if not CHECKS.is_dir():CHECKS=subject.ROOT / "target/sample-boot-baseline-checks/first"


class ReviewBootTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze(checks=CHECKS)
        cls.text=(CHECKS / "194-controls.log").read_text(encoding="utf-8-sig")

    def test_exact_first_windows_cohort_and_fourteen_old_seals(self):
        self.assertEqual(len(self.facts["input_packets"]),14)
        self.assertEqual(self.facts["input_members_verified"],813)
        self.assertEqual([r["compiler"] for r in self.facts["lanes"]],["1.94.0","1.88.0"])
        self.assertEqual(self.facts["source_canaries"],43)
        self.assertEqual(self.facts["allocation_baseline_runs"],2)

    def test_same_corpus_totals_and_fixed_shape_subset_not_new_operations(self):
        row=self.facts["lanes"][0]["facts"]
        self.assertEqual(row["main_samples"],2304)
        self.assertEqual(row["main_totals"],subject.counts(2048,0,0,324864,2048,324864))
        self.assertEqual(row["fixed_shape_samples"],512)
        self.assertEqual(row["fixed_shape_subset"],subject.counts(512,0,0,18432,512,18432))
        self.assertEqual(row["stage_samples_per_arm_separate"],9)
        for stage in row["rows"]["STAGES"]:
            self.assertEqual(stage["frame"],subject.counts())
            self.assertEqual(stage["cloned"],stage["whole_returning"])
            self.assertEqual(stage["dropped"],stage["whole_drop"])

    def test_changed_case_counts_capacity_flags_seed_and_error_contract_refuse(self):
        for before,after in [
            ('"samples":256','"samples":255'),('"length":35','"length":34'),
            ('"input_capacity":256','"input_capacity":36'),('"output_capacity":36','"output_capacity":256'),
            ('"zeroed":0','"zeroed":false'),('"requested_bytes":256','"requested_bytes":256.0'),
            ('"not_pooled":true','"not_pooled":false'),('"timing_claim":false','"timing_claim":true'),
            ('"server":false','"server":true'),('"refused":15','"refused":14'),
            ('"first_error_prefix":true','"first_error_prefix":false'),('0x074b0a19','0x074b0a18'),
            ('"invalid_bytes":7','"invalid_bytes":6'),('7 passed; 0 failed','6 passed; 1 failed')]:
            self.assertIn(before,self.text)
            with self.subTest(before=before),self.assertRaises(ValueError):subject.log_facts(self.text.replace(before,after))

    def test_missing_duplicate_unknown_and_reordered_rows_refuse(self):
        lines=self.text.splitlines()
        index=next(i for i,line in enumerate(lines) if 'HC074-SAMPLE-BOOT-MAIN {"case":"one"' in line)
        for text in ("\n".join(lines[:index]+lines[index+1:]), self.text+"\n"+lines[index],
                     self.text+"\nHC074-SAMPLE-BOOT-OTHER {}",
                     self.text.replace('"case":"one"','"case":"below"'),
                     self.text.replace("test counter::counter_zeroed_reallocation_and_drop_forwarding ... ok","")):
            with self.assertRaises(ValueError):subject.log_facts(text)

    def test_original_windows_refusals_and_gates_preserved(self):
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]],[10042390247,10010835739])
        self.assertTrue(all(self.facts["interpretation"].values()))
        self.assertEqual(self.facts["original_owner_profiles"],0)
        for field in ("runtime_candidate_implemented","representation_changed","full_path_speedup_proven",
                      "original_material_gate_passed","full_sigkill_matrix","clocks_changed","services_changed","server","product"):
            self.assertIs(self.facts[field],False)

    def test_probe_source_drift_refuses_except_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);file=root / "one.rs"
            pin=hashlib.sha256(b"fresh();\n").hexdigest()
            with patch.dict(subject.PROBE_PINS,{"one.rs":pin},clear=True):
                file.write_bytes(b"fresh();\r\n");subject.source_pins(root)
                file.write_bytes(b"cached();\n")
                with self.assertRaises(ValueError):subject.source_pins(root)

    def test_separate_output_and_links_refuse(self):
        subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        for name in ("proc-document-baseline-20261022","held-material-live-20261021"):
            with self.assertRaises(ValueError):subject.require_output(subject.ROOT / subject.BASE / name / "facts.json")
        with patch.object(Path,"is_symlink",return_value=True),self.assertRaises(ValueError):
            subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")

    def test_exclusive_publication_does_not_replace_a_first_outcome(self):
        with tempfile.TemporaryDirectory() as directory:
            file=Path(directory)/"facts.json"
            subject.publish(file,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(file,dict(first=False))
            self.assertEqual(subject.load(file),dict(first=True))


if __name__=="__main__":unittest.main(verbosity=2)
