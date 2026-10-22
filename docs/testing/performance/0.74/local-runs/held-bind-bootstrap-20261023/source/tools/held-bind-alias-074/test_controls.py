import ctypes
import errno
import unittest
from pathlib import Path
from unittest.mock import Mock, patch
import cleanup
import check as c
with patch('ctypes.CDLL'):
    import probe


class Controls(unittest.TestCase):
    def test_check_lanes_cannot_repeat_the_mount_or_original_controls(self):
        for lane in ('host', '194', '188', 'metadata'):
            for _, argv in c.commands(lane):
                self.assertFalse(set(argv) & {'--ignored', 'unshare', 'ssh', 'systemctl', 'probe.py', 'regular_control.py'})
        with self.assertRaises(ValueError): c.commands('server')

    def test_errno_from_failed_syscall_is_not_a_success_or_fallback(self):
        with patch.object(probe.libc, 'syscall', return_value=-1) as call:
            ctypes.set_errno(errno.EINVAL)
            with self.assertRaisesRegex(OSError, 'Invalid argument'): probe.syscall(428, ctypes.c_int(3))
            call.assert_called_once()

    def test_cleanup_does_not_depend_on_ismount_and_uses_exact_leaf(self):
        parent = Path('/tmp/hc074-bind-regular-control-synthetic')
        libc = Mock()
        libc.umount2.return_value = 0
        with patch('os.path.ismount', return_value=False) as detect:
            cleanup.unmount_owned_leaf(parent / 'launcher', parent, libc)
            detect.assert_not_called()
        libc.umount2.assert_called_once()
        self.assertEqual(libc.umount2.call_args.args[0].value, bytes(str(parent / 'launcher'), 'utf-8'))

    def test_cleanup_errno_and_foreign_targets_refuse(self):
        parent = Path('/tmp/hc074-bind-regular-control-synthetic')
        libc = Mock()
        libc.umount2.return_value = -1
        ctypes.set_errno(errno.EINVAL)
        cleanup.unmount_owned_leaf(parent / 'launcher', parent, libc)
        ctypes.set_errno(errno.EPERM)
        with self.assertRaises(OSError): cleanup.unmount_owned_leaf(parent / 'launcher', parent, libc)
        for target, root in [('/tmp/other/launcher', '/tmp/other'), ('/launcher', '/'),
                             (str(parent / 'other'), str(parent)), ('/tmp/other/launcher', str(parent))]:
            libc.reset_mock()
            with self.assertRaises(ValueError): cleanup.unmount_owned_leaf(target, root, libc)
            libc.umount2.assert_not_called()


if __name__ == '__main__': unittest.main()
