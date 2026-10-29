"""Offline receipt mutations; never reruns the historical worker."""
from copy import deepcopy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import review_first as subject


class FirstReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts=subject.analyze();cls.m=subject.view.manifest();cls.directory=subject.ROOT/subject.OUTPUT/'first'
        cls.row=subject.load(cls.directory/'result.json')

    def test_history_and_current_admission_are_not_pooled(self):
        self.assertEqual((len(self.facts['input_packets']),self.facts['input_members_verified']),(24,1423))
        self.assertIs(self.facts['historical_replay_passed'],True)
        self.assertIs(self.facts['current_runtime_admitted'],False)
        self.assertIs(self.facts['runtime_candidate_guard_implemented'],False)

    def altered(self,row):
        with patch.object(subject,'load',side_effect=lambda p:row if Path(p).name=='result.json' else subject.view.decode(Path(p).read_bytes())),self.assertRaises(ValueError):
            subject.receipt(self.directory,self.m)

    def test_source_manifest_and_failure_receipts_refuse(self):
        for key,value in [('history_source','0'*40),('manifest_sha256','0'*64),('tooling_source','unknown'),('failure','refused')]:
            changed=deepcopy(self.row);changed[key]=value;self.altered(changed)

    def test_worker_digest_and_decoded_membership_drift_refuse(self):
        changed=deepcopy(self.row);changed['worker_stdout_sha256']='0'*64;self.altered(changed)
        changed=deepcopy(self.row);changed['worker']['history_only']=False;self.altered(changed)

    def test_false_candidate_and_live_flags_refuse(self):
        for key in ('runtime_candidate_implemented','runtime_candidate_guard_implemented','server','product'):
            changed=deepcopy(self.row);changed[key]=True;self.altered(changed)
        changed=deepcopy(self.row);changed['model_images_executed']=1;self.altered(changed)
        changed=deepcopy(self.row);changed['current_baseline']['runtime_candidate_guard_implemented']=True;self.altered(changed)

    def test_generated_retirement_and_archive_bounds_refuse(self):
        changed=deepcopy(self.row);changed['generated_view_removed']=False;self.altered(changed)
        changed=deepcopy(self.row);changed['archive']['bytes']=self.m['bounds']['archive_bytes']+1;self.altered(changed)

    def test_sealed_inventory_or_method_source_drift_refuses(self):
        original=Path.read_bytes
        def changed(path,*args,**kwargs):
            if Path(path).name=='inventory.json':return original(path,*args,**kwargs)+b' '
            return original(path,*args,**kwargs)
        with patch.object(Path,'read_bytes',changed),self.assertRaises(ValueError):subject.analyze()
        def source_changed(path,*args,**kwargs):
            if Path(path)==subject.ROOT/'tools/authority-history-view-074/worker.py':return b'drift'
            return original(path,*args,**kwargs)
        with patch.object(Path,'read_bytes',source_changed),self.assertRaises(ValueError):subject.analyze()

    def test_output_isolation_and_no_clobber(self):
        with tempfile.TemporaryDirectory(dir=subject.ROOT/'target',prefix='ahv-publish-') as directory:
            path=Path(directory)/'facts.json'
            with self.assertRaises(ValueError):subject.publish(path,{})
        output=subject.ROOT/subject.OUTPUT/'facts.json'
        with self.assertRaises(FileExistsError):subject.publish(output,self.facts)

    def test_original_windows_refusals_and_closed_real_gates(self):
        self.assertEqual(self.facts['original_windows_seconds'],[300,60,10])
        self.assertEqual([r['age_ns'] for r in self.facts['first_refusals']],[10042390247,10010835739])
        for key in ('old_measurements_reexecuted','full_sigkill_matrix','reconnect_proven','clocks_changed','services_changed','server','product'):
            self.assertIs(self.facts[key],False)


if __name__=='__main__':unittest.main(verbosity=2)
