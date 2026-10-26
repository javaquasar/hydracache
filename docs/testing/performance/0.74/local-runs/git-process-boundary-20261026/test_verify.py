"""Pure immutable-packet integrity refusals; no process launch."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_first_result_replays_offline(self):
        self.assertEqual(verify.verify(FOLDER)['distinct_held_processes'], 2)

    def test_index_mutation_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda path: b'{}' if path == FOLDER / 'index.json' else original(path)):
            with self.assertRaisesRegex(ValueError, 'index differs'):
                verify.verify(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def read(path):
            if path.parent.name == 'objects':
                raise FileNotFoundError('missing object')
            return original(path)
        with patch.object(Path, 'read_bytes', read):
            with self.assertRaises(FileNotFoundError):
                verify.verify(FOLDER)

    def test_identities_pins_streams_source_first_failure_and_raw_baseline_cannot_change(self):
        index = json.loads((FOLDER / 'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('native/result.json', 'native/intent.json', 'native/stdout.log',
                     'source/tools/xtask-package-attribution-074/git_process_boundary.py',
                     'source/tools/xtask-package-attribution-074/process_lineage.py',
                     'raw-before-close-budget/git_process_boundary.py',
                     'checks/git-boundary-close-budget-first/stderr.log',
                     'checks/git-boundary-native-first/result.json'):
            target = FOLDER / 'objects' / index['members'][name]['sha256']
            with self.subTest(member=name), patch.object(Path, 'read_bytes',
                    lambda path: original(path) + b'x' if path == target else original(path)):
                with self.assertRaisesRegex(ValueError, 'raw member differs'):
                    verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
