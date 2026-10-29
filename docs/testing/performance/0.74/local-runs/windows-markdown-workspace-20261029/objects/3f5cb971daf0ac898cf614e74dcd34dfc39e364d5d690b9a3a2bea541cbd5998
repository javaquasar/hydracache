"""Model/source controls only: never launch a process or read a native clock."""
import ast
import copy
import inspect
import json
from pathlib import Path
import runpy
import tempfile
import unittest
from unittest.mock import Mock, patch

import capture_leaves as subject
import coordinator_frames as outer
import git_cpu_overhead as caller
import git_job_cpu as original
import test_git_cpu_overhead as fixture
from test_coordinator_frames import Clock


class Strip(ast.NodeTransformer):
    def visit_Expr(self, node):
        call = node.value
        if isinstance(call, ast.Call) and isinstance(call.func, ast.Attribute):
            receiver = call.func.value
            leaf = isinstance(receiver, ast.Name) and receiver.id == 'leaves'
            leaf |= isinstance(receiver, ast.Attribute) and isinstance(receiver.value, ast.Name) and receiver.value.id == 'self' and receiver.attr == 'leaves'
            if leaf:
                assert call.func.attr in ('begin', 'end') and len(call.args) == 2 and not call.keywords
                assert all(isinstance(a, ast.Constant) for a in call.args)
                assert call.args[1].value in subject.GROUPS[call.args[0].value]
                return None
        return self.generic_visit(node)

    def visit_Call(self, node):
        if isinstance(node.func, ast.Name) and node.func.id in ('LeafAccountingJob', 'leaf_rows', 'attach_leaves'):
            assert not node.keywords and isinstance(node.args[-1], ast.Name) and node.args[-1].id == 'leaves'
            name = node.func.id
            assert len(node.args) == {'LeafAccountingJob': 4, 'leaf_rows': 3, 'attach_leaves': 2}[name]
            node.args.pop()
            if name == 'attach_leaves':
                return self.visit(node.args[0])
            node.func.id = {'LeafAccountingJob': 'AccountingJob', 'leaf_rows': 'rows'}[name]
        return self.generic_visit(node)


class Model(unittest.TestCase):
    def leaves(self, clock):
        return subject.Leaves(wall=lambda: clock.read('wall'), cpu=lambda: clock.read('cpu'), clock_domain='synthetic-control')

    def test_restricted_ast_preserves_frame_rows_and_retained_owner_setter(self):
        for before, after in ((outer.measure, subject.measure), (original.rows, subject.leaf_rows)):
            old, new = (ast.parse(inspect.getsource(f)).body[0] for f in (before, after))
            self.assertEqual(new.args.args.pop().arg, 'leaves')
            new.name = old.name
            self.assertEqual(ast.dump(old), ast.dump(Strip().visit(new)))
        import textwrap
        old, new = (ast.parse(textwrap.dedent(inspect.getsource(f))).body[0] for f in
                    (original.AccountingJob.child.fset, subject.LeafAccountingJob.child.fset))
        new.decorator_list = old.decorator_list
        self.assertEqual(ast.dump(old), ast.dump(Strip().visit(new)))

    def test_complete_budget_domains_zero_values_and_export_copy(self):
        clock = Clock()
        leaves = self.leaves(clock)
        for group, phases in subject.GROUPS.items():
            for phase in phases:
                leaves.begin(group, phase)
                leaves.end(group, phase)
        result = leaves.finish(True)
        self.assertTrue(result['complete'])
        self.assertTrue(result['capture_valid'])
        self.assertEqual(len(clock.calls), 48)
        self.assertEqual([len(v['probes']) for v in result['groups'].values()], [4, 12])
        before = copy.deepcopy(result)
        leaves.begin('owner', 'owner_identity')
        self.assertFalse(leaves.finish(True)['capture_valid'])
        self.assertEqual(before, result)
        zero = subject.Leaves(wall=lambda: 0, cpu=lambda: 0, clock_domain='synthetic-control')
        for group, phases in subject.GROUPS.items():
            for phase in phases:
                zero.begin(group, phase)
                zero.end(group, phase)
        self.assertTrue(zero.finish(True)['complete'])
        for kwargs in (dict(wall=lambda: 0), dict(clock_domain='unknown')):
            with self.assertRaises(original.Refusal):
                subject.Leaves(**kwargs)

    def scenario(self, folder, *, leaf=False, clock=None, fault=None, trace_clock=None, late=False):
        api, observer, job, image = fixture.Model().frame(folder, caller.PROFILE)
        events = []
        leaves = self.leaves(clock or Clock())
        spawn_original = job.spawn.side_effect
        def spawn(*args):
            events.append('spawn')
            if leaf:
                for name in subject.GROUPS['owner']:
                    leaves.begin('owner', name)
                    leaves.end('owner', name)
            return spawn_original(*args)
        job.spawn.side_effect = spawn
        counters = api.counters.side_effect
        api.counters.side_effect = lambda *args: (events.append('counters'), next(counters))[1]
        observer.dll.GetProcessId.side_effect = lambda *args: (events.append('pid'), 43 if fault else 42)[1]
        worker = observer.read.return_value
        observer.read.side_effect = lambda *args: (events.append('worker-read'), worker)[1]
        job.child.wait.side_effect = lambda **kwargs: (events.append('wait'), 0)[1]
        job.child.close.side_effect = lambda: events.append('child-close')
        job.close.side_effect = lambda: events.append('job-close')
        module, factory = (subject, 'LeafAccountingJob') if leaf else (outer, 'AccountingJob')
        with patch.object(module, factory, return_value=job), \
             patch.object(module.time, 'monotonic', side_effect=[1, 1, 1, 6] if late else None, return_value=1) as deadlines, \
             patch.object(module.time, 'monotonic_ns', side_effect=[100, 300]), \
             patch.object(module.time, 'process_time_ns', side_effect=[10, 20]), \
             patch.object(module, 'cleanup', return_value=['cleanup-second'] if fault else []) as cleanup:
            args = (api, observer, image, 0, 0, caller.PROFILE, folder, 5, (trace_clock or Clock()).tape())
            result = module.measure(*args, leaves) if leaf else module.measure(*args)
        return result, events, deadlines.call_count, cleanup, job

    def test_differential_frame_keeps_all_original_fields_calls_and_deadline_reads(self):
        with tempfile.TemporaryDirectory() as directory:
            a, b = (Path(directory)/name for name in ('old', 'leaf'))
            a.mkdir()
            b.mkdir()
            before, after = self.scenario(a), self.scenario(b, leaf=True)
            result = dict(after[0])
            self.assertTrue(result.pop('capture_leaves')['complete'])
            result['worker_image'] = before[0]['worker_image']
            self.assertEqual(result, before[0])
            self.assertEqual(after[1:3], before[1:3])
            self.assertEqual(after[2], 4)
            after[3].assert_not_called()

    def test_all_48_ordinary_clock_failures_freeze_both_groups_and_drain_owner(self):
        for failure in range(1, 49):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                clock = Clock(failure)
                result, events, deadlines, cleanup, job = self.scenario(Path(directory), leaf=True, clock=clock)
                self.assertFalse(result['passed'])
                self.assertTrue(result['operation_verdict']['passed'])
                self.assertIn('first clock fault', result['first_refusal'])
                self.assertIn('first clock fault', result['capture_leaves']['first_refusal'])
                self.assertEqual(len(clock.calls), failure)
                self.assertEqual(deadlines, 4)
                self.assertEqual(events, ['counters', 'spawn', 'wait', 'pid', 'worker-read', 'counters', 'counters',
                                          'child-close', 'job-close'])
                job.terminate.assert_not_called()
                cleanup.assert_not_called()

    def test_original_refusal_and_cleanup_survive_concurrent_leaf_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            result, _, _, cleanup, job = self.scenario(Path(directory), leaf=True, clock=Clock(1), fault=True)
            self.assertEqual(result['first_refusal'], result['operation_verdict']['first_refusal'])
            self.assertNotIn('clock fault', result['first_refusal'])
            self.assertIn('clock fault', result['capture_leaves']['first_refusal'])
            self.assertEqual(result['cleanup_errors'], ['cleanup-second'])
            cleanup.assert_called_once_with(job, job.child, 6)

    def test_all_48_fatal_clock_cuts_keep_original_owned_cleanup(self):
        class FatalClock(Clock):
            def read(self, kind):
                self.calls.append(kind)
                if len(self.calls) == self.fail:
                    raise KeyboardInterrupt('fatal leaf clock cut')
                return len(self.calls)*10
        for failure in range(1, 49):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as directory:
                result, events, _, cleanup, job = self.scenario(Path(directory), leaf=True, clock=FatalClock(failure))
                self.assertFalse(result['passed'])
                self.assertIn('KeyboardInterrupt', result['first_refusal'])
                cleanup.assert_called_once_with(job, None if failure <= 12 else job.child, 6)
                self.assertIn('spawn', events)

    def test_first_outer_refusal_is_not_replaced_by_later_leaf_refusal(self):
        with tempfile.TemporaryDirectory() as directory:
            result, _, _, cleanup, _ = self.scenario(Path(directory), leaf=True, clock=Clock(1), trace_clock=Clock(1))
            self.assertTrue(result['operation_verdict']['passed'])
            self.assertIn('coordinator telemetry', result['first_refusal'])
            self.assertIn('clock fault', result['capture_leaves']['first_refusal'])
            cleanup.assert_not_called()

    def test_original_final_deadline_still_refuses_after_healthy_close(self):
        with tempfile.TemporaryDirectory() as directory:
            result, events, checks, cleanup, _ = self.scenario(Path(directory), leaf=True, late=True)
            self.assertFalse(result['passed'])
            self.assertIn('late worker frame witness', result['first_refusal'])
            self.assertTrue(result['capture_leaves']['complete'])
            self.assertEqual(checks, 4)
            self.assertEqual(events[-2:], ['child-close', 'job-close'])
            cleanup.assert_not_called()

    def test_real_setter_retains_child_before_first_clock_even_if_fatal(self):
        class FatalClock(Clock):
            def read(self, kind):
                raise KeyboardInterrupt('before original identity observation')
        job = object.__new__(subject.LeafAccountingJob)
        job.leaves = self.leaves(FatalClock())
        job.observer = Mock()
        child = Mock()
        with self.assertRaises(KeyboardInterrupt):
            job.child = child
        self.assertIs(job.child, child)
        job.observer.identity.assert_not_called()
        with self.assertRaisesRegex(original.Refusal, 'cannot be replaced'):
            job.child = Mock()

    def test_original_owner_validation_and_first_error_still_apply(self):
        for wrong in (False, True):
            with tempfile.TemporaryDirectory() as directory:
                api, observer, _, image = fixture.Model().frame(Path(directory), caller.REFERENCE)
                job = object.__new__(subject.LeafAccountingJob)
                job.leaves = self.leaves(Clock(1))
                job.observer, job.image = observer, image.resolve()
                child = Mock(handle=9, pid=42)
                observer.identity.return_value = (43 if wrong else 42, str(image))
                first = dict(creation_100ns=1, exit_100ns=None, terminal=False, user_100ns=0, kernel_100ns=0)
                first.update(io={}, memory={})
                observer.read.return_value = first
                with patch.object(subject, 'validate') as validate:
                    if wrong:
                        with self.assertRaisesRegex(original.Refusal, 'identity drift'):
                            job.child = child
                        observer.read.assert_not_called()
                    else:
                        job.child = child
                        validate.assert_called_once_with(None, first)
                        self.assertIs(job.worker_start, first)
                self.assertIs(job.child, child)

    def test_constructor_publishes_none_without_clock_reads_or_identity_query(self):
        clock = Clock()
        api, observer = Mock(), Mock()
        with tempfile.TemporaryDirectory() as directory:
            job = subject.LeafAccountingJob(api, observer, Path(directory).resolve()/'worker.exe', self.leaves(clock))
        self.assertIsNone(job.child)
        self.assertEqual(clock.calls, [])
        observer.identity.assert_not_called()
        api.create_job.assert_called_once_with()
        api.configure.assert_called_once_with(job.handle)

    def test_rows_keep_original_decode_and_protocol_errors(self):
        from test_git_cpu_overhead import encode, transcript
        good = encode(*transcript(True))
        for raw in (b'\xff', b'HC074_GIT_CPU {\n', b'', good.replace(b'23589', b'23590')):
            expected = None
            try:
                original.rows(raw, True)
            except Exception as error:
                expected = (type(error), str(error))
            self.assertIsNotNone(expected)
            leaves = self.leaves(Clock())
            leaves.begin('decode', 'log_stat')
            leaves.end('decode', 'log_stat')
            leaves.begin('decode', 'log_read')
            with self.assertRaises(expected[0]) as caught:
                subject.leaf_rows(raw, True, leaves)
            self.assertEqual(str(caught.exception), expected[1])

    def test_bad_leaf_clock_values_freeze_both_groups(self):
        for value in (True, -1, 1.5):
            for kind in ('wall', 'cpu'):
                wall, cpu = Mock(return_value=value if kind == 'wall' else 0), Mock(return_value=value if kind == 'cpu' else 0)
                leaves = subject.Leaves(wall=wall, cpu=cpu, clock_domain='synthetic-control')
                leaves.begin('owner', 'owner_identity')
                before = (wall.call_count, cpu.call_count)
                leaves.begin('decode', 'log_stat')
                leaves.end('owner', 'owner_identity')
                self.assertEqual((wall.call_count, cpu.call_count), before)
                self.assertIn('invalid', leaves.first_refusal)

    def test_wall_and_cpu_regression_between_groups_refuse(self):
        class RegressClock(Clock):
            def read(self, kind):
                value = super().read(kind)
                return 0 if len(self.calls) == self.fail else value
        for read in (13, 14):
            clock = RegressClock()
            clock.fail = read
            # Do not use Clock's ordinary-fault branch for this regression.
            def raw(kind, clock=clock, read=read):
                clock.calls.append(kind)
                return 0 if len(clock.calls) == read else len(clock.calls)*10
            leaves = subject.Leaves(wall=lambda: raw('wall'), cpu=lambda: raw('cpu'), clock_domain='synthetic-control')
            for phase in subject.GROUPS['owner']:
                leaves.begin('owner', phase)
                leaves.end('owner', phase)
            leaves.begin('decode', 'log_stat')
            self.assertIn('groups clock sequence regressed', leaves.first_refusal)
            count = len(clock.calls)
            leaves.end('decode', 'log_stat')
            self.assertEqual(len(clock.calls), count)

    def test_incomplete_schedule_and_wrong_order_refuse_without_fabricating_ends(self):
        leaves = self.leaves(Clock())
        leaves.begin('decode', 'log_read')
        result = leaves.finish(True)
        self.assertFalse(result['capture_valid'])
        self.assertIn('unexpected leaf begin', result['first_refusal'])
        leaves = self.leaves(Clock())
        leaves.begin('owner', 'owner_identity')
        result = leaves.finish(False)
        span = result['groups']['owner']['spans'][0]
        self.assertEqual(span['status'], 'operation_interrupted')
        self.assertIsNone(span['end_probe'])
        self.assertIsNone(span['elapsed'])

    def cohort(self, folder, *, fail=False):
        image, gate = fixture.Model().orchestration(folder)
        data = fixture.attempts()
        if fail:
            data[0].update(passed=False, first_refusal='first leaf refusal')
        with patch.object(caller, 'ROOT', folder), patch.object(caller, 'pins', return_value={}), \
             patch.dict('sys.modules', package_effects_review_v6=gate), \
             patch.object(caller.shutil, 'disk_usage', return_value=Mock(free=caller.FREE_BYTES)), \
             patch.object(caller, 'artifact', return_value=(image, dict(source={}, head='head'))), \
             patch.object(caller, 'AccountingApi'), patch.object(caller, 'WindowsApi'), \
             patch.object(caller.subprocess, 'check_output', return_value='head'), \
             patch.object(caller.time, 'monotonic', return_value=1), \
             patch.object(subject, 'measure', side_effect=data) as measure, patch.object(subject, 'Leaves') as leaves, \
             patch.object(outer, 'Tape') as tapes:
            result = caller.run('build', 'first', coordinator_frames=True, capture_leaves=True)
        return result, measure, leaves, tapes

    def test_opt_in_caller_has_separate_output_and_same_single_origin(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            result, measure, leaves, tapes = self.cohort(folder)
            self.assertTrue(result['passed'])
            self.assertEqual((measure.call_count, leaves.call_count, tapes.call_count), (16, 16, 16))
            self.assertEqual({c.args[-3] for c in measure.call_args_list}, {121})
            intent = json.loads((folder/'target/git-cpu-capture-leaves-20261026/first/intent.json').read_bytes())
            self.assertEqual(intent['capture_leaves'], dict(enabled=True, schema='hc074-capture-leaves-v1'))
            self.assertEqual(result['capture_leaves_contract'], subject.contract())
            self.assertFalse((folder/'target/git-cpu-coordinator-frames-20261026').exists())

    def test_first_leaf_refusal_stops_cohort_without_subset_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            result, measure, _, _ = self.cohort(Path(directory), fail=True)
            self.assertFalse(result['passed'])
            self.assertIn('first leaf refusal', result['first_refusal'])
            self.assertEqual(measure.call_count, 1)
            self.assertNotIn('analysis', result)

    def test_import_schema_and_boolean_gates_refuse_before_apis(self):
        for kind in ('missing', 'schema'):
            with tempfile.TemporaryDirectory() as directory, patch.object(caller, 'ROOT', Path(directory)), \
                 patch.object(caller, 'AccountingApi') as api:
                with patch.dict('sys.modules', capture_leaves=None) if kind == 'missing' else \
                        patch.object(subject, 'contract', return_value=dict(schema='future')):
                    result = caller.run('build', 'first', coordinator_frames=True, capture_leaves=True)
                self.assertFalse(result['passed'])
                self.assertTrue((Path(directory)/'target/git-cpu-capture-leaves-20261026/first/intent.json').exists())
                api.assert_not_called()
        for value in (True, 1, None, 'true'):
            with self.assertRaises(original.Refusal):
                caller.run('build', 'first', capture_leaves=value)
        for argv in (['--capture-leaves'], ['--capture-leaves', '--coordinator-frames'], ['--coordinator-frames', '--other']):
            with patch('sys.argv', [caller.__file__, 'build', 'first', *argv]), self.assertRaises(original.Refusal):
                runpy.run_path(caller.__file__, run_name='__main__')


if __name__ == '__main__':
    unittest.main()
