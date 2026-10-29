"""Unwired model-stage spawn brackets. No CLI, native run or production hook."""
import ctypes as c
from ctypes import wintypes as w
from pathlib import Path
import subprocess
import sys
import time

from capture_leaves import LeafAccountingJob, LeafTape
from coordinator_frames import Tape
from git_job_cpu import AccountingApi, require, validate
from watchdog_windows_job import ProcessInfo, StartupInfoEx, WindowsChild, Win32Failure

PHASES = ('preparation', 'create_to_retention', 'membership_release')


def contract():
    return dict(schema='hc074-spawn-frames-model-v1', phases=list(PHASES),
        max_additional_probes=6, max_additional_clock_reads=18,
        units='ns', wall_clock='time.monotonic_ns', cpu_clock='time.process_time_ns',
        cpu_scope='coordinator process-wide, all threads; not child or native Git CPU',
        wall_bounds_scope='elapsed between bracketed CPU-read instants, not effect-only duration',
        successful_create_to_retention_has_no_added_probe=True,
        failed_create_last_error_read_unchanged=True, cleanup_instrumented=False,
        complete_frame_does_not_prove_resume=True,
        pure_create_cost_isolated=False, existing_predicates_removed_or_reordered=False,
        window_renewed=False, caller_connected=False, native_control_executed=False,
        overhead_accepted=False, package_pass=False)


class SpawnTape(LeafTape):
    def __init__(self, **clocks):
        Tape.__init__(self, **clocks)
        self.group, self.phases = 'spawn', PHASES

    def finish(self, operation_passed):
        return dict(contract=contract(), **super().finish(operation_passed))


class SpawnAccountingJob(LeafAccountingJob):
    def __init__(self, api, observer, image, leaves, spawn_frames):
        require(type(spawn_frames) is SpawnTape, 'explicit spawn frame owner')
        self.spawn_frames = spawn_frames
        super().__init__(api, observer, image, leaves)

    @LeafAccountingJob.child.setter
    def child(self, value):
        require(getattr(self, '_child', None) is None, 'original worker cannot be replaced')
        self._child = value  # No added observation before original retention.
        if value is not None:
            self.spawn_frames.end('create_to_retention')
            self.leaves.begin('owner', 'owner_identity')
            pid, path = self.observer.identity(value.handle)
            require(pid == value.pid and Path(path).resolve() == self.image, 'original worker identity drift')
            self.worker_identity = (pid, path)
            self.leaves.end('owner', 'owner_identity')
            self.leaves.begin('owner', 'owner_initial')
            first = self.observer.read(value.handle)
            validate(None, first)
            require(first['terminal'] is False, 'original worker exited before release')
            self.worker_start = first
            self.leaves.end('owner', 'owner_initial')


class SpawnAccountingApi(AccountingApi):
    def spawn(self, owner, command, cwd, log_handle, deadline):
        import msvcrt
        inherited = []
        attributes = None
        initialized = False
        try:
            owner.spawn_frames.begin('preparation')
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
            startup.info.flags = 0x100
            startup.info.stdin, startup.info.stdout, startup.info.stderr = standard
            startup.attributes = c.cast(attributes, c.c_void_p)
            info = ProcessInfo()
            line = c.create_unicode_buffer(subprocess.list2cmdline(command))
            flags = 0x4 | 0x200 | 0x80000
            owner.spawn_frames.end('preparation')
            owner.spawn_frames.begin('create_to_retention')
            if not self.dll.CreateProcessW(None, line, None, None, bool(listed), flags,
                                          None, str(cwd) if cwd is not None else None,
                                          c.byref(startup), c.byref(info)):
                raise c.WinError(c.get_last_error())
            child = WindowsChild(self, info.process, info.thread, info.pid)
            owner.child = child
            owner.spawn_frames.begin('membership_release')
            present = w.BOOL()
            self.require(self.dll.IsProcessInJob(child.handle, owner.handle, c.byref(present)), 'IsProcessInJob')
            if not present.value:
                raise Win32Failure('created child is outside its original Job')
            if time.monotonic() < deadline:
                previous = self.dll.ResumeThread(child.thread)
                if previous != 1:
                    raise Win32Failure(f'ResumeThread: unexpected suspend count {previous}')
            owner.spawn_frames.end('membership_release')
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
