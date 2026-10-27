"""Pinned source data controls; never import or run archived project bodies."""
import copy
import json
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import package_effects_review_v9 as gate

ROOT = Path(__file__).resolve().parents[2]
PACKET = 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v9-20261027'
INDEX_SHA256 = 'e2c54681e571204fc927d7df3fe99a08303474c52af7a945d1a2764bb601bfc9'


class ReviewV9(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name).resolve()
        folder = ROOT / PACKET
        raw = (folder / 'index.json').read_bytes()
        assert gate.sha(raw) == INDEX_SHA256
        index = gate.decode(raw)
        assert not index['package_pass'] and not index['archived_code_executed']
        for name, row in index['members'].items():
            body = (folder / 'objects' / row['sha256']).read_bytes()
            assert len(body) == row['bytes'] and gate.sha(body) == row['sha256']
            if name == 'capture/v9-syntax-first/result.json':
                cls.document = gate.decode(body)
            if name.startswith('fixture/'):
                path = cls.root / name[8:]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(body)

    def checked_review(self, document=None):
        # Actual current graph closure has a separate captured review. The unit
        # fixture substitutes this callback instead of executing archived Python.
        with patch.object(gate, 'load_context', return_value=SimpleNamespace(module_edges=lambda *a: ({}, []))):
            return gate.source_review(self.root, self.document if document is None else document)

    def test_exact_data_scope_retains_all_nonadmission_fields(self):
        result = self.checked_review()
        self.assertEqual((len(result['sources']), len(result['auxiliary']), len(result['targets'])), (386, 56, 153))
        self.assertEqual((len(result['changed_sources']), len(result['added_sources'])), (17, 12))
        self.assertTrue(result['source_effect_review_complete'])
        for name in ('package_dispatched', 'package_pass', 'formal_whole_program_proof',
                     'linux_or_all_features_cleared', 'original_execution_or_product_or_qualification_admission'):
            self.assertFalse(result[name])
        self.assertEqual(gate.NEW_TARGETS, [name for name in result['targets'] if name not in gate.previous(self.root)[0]['targets']])

    def test_current_unchanged_changed_and_added_source_bytes_are_all_pinned(self):
        original = gate.read_inside
        for name in (*gate.CHANGED, *gate.ADDED, 'crates/xtask/tests/census_cost_074.rs'):
            with self.subTest(name=name), patch.object(gate, 'read_inside',
                    lambda root, path: b'drift' if path == name else original(root, path)):
                with self.assertRaisesRegex(ValueError, 'source bytes'):
                    self.checked_review()

    def test_ast_cannot_learn_approval_from_a_new_observation(self):
        bad = copy.deepcopy(self.document)
        row = next(r for r in bad['sources'] if r['source'] == 'crates/xtask/tests/census_cost_074.rs')
        row['sites'].append(dict(kind='function-call', scope='control', detail='unreviewed'))
        with self.assertRaisesRegex(ValueError, 'unreviewed v9'):
            self.checked_review(bad)
        with patch.object(gate, 'REVIEWED_AST_SHA256', gate.document_digest(bad)):
            with self.assertRaisesRegex(ValueError, 'changed source/AST delta'):
                self.checked_review(bad)

    def test_missing_duplicate_removed_and_executed_inventory_refuse(self):
        for mode in ('missing', 'duplicate', 'remove-old', 'executed', 'expansion'):
            bad = copy.deepcopy(self.document)
            if mode == 'missing':
                bad['sources'].pop()
            elif mode == 'duplicate':
                bad['sources'].append(bad['sources'][0])
            elif mode == 'remove-old':
                bad['sources'] = [r for r in bad['sources'] if r['source'] != 'crates/xtask/tests/census_cost_074.rs']
                bad['sources'].append(dict(source='unknown.rs', sha256='0' * 64, sites=[]))
            elif mode == 'executed':
                bad['executed_input_code'] = True
            else:
                bad['compiler_expansion'] = True
            with self.subTest(mode=mode), patch.object(gate, 'REVIEWED_AST_SHA256', gate.document_digest(bad)):
                with self.assertRaises(ValueError):
                    self.checked_review(bad)

    def test_all_auxiliary_changes_refuse_before_context_or_environment_import(self):
        names = gate.checked_auxiliary(self.root)
        original = gate.read_inside
        for name in names:
            with self.subTest(name=name), patch.object(gate, 'read_inside',
                    lambda root, path: b'drift' if path == name else original(root, path)), \
                    patch.object(gate, 'load_context') as context, \
                    patch.object(gate.importlib.util, 'spec_from_file_location') as imported:
                with self.assertRaisesRegex(ValueError, 'auxiliary input drift'):
                    gate.source_review(self.root, self.document)
                with self.assertRaisesRegex(ValueError, 'auxiliary input drift'):
                    gate.environment_preflight(self.root)
                context.assert_not_called()
                imported.assert_not_called()

    def test_old_receipt_gate_and_syntax_remain_immutable_data(self):
        original = gate.read_inside
        for name in (gate.PREVIOUS_PACKET + '/index.json',
                     gate.PREVIOUS_PACKET + '/objects/' + gate.PREVIOUS_GATE_SHA256,
                     gate.PREVIOUS_PACKET + '/objects/' + gate.PREVIOUS_SYNTAX_SHA256):
            with self.subTest(name=name), patch.object(gate, 'read_inside',
                    lambda root, path: b'drift' if path == name else original(root, path)):
                with self.assertRaisesRegex(ValueError, 'V8'):
                    self.checked_review()

    def test_missing_context_or_additional_edges_refuse(self):
        for edges in (({'foreign.rs': '0' * 64}, []), ({}, ['unresolved'])):
            with patch.object(gate, 'load_context', return_value=SimpleNamespace(module_edges=lambda *a: edges)):
                with self.assertRaisesRegex(ValueError, 'module graph'):
                    gate.source_review(self.root, self.document)

    def test_target_removal_duplication_reordering_and_addition_refuse(self):
        good = gate.target_catalog(self.root)
        for bad in (good[:-1], good + ['tests/unknown.rs'], good + [good[0]], list(reversed(good))):
            with patch.object(gate, 'target_catalog', return_value=bad):
                with self.assertRaisesRegex(ValueError, 'target catalog'):
                    self.checked_review()

    def test_unreferenced_rust_source_is_not_hidden_by_module_closure(self):
        path = self.root / 'crates/xtask/tests/unreviewed.rs'
        try:
            path.write_bytes(b'// owned unreferenced fixture')
            with self.assertRaisesRegex(ValueError, 'uncensused'):
                self.checked_review()
        finally:
            path.unlink()

    def test_environment_review_body_cannot_import_before_its_pin(self):
        original = Path.read_bytes
        path = self.root / 'tools/historical-blob-batch-074/package_effects_review_v8.py'
        with patch.object(Path, 'read_bytes', lambda p: b'drift' if p == path else original(p)), \
                patch.object(gate.importlib.util, 'spec_from_file_location') as imported:
            with self.assertRaisesRegex(ValueError, 'environment review body drift'):
                gate.environment_preflight(self.root)
            imported.assert_not_called()

    def test_context_body_pin_precedes_import_and_paths_cannot_escape(self):
        original = Path.read_bytes
        path = self.root / 'tools/historical-blob-batch-074/module_context_v5.py'
        with patch.object(Path, 'read_bytes', lambda p: b'drift' if p == path else original(p)), \
                patch.object(gate.importlib.util, 'spec_from_file_location') as imported:
            with self.assertRaisesRegex(ValueError, 'context body drift'):
                gate.load_context(self.root)
            imported.assert_not_called()
        for name in ('../outside', str(self.root / 'absolute')):
            with self.assertRaisesRegex(ValueError, 'nonrelative'):
                gate.read_inside(self.root, name)
        with self.assertRaisesRegex(ValueError, 'duplicate JSON'):
            gate.decode(b'{"x":1,"x":2}')


if __name__ == '__main__':
    unittest.main()
