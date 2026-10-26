"""Integrity refusal controls for offline evidence replay only."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('frontier_packet', HERE / 'verify.py')
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
        self.assertEqual(verifier.verify(self.folder)['accounted_inputs'], 26)

    def test_changed_index_refuses(self):
        with (self.folder / 'index.json').open('ab') as output:
            output.write(b' ')
        with self.assertRaisesRegex(ValueError, 'index'):
            verifier.verify(self.folder)

    def test_missing_source_descriptor_refuses(self):
        self.object('source-final/tools/current-frontier-074/rules.json').unlink()
        with self.assertRaises(FileNotFoundError):
            verifier.verify(self.folder)

    def test_truncated_independent_inventory_refuses(self):
        self.object('independent-inventory544.json').write_bytes(b'')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_changed_first_refusal_refuses(self):
        path = self.object('checks/current-frontier-baseline-first/stdout.log')
        body = path.read_bytes()
        path.write_bytes(bytes([body[0] ^ 1]) + body[1:])
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_forged_final_outcome_refuses(self):
        path = self.object('checks/current-frontier-candidate-first/result.json')
        value = json.loads(path.read_bytes())
        value['qualification'] = True
        path.write_text(json.dumps(value), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_changed_reconstructed_observation_refuses(self):
        index = json.loads((self.folder / 'observed-bindings.json').read_bytes())
        row = index['members']['observed/crates/hydracache-redis-compat/Cargo.toml']
        (self.folder / 'objects' / row['sha256']).write_bytes(b'changed observed input')
        with self.assertRaisesRegex(ValueError, 'raw observed member'):
            verifier.verify(self.folder)


if __name__ == '__main__':
    unittest.main()
