"""Pure evidence-integrity controls; no Git, compiler or retained collector."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_raw_replay_without_workload(self):
        self.assertEqual(verify.verify(FOLDER)['git_calls'], 64)

    def test_index_mutation_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: b'{}' if p == FOLDER/'index.json' else original(p)):
            with self.assertRaisesRegex(ValueError, 'index differs'):
                verify.verify(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def read(p):
            if p.parent.name == 'objects':
                raise FileNotFoundError('missing raw object')
            return original(p)
        with patch.object(Path, 'read_bytes', read), self.assertRaises(FileNotFoundError):
            verify.verify(FOLDER)

    def test_first_failures_metrics_raw_logs_and_sources_cannot_change(self):
        index = json.loads((FOLDER/'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('baseline/direct_output_cost_074.rs', 'baseline/checks/git-job-cpu-rust-current/stdout.log',
                     'baseline/checks/git-job-cpu-msrv/stdout.log', 'source/crates/xtask/tests/direct_output_cost_074.rs',
                     'source/tools/xtask-package-attribution-074/git_cpu_overhead.py', 'raw-first-model/test_git_cpu_overhead.py',
                     'native/result.json', 'native/0-0-result.json', 'native/3-3-accounted_cpu_child.log'):
            target = FOLDER/'objects'/index['members'][name]['sha256']
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: original(p)+b'x' if p == target else original(p)):
                with self.assertRaisesRegex(ValueError, 'raw member differs'):
                    verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
