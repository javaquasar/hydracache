"""Inert fixed-command, original capacity class and source-mutation controls."""
from pathlib import Path
import json
import os
import tempfile
import unittest
from unittest import mock

import run as runner


class Runner(unittest.TestCase):
    def test_fixed_offline_command_ignores_caller_compiler_wrappers_and_cache_override(self):
        with mock.patch.dict(os.environ, {'RUSTC_WRAPPER': 'untrusted-wrapper', 'RUSTC': 'foreign-rustc',
                                        'CARGO_HOME': 'foreign-cache', 'SECRET_TOKEN': 'do-not-forward'}):
            argv, env = runner.command()
        self.assertEqual(argv[1:5], ['metadata', '--offline', '--format-version', '1'])
        self.assertNotIn('--locked', argv)
        self.assertNotIn('SECRET_TOKEN', env)
        self.assertEqual(env['RUSTC_WRAPPER'], '')
        self.assertEqual(env['RUSTC_WORKSPACE_WRAPPER'], '')
        self.assertEqual(env['RUSTC'], str(runner.CARGO.parent / 'rustc.exe'))
        self.assertEqual(env['CARGO_NET_OFFLINE'], 'true')
        self.assertEqual(env['CARGO_HOME'], str(Path(os.environ['USERPROFILE']) / '.cargo'))

    def test_original_capacity_exception_identity_is_stable_and_floor_preserved(self):
        owner = runner.owner()
        self.assertIs(owner, runner.owner())
        with mock.patch.object(runner.shutil, 'disk_usage', return_value=mock.Mock(free=runner.FLOOR-1)):
            with self.assertRaises(owner.CapacityError):
                runner.capacity()
        self.assertEqual(runner.FLOOR, 4*1024**3)

    def test_only_candidate_lock_may_change_no_source_or_additional_files(self):
        with tempfile.TemporaryDirectory(dir=runner.r.ROOT / 'target') as name:
            folder = Path(name)
            source = folder / 'input/workspace'
            source.mkdir(parents=True)
            (source / 'Cargo.lock').write_bytes(b'original lock')
            (source / 'Cargo.toml').write_bytes(b'original manifest')
            review = dict(files=runner.r.inventory(folder / 'input'), root_files={}, protected={}, helpers={})
            with mock.patch.object(runner.r, 'BASE', folder):
                runner.guard(review)
                (source / 'Cargo.lock').write_bytes(b'new candidate lock')
                with self.assertRaises(ValueError):
                    runner.guard(review)
                runner.guard(review, lock_may_change=True)
                (source / 'Cargo.toml').write_bytes(b'changed manifest')
                with self.assertRaises(ValueError):
                    runner.guard(review, lock_may_change=True)

    def test_streams_and_error_receipt_exist_before_missing_freeze_preflight(self):
        with tempfile.TemporaryDirectory(dir=runner.r.ROOT / 'target') as name:
            folder = Path(name)
            with mock.patch.object(runner.r, 'BASE', folder), mock.patch.object(runner, 'OUT', folder / 'metadata'):
                with self.assertRaises(FileNotFoundError):
                    runner.main()
            self.assertTrue((folder / 'metadata/stdout.log').is_file())
            self.assertIn(b'FileNotFoundError', (folder / 'metadata/stderr.log').read_bytes())
            self.assertEqual(json.loads((folder / 'metadata/refusal.json').read_bytes())['type'], 'FileNotFoundError')
            self.assertFalse((folder / 'metadata/execution.json').exists())

    def test_complete_metadata_cannot_hide_members_vendor_targets_or_foreign_registry(self):
        import copy
        root = runner.r.BASE / 'input/workspace'
        members = ['crates/hydracache', runner.r.PKG]
        paths = members + [runner.r.VENDOR + '/' + n for n in ('zbus', 'zbus_macros', 'zbus_utils')]
        packages = [dict(id='p'+str(i), name=Path(p).name, version='6.1.0' if p.endswith('/zbus') else '1.0.0',
                         source=None, manifest_path=str(root / p / 'Cargo.toml')) for i, p in enumerate(paths)]
        metadata = dict(version=1, packages=packages, workspace_members=['p0', 'p1'],
                        resolve=dict(nodes=[dict(id=p['id'], deps=[]) for p in packages]))
        review = dict(publication_scope=dict(members=members))
        runner.validate_metadata(metadata, review)
        for mutation in ('missing-member', 'vendor-member', 'foreign-source', 'missing-node', 'old-backend'):
            altered = copy.deepcopy(metadata)
            if mutation == 'missing-member':
                altered['workspace_members'].pop()
            elif mutation == 'vendor-member':
                altered['workspace_members'].append('p2')
            elif mutation == 'foreign-source':
                altered['packages'][2]['source'] = 'registry+https://unreviewed.invalid/index'
            elif mutation == 'missing-node':
                altered['resolve']['nodes'].pop()
            else:
                altered['packages'][2]['version'] = '5.19.0'
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                runner.validate_metadata(altered, review)


if __name__ == '__main__':
    unittest.main()
