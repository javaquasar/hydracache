"""Frozen data-only v4 scope; mutations cannot update admission pins."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_effects_review_v4 as gate

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v4-closure-20261025'


class RetirementScope(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw = (PACKET / 'index.json').read_bytes()
        assert gate.sha(raw) == 'e46b334397fcb7ef6ffe36335fbe86a908be481dc12832109bf1da99b93ea6fb'
        index = json.loads(raw)
        cls.directory = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.directory.cleanup)
        cls.root = Path(cls.directory.name).resolve()
        for name, row in index['members'].items():
            body = (PACKET / 'objects' / row['sha256']).read_bytes()
            assert gate.sha(body) == row['sha256'] and len(body) == row['bytes']
            if name == 'syntax/result.json':
                cls.document = json.loads(body)
            if name.startswith('fixture/'):
                target = cls.root / name.removeprefix('fixture/')
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(body)
        cls.targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + [
            'tests/' + p.name for p in sorted((cls.root / 'crates/xtask/tests').glob('*.rs'))]

    def review(self, document=None):
        with patch.object(gate.previous.previous.legacy.package_replay_audit, 'audit',
                          return_value={'targets': self.targets}):
            return gate.source_review(self.root, self.document if document is None else document)

    def test_frozen_scope_keeps_full_targets_and_no_execution_admission(self):
        result = self.review()
        self.assertEqual((len(result['sources']), len(result['auxiliary']), len(result['targets'])), (344, 29, 138))
        self.assertTrue(result['source_effect_review_complete'])
        for key in ('package_dispatched', 'package_pass', 'formal_whole_program_proof',
                    'linux_or_all_features_cleared', 'original_execution_or_product_or_qualification_admission'):
            self.assertFalse(result[key])

    def test_previous_review_still_refuses_current_graph(self):
        with self.assertRaisesRegex(ValueError, 'unreviewed v3'):
            gate.previous.source_review(self.root, self.document)

    def test_document_cannot_approve_itself(self):
        bad = copy.deepcopy(self.document)
        bad['compiler_expansion'] = True
        with self.assertRaisesRegex(ValueError, 'unreviewed v4'):
            self.review(bad)
        with patch.object(gate, 'REVIEWED_AST_SHA256', gate.previous.previous.legacy.document_digest(bad)):
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
            with patch.object(gate, 'REVIEWED_AST_SHA256', gate.previous.previous.legacy.document_digest(bad)):
                with self.assertRaises(ValueError):
                    self.review(bad)

    def test_host_effect_and_include_refuse_even_with_different_document_pin(self):
        for kind, detail in (('command-constructor', 'literal:ssh'),
                             ('macro-command-constructor', 'literal:systemctl'), ('include', 'other.rs')):
            bad = copy.deepcopy(self.document)
            bad['sources'][0]['sites'].append(dict(kind=kind, scope='fixture', detail=detail))
            with patch.object(gate, 'REVIEWED_AST_SHA256', gate.previous.previous.legacy.document_digest(bad)):
                with self.assertRaisesRegex(ValueError, 'host/service|unexpanded include'):
                    self.review(bad)

    def test_unchanged_source_ast_cannot_be_reclassified(self):
        bad = copy.deepcopy(self.document)
        row = next(row for row in bad['sources'] if row['source'] == 'tools/authority-runtime-074/gate.rs')
        row['sites'].append(dict(kind='function-call', scope='fixture', detail='unreviewed'))
        with patch.object(gate, 'REVIEWED_AST_SHA256', gate.previous.previous.legacy.document_digest(bad)):
            with self.assertRaisesRegex(ValueError, 'source/AST delta'):
                self.review(bad)

    def test_raw_source_auxiliary_and_historical_packet_drift_refuse(self):
        original = Path.read_bytes
        for name in ('crates/xtask/tests/get_owner_terminal_074.rs', '.cargo/config.toml',
                     gate.PREVIOUS_PACKET + '/index.json'):
            target = self.root / name
            with patch.object(Path, 'read_bytes', lambda path: b'changed' if path == target else original(path)):
                with self.assertRaises(ValueError):
                    self.review()

    def test_manual_delta_and_catalog_cannot_be_broadened(self):
        with patch.object(gate, 'DELTA', {}):
            with self.assertRaisesRegex(ValueError, 'manual delta'):
                self.review()
        for targets in (self.targets[:-1], self.targets + [self.targets[0]]):
            with patch.object(self, 'targets', targets):
                with self.assertRaisesRegex(ValueError, 'target catalog'):
                    self.review()
