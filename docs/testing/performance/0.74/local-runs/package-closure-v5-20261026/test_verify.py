"""Offline packet controls; no package or workload rerun."""
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('package_v5_packet', HERE / 'verify.py')
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class PacketControls(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.folder = Path(self.temp.name) / 'packet'
        shutil.copytree(HERE, self.folder, ignore=shutil.ignore_patterns('__pycache__'))

    def member(self, name):
        row = json.loads((self.folder / 'index.json').read_bytes())['members'][name]
        return self.folder / 'objects' / row['sha256']

    def test_clean_red_result_keeps_partial_and_unstarted_coverage(self):
        result = verifier.verify(self.folder)
        self.assertEqual((result['completed_targets'], result['failed_completed'], result['unstarted_targets']), (76, 3, 67))
        self.assertFalse(result['package_pass'])

    def test_forged_index_cannot_promote_result(self):
        path = self.folder / 'index.json'
        value = json.loads(path.read_bytes())
        value['package_pass'] = True
        path.write_text(json.dumps(value), encoding='utf-8')
        with self.assertRaisesRegex(ValueError, 'packet index'):
            verifier.verify(self.folder)

    def test_truncated_stdout_refuses(self):
        self.member('checks/package-v5-first/stdout.log').write_bytes(b'')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_changed_source_receipt_refuses(self):
        self.member('checks/package-v5-first/result.json').write_bytes(b'{"source_unchanged":true}')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_shortened_derived_coverage_refuses(self):
        self.member('derived/accounting.json').write_bytes(b'{"complete":true}')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_hidden_cleanup_error_refuses(self):
        self.member('checks/package-v5-first/owned-cleanup.log').write_bytes(b'success')
        with self.assertRaisesRegex(ValueError, 'raw member'):
            verifier.verify(self.folder)

    def test_missing_first_source_change_refusal_refuses(self):
        self.member('checks/package-effect-v5-controls-first/result.json').unlink()
        with self.assertRaises(FileNotFoundError):
            verifier.verify(self.folder)


if __name__ == '__main__':
    unittest.main()
