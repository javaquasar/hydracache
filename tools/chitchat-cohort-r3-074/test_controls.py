"""Baseline, exact predicate preservation and terminal recovery refusal controls."""
import ast
import hashlib
import importlib.util
import inspect
import json
import os
from pathlib import Path
import tempfile
import types
import unittest
import sys
from unittest.mock import Mock,patch
import coordinator as c
import namespace as ns
import plan
import recovery as r

ROOT=Path(__file__).resolve().parents[2]

def load(path,name):
    s=importlib.util.spec_from_file_location(name,path);m=importlib.util.module_from_spec(s)
    if sys.platform=='win32':
        with patch.dict(sys.modules,{'fcntl':Mock()}):s.loader.exec_module(m)
    else:s.loader.exec_module(m)
    return m

class Controls(unittest.TestCase):
    def test_numerical_main_and_validator_stay_exact(self):
        old=ast.parse((ROOT/'tools/chitchat-cohort-074/coordinator.py').read_bytes());new=ast.parse(Path(c.__file__).read_bytes())
        for name in ('main','validate_receipt','timer_install','read_plan','cpu_profile'):
            a=next(x for x in old.body if isinstance(x,ast.FunctionDef) and x.name==name);b=next(x for x in new.body if isinstance(x,ast.FunctionDef) and x.name==name)
            self.assertEqual(ast.dump(a,include_attributes=False),ast.dump(b,include_attributes=False))

    def test_every_original_frame_predicate_order_and_count_stays(self):
        old=ast.parse((ROOT/'tools/chitchat-cohort-074/coordinator.py').read_bytes());new=ast.parse(Path(c.__file__).read_bytes())
        a=next(x for x in old.body if isinstance(x,ast.FunctionDef) and x.name=='frame');b=next(x for x in new.body if isinstance(x,ast.FunctionDef) and x.name=='frame')
        self.assertEqual(len(b.body),len(a.body)+2);b.body=b.body[2:]
        self.assertEqual(ast.dump(a,include_attributes=False),ast.dump(b,include_attributes=False))

    def test_worker_is_byte_identical(self):
        self.assertEqual((ROOT/'tools/chitchat-cohort-074/worker.py').read_bytes(),Path(__file__).with_name('worker.py').read_bytes())

    def test_full_five_block_plan_and_frozen_bounds(self):
        root=Path('/tmp/hc074-chitchat-cost-20261029.test')
        p=plan.make(root,dict(supervisor='MainPID=1\nActiveState=active',boot='boot',protected={}),{str(root/'input/protocol.json'):'p'},root/'fence',[1,2])
        self.assertEqual([(x['mode'],x['variant']) for x in p['attempts']],[(m,v) for m in ('cpu','memory') for v in ('B','P','Q','Q','P','B')*5])
        self.assertEqual([p[k] for k in ('outer_bound_seconds','session_bound_seconds','capacity_poll_seconds','cleanup_seconds')],[1200,1100,5,5])
        self.assertIn('--property=RuntimeMaxSec=1200',plan.launch(root,'f'*64))

    def exercise(self,*,denied=False,populated=False,busy=False,protected=False):
        tmp=ROOT/'target/chitchat-cohort-r3-20261030/tmp';tmp.mkdir(parents=True,exist_ok=True)
        with tempfile.TemporaryDirectory(prefix='recovery-',dir=tmp) as folder:
            root=Path(folder).resolve();self.assertTrue(root.is_relative_to(tmp.resolve()));self.assertFalse(root.is_symlink())
            g=root/'cg';(g/'control').mkdir(parents=True);(g/'writers-0').mkdir()
            for group in (g/'control',g/'writers-0'):(group/'cgroup.events').write_text('populated '+('1' if populated and group.name=='writers-0' else '0')+'\nfrozen 0\n')
            m=load(ROOT/'tools/get-owner-scheduled-controls-074/maintenance_d3.py','hc074_original_post_control')
            intent=dict(group=str(g/'writers-0'),group_identity=m.group_identity(g/'writers-0'))
            (root/'0.intent.json').write_text(json.dumps(intent));(root/'inactive.json').write_text('{}');(root/'fence').touch()
            expected=dict(mounts='unchanged',supervisor='live',boot='same',protected={'image':'same'})
            p=dict(unit='owned.service',supervisor='owned-standin.service',synthetic=True,expected_host=expected,external_host=expected,cpu_profile={'x':1})
            states=[dict(ActiveState='inactive',MainPID='0'),dict(ActiveState='inactive',MainPID='0'),dict(ActiveState='active',SubState='running',MainPID='9')]
            records={};actual=dict(expected,boot='changed') if protected else expected
            namespace=types.SimpleNamespace(host=lambda root:actual,current=Mock(),load=lambda *args:types.SimpleNamespace(cpu_profile=lambda:{'x':1}))
            frame=Mock(side_effect=RuntimeError('numeric frame denied') if denied else None)
            fake_open=Mock(side_effect=RuntimeError('original fence busy') if busy else lambda p:os.open(root/'fence',os.O_RDONLY))
            with patch.object(m,'own_group',return_value=g),patch.object(m,'static_frame'),patch.object(m,'timer_stop'),patch.object(m,'service_state',side_effect=states),patch.object(m,'command') as command,patch.object(m,'open_fence',fake_open),patch.object(m,'publish',side_effect=lambda root,name,value:records.update({name:value})),patch.object(r.shutil,'disk_usage',return_value=types.SimpleNamespace(free=8*1024**3)):
                if populated or busy or protected:
                    with self.assertRaises(RuntimeError):r.post(p,root,m,namespace,frame,c.types_base)
                    self.assertNotIn('restored.json',records);command.assert_not_called();return
                r.post(p,root,m,namespace,frame,c.types_base)
                self.assertTrue(records['restored.json']['verified']);self.assertEqual(frame.call_count,2)
                self.assertEqual(records['postflight-result.json']['full_measurement_frames_pass'],not denied)
                self.assertTrue(records['all-writers-stopped.json']['original_fence_acquired'])

    def test_healthy_post_restores_and_checks_both_frames(self):self.exercise()
    def test_denied_numeric_frame_does_not_block_proved_restoration(self):self.exercise(denied=True)
    def test_populated_original_writer_prevents_restoration(self):self.exercise(denied=True,populated=True)
    def test_busy_original_fence_prevents_restoration(self):self.exercise(denied=True,busy=True)
    def test_protected_host_drift_prevents_restoration(self):self.exercise(denied=True,protected=True)

    def test_predecessor_reproduces_frame_denial_before_restore(self):
        old=load(ROOT/'tools/chitchat-cohort-074/coordinator.py','hc074_first_post_baseline');m=types.SimpleNamespace(static_frame=Mock())
        m.post=lambda p,root:m.static_frame(p)
        with patch.object(old,'frame',side_effect=RuntimeError('host frame changed')):
            with self.assertRaisesRegex(RuntimeError,'host frame changed'):old.post({},Path('/tmp/never_effect'),m)

    def test_fixture_only_readonly_remount_and_user_leaf_recovery(self):
        old='1 2 0:1 / /tmp/owned rw,relatime - ext4 disk rw';new=old.replace('rw,relatime','ro,relatime')
        r.mount_delta(old,new,'/tmp/owned',True)
        with self.assertRaises(RuntimeError):r.mount_delta(old,new.replace('disk','other'),'/tmp/owned',True)
        a='3 2 0:3 / /run/user/1000 rw shared:4 - tmpfs tmpfs rw,uid=1000';b=a.replace('3 2 0:3','5 2 0:5').replace('shared:4','shared:6')
        r.mount_delta(a,b,'/run/user/1000')
        with self.assertRaises(RuntimeError):r.mount_delta(a,b.replace('uid=1000','uid=0'),'/run/user/1000')

    def test_recorded_leaf_disappearance_never_admits_measurement(self):
        a='3 2 0:3 / /run/user/1000 rw - tmpfs tmpfs rw,uid=1000'
        self.assertFalse(r.mount_delta(a,'','/run/user/1000')['measurement_frame_pass'])
        for before,after,target in ((a,'','/run/user/1001'),(a.replace('tmpfs tmpfs','ext4 disk'),'','/run/user/1000'),(a,a+'\n4 2 0:4 / /other rw - tmpfs tmpfs rw','/run/user/1000')):
            with self.assertRaises(RuntimeError):r.mount_delta(before,after,target)

    def test_user_root_relocation_is_private_and_retains_fd(self):
        text=inspect.getsource(ns.keeper);self.assertLess(text.index('own private namespace'),text.index('user_tree'))
        text=inspect.getsource(ns.user_tree);self.assertLess(text.index('os.open(child'),text.index("'/usr/bin/umount'"))
        self.assertIn("'--lazy'",text);self.assertIn('pass_fds=(fd,)',text);self.assertIn('same held user filesystem root',text)
        self.assertIn("'--no-canonicalize'",text);self.assertIn("'--internal-only'",text)

    def test_namespace_join_opens_and_checks_original_fd(self):
        meta=dict(pid=123,start_ticks='7',identity=[1,2],deadline_ns=100)
        with patch.object(ns,'live') as live,patch.object(ns.os,'O_CLOEXEC',0,create=True),patch.object(ns.os,'open',return_value=42),patch.object(ns.os,'fstat',return_value=types.SimpleNamespace(st_dev=1,st_ino=2)),patch.object(ns.os,'setns',create=True) as setns,patch.object(ns.os,'close') as close,patch.object(ns,'verify'):
            ns.join(Path('/tmp/owned'),dict(namespace=meta));setns.assert_called_once_with(42,0);close.assert_called_once_with(42);self.assertEqual(live.call_count,2)

    def test_mismatched_namespace_fd_is_never_entered(self):
        with patch.object(ns,'live'),patch.object(ns.os,'O_CLOEXEC',0,create=True),patch.object(ns.os,'open',return_value=42),patch.object(ns.os,'fstat',return_value=types.SimpleNamespace(st_dev=1,st_ino=3)),patch.object(ns.os,'setns',create=True) as setns,patch.object(ns.os,'close'):
            with self.assertRaises(RuntimeError):ns.join(Path('/tmp/owned'),dict(namespace=dict(pid=123,identity=[1,2])))
            setns.assert_not_called()

if __name__=='__main__':unittest.main()
