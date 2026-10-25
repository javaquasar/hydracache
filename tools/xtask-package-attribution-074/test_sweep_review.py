"""Retained-source and malformed-proof controls; no package or Git execution."""
import ast
import copy
import json
from pathlib import Path
import unittest
import tomllib

import sweep_review as sweep


class SweepControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.values = sweep.load_snapshot(Path(__file__).resolve().parents[2])

    def test_exact_retained_lower_bound_not_time_or_package_pass(self):
        result = sweep.analyze_snapshot(self.values)
        self.assertEqual(result['target_reported_ns'], 108_310_000_000)
        self.assertEqual([row['loop_iterations'] for row in result['sweeps']], [450, 442, 442, 444, 445, 446, 447])
        self.assertEqual(result['proven_direct_show_launches_lower_bound'], 3116)
        self.assertEqual(result['proven_direct_reads_lower_bound'], 3116)
        self.assertFalse(result['package_pass'])
        self.assertEqual((result['completed_targets'], result['expected_targets'], result['unstarted_targets']), (111, 135, 23))
        self.assertFalse(result['elapsed_time_attributed'])
        self.assertFalse(result['multiplied_by_other_cohort_timing'])

    def test_prepared_helper_protocol_preserves_original_scope_and_bounds(self):
        path = Path(sweep.__file__).with_name('helper-attribution-protocol.toml')
        protocol = tomllib.loads(path.read_text())
        self.assertEqual(protocol['status'], 'prepared-not-executed')
        self.assertEqual(protocol['target_source_sha256'], sweep.SOURCE_SHA256)
        self.assertEqual((protocol['case_threads'], protocol['outer_bound_seconds']), (4, 780))
        for key in ('fresh_checks_cached', 'lookup_key_cache_candidate', 'object_cache_candidate',
                    'persistent_git_candidate', 'original_conditional_reads_or_pins_changed',
                    'live_frequency_order_or_deadlines_changed', 'retry_to_pass', 'package_replay',
                    'package_gate_completed_by_target_result', 'product', 'server', 'services',
                    'clocks', 'dependencies_changed', 'qualification'):
            self.assertIs(protocol[key], False, key)

    def test_source_drift_and_changed_bounds_refuse_before_proof(self):
        for replacement in (b'// comment drift\n', b'fn replacement() {}\n'):
            values = dict(self.values)
            values['reviewed-source/' + sweep.SOURCE] += replacement
            with self.assertRaises(ValueError):
                sweep.analyze_snapshot(values)
        values = dict(self.values)
        key = 'reviewed-source/' + sweep.SOURCE
        values[key] = values[key].replace(b'count(), 446', b'count(), 445')
        with self.assertRaises(ValueError):
            sweep.analyze_snapshot(values)

    def test_wrong_identity_retry_bound_and_source_maps_refuse(self):
        for mode in range(6):
            values = dict(self.values)
            key = sweep.PHASE + ('intent.json' if mode < 4 else 'result.json')
            row = json.loads(values[key])
            if mode == 0: row['head'] = '0' * 40
            elif mode == 1: row['retry'] = True
            elif mode == 2: row['bound_seconds'] = 781
            elif mode == 3: row['source'][sweep.SOURCE] = '0' * 64
            elif mode == 4: row['head_unchanged'] = False
            else: row['qualification'] = True
            values[key] = json.dumps(row).encode()
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                sweep.analyze_snapshot(values)

    def test_syntax_identity_scope_duplicate_and_missing_sites_refuse(self):
        key = 'syntax/final-closure/result.json'
        original = json.loads(self.values[key])
        for mode in range(5):
            values = dict(self.values)
            row = copy.deepcopy(original)
            source = next(v for v in row['sources'] if v['source'] == sweep.SOURCE)
            if mode == 0: row['executed_input_code'] = True
            elif mode == 1: source['sha256'] = '0' * 64
            elif mode == 2: row['sources'].append(source)
            elif mode == 3: source['sites'] = [v for v in source['sites'] if v['kind'] != 'command-constructor']
            else: source['sites'] = [v for v in source['sites'] if v['kind'] != 'function-attribute']
            values[key] = json.dumps(row).encode()
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                sweep.analyze_snapshot(values)

    def test_missing_pass_and_censored_target_cannot_supply_success_proof(self):
        key = sweep.PHASE + 'stdout.log'
        case = sweep.SWEEPS[0][0]
        for text in (self.values[key].decode().replace(f'test {case} ... ok', ''),
                     self.values[key].decode().replace(f'test {case} ... ok', f'test other_name ... ok')):
            values = dict(self.values, **{key: text.encode()})
            with self.assertRaises(ValueError):
                sweep.analyze_snapshot(values)

    def test_offline_reader_imports_no_execution_interfaces(self):
        tree = ast.parse(Path(sweep.__file__).read_text())
        imports = {node.names[0].name for node in ast.walk(tree) if isinstance(node, ast.Import)}
        self.assertEqual(imports, {'hashlib', 'json', 'sys', 'tomllib', 'review'})
        forbidden = {'subprocess', 'socket', 'os', 'time', 'exec', 'eval', 'compile'}
        self.assertFalse(any(isinstance(node, ast.Name) and node.id in forbidden for node in ast.walk(tree)))


if __name__ == '__main__':
    unittest.main()
