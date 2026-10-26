"""Mutation controls over sealed bytes; no source scanner or workload rerun."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_original_replays_offline(self):
        self.assertEqual(verify.verify(FOLDER)['sources'], 368)

    def test_index_change_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda path: b'{}' if path == FOLDER / 'index.json' else original(path)):
            with self.assertRaisesRegex(ValueError, 'index differs'):
                verify.verify(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def missing(path):
            if path.parent.name == 'objects':
                raise FileNotFoundError('injected missing object')
            return original(path)
        with patch.object(Path, 'read_bytes', missing):
            with self.assertRaises(FileNotFoundError):
                verify.verify(FOLDER)

    def test_source_syntax_receipts_and_unchanged_auxiliary_mutations_refuse(self):
        original = Path.read_bytes
        index = json.loads((FOLDER / 'index.json').read_bytes())
        for name in ('fixture/scripts/ci/watchdog_windows_job.py',
                     'fixture/tools/history-status-readers-074/rules.json',
                     'fixture/.cargo/config.toml', 'syntax/result.json',
                     'previous/v5-syntax.json',
                     'checks/package-v6-syntax-build-first/result.json'):
            target = FOLDER / 'objects' / index['members'][name]['sha256']
            with self.subTest(member=name), patch.object(Path, 'read_bytes',
                    lambda path: original(path) + b'x' if path == target else original(path)):
                with self.assertRaisesRegex(ValueError, 'raw member differs'):
                    verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
