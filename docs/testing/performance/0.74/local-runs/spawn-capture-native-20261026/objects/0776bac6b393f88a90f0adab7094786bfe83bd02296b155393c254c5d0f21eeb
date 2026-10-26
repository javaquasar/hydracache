"""Dispatch-oracle controls only: mocks never execute the package wrapper."""
from contextlib import ExitStack
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

import package_replay_v2 as replay


class ReplayV2(unittest.TestCase):
    def patches(self, stack, dirty=b''):
        stack.enter_context(patch.object(replay, 'load_document', return_value={}))
        stack.enter_context(patch.object(replay.review, 'source_review', return_value={}))
        stack.enter_context(patch.object(replay.review, 'environment_preflight', return_value={}))
        stack.enter_context(patch.object(replay, 'checked_wrappers', return_value=replay.WRAPPERS))
        stack.enter_context(patch.object(replay.subprocess, 'check_output', side_effect=['a'*40, dirty]))

    def test_default_never_dispatches_and_destination_is_first_only(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack)
            call = stack.enter_context(patch.object(replay.runpy, 'run_path', side_effect=AssertionError('no dispatch')))
            root = Path(directory)
            result = replay.run('fixture', root=root)
            self.assertEqual(result, dict(preflight_complete=True, package_dispatched=False, package_pass=False))
            call.assert_not_called()
            with self.assertRaises(FileExistsError): replay.run('fixture', root=root)

    def test_explicit_dispatch_has_only_original_argv_and_restores_sys_argv(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack)
            seen = []
            stack.enter_context(patch.object(replay.runpy, 'run_path', side_effect=lambda *a, **kw: seen.append(list(sys.argv))))
            original = sys.argv
            root = Path(directory)
            result = replay.run('fixture', root=root, execute=True)
            self.assertIs(sys.argv, original)
            self.assertEqual(seen, [[str(root/'target/run-xtask-package-780-20261024.py'), 'package-replay-v2-fixture',
                'cargo', '+1.94.0', 'test', '--offline', '--locked', '--no-fail-fast', '-p', 'xtask', '--lib', '--tests', '--', '--test-threads=4']])
            self.assertTrue(result['package_result_not_assessed'])
            self.assertFalse(result['package_pass'])

    def test_dirty_source_refuses_before_dispatch_with_saved_intent(self):
        with tempfile.TemporaryDirectory() as directory, ExitStack() as stack:
            self.patches(stack, dirty=b'?? unreviewed-source.rs')
            call = stack.enter_context(patch.object(replay.runpy, 'run_path', side_effect=AssertionError('no dispatch')))
            root = Path(directory)
            with self.assertRaisesRegex(ValueError, 'source worktree must be clean'):
                replay.run('fixture', root=root, execute=True)
            out = root/'target/reviewed-package-v2-20261025/fixture'
            self.assertTrue((out/'intent.json').is_file())
            self.assertFalse(json.loads((out/'preflight-refusal.json').read_bytes())['package_dispatched'])
            call.assert_not_called()

    def test_early_failure_is_saved_before_any_gate_or_dispatch(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(FileNotFoundError): replay.run('fixture', root=root)
            out = root/'target/reviewed-package-v2-20261025/fixture'
            self.assertTrue((out/'intent.json').is_file())
            self.assertTrue((out/'preflight-refusal.json').is_file())

    def test_wrapper_drift_and_label_injection_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture = root/'wrapper.py'; fixture.write_bytes(b'# original\n')
            with patch.object(replay, 'WRAPPERS', {'wrapper.py':replay.review.sha(fixture.read_bytes())}):
                self.assertEqual(replay.checked_wrappers(root)['wrapper.py'], replay.review.sha(fixture.read_bytes()))
                fixture.write_bytes(b'# changed bound\n')
                with self.assertRaisesRegex(ValueError, 'wrapper drift'): replay.checked_wrappers(root)
            for value in ('../outside', 'a/b', '', 'a'*65, 'UPPER'):
                with self.assertRaisesRegex(ValueError, 'bounded label'): replay.run(value, root=root)
