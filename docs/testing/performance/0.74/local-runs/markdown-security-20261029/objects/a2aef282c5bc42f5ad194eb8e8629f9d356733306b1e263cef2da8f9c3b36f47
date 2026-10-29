"""Owned kernel counterexamples for process-global FD/EOF test assumptions.

These controls neither authenticate an owner nor prove all-writer retirement.
They demonstrate possible interference; they do not attribute a saved race.
"""
import errno
import os
import select
import signal
import sys
import tempfile
import threading
import unittest


@unittest.skipUnless(sys.platform == 'linux', 'requires local Linux procfs/fork/pidfd')
class DescriptorOracleControls(unittest.TestCase):
    def test_cloexec_writer_survives_fork_until_exact_owned_child_exits(self):
        reader, writer = os.pipe2(os.O_NONBLOCK | os.O_CLOEXEC)
        release_r, release_w = os.pipe2(os.O_CLOEXEC)
        ready_r, ready_w = os.pipe2(os.O_CLOEXEC)
        pid = os.fork()
        if pid == 0:
            try:
                for fd in (reader, release_w, ready_r):
                    os.close(fd)
                os.write(ready_w, b'R')
                if os.read(release_r, 1) != b'X':
                    os._exit(21)
                os._exit(0)
            except BaseException:
                os._exit(22)
        pidfd = os.pidfd_open(pid)
        reaped = False
        try:
            os.close(release_r)
            os.close(ready_w)
            ready = select.poll()
            ready.register(ready_r, select.POLLIN)
            self.assertTrue(ready.poll(5000), 'owned child did not reach the barrier')
            self.assertEqual(os.read(ready_r, 1), b'R')
            self.assertFalse(os.get_inheritable(writer))
            os.close(writer)
            writer = -1
            with self.assertRaises(BlockingIOError) as refusal:
                os.read(reader, 1)
            self.assertEqual(refusal.exception.errno, errno.EAGAIN)
            os.write(release_w, b'X')
            terminal = select.poll()
            terminal.register(pidfd, select.POLLIN)
            self.assertTrue(terminal.poll(5000), 'owned child failed to exit')
            observed, status = os.waitpid(pid, 0)
            reaped = True
            self.assertEqual(observed, pid)
            self.assertTrue(os.WIFEXITED(status))
            self.assertEqual(os.WEXITSTATUS(status), 0)
            self.assertEqual(os.read(reader, 1), b'')
        finally:
            if not reaped:
                # Emergency owned-child cleanup cannot count as positive proof.
                signal.pidfd_send_signal(pidfd, signal.SIGKILL)
                os.waitpid(pid, 0)
            for fd in (reader, writer, release_w, ready_r, pidfd):
                if fd >= 0:
                    os.close(fd)

    def test_same_thread_fd_directory_counts_other_threads_unrelated_resources(self):
        opened, close, closed = (threading.Event() for _ in range(3))
        errors = []
        def unrelated_thread():
            try:
                files = [tempfile.TemporaryFile() for _ in range(4)]
                opened.set()
                if not close.wait(5):
                    raise AssertionError('parent did not release the owned thread')
                for file in files:
                    file.close()
            except BaseException as error:
                errors.append(error)
                opened.set()
            finally:
                closed.set()
        thread = threading.Thread(target=unrelated_thread)
        thread.start()
        try:
            self.assertTrue(opened.wait(5))
            self.assertFalse(errors)
            before = len(os.listdir('/proc/thread-self/fd'))
            close.set()
            self.assertTrue(closed.wait(5))
            thread.join(5)
            self.assertFalse(thread.is_alive())
            self.assertFalse(errors)
            after = len(os.listdir('/proc/thread-self/fd'))
            self.assertEqual(before - after, 4)
        finally:
            close.set()
            thread.join(5)


if __name__ == '__main__':
    unittest.main(verbosity=2)
