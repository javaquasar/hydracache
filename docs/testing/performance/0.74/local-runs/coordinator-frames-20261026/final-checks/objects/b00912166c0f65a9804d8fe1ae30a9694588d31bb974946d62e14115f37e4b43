"""Evidence-integrity controls, without executing any retained collector."""
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_arithmetic_and_ast_replay_without_native_control(self):
        self.assertTrue(verify.verify(FOLDER)['operational_ast_preserved'])

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

    def test_first_sources_refusals_raw_examples_and_operational_ast_cannot_change(self):
        index = json.loads((FOLDER/'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('baseline/git_cpu_overhead.py', 'raw-first-model/coordinator_frames.py',
                     'source/tools/xtask-package-attribution-074/coordinator_frames.py',
                     'source/tools/xtask-package-attribution-074/git_cpu_overhead.py',
                     'synthetic/result.json', 'synthetic/intent.json',
                     'checks/coordinator-frames-model-precommit/stderr.log'):
            target = FOLDER/'objects'/index['members'][name]['sha256']
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: original(p)+b'x' if p == target else original(p)), \
                 self.assertRaisesRegex(ValueError, 'raw member differs'):
                verify.verify(FOLDER)


if __name__ == '__main__':
    unittest.main()
