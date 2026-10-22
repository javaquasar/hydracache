"""Offline mutations only; never launches a model image."""
from copy import deepcopy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_targeted as subject


class TargetedReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze();cls.directory=subject.ROOT/subject.OUTPUT/"first/194"
        cls.text=(cls.directory/"controls.log").read_text(encoding="utf-8-sig")
        cls.expected=cls.facts["lanes"][0]["rows"]
        cls.listing=(cls.directory/"listing.log").read_text(encoding="utf-8-sig")

    def test_independent_clean_gate_preserves_red_history(self):
        self.assertEqual(len(self.facts["input_packets"]),22);self.assertEqual(self.facts["input_members_verified"],1282)
        self.assertIs(self.facts["targeted_isolation_gate_passed"],True)
        self.assertIs(self.facts["prior_targeted_isolation_gate_passed"],False)
        self.assertIs(self.facts["old_controls_executed"],False)
        for lane in self.facts["lanes"]:
            self.assertEqual((lane["passed"],lane["filtered"]),(3,9))
            self.assertEqual(sum(map(len,lane["rows"].values())),3296)

    def test_listing_and_native_filter_are_exact(self):
        for changed in (self.listing.replace('candidate_state_priority: test','reference_only: test'),self.listing+'\nother: test\n',self.listing+'\nHC074-OP-STATE {}'):
            with self.assertRaises(ValueError):subject.listing(changed)
        original=Path.read_text
        def changed(path,*args,**kwargs):
            if Path(path)==self.directory/"argv-controls.log":return '"--nocapture"\n"--test-threads=1"\n"m"\n'
            return original(path,*args,**kwargs)
        with patch.object(Path,"read_text",changed),self.assertRaises(ValueError):subject.receipt(self.directory,"194")

    def test_old_controls_and_wrong_completed_membership_refuse(self):
        for changed in (self.text+'\nHC074-OWNED-MAIN {}',self.text+'\nHC074-OP-LEGACY {}',
            self.text.replace('candidate_state_priority ...','reference_only ...'),self.text.replace('3 passed;','12 passed;')):
            with self.assertRaises(ValueError):subject.controls(changed,self.expected)

    def test_missing_duplicate_unknown_rows_refuse(self):
        line=next(s for s in self.text.splitlines() if s.startswith('HC074-OP-FAULT '))
        for changed in (self.text.replace(line+'\n','',1),self.text+'\n'+line,self.text+'\nHC074-OP-UNKNOWN {}'):
            with self.assertRaises(ValueError):subject.controls(changed,self.expected)

    def test_state_trace_round_and_type_mutations_refuse(self):
        for before,after in [('"walks":3','"walks":2'),('"outcome":"io"','"outcome":"transition"'),
            ('"entries_after":65','"entries_after":64'),('"cache_after":0','"cache_after":false'),
            ('"trace_hash":"e7228f7fa9b859c1"','"trace_hash":"0000000000000000"')]:
            self.assertIn(before,self.text)
            with self.assertRaises(ValueError):subject.controls(self.text.replace(before,after),self.expected)

    def test_receipt_hash_source_order_and_scope_refuse(self):
        row=subject.load(self.directory/"result.json")
        changes=[dict(row,source='0'*40),dict(row,old_controls_executed=True),dict(row,selection=['m']),dict(row,model_only=False)]
        wrong=deepcopy(row);wrong['steps'][4],wrong['steps'][5]=wrong['steps'][5],wrong['steps'][4];changes.append(wrong)
        for changed in changes:
            with patch.object(subject,"load",return_value=changed),self.assertRaises(ValueError):subject.receipt(self.directory,"194")
        with patch.object(subject,"sha",return_value='0'*64),self.assertRaises(ValueError):subject.receipt(self.directory,"194")

    def test_source_and_output_isolation_refuse(self):
        with patch.object(subject,"canonical",return_value='drift'),self.assertRaises(ValueError):subject.source_pins(subject.ROOT)
        with tempfile.TemporaryDirectory() as directory:
            output=Path(directory)/"facts.json";subject.publish(output,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(output,dict(first=False))
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.previous.OUTPUT/"facts.json")

    def test_original_windows_refusals_and_admission_unchanged(self):
        self.assertEqual(self.facts['original_windows_seconds'],[300,60,10])
        self.assertEqual([r['age_ns'] for r in self.facts['first_refusals']],[10042390247,10010835739])
        self.assertEqual(self.facts['original_owner_profiles'],0)
        for key in ('performance_claim','runtime_candidate_implemented','original_material_gate_passed','full_sigkill_matrix','reconnect_proven','clocks_changed','services_changed','server','product'):
            self.assertIs(self.facts[key],False)


if __name__=="__main__":unittest.main(verbosity=2)
