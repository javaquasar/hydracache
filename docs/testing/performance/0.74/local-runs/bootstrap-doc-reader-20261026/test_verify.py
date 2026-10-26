"""Offline packet integrity refusal controls, not another Rust or live run."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('bootstrap_packet', HERE / 'verify.py')
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class Controls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name) / 'packet'
        shutil.copytree(HERE, self.folder, ignore=shutil.ignore_patterns('__pycache__'))

    def object(self, name):
        row = json.loads((self.folder / 'index.json').read_bytes())['members'][name]
        return self.folder / 'objects' / row['sha256']

    def test_clean_replay(self):
        self.assertEqual(verifier.verify(self.folder)['cases_per_final_compiler'], 69)

    def test_changed_index_refuses(self):
        with (self.folder / 'index.json').open('ab') as output:
            output.write(b' ')
        with self.assertRaisesRegex(ValueError, 'index'):
            verifier.verify(self.folder)

    def test_missing_object_refuses(self):
        self.object('source-final/tools/bootstrap-doc-reader-074/rules.json').unlink()
        with self.assertRaises(FileNotFoundError):
            verifier.verify(self.folder)

    def test_truncated_baseline_refuses(self):
        self.object('baseline/crates/xtask/tests/owned_case_074.rs').write_bytes(b'')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_changed_first_failure_refuses(self):
        path = self.object('checks/bootstrap-doc-reader-clippy-first/stderr.log')
        body = path.read_bytes()
        path.write_bytes(bytes([body[0] ^ 1]) + body[1:])
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_rewritten_success_receipt_refuses(self):
        path = self.object('checks/bootstrap-doc-reader-candidate-third/result.json')
        value = json.loads(path.read_bytes())
        value['qualification'] = True
        path.write_text(json.dumps(value), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)


if __name__ == '__main__':
    unittest.main()
