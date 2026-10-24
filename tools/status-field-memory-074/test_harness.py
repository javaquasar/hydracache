"""Offline exact-harness extraction controls; no WSL or measurement."""
import importlib.util
from pathlib import Path
import unittest
import tempfile
from unittest.mock import patch
SPEC=importlib.util.spec_from_file_location('memory_run',Path(__file__).with_name('run.py'))
run=importlib.util.module_from_spec(SPEC);SPEC.loader.exec_module(run)

class HarnessControls(unittest.TestCase):
    def test_only_timing_entry_is_removed_dataset_and_parser_context_stay_exact(self):
        source=run.archived_sources('baseline')['main.rs']
        probe=b'fn main() { /* memory */ }\n'
        result=run.harness(source,probe)
        self.assertIn(b'include!("projection.rs");',result)
        self.assertIn(b'fn dataset(shape: usize)',result)
        self.assertIn(b'const SEED: u64 = 0x0740_5ca1;',result)
        self.assertNotIn(b'clock_gettime',result)
        self.assertNotIn(b'Instant',result)
        self.assertTrue(result.endswith(probe))
        self.assertEqual(run.harness(source.replace(b'\n',b'\r\n'),probe),result)

    def test_ambiguous_missing_or_shifted_boundaries_refuse(self):
        source=run.archived_sources('baseline')['main.rs']
        for bad in (b'',source+source,source.replace(b'const SHAPES:',b'const OTHER:'),
                    source.replace(b'const ITERATIONS: usize = 20_000;',b'const ITERATIONS: usize = 10;')):
            with self.subTest(raw=bad[:40]),self.assertRaises(ValueError):run.harness(bad,b'')

    def test_retained_both_source_sets_match_existing_build_receipts(self):
        baseline=run.sources(run.PRIOR/'baseline')
        candidate=run.sources(run.PRIOR/'candidate')
        self.assertEqual(baseline['main.rs'],candidate['main.rs'])
        self.assertNotEqual(baseline['projection.rs'],candidate['projection.rs'])
        self.assertNotIn('projection_fields.rs',baseline)
        self.assertIn('projection_fields.rs',candidate)

    def test_repository_archive_reconstructs_the_exact_sources_without_old_elf(self):
        for lane in ('baseline','candidate'):
            self.assertEqual(run.archived_sources(lane),run.sources(run.PRIOR/lane))
        with self.assertRaises(ValueError):run.archived_sources('../candidate')

    def test_missing_local_sources_use_only_the_pinned_repository_archive(self):
        with tempfile.TemporaryDirectory(prefix='hc074-parser-source-control-') as directory:
            self.assertEqual(run.sources(Path(directory)/'baseline'),run.archived_sources('baseline'))
            fake=Path(directory)
            (fake/'index.json').write_bytes(b'{}')
            with self.assertRaises(ValueError):run.archived_sources('baseline',fake)

    def test_source_object_corruption_refuses_without_regenerating_a_baseline(self):
        original=Path.read_bytes
        import json
        index=json.loads(original(run.ARCHIVE/'index.json'))
        digest=index['files']['observation/baseline/main.rs']['sha256']
        def changed(path):
            raw=original(path)
            return raw+b'// drift' if path.name==digest else raw
        with patch.object(Path,'read_bytes',changed),self.assertRaises(ValueError):
            run.archived_sources('baseline')

if __name__=='__main__':unittest.main()
