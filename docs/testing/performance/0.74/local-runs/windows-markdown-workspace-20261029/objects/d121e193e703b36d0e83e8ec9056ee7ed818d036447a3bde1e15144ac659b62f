"""Pinned frozen fixture, mutation refusals and import-before-check controls."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_effects_review_v8 as gate

ROOT = Path(__file__).resolve().parents[2]
PACKET = 'docs/testing/performance/0.74/local-runs/package-review-v8-20261027'
INDEX = 'ccb759b4905dc9685485937507e346748c7ed77ae50ed6a24512666393f4a47d'


class ReviewV8(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name).resolve()
        folder = ROOT / PACKET
        raw = (folder / 'index.json').read_bytes()
        assert gate.sha(raw) == INDEX
        for name, row in json.loads(raw)['members'].items():
            body = (folder / 'objects' / row['sha256']).read_bytes()
            assert len(body) == row['bytes'] and gate.sha(body) == row['sha256']
            if name == 'syntax/result.json':
                cls.document = gate.decode(body)
            if name.startswith('fixture/'):
                path = cls.root / name[8:]
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(body)

    def test_exact_new_scope_and_no_execution_authority(self):
        result = gate.source_review(self.root, self.document)
        self.assertEqual((len(result['sources']), len(result['auxiliary']), len(result['targets'])), (374, 54, 148))
        self.assertEqual((len(result['changed_sources']), len(result['added_sources'])), (2, 2))
        self.assertTrue(result['source_effect_review_complete'])
        for name in ('package_dispatched', 'package_pass', 'formal_whole_program_proof',
                     'linux_or_all_features_cleared', 'original_execution_or_product_or_qualification_admission'):
            self.assertFalse(result[name])

    def test_all_source_body_classes_are_pinned(self):
        original = Path.read_bytes
        for name in (*gate.CHANGED, *gate.ADDED, 'crates/xtask/tests/census_cost_074.rs'):
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: b'changed' if p == self.root / name else original(p)):
                with self.assertRaisesRegex(ValueError, 'source bytes'):
                    gate.source_review(self.root, self.document)

    def test_ast_cannot_learn_pin_or_change_an_unchanged_row(self):
        bad = copy.deepcopy(self.document)
        row = next(r for r in bad['sources'] if r['source'] == 'crates/xtask/tests/census_cost_074.rs')
        row['sites'].append(dict(kind='function-call', scope='fixture', detail='unknown'))
        with self.assertRaisesRegex(ValueError, 'unreviewed v8'):
            gate.source_review(self.root, bad)
        with patch.object(gate, 'REVIEWED_AST_SHA256', gate.document_digest(bad)):
            with self.assertRaisesRegex(ValueError, 'source/AST delta'):
                gate.source_review(self.root, bad)

    def test_missing_duplicate_and_execution_claim_refuse(self):
        for mode in ('missing', 'duplicate', 'executed'):
            bad = copy.deepcopy(self.document)
            if mode == 'missing':
                bad['sources'].pop()
            elif mode == 'duplicate':
                bad['sources'].append(bad['sources'][0])
            else:
                bad['executed_input_code'] = True
            with patch.object(gate, 'REVIEWED_AST_SHA256', gate.document_digest(bad)):
                with self.assertRaises(ValueError):
                    gate.source_review(self.root, bad)

    def test_every_auxiliary_mutation_refuses_before_import(self):
        names = gate.checked_auxiliary(self.root)
        original = Path.read_bytes
        for name in names:
            with self.subTest(name=name), patch.object(Path, 'read_bytes',
                    lambda p: b'changed' if p == self.root / name else original(p)), \
                    patch.object(gate, 'load_context') as imported:
                with self.assertRaisesRegex(ValueError, 'auxiliary input drift'):
                    gate.source_review(self.root, self.document)
                imported.assert_not_called()

    def test_old_receipt_and_syntax_cannot_be_replaced(self):
        original = Path.read_bytes
        for name in (gate.PREVIOUS_SYNTAX, gate.PREVIOUS_PACKET + '/index.json',
                     gate.PREVIOUS_PACKET + '/objects/' + gate.PREVIOUS_GATE_SHA256):
            with patch.object(Path, 'read_bytes', lambda p: b'changed' if p == self.root / name else original(p)):
                with self.assertRaisesRegex(ValueError, 'V7'):
                    gate.source_review(self.root, self.document)

    def test_missing_context_and_catalog_drift_refuse(self):
        from types import SimpleNamespace
        for context in (({'unknown.rs': 'a' * 64}, []), ({}, ['unknown'])):
            with patch.object(gate, 'load_context', return_value=SimpleNamespace(module_edges=lambda *a: context)):
                with self.assertRaisesRegex(ValueError, 'module graph'):
                    gate.source_review(self.root, self.document)
        catalog = gate.target_catalog(self.root)
        for bad in (catalog[:-1], catalog + ['unknown'], catalog + [catalog[0]], list(reversed(catalog))):
            with patch.object(gate, 'target_catalog', return_value=bad):
                with self.assertRaisesRegex(ValueError, 'target catalog'):
                    gate.source_review(self.root, self.document)

    def test_new_unreferenced_source_also_refuses(self):
        path = self.root / 'crates/xtask/tests/unreviewed.rs'
        try:
            path.write_bytes(b'// uncensused')
            with self.assertRaisesRegex(ValueError, 'uncensused'):
                gate.source_review(self.root, self.document)
        finally:
            path.unlink()

    def test_duplicate_json_and_escape_refuse(self):
        with self.assertRaisesRegex(ValueError, 'duplicate JSON'):
            gate.decode(b'{"x":1,"x":2}')
        for name in ('../outside', str(self.root / 'absolute')):
            with self.assertRaisesRegex(ValueError, 'nonrelative'):
                gate.read_inside(self.root, name)


if __name__ == '__main__':
    unittest.main()
