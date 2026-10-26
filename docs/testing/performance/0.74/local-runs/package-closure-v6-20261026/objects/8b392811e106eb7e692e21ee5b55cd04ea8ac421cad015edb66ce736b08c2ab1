"""Private CI containment, not production writer authority. Windows 10+ only.

Job-list creation closes the suspended-create/assign owner-loss gap. No PID reopen,
taskkill, breakaway or fallback to an uncontained child is permitted.
"""
import ctypes as c
from ctypes import wintypes as w
import math
import os
import subprocess
import sys
import time


class Win32Failure(RuntimeError):
    pass


class BasicLimits(c.Structure):
    _fields_ = [('process_time', c.c_longlong), ('job_time', c.c_longlong),
        ('flags', w.DWORD), ('min_working_set', c.c_size_t), ('max_working_set', c.c_size_t),
        ('active_limit', w.DWORD), ('affinity', c.c_size_t), ('priority', w.DWORD), ('scheduling', w.DWORD)]


class IoCounters(c.Structure):
    _fields_ = [(name, c.c_ulonglong) for name in ['read_ops', 'write_ops', 'other_ops',
                                                'read_bytes', 'write_bytes', 'other_bytes']]


class ExtendedLimits(c.Structure):
    _fields_ = [('basic', BasicLimits), ('io', IoCounters), ('process_memory', c.c_size_t),
        ('job_memory', c.c_size_t), ('peak_process_memory', c.c_size_t), ('peak_job_memory', c.c_size_t)]


class Accounting(c.Structure):
    _fields_ = [(name, c.c_longlong) for name in ['user_time', 'kernel_time', 'period_user', 'period_kernel']] + [
        (name, w.DWORD) for name in ['page_faults', 'total', 'active', 'terminated']]


class StartupInfo(c.Structure):
    _fields_ = [('cb', w.DWORD), ('reserved', w.LPWSTR), ('desktop', w.LPWSTR), ('title', w.LPWSTR)] + [
        (name, w.DWORD) for name in ['x', 'y', 'xsize', 'ysize', 'xchars', 'ychars', 'fill', 'flags']] + [
        ('show', w.WORD), ('reserved_size', w.WORD), ('reserved_bytes', c.c_void_p),
        ('stdin', w.HANDLE), ('stdout', w.HANDLE), ('stderr', w.HANDLE)]


class StartupInfoEx(c.Structure):
    _fields_ = [('info', StartupInfo), ('attributes', c.c_void_p)]


class ProcessInfo(c.Structure):
    _fields_ = [('process', w.HANDLE), ('thread', w.HANDLE), ('pid', w.DWORD), ('tid', w.DWORD)]


class Api:
    def __init__(self):
        if os.name != 'nt':
            raise Win32Failure('Windows-only Job containment')
        self.dll = c.WinDLL('kernel32', use_last_error=True)
        handle, pointer, size = w.HANDLE, c.c_void_p, c.c_size_t
        declarations = {
            'CreateJobObjectW': ([pointer, w.LPCWSTR], handle),
            'SetInformationJobObject': ([handle, c.c_int, pointer, w.DWORD], w.BOOL),
            'QueryInformationJobObject': ([handle, c.c_int, pointer, w.DWORD, pointer], w.BOOL),
            'TerminateJobObject': ([handle, w.UINT], w.BOOL),
            'CloseHandle': ([handle], w.BOOL),
            'InitializeProcThreadAttributeList': ([pointer, w.DWORD, w.DWORD, c.POINTER(size)], w.BOOL),
            'UpdateProcThreadAttribute': ([pointer, w.DWORD, size, pointer, size, pointer, pointer], w.BOOL),
            'DeleteProcThreadAttributeList': ([pointer], None),
            'GetCurrentProcess': ([], handle),
            'GetStdHandle': ([w.DWORD], handle),
            'GetFileType': ([handle], w.DWORD),
            'DuplicateHandle': ([handle, handle, handle, c.POINTER(handle), w.DWORD, w.BOOL, w.DWORD], w.BOOL),
            'CreateProcessW': ([w.LPCWSTR, w.LPWSTR, pointer, pointer, w.BOOL, w.DWORD,
                                pointer, w.LPCWSTR, pointer, c.POINTER(ProcessInfo)], w.BOOL),
            'IsProcessInJob': ([handle, handle, c.POINTER(w.BOOL)], w.BOOL),
            'ResumeThread': ([handle], w.DWORD),
            'WaitForSingleObject': ([handle, w.DWORD], w.DWORD),
            'GetExitCodeProcess': ([handle, c.POINTER(w.DWORD)], w.BOOL),
            'TerminateProcess': ([handle, w.UINT], w.BOOL),
        }
        for name, (args, result) in declarations.items():
            fn = getattr(self.dll, name)
            fn.argtypes, fn.restype = args, result

    @staticmethod
    def require(ok, operation):
        if not ok:
            raise Win32Failure(f'{operation}: {c.WinError(c.get_last_error())}')

    def close(self, handle):
        self.require(self.dll.CloseHandle(handle), 'CloseHandle')

    def create_job(self):
        handle = self.dll.CreateJobObjectW(None, None)
        self.require(handle, 'CreateJobObjectW')
        return handle

    def configure(self, handle):
        limits = ExtendedLimits()
        limits.basic.flags = 0x2000  # KILL_ON_JOB_CLOSE, no breakaway/inheritance.
        self.require(self.dll.SetInformationJobObject(handle, 9, c.byref(limits), c.sizeof(limits)),
                     'SetInformationJobObject')

    def empty(self, handle):
        info = Accounting()
        self.require(self.dll.QueryInformationJobObject(handle, 1, c.byref(info), c.sizeof(info), None),
                     'QueryInformationJobObject')
        return info.active == 0

    def terminate(self, handle):
        self.require(self.dll.TerminateJobObject(handle, 124), 'TerminateJobObject')

    def spawn(self, owner, command, cwd, log_handle, deadline):
        import msvcrt
        inherited = []
        attributes = None
        initialized = False
        try:
            current = self.dll.GetCurrentProcess()
            standard = []
            for number in [-10, -11, -12]:
                original = (msvcrt.get_osfhandle(log_handle.fileno())
                            if log_handle is not None and number != -10 else self.dll.GetStdHandle(number))
                if original in (None, 0, c.c_void_p(-1).value):
                    standard.append(None)
                    continue
                duplicate = w.HANDLE()
                self.require(self.dll.DuplicateHandle(current, original, current, c.byref(duplicate),
                                                       0, True, 2), 'DuplicateHandle(stdio)')
                inherited.append(duplicate.value)
                standard.append(duplicate.value)
            # Match CPython's exclusion of legacy special console handles from
            # HANDLE_LIST; they remain selected standard handles in STARTUPINFO.
            listed = [h for h in inherited if h & 3 != 3 or self.dll.GetFileType(h) != 2]
            count = 2 if listed else 1
            size = c.c_size_t()
            ok = self.dll.InitializeProcThreadAttributeList(None, count, 0, c.byref(size))
            if ok or c.get_last_error() != 122 or not size.value:
                raise Win32Failure('attribute-list sizing did not report INSUFFICIENT_BUFFER')
            attributes = c.create_string_buffer(size.value)
            self.require(self.dll.InitializeProcThreadAttributeList(attributes, count, 0, c.byref(size)),
                         'InitializeProcThreadAttributeList')
            initialized = True
            jobs = (w.HANDLE * 1)(owner.handle)
            self.require(self.dll.UpdateProcThreadAttribute(attributes, 0, 0x2000D, jobs, c.sizeof(jobs), None, None),
                         'UpdateProcThreadAttribute(JOB_LIST)')
            handles = (w.HANDLE * len(listed))(*listed)
            if listed:
                self.require(self.dll.UpdateProcThreadAttribute(attributes, 0, 0x20002, handles,
                             c.sizeof(handles), None, None), 'UpdateProcThreadAttribute(HANDLE_LIST)')
            startup = StartupInfoEx()
            startup.info.cb = c.sizeof(startup)
            startup.info.flags = 0x100  # STARTF_USESTDHANDLES
            startup.info.stdin, startup.info.stdout, startup.info.stderr = standard
            startup.attributes = c.cast(attributes, c.c_void_p)
            info = ProcessInfo()
            line = c.create_unicode_buffer(subprocess.list2cmdline(command))
            flags = 0x4 | 0x200 | 0x80000  # suspended, new process group, extended startup.
            if not self.dll.CreateProcessW(None, line, None, None, bool(listed), flags,
                                          None, str(cwd) if cwd is not None else None,
                                          c.byref(startup), c.byref(info)):
                raise c.WinError(c.get_last_error())  # Preserve missing-program classification.
            child = WindowsChild(self, info.process, info.thread, info.pid)
            owner.child = child  # Retain even if any later setup/release step fails.
            present = w.BOOL()
            self.require(self.dll.IsProcessInJob(child.handle, owner.handle, c.byref(present)), 'IsProcessInJob')
            if not present.value:
                raise Win32Failure('created child is outside its original Job')
            if time.monotonic() < deadline:
                previous = self.dll.ResumeThread(child.thread)
                if previous != 1:
                    raise Win32Failure(f'ResumeThread: unexpected suspend count {previous}')
            return child
        finally:
            original_error = sys.exc_info()[1]
            if initialized:
                self.dll.DeleteProcThreadAttributeList(attributes)
            errors = []
            for handle in reversed(inherited):
                try:
                    self.close(handle)
                except Exception as error:
                    errors.append(str(error))
            if errors:
                raise Win32Failure(f'{original_error or "setup"}; release stdio duplicates: ' + '; '.join(errors))


class WindowsChild:
    def __init__(self, api, handle, thread, pid):
        self.api, self.handle, self.thread, self.pid = api, handle, thread, pid

    def poll(self):
        if self.handle is None:
            raise Win32Failure('child handle already closed')
        status = self.api.dll.WaitForSingleObject(self.handle, 0)
        if status == 258:
            return None
        if status != 0:
            raise Win32Failure(f'WaitForSingleObject: {status}')
        code = w.DWORD()
        self.api.require(self.api.dll.GetExitCodeProcess(self.handle, c.byref(code)), 'GetExitCodeProcess')
        return code.value

    def wait(self, timeout=None):
        if timeout is None or not math.isfinite(timeout) or timeout < 0:
            raise Win32Failure('finite nonnegative owned-child wait required')
        status = self.api.dll.WaitForSingleObject(self.handle, min(0xFFFFFFFE, math.ceil(timeout * 1000)))
        if status == 258:
            raise subprocess.TimeoutExpired('owned CI child', timeout)
        if status != 0:
            raise Win32Failure(f'WaitForSingleObject: {status}')
        return self.poll()

    def kill(self):
        if self.handle is None:
            raise Win32Failure('termination of closed child handle')
        self.api.require(self.api.dll.TerminateProcess(self.handle, 124), 'TerminateProcess(original child)')

    def close(self):
        errors = []
        for name in ['thread', 'handle']:
            handle = getattr(self, name)
            if handle is not None:
                try:
                    self.api.close(handle)
                    setattr(self, name, None)
                except Exception as error:
                    errors.append(str(error))
        if errors:
            raise Win32Failure('release child handles: ' + '; '.join(errors))


class WindowsJob:
    def __init__(self, api=None):
        self.api = api if api is not None else Api()
        self.child = None
        self.attempted = False
        self.handle = self.api.create_job()
        try:
            self.api.configure(self.handle)
        except BaseException:
            self.close()
            raise

    def spawn(self, command, cwd, log_handle, deadline):
        if self.handle is None or self.attempted:
            raise Win32Failure('closed or already-used CI Job')
        self.attempted = True
        return self.api.spawn(self, command, cwd, log_handle, deadline)

    def is_empty(self):
        if self.handle is None:
            raise Win32Failure('query of closed CI Job')
        return self.api.empty(self.handle)

    def terminate(self):
        if self.handle is None:
            raise Win32Failure('termination of closed CI Job')
        self.api.terminate(self.handle)

    def close(self):
        if self.handle is not None:
            self.api.close(self.handle)
            self.handle = None
