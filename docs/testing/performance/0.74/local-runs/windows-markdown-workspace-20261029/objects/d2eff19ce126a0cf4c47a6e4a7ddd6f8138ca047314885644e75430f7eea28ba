import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import getunit_once as g

class GetUnitControls(unittest.TestCase):
    def test_empty_success_is_not_admission(self):
        self.assertEqual(g.outcome(0,'')['returncode'],74)
    def test_first_refusal_is_never_promoted(self):
        self.assertEqual(g.outcome(101,'running 1 test')['returncode'],101)
    def test_exact_positive_requires_send_and_readiness(self):
        live='running 1 test\noriginal_send=true manager_ack=true\nreadiness=true release=false completion=false'
        self.assertEqual(g.outcome(0,live)['returncode'],0)
        self.assertEqual(g.outcome(0,live+'\nrunning 1 test')['returncode'],74)
    def test_dirty_candidate_refuses_before_lookup_or_mutation(self):
        with patch.object(g,'git',return_value='dirty'):
            with self.assertRaises(ValueError): g.verify_candidate()
    def test_selected_binary_requires_build_receipt(self):
        with tempfile.TemporaryDirectory() as tmp, patch.object(g,'CHECKS',Path(tmp)):
            (Path(tmp)/'native-libtest.json').write_text(json.dumps({'exit':101}))
            with self.assertRaises(ValueError): g.selected_binary()
    def test_predecessor_must_be_original_preflight_refusal(self):
        with tempfile.TemporaryDirectory() as tmp:
            folder=Path(tmp)
            (folder/'result.json').write_text(json.dumps({'returncode':0,'original_send_observed':False,'readiness_observed':False}))
            with self.assertRaises(ValueError): g.prior_refusal(folder)
    def test_runtime_is_not_edited_or_rebuilt_by_dispatch(self):
        source=Path(g.__file__).read_text()
        self.assertEqual(source.count('child=subprocess.run(command'),1)
        self.assertIn('exist_ok=False',source)
        self.assertNotIn('ssh',source)
        self.assertNotIn('cargo',source)

if __name__=='__main__': unittest.main()
