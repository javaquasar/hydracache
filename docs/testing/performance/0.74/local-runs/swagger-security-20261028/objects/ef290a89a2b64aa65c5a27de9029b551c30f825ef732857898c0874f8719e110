import copy
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

import prepare as p
import storage as s
import metadata as m


class Contracts(unittest.TestCase):
    def test_original_complete_package_contract(self):
        value = p.contract()
        package = value['package']
        self.assertEqual(package['argv'], ['cargo', '+1.94.0', 'test', '--offline',
            '--locked', '--no-fail-fast', '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4'])
        self.assertEqual((package['bound_seconds'], package['test_threads'],
            package['physical_floor_bytes'], package['target_catalog_count']), (780, 4, 4294967296, 156))
        self.assertTrue(package['includes_compilation'] and package['all_default_targets_required'])
        self.assertFalse(package['retry'] or package['concatenate_prior_results'])
        self.assertIsNone(package['filter'])
        self.assertEqual(value['live_contract']['origins_and_windows'], [300, 60, 10])
        self.assertFalse(value['dispatch_supported'] or value['release_admitted'])

    def test_workspace_is_not_replaced_by_package_or_new_budget(self):
        value = p.contract()['workspace']
        self.assertEqual(value['argv'], ['cargo', 'xtask', 'verify'])
        self.assertEqual((value['original_captured_outer_bound_seconds'],
            value['original_start_free_bytes'], value['original_running_floor_bytes']),
            (4800, 16*1024**3, 4*1024**3))
        self.assertEqual(value['windows_tests'], [
            ['cargo', 'test', '--workspace', '--exclude', 'xtask', '--locked', '-j', '1'],
            ['cargo', 'test', '-p', 'xtask', '--lib', '--tests', '--locked', '-j', '1']])
        self.assertEqual(len(value['remaining_stages']), 13)
        self.assertTrue(value['package_780_is_not_a_whole_verify_budget'])
        self.assertFalse(value['bounded_outer_execution_envelope_prepared'] or value['result'])
        self.assertEqual(value['isolated_target_for'],
            ['tests (xtask lib/integration)', 'performance budget contract'])

    def test_sampled_floor_never_implies_growth_reservation(self):
        for free, floor_ok in [(0, False), (p.FLOOR-1, False), (p.FLOOR, True), (100*p.FLOOR, True)]:
            with self.subTest(free=free):
                value = p.capacity(free)
                self.assertEqual(value['original_floor_satisfied'], floor_ok)
                self.assertFalse(value['launch_ready'] or value['build_growth_reservation_established']
                    or value['pagefile_growth_reservation_established'])
        for bad in (-1, '1', True):
            with self.assertRaises(ValueError):
                p.capacity(bad)

    def test_fixed_verify_and_source_helper_drift_refuses_before_import(self):
        for name in ('crates/xtask/src/verify.rs', 'tools/package-frontier-gate-074/review.py'):
            with tempfile.TemporaryDirectory() as folder:
                root = Path(folder)
                path = root / name
                path.parent.mkdir(parents=True)
                path.write_bytes(b'not the reviewed body')
                with patch.dict(p.PINS, {name: p.PINS[name]}, clear=True), \
                        patch.object(p, 'load') as imported, self.assertRaises(ValueError):
                    p.source_review(root)
                imported.assert_not_called()

    def test_path_escape_refuses(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder) / 'root'
            root.mkdir()
            outside = root.parent / 'outside'
            outside.write_bytes(b'no')
            with self.assertRaises(ValueError):
                p.checked(root, '../outside', p.sha(b'no'))

    def test_workspace_inventory_is_manifest_only(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'one').mkdir()
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=["one"]\n')
            (root / 'one/Cargo.toml').write_text('[package]\nname="one"\n')
            self.assertEqual(p.workspace_members(root)['one/Cargo.toml']['package'], 'one')
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=["../outside"]\n')
            with self.assertRaises(ValueError):
                p.workspace_members(root)

    def test_first_refusal_and_raw_streams_are_retained_and_exclusive(self):
        with tempfile.TemporaryDirectory() as folder:
            out = Path(folder) / 'attempt'
            with patch.object(p.os, 'name', 'nt'), patch.object(p.subprocess, 'run',
                    return_value=subprocess.CompletedProcess(['git'], 128)) as run:
                with self.assertRaises(ValueError):
                    p.prepare(out)
                self.assertEqual(run.call_count, 1)
            self.assertTrue((out / 'intent.json').is_file())
            self.assertTrue((out / 'contract.json').is_file())
            self.assertTrue((out / 'head-before.stdout.log').is_file())
            self.assertTrue((out / 'head-before.stderr.log').is_file())
            self.assertFalse(json.loads((out / 'refusal.json').read_bytes())['gate_dispatched'])
            original = {path.name: path.read_bytes() for path in out.iterdir()}
            with self.assertRaises(FileExistsError):
                p.prepare(out)
            self.assertEqual(original, {path.name: path.read_bytes() for path in out.iterdir()})

    def test_cli_has_no_execution_retry_timeout_or_filter_switch(self):
        for flag in ('--execute', '--retry', '--timeout', '--test'):
            with self.subTest(flag=flag):
                result = subprocess.run([sys.executable, '-B', str(Path(p.__file__)), flag],
                    capture_output=True, timeout=10)
                self.assertEqual(result.returncode, 2)

    def test_metadata_scan_does_not_read_contents_or_claim_growth(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'debug').mkdir()
            (root / 'debug/one').write_bytes(b'abc')
            (root / 'two').write_bytes(b'four')
            with patch.object(Path, 'read_bytes', side_effect=AssertionError('content read')):
                value = s.scan(root, since=0)
            self.assertTrue(value['complete'])
            self.assertEqual(sum(row['logical_bytes'] for row in value['groups'].values()), 7)
            self.assertEqual(sum(row['files'] for row in value['groups'].values()), 2)
            self.assertFalse(value['allocation_or_growth_measurement'])

    def test_metadata_reparse_entry_is_not_traversed(self):
        class Entry:
            path = 'never-traverse'
            def stat(self, **_):
                return type('Stat', (), {'st_file_attributes': 0x400})()
        class Entries:
            def __enter__(self):
                return iter([Entry()])
            def __exit__(self, *_):
                pass
        with patch.object(s.os, 'scandir', return_value=Entries()) as scan:
            result = s.scan(Path('owned'))
        self.assertEqual(scan.call_count, 1)
        self.assertEqual(result['skipped_reparse'], ['never-traverse'])
        self.assertFalse(result['complete'])

    @unittest.skipUnless(sys.platform == 'win32', 'Windows metadata API')
    def test_zero_access_metadata_query_and_missing_file_refusal(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'one'
            path.write_bytes(b'abc')
            value = m.query(path)
            self.assertEqual(value['logical_bytes'], 3)
            self.assertGreaterEqual(value['allocation_bytes'], 3)
            self.assertIsNone(value['query_winerror'])
            self.assertIsNone(value['close_winerror'])
            absent = m.query(Path(folder) / 'absent')
            self.assertIsNone(absent['allocation_bytes'])
            self.assertEqual(absent['open_winerror'], 2)


if __name__ == '__main__':
    unittest.main()
