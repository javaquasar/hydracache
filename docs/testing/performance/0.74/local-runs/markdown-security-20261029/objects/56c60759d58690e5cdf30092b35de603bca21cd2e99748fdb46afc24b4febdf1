"""Integrated synthetic custody/CLI controls, without native dispatch."""
import ast
import inspect
import json
from pathlib import Path
import runpy
import tempfile
import tomllib
import unittest
from unittest.mock import Mock, patch

import capture_leaves as old
import coordinator_frames as frames
import git_cpu_overhead as legacy
import git_cpu_spawn as caller
import spawn_capture as subject
from owned_process_observer import Refusal
from test_coordinator_frames import Clock as FrameClock
from test_spawn_frames import Clock
import test_git_cpu_overhead as fixture


class Strip(ast.NodeTransformer):
    def visit_Call(self, node):
        if isinstance(node.func, ast.Name) and node.func.id in ('SpawnAccountingJob', 'attach_spawn'):
            assert not node.keywords and isinstance(node.args[-1], ast.Name) and node.args[-1].id == 'spawn'
            if node.func.id == 'attach_spawn':
                assert len(node.args) == 2
                return self.visit(node.args[0])
            assert len(node.args) == 5
            node.args.pop()
            node.func.id = 'LeafAccountingJob'
        return self.generic_visit(node)


class Integration(unittest.TestCase):
    def test_protocol_exact_budget_and_closed_admission(self):
        doc = tomllib.loads(Path(subject.__file__).with_name('spawn-capture-protocol.toml').read_text())
        self.assertEqual(doc['schema'], subject.contract()['schema'])
        self.assertEqual(doc['cli_flags'], ['--spawn-frames'])
        self.assertEqual((doc['bound_seconds'], doc['cleanup_seconds'], doc['workers'], doc['calls_per_worker']), (120, 5, 16, 4))
        self.assertEqual(doc['max_recorder_clock_reads_with_outer_and_leaves'], 102)
        for key in ('pure_create_cost_isolated', 'existing_predicates_removed_or_reordered', 'window_renewed',
                    'overhead_accepted', 'package_pass', 'cleanup_instrumented'):
            self.assertIs(doc[key], False)
            self.assertIs(subject.contract()[key], False)

    def scenario(self, folder, *, integrated=True, cut=None, fatal=False, fault=False, outer_cut=None, late=False):
        api, observer, job, image = fixture.Model().frame(folder, legacy.PROFILE)
        held = job.child
        job.child = None
        original_spawn = job.spawn.side_effect
        clock, leaves = Clock(cut, fatal), old.Leaves(wall=lambda: 100, cpu=lambda: 10, clock_domain='synthetic-control')
        spawn = clock.tape()
        def create(*args):
            if integrated:
                spawn.begin('preparation')
                spawn.end('preparation')
                spawn.begin('create_to_retention')
            job.child = held
            if integrated:
                spawn.end('create_to_retention')
            for name in old.GROUPS['owner']:
                leaves.begin('owner', name)
                leaves.end('owner', name)
            if integrated:
                spawn.begin('membership_release')
                spawn.end('membership_release')
            return original_spawn(*args)
        job.spawn.side_effect = create
        if fault:
            observer.dll.GetProcessId.return_value = 43
        module, factory = (subject, 'SpawnAccountingJob') if integrated else (old, 'LeafAccountingJob')
        with patch.object(module, factory, return_value=job), \
             patch.object(module.time, 'monotonic', side_effect=[1, 1, 1, 6] if late else None, return_value=1) as deadlines, \
             patch.object(module.time, 'monotonic_ns', side_effect=[100, 300]), \
             patch.object(module.time, 'process_time_ns', side_effect=[10, 20]), \
             patch.object(module, 'cleanup', return_value=['owned cleanup error'] if fault else []) as cleanup:
            args = (api, observer, image, 0, 0, legacy.PROFILE, folder, 5, FrameClock(outer_cut).tape(), leaves)
            result = module.measure(*args, spawn) if integrated else module.measure(*args)
        return result, job, cleanup, deadlines.call_count, clock

    def test_measure_ast_preserves_original_body_and_new_attachment_only(self):
        a, b = (ast.parse(inspect.getsource(f)).body[0] for f in (old.measure, subject.measure))
        self.assertEqual(b.args.args.pop().arg, 'spawn')
        self.assertEqual(ast.dump(a), ast.dump(Strip().visit(b)))

    def test_differential_frame_keeps_fields_and_four_original_deadline_reads(self):
        with tempfile.TemporaryDirectory() as directory:
            paths = [Path(directory)/name for name in ('old', 'new')]
            for p in paths:
                p.mkdir()
            a, b = self.scenario(paths[0], integrated=False), self.scenario(paths[1])
            new = dict(b[0])
            self.assertTrue(new.pop('spawn_frames')['complete'])
            new['worker_image'] = a[0]['worker_image']
            self.assertEqual(a[0], new)
            self.assertEqual((a[3], b[3], len(b[4].calls)), (4, 4, 18))
            b[2].assert_not_called()
            b[1].child.close.assert_called_once_with()

    def test_all_18_ordinary_faults_refuse_diagnostic_after_original_healthy_close(self):
        for cut in range(1, 19):
            with self.subTest(cut=cut), tempfile.TemporaryDirectory() as directory:
                result, job, cleanup, deadlines, clock = self.scenario(Path(directory), cut=cut)
                self.assertFalse(result['passed'])
                self.assertTrue(result['operation_verdict']['passed'])
                self.assertTrue(result['coordinator_frames']['complete'])
                self.assertTrue(result['capture_leaves']['complete'])
                self.assertFalse(result['spawn_frames']['capture_valid'])
                self.assertIn('spawn telemetry', result['first_refusal'])
                self.assertEqual((deadlines, len(clock.calls)), (4, cut))
                job.child.close.assert_called_once_with()
                job.close.assert_called_once_with()
                cleanup.assert_not_called()

    def test_all_18_fatal_cuts_reach_original_cleanup_with_caller_child_unset(self):
        for cut in range(1, 19):
            with self.subTest(cut=cut), tempfile.TemporaryDirectory() as directory:
                result, job, cleanup, _, clock = self.scenario(Path(directory), cut=cut, fatal=True)
                self.assertFalse(result['passed'])
                self.assertFalse(result['operation_verdict']['passed'])
                self.assertIn('KeyboardInterrupt: fatal spawn clock cut', result['first_refusal'])
                cleanup.assert_called_once_with(job, None, 6)
                self.assertEqual(job.child is not None, cut >= 10)
                self.assertEqual(len(clock.calls), cut)

    def test_prior_operational_and_outer_errors_take_precedence_without_erasing_spawn_error(self):
        for fault, outer in ((True, None), (False, 1)):
            with tempfile.TemporaryDirectory() as directory:
                result, _, cleanup, _, _ = self.scenario(Path(directory), cut=1, fault=fault, outer_cut=outer)
                self.assertNotIn('spawn telemetry', result['first_refusal'])
                self.assertIn('ordinary spawn clock cut', result['spawn_frames']['first_refusal'])
                if fault:
                    self.assertEqual(result['cleanup_errors'], ['owned cleanup error'])
                    self.assertEqual(result['first_refusal'], result['operation_verdict']['first_refusal'])
                    self.assertEqual(cleanup.call_count, 1)
                else:
                    self.assertIn('coordinator telemetry', result['first_refusal'])
                    cleanup.assert_not_called()

    def test_original_final_deadline_remains_a_refusal_after_healthy_close(self):
        with tempfile.TemporaryDirectory() as directory:
            result, job, cleanup, checks, _ = self.scenario(Path(directory), late=True)
            self.assertIn('late worker frame witness', result['first_refusal'])
            self.assertFalse(result['operation_verdict']['passed'])
            self.assertTrue(result['spawn_frames']['complete'])
            self.assertEqual(checks, 4)
            job.close.assert_called_once_with()
            cleanup.assert_not_called()

    def cohort(self, folder, fail=False):
        image, gate = fixture.Model().orchestration(folder)
        data = fixture.attempts()
        if fail:
            data[0].update(passed=False, first_refusal='first integrated spawn refusal')
        with patch.object(caller, 'ROOT', folder), patch.object(caller, 'pins', return_value={}), \
             patch.dict('sys.modules', package_effects_review_v6=gate), \
             patch.object(caller.shutil, 'disk_usage', return_value=Mock(free=caller.FREE_BYTES)), \
             patch.object(caller, 'artifact', return_value=(image, dict(source={}, head='head'))), \
             patch.object(subject, 'SpawnAccountingApi') as api, patch.object(caller, 'WindowsApi'), \
             patch.object(caller.subprocess, 'check_output', return_value='head'), \
             patch.object(caller.time, 'monotonic', return_value=1), \
             patch.object(subject, 'measure', side_effect=data) as measure:
            result = caller.run('build', 'first', spawn_frames=True)
        return result, measure, api

    def test_caller_exclusive_intent_16_workers_and_same_origin(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)
            result, measure, api = self.cohort(folder)
            self.assertTrue(result['passed'])
            self.assertEqual(measure.call_count, 16)
            self.assertEqual({c.args[7] for c in measure.call_args_list}, {121})
            self.assertEqual(result['spawn_frames_contract'], subject.contract())
            intent = json.loads((folder/'target/git-cpu-spawn-capture-20261026/first/intent.json').read_bytes())
            self.assertEqual(intent['spawn_frames'], dict(enabled=True, schema='hc074-spawn-capture-v1'))
            self.assertFalse((folder/'target/git-cpu-capture-leaves-20261026').exists())
            with patch.object(caller, 'ROOT', folder), self.assertRaises(FileExistsError):
                caller.run('build', 'first', spawn_frames=True)
            api.assert_called_once_with()

    def test_first_worker_refusal_never_becomes_successful_subset(self):
        with tempfile.TemporaryDirectory() as directory:
            result, measure, _ = self.cohort(Path(directory), fail=True)
            self.assertFalse(result['passed'])
            self.assertIn('first integrated spawn refusal', result['first_refusal'])
            self.assertEqual(measure.call_count, 1)
            self.assertNotIn('analysis', result)

    def test_lazy_import_and_schema_failure_retain_intent_before_api(self):
        for kind in ('missing', 'schema'):
            with tempfile.TemporaryDirectory() as directory, patch.object(caller, 'ROOT', Path(directory)), \
                 patch.object(subject, 'SpawnAccountingApi') as api:
                with patch.dict('sys.modules', spawn_capture=None) if kind == 'missing' else \
                        patch.object(subject, 'contract', return_value=dict(schema='future')):
                    result = caller.run('build', 'first', spawn_frames=True)
                self.assertFalse(result['passed'])
                self.assertTrue((Path(directory)/'target/git-cpu-spawn-capture-20261026/first/intent.json').exists())
                api.assert_not_called()

    def test_old_schema_and_new_budget_or_policy_drift_refuse_before_api(self):
        for module in (old, frames):
            with tempfile.TemporaryDirectory() as directory, patch.object(caller, 'ROOT', Path(directory)), \
                 patch.object(module, 'contract', return_value=dict(schema='future')), \
                 patch.object(subject, 'SpawnAccountingApi') as api:
                self.assertFalse(caller.run('build', 'first', spawn_frames=True)['passed'])
                api.assert_not_called()
        for key, value in (('max_additional_clock_reads', 18.0), ('max_additional_probes', 7),
                           ('caller_connected', 1), ('window_renewed', True), ('overhead_accepted', True)):
            contract = subject.contract()
            contract[key] = value
            with tempfile.TemporaryDirectory() as directory, patch.object(caller, 'ROOT', Path(directory)), \
                 patch.object(subject, 'contract', return_value=contract), patch.object(subject, 'SpawnAccountingApi') as api:
                result = caller.run('build', 'first', spawn_frames=True)
                self.assertFalse(result['passed'])
                self.assertIn('scope drift', result['first_refusal'])
                api.assert_not_called()

    def test_only_explicit_true_and_exact_new_cli_select_this_path(self):
        for value in (False, 1, None, 'true'):
            with self.assertRaises(Refusal):
                caller.run('build', 'first', spawn_frames=value)
        for flags in ([], ['--capture-leaves'], ['--spawn-frames', '--spawn-frames'], ['--coordinator-frames', '--spawn-frames']):
            with patch('sys.argv', [caller.__file__, 'build', 'first', *flags]), \
                 self.assertRaises(Refusal):
                runpy.run_path(caller.__file__, run_name='__main__')


if __name__ == '__main__':
    unittest.main()
