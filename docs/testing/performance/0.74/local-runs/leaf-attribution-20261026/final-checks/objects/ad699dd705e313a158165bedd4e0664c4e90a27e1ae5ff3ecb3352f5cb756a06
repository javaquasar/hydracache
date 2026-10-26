"""Independent packet integrity and arithmetic controls."""
import copy
import importlib.util
import json
from pathlib import Path
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('leaf_packet_verifier', HERE/'verify.py')
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)


class Integrity(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.members = subject.load(HERE)

    def test_positive_independent_arithmetic(self):
        result = subject.audit(copy.deepcopy(self.members))
        self.assertEqual((result['workers'], result['checks_passed'], result['retained_format_refusals']), (16, 6, 1))

    def test_index_mutation_and_missing_object(self):
        with patch.object(Path, 'read_bytes', return_value=b'{}'), self.assertRaisesRegex(ValueError, 'index differs'):
            subject.load(HERE)
        with patch.object(Path, 'read_bytes', side_effect=FileNotFoundError('missing')), self.assertRaises(FileNotFoundError):
            subject.load(HERE)

    def test_raw_object_mutation(self):
        index = json.loads((HERE/'index.json').read_bytes())
        entry = next(iter(index['members'].values()))
        target, read = HERE/'objects'/entry['sha256'], Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: read(p)+b'x' if p == target else read(p)), \
             self.assertRaisesRegex(ValueError, 'raw member differs'):
            subject.load(HERE)

    def test_native_provenance_mutation(self):
        for key in ('native-index.json', 'native/result.json', 'native/0-0-reference_cpu_child.log'):
            members = copy.deepcopy(self.members)
            members[key] += b' '
            with self.assertRaisesRegex(ValueError, 'native index differs|native join differs'):
                subject.audit(members)

    def test_projection_blocks_totals_distributions_and_scope_mutations(self):
        for change in (
            lambda a: a['frames'][0]['leaves']['log_read'].update(wall_upper_ns=0),
            lambda a: a['frames'][0]['parent_complements']['owner'].update(process_cpu_ns=0),
            lambda a: a['frames'][0]['cpu_owners'].update(descendant_cpu_ns=0),
            lambda a: a['frames'][0].update(nested_worker_wall_ns={}),
            lambda a: a['frames'][0].update(log_bytes=1),
            lambda a: a['leaf_totals']['log_read'].update(wall_upper_ns=0),
            lambda a: a['leaf_distributions']['log_read'].update(zero_cpu_spans=0),
            lambda a: a['blocks'][0]['leaves']['log_read'].update(signed_wall_delta_lower_ns=0),
            lambda a: a['blocks'][2]['parent_complements']['owner'].update(signed_cpu_delta_ns=0),
            lambda a: a.update(overhead_accepted=True),
            lambda a: a.update(reference_worker_phases={}),
            lambda a: a['frames'].pop(),
        ):
            members = copy.deepcopy(self.members)
            value = json.loads(members['offline/result.json'])
            change(value['analysis'])
            members['offline/result.json'] = json.dumps(value).encode()
            with self.assertRaises(ValueError):
                subject.audit(members)

    def test_first_format_refusal_receipt_and_intent_are_preserved(self):
        for key, change in (
            ('checks/leaf-attribution-format/result.json', lambda d: d.update(exit=0)),
            ('checks/leaf-attribution-offline-first/result.json', lambda d: d.update(source_unchanged=False)),
            ('offline/intent.json', lambda d: d.update(processes_launched_by_analyzer=1)),
            ('offline/result.json', lambda d: d.update(first_refusal='first result')),
        ):
            members = copy.deepcopy(self.members)
            value = json.loads(members[key])
            change(value)
            members[key] = json.dumps(value).encode()
            with self.assertRaises(ValueError):
                subject.audit(members)


if __name__ == '__main__':
    unittest.main()
