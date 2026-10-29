"""One-dispatch/refusal controls for v6; never launches a real package."""
from contextlib import ExitStack
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import package_replay_v6 as replay


class ReplayV6(unittest.TestCase):
    def patches(self, stack, dirty=b''):
        stack.enter_context(patch.object(replay, 'load_document', return_value={}))
        stack.enter_context(patch.object(replay.review, 'source_review', return_value={}))
        stack.enter_context(patch.object(replay.review, 'environment_preflight', return_value={}))
        stack.enter_context(patch.object(replay, 'checked_wrappers', return_value={}))
        stack.enter_context(patch.object(replay.subprocess, 'check_output', side_effect=['a' * 40, dirty]))

    def test_default_and_exclusive_destination_do_not_dispatch(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack)
            call = stack.enter_context(patch.object(replay.runpy, 'run_path'))
            root = Path(directory)
            self.assertFalse(replay.run('fixture', root=root)['package_dispatched'])
            call.assert_not_called()
            with self.assertRaises(FileExistsError):
                replay.run('fixture', root=root, execute=True)

    def test_single_dispatch_keeps_original_command_wrappers_and_expanded_dirty_scope(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack)
            seen = []
            stack.enter_context(patch.object(replay.runpy, 'run_path', side_effect=lambda *a, **k: seen.append(list(sys.argv))))
            original = sys.argv
            root = Path(directory)
            result = replay.run('fixture', root=root, execute=True)
            self.assertIs(sys.argv, original)
            self.assertEqual(seen, [[str(root / 'target/run-xtask-package-780-20261024.py'), 'package-v6-fixture',
                                    'cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast',
                                    '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']])
            self.assertFalse(result['package_pass'])
            intent = json.loads((root / 'target/reviewed-package-v6-20261026/fixture/intent.json').read_bytes())
            self.assertEqual((intent['package_bound_seconds'], intent['case_threads']), (780, 4))
            self.assertFalse(intent['observer'])
            dirty_argv = replay.subprocess.check_output.call_args_list[1].args[0]
            self.assertIn('scripts/ci', dirty_argv)
            self.assertIn('.gitattributes', dirty_argv)

    def test_dirty_source_retains_refusal_without_dispatch(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack, b'?? scripts/ci/unreviewed.py')
            call = stack.enter_context(patch.object(replay.runpy, 'run_path'))
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'source worktree must be clean'):
                replay.run('fixture', root=root, execute=True)
            out = root / 'target/reviewed-package-v6-20261026/fixture'
            self.assertFalse(json.loads((out / 'preflight-refusal.json').read_bytes())['package_dispatched'])
            call.assert_not_called()

    def test_missing_input_and_syntax_drift_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(FileNotFoundError):
                replay.run('fixture', root=root)
            self.assertTrue((root / 'target/reviewed-package-v6-20261026/fixture/preflight-refusal.json').is_file())
            path = root / replay.SYNTAX
            path.parent.mkdir(parents=True)
            path.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'syntax artifact drift'):
                replay.load_document(root)

    def test_bad_labels_and_wrapper_drift_refuse(self):
        for label in ('../outside', '', 'a' * 65, 'UPPER'):
            with self.assertRaisesRegex(ValueError, 'bounded label'):
                replay.run(label)
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(FileNotFoundError):
                replay.checked_wrappers(Path(directory))

    def test_environment_or_review_failure_never_dispatches(self):
        for name in ('source_review', 'environment_preflight'):
            with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
                self.patches(stack)
                stack.enter_context(patch.object(replay.review, name, side_effect=ValueError('original refusal')))
                call = stack.enter_context(patch.object(replay.runpy, 'run_path'))
                with self.assertRaisesRegex(ValueError, 'original refusal'):
                    replay.run('fixture', root=Path(directory), execute=True)
                call.assert_not_called()


if __name__ == '__main__':
    unittest.main()
