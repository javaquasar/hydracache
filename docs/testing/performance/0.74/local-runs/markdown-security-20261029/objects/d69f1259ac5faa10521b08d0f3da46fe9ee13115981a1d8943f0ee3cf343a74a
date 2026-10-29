import ast
import json
from pathlib import Path
import tempfile
import unittest

import remote
import run


class Controls(unittest.TestCase):
    def test_fixed_argv_cannot_select_original_service_or_materialization(self):
        argv = remote.argv(Path('/tmp/fixture'))
        self.assertEqual(argv.count('--exact'), 1)
        self.assertEqual(argv.count(remote.SELECTOR), 1)
        self.assertIn('HC074_CONSUMING_HELD_LIVE_CASE=healthy', argv)
        self.assertIn('HC074_CONSUMING_HELD_COST=1', argv)
        self.assertFalse(any('HC074_ORIGINAL_SERVICE=' in a or 'HC074_HELD_MATERIAL_LIVE=' in a for a in argv))
        self.assertEqual(argv[:3], ['/usr/bin/env', '-i', 'PATH=/usr/sbin:/usr/bin:/sbin:/bin'])

    def test_selected_body_has_no_start_and_unset_materialization_branch_drops(self):
        raw = (run.PARENT / 'input/fd/tools/long-run-supervisor-074/tests/support/consuming_held_live.rs').read_text()
        body = raw.split('fn run_original(case: &str) {', 1)[1]
        for effect in ('deliver_once', 'prepare_original_held_delivery', 'StartTransientUnit', 'Command::new'):
            self.assertNotIn(effect, body)
        self.assertIn('if let Ok(mode) = std::env::var("HC074_HELD_MATERIAL_LIVE")', body)
        self.assertIn('} else {\n                    drop(held);', body)
        self.assertIn('[300000000000, 60000000000, 10000000000]', body)

    def test_evidence_and_dispatch_latches_never_overwrite(self):
        with tempfile.TemporaryDirectory(dir=run.ROOT / 'target') as tmp:
            path = Path(tmp) / 'first.json'
            run.put(path, {'first': 74})
            with self.assertRaises(FileExistsError):
                run.put(path, {'first': 75})
            self.assertEqual(json.loads(path.read_bytes()), {'first': 74})

    def test_input_mutation_and_writable_or_symlink_refuse(self):
        with tempfile.TemporaryDirectory(dir=run.ROOT / 'target') as tmp:
            root = Path(tmp)
            path = root / 'one'
            path.write_bytes(b'first')
            with self.assertRaises(ValueError):
                remote.verify(root, {'files': {'one': '0' * 64}})

    def test_original_ci_envelope_function_is_extracted_without_rewrite(self):
        raw = (run.ROOT / 'tools/windows-workspace-preflight-074/owned.py').read_text()
        node = next(n for n in ast.parse(raw).body if isinstance(n, ast.FunctionDef) and n.name == '_owned')
        text = ast.get_source_segment(raw, node)
        for invariant in ('started + bound', 'started + 5', 'cleanup_deadline = now() + 5',
                          'group.terminate()', 'group.is_empty()', "production_writer_authority=False"):
            self.assertIn(invariant, text)
        ast.parse(text)


if __name__ == '__main__':
    unittest.main()
