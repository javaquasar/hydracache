"""Process-local Windows output custody. Not production writer retirement.

The trusted workspace and unchanged wrapper are prerequisites. Held objects deny
delete sharing; sealing also denies write sharing. No PID lookup or dispatch.
"""

import ctypes
from ctypes import wintypes
import hashlib
import os
from pathlib import Path
import re

from owned_process_observer import FileTime, Refusal


class FileInfo(ctypes.Structure):
    _fields_ = [("attributes", wintypes.DWORD), ("created", FileTime),
                ("accessed", FileTime), ("written", FileTime)] + [
        (name, wintypes.DWORD) for name in ("volume", "size_high", "size_low", "links", "index_high", "index_low")]


class WindowsFiles:
    """Non-inheritable, OPEN_EXISTING, no-delete handles; no write access."""

    def __init__(self):
        if os.name != "nt":
            raise Refusal("Windows-only original output binding")
        self.dll = ctypes.WinDLL("kernel32", use_last_error=True)
        declarations = {
            "CreateFileW": ([wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD, ctypes.c_void_p,
                             wintypes.DWORD, wintypes.DWORD, wintypes.HANDLE], wintypes.HANDLE),
            "GetFileInformationByHandle": ([wintypes.HANDLE, ctypes.POINTER(FileInfo)], wintypes.BOOL),
            "ReadFile": ([wintypes.HANDLE, ctypes.c_void_p, wintypes.DWORD,
                          ctypes.POINTER(wintypes.DWORD), ctypes.c_void_p], wintypes.BOOL),
            "CloseHandle": ([wintypes.HANDLE], wintypes.BOOL),
        }
        for name, (args, result) in declarations.items():
            getattr(self.dll, name).argtypes, getattr(self.dll, name).restype = args, result

    @staticmethod
    def require(ok, operation):
        if not ok:
            raise Refusal(f"{operation}: Win32 error {ctypes.get_last_error()}")

    def open(self, path, directory=False, sealed=False):
        handle = self.dll.CreateFileW(str(path), 0x80000000, 1 if sealed else 3, None, 3,
                                      0x00200000 | (0x02000000 if directory else 0), None)
        self.require(handle not in (None, ctypes.c_void_p(-1).value), "hold original output")
        try:
            info = self.info(handle)
            if info["attributes"] & 0x400 or bool(info["attributes"] & 0x10) != directory:
                raise Refusal("output reparse point or object type mismatch")
            if not directory and info["links"] != 1:
                raise Refusal("output has hard-link aliases")
            return handle
        except BaseException:
            self.close(handle)
            raise

    def info(self, handle):
        value = FileInfo()
        self.require(self.dll.GetFileInformationByHandle(handle, ctypes.byref(value)), "output identity")
        return dict(volume=value.volume, index=(value.index_high << 32) | value.index_low,
                    attributes=value.attributes, links=value.links,
                    size=(value.size_high << 32) | value.size_low, written=value.written.ticks())

    def read(self, handle, limit):
        # A fresh sealed handle starts at zero. Never reopen a path for content.
        before = self.info(handle)
        if before["size"] > limit:
            raise Refusal("original output exceeds fixed read limit")
        chunks, remaining = [], before["size"]
        while remaining:
            buffer = ctypes.create_string_buffer(min(remaining, 65536))
            count = wintypes.DWORD()
            self.require(self.dll.ReadFile(handle, buffer, len(buffer), ctypes.byref(count), None), "held output read")
            if not 0 < count.value <= min(remaining, len(buffer)):
                raise Refusal("short original output read")
            chunks.append(buffer.raw[:count.value])
            remaining -= count.value
        if self.info(handle) != before:
            raise Refusal("sealed original output changed during read")
        return b"".join(chunks)

    def close(self, handle):
        self.require(self.dll.CloseHandle(handle), "close held output")


def identity(info):
    return info["volume"], info["index"]


class AttemptOutputs:
    LIMITS = {"intent.json": 16 * 1024**2, "stdout.log": 64 * 1024**2,
              "stderr.log": 64 * 1024**2, "result.json": 16 * 1024**2}
    LIVE_FILES = ("intent.json", "stdout.log", "stderr.log")

    def __init__(self, attempt_id, reservation, wrapper_parent, expected, api=None):
        if re.fullmatch(r"[a-z][a-z0-9-]{0,63}", attempt_id) is None:
            raise Refusal("invalid output attempt identifier")
        self.attempt_id = attempt_id
        self.reservation, self.parent = Path(reservation).absolute(), Path(wrapper_parent).absolute()
        self.path = self.parent / attempt_id
        self.expected, self.api = expected, api if api is not None else WindowsFiles()
        self.handles, self.identities = {}, {}
        self.custody = self.owner = None
        self.owner_identity = None
        self.first_refusal = None
        self.attach_attempted = self.seal_attempted = self.closed = False
        try:
            for name, path in (("reservation", self.reservation), ("parent", self.parent)):
                self.handles[name] = self.api.open(path, directory=True)
                self.identities[name] = identity(self.api.info(self.handles[name]))
            if os.path.lexists(self.path):
                raise Refusal("wrapper output slot already exists")
        except BaseException:
            self.close()
            raise

    def check(self):
        if self.closed or self.first_refusal is not None:
            raise Refusal("output binding closed or previously refused")

    def latch(self, error):
        if self.first_refusal is None:
            self.first_refusal = str(error)

    def hold(self, name, path, directory=False):
        handle = self.api.open(path, directory=directory)
        self.handles[name] = handle
        self.identities[name] = identity(self.api.info(handle))

    def attach(self, custody):
        """Call once while the original wrapper is live and its streams exist."""
        self.check()
        if self.attach_attempted:
            raise Refusal("output attachment cannot be repeated")
        self.attach_attempted = True
        self.custody, self.owner = custody, custody.owner
        try:
            if self.owner.poll() is not None or self.owner.pid != custody.root.pid or custody.root.handle is None:
                raise Refusal("original output owner is not live")
            self.owner_identity = dict(pid=custody.root.pid,
                                       creation_100ns=custody.root.samples[0]["creation_100ns"])
            if self.owner_identity["creation_100ns"] <= 0:
                raise Refusal("invalid original output owner birth")
            self.hold("directory", self.path, directory=True)
            for name in self.LIVE_FILES:
                self.hold(name, self.path / name)
            if self.owner.poll() is not None:
                raise Refusal("owner exited during output attachment")
        except Exception as error:
            self.latch(error)
            raise

    def seal(self, custody):
        """One read after held terminal proof; response cannot select other files."""
        self.check()
        if self.seal_attempted:
            raise Refusal("output sealing cannot be repeated")
        self.seal_attempted = True
        sealed = []
        try:
            if custody is not self.custody or custody.exit_proof is None or custody.custody_refusal is not None:
                raise Refusal("original terminal custody proof required")
            witness = custody.exit_proof["witnesses"][0]
            if any(witness[key] != value for key, value in self.owner_identity.items()):
                raise Refusal("output owner differs from terminal witness")
            if custody.owner is not self.owner or self.owner.poll() != custody.exit_proof["owner_returncode"]:
                raise Refusal("original output owner return code changed")
            # Lock *all* files against writers before reading any content. Existing
            # write handles (including leaked aliases) make this attempt refuse.
            self.hold("result.json", self.path / "result.json")
            for name in ("reservation", "parent", "directory"):
                path = {"reservation": self.reservation, "parent": self.parent, "directory": self.path}[name]
                handle = self.api.open(path, directory=True)
                sealed.append(handle)
                if identity(self.api.info(handle)) != self.identities[name]:
                    raise Refusal("original output directory identity changed")
            locks = {}
            for name in self.LIMITS:
                handle = self.api.open(self.path / name, sealed=True)
                sealed.append(handle)
                if identity(self.api.info(handle)) != self.identities[name]:
                    raise Refusal("original output file identity changed")
                locks[name] = handle
            payload = {name: self.api.read(locks[name], limit) for name, limit in self.LIMITS.items()}
            binding = dict(attempt_id=self.attempt_id, owner=self.owner_identity.copy(),
                           directory_identity=self.identities["directory"],
                           files={name: dict(identity=self.identities[name], bytes=len(raw),
                                             sha256=hashlib.sha256(raw).hexdigest()) for name, raw in payload.items()},
                           scope="trusted local wrapper; held output objects only", all_writers_proven_stopped=False,
                           durable_recovery=False, admission=False)
            return payload, binding
        except Exception as error:
            self.latch(error)
            raise
        finally:
            for handle in reversed(sealed):
                try:
                    self.api.close(handle)
                except Exception as error:
                    self.latch(error)
            if self.first_refusal is not None:
                raise Refusal(self.first_refusal)

    def close(self):
        if self.closed:
            return
        if self.owner is not None and self.owner.poll() is None:
            raise Refusal("live original owner: retain output custody")
        self.closed = True
        first = None
        for handle in reversed(list(self.handles.values())):
            try:
                self.api.close(handle)
            except Exception as error:
                first = first or error
        if first is not None:
            self.latch(first)
            raise Refusal(str(first))
