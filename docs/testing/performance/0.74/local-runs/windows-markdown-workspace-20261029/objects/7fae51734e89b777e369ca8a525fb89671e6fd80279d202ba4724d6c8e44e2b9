"""Fixed-byte and semantic controls; no workload, clock or collector imports."""
import ast
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import capture_attribution as subject


class Offline(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = subject.load_packet(subject.PACKET)

    def fresh(self):
        return copy.deepcopy(self.baseline)

    def test_all_samples_owners_and_nested_scopes_remain_distinct(self):
        result = subject.analyze(self.fresh())
        self.assertEqual((len(result['frames']), len(result['blocks']), len(result['profile_calls'])), (16, 4, 32))
        self.assertEqual(result['coordinator_phase_totals']['spawn']['process_cpu_ns'], 234375000)
        self.assertEqual(result['coordinator_phase_totals']['wait']['process_cpu_ns'], 15625000)
        self.assertEqual(result['coordinator_phase_totals']['decode']['wall_upper_ns'], 519883900)
        self.assertEqual(result['arithmetic_complement']['process_cpu_ns'], 0)
        self.assertIsNone(result['reference_call_wall_phases'])
        for call in result['profile_calls']:
            self.assertIsNone(call['phase_cpu_ns'])
            self.assertIsNone(call['absolute_monotonic_endpoints'])
        for key in ('nested_timing_scopes_additive', 'worker_phase_cpu_available', 'pure_api_cost_proved',
                    'zero_delta_means_zero_cost', 'causal_capture_cost_available', 'telemetry_on_off_compared',
                    'workload_replayed', 'overhead_accepted', 'speedup_established', 'package_pass'):
            self.assertIs(result[key], False)
        self.assertEqual(result['new_workload_calls'], 0)

    def test_signed_block_partition_closes_without_trimming(self):
        result = subject.analyze(self.fresh())
        block = result['blocks'][2]
        self.assertEqual(block['owners']['job_cpu_ns']['delta'], 859375000)
        self.assertEqual(block['owners']['worker_cpu_ns']['delta'], 375000000)
        self.assertEqual(block['owners']['descendant_cpu_ns']['delta'], 484375000)
        self.assertEqual(block['owners']['coordinator_cpu_ns']['delta'], -15625000)
        for block in result['blocks']:
            self.assertEqual(sum(p['signed_cpu_delta_ns'] for p in block['coordinator_phases'].values())+
                block['arithmetic_complement_cpu_delta_ns'], block['owners']['coordinator_cpu_ns']['delta'])
            for phase in block['coordinator_phases'].values():
                self.assertLessEqual(phase['signed_wall_delta_lower_ns'], phase['signed_wall_delta_upper_ns'])

    def test_zero_cpu_is_retained_not_promoted_to_zero_work(self):
        result = subject.analyze(self.fresh())
        for p in ('setup', 'terminal_reads', 'decode', 'close'):
            self.assertEqual(result['coordinator_phase_distributions'][p]['zero_cpu_spans'], 16)
            self.assertGreater(result['coordinator_phase_totals'][p]['wall_lower_ns'], 0)
        self.assertFalse(result['zero_delta_means_zero_cost'])

    def test_metric_complement_is_arithmetic_not_extra_phase(self):
        data = self.fresh()
        data['attempts'][0]['metrics']['wall_ns'] += 1234
        data['attempts'][0]['metrics']['coordinator_cpu_ns'] += 10
        original, result = (subject.analyze(v) for v in (self.fresh(), data))
        self.assertEqual(result['arithmetic_complement']['wall_lower_ns']-original['arithmetic_complement']['wall_lower_ns'], 1234)
        self.assertEqual(result['arithmetic_complement']['process_cpu_ns'], 10)
        self.assertFalse(result['frames'][0]['complement_is_pure_effect_cost'])
        self.assertEqual(set(result['frames'][0]['phases']), set(subject.PHASES))

    def test_incomplete_reordered_refused_and_promoted_cohort_refuse(self):
        for change in (
            lambda d: d['attempts'].pop(),
            lambda d: d['attempts'].reverse(),
            lambda d: d.update(passed=False),
            lambda d: d.update(first_refusal='first outcome'),
            lambda d: d.update(overhead_accepted=True),
            lambda d: d['attempts'][0]['cleanup_errors'].append('owned close refused'),
        ):
            data = self.fresh()
            change(data)
            with self.assertRaises(ValueError):
                subject.analyze(data)

    def test_operation_contract_and_native_domain_drift_refuse(self):
        for change in (
            lambda d: d['attempts'][0]['operation_verdict'].update(passed=False),
            lambda d: d['attempts'][0]['operation_verdict'].update(passed=1),
            lambda d: d['coordinator_frames_contract'].update(window_renewed=True),
            lambda d: d['coordinator_frames_contract'].update(cpu_is_native_git=0),
            lambda d: d['attempts'][0]['coordinator_frames'].update(clock_domain='synthetic-control'),
            lambda d: d['attempts'][0]['coordinator_frames'].update(complete=False),
            lambda d: d['attempts'][0]['coordinator_frames'].update(first_refusal='first clock error'),
            lambda d: d['attempts'][0]['coordinator_frames']['probes'].pop(),
            lambda d: d['attempts'][0]['coordinator_frames']['not_observed'].append('close'),
        ):
            data = self.fresh()
            change(data)
            with self.assertRaises(ValueError):
                subject.analyze(data)

    def test_probe_types_order_clock_regression_and_spans_refuse(self):
        for change in (
            lambda t: t['probes'][0]['raw'].update(process_cpu_ns=True),
            lambda t: t['probes'][0]['raw'].update(wall_before_ns=-1),
            lambda t: t['probes'][0]['raw'].update(wall_after_ns=0),
            lambda t: t['probes'][1]['raw'].update(process_cpu_ns=0),
            lambda t: t['probes'][2]['raw'].update(wall_before_ns=0),
            lambda t: t['probes'][0].update(kind='end'),
            lambda t: t['probes'][0].update(phase='close'),
            lambda t: t['spans'][0].update(start_probe=False),
            lambda t: t['spans'][0].update(status='open'),
            lambda t: t['spans'][0]['elapsed'].update(process_cpu_ns=True),
            lambda t: t['spans'][0]['elapsed'].update(wall_lower_ns=0),
        ):
            data = self.fresh()
            change(data['attempts'][0]['coordinator_frames'])
            with self.assertRaises(ValueError):
                subject.analyze(data)

    def test_phase_sums_cannot_exceed_original_frame_metrics(self):
        for key in ('wall_ns', 'coordinator_cpu_ns'):
            data = self.fresh()
            row = data['attempts'][1]
            row['metrics'][key] = 0
            with self.assertRaises(ValueError):
                subject.analyze(data)

    def test_existing_owner_and_worker_phase_faults_still_refuse(self):
        for change in (
            lambda d: d['attempts'][0]['job'].update(active=1),
            lambda d: d['attempts'][0]['worker'].update(creation_100ns=1),
            lambda d: d['attempts'][1]['records'][0]['phases'].update(spawn_ns=True),
            lambda d: d['attempts'][0]['records'][0].update(stdout_bytes=0),
            lambda d: d['attempts'][1]['native_inclusive']['non_launcher_descendant_cpu'].update(user_ns=-1),
        ):
            data = self.fresh()
            change(data)
            with self.assertRaises(ValueError):
                subject.analyze(data)

    def test_index_mutation_refuses_before_object_reads(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaisesRegex(ValueError, 'fixed input index differs'):
            subject.load_packet(Path('.'))

    def target(self):
        index = json.loads((subject.PACKET/'index.json').read_bytes())
        item = next(iter(index['members'].values()))
        return subject.PACKET/'objects'/item['sha256'], item['bytes']

    def test_size_missing_and_byte_mutation_refuse(self):
        target, size = self.target()
        read, stat = Path.read_bytes, Path.stat
        with patch.object(Path, 'stat', lambda p, *a, **k: Mock(st_size=size+1) if p == target else stat(p, *a, **k)), \
             self.assertRaisesRegex(ValueError, 'raw input size differs'):
            subject.load_packet(subject.PACKET)
        def missing(p):
            if p == target:
                raise FileNotFoundError('retained object missing')
            return read(p)
        with patch.object(Path, 'read_bytes', missing), self.assertRaises(FileNotFoundError):
            subject.load_packet(subject.PACKET)
        with patch.object(Path, 'read_bytes', lambda p: read(p)+b'x' if p == target else read(p)), \
             self.assertRaisesRegex(ValueError, 'raw input differs'):
            subject.load_packet(subject.PACKET)

    def test_import_boundary_has_no_process_clock_or_retained_code(self):
        tree = ast.parse(Path(subject.__file__).read_text())
        imports = {n.names[0].name for n in ast.walk(tree) if isinstance(n, ast.Import)}
        imports.update(n.module for n in ast.walk(tree) if isinstance(n, ast.ImportFrom))
        self.assertEqual(imports, {'hashlib', 'json', 'pathlib', 're', 'sys', 'git_cpu_variance'})
        self.assertFalse(any(isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and
                             n.func.id in ('eval', 'exec', 'compile', '__import__') for n in ast.walk(tree)))

    def test_exclusive_first_refusal_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
             patch.object(subject, 'load_packet', side_effect=ValueError('first retained input refusal')) as load:
            result = subject.run('first')
            self.assertEqual(result['first_refusal'], 'ValueError: first retained input refusal')
            self.assertFalse(result['passed'])
            path = Path(directory)/'target/capture-attribution-20261026/first/result.json'
            before = path.read_bytes()
            with self.assertRaises(FileExistsError):
                subject.run('first')
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(load.call_count, 1)

    def test_successful_saved_analysis_and_bad_labels(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
             patch.object(subject, 'load_packet', return_value=self.fresh()):
            self.assertTrue(subject.run('first')['passed'])
            for label in ('', '../escape', 'UPPER', 'x'*65):
                with self.assertRaises(ValueError):
                    subject.run(label)


if __name__ == '__main__':
    unittest.main()
