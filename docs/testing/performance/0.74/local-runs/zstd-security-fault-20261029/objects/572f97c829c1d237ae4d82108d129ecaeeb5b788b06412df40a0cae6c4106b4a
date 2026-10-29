import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import analysis as a
import gnu_review as g


class GnuReviewControls(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        _, cls.gate, cls.document, cls.texts = a.inputs()

    def test_full_data_scope_does_not_dispatch_or_admit(self):
        result = g.source_review(self.gate, self.document, self.texts, {}, [])
        self.assertEqual(len(result['targets']), 153)
        self.assertEqual(len(result['manual_groups']), 8)
        self.assertEqual((result['package_bound_seconds'], result['test_threads']), (780, 4))
        for key in ('tests_dispatched', 'package_pass', 'release_admitted', 'compiler_expansion',
                    'windows_v9_reinterpreted', 'formal_whole_program_proof'):
            self.assertFalse(result[key])

    def test_unchanged_complete_command_without_filters(self):
        argv = g.command()
        self.assertEqual(argv[-2:], ['--', '--test-threads=4'])
        self.assertIn('--lib', argv)
        self.assertIn('--tests', argv)
        for bad in ('--test', '--ignored', '--skip', '--exact', '--all-features'):
            self.assertNotIn(bad, argv)

    def test_body_graph_census_and_constructor_drift_refuse(self):
        for mode in ('body', 'graph', 'unresolved', 'catalog', 'constructor'):
            gate, texts = copy.deepcopy(self.gate), self.texts.copy()
            additions, unresolved = {}, []
            if mode == 'body': texts[next(iter(texts))] = b'changed'
            if mode == 'graph': additions['new.rs'] = '0' * 64
            if mode == 'unresolved': unresolved.append('unknown-edge')
            if mode == 'catalog': gate['targets'].pop()
            if mode == 'constructor': gate['constructors'].append({'detail': 'literal:systemctl'})
            with self.subTest(mode=mode), self.assertRaises(ValueError):
                g.source_review(gate, self.document, texts, additions, unresolved)

    def test_helper_hash_precedes_import(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'helper.py'
            path.write_bytes(b'raise AssertionError("not approved")')
            with patch.object(g.importlib.util, 'spec_from_file_location') as imported:
                with self.assertRaisesRegex(ValueError, 'helper drift'):
                    g.checked_module(path, 'refuse', '0' * 64)
                imported.assert_not_called()


if __name__ == '__main__':
    unittest.main()
