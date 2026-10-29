"""Exact lock/source effect controls using saved metadata, no Cargo or code execution."""
import copy
import json
from pathlib import Path
import tempfile
import unittest

import effects as e


class Effects(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.before = e.r.checked('Cargo.lock', e.r.PROTECTED['Cargo.lock'])
        cls.after = (e.r.BASE / 'input/workspace/Cargo.lock').read_bytes()
        cls.qualified = e.r.held('consumer2/input/fd/Cargo.lock')
        cls.metadata = json.loads((e.r.BASE / 'metadata/cargo.stdout.json').read_bytes())
        cls.zbus = e.native_member('feature-audit/old-Cargo.toml')
        cls.tokio = e.native_member('feature-audit/tokio-Cargo.toml')

    def check(self, after=None, metadata=None, qualified=None, tokio=None):
        return e.review(self.before, self.after if after is None else after,
            self.qualified if qualified is None else qualified,
            self.metadata if metadata is None else metadata, self.zbus, self.tokio if tokio is None else tokio)

    def test_lock_default_metadata_and_optional_nodes_have_separate_exact_accounting(self):
        result = self.check()
        self.assertEqual((result['old_lock_packages'], result['new_lock_packages'], result['default_metadata_packages']), (689, 683, 678))
        self.assertEqual(len(result['added']), 8)
        self.assertEqual(len(result['removed']), 14)
        self.assertEqual(len(result['lock_only_default_feature_nodes']), 5)
        self.assertTrue(result['retained_checksums_unchanged'] and result['all_added_nodes_equal_qualified'])
        self.assertFalse(result['upstream_body_review_complete'] or result['compiler_gate_complete'] or result['root_adopted'])

    def test_added_identity_checksum_dependency_or_primary_feature_mutation_refuses(self):
        # Real retained checksum, unexpected edge, duplicate package, and old backend.
        catalog = e.core.packages(self.after)
        row = next(p for k, p in catalog.items() if k[0] == 'tracing')
        for mutated in (self.after.replace(row['checksum'].encode(), b'0'*64, 1),
                        self.after + b'\n[[package]]\nname = "foreign"\nversion = "1.0.0"\n',
                        self.after + b'\n[[package]]\nname = "zbus"\nversion = "5.19.0"\n'):
            with self.assertRaises(ValueError):
                self.check(after=mutated)
        with self.assertRaises(ValueError):
            self.check(tokio=self.tokio + b'\n')
        changed = copy.deepcopy(self.metadata)
        changed['packages'].pop()
        with self.assertRaises(ValueError):
            self.check(metadata=changed)

    def test_same_checksum_other_cache_slot_cannot_choose_origin(self):
        with tempfile.TemporaryDirectory(dir=e.r.ROOT / 'target') as name:
            home = Path(name)
            slot = 'index.crates.io-1949cf8c6b5b557f'
            actual = dict(name='futures-core', version='0.3.34',
                          manifest_path=str(home / 'registry/src' / slot / 'futures-core-0.3.34/Cargo.toml'))
            row = dict(name='futures-core', version='0.3.34')
            chosen = e.indexed_archive(row, actual, home)
            chosen.parent.mkdir(parents=True)
            chosen.write_bytes(b'identical primary bytes')
            alternative = home / 'registry/cache/index.crates.io-6f17d22bba15001f/futures-core-0.3.34.crate'
            alternative.parent.mkdir(parents=True)
            alternative.write_bytes(chosen.read_bytes())
            self.assertEqual(chosen, home / 'registry/cache' / slot / 'futures-core-0.3.34.crate')
            self.assertNotEqual(chosen, alternative)
            other = copy.deepcopy(actual)
            other['manifest_path'] = str(home / '../external/futures-core-0.3.34/Cargo.toml')
            with self.assertRaises(ValueError):
                e.indexed_archive(row, other, home)
            with self.assertRaises(ValueError):
                e.indexed_archive(dict(name='foreign', version='0.3.34'), actual, home)


if __name__ == '__main__':
    unittest.main()
