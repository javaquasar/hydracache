"""Mutation tests for saved observations only; no measurement reruns."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_reuse as subject


class ScratchReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze()
        packet=subject.ROOT/subject.OUTPUT
        cls.text=(packet/"candidate/first/194-controls.log").read_text(encoding="utf-8-sig")
        cls.reference=(packet/"reference/first/194-controls.log").read_text(encoding="utf-8-sig")

    def test_seals_counts_and_unpromoted_decision(self):
        self.assertEqual(len(self.facts["input_packets"]),18)
        self.assertEqual(self.facts["input_members_verified"],1000)
        self.assertEqual(self.facts["source_canaries"],52)
        self.assertEqual(self.facts["decision"],"retain-unpromoted-prototype")
        for lane in self.facts["lanes"]:
            self.assertEqual(lane["candidate"]["main_candidate"],subject.counts(8,0,0,4776))
            self.assertEqual(lane["candidate"]["main_reference"],subject.counts(2048,0,0,1222656,2048,1222656))

    def test_cold_main_and_separate_drop_mutations_refuse(self):
        for before,after in [('"samples":256','"samples":255'),('"cold":true','"cold":false'),
            ('"cache_capacity":37','"cache_capacity":38'),('"allocations":256','"allocations":255'),
            ('"zeroed":0','"zeroed":false'),('"requested_bytes":37','"requested_bytes":37.0')]:
            self.assertIn(before,self.text)
            with self.subTest(before=before),self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after))

    def test_warm_fault_eviction_first_error_and_owner_lifetime_refuse_drift(self):
        for before,after in [('"cached":null','"cached":65'),('"mask":63','"mask":62'),
            ('"first_error":"held"','"first_error":"open"'),('"held_live":true','"held_live":false'),
            ('"owner_bytes_unchanged":true','"owner_bytes_unchanged":false'),('"size":32','"size":24')]:
            with self.subTest(before=before),self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after))

    def test_schedule_and_drift_membership_and_bounds_refuse(self):
        for before,after in [('"declared":66','"declared":65'),('"body":67','"body":68'),
            ('"max_request":65','"max_request":67'),('"metadata":28','"metadata":27'),('"content":256','"content":255')]:
            self.assertIn(before,self.text)
            with self.subTest(before=before),self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after))

    def test_paired_cost_protocol_and_duration_types_refuse(self):
        for before,after in [('"warm":64','"warm":63'),('"calls":512','"calls":511'),
            ('"candidate_first":true','"candidate_first":false'),('"checksum":33280','"checksum":33281')]:
            with self.subTest(before=before),self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after))
        import re
        for value in ('false','0','1.5'):
            changed=re.sub(r'"candidate_ns":\d+', '"candidate_ns":'+value,self.text)
            with self.assertRaises(ValueError):subject.candidate_facts(changed)
        for before,after in [('"sample":6','"sample":5'),('"warm":64','"warm":0'),('"allocations":512','"allocations":511')]:
            with self.assertRaises(ValueError):subject.preflight_facts(self.reference.replace(before,after))

    def test_exact_test_selection_and_missing_duplicate_unknown_rows_refuse(self):
        line=next(l for l in self.text.splitlines() if l.startswith('HC074-SCRATCH-IO '))
        for changed in (self.text.replace(line+'\n','',1),self.text+'\n'+line,
            self.text+'\nHC074-SCRATCH-UNKNOWN {}',self.text.replace('15 passed; 0 failed','14 passed; 1 failed')):
            with self.assertRaises(ValueError):subject.candidate_facts(changed)

    def test_source_receipts_and_exclusive_output_guards(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"one.rs"
            with patch.dict(subject.PINS,{"one.rs":hashlib.sha256(b"fresh();\n").hexdigest()},clear=True):
                path.write_bytes(b"fresh();\r\n");subject.source_pins(directory)
                path.write_bytes(b"cached();\n")
                with self.assertRaises(ValueError):subject.source_pins(directory)
            facts=Path(directory)/"facts.json";subject.publish(facts,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(facts,dict(first=False))
        with patch.object(subject.core,"sha",return_value="0"*64),self.assertRaises(ValueError):
            subject.core.receipt(subject.ROOT/subject.OUTPUT/"candidate/first","194")
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.BASE/"authority-read-baseline-20261022/facts.json")

    def test_old_refusals_and_closed_live_gates_preserved(self):
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]],[10042390247,10010835739])
        self.assertEqual(self.facts["original_owner_profiles"],0)
        for key in ("runtime_candidate_implemented","original_material_gate_passed","full_sigkill_matrix","reconnect_proven",
                    "clocks_changed","services_changed","server","product","rss_measured","physical_copy_bytes_measured"):
            self.assertIs(self.facts[key],False)


if __name__=="__main__":unittest.main(verbosity=2)
