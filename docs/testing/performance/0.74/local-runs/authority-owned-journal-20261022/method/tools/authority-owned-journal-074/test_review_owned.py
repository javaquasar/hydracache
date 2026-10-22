"""Offline first-result mutations only; never reruns a measured image."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_owned as subject


class OwnedReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze();cls.packet=subject.ROOT/subject.OUTPUT
        cls.reference_text=(cls.packet/"reference/first/194-controls.log").read_text(encoding="utf-8-sig")
        cls.text=(cls.packet/"candidate/first/194-controls.log").read_text(encoding="utf-8-sig")
        cls.reference=subject.reference_facts(cls.reference_text)

    def test_history_compilers_and_separate_healthy_eviction_counts(self):
        self.assertEqual(len(self.facts["input_packets"]),20);self.assertEqual(self.facts["input_members_verified"],1111)
        self.assertEqual(self.facts["source_canaries"],55)
        for lane in self.facts["lanes"]:
            totals=lane["candidate"]["main_totals"]
            self.assertEqual(totals["reference"],subject.counts(6752,0,0,7133488,6752,7133488))
            self.assertEqual(totals["candidate"],subject.counts(422,0,0,445843))
            self.assertEqual(totals["eviction"],subject.counts(deallocations=422,deallocated_bytes=445843))

    def test_reference_first_cohort_bounds_and_counts_refuse_mutations(self):
        for before,after in [('"rounds":16','"rounds":15'),('"generations":128','"generations":129'),
            ('"requested_bytes":4327424','"requested_bytes":4327423'),('"held":256','"held":255'),
            ('"fresh":0','"fresh":false')]:
            self.assertTrue(before in self.reference_text)
            with self.assertRaises(ValueError):subject.reference_facts(self.reference_text.replace(before,after))

    def test_cold_candidate_and_trace_correlation_refuse_mutations(self):
        for before,after in [('"rounds":16','"rounds":17'),('"allocations":256','"allocations":255'),
            ('"requested_bytes":270464','"requested_bytes":270463'),('"events":2052','"events":2051'),
            ('"hash":"b4b010d909b70f4b"','"hash":"b4b010d909b70f4c"'),('"zeroed":0','"zeroed":false')]:
            self.assertTrue(before in self.text)
            with self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after),self.reference)

    def test_fault_priority_whole_cache_eviction_and_gate_coverage_refuse(self):
        for before,after in [('"mask":63','"mask":62'),('"first_error":"held"','"first_error":"open"'),
            ('"site":"dir-before"','"site":"dir-after"'),('"site":"external"','"site":"bind"'),
            ('"deallocated_bytes":270464','"deallocated_bytes":0'),('"outcome":"io"','"outcome":"ok"')]:
            self.assertTrue(before in self.text)
            with self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after),self.reference)

    def test_layout_logical_reopen_and_no_new_measurement_claims(self):
        for before,after in [('"owned_leaf":176','"owned_leaf":184'),('"reopen_new_tokens":true','"reopen_new_tokens":false'),
            ('"append_failure_sticky":true','"append_failure_sticky":false'),('"cases":582','"cases":581'),
            ('"operator_state_distinct":true','"operator_state_distinct":false'),('"send":false','"send":true')]:
            with self.assertRaises(ValueError):subject.candidate_facts(self.text.replace(before,after),self.reference)
        self.assertEqual(self.facts["measurement_retries"],0)
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]],[10042390247,10010835739])
        for k in ("new_timing_claim","physical_copy_measured","rss_measured","real_linux_layout_measured","runtime_candidate_implemented",
                  "original_material_gate_passed","full_sigkill_matrix","reconnect_proven","clocks_changed","services_changed","server","product"):
            self.assertIs(self.facts[k],False)

    def test_unknown_missing_duplicate_and_wrong_test_selection_refuse(self):
        line=next(l for l in self.text.splitlines() if l.startswith('HC074-OWNED-FAULT '))
        for changed in (self.text.replace(line+'\n','',1),self.text+'\n'+line,self.text+'\nHC074-OWNED-UNKNOWN {}',
                        self.text.replace('7 passed; 0 failed','6 passed; 1 failed')):
            with self.assertRaises(ValueError):subject.candidate_facts(changed,self.reference)

    def test_first_red_reference_receipt_is_preserved_not_relabelled_green(self):
        directory=self.packet/"reference/first";original=subject.load(directory/"194-result.json")
        self.assertEqual(original["steps"][-1]["exit"],101)
        subject.receipt(directory,"194","reference")
        for changed in (dict(original,failure=None),dict(original,lane="188"),dict(original,product=True)):
            with patch.object(subject,"load",return_value=changed),self.assertRaises(ValueError):subject.receipt(directory,"194","reference")
        with patch.object(subject,"sha",return_value="0"*64),self.assertRaises(ValueError):subject.receipt(directory,"194","reference")

    def test_source_pins_separate_output_and_no_clobber(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"one.rs"
            with patch.dict(subject.PINS,{"one.rs":hashlib.sha256(b"owned();\n").hexdigest()},clear=True):
                path.write_bytes(b"owned();\r\n");subject.source_pins(directory)
                path.write_bytes(b"cached();\n")
                with self.assertRaises(ValueError):subject.source_pins(directory)
            output=Path(directory)/"facts.json";subject.publish(output,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(output,dict(first=False))
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.previous.OUTPUT/"facts.json")
        with patch.object(Path,"is_symlink",return_value=True),self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.OUTPUT/"facts.json")


if __name__=="__main__":unittest.main(verbosity=2)
