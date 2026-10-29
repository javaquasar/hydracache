"""Inert root/preimage/path controls; no mutation of the actual product root."""
from pathlib import Path
import json
import tempfile
import unittest

import apply as a


class Application(unittest.TestCase):
    def test_whole_approved_packet_and_original_payloads_are_bound(self):
        recipe = json.loads(a.member('candidate/fileset.json'))
        self.assertEqual(len(recipe['operations']), 291)
        self.assertFalse(recipe['applied'])
        for row in recipe['operations']:
            self.assertEqual(a.sha(a.member('candidate/after/' + row['path'])), row['after_sha256'])
            if row['before_sha256'] is not None:
                self.assertEqual(a.sha(a.member('candidate/before/' + row['path'])), row['before_sha256'])

    def test_wrong_preimage_existing_new_source_and_payload_refuse(self):
        with tempfile.TemporaryDirectory(dir=a.ROOT/'target') as name:
            root = Path(name)
            (root/'Cargo.toml').write_bytes(b'original')
            rows = [dict(path='Cargo.toml', before_sha256=a.sha(b'original'), after_sha256=a.sha(b'candidate'), bytes=9)]
            get = lambda name: b'original' if '/before/' in name else b'candidate'
            a.verify(root, rows, get)
            (root/'Cargo.toml').write_bytes(b'foreign')
            with self.assertRaises(ValueError): a.verify(root, rows, get)
            rows[0]['before_sha256'] = None
            with self.assertRaises(ValueError): a.verify(root, rows, get)
            (root/'Cargo.toml').unlink()
            with self.assertRaises(ValueError): a.verify(root, rows, lambda n: b'wrong')

    def test_traversal_foreign_directories_and_policy_changes_are_not_candidate_paths(self):
        for name in ('../Cargo.toml', 'a/../Cargo.toml', '/Cargo.toml', 'C:/x', 'a\\b',
                     'deny.toml', '.git/config', 'docs/testing/performance/0.74/qualification-manifest.toml',
                     'third_party/foreign/lib.rs', 'tools/long-run-supervisor-074/src/../Cargo.toml'):
            with self.subTest(name=name), self.assertRaises(ValueError): a.destination(a.ROOT, name)


if __name__ == '__main__': unittest.main()
