"""Fixed-input arithmetic, fault and custody controls; no native execution."""
import ast
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import Mock, patch

import leaf_attribution as subject


class Offline(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.baseline = subject.load_packet(subject.PACKET)

    def fresh(self):
        return copy.deepcopy(self.baseline)

    def test_all_workers_nested_scopes_zeros_and_raw_totals(self):
        a = subject.analyze(*self.fresh())
        self.assertEqual((len(a['frames']), len(a['blocks']), len(a['outer']['profile_calls'])), (16, 4, 32))
        self.assertEqual(a['leaf_totals']['log_read']['wall_upper_ns'], 380845500)
        self.assertEqual(a['parent_complement_totals']['owner']['process_cpu_ns'], 2375000000)
        self.assertEqual(a['parent_complement_totals']['decode']['wall_lower_ns'], 1556700)
        self.assertEqual((a['leaf_probes'], a['total_recorder_clock_reads']), (256, 1344))
        for p in a['leaf_distributions'].values():
            self.assertEqual(p['zero_cpu_spans'], 16)
            self.assertGreater(p['wall_lower_ns']['sum'], 0)
        self.assertIsNone(a['frames'][0]['nested_worker_wall_ns'])
        self.assertIsNotNone(a['frames'][1]['nested_worker_wall_ns'])
        for flag in ('nested_scopes_additive', 'worker_phase_cpu_available', 'zero_delta_means_zero_work',
                     'pure_api_cost_proved', 'causal_read_size_relation_proved', 'telemetry_on_off_compared',
                     'workload_replayed', 'overhead_accepted', 'speedup_established', 'package_pass'):
            self.assertIs(a[flag], False)

    def test_signed_blocks_close_and_read_size_order_is_not_latency_order(self):
        a = subject.analyze(*self.fresh())
        for block, outer in zip(a['blocks'], a['outer']['blocks']):
            for group, phases in subject.GROUPS.items():
                parent = 'spawn' if group == 'owner' else 'decode'
                self.assertEqual(sum(block['leaves'][p]['signed_cpu_delta_ns'] for p in phases) +
                    block['parent_complements'][group]['signed_cpu_delta_ns'],
                    outer['coordinator_phases'][parent]['signed_cpu_delta_ns'])
            for field in ('leaves', 'parent_complements'):
                for value in block[field].values():
                    self.assertLessEqual(value['signed_wall_delta_lower_ns'], value['signed_wall_delta_upper_ns'])
        self.assertTrue(any(b['leaves']['log_read']['signed_wall_delta_upper_ns'] < 0 for b in a['blocks']))
        refs = [f for f in a['frames'] if f['mode'] == subject.REFERENCE]
        self.assertEqual({f['log_bytes'] for f in refs}, {1264})
        self.assertGreater(max(f['leaves']['log_read']['wall_lower_ns'] for f in refs),
                           min(f['leaves']['log_read']['wall_upper_ns'] for f in refs))

    def test_incomplete_schedule_operation_and_owner_faults_refuse(self):
        for change in (
            lambda d: d['attempts'].pop(),
            lambda d: d['attempts'].reverse(),
            lambda d: d.update(passed=False),
            lambda d: d.update(overhead_accepted=True),
            lambda d: d['attempts'][0]['operation_verdict'].update(passed=False),
            lambda d: d['attempts'][0]['job'].update(active=1),
            lambda d: d['attempts'][1]['records'][0].update(stdout_bytes=0),
        ):
            data, logs = self.fresh()
            change(data)
            with self.assertRaises(ValueError):
                subject.analyze(data, logs)

    def test_contract_and_domain_types_and_partial_groups_refuse(self):
        for change in (
            lambda d: d['capture_leaves_contract'].update(window_renewed=0),
            lambda d: d['capture_leaves_contract'].update(max_additional_probes=16.0),
            lambda d: d['capture_leaves_contract'].update(window_renewed=True),
            lambda d: d['attempts'][0]['capture_leaves'].update(complete=False),
            lambda d: d['attempts'][0]['capture_leaves'].update(capture_valid=1),
            lambda d: d['attempts'][0]['capture_leaves'].update(first_refusal='first error'),
            lambda d: d['attempts'][0]['capture_leaves']['groups']['owner'].update(clock_domain='synthetic-control'),
            lambda d: d['attempts'][0]['capture_leaves']['groups']['owner']['probes'].pop(),
            lambda d: d['attempts'][0]['capture_leaves']['groups']['decode']['not_observed'].append('log_read'),
        ):
            data, logs = self.fresh()
            change(data)
            with self.assertRaises(ValueError):
                subject.analyze(data, logs)

    def test_probe_order_integer_clock_and_span_mutations_refuse(self):
        for change in (
            lambda t: t['probes'][0]['raw'].update(process_cpu_ns=True),
            lambda t: t['probes'][0]['raw'].update(wall_before_ns=-1),
            lambda t: t['probes'][1]['raw'].update(wall_before_ns=0),
            lambda t: t['probes'][1]['raw'].update(process_cpu_ns=0),
            lambda t: t['probes'][0].update(kind='end'),
            lambda t: t['spans'][0].update(start_probe=False),
            lambda t: t['spans'][0].update(status='open'),
            lambda t: t['spans'][0]['elapsed'].update(process_cpu_ns=True),
            lambda t: t['spans'][0]['elapsed'].update(wall_lower_ns=0),
        ):
            data, logs = self.fresh()
            change(data['attempts'][0]['capture_leaves']['groups']['owner'])
            with self.assertRaises(ValueError):
                subject.analyze(data, logs)

    def test_preserved_local_deltas_cannot_escape_parent(self):
        for group in subject.GROUPS:
            data, logs = self.fresh()
            for p in data['attempts'][0]['capture_leaves']['groups'][group]['probes']:
                for key in ('wall_before_ns', 'wall_after_ns'):
                    p['raw'][key] += 10**15
            with self.assertRaisesRegex(ValueError, 'outside original parent|clock sequence regressed'):
                subject.analyze(data, logs)

    def test_log_inventory_and_bounds_refuse(self):
        for change in (lambda logs: logs.pop((0, 0)), lambda logs: logs.update({(0, 0): 'not bytes'}),
                       lambda logs: logs.update({(0, 0): b'x'*(16*1024**2+1)})):
            data, logs = self.fresh()
            change(logs)
            with self.assertRaisesRegex(ValueError, 'log inventory differs'):
                subject.analyze(data, logs)

    def test_fixed_index_and_object_size_missing_corruption_refuse(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaisesRegex(ValueError, 'fixed input index'):
            subject.load_packet(Path('.'))
        index = json.loads((subject.PACKET/'index.json').read_bytes())
        item = next(iter(index['members'].values()))
        target = subject.PACKET/'objects'/item['sha256']
        read, stat = Path.read_bytes, Path.stat
        with patch.object(Path, 'stat', lambda p, *a, **k: Mock(st_size=item['bytes']+1) if p == target else stat(p, *a, **k)), \
             self.assertRaisesRegex(ValueError, 'raw input size differs'):
            subject.load_packet(subject.PACKET)
        def missing(p):
            if p == target:
                raise FileNotFoundError('first missing object')
            return read(p)
        with patch.object(Path, 'read_bytes', missing), self.assertRaises(FileNotFoundError):
            subject.load_packet(subject.PACKET)
        with patch.object(Path, 'read_bytes', lambda p: read(p)+b'x' if p == target else read(p)), \
             self.assertRaisesRegex(ValueError, 'raw input differs'):
            subject.load_packet(subject.PACKET)

    def test_imports_exclude_process_clock_collector_and_retained_code(self):
        tree = ast.parse(Path(subject.__file__).read_text())
        imports = {n.names[0].name for n in ast.walk(tree) if isinstance(n, ast.Import)}
        imports.update(n.module for n in ast.walk(tree) if isinstance(n, ast.ImportFrom))
        self.assertEqual(imports, {'hashlib', 'json', 'pathlib', 're', 'sys', 'capture_attribution', 'git_cpu_variance'})
        self.assertFalse(any(isinstance(n, ast.Call) and isinstance(n.func, ast.Name) and
            n.func.id in ('eval', 'exec', 'compile', '__import__') for n in ast.walk(tree)))

    def test_exclusive_first_refusal_and_success_custody(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
             patch.object(subject, 'load_packet', side_effect=ValueError('first input refusal')) as load:
            self.assertEqual(subject.run('first')['first_refusal'], 'ValueError: first input refusal')
            path = Path(directory)/'target/leaf-attribution-20261026/first/result.json'
            before = path.read_bytes()
            with self.assertRaises(FileExistsError):
                subject.run('first')
            self.assertEqual(path.read_bytes(), before)
            self.assertEqual(load.call_count, 1)
        with tempfile.TemporaryDirectory() as directory, patch.object(subject, 'ROOT', Path(directory)), \
             patch.object(subject, 'load_packet', return_value=self.fresh()):
            self.assertTrue(subject.run('first')['passed'])
            for label in ('', '../escape', 'UPPER', 'x'*65):
                with self.assertRaises(ValueError):
                    subject.run(label)


if __name__ == '__main__':
    unittest.main()
