"""Offline reviewer mutations only; never executes the synthetic test image."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_read as subject

CHECKS=subject.ROOT/subject.OUTPUT/"first"
if not CHECKS.is_dir():CHECKS=subject.ROOT/"target/authority-read-baseline-checks/first"


class ReadBaselineReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze(checks=CHECKS)
        cls.text=(CHECKS/"194-controls.log").read_text(encoding="utf-8-sig")

    def test_first_lanes_seals_and_separate_weighting(self):
        self.assertEqual(len(self.facts["input_packets"]),17)
        self.assertEqual(self.facts["input_members_verified"],955)
        self.assertEqual(self.facts["source_canaries"],49)
        for lane in self.facts["lanes"]:
            facts=lane["facts"]
            self.assertEqual(facts["main_samples"],2304)
            self.assertEqual(facts["main_totals"],subject.counts(2048,0,0,1222656,2048,1222656))
            self.assertEqual(facts["read_stage_totals_separate"],subject.counts())
            self.assertEqual(facts["stage_samples_per_arm"],9)

    def test_changed_corpus_trace_capacity_counts_and_scalar_types_refuse(self):
        for before,after in [('"samples":256','"samples":255'),('"budget":65','"budget":64'),
            ('"chunk":7','"chunk":8'),('"interrupt_at":1','"interrupt_at":0'),('0x074a17b3','0x074a17b2'),
            ('"output_capacity":37','"output_capacity":38'),('"max_request":32','"max_request":33'),
            ('"allocations":256','"allocations":255'),('"zeroed":0','"zeroed":false'),
            ('"requested_bytes":256','"requested_bytes":256.0'),('"not_pooled":true','"not_pooled":false')]:
            self.assertIn(before,self.text)
            with self.subTest(before=before),self.assertRaises(ValueError):subject.log_facts(self.text.replace(before,after))

    def test_every_first_fault_prefix_and_fake_descriptor_lifetime_remains_exact(self):
        rows=self.facts["lanes"][0]["facts"]["rows"]["FAULT"]
        self.assertEqual(len(rows),96)
        for phase,size in (("read",32),("revalidate",64)):
            group=[row for row in rows if row["phase"]==phase]
            self.assertEqual(len(group),size)
            self.assertEqual(sum(row["outcome"]=="io" for row in group),size-1)
        for before,after in [('"held_live":true','"held_live":false'),('"first_error":"held"','"first_error":"open"'),
            ('"mask":63','"mask":62'),('"read_errors":1','"read_errors":0'),
            ('"outcome":"io"','"outcome":"ok"'),('"drop-fresh"','"drop-held"')]:
            with self.subTest(before=before),self.assertRaises(ValueError):subject.log_facts(self.text.replace(before,after))

    def test_missing_duplicate_unknown_reordered_and_test_outcome_rows_refuse(self):
        lines=self.text.splitlines();index=next(i for i,line in enumerate(lines) if 'HC074-AUTH-READ-MAIN {"case":"one"' in line)
        for changed in ("\n".join(lines[:index]+lines[index+1:]),self.text+"\n"+lines[index],
            self.text+"\nHC074-AUTH-READ-OTHER {}",self.text.replace('"case":"one"','"case":"binary"'),
            self.text.replace('9 passed; 0 failed','8 passed; 1 failed')):
            with self.assertRaises(ValueError):subject.log_facts(changed)

    def test_metadata_eof_partial_and_owned_result_contracts_refuse_drift(self):
        for before,after in [('"held_fields":11','"held_fields":10'),('"parent_device":1','"parent_device":0'),
            ('"over_bound_bytes":66','"over_bound_bytes":65'),('"partial_error_bytes":7','"partial_error_bytes":14'),
            ('"application_retry":false','"application_retry":true'),
            ('"owned_after_token_drop_and_receiver_reset":true','"owned_after_token_drop_and_receiver_reset":false')]:
            with self.subTest(before=before),self.assertRaises(ValueError):subject.log_facts(self.text.replace(before,after))

    def test_source_pins_bind_complete_files_except_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            file=Path(directory)/"one.rs";pin=hashlib.sha256(b"fresh();\n").hexdigest()
            with patch.dict(subject.PINS,{"one.rs":pin},clear=True):
                file.write_bytes(b"fresh();\r\n");subject.source_pins(directory)
                file.write_bytes(b"cached();\n")
                with self.assertRaises(ValueError):subject.source_pins(directory)

    def test_old_windows_refusals_rejection_and_closed_gates_remain(self):
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]],[10042390247,10010835739])
        self.assertEqual(self.facts["previous_boot_decision"],"reject-for-adoption")
        self.assertTrue(all(self.facts["interpretation"].values()))
        self.assertEqual(self.facts["original_owner_profiles"],0)
        for key in ("runtime_candidate_implemented","original_material_gate_passed","full_sigkill_matrix","reconnect_proven",
                    "clocks_changed","services_changed","server","product"):
            self.assertIs(self.facts[key],False)

    def test_exclusive_separate_output_and_changed_check_hashes_refuse(self):
        subject.require_output(subject.ROOT/subject.OUTPUT/"facts.json")
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.BASE/"authority-read-audit-20261022/facts.json")
        with patch.object(Path,"is_symlink",return_value=True),self.assertRaises(ValueError):
            subject.require_output(subject.ROOT/subject.OUTPUT/"facts.json")
        with patch.object(subject.core,"sha",return_value="0"*64),self.assertRaises(ValueError):subject.core.receipt(CHECKS,"194")
        with tempfile.TemporaryDirectory() as directory:
            file=Path(directory)/"facts.json";subject.publish(file,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(file,dict(first=False))


if __name__=="__main__":unittest.main(verbosity=2)
