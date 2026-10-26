"""Immutable offline result integrity; no workloads or retained-code import."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_recomputes_cpu_owners_without_replaying_workload(self):
        result = verify.verify(FOLDER)
        self.assertEqual(result['new_workload_calls'], 0)
        self.assertEqual(result['observation_wall_sum_ns'], 643500)

    def test_index_mutation_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: b'{}' if p == FOLDER/'index.json' else original(p)), \
             self.assertRaisesRegex(ValueError, 'index differs'):
            verify.verify(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def read(p):
            if p.parent.name == 'objects':
                raise FileNotFoundError('missing retained object')
            return original(p)
        with patch.object(Path, 'read_bytes', read), self.assertRaises(FileNotFoundError):
            verify.verify(FOLDER)

    def test_input_counters_first_results_and_analysis_source_cannot_change(self):
        index = json.loads((FOLDER/'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('input/index.json', 'input/native/result.json', 'input/native/2-2-result.json',
                     'input/source/tools/xtask-package-attribution-074/pipe_capture.rs',
                     'analysis/result.json', 'analysis/intent.json', 'raw-first-model/test_git_cpu_variance.py',
                     'source/tools/xtask-package-attribution-074/git_cpu_variance.py',
                     'checks/git-cpu-variance-analysis-first/result.json'):
            target = FOLDER/'objects'/index['members'][name]['sha256']
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: original(p)+b'x' if p == target else original(p)), \
                 self.assertRaisesRegex(ValueError, 'raw member differs'):
                verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
