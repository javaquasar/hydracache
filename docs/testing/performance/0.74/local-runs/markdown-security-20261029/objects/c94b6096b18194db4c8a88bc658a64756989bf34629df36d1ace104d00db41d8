"""Fixed data-only v5 admission controls; never dispatches the package."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_effects_review_v5 as gate

ROOT = Path(__file__).resolve().parents[2]
CURRENT_PACKET = 'docs/testing/performance/0.74/local-runs/module-context-v5-20261026'


class ReaderRepairScope(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.temp.cleanup)
        cls.root = Path(cls.temp.name).resolve()
        for folder, pin in ((gate.PREVIOUS_PACKET, gate.PREVIOUS_INDEX_SHA256),
                            (CURRENT_PACKET, 'be8eb6bc81a7ddfb2dc118827f2946ebab0850e9d6aff1171657898e9a795dc0')):
            packet = ROOT / folder
            raw = (packet / 'index.json').read_bytes()
            assert gate.sha(raw) == pin
            index = json.loads(raw)
            for name, row in index['members'].items():
                body = (packet / 'objects' / row['sha256']).read_bytes()
                assert gate.sha(body) == row['sha256'] and len(body) == row['bytes']
                if folder == CURRENT_PACKET and name == 'syntax/contextual-closure/result.json':
                    cls.document = json.loads(body)
                if name.startswith(('fixture/', 'delta-source/')):
                    path = cls.root / name.split('/', 1)[1]
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(body)
            if folder == gate.PREVIOUS_PACKET:
                target = cls.root / folder
                (target / 'objects').mkdir(parents=True, exist_ok=True)
                (target / 'index.json').write_bytes(raw)
                row = index['members']['syntax/result.json']
                (target / 'objects' / row['sha256']).write_bytes((packet / 'objects' / row['sha256']).read_bytes())
        for name, digest in gate.EXTRA_AUXILIARY.items():
            body = (ROOT / name).read_bytes()
            assert gate.sha(body) == digest
            path = cls.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        cls.targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + [
            'tests/' + path.name for path in sorted((cls.root / 'crates/xtask/tests').glob('*.rs'))]

    def review(self, document=None):
        with patch.object(gate.legacy.package_replay_audit, 'audit', return_value={'targets': self.targets}):
            return gate.source_review(self.root, self.document if document is None else document)

    def test_exact_scope_keeps_365_sources_42_auxiliary_144_targets_and_no_production_admission(self):
        result = self.review()
        self.assertEqual((len(result['sources']), len(result['auxiliary']), len(result['targets'])), (365, 42, 144))
        self.assertEqual((len(result['changed_sources']), len(result['added_sources'])), (21, 21))
        self.assertTrue(result['source_effect_review_complete'])
        for name in ('package_dispatched', 'package_pass', 'formal_whole_program_proof',
                     'linux_or_all_features_cleared', 'original_execution_or_product_or_qualification_admission'):
            self.assertFalse(result[name])

    def test_v4_still_refuses_current_document(self):
        with self.assertRaisesRegex(ValueError, 'unreviewed v4'):
            gate.previous.source_review(self.root, self.document)

    def test_document_cannot_self_approve_or_enable_execution(self):
        bad = copy.deepcopy(self.document)
        bad['executed_input_code'] = True
        with self.assertRaisesRegex(ValueError, 'unreviewed v5'):
            self.review(bad)
        with patch.object(gate, 'REVIEWED_AST_SHA256', gate.legacy.document_digest(bad)):
            with self.assertRaisesRegex(ValueError, 'source-only'):
                self.review(bad)

    def test_missing_duplicate_and_unclosed_rows_refuse(self):
        for mode in ('missing', 'duplicate', 'unclosed'):
            bad = copy.deepcopy(self.document)
            if mode == 'missing':
                bad['sources'].pop()
            elif mode == 'duplicate':
                bad['sources'].append(bad['sources'][0])
            else:
                bad['sources'][0]['sites'].append(dict(kind='module', scope='absent', detail='default-module-path'))
            with patch.object(gate, 'REVIEWED_AST_SHA256', gate.legacy.document_digest(bad)):
                with self.assertRaises(ValueError):
                    self.review(bad)

    def test_host_constructor_and_include_refuse_even_under_different_document_pin(self):
        for kind, detail in (('command-constructor', 'literal:ssh'),
                             ('macro-command-constructor', 'literal:systemctl'), ('include', 'unreviewed.rs')):
            bad = copy.deepcopy(self.document)
            bad['sources'][0]['sites'].append(dict(kind=kind, scope='fixture', detail=detail))
            with patch.object(gate, 'REVIEWED_AST_SHA256', gate.legacy.document_digest(bad)):
                with self.assertRaisesRegex(ValueError, 'host/service|unexpanded include'):
                    self.review(bad)

    def test_unchanged_ast_reclassification_and_manual_delta_drift_refuse(self):
        bad = copy.deepcopy(self.document)
        row = next(row for row in bad['sources'] if row['source'] == 'crates/xtask/tests/census_cost_074.rs')
        row['sites'].append(dict(kind='function-call', scope='fixture', detail='unreviewed'))
        with patch.object(gate, 'REVIEWED_AST_SHA256', gate.legacy.document_digest(bad)):
            with self.assertRaisesRegex(ValueError, 'source/AST delta'):
                self.review(bad)
        with patch.object(gate, 'DELTA', {}):
            with self.assertRaisesRegex(ValueError, 'manual delta'):
                self.review()

    def test_raw_source_auxiliary_and_old_packet_cannot_change(self):
        original = Path.read_bytes
        for name in ('tools/remaining-readers-074/inputs.rs', 'tools/current-frontier-074/rules.json',
                     'tools/historical-blob-batch-074/module_context_v5.py', '.cargo/config.toml',
                     gate.PREVIOUS_PACKET + '/index.json'):
            target = self.root / name
            with patch.object(Path, 'read_bytes', lambda path: b'changed' if path == target else original(path)):
                with self.assertRaises(ValueError):
                    self.review()

    def test_catalog_cannot_drop_add_reorder_or_duplicate_targets(self):
        for targets in (self.targets[:-1], self.targets + ['tests/unknown.rs'],
                        list(reversed(self.targets)), self.targets + [self.targets[0]]):
            with patch.object(self, 'targets', targets):
                with self.assertRaisesRegex(ValueError, 'target catalog'):
                    self.review()


if __name__ == '__main__':
    unittest.main()
