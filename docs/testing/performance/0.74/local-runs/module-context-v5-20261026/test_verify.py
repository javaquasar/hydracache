"""Offline evidence controls; never runs a compiler or retained Rust."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('module_packet_verify', HERE / 'verify.py')
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class IntegrityControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name) / 'packet'
        shutil.copytree(HERE, self.folder, ignore=shutil.ignore_patterns('__pycache__'))

    def member(self, name):
        row = json.loads((self.folder / 'index.json').read_bytes())['members'][name]
        return self.folder / 'objects' / row['sha256']

    def test_clean_replay_keeps_source_only_scope(self):
        result = verifier.verify(self.folder)
        self.assertEqual((result['sources'], result['changed'], result['added']), (365, 21, 21))
        self.assertFalse(result['execution_effects_review_complete'])

    def test_repository_root_is_not_workspace_parent(self):
        self.assertTrue((verifier.ROOT / 'crates/xtask/Cargo.toml').is_file())
        self.assertEqual(verifier.ROOT, HERE.parents[5])

    def test_self_refreshed_index_refuses(self):
        path = self.folder / 'index.json'
        data = json.loads(path.read_bytes())
        data['execution_effects_review_complete'] = True
        path.write_text(json.dumps(data), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'packet index'):
            verifier.verify(self.folder)

    def test_first_refusal_mutation_refuses(self):
        self.member('syntax/first-refusal/refusal.json').write_bytes(b'passed')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_missing_delta_source_refuses(self):
        self.member('delta-source/tools/current-frontier-074/reader.rs').unlink()
        with self.assertRaises(FileNotFoundError):
            verifier.verify(self.folder)

    def test_truncated_final_syntax_refuses(self):
        self.member('syntax/contextual-closure/result.json').write_bytes(b'{}')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_historical_packet_cannot_change(self):
        original = Path.read_bytes
        index = verifier.ROOT / 'docs/testing/performance/0.74/local-runs/package-effect-syntax-v4-closure-20261025/index.json'
        with patch.object(Path, 'read_bytes', lambda path: b'changed' if path == index else original(path)):
            with self.assertRaisesRegex(ValueError, 'old index'):
                verifier.verify(self.folder)


if __name__ == '__main__':
    unittest.main()
