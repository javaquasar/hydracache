"""Offline mutations; verifies the refused isolation status without running models."""
import hashlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_operator as subject


class OperatorReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze();cls.packet=subject.ROOT/subject.OUTPUT
        cls.text=(cls.packet/"broad-first/first/194-controls.log").read_text(encoding="utf-8-sig")
        cls.legacy=(cls.packet/"reference/first/194-controls.log").read_text(encoding="utf-8-sig")

    def test_history_and_scope_refusal_are_explicit(self):
        self.assertEqual(len(self.facts["input_packets"]),21);self.assertEqual(self.facts["input_members_verified"],1183)
        self.assertEqual(self.facts["source_guards"],58)
        self.assertIs(self.facts["targeted_isolation_gate_passed"],False)
        self.assertIs(self.facts["unexpected_old_controls_executed"],True)
        self.assertIs(self.facts["unexpected_allocation_rows_excluded"],True)
        self.assertIs(self.facts["model_rerun_after_argv_repair"],False)
        for lane in self.facts["lanes"]:
            self.assertEqual(len(lane["first_broad"]["rows"]["STATE"]),80)
            self.assertIs(lane["first_broad"]["scoped_model_outcomes_match"],True)

    def test_legacy_priority_and_validation_counterexamples_remain(self):
        rows=subject.legacy_facts(self.legacy)
        self.assertEqual(sum(r["outcome_mismatch"] for r in rows),6)
        self.assertEqual(sum(r["walk_mismatch"] for r in rows),8)
        for before,after in [('"outcome_mismatch":true','"outcome_mismatch":false'),('"expected_walks":3','"expected_walks":2'),('"raw":"invalid"','"raw":"refused"')]:
            with self.assertRaises(ValueError):subject.legacy_facts(self.legacy.replace(before,after))

    def test_failed_state_transition_and_walk_order_mutations_refuse(self):
        for before,after in [('"walks":3','"walks":2'),('"entries_after":65','"entries_after":64'),
            ('"epoch_after":8','"epoch_after":7'),('"outcome":"transition"','"outcome":"refused"'),
            ('"events":88','"events":87'),('"cache_after":0','"cache_after":false')]:
            self.assertTrue(before in self.text)
            with self.assertRaises(ValueError):subject.broad_facts(self.text.replace(before,after))

    def test_initial_fault_precedence_and_publication_cut_state_refuse_drift(self):
        for before,after in [('"mask":63','"mask":62'),('"outcome":"io"','"outcome":"transition"'),
            ('"cut":3','"cut":2'),('"site":"inv-before"','"site":"dir-before"'),
            ('"events":0','"events":1'),('"refused_after":true','"refused_after":false')]:
            with self.assertRaises(ValueError):subject.broad_facts(self.text.replace(before,after))

    def test_broad_run_cannot_be_relabelled_or_missing_rows_hidden(self):
        line=next(l for l in self.text.splitlines() if l.startswith('HC074-OP-FAULT '))
        for changed in (self.text.replace(line+'\n','',1),self.text+'\n'+line,self.text+'\nHC074-OP-UNKNOWN {}',
                        self.text.replace('12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;','3 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out;')):
            with self.assertRaises(ValueError):subject.broad_facts(changed)
        # Unwanted allocation values are not used as new measurements. Their raw
        # logs remain protected by receipt hashes and the outer inventory seal.
        before=subject.broad_facts(self.text)
        line=next(l for l in self.text.splitlines() if 'HC074-OWNED-MAIN ' in l and '"generations":128' in l)
        altered=line.replace('"requested_bytes":4327424','"requested_bytes":9999999')
        self.assertNotEqual(altered,line)
        self.assertEqual(subject.broad_facts(self.text.replace(line,altered)),before)

    def test_native_argv_receipts_and_single_argument_fix_refuse_mutations(self):
        directory=self.packet/"argv";observed=subject.argv_facts(directory)
        self.assertEqual(observed["fixed-candidate"][-1],"model::candidate_controls::candidate_")
        self.assertGreater(len(observed["broken-candidate"]),3)
        row=subject.load(directory/"result.json")
        for changed in (dict(row,model_image_executed=True),dict(row,native_argv_only=False)):
            with patch.object(subject,"load",return_value=changed),self.assertRaises(ValueError):subject.argv_facts(directory)
        with patch.object(subject,"sha",return_value="0"*64),self.assertRaises(ValueError):subject.argv_facts(directory)
        original=Path.read_text
        def altered(path,*args,**kwargs):
            if Path(path)==directory/"fixed-candidate.log":return '"--nocapture"\n"--test-threads=1"\n"m"\n'
            return original(path,*args,**kwargs)
        with patch.object(Path,"read_text",altered),self.assertRaises(ValueError):subject.argv_facts(directory)

    def test_source_output_and_no_clobber_guards(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"one.rs"
            with patch.dict(subject.PINS,{"one.rs":hashlib.sha256(b"fresh();\n").hexdigest()},clear=True):
                path.write_bytes(b"fresh();\r\n");subject.source_pins(directory)
                path.write_bytes(b"cached();\n")
                with self.assertRaises(ValueError):subject.source_pins(directory)
            output=Path(directory)/"facts.json";subject.publish(output,dict(first=True))
            with self.assertRaises(FileExistsError):subject.publish(output,dict(first=False))
        with self.assertRaises(ValueError):subject.require_output(subject.ROOT/subject.previous.OUTPUT/"facts.json")

    def test_original_windows_refusals_and_closed_admission_remain(self):
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual([r["age_ns"] for r in self.facts["first_refusals"]],[10042390247,10010835739])
        self.assertEqual(self.facts["original_owner_profiles"],0)
        for key in ("performance_claim","runtime_candidate_implemented","original_material_gate_passed","full_sigkill_matrix","reconnect_proven","clocks_changed","services_changed","server","product"):
            self.assertIs(self.facts[key],False)


if __name__=="__main__":unittest.main(verbosity=2)
