"""Data-only frozen v3 scope and negative admission controls; no dispatch."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_effects_review_v3 as gate

ROOT=Path(__file__).resolve().parents[2]
PACKET=ROOT/'docs/testing/performance/0.74/local-runs/package-effect-syntax-v3-20261025'


class RepairedScope(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw=(PACKET/'index.json').read_bytes()
        assert gate.sha(raw)=='1e461457bb2448b9caabfe396b0e9999ec8f34e28522036dd4c7dd4296bac0aa'
        cls.index=gate.json.loads(raw)
        cls.directory=tempfile.TemporaryDirectory();cls.addClassCleanup(cls.directory.cleanup)
        cls.root=Path(cls.directory.name).resolve()
        def member(name):
            row=cls.index['members'][name];body=(PACKET/'objects'/row['sha256']).read_bytes()
            assert gate.sha(body)==row['sha256'] and len(body)==row['bytes'];return body
        for name in cls.index['members']:
            if not name.startswith('fixture/'):continue
            target=cls.root/name.removeprefix('fixture/');target.parent.mkdir(parents=True,exist_ok=True)
            target.write_bytes(member(name))
        cls.document=gate.json.loads(member('syntax/result.json'))
        assert gate.previous.legacy.document_digest(cls.document)==gate.REVIEWED_AST_SHA256
        cls.targets=['unittests src/lib.rs','unittests src/main.rs']+[
            'tests/'+p.name for p in sorted((cls.root/'crates/xtask/tests').glob('*.rs'))]
        assert len(cls.targets)==137

    def review(self, document=None):
        # This unit fixture substitutes the independently audited target catalog;
        # it does not load executable Python from the frozen fixture tree.
        with patch.object(gate.previous.legacy.package_replay_audit,'audit',return_value={'targets':self.targets}):
            return gate.source_review(self.root,self.document if document is None else document)

    def test_exact_frozen_scope_does_not_admit_a_package_or_live_execution(self):
        with patch.object(gate.previous.legacy.subprocess,'Popen',side_effect=AssertionError('no child')):
            result=self.review()
        self.assertEqual((len(result['sources']),len(result['auxiliary']),len(result['targets'])),(343,29,137))
        self.assertEqual(result['changed_original_sources'],[])
        self.assertEqual(len(result['added_sources']),9)
        self.assertTrue(result['source_effect_review_complete'])
        for name in ('formal_whole_program_proof','linux_or_all_features_cleared','package_dispatched',
                     'package_pass','original_execution_or_product_or_qualification_admission'):
            self.assertFalse(result[name])

    def test_v1_and_v2_do_not_inherit_this_scope(self):
        for reviewer in (gate.previous.legacy,gate.previous):
            with self.assertRaisesRegex(ValueError,'unreviewed'):
                reviewer.source_review(self.root,self.document)

    def test_document_and_scope_cannot_self_refresh(self):
        bad=copy.deepcopy(self.document);bad['compiler_expansion']=True
        with self.assertRaisesRegex(ValueError,'unreviewed v3'):self.review(bad)
        with patch.object(gate,'REVIEWED_AST_SHA256',gate.previous.legacy.document_digest(bad)):
            with self.assertRaisesRegex(ValueError,'source-only'):self.review(bad)

    def test_missing_duplicate_and_unclosed_sources_refuse(self):
        for mode in ('missing','duplicate','unclosed'):
            bad=copy.deepcopy(self.document)
            if mode=='missing':bad['sources'].pop()
            elif mode=='duplicate':bad['sources'].append(bad['sources'][0])
            else:bad['sources'][0]['sites'].append(dict(kind='module',scope='absent',detail='default-module-path'))
            with patch.object(gate,'REVIEWED_AST_SHA256',gate.previous.legacy.document_digest(bad)):
                with self.assertRaises(ValueError):self.review(bad)

    def test_host_constructor_and_unexpanded_include_refuse(self):
        for kind,detail in [('command-constructor','literal:ssh'),('macro-command-constructor','literal:systemctl'),('include','other.rs')]:
            bad=copy.deepcopy(self.document)
            bad['sources'][0]['sites'].append(dict(kind=kind,scope='fixture',detail=detail))
            with patch.object(gate,'REVIEWED_AST_SHA256',gate.previous.legacy.document_digest(bad)):
                with self.assertRaisesRegex(ValueError,'host/service|unexpanded include'):self.review(bad)

    def test_source_and_verifier_dependency_bytes_refuse_drift(self):
        original=Path.read_bytes
        for path in (self.root/'crates/xtask/tests/package_diagnostics_074.rs',
                     self.root/'tools/historical-blob-batch-074/package_effects_review_v2.py'):
            with patch.object(Path,'read_bytes',lambda current: b'changed' if current==path else original(current)):
                with self.assertRaisesRegex(ValueError,'source bytes/path drift|auxiliary input drift'):self.review()

    def test_original_sites_cannot_be_reclassified_even_with_same_raw_hash(self):
        bad=copy.deepcopy(self.document)
        row=next(v for v in bad['sources'] if v['source']=='tools/authority-runtime-074/gate.rs')
        row['sites'].append(dict(kind='function-call',scope='fixture',detail='unreviewed'))
        with patch.object(gate,'REVIEWED_AST_SHA256',gate.previous.legacy.document_digest(bad)):
            with self.assertRaisesRegex(ValueError,'original source/AST drift'):self.review(bad)

    def test_manual_additions_auxiliary_and_catalog_are_independent(self):
        with patch.object(gate,'ADDED',{}):
            with self.assertRaisesRegex(ValueError,'manual added-source'):self.review()
        with patch.object(gate,'AUXILIARY',{}):
            with self.assertRaisesRegex(ValueError,'auxiliary input'):self.review()
        for values in (self.targets[:-1],self.targets+[self.targets[0]]):
            with patch.object(self,'targets',values):
                with self.assertRaisesRegex(ValueError,'target catalog'):self.review()

    def test_independent_old_packet_identity_cannot_change(self):
        with patch.object(gate.previous,'PREVIOUS_INDEX_SHA256','0'*64):
            with self.assertRaisesRegex(ValueError,'old packet identity'):self.review()
