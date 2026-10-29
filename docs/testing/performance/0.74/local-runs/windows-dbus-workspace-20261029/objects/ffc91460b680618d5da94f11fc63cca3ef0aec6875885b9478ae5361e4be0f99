"""Offline source/counter receipt controls, never native measurements."""
import copy
import io
import json
from pathlib import Path
import tarfile
import tempfile
import unittest
import run as method
import review
import seal


class Controls(unittest.TestCase):
    def test_dependency_copy_preserves_executable_mode_without_aliasing(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); original = root / 'original'; destination = root / 'copy'
            original.write_bytes(b'build-script-control'); original.chmod(0o755)
            method.copy_dependency(original, destination, method.sha(original))
            self.assertEqual(original.stat().st_mode, destination.stat().st_mode)
            destination.write_bytes(b'copy-only')
            self.assertEqual(original.read_bytes(), b'build-script-control')

    def test_original_static_includes_must_exist_inside_overlay(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); package = root / method.PACKAGE; package.mkdir(parents=True)
            (package / 'probe.rs').write_text('const X: &[u8] = include_bytes!("included");')
            with self.assertRaises(ValueError):
                method.validate_includes(root)
            (package / 'included').write_bytes(b'original')
            self.assertEqual(list(method.validate_includes(root)), [method.PACKAGE + 'included'])
            (package / 'probe.rs').write_text('const X: &str = include_str!("../../../../outside");')
            with self.assertRaises(ValueError):
                method.validate_includes(root)

    def test_archive_refuses_paths_and_links(self):
        for path in ('../a', '/a', 'a/../b', 'C:/a', 'a\\b', 'a//b', 'a\x00b'):
            with self.assertRaises(ValueError):
                method.member(path)
        with tempfile.TemporaryDirectory() as directory:
            raw = io.BytesIO()
            with tarfile.open(fileobj=raw, mode='w') as archive:
                item = tarfile.TarInfo('link'); item.type = tarfile.SYMTYPE; item.linkname = '/outside'
                archive.addfile(item)
            with self.assertRaises(ValueError):
                method.extract(raw.getvalue(), Path(directory) / 'new')

    def test_overlay_whole_file_and_membership_are_exact(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); path = root / method.TESTS; path.parent.mkdir(parents=True)
            originals = {method.TESTS: b'fn original_body() {}\n'}
            path.write_bytes(originals[method.TESTS] + method.HOOK.encode())
            for name, data in method.generated('installed').items():
                target = root / name; target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(data)
            method.validate_package(root, originals, 'installed')
            path.write_bytes(path.read_bytes() + b'fn unexpected() {}')
            with self.assertRaises(ValueError):
                method.validate_package(root, originals, 'installed')

    def test_exact_test_result_and_single_release_image(self):
        row = dict(reason='compiler-artifact', executable='/owned/image', fresh=False, target=dict(name='hydracache_long_run_supervisor_074'), profile=dict(test=True, opt_level='3'))
        raw = json.dumps(row).encode() + b'\n'
        self.assertEqual(method.image(raw), '/owned/image')
        for bad in (b'', raw + raw):
            with self.assertRaises(ValueError):
                method.image(bad)
        with self.assertRaises(ValueError):
            method.image(raw.replace(b'"fresh": false', b'"fresh": true'))
        good = b'running 1 test\ntest exact ... ok\ntest result: ok. 1 passed; 0 failed; 0 ignored;\n'
        method.passed(good, 'exact')
        method.passed(good.replace(b'test exact ... ok', b'test exact ... ROW\nok'), 'exact')
        with self.assertRaises(ValueError):
            method.passed(good.replace(b'test exact ... ok', b'test exact ... FAILED'), 'exact')
        with self.assertRaises(ValueError):
            method.passed(good.replace(b'0 ignored', b'1 ignored'), 'exact')

    def test_pair_drift_and_missing_pair_refuse_without_win_threshold(self):
        counts = dict(allocations=0, zeroed=0, reallocations=0, requested_bytes=0, deallocations=0, released_bytes=0, live_delta=0, peak_delta=0)
        cells = []
        for depth, fault in review.CELLS:
            row = dict(depth=depth, fault=fault, seed=74, iterations=32, corpus='a' * 64,
                root_name_bytes=28, owner_reconciled=True, original_objects_preserved=True, server=False, product=False,
                after_refusal=dict(scratch_capacity=0, scratch_slots=0), after_warm=dict(scratch_capacity=0),
                max_publication=None, rss=dict(warm=4096, drop=4096), **{key: copy.deepcopy(counts) for key in ('open', 'warm', 'refusal', 'sticky', 'drop')})
            cells.append(row)
            if depth == 127:
                row['max_publication'] = dict(counts=copy.deepcopy(counts), state=dict(generations=128))
        raw = '\n'.join('HC074-RUNTIME-ALLOCATION ' + json.dumps(row) for row in cells)
        rows = review.rows(raw)
        self.assertEqual(rows, review.rows('test ' + method.MEASURE + ' ... ' + raw))
        with self.assertRaises(ValueError):
            review.rows('unrelated prefix ' + raw)
        pairs = [dict(baseline=copy.deepcopy(rows), installed=copy.deepcopy(rows)) for _ in range(5)]
        self.assertEqual(len(review.summarize(pairs)), 8)  # Zero win remains valid data.
        with self.assertRaises(ValueError):
            review.summarize(pairs[:-1])
        pairs[0]['installed'][0]['corpus'] = 'different'
        with self.assertRaises(ValueError):
            review.summarize(pairs)

    def test_lifetime_counter_negative_controls(self):
        value = dict(allocations=1, zeroed=0, reallocations=0, requested_bytes=65, deallocations=0, released_bytes=0, live_delta=65, peak_delta=65)
        self.assertEqual(review.validate_counts(value), 65)
        for key, bad in (('live_delta', 64), ('live_delta', 65.0), ('requested_bytes', -1), ('zeroed', True)):
            changed = dict(value); changed[key] = bad
            with self.assertRaises(ValueError):
                review.validate_counts(changed)

    def test_inventory_tracks_raw_bytes_and_excludes_only_root_seal(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory); (root / 'data').write_bytes(b'first')
            before = seal.inventory(root)
            (root / 'inventory.json').write_text('{}')
            self.assertEqual(seal.inventory(root), before)
            (root / 'data').write_bytes(b'other')
            self.assertNotEqual(seal.inventory(root), before)
            (root / 'nested').mkdir(); (root / 'nested/inventory.json').write_text('{}')
            self.assertEqual(len(seal.inventory(root)), 2)


if __name__ == '__main__':
    unittest.main(verbosity=2)
