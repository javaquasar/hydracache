"""Read-only artifact and current-process controls; no target replay."""
import ctypes
import os
from pathlib import Path
import unittest
from unittest import mock
import tempfile

import capture


class CaptureTests(unittest.TestCase):
    def test_exact_saved_case_catalog(self):
        self.assertEqual(len(capture.expected_cases()),321)

    def test_artifact_is_the_fresh_receipted_target(self):
        path = capture.artifact()
        self.assertEqual(path.parent,(capture.ROOT/"target/debug/deps").resolve())
        self.assertTrue(path.name.startswith("performance_contract_074-"))

    @unittest.skipUnless(os.name == "nt", "Windows GetProcessTimes control")
    def test_current_process_handle_remains_same_and_cpu_does_not_go_backwards(self):
        api = ctypes.WinDLL("kernel32",use_last_error=True)
        api.GetCurrentProcess.restype = ctypes.c_void_p
        handle = api.GetCurrentProcess()
        before, after = capture.cpu_times(handle), capture.cpu_times(handle)
        self.assertEqual(before["created_filetime"],after["created_filetime"])
        self.assertEqual(after["exited_filetime"],0)
        self.assertGreaterEqual(after["kernel_ns"],before["kernel_ns"])
        self.assertGreaterEqual(after["user_ns"],before["user_ns"])
        self.assertTrue(after["excludes_children"])

    def test_scope_and_original_bound_are_fixed(self):
        self.assertEqual(capture.BOUND,780)
        self.assertEqual(capture.OUT,capture.ROOT/"target/static-contract-trace-20261024/first")
        raw = Path(capture.__file__).read_text()
        self.assertNotIn("workflow_dispatch",raw)
        self.assertNotIn("systemctl",raw)
        self.assertNotIn("ssh",raw)
        self.assertNotIn("--ignored",raw)
        self.assertNotIn("--nocapture",raw)

    def test_cleanup_never_signals_an_already_completed_process(self):
        child = mock.Mock()
        child.poll.return_value = 0
        with mock.patch.object(capture.subprocess,"run") as run:
            capture.stop_owned(child)
        run.assert_not_called()
        child.kill.assert_not_called()

    def test_cleanup_uses_only_owned_pid_then_exact_handle_fallback(self):
        child = mock.Mock(pid=74)
        child.poll.return_value = None
        with tempfile.TemporaryDirectory() as temp, mock.patch.object(capture,"OUT",Path(temp)), \
             mock.patch.object(capture.subprocess,"run",return_value=mock.Mock(stdout=b"kept",stderr=b"")) as run:
            capture.stop_owned(child)
        self.assertEqual(run.call_args.args[0],["taskkill","/PID","74","/T","/F"])
        self.assertEqual(run.call_args.kwargs["timeout"],30)
        child.kill.assert_called_once_with()
        child.wait.assert_called_once_with(timeout=30)

    def test_cleanup_handle_fallback_survives_taskkill_exception(self):
        child = mock.Mock(pid=74)
        child.poll.return_value = None
        with mock.patch.object(capture.subprocess,"run",side_effect=OSError("fixture")), self.assertRaises(OSError):
            capture.stop_owned(child)
        child.kill.assert_called_once_with()
        child.wait.assert_called_once_with(timeout=30)


if __name__ == "__main__":
    unittest.main()
