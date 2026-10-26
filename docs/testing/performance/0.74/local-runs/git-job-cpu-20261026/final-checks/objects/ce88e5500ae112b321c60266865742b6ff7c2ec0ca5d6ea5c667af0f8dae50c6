"""No processes or retained collectors; test immutable evidence integrity."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_offline_accounting_and_first_refusal_replay(self):
        result = verify.verify(FOLDER)
        self.assertEqual(result['non_launcher_descendant_cpu_ns'], 218750000)
        self.assertFalse(result['workload_replayed'])

    def test_changed_index_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: b'{}' if p == FOLDER/'index.json' else original(p)):
            with self.assertRaisesRegex(ValueError, 'index differs'):
                verify.verify(FOLDER)

    def test_missing_raw_object_refuses(self):
        original = Path.read_bytes
        def read(p):
            if p.parent.name == 'objects':
                raise FileNotFoundError('missing raw object')
            return original(p)
        with patch.object(Path, 'read_bytes', read), self.assertRaises(FileNotFoundError):
            verify.verify(FOLDER)

    def test_first_failures_counters_sources_and_raw_protocol_cannot_change(self):
        index = json.loads((FOLDER/'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('native/first/result.json', 'native/live-identity/result.json',
                     'native/live-identity/accounted_cpu_child.log',
                     'checks/git-job-cpu-rust-current/stdout.log', 'checks/git-job-cpu-msrv/stdout.log',
                     'source/tools/xtask-package-attribution-074/git_job_cpu.py',
                     'source/scripts/ci/watchdog_windows_job.py', 'raw-before-live-identity/git_job_cpu.py',
                     'raw-first-model/test_git_job_cpu.py', 'offline/first-pipe-fixture-difference.json'):
            target = FOLDER/'objects'/index['members'][name]['sha256']
            with self.subTest(member=name), patch.object(Path, 'read_bytes',
                    lambda p: original(p)+b'x' if p == target else original(p)):
                with self.assertRaisesRegex(ValueError, 'raw member differs'):
                    verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
