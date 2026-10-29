"""Mutate copied JSON receipts only; never read live kernel or launch a child."""
import json
import unittest
import analyze as a


class Accounting(unittest.TestCase):
    def refuse(self,name,change):
        def get(path):
            raw=a.local(path)
            if path==name:
                value=json.loads(raw);change(value);return json.dumps(value).encode()
            return raw
        with self.assertRaises((ValueError,KeyError)):a.reconcile(get)

    def test_complete_native_control_only(self):
        r=a.reconcile();self.assertTrue(r['host_control_gate_pass']);self.assertEqual(len(r['controls']),3)
        self.assertFalse(r['CPU_qualified']);self.assertFalse(r['cohort_dispatched'])

    def test_first_refusal_cannot_be_promoted(self):self.refuse(a.R0+'/collection/result.json',lambda r:r.update(complete=True))
    def test_retirement_missing(self):self.refuse(a.R1+'/collection/control-Q/execution.json',lambda r:r.update(retired=False))
    def test_host_or_supervisor_change(self):self.refuse(a.R1+'/collection/postflight.json',lambda r:r.update(boot='changed'))
    def test_wrong_image(self):self.refuse(a.R1+'/collection/control-P/intent.json',lambda r:r.update(image_sha256='0'*64))
    def test_deadline_rebase(self):self.refuse(a.R1+'/collection/control-Q/intent.json',lambda r:r.update(deadline=r['deadline']+1))
    def test_no_scope_promotion(self):self.refuse(a.R1+'/collection/result.json',lambda r:r.update(CPU_qualified=True))
    def test_missing_semantic_row(self):self.refuse(a.R1+'/collection/control-B/raw-body.json',lambda r:r['rows'].pop())
    def test_parent_graph_mutation(self):self.refuse(a.R1+'/patch/input/inputs.json',lambda r:r['files'].update({'images/Q':'0'*64}))


if __name__=='__main__':unittest.main()
