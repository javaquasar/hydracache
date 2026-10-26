"""Independent native packet integrity controls; no worker/collector import."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('spawn_native_verify', HERE/'verify.py')
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)


class Integrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.members = subject.load(HERE)

    def test_positive_raw_joins_and_nested_arithmetic(self):
        r = subject.audit(self.members)
        self.assertEqual((r['workers'], r['git_calls'], r['native_attempts']), (16, 64, 1))
        s = r['spawn_summary']
        self.assertEqual((s['spawn_probes'], s['total_recorder_probes'], s['total_recorder_clock_reads']), (96, 544, 1632))
        self.assertEqual(s['phase_totals']['create_to_retention']['process_cpu_ns'], 234375000)
        self.assertEqual(s['phase_totals']['preparation']['wall_upper_ns'], 4413300)
        self.assertEqual(s['parent_complement_after_spawn_and_owner_leaves']['process_cpu_ns'], 15625000)
        self.assertFalse(r['overhead_accepted'])
        self.assertFalse(r['package_pass'])

    def test_index_missing_and_raw_mutations_refuse(self):
        read = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: b'{}' if p == HERE/'index.json' else read(p)), \
             self.assertRaisesRegex(ValueError, 'index differs'):
            subject.load(HERE)
        def missing(p):
            if p.parent.name == 'objects':
                raise FileNotFoundError('missing')
            return read(p)
        with patch.object(Path, 'read_bytes', missing), self.assertRaises(FileNotFoundError):
            subject.load(HERE)
        index = json.loads((HERE/'index.json').read_bytes())
        for name in ('native/result.json', 'native/build.json', 'native/0-0-result.json',
                     'native/0-0-reference_cpu_child.log', 'first-test-spawn-capture.py',
                     'source/tools/xtask-package-attribution-074/spawn_capture.py'):
            target = HERE/'objects'/index['members'][name]['sha256']
            with patch.object(Path, 'read_bytes', lambda p: read(p)+b'x' if p == target else read(p)), \
                 self.assertRaisesRegex(ValueError, 'raw member differs'):
                subject.load(HERE)

    def test_spawn_contract_order_types_completeness_and_arithmetic_faults(self):
        base = json.loads(self.members['native/result.json'])
        for change in (
            lambda t: t.update(complete=False),
            lambda t: t.update(first_refusal='first spawn clock refusal'),
            lambda t: t.update(clock_domain='synthetic-control'),
            lambda t: t['contract'].update(window_renewed=True),
            lambda t: t['contract'].update(window_renewed=0),
            lambda t: t['probes'][0]['raw'].update(process_cpu_ns=True),
            lambda t: t['probes'][0].update(kind='end'),
            lambda t: t['probes'][1]['raw'].update(wall_before_ns=0),
            lambda t: t['spans'][0]['elapsed'].update(wall_upper_ns=0),
            lambda t: t['spans'][0].update(start_probe=False),
            lambda t: t['probes'].pop(),
        ):
            data = copy.deepcopy(base)
            change(data['attempts'][0]['spawn_frames'])
            with self.assertRaises(ValueError):
                subject.spawn_audit(data)

    def test_preserved_local_deltas_cannot_escape_parent_and_owner_order(self):
        data = json.loads(self.members['native/result.json'])
        for probe in data['attempts'][0]['spawn_frames']['probes']:
            probe['raw']['wall_before_ns'] += 10**15
            probe['raw']['wall_after_ns'] += 10**15
        with self.assertRaisesRegex(ValueError, 'outside original parent or custody order'):
            subject.spawn_audit(data)

    def test_old_leaf_domains_counters_and_parent_containment_still_refuse(self):
        base = json.loads(self.members['native/result.json'])
        for change in (
            lambda t: t['groups']['owner'].update(clock_domain='synthetic-control'),
            lambda t: t['groups']['owner']['probes'][0]['raw'].update(process_cpu_ns=True),
            lambda t: t['groups']['decode']['spans'][0]['elapsed'].update(wall_upper_ns=0),
        ):
            data = copy.deepcopy(base)
            change(data['attempts'][0]['capture_leaves'])
            with self.assertRaises(ValueError):
                subject.leaf_audit(data, self.members)

    def test_operational_verdict_and_explicit_new_gate_cannot_be_promoted(self):
        members = dict(self.members)
        data = json.loads(members['native/result.json'])
        data['attempts'][0]['operation_verdict']['passed'] = False
        members['native/result.json'] = json.dumps(data).encode()
        members['native/0-0-result.json'] = json.dumps(data['attempts'][0]).encode()
        with self.assertRaisesRegex(ValueError, 'operation differs'):
            subject.audit(members)
        members = dict(self.members)
        intent = json.loads(members['native/intent.json'])
        intent['spawn_frames']['enabled'] = False
        members['native/intent.json'] = json.dumps(intent).encode()
        with self.assertRaisesRegex(ValueError, 'explicit spawn intent differs'):
            subject.audit(members)

    def test_first_fixture_exit_and_source_are_preserved(self):
        members = dict(self.members)
        label = 'checks/spawn-capture-models-first/result.json'
        row = json.loads(members[label])
        row['exit'] = 0
        members[label] = json.dumps(row).encode()
        with self.assertRaisesRegex(ValueError, 'model/check receipt differs'):
            subject.model_audit(members)
        members = dict(self.members)
        members['first-test-spawn-capture.py'] += b' '
        with self.assertRaisesRegex(ValueError, 'first test source differs'):
            subject.model_audit(members)


if __name__ == '__main__':
    unittest.main()
