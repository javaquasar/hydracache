"""Offline controller/plan/fence propagation checks; no host or clock effects."""
import ast
import hashlib
import importlib.util
import inspect
from pathlib import Path
import types
import unittest
from unittest.mock import Mock,patch
import coordinator as c
import plan
import worker

ROOT=Path(__file__).resolve().parents[2]


class Controller(unittest.TestCase):
    def make(self):
        root=Path('/tmp/hc074-chitchat-cost-20261029.abcdef')
        state=dict(supervisor='ActiveState=active\nSubState=running\nMainPID=377415\nNRestarts=0\nInvocationID=x',boot='boot',protected={})
        return root,plan.make(root,state,{str(root/'input/protocol.json'):'f'*64},Path('/original/fence'),[1,2])

    def test_exact_five_counterbalanced_blocks_per_mode(self):
        root,p=self.make();self.assertEqual(len(p['attempts']),60)
        self.assertEqual([(v['mode'],v['variant']) for v in p['attempts']],[(m,v) for m in ('cpu','memory') for v in ('B','P','Q','Q','P','B')*5])
        for i,attempt in enumerate(p['attempts']):
            self.assertEqual(attempt['argv'][-3:],[str(i),'@PLAN_SHA@','@FENCE_FD@'])
        self.assertEqual(p['supervisor'],'hydracache-performance-supervisor-074.service');self.assertTrue(p['operator_approved'])

    def test_unit_has_autonomous_post_and_finite_kill_scope(self):
        root,p=self.make();argv=plan.launch(root,'a'*64)
        for value in ('--property=Delegate=yes','--property=DelegateSubgroup=control','--property=RuntimeMaxSec=1200',
            '--property=KillMode=control-group','--property=KillSignal=SIGKILL','--property=Restart=no'):
            self.assertIn(value,argv)
        self.assertTrue(any(v.startswith('--property=ExecStopPost=') and v.endswith('a'*64) for v in argv))
        self.assertEqual(argv[-1],'a'*64)

    def test_original_base_source_is_unchanged(self):
        raw=(ROOT/'tools/get-owner-scheduled-controls-074/maintenance_d3.py').read_bytes()
        self.assertEqual(hashlib.sha256(raw).hexdigest(),c.BASE_SHA)

    def test_spawn_only_adds_original_fence_inheritance(self):
        raw=(ROOT/'scripts/ci/watchdog_linux_group.py').read_text();tree=ast.parse(raw)
        cls=next(n for n in tree.body if isinstance(n,ast.ClassDef) and n.name=='LinuxGroup')
        original=next(n for n in cls.body if isinstance(n,ast.FunctionDef) and n.name=='spawn')
        changed=ast.parse(inspect.getsource(worker.fenced_spawn)).body[0];changed.name='spawn'
        calls=[n for n in ast.walk(changed) if isinstance(n,ast.Call) and isinstance(n.func,ast.Attribute) and n.func.attr=='Popen']
        self.assertEqual(len(calls),1);keywords=calls[0].keywords
        fd=next(k for k in keywords if k.arg=='pass_fds');self.assertEqual(ast.dump(fd.value),ast.dump(ast.parse('(self.fence,)',mode='eval').body))
        calls[0].keywords=[k for k in keywords if k.arg!='pass_fds']
        self.assertEqual(ast.dump(changed,include_attributes=False),ast.dump(original,include_attributes=False))

    def test_closed_or_second_spawn_is_refused(self):
        with patch.object(worker,'GroupFailure',RuntimeError,create=True),patch.object(worker.subprocess,'Popen') as popen:
            for closed,attempted in ((True,False),(False,True)):
                with self.assertRaises(RuntimeError):worker.fenced_spawn(types.SimpleNamespace(closed=closed,attempted=attempted),[],None,None,0)
            popen.assert_not_called()

    def test_actual_spawn_keeps_fence_copy(self):
        state=types.SimpleNamespace(closed=False,attempted=False,fence=42)
        with patch.object(worker,'LinuxChild',lambda owner,p:types.SimpleNamespace(owner=owner,process=p),create=True),patch.object(worker.subprocess,'Popen') as popen:
            child=worker.fenced_spawn(state,['image','cpu'],'/root',None,17)
            self.assertEqual(popen.call_args.kwargs['pass_fds'],(42,));self.assertIs(child.owner,state);self.assertTrue(state.attempted)

    def test_absolute_timer_carries_same_plan_pin(self):
        m=types.SimpleNamespace(timer_name=lambda p,s:'owned-deadline',command=Mock())
        c.timer_install({'_pin':'f'*64},2,Path('/tmp/owned'),1234567890,m)
        argv=m.command.call_args.args[0]
        self.assertIn('--on-boot=1.234567890s',argv);self.assertNotIn('--on-active=',str(argv));self.assertEqual(argv[-2:],['2','f'*64])

    def test_stop_precedes_fence_and_retirement_precedes_validation(self):
        raw=inspect.getsource(c.main)
        self.assertLess(raw.index("'stop'"),raw.index('m.open_fence'))
        self.assertLess(raw.index('m.empty_group'),raw.index('validate_receipt'))
        self.assertEqual(raw.count('time.monotonic_ns()+1100*10**9'),1)
        self.assertIn("else deadline",raw);self.assertIn("need(time.monotonic_ns()<at",raw)

    def test_post_uses_exact_original_body_and_restores_binding(self):
        old=Mock();post=Mock();m=types.SimpleNamespace(static_frame=old,post=post,publish=Mock())
        with patch.object(c,'load',return_value=types.SimpleNamespace(snapshot=lambda:{})):
            c.post({},Path('/tmp/private'),m)
        post.assert_called_once_with({},Path('/tmp/private'));self.assertIs(m.static_frame,old)

    def test_dispatch_requires_current_payload_and_one_shot_latches(self):
        text=(Path(__file__).parent/'client.py').read_text()
        self.assertIn("sha(OUT/'payload'/name)==sha(HERE/name)",text)
        self.assertLess(text.index("'numerical-launch-once.json'"),text.index("d.call('remote-'+mode"))
        self.assertIn("'stage-once.json'",text)

    def test_kernel_cpuset_not_ssh_process_affinity_owns_admission(self):
        with patch.object(Path,'read_text',side_effect=['0-7','0-7']),patch.object(c.os,'sched_getaffinity',side_effect=AssertionError('SSH process affinity is not availability'),create=True):
            self.assertEqual(c.cpu_profile(),dict(online='0-7',system_slice_effective='0-7'))
        for raw in ('0,5-7','0-3'):
            with patch.object(Path,'read_text',side_effect=['0-7',raw]):
                with self.assertRaises(RuntimeError):c.cpu_profile()

    def test_kernel_cpu_list_parser_refusals(self):
        self.assertEqual(c.parse_cpus('0,1-4,7'),{0,1,2,3,4,7})
        for raw in ('','7-0','0-x','-1','1-2-3'):
            with self.assertRaises(RuntimeError):c.parse_cpus(raw)


if __name__=='__main__':unittest.main()
