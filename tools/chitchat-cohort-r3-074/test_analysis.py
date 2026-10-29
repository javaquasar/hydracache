"""Saved-data mutations; never repeat a live control or numerical sample."""
import json
import unittest
import analyze as a

class Accounting(unittest.TestCase):
    def nr(self,name):return 'collection/'+a.NUMERIC+'/'+name
    def change(self,name,mutate):
        value=json.loads(a.local(name));mutate(value);raw=json.dumps(value).encode()
        with self.assertRaises((ValueError,RuntimeError,KeyError,AssertionError)):
            a.reconcile(lambda n:raw if n==name else a.local(n))

    def test_complete_matrix_retains_negative_cpu_gate(self):
        v=a.reconcile();self.assertTrue(v['complete']);self.assertTrue(v['automatic_restoration_verified'])
        self.assertFalse(v['cpu_nonregression_gate_pass']);self.assertTrue(v['memory_growth_screen_pass'])
        self.assertEqual(len(v['timing']),62);self.assertEqual(sum(x['upper95']>1.03 for x in v['timing']),11)
        self.assertFalse(v['root_adopted']);self.assertFalse(v['release_admitted']);self.assertFalse(v['retry'])

    def test_missing_sixtieth_result_cannot_be_filtered(self):
        name=self.nr('59.body.json')
        def get(n):
            if n==name:raise FileNotFoundError(n)
            return a.local(n)
        with self.assertRaises(FileNotFoundError):a.reconcile(get)

    def test_body_drift_refused(self):self.change(self.nr('0.body.json'),lambda x:x.update(seed=0))
    def test_held_process_identity_drift_refused(self):self.change(self.nr('0.worker-child.json'),lambda x:x.update(pid=x['pid']+1))
    def test_deadline_rebase_refused(self):self.change(self.nr('10.intent.json'),lambda x:x.update(deadline_ns=x['deadline_ns']+1))
    def test_populated_group_refused(self):self.change(self.nr('59.retired.json'),lambda x:x['events'].update(populated='1'))
    def test_lost_fence_proof_refused(self):self.change(self.nr('all-writers-stopped.json'),lambda x:x.update(original_fence_acquired=False))
    def test_terminal_frame_refusal_cannot_be_promoted(self):self.change(self.nr('postflight-result.json'),lambda x:x.update(full_measurement_frames_pass=False))
    def test_public_source_drift_refused(self):self.change(self.nr('host-postflight.json'),lambda x:x.update(boot='other'))
    def test_plan_drift_refused(self):self.change(self.nr('plan.json'),lambda x:x.update(session_bound_seconds=1200))

if __name__=='__main__':unittest.main()
