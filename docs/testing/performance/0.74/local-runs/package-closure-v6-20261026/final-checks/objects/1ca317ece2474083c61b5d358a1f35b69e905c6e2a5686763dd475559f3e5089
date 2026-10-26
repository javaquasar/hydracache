"""No workload dispatch; strict sealed-result mutation controls."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_original_replays_offline(self):
        self.assertEqual(verify.verify(FOLDER)['completed'], 72)

    def test_changed_index_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda path: b'{}' if path == FOLDER / 'index.json' else original(path)):
            with self.assertRaisesRegex(ValueError, 'index differs'):
                verify.verify(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def read(path):
            if path.parent.name == 'objects':
                raise FileNotFoundError('injected missing object')
            return original(path)
        with patch.object(Path, 'read_bytes', read):
            with self.assertRaises(FileNotFoundError):
                verify.verify(FOLDER)

    def test_source_streams_accounting_cleanup_and_first_refusal_cannot_change(self):
        index = json.loads((FOLDER / 'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('derived/accounting.json', 'coordinator/first/gate.json',
                     'checks/package-v6-first/stdout.log', 'checks/package-v6-first/stderr.log',
                     'checks/package-v6-first/result.json', 'checks/package-v6-first/owned-cleanup.log',
                     'checks/package-v6-controls-first/stderr.log',
                     'reconstructed-first-source/tools/historical-blob-batch-074/test_package_replay_v6.py',
                     'auxiliary/scripts/ci/watchdog_windows_job.py'):
            target = FOLDER / 'objects' / index['members'][name]['sha256']
            with self.subTest(member=name), patch.object(Path, 'read_bytes',
                    lambda path: original(path) + b'x' if path == target else original(path)):
                with self.assertRaisesRegex(ValueError, 'raw member differs'):
                    verify.verify(FOLDER)

    def test_unreviewed_local_parser_refuses(self):
        target = verify.ROOT / 'tools/xtask-package-attribution-074/review.py'
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda path: b'changed' if path == target else original(path)):
            with self.assertRaisesRegex(ValueError, 'fixed local parser differs'):
                verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
