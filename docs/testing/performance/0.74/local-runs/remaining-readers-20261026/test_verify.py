"""Offline packet integrity controls; no Rust, fixture or measurement run."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('reader_packet', HERE / 'verify.py')
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
        self.assertEqual(verifier.verify(self.folder)['cases_per_compiler'], 61)

    def test_changed_index_refuses(self):
        with (self.folder / 'index.json').open('ab') as stream:
            stream.write(b' ')
        with self.assertRaisesRegex(ValueError, 'index'):
            verifier.verify(self.folder)

    def test_missing_final_descriptor_refuses(self):
        self.object('source-final/tools/remaining-readers-074/rules.json').unlink()
        with self.assertRaises(FileNotFoundError):
            verifier.verify(self.folder)

    def test_truncated_original_caller_refuses(self):
        self.object('baseline/crates/xtask/tests/fresh_kernel_cost_074.rs').write_bytes(b'')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_changed_first_integration_refusal_refuses(self):
        path = self.object('checks/remaining-readers-current-controls/stdout.log')
        body = path.read_bytes()
        path.write_bytes(bytes([body[0] ^ 1]) + body[1:])
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_forged_final_receipt_refuses(self):
        path = self.object('checks/remaining-readers-candidate-second/result.json')
        value = json.loads(path.read_bytes())
        value['qualification'] = True
        path.write_text(json.dumps(value), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_changed_intermediate_body_refuses(self):
        self.object('interim-source/tools/remaining-readers-074/inputs.rs').write_bytes(b'changed intermediate')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)


if __name__ == '__main__':
    unittest.main()
