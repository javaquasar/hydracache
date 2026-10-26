"""Separate v2 admission controls; no package/product dispatch or live fixtures."""
import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import package_effects_review_v2 as review

ROOT = Path(__file__).resolve().parents[2]
DOCUMENT = ROOT/'docs/testing/performance/0.74/local-runs/package-effect-syntax-v2-20261025/objects/07d9960365b4976ba7f0a618799b222fad89c3d6e5dffa9a29436186c802e94d'


class ReviewV2(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        raw = DOCUMENT.read_bytes()
        assert review.sha(raw) == DOCUMENT.name
        cls.document = review.json.loads(raw)
        # Positive tests exercise the exact retained v2 scope, not whichever
        # source happens to be installed today. This is a data-only fixture;
        # no retained source is compiled or dispatched.
        folder = ROOT/'docs/testing/performance/0.74/local-runs/package-effect-review-v2-20261025'
        index_raw = (folder/'index.json').read_bytes()
        assert review.sha(index_raw) == '58b7a5f845abd8e791b5d150f98f461fc069869195131fe3e76ea620df772dae'
        members = review.json.loads(index_raw)['members']
        cls.directory = tempfile.TemporaryDirectory()
        cls.addClassCleanup(cls.directory.cleanup)
        cls.root = Path(cls.directory.name).resolve()
        def materialize(name, body):
            path = cls.root/name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(body)
        for prefix, pins in (
                ('reviewed-source/', {v['source']:v['sha256'] for v in cls.document['sources']}),
                ('reviewed-auxiliary/', {**review.legacy.REVIEWED_AUXILIARY, **review.EXTRA_AUXILIARY})):
            for name, digest in pins.items():
                entry = members[prefix+name]
                body = (folder/'objects'/entry['sha256']).read_bytes()
                assert review.sha(body) == digest == entry['sha256'] and len(body) == entry['bytes']
                materialize(name, body)
        previous = ROOT/review.PREVIOUS_PACKET
        index_raw = (previous/'index.json').read_bytes()
        assert review.sha(index_raw) == review.PREVIOUS_INDEX_SHA256
        entry = review.json.loads(index_raw)['members']['syntax/final-closure/result.json']
        body = (previous/'objects'/entry['sha256']).read_bytes()
        assert review.sha(body) == entry['sha256'] and len(body) == entry['bytes']
        materialize(review.PREVIOUS_PACKET+'/index.json', index_raw)
        materialize(review.PREVIOUS_PACKET+'/objects/'+entry['sha256'], body)
        cls.targets = ['unittests src/lib.rs', 'unittests src/main.rs'] + [
            'tests/'+p.name for p in sorted((cls.root/'crates/xtask/tests').glob('*.rs'))]
        assert len(cls.targets) == 136

    def frozen_review(self, document):
        # The separate catalog/old-log audit has its own tests. Avoid loading
        # executable Python from the source fixture in this unit-level join.
        with patch.object(review.legacy.package_replay_audit, 'audit', return_value={'targets':self.targets}):
            return review.source_review(self.root, document)

    def test_exact_current_scope_and_old_receipt_refusal(self):
        with patch.object(review.legacy.subprocess, 'Popen', side_effect=AssertionError('no child')):
            value = self.frozen_review(self.document)
        self.assertEqual(len(value['sources']), 342)
        self.assertEqual(len(value['targets']), 136)
        self.assertEqual(len(value['auxiliary']), 28)
        self.assertEqual(len(value['added_sources']), 8)
        self.assertEqual(len(value['changed_sources']), 4)
        self.assertTrue(value['source_effect_review_complete'])
        self.assertFalse(value['package_pass'])
        self.assertFalse(value['linux_or_all_features_cleared'])
        self.assertFalse(value['original_execution_or_product_or_qualification_admission'])
        with self.assertRaisesRegex(ValueError, 'unreviewed syntax/source inventory'):
            review.legacy.source_review(ROOT, self.document)
        self.assertEqual(review.legacy.REVIEWED_AST_SHA256, review.PREVIOUS_AST_SHA256)

    def test_repaired_current_tree_is_not_admitted_by_old_v2_scope(self):
        with patch.object(review.legacy.subprocess, 'Popen', side_effect=AssertionError('no child')):
            with self.assertRaisesRegex(ValueError, 'source bytes/path drift'):
                review.source_review(ROOT, self.document)

    def test_unknown_digest_and_scope_cannot_self_refresh(self):
        bad = copy.deepcopy(self.document)
        bad['compiler_expansion'] = True
        with self.assertRaisesRegex(ValueError, 'unreviewed v2'):
            self.frozen_review(bad)
        with patch.object(review, 'REVIEWED_AST_SHA256', review.legacy.document_digest(bad)):
            with self.assertRaisesRegex(ValueError, 'source-only inventory scope'):
                self.frozen_review(bad)

    def test_missing_duplicate_and_unclosed_inventory_refuse(self):
        for mode in ('missing', 'duplicate', 'unclosed'):
            bad = copy.deepcopy(self.document)
            if mode == 'missing': bad['sources'].pop()
            elif mode == 'duplicate': bad['sources'].append(bad['sources'][0])
            else:
                bad['sources'][0]['sites'].append(dict(kind='module', scope='unreviewed_absent', detail='default-module-path'))
            with patch.object(review, 'REVIEWED_AST_SHA256', review.legacy.document_digest(bad)):
                with self.assertRaises(ValueError): self.frozen_review(bad)

    def test_service_constructor_and_unexpanded_include_are_still_forbidden(self):
        for kind, detail in (('command-constructor', 'literal:ssh'), ('macro-command-constructor', 'literal:systemctl'),
                             ('include', 'unexpanded.rs')):
            bad = copy.deepcopy(self.document)
            bad['sources'][0]['sites'].append(dict(kind=kind, scope='fixture', detail=detail))
            with patch.object(review, 'REVIEWED_AST_SHA256', review.legacy.document_digest(bad)):
                with self.assertRaisesRegex(ValueError, 'host/service constructor|unexpanded include'):
                    self.frozen_review(bad)

    def test_actual_source_drift_and_new_data_drift_refuse(self):
        original = Path.read_bytes
        for path in (self.root/'crates/xtask/tests/direct_output_cost_074.rs',
                     self.root/'tools/xtask-package-attribution-074/pipe-execution-protocol.toml'):
            def read(current):
                return b'unreviewed drift' if current == path else original(current)
            with patch.object(Path, 'read_bytes', read):
                with self.assertRaisesRegex(ValueError, 'source bytes/path drift|auxiliary input drift'):
                    self.frozen_review(self.document)

    def test_manual_delta_and_legacy_auxiliary_pins_are_independent(self):
        for name, value in (('REVIEWED_DELTA', {}), ('EXTRA_AUXILIARY', {})):
            with patch.object(review, name, value):
                with self.assertRaisesRegex(ValueError, 'manual delta pins drift|auxiliary input drift'):
                    self.frozen_review(self.document)

    def test_old_packet_pin_drift_refuses(self):
        with patch.object(review, 'PREVIOUS_INDEX_SHA256', '0'*64):
            with self.assertRaisesRegex(ValueError, 'old packet identity'):
                self.frozen_review(self.document)

    def test_negative_program_presence_in_application_directory_refuses(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(review.shutil, 'which', return_value=None):
            root = Path(directory)
            self.assertTrue(review.negative_program_preflight(root)['negative_fixture_executable_absent'])
            folder = root/'target/debug/deps'; folder.mkdir(parents=True)
            (folder/review.NEGATIVE_PROGRAM).write_bytes(b'not executable fixture; must refuse by presence')
            with self.assertRaisesRegex(ValueError, 'negative-control executable'):
                review.negative_program_preflight(root)

    def test_negative_program_on_path_refuses(self):
        with tempfile.TemporaryDirectory() as directory, patch.object(review.shutil, 'which', return_value='unexpected.exe'):
            with self.assertRaisesRegex(ValueError, 'negative-control executable'):
                review.negative_program_preflight(Path(directory))

    def test_inherited_profile_control_refuses_before_git_probe(self):
        with patch.dict(review.legacy.os.environ, {'HC074_PIPE_COST':'v1'}, clear=True), patch.object(
                review.legacy.subprocess, 'run', side_effect=AssertionError('must refuse before Git')), patch.object(
                Path, 'home', return_value=ROOT):
            with self.assertRaisesRegex(ValueError, 'fixture/capture overrides'):
                review.environment_preflight(ROOT)

    def test_build_overrides_refuse_without_git_or_configuration_changes(self):
        for name, value in [('RUSTFLAGS','--cfg linux'), ('CARGO_BUILD_TARGET','other-target'),
                            ('CARGO_ENCODED_RUSTFLAGS','--cfg=fixture'), ('RUSTC_WRAPPER','unexpected'),
                            ('CARGO_HOME','other-home'), ('CARGO_INCREMENTAL','1'), ('CC','unexpected')]:
            with patch.dict(review.os.environ, {name:value}, clear=True), patch.object(
                    review.legacy.subprocess, 'run', side_effect=AssertionError('no Git')):
                with self.assertRaisesRegex(ValueError, 'unreviewed build overrides'):
                    review.environment_preflight(ROOT)

    def test_external_shadowing_and_home_cargo_configs_refuse(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(review.os.environ, {}, clear=True):
            base = Path(directory).resolve()
            root = base/'repo'; root.mkdir()
            home = base/'home-cargo'; home.mkdir()
            # Do not depend on any actual machine's ancestor Cargo config.
            with patch.object(Path, 'exists', return_value=False):
                self.assertTrue(review.build_environment_preflight(root, home)['external_cargo_configuration_absent'])
            for path in (root/'.cargo/config', base/'.cargo/config.toml', home/'config.toml'):
                with patch.object(Path, 'exists', lambda current: current == path):
                    with self.assertRaisesRegex(ValueError, 'Cargo configuration requires review'):
                        review.build_environment_preflight(root, home)

    def test_only_original_exact_wrapper_build_controls_are_allowed(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(review.os.environ,
                {'CARGO_INCREMENTAL':'0','CARGO_BUILD_JOBS':'2'}, clear=True), patch.object(Path, 'exists', return_value=False):
            root = Path(directory)
            self.assertTrue(review.build_environment_preflight(root, root/'home')['unreviewed_build_overrides_absent'])
