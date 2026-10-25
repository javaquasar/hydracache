"""Read-only counters for one owned Windows child; never a tree or PID census.

No Cargo dispatcher is admitted here. The only executable entry point is a
bounded synthetic control. Importing this module performs no OS operations.
"""

import ctypes
import json
import os
from pathlib import Path
import subprocess
import sys
import time
from ctypes import wintypes


class Refusal(RuntimeError):
    pass


class FileTime(ctypes.Structure):
    _fields_ = [("low", wintypes.DWORD), ("high", wintypes.DWORD)]

    def ticks(self):
        return (self.high << 32) | self.low


class IoCounters(ctypes.Structure):
    _fields_ = [(name, ctypes.c_ulonglong) for name in (
        "read_ops", "write_ops", "other_ops", "read_bytes", "write_bytes",
        "other_bytes")]


class MemoryCounters(ctypes.Structure):
    _fields_ = [("cb", wintypes.DWORD), ("page_faults", wintypes.DWORD)] + [
        (name, ctypes.c_size_t) for name in (
            "peak_working_set_bytes", "working_set_bytes", "peak_paged_pool",
            "paged_pool", "peak_nonpaged_pool", "nonpaged_pool",
            "private_commit_bytes", "peak_private_commit_bytes", "private_usage")]


class WindowsApi:
    """Query + synchronization rights only; no write, debug or terminate rights."""

    def __init__(self):
        if os.name != "nt":
            raise Refusal("Windows-only observer")
        self.dll = ctypes.WinDLL("kernel32", use_last_error=True)
        declarations = {
            "OpenProcess": ([wintypes.DWORD, wintypes.BOOL, wintypes.DWORD], wintypes.HANDLE),
            "CloseHandle": ([wintypes.HANDLE], wintypes.BOOL),
            "GetProcessId": ([wintypes.HANDLE], wintypes.DWORD),
            "WaitForSingleObject": ([wintypes.HANDLE, wintypes.DWORD], wintypes.DWORD),
            "GetProcessTimes": ([wintypes.HANDLE] + [ctypes.POINTER(FileTime)] * 4, wintypes.BOOL),
            "GetProcessIoCounters": ([wintypes.HANDLE, ctypes.POINTER(IoCounters)], wintypes.BOOL),
            "K32GetProcessMemoryInfo": ([wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD], wintypes.BOOL),
            "QueryFullProcessImageNameW": ([wintypes.HANDLE, wintypes.DWORD, wintypes.LPWSTR, ctypes.POINTER(wintypes.DWORD)], wintypes.BOOL),
        }
        for name, (args, result) in declarations.items():
            fn = getattr(self.dll, name)
            fn.argtypes, fn.restype = args, result

    @staticmethod
    def require(ok, operation):
        if not ok:
            raise Refusal(f"{operation}: Win32 error {ctypes.get_last_error()}")

    def open(self, pid):
        handle = self.dll.OpenProcess(0x1000 | 0x100000, False, pid)
        self.require(handle, "OpenProcess(query-limited|synchronize)")
        return handle

    def close(self, handle):
        self.require(self.dll.CloseHandle(handle), "CloseHandle")

    def identity(self, handle):
        pid = self.dll.GetProcessId(handle)
        self.require(pid, "GetProcessId")
        size = wintypes.DWORD(32768)
        buffer = ctypes.create_unicode_buffer(size.value)
        self.require(self.dll.QueryFullProcessImageNameW(handle, 0, buffer, ctypes.byref(size)), "QueryFullProcessImageNameW")
        return pid, buffer.value

    def read(self, handle):
        status = self.dll.WaitForSingleObject(handle, 0)
        if status not in (0, 258):
            raise Refusal(f"WaitForSingleObject: {status}")
        terminal = status == 0
        created, exited, kernel, user = (FileTime() for _ in range(4))
        self.require(self.dll.GetProcessTimes(handle, ctypes.byref(created), ctypes.byref(exited), ctypes.byref(kernel), ctypes.byref(user)), "GetProcessTimes")
        io = IoCounters()
        self.require(self.dll.GetProcessIoCounters(handle, ctypes.byref(io)), "GetProcessIoCounters")
        memory = MemoryCounters()
        memory.cb = ctypes.sizeof(memory)
        memory_ok = self.dll.K32GetProcessMemoryInfo(handle, ctypes.byref(memory), memory.cb)
        memory_error = None if memory_ok else ctypes.get_last_error()
        if not memory_ok and not terminal:
            raise Refusal(f"live K32GetProcessMemoryInfo: {memory_error}")
        return {
            "terminal": terminal, "creation_100ns": created.ticks(),
            # Windows leaves exit time undefined while the process is running.
            "exit_100ns": exited.ticks() if terminal else None,
            "kernel_100ns": kernel.ticks(), "user_100ns": user.ticks(),
            "io": {name: getattr(io, name) for name, _ in IoCounters._fields_},
            "memory": {name: getattr(memory, name) for name in (
                "working_set_bytes", "peak_working_set_bytes", "private_usage",
                "peak_private_commit_bytes")} if memory_ok else None,
            "terminal_memory_error": memory_error,
        }


def validate(previous, current):
    """Validate lifetime counters, not interval CPU or descendant totals."""
    if current["creation_100ns"] <= 0:
        raise Refusal("invalid creation identity")
    if current["terminal"] and current["exit_100ns"] < current["creation_100ns"]:
        raise Refusal("invalid terminal exit time")
    if not current["terminal"] and current["exit_100ns"] is not None:
        raise Refusal("undefined live exit time must be masked")
    if previous is None:
        return
    if previous["terminal"]:
        raise Refusal("sample after terminal record")
    if current["creation_100ns"] != previous["creation_100ns"]:
        raise Refusal("creation identity changed")
    for key in ("kernel_100ns", "user_100ns"):
        if current[key] < previous[key]:
            raise Refusal(f"counter regressed: {key}")
    for key in previous["io"]:
        if current["io"][key] < previous["io"][key]:
            raise Refusal(f"I/O counter regressed: {key}")


class OwnedObserver:
    """Attach once to a still-live Popen child. Never reopen by PID."""

    def __init__(self, child, expected_image, api=None, max_samples=400):
        if type(max_samples) is not int or not 1 <= max_samples <= 800:
            raise Refusal("sample budget outside 1..800")
        self.api = api if api is not None else WindowsApi()
        self.handle = None
        self.samples = []
        self.max_samples = max_samples
        self.refusal = None
        if child.poll() is not None:
            raise Refusal("child already exited before attachment")
        self.handle = self.api.open(child.pid)
        try:
            if child.poll() is not None:
                raise Refusal("child exited during attachment")
            self.pid, self.image = self.api.identity(self.handle)
            if self.pid != child.pid or os.path.normcase(self.image) != os.path.normcase(str(expected_image)):
                raise Refusal("owned child image/PID mismatch")
            self.sample()
        except BaseException as error:
            try:
                self.close()
            except Exception as close_error:
                error.add_note(f"attachment cleanup: {close_error}")
            raise

    def sample(self):
        if self.refusal is not None:
            raise Refusal(f"observer already refused: {self.refusal}")
        try:
            if self.handle is None:
                raise Refusal("observer handle closed")
            if len(self.samples) >= self.max_samples:
                raise Refusal("sample budget exhausted")
            row = self.api.read(self.handle)
            validate(self.samples[-1] if self.samples else None, row)
            row["observed_monotonic_ns"] = time.monotonic_ns()
            self.samples.append(row)
            return row
        except Exception as error:
            self.refusal = str(error)
            raise

    def close(self):
        if self.handle is not None:
            handle, self.handle = self.handle, None
            self.api.close(handle)


def fixture(output):
    """One synthetic child, no descendants, no Git/Cargo/server dispatcher."""
    out = Path(output).resolve()
    out.mkdir(parents=True, exist_ok=False)
    result = {"schema": 1, "scope": "owned-synthetic-child-only", "passed": False,
              "product_admission": False, "cargo_dispatch_admission": False,
              "refusal": None, "samples": []}
    child = observer = None
    try:
        # Persist intent before API/platform/child preflights.
        (out / "intent.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
        api = WindowsApi()
        code = (
            "import pathlib,sys,time; "
            "p=pathlib.Path(sys.argv[1]); p.write_bytes(b'x'*262144); "
            "assert len(p.read_bytes())==262144; "
            "stop=time.monotonic()+0.4; "
            "exec('while time.monotonic()<stop: sum(range(1000))'); "
            "print('synthetic-child-complete')"
        )
        with (out / "stdout.txt").open("xb") as stdout, (out / "stderr.txt").open("xb") as stderr:
            child = subprocess.Popen([sys.executable, "-I", "-c", code, str(out / "payload.bin")],
                                     stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr)
            observer = OwnedObserver(child, Path(sys.executable).resolve(), api)
            deadline = time.monotonic() + 10
            while not observer.samples[-1]["terminal"]:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise Refusal("synthetic child timeout")
                try:
                    child.wait(timeout=min(0.05, remaining))
                except subprocess.TimeoutExpired:
                    pass
                observer.sample()
            result["exit_code"] = child.wait(timeout=1)
        result["pid"], result["image"] = observer.pid, observer.image
        if result["exit_code"] != 0 or (out / "payload.bin").stat().st_size != 262144:
            raise Refusal("synthetic payload/status mismatch")
        if (out / "stdout.txt").read_bytes().strip() != b"synthetic-child-complete" or (out / "stderr.txt").read_bytes():
            raise Refusal("synthetic streams mismatch")
        result["passed"] = True
    except BaseException as error:
        result["refusal"] = f"{type(error).__name__}: {error}"
    finally:
        # Only this fixture's direct owned child can be stopped; it creates no tree.
        if child is not None and child.poll() is None:
            try:
                child.kill()
                child.wait(timeout=5)
            except Exception as error:
                result["passed"] = False
                result["cleanup_error"] = str(error)
        if observer is not None:
            result["samples"] = observer.samples
            try:
                observer.close()
            except Exception as error:
                result["passed"] = False
                result["close_error"] = str(error)
        (out / "result.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    return result["passed"]


if __name__ == "__main__":
    if len(sys.argv) != 3 or sys.argv[1] != "synthetic-control":
        raise SystemExit("usage: owned_process_observer.py synthetic-control NEW_OUTPUT_DIR")
    raise SystemExit(0 if fixture(sys.argv[2]) else 1)
