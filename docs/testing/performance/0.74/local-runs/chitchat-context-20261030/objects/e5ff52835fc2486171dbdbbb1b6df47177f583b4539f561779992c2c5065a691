"""Exclusive Linux CI subreaper, not production all-writer authority.

Only kernel ECHILD proves retirement. The proc child list supplies candidates,
never an empty-tree proof; cleanup signals retained pidfds, never numeric PGIDs.
This dedicated single-threaded owner must remain the sole child reaper.
"""
import ctypes
import os
from pathlib import Path
import signal
import subprocess
import sys


class GroupFailure(RuntimeError):
    pass


def children():
    raw = Path('/proc/thread-self/children').read_bytes().split()
    if any(not item.isdigit() or int(item) <= 0 for item in raw) or len(set(raw)) != len(raw):
        raise GroupFailure('malformed original child census')
    return [int(item) for item in raw]


class LinuxChild:
    def __init__(self, owner, process):
        self.owner, self.process, self.pid = owner, process, process.pid
        self.reaped = False

    def poll(self):
        if self.reaped:
            return self.process.returncode
        event = os.waitid(os.P_PID, self.pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
        if event is None:
            return None
        if event.si_pid != self.pid:
            raise GroupFailure('original child wait identity differs')
        if event.si_code == os.CLD_EXITED:
            return event.si_status
        if event.si_code in (os.CLD_KILLED, os.CLD_DUMPED):
            return -event.si_status
        raise GroupFailure('unexpected terminal child wait status')

    def wait(self, timeout=None):
        if not self.owner.retired:
            raise GroupFailure('leader cannot be reaped before group retirement')
        code = self.process.wait(timeout=timeout)
        self.reaped = True
        return code

    def kill(self):
        self.owner.terminate()

    def close(self):
        if not self.reaped:
            raise GroupFailure('unreaped original child')


class LinuxGroup:
    def __init__(self):
        if (not sys.platform.startswith('linux') or not hasattr(os, 'WNOWAIT')
                or not hasattr(os, 'pidfd_open') or not hasattr(signal, 'pidfd_send_signal')):
            raise GroupFailure('Linux nonreaping wait and pidfd custody required')
        if signal.getsignal(signal.SIGCHLD) != signal.SIG_DFL:
            raise GroupFailure('exclusive default child reaper required')
        if children():
            raise GroupFailure('dedicated CI owner must have no prior children')
        self.libc = ctypes.CDLL(None, use_errno=True)
        self.libc.prctl.argtypes = [ctypes.c_int] + [ctypes.c_ulong]*4
        self.libc.prctl.restype = ctypes.c_int
        prior = ctypes.c_int()
        if self.libc.prctl(37, ctypes.addressof(prior), 0, 0, 0) != 0 or prior.value != 0:
            raise GroupFailure('fresh CI subreaper context required')
        if self.libc.prctl(36, 1, 0, 0, 0) != 0:
            raise GroupFailure('cannot enable private CI subreaper')
        self.child = None
        self.attempted = self.retired = self.closed = False
        self.terminating = False

    def spawn(self, command, cwd, log_handle, deadline):
        if self.closed or self.attempted:
            raise GroupFailure('closed or already-used CI group')
        self.attempted = True
        process = subprocess.Popen(command, cwd=cwd, stdout=log_handle,
            stderr=subprocess.STDOUT if log_handle is not None else None,
            start_new_session=True, creationflags=0)
        self.child = LinuxChild(self, process)
        return self.child

    def is_empty(self):
        if self.closed:
            raise GroupFailure('query of closed CI group')
        if self.retired or self.child is None:
            return True
        if self.terminating:
            self._stop_owned_children()
        # Bound each observation; the caller retains its original deadlines.
        # waitid reaps terminal children; ECHILD is authoritative, unlike proc.
        for _ in range(32):
            try:
                event = os.waitid(os.P_ALL, 0, os.WEXITED | os.WNOHANG)
            except ChildProcessError:
                if not self.child.reaped:
                    raise GroupFailure('original child ownership lost')
                self.retired = True
                return True
            if event is None:
                return False
            if event.si_pid == self.child.pid:
                if event.si_code == os.CLD_EXITED:
                    code = event.si_status
                elif event.si_code in (os.CLD_KILLED, os.CLD_DUMPED):
                    code = -event.si_status
                else:
                    raise GroupFailure('unexpected original child status')
                self.child.process.returncode = code
                self.child.reaped = True
        return False

    def _stop_owned_children(self):
        for pid in children():
            try:
                terminal = os.waitid(os.P_PID, pid, os.WEXITED | os.WNOHANG | os.WNOWAIT)
            except ChildProcessError:
                continue  # A stale candidate, never signal it by number.
            if terminal is not None:
                continue
            # Sole reaper + default SIGCHLD keeps this live child unreaped and
            # prevents PID reuse between the ownership test and pidfd_open.
            descriptor = os.pidfd_open(pid)
            try:
                try:
                    signal.pidfd_send_signal(descriptor, signal.SIGKILL)
                except ProcessLookupError:
                    pass  # Exit raced the retained-descriptor signal.
            finally:
                os.close(descriptor)

    def terminate(self):
        if self.closed:
            raise GroupFailure('termination of closed CI group')
        if self.retired or self.child is None:
            return
        self.terminating = True
        self._stop_owned_children()

    def close(self):
        if self.closed:
            return
        if self.child is not None and not self.retired:
            raise GroupFailure('cannot close live CI group custody')
        if self.libc.prctl(36, 0, 0, 0, 0) != 0:
            raise GroupFailure('cannot restore private CI subreaper state')
        self.closed = True
