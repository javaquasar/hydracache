"""No real package dispatch: exact argv, exclusive destinations and early refusals."""
from contextlib import ExitStack
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch

import package_replay_v7 as replay


class ReplayV7(unittest.TestCase):
    def patches(self, stack, dirty=b''):
        review = SimpleNamespace(source_review=lambda *a: {}, environment_preflight=lambda *a: {})
        stack.enter_context(patch.object(replay, 'load_review', return_value=review))
        stack.enter_context(patch.object(replay, 'load_document', return_value={}))
        stack.enter_context(patch.object(replay, 'checked_wrappers', return_value={}))
        stack.enter_context(patch.object(replay.subprocess, 'check_output', side_effect=['a' * 40, dirty]))
        return review

    def test_default_and_exclusive_destination(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack)
            call = stack.enter_context(patch.object(replay.runpy, 'run_path'))
            root = Path(directory)
            self.assertFalse(replay.run('fixture', root=root)['package_dispatched'])
            call.assert_not_called()
            with self.assertRaises(FileExistsError):
                replay.run('fixture', root=root, execute=True)

    def test_one_original_argv_and_restored_interpreter_arguments(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack)
            seen = []
            stack.enter_context(patch.object(replay.runpy, 'run_path', side_effect=lambda *a, **k: seen.append(list(sys.argv))))
            root = Path(directory)
            original = sys.argv
            self.assertFalse(replay.run('fixture', root=root, execute=True)['package_pass'])
            self.assertIs(sys.argv, original)
            self.assertEqual(seen, [[str(root / 'target/run-xtask-package-780-20261024.py'),
                                    'package-v7-fixture', 'cargo', '+1.94.0', 'test', '--offline', '--locked',
                                    '--no-fail-fast', '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']])
            out = root / 'target/reviewed-package-v7-20261026/fixture'
            intent = json.loads((out / 'intent.json').read_bytes())
            self.assertEqual((intent['package_bound_seconds'], intent['case_threads']), (780, 4))
            self.assertFalse(intent['retry'])
            for name in ('scripts/ci', '.gitattributes', 'tools'):
                self.assertIn(name, replay.subprocess.check_output.call_args_list[1].args[0])

    def test_preflight_refusals_never_dispatch(self):
        for stage in ('load_review', 'load_document', 'checked_wrappers', 'source_review', 'environment_preflight'):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
                review = self.patches(stack)
                target = review if stage in ('source_review', 'environment_preflight') else replay
                stack.enter_context(patch.object(target, stage, side_effect=ValueError('first refusal')))
                call = stack.enter_context(patch.object(replay.runpy, 'run_path'))
                root = Path(directory)
                with self.assertRaisesRegex(ValueError, 'first refusal'):
                    replay.run('fixture', root=root, execute=True)
                out = root / 'target/reviewed-package-v7-20261026/fixture'
                self.assertFalse(json.loads((out / 'preflight-refusal.json').read_bytes())['package_dispatched'])
                call.assert_not_called()

    def test_dirty_input_never_dispatches(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack, b'?? scripts/ci/unreviewed.py')
            call = stack.enter_context(patch.object(replay.runpy, 'run_path'))
            with self.assertRaisesRegex(ValueError, 'source worktree must be clean'):
                replay.run('fixture', root=Path(directory), execute=True)
            call.assert_not_called()

    def test_review_bytes_are_checked_before_import(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / 'tools/historical-blob-batch-074/package_effects_review_v7.py'
            path.parent.mkdir(parents=True)
            path.write_text('raise RuntimeError("must not execute")')
            with patch.object(replay.importlib.util, 'spec_from_file_location') as imported:
                with self.assertRaisesRegex(ValueError, 'implementation drift'):
                    replay.load_review(root)
                imported.assert_not_called()

    def test_fixed_review_and_syntax_load(self):
        review = replay.load_review(replay.ROOT)
        self.assertEqual(review.document_digest(replay.load_document(replay.ROOT)), review.REVIEWED_AST_SHA256)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            path = root / replay.SYNTAX
            path.parent.mkdir(parents=True)
            path.write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'syntax artifact drift'):
                replay.load_document(root)

    def test_each_wrapper_is_checked(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in replay.WRAPPERS:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b'changed')
            with self.assertRaisesRegex(ValueError, 'wrapper drift'):
                replay.checked_wrappers(root)

    def test_invalid_label_before_output_and_unknown_cli_before_dispatch(self):
        for label in ('../outside', '', 'a' * 65, 'UPPER'):
            with self.assertRaisesRegex(ValueError, 'bounded label'):
                replay.run(label)
        result = subprocess.run([sys.executable, str(Path(replay.__file__)), 'unused', '--timeout', '900'],
                                capture_output=True, timeout=10)
        self.assertEqual(result.returncode, 2)
        self.assertIn(b'unrecognized arguments', result.stderr)


if __name__ == '__main__':
    unittest.main()
