"""Pure retained-data controls: no clocks, workers or retained-code imports."""
import copy
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.members = verify.load(FOLDER)

    def test_first_native_result_and_bracket_totals(self):
        result = verify.audit(self.members)
        self.assertEqual((result['native_attempts'], result['workers'], result['git_calls'],
                          result['probes'], result['clock_reads']), (1, 16, 64, 192, 576))
        self.assertEqual(result['whole_coordinator_cpu_ns'], 250000000)
        self.assertEqual(result['phase_totals']['spawn']['process_cpu_ns'], 234375000)
        self.assertEqual(result['phase_totals']['wait']['process_cpu_ns'], 15625000)
        self.assertFalse(result['overhead_accepted'])
        self.assertFalse(result['package_pass'])
        self.assertEqual(result['new_workload_calls'], 0)

    def test_index_corruption_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: b'{}' if p == FOLDER/'index.json' else original(p)), \
             self.assertRaisesRegex(ValueError, 'index differs'):
            verify.load(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def read(p):
            if p.parent.name == 'objects':
                raise FileNotFoundError('missing object')
            return original(p)
        with patch.object(Path, 'read_bytes', read), self.assertRaises(FileNotFoundError):
            verify.load(FOLDER)

    def test_raw_receipts_results_logs_and_sources_cannot_change(self):
        index = json.loads((FOLDER/'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('native/result.json', 'native/build.json', 'native/0-0-result.json',
                     'native/0-0-reference_cpu_child.log', 'checks/coordinator-frames-native-first/result.json',
                     'source/tools/xtask-package-attribution-074/coordinator_frames.py', 'baseline/model-index.json'):
            target = FOLDER/'objects'/index['members'][name]['sha256']
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: original(p)+b'x' if p == target else original(p)), \
                 self.assertRaisesRegex(ValueError, 'raw member differs'):
                verify.load(FOLDER)

    def attempt_change(self, change):
        members = dict(self.members)
        result = json.loads(members['native/result.json'])
        change(result['attempts'][0])
        members['native/0-0-result.json'] = json.dumps(result['attempts'][0]).encode()
        members['native/result.json'] = json.dumps(result).encode()
        return members

    def test_consistent_duplicate_ledgers_do_not_hide_semantic_faults(self):
        changes = (
            lambda a: a['operation_verdict'].update(passed=False),
            lambda a: a['worker'].update(creation_100ns=1),
            lambda a: a['job'].update(active=1),
            lambda a: a['metrics'].update(whole_job_cpu_ns=0),
            lambda a: a['coordinator_frames'].update(clock_domain='synthetic-control'),
            lambda a: a['coordinator_frames'].update(complete=False),
            lambda a: a['coordinator_frames']['probes'][1]['raw'].update(process_cpu_ns=-1),
            lambda a: a['coordinator_frames']['probes'][2].update(phase='decode'),
            lambda a: a['coordinator_frames']['spans'][0]['elapsed'].update(wall_upper_ns=0),
            lambda a: a['coordinator_frames']['probes'].pop(),
            lambda a: a['records'][0].update(stdout_bytes=1),
        )
        for number, change in enumerate(changes):
            with self.subTest(number=number), self.assertRaises(ValueError):
                verify.audit(self.attempt_change(change))

    def test_deadline_build_pin_and_block_summary_drift_refuse(self):
        for name, change in (
            ('native/intent.json', lambda v: v.update(bound_seconds=121)),
            ('native/build.json', lambda v: v.update(head='0'*40)),
            ('native/result.json', lambda v: v['pins_before'].clear()),
            ('native/result.json', lambda v: v['analysis']['blocks'][0]['metrics']['wall_ns'].update(delta=0)),
            ('native/result.json', lambda v: v.update(overhead_accepted=True)),
        ):
            members = copy.copy(self.members)
            value = json.loads(members[name])
            change(value)
            members[name] = json.dumps(value).encode()
            with self.subTest(name=name), self.assertRaises(ValueError):
                verify.audit(members)


if __name__ == '__main__':
    unittest.main()
