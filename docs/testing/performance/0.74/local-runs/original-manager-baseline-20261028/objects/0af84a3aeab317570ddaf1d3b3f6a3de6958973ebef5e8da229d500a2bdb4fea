from pathlib import Path
import importlib.util,unittest
ROOT=Path(__file__).resolve().parents[2]
s=importlib.util.spec_from_file_location('original_live_controls',Path(__file__).parent/'run.py');r=importlib.util.module_from_spec(s);s.loader.exec_module(r)
class Controls(unittest.TestCase):
 def test_only_shell_whitelist_differs(self):
  raw=(ROOT/'tools/bootstrap-original-service-074/native_fixture.sh').read_bytes();self.assertEqual(r.sha(raw),'55383db02fce60bf3093c45b4b21c7ab6725eb089a8720ad34b95d78016efec6')
  old=b'/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/bootstrap-original-service-20261023/native-account-compatible-once|/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/bootstrap-original-getunit-20261023/native-once'
  self.assertEqual((Path(__file__).parent/'baseline_fixture.sh').read_bytes(),raw.replace(old,b'/mnt/c/Workspace/prj/jq/cashe/hydracache-074/target/original-manager-live-20261028/baseline-once',1))
 def test_empty_or_cleanup_only_success_is_refused(self):
  for text in ('','running 1 test','running 1 test\neof=true fence_retained=true custodians_terminal=true'):self.assertEqual(r.outcome(0,text)['exit'],74)
 def test_prior_failure_is_retained_and_never_promoted(self):self.assertEqual(r.outcome(101,'running 1 test')['exit'],101)
 def test_positive_is_readiness_only_and_exact_selection(self):
  text='running 1 test\noriginal_send=true manager_ack=true\nreadiness=true release=false completion=false\neof=true fence_retained=true custodians_terminal=true'
  value=r.outcome(0,text);self.assertEqual(value['exit'],0);self.assertFalse(value['original_release']);self.assertFalse(value['reconnect_tested']);self.assertTrue(value['cleanup_is_not_autonomous_retirement']);self.assertEqual(r.outcome(0,text+'\nrunning 1 test')['exit'],74)
 def test_frozen_candidate_and_root_stay_exact(self):r.guard()
if __name__=='__main__':unittest.main()
