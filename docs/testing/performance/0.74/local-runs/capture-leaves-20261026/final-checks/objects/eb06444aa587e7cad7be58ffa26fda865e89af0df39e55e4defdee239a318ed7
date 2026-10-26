"""Offline integrity only, with no retained-code execution."""
import ast
import json
from pathlib import Path
import unittest
from unittest.mock import patch

import verify

FOLDER = Path(__file__).resolve().parent


class Integrity(unittest.TestCase):
    def test_source_ast_and_first_result(self):
        result = verify.verify(FOLDER)
        self.assertTrue(result['operational_ast_preserved'])
        self.assertTrue(result['first_fixture_refusal_preserved'])

    def test_index_mutation_refuses(self):
        original = Path.read_bytes
        with patch.object(Path, 'read_bytes', lambda p: b'{}' if p == FOLDER/'index.json' else original(p)), \
             self.assertRaisesRegex(ValueError, 'index differs'):
            verify.verify(FOLDER)

    def test_missing_object_refuses(self):
        original = Path.read_bytes
        def read(p):
            if p.parent.name == 'objects':
                raise FileNotFoundError('missing retained member')
            return original(p)
        with patch.object(Path, 'read_bytes', read), self.assertRaises(FileNotFoundError):
            verify.verify(FOLDER)

    def test_original_final_first_failure_and_source_bytes_cannot_change(self):
        index = json.loads((FOLDER/'index.json').read_bytes())
        original = Path.read_bytes
        for name in ('source/tools/xtask-package-attribution-074/capture_leaves.py',
                     'baseline/source/scripts/ci/watchdog_windows_job.py',
                     'history/first12/test_capture_leaves.py', 'checks/capture-leaves-model-first/stderr.log',
                     'checks/capture-leaves-postcommit/result.json', 'qualification-manifest.toml'):
            target = FOLDER/'objects'/index['members'][name]['sha256']
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: original(p)+b'x' if p == target else original(p)), \
                 self.assertRaisesRegex(ValueError, 'raw member differs'):
                verify.verify(FOLDER)

    def test_strip_does_not_accept_unknown_phase_or_substitution_arguments(self):
        for text in ("leaves.begin('owner', 'unknown')", "leaf_rows(raw, accounted, other)"):
            with self.assertRaises(ValueError):
                verify.Strip().visit(ast.parse(text))


if __name__ == '__main__':
    unittest.main()
