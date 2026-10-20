"""Read-only vDSO versus direct syscall observations on Linux x86_64."""
import ctypes
import heapq
import json
import os
import platform
import sys
import time

assert platform.machine() == "x86_64"
libc = ctypes.CDLL(None, use_errno=True)
libc.syscall.restype = ctypes.c_long


class Timespec(ctypes.Structure):
    _fields_ = [("seconds", ctypes.c_long), ("nanoseconds", ctypes.c_long)]


def sample(direct):
    value = Timespec()
    if direct:
        result = libc.syscall(ctypes.c_long(228), ctypes.c_int(0), ctypes.byref(value))
    else:
        result = libc.clock_gettime(ctypes.c_int(0), ctypes.byref(value))
    assert result == 0
    return value.seconds * 1_000_000_000 + value.nanoseconds


allowed = sorted(os.sched_getaffinity(0))
if len(sys.argv) > 1 and sys.argv[1] == "pin":
    os.sched_setaffinity(0, {allowed[0]})
worst = []
backwards = [0, 0, 0]
last = None
for index in range(20000):
    os.sched_setaffinity(0, {allowed[index % len(allowed)]})
    before_cpu = libc.sched_getcpu()
    vdso = sample(False)
    direct = sample(True)
    python = time.time_ns()
    after_cpu = libc.sched_getcpu()
    values = [vdso, direct, python]
    if last is not None:
        for column in range(3):
            backwards[column] += int(values[column] < last[column])
    last = values
    row = (max(abs(vdso - direct), abs(python - direct)), index,
           before_cpu, after_cpu, vdso, direct, python)
    heapq.heappush(worst, row)
    if len(worst) > 8:
        heapq.heappop(worst)
    if index % 1000 == 0:
        os.sched_yield()
print(json.dumps({"samples": 20000, "affinity": sorted(os.sched_getaffinity(0)),
                  "backwards_vdso_syscall_python": backwards,
                  "worst_columns": ["absolute_skew_ns", "sample", "cpu_before", "cpu_after", "vdso", "syscall", "python"],
                  "worst": sorted(worst, reverse=True),
                  "manual_clock_changed": False, "qualification": False, "forced_rotation": True}))
