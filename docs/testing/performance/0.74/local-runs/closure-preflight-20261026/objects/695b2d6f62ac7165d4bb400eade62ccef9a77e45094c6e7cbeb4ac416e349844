"""Deterministic ownership/retirement controls, including non-Linux hosts."""
import importlib.util
from pathlib import Path
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch

SPEC = importlib.util.spec_from_file_location('linux_group_models', Path(__file__).with_name('watchdog_linux_group.py'))
group = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(group)


class Ownership(unittest.TestCase):
    def setUp(self):
        self.constants = patch.multiple(group.os, P_PID=1, P_ALL=0, WEXITED=4,
            WNOHANG=1, WNOWAIT=0x1000000, CLD_EXITED=1, CLD_KILLED=2,
            CLD_DUMPED=3, create=True)
        self.constants.start()
        self.addCleanup(self.constants.stop)
        self.signals = patch.object(group.signal, 'SIGKILL', 9, create=True)
        self.signals.start()
        self.addCleanup(self.signals.stop)
        self.owner = object.__new__(group.LinuxGroup)
        self.owner.closed = self.owner.retired = self.owner.terminating = False
        self.owner.libc = SimpleNamespace(prctl=Mock(return_value=0))
        self.owner.child = group.LinuxChild(self.owner, SimpleNamespace(pid=91, returncode=None))

    def event(self, pid=91, code=1, status=23):
        return SimpleNamespace(si_pid=pid, si_code=code, si_status=status)

    def test_only_kernel_echild_after_original_reap_retires(self):
        with patch.object(group.os, 'waitid', side_effect=[self.event(), ChildProcessError()], create=True), \
                patch.object(group, 'children', side_effect=AssertionError('not an empty proof')):
            self.assertTrue(self.owner.is_empty())
        self.assertEqual(self.owner.child.poll(), 23)
        self.assertTrue(self.owner.child.reaped)

    def test_echild_before_original_reap_is_refused(self):
        with patch.object(group.os, 'waitid', side_effect=ChildProcessError(), create=True):
            with self.assertRaisesRegex(group.GroupFailure, 'ownership lost'):
                self.owner.is_empty()
        self.assertFalse(self.owner.retired)

    def test_nonterminal_kernel_result_is_not_retirement(self):
        with patch.object(group.os, 'waitid', return_value=None, create=True):
            self.assertFalse(self.owner.is_empty())
        self.assertFalse(self.owner.retired)

    def test_reaping_is_bounded_per_observation(self):
        with patch.object(group.os, 'waitid', return_value=self.event(92), create=True) as wait:
            self.assertFalse(self.owner.is_empty())
        self.assertEqual(wait.call_count, 32)

    def test_candidates_require_child_ownership_and_retained_pidfd(self):
        calls = []
        with patch.object(group, 'children', return_value=[91, 92, 93]), \
                patch.object(group.os, 'waitid', side_effect=[ChildProcessError(), self.event(92), None], create=True), \
                patch.object(group.os, 'pidfd_open', side_effect=lambda pid: calls.append(('open', pid)) or 17, create=True), \
                patch.object(group.signal, 'pidfd_send_signal', side_effect=lambda fd, sig: calls.append(('signal', fd)), create=True), \
                patch.object(group.os, 'close', side_effect=lambda fd: calls.append(('close', fd))), \
                patch.object(group.os, 'killpg', side_effect=AssertionError('numeric PGID'), create=True):
            self.owner.terminate()
        self.assertTrue(self.owner.terminating)
        self.assertEqual(calls, [('open', 93), ('signal', 17), ('close', 17)])

    def test_signal_failure_still_closes_descriptor(self):
        with patch.object(group, 'children', return_value=[91]), \
                patch.object(group.os, 'waitid', return_value=None, create=True), \
                patch.object(group.os, 'pidfd_open', return_value=17, create=True), \
                patch.object(group.signal, 'pidfd_send_signal', side_effect=PermissionError(), create=True), \
                patch.object(group.os, 'close') as close:
            with self.assertRaises(PermissionError):
                self.owner.terminate()
        close.assert_called_once_with(17)

    def test_live_custody_cannot_close_or_reap(self):
        with self.assertRaises(group.GroupFailure):
            self.owner.close()
        with self.assertRaises(group.GroupFailure):
            self.owner.child.wait(1)
        self.owner.libc.prctl.assert_not_called()

    def test_retired_custody_restores_private_subreaper_once(self):
        self.owner.retired = True
        self.owner.close()
        self.owner.close()
        self.owner.libc.prctl.assert_called_once_with(36, 0, 0, 0, 0)


if __name__ == '__main__':
    unittest.main()
