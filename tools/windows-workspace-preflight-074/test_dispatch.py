import copy
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import accounting as a
import custody
import dispatch as d
import owned
import prepare as p
import worker
from test_owned import Group

CATALOG=['unittests src/lib.rs']
OUT='running 1 test\ntest alpha ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n'
ERR='    Finished `test` profile [unoptimized] target(s) in 0.01s\n     Running unittests src/lib.rs (target/debug/deps/lib-test.exe)\n'


def receipt(mode='package',code=0,kind='success'):
    return dict(classification=kind,exit=code,retired=True,errors=[],
        bound_seconds=780 if mode=='package' else 4800,cleanup_seconds=5)


class Accounting(unittest.TestCase):
    def test_strict_original_package_parser_complete_and_missing_targets(self):
        self.assertTrue(a.account('package',OUT,ERR,receipt(),CATALOG)['package_pass'])
        with self.assertRaises(ValueError):a.account('package',OUT,ERR,receipt(),CATALOG+['tests/missing.rs'])
        with self.assertRaises(ValueError):a.account('package',OUT.replace('1 passed','2 passed'),ERR,receipt(),CATALOG)
        with self.assertRaises(ValueError):a.account('package',OUT,ERR,dict(receipt(),retired=False),CATALOG)

    def test_original_timeout_preserves_incomplete_and_unstarted_not_success(self):
        value=a.account('package','running 2 tests\ntest alpha ... ok\n',ERR,
            receipt(code=124,kind='timeout'),CATALOG+['tests/later.rs'])
        self.assertFalse(value['complete'] or value['package_pass'])
        self.assertEqual(value['unstarted_targets'],['tests/later.rs'])
        self.assertEqual(value['incomplete_target']['declared'],2)

    def test_build_failure_and_resource_floor_keep_complete_partial_accounting(self):
        result=a.account('package','', 'error: compilation failed\n',receipt(code=101,kind='test-failure'),CATALOG)
        self.assertEqual(result['unstarted_targets'],CATALOG)
        self.assertFalse(result['build_finished'] or result['package_pass'])
        result=a.account('package',OUT,ERR,receipt(code=125,kind='resource-floor'),CATALOG+['tests/later.rs'])
        self.assertEqual(result['passed'],1)
        self.assertEqual(result['unstarted_targets'],['tests/later.rs'])
        self.assertFalse(result['package_pass'])

    def test_workspace_complete_stages_and_tests_are_separate_from_package(self):
        text='\n'.join('== '+s+' ==' for s in a.STAGES)+'\n'+OUT
        text+='-- console npm deps\n-- console static check\n-- console playwright\nverify: all gates passed\n'
        value=a.account('workspace',text,ERR,receipt('workspace'),CATALOG)
        self.assertTrue(value['workspace_pass'])
        self.assertEqual(len(value['stages']),21)
        self.assertTrue(all(row['state']=='complete' for row in value['stages']))
        for bad in (text.replace('== format ==','== unknown =='),text.replace('verify: all gates passed\n',''),
                    text.replace('-- console playwright\n',''),text+OUT):
            with self.assertRaises(ValueError):a.account('workspace',bad,ERR,receipt('workspace'),CATALOG)

    def test_workspace_failure_prefix_and_original_node_skip_not_reinterpreted(self):
        value=a.account('workspace','== doc-check ==\n', '',receipt('workspace',1,'test-failure'),[])
        self.assertFalse(value['workspace_pass'])
        self.assertEqual(value['stages'][0]['state'],'incomplete-or-failed')
        self.assertEqual(value['stages'][1]['state'],'unstarted')
        text='\n'.join('== '+s+' ==' for s in a.STAGES)+'\n'+OUT
        text+='management console: SKIP (node not found)\nverify: all gates passed\n'
        self.assertTrue(a.account('workspace',text,ERR,receipt('workspace'),[])['workspace_pass'])
        with self.assertRaises(ValueError):a.account('workspace',text.replace('node not found','disabled'),ERR,receipt('workspace'),[])

    def test_rustdoc_multigroup_counts_without_invented_target_attribution(self):
        text='\n'.join('== '+s+' ==' for s in a.STAGES)+'\n'+OUT*2
        text+='management console: SKIP (npm not found)\nverify: all gates passed\n'
        result=a.account('workspace',text,'   Doc-tests synthetic\n',receipt('workspace'),[])
        self.assertEqual(result['passed'],2)
        self.assertEqual(result['case_to_target_assignment'],'not-asserted-rustdoc-grouping')
        self.assertFalse(any('target' in row for row in result['targets']))
        self.assertTrue(result['workspace_pass'])

    def test_doctest_names_and_ignored_counts_remain_visible(self):
        text=OUT.replace('alpha','src/lib.rs - Example (line 3) - compile fail')
        self.assertEqual(a.rust_blocks(text)[0]['cases'][0]['name'],'src/lib.rs - Example (line 3) - compile fail')
        with self.assertRaises(ValueError):a.rust_blocks(OUT.replace('0 filtered','1 filtered'))


class Dispatcher(unittest.TestCase):
    def context(self,root,mode='package'):
        return dict(head='fixed',helpers={},targets=CATALOG,sources={},versions={},environment={})

    def invoke(self,root,*,run=False,factory=None,frames=None,mode='package'):
        frame=self.context(root)
        with patch.object(d,'controls'),patch.object(custody,'frame',side_effect=frames or [frame,frame]), \
             patch.object(d.shutil,'disk_usage',return_value=shutil._ntuple_diskusage(100*1024**3,0,100*1024**3)), \
             patch.object(d.subprocess,'Popen',side_effect=factory) as spawned:
            code=d.execute(root/'target/attempt',mode,run=run,root=root)
        return code,spawned

    def test_default_preflight_never_spawns_and_consumes_output(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve()
            code,spawned=self.invoke(root)
            self.assertEqual(code,0)
            spawned.assert_not_called()
            with self.assertRaises(FileExistsError):self.invoke(root)

    def factory(self,root,*,kind='success',drift=False,lost=False,mode='package'):
        class Process:
            pid=42
            returncode=None
            killed=False
            def __init__(self,argv,**kwargs):
                self.out=root/'target/attempt'
                request=json.loads((self.out/'request.json').read_bytes())
                assert argv[-1]==mode and argv[-2]==p.sha((self.out/'request.json').read_bytes())
                assert kwargs['env']['CARGO_INCREMENTAL']=='0'
                if mode=='package':assert kwargs['env']['CARGO_BUILD_JOBS']=='2'
                assert request['specification']['bound_seconds']==(780 if mode=='package' else 4800)
                text=OUT if mode=='package' else '\n'.join('== '+s+' ==' for s in a.STAGES)+'\n'+OUT+'management console: SKIP (node not found)\nverify: all gates passed\n'
                kwargs['stdout'].write(text.encode());kwargs['stderr'].write(ERR.encode())
                p.write(self.out,'controller.json',dict(mode=mode,request_sha256='0'*64 if drift else argv[-2],
                    specification=request['specification'],execution=receipt(mode),original_job_sha256=owned.JOB_SHA,worker_pid=42))
            def poll(self):return self.returncode
            def wait(self,timeout):
                if lost and not self.killed:raise subprocess.TimeoutExpired('original worker',timeout)
                self.returncode=125 if self.killed else 0
                return self.returncode
            def kill(self):self.killed=True
        return Process

    def test_fixed_real_command_is_connected_and_accounted_with_model_worker(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve()
            code,_=self.invoke(root,run=True,factory=self.factory(root))
            self.assertEqual(code,0)
            self.assertTrue(json.loads((root/'target/attempt/result.json').read_bytes())['package_pass'])
            self.assertEqual(json.loads((root/'target/attempt/accounting.json').read_bytes())['passed'],1)

    def test_workspace_fixed_command_and_complete_accounting_with_model_worker(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve()
            code,_=self.invoke(root,run=True,factory=self.factory(root,mode='workspace'),mode='workspace')
            self.assertEqual(code,0)
            result=json.loads((root/'target/attempt/result.json').read_bytes())
            self.assertTrue(result['workspace_pass'])
            self.assertFalse(result['package_pass'] or result['release_admitted'])

    def test_worker_identity_and_postflight_drift_refuse_not_pass(self):
        for kind in ('identity','frame','worker-loss'):
            with self.subTest(kind=kind),tempfile.TemporaryDirectory() as directory:
                root=Path(directory).resolve()
                before=self.context(root)
                after=dict(before,head='changed') if kind=='frame' else before
                with self.assertRaises(ValueError):
                    self.invoke(root,run=True,factory=self.factory(root,drift=kind=='identity',lost=kind=='worker-loss'),frames=[before,after])
                self.assertFalse(json.loads((root/'target/attempt/refusal.json').read_bytes())['gate_pass'])
                self.assertFalse(json.loads((root/'target/attempt/accounting.json').read_bytes())['complete'])

    def test_missing_red_stale_control_receipts_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):d.controls(Path(directory))

    def test_completed_or_failed_execution_claim_cannot_be_replayed(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory).resolve()
            self.invoke(root,run=True,factory=self.factory(root))
            # Remove neither a claim nor the first result: use another capture directory.
            before=self.context(root)
            with patch.object(d,'controls'),patch.object(custody,'frame',return_value=before), \
                    patch.object(d.shutil,'disk_usage',return_value=shutil._ntuple_diskusage(100*1024**3,0,100*1024**3)), \
                    patch.object(d.subprocess,'Popen') as spawned:
                with self.assertRaises(FileExistsError):d.execute(root/'target/second','package',run=True,root=root)
                spawned.assert_not_called()

    @unittest.skipUnless(os.name=='nt','Windows nonblocking file lock')
    def test_own_worker_lock_is_exclusive_and_released(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            with worker.gate_lock(root):
                with self.assertRaises(OSError):
                    with worker.gate_lock(root):self.fail('concurrent worker acquired original lock')
            with worker.gate_lock(root):pass

    def test_no_cli_limit_filter_or_arbitrary_command_options(self):
        for flag in ('--timeout','--retry','--test','--command'):
            result=subprocess.run([sys.executable,'-B',str(Path(d.__file__)),flag],capture_output=True,timeout=10)
            self.assertEqual(result.returncode,2)

    def test_native_worker_refuses_changed_request_before_helpers_or_cargo(self):
        folder=Path(tempfile.mkdtemp(prefix='worker-request-fault-',dir=p.ROOT/'target'))
        with (folder/'request.json').open('x') as stream:json.dump({'untrusted':'not executed'},stream)
        with (folder/'stdout.log').open('xb') as stdout,(folder/'stderr.log').open('xb') as stderr:
            result=subprocess.run([sys.executable,'-B',str(Path(worker.__file__)),str(folder),'0'*64,'package'],
                stdout=stdout,stderr=stderr,timeout=10,check=False)
        print('retained-dispatch-fixture '+str(folder),flush=True)
        self.assertNotEqual(result.returncode,0)
        self.assertIn('parent request bytes drift',json.loads((folder/'worker-refusal.json').read_bytes())['reason'])
        self.assertFalse((folder/'controller.json').exists())

    def test_absolute_deadline_cannot_extend_bound_and_timeout_is_preserved(self):
        group=Group(0)
        with self.assertRaises(ValueError):owned._owned(group,['synthetic'],None,lambda:None,bound=780,deadline=781,now=lambda:0)
        group=Group(0)
        result=owned._owned(group,['synthetic'],None,lambda:None,bound=780,deadline=-1,now=lambda:0)
        self.assertEqual(result['classification'],'timeout')
        self.assertTrue(result['retired'])


if __name__=='__main__':unittest.main()
