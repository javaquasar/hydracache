"""Bounded Windows diagnostic lineage, not authenticated production ownership.

Toolhelp parent identifiers plus retained live handles reject stale/reused PID
joins. They do not attest command lines, mapped image bytes or hostile PPID
spoofing. There is deliberately no Cargo/test dispatcher in this module.
"""

import ctypes
from ctypes import wintypes
import ntpath
import time

from owned_process_observer import Refusal, WindowsApi, validate


class ProcessEntry(ctypes.Structure):
    _fields_ = [("size", wintypes.DWORD), ("usage", wintypes.DWORD),
                ("pid", wintypes.DWORD), ("heap", ctypes.c_size_t),
                ("module", wintypes.DWORD), ("threads", wintypes.DWORD),
                ("parent", wintypes.DWORD), ("priority", wintypes.LONG),
                ("flags", wintypes.DWORD), ("image", wintypes.WCHAR * 260)]


class SnapshotApi(WindowsApi):
    def __init__(self):
        super().__init__()
        declarations = {
            "CreateToolhelp32Snapshot": ([wintypes.DWORD, wintypes.DWORD], wintypes.HANDLE),
            "Process32FirstW": ([wintypes.HANDLE, ctypes.POINTER(ProcessEntry)], wintypes.BOOL),
            "Process32NextW": ([wintypes.HANDLE, ctypes.POINTER(ProcessEntry)], wintypes.BOOL),
        }
        for name, (args, result) in declarations.items():
            function = getattr(self.dll, name)
            function.argtypes, function.restype = args, result

    def snapshot(self):
        handle = self.dll.CreateToolhelp32Snapshot(2, 0)  # processes only; no inheritance
        if handle == ctypes.c_void_p(-1).value or handle is None:
            raise Refusal(f"process snapshot: {ctypes.get_last_error()}")
        rows = []
        try:
            entry = ProcessEntry()
            entry.size = ctypes.sizeof(entry)
            ok = self.dll.Process32FirstW(handle, ctypes.byref(entry))
            while ok:
                if len(rows) >= 32768:
                    raise Refusal("process snapshot row budget exhausted")
                rows.append((entry.pid, entry.parent, entry.image))
                ok = self.dll.Process32NextW(handle, ctypes.byref(entry))
            error = ctypes.get_last_error()
            if error != 18:  # ERROR_NO_MORE_FILES; other failures are not EOF
                raise Refusal(f"process enumeration: {error}")
        except BaseException as error:
            try:
                self.close(handle)
            except Exception as close_error:
                error.add_note(f"snapshot cleanup: {close_error}")
            raise
        self.close(handle)
        return rows


def normalize(image):
    if not isinstance(image, str) or not ntpath.isabs(image) or "\0" in image:
        raise Refusal("lineage image must be an absolute Windows path")
    return ntpath.normcase(ntpath.normpath(image))


def candidates(rows, root_pid, policies):
    """Pure bounded shape selector; full paths are checked only on held handles."""
    if not policies or len(policies) > 4:
        raise Refusal("lineage policy count outside 1..4")
    normalized = []
    for policy in policies:
        if not 1 <= len(policy) <= 4:
            raise Refusal("lineage depth outside 1..4")
        normalized.append(tuple(normalize(image) for image in policy))
    if len(set(normalized)) != len(normalized):
        raise Refusal("duplicate lineage policy")
    if len(rows) > 32768:
        raise Refusal("process snapshot row budget exhausted")
    by_pid, children = {}, {}
    for pid, parent, image in rows:
        if type(pid) is not int or type(parent) is not int or pid < 0 or parent < 0 or not isinstance(image, str):
            raise Refusal("invalid process snapshot row")
        if pid in by_pid:
            raise Refusal("duplicate snapshot PID")
        by_pid[pid] = (parent, image)
        children.setdefault(parent, []).append(pid)
    if root_pid not in by_pid:
        raise Refusal("original root absent from snapshot")
    found = []
    for policy in normalized:
        partial = [(root_pid,)]
        for image in policy:
            next_partial = []
            for prefix in partial:
                for pid in children.get(prefix[-1], []):
                    if pid in prefix:
                        raise Refusal("cycle in candidate lineage")
                    if by_pid[pid][1].casefold() == ntpath.basename(image).casefold():
                        next_partial.append((*prefix, pid))
                        if len(next_partial) > 64:
                            raise Refusal("candidate frontier budget exhausted")
            partial = next_partial
        found.extend((pids, policy) for pids in partial)
    if len(found) > 1:
        raise Refusal("ambiguous candidate lineage")
    return found


def live_identity(api, handle, pid, image, creation=None):
    observed_pid, observed_image = api.identity(handle)
    row = api.read(handle)
    validate(None, row)
    if observed_pid != pid or normalize(observed_image) != normalize(image):
        raise Refusal("held lineage image/PID mismatch")
    if row["terminal"]:
        raise Refusal("lineage process no longer live")
    if creation is not None and row["creation_100ns"] != creation:
        raise Refusal("held lineage creation identity changed")
    return row["creation_100ns"]


class HeldLineage:
    """One attachment attempt; borrow the root and own all descendant handles."""

    def __init__(self, root, api, policies, max_samples=800):
        if type(max_samples) is not int or not 1 <= max_samples <= 800:
            raise Refusal("lineage sample budget outside 1..800")
        if root.handle is None or root.refusal or not root.samples:
            raise Refusal("root observer is not usable")
        self.api, self.root = api, root
        self.handles, self.identities, self.samples = [], [], []
        self.max_samples, self.refusal = max_samples, None
        first = api.snapshot()
        selected = candidates(first, root.pid, policies)
        if not selected:
            raise Refusal("candidate lineage not present")
        self.pids, self.images = selected[0]
        birth = live_identity(api, root.handle, root.pid, root.image, root.samples[0]["creation_100ns"])
        self.identities.append(dict(pid=root.pid, image=root.image, creation_100ns=birth))
        parent_handle, parent_pid, parent_image = root.handle, root.pid, root.image
        try:
            for pid, image in zip(self.pids[1:], self.images):
                live_identity(api, parent_handle, parent_pid, parent_image, birth)
                handle = api.open(pid)
                self.handles.append(handle)
                child_birth = live_identity(api, handle, pid, image)
                if child_birth < birth:
                    raise Refusal("child predates held parent")
                live_identity(api, parent_handle, parent_pid, parent_image, birth)
                self.identities.append(dict(pid=pid, image=image, creation_100ns=child_birth))
                parent_handle, parent_pid, parent_image, birth = handle, pid, image, child_birth
            second = candidates(api.snapshot(), root.pid, policies)
            if second != selected:
                raise Refusal("lineage changed between snapshots")
            for handle, identity in zip([root.handle, *self.handles], self.identities):
                live_identity(api, handle, identity["pid"], identity["image"], identity["creation_100ns"])
            self.sample()
        except BaseException as error:
            try:
                self.close()
            except Exception as close_error:
                error.add_note(f"lineage cleanup: {close_error}")
            raise

    def sample(self):
        if self.refusal is not None:
            raise Refusal(f"lineage already refused: {self.refusal}")
        try:
            if not self.handles:
                raise Refusal("lineage handles closed")
            if len(self.samples) >= self.max_samples:
                raise Refusal("lineage sample budget exhausted")
            row = self.api.read(self.handles[-1])
            validate(self.samples[-1] if self.samples else None, row)
            if row["creation_100ns"] != self.identities[-1]["creation_100ns"]:
                raise Refusal("leaf creation identity changed")
            row["observed_monotonic_ns"] = time.monotonic_ns()
            self.samples.append(row)
            return row
        except Exception as error:
            self.refusal = str(error)
            raise

    def close(self):
        first_error = None
        while self.handles:
            handle = self.handles.pop()
            try:
                self.api.close(handle)
            except Exception as error:
                if first_error is None:
                    first_error = error
        if first_error is not None:
            raise first_error
