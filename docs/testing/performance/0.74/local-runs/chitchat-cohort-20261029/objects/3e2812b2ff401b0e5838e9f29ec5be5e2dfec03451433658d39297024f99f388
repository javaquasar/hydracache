"""Mutate saved receipts only: no server action, deadline or sample rerun."""
import json
import unittest
from pathlib import Path
import analyze as a


class Accounting(unittest.TestCase):
    def check_change(self,name,change):
        body=json.loads(a.local(name));change(body);raw=json.dumps(body).encode()
        with self.assertRaises((ValueError,RuntimeError,KeyError,AssertionError)):
            a.reconcile(lambda n:raw if n==name else a.local(n))

    def nr(self,name):return a.R1+'/collection/'+a.NUMERIC+'/'+name

    def test_incomplete_is_terminal_and_never_estimated(self):
        v=a.reconcile();self.assertTrue(v['terminal_accounting_complete']);self.assertFalse(v['complete'])
        self.assertEqual((v['worker_intents'],v['completed_cpu_images'],v['completed_memory_images']),(5,4,0))
        for k in ('estimator_applied','CPU_qualified','RSS_qualified','root_adopted','release_admitted','retry'):self.assertFalse(v[k])

    def test_partial_cohort_cannot_be_promoted(self):
        name=self.nr('cohort-complete.json')
        with self.assertRaises(ValueError):a.reconcile(lambda n:b'{"complete":true}' if n==name else a.local(n))

    def test_later_intent_cannot_be_spliced(self):
        name=self.nr('5.intent.json')
        with self.assertRaises(ValueError):a.reconcile(lambda n:b'{}' if n==name else a.local(n))

    def test_lost_empty_proof_refused(self):
        self.check_change(self.nr('4.retired.json'),lambda v:v['events'].update(populated='1'))

    def test_missing_fence_proof_refused(self):
        self.check_change(self.nr('manual-recovery-fence.json'),lambda v:v.update(original_fence_acquired=False))

    def test_absent_group_cannot_be_called_empty(self):
        self.check_change(self.nr('manual-recovery-fence.json'),lambda v:v.update(missing_cgroup_used_as_empty_proof=True))

    def test_measurement_admission_cannot_be_granted_by_recovery(self):
        self.check_change(self.nr('manual-restored.json'),lambda v:v.update(measurement_frame_pass=True))

    def test_protected_host_change_refused(self):
        self.check_change(self.nr('manual-restored.json'),lambda v:v['host'].update(boot='new boot'))

    def test_changed_deadline_refused(self):
        self.check_change(self.nr('session-deadline.json'),lambda v:v.update(deadline_ns=v['deadline_ns']+1))

    def test_original_child_identity_mismatch_refused(self):
        self.check_change(self.nr('0.worker-child.json'),lambda v:v.update(pid=v['pid']+1))

    def test_original_clock_predicate_stays_active(self):
        folder=a.R1+'/collection/'+a.WORKER+'/0.stdout.log'
        raw=a.local(folder).decode();line=next(x for x in raw.splitlines() if x.startswith('HC074_CLEAN_COST:'))
        value=json.loads(line.removeprefix('HC074_CLEAN_COST:'));value['elapsed']['mono']=value['elapsed']['raw']*2
        changed=raw.replace(line,'HC074_CLEAN_COST:'+json.dumps(value)).encode()
        with self.assertRaises(ValueError):a.reconcile(lambda n:changed if n==folder else a.local(n))

    def test_original_plan_drift_refused(self):
        self.check_change(self.nr('plan.json'),lambda v:v.update(session_bound_seconds=1200))


if __name__=='__main__':unittest.main()
