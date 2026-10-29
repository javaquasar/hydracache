from pathlib import Path
import json
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import helper_capture as capture
import preservation_reuse_audit as reuse
import sweep_review as sweep


class HelperCaptureControls(unittest.TestCase):
    def test_complete_trace_catalog_comes_from_original_ast_not_collected_rows(self):
        cases = capture.original_trace_cases()
        self.assertEqual(len(cases), 19)
        self.assertNotIn('held_build_source_allowlist_requires_all_old_files_and_exactly_one_named_addition', cases)
        self.assertTrue({name for name, *_ in sweep.SWEEPS}.issubset(cases))
    def test_erasure_reconstructs_original_not_arbitrary_body(self):
        root = Path(__file__).resolve().parents[2]
        folder = root/reuse.PACKET
        index_raw = (folder/'index.json').read_bytes()
        self.assertEqual(capture.review.sha(index_raw), reuse.PACKET_SHA256)
        source = capture.review.member(folder, json.loads(index_raw), 'measured-source/'+sweep.SOURCE).decode()
        contract = (root/'tools/xtask-package-attribution-074/helper_contract.rs').read_text()
        baseline = sweep.load_snapshot(root)['reviewed-source/'+sweep.SOURCE].decode()
        self.assertEqual(capture.erase(source, contract, baseline), baseline)
        # Current frozen source has no observer. The historical capture runner
        # must refuse it rather than manufacture a replacement measurement.
        self.assertEqual((root/sweep.SOURCE).read_text(), baseline)
        with self.assertRaises(ValueError):
            capture.erase(baseline, contract, baseline)
        for bad in (source.replace('output.status.success()', '!output.status.success()'),
                    source.replace('helper_trace::git(args)', 'helper_trace::git(&[])'),
                    source+'\nfn drift() {}\n'):
            with self.assertRaises(ValueError):
                capture.erase(bad, contract, baseline)
        with self.assertRaises(ValueError):
            capture.erase(source, contract.replace('pub const NEW_GIT:', 'pub const CHANGED_GIT:'), baseline)

    def test_fixture_filter_is_explicit_and_never_weakens_main_scope(self):
        raw = (b'running 1 test\ntest fixture ... ok\n'
               b'test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.01s\n')
        with self.assertRaises(ValueError):
            capture.target_result(raw, ['fixture'])
        self.assertTrue(capture.target_result(raw, ['fixture'], filtered=17)['target_pass'])
        for changed in (raw.replace(b'fixture ... ok', b'other ... ok'), raw+raw,
                        raw.replace(b'0 failed', b'1 failed'), raw+b'test result: unknown\n'):
            with self.assertRaises(ValueError):
                capture.target_result(changed, ['fixture'], filtered=17)
        with self.assertRaises(ValueError):
            capture.target_result(raw, ['fixture'], filtered=True)

    def test_disk_preflight_refusal_retains_intent_without_starting_child(self):
        with tempfile.TemporaryDirectory() as directory:
            folder = Path(directory)/'first'
            with patch.object(capture.shutil, 'disk_usage', return_value=SimpleNamespace(free=0)), \
                 patch.object(capture.subprocess, 'Popen') as spawn:
                with self.assertRaises(ValueError):
                    capture.capture(Path(directory)/'absent-build', folder, 'off')
            spawn.assert_not_called()
            self.assertFalse(json.loads((folder/'intent.json').read_text())['retry'])
            self.assertEqual(json.loads((folder/'refusal.json').read_text())['reason'], 'platform/disk')


if __name__ == '__main__':
    unittest.main()
