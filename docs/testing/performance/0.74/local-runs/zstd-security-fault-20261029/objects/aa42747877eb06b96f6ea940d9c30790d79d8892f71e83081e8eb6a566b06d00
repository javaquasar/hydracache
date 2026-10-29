#!/usr/bin/env python3
"""Bounded read-only Linux inventory; never a workload or host-admission gate.

Run the reviewed source over SSH stdin. No uploads, writes, service operations,
package installation, process arguments, environment or secret files are used.
Privileged read access is needed for the protected campaign directory.
"""

from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import time


SERVICE = "hydracache-performance-supervisor-074.service"
CAMPAIGNS = Path("/var/lib/hydracache-performance/campaigns")
MARKERS = (
    CAMPAIGNS / "active-campaign",
    Path("/var/lib/hydracache-performance/campaign-lifecycle-smoke-v1.json"),
    Path("/run/hydracache-perf/controller-loss-smoke-v1.json"),
)
RECEIPT = Path("/var/lib/hydracache-performance/provisioning-receipt-074.json")
MAX_BYTES = 1024 * 1024


def read_text(path: Path) -> str:
    with path.open("rb") as stream:
        raw = stream.read(MAX_BYTES + 1)
    if len(raw) > MAX_BYTES:
        raise ValueError("read limit exceeded")
    return raw.decode("utf-8")


def read_value(path: Path) -> dict:
    try:
        return {"status": "available", "value": read_text(path).strip()}
    except (OSError, ValueError, UnicodeError) as error:
        return {"status": "unavailable", "reason": type(error).__name__}


def command(arguments: list[str]) -> dict:
    if shutil.which(arguments[0]) is None:
        return {"status": "unavailable", "reason": "executable-not-on-audit-PATH"}
    try:
        result = subprocess.run(arguments, capture_output=True, timeout=10, check=False)
        if len(result.stdout) + len(result.stderr) > MAX_BYTES:
            return {"status": "unavailable", "reason": "output-limit-exceeded"}
        return {
            "status": "available" if result.returncode == 0 else "command-failed",
            "exit_code": result.returncode,
            "stdout": result.stdout.decode("utf-8").strip(),
            # No arbitrary error payloads, paths or process arguments in evidence.
            "stderr_present": bool(result.stderr),
        }
    except (OSError, subprocess.TimeoutExpired, UnicodeError) as error:
        return {"status": "unavailable", "reason": type(error).__name__}


def directory_safe(path: Path) -> bool:
    """Every ancestor must be a real searchable directory, never a symlink."""
    for ancestor in [*reversed(path.parents), path]:
        info = ancestor.lstat()
        if not stat.S_ISDIR(info.st_mode):
            return False
        # Probe search/read access; permission errors are unknown, not absence.
        with os.scandir(ancestor):
            pass
    return True


def marker_state(path: Path) -> dict:
    try:
        if not directory_safe(path.parent):
            return {"path": str(path), "status": "unknown", "reason": "unsafe-parent"}
        try:
            info = path.lstat()
        except FileNotFoundError:
            return {"path": str(path), "status": "absent"}
        return {
            "path": str(path), "status": "present",
            "symlink": stat.S_ISLNK(info.st_mode),
        }
    except (OSError, ValueError) as error:
        return {"path": str(path), "status": "unknown", "reason": type(error).__name__}


def cpu_snapshot(path: Path) -> dict[str, list[int]]:
    rows = {}
    for line in read_text(path).splitlines():
        fields = line.split()
        if fields and (fields[0] == "cpu" or fields[0][3:].isdigit() and fields[0].startswith("cpu")):
            values = [int(value) for value in fields[1:]]
            if len(values) < 8 or any(value < 0 for value in values):
                raise ValueError("invalid CPU counters")
            rows[fields[0]] = values[:8]  # guest counters already included in user/nice.
    if "cpu" not in rows:
        raise ValueError("aggregate CPU counter missing")
    return rows


def cpu_deltas(before: dict, after: dict) -> dict:
    if set(before) != set(after):
        raise ValueError("CPU roster changed")
    result = {}
    for name in sorted(before):
        delta = [end - start for start, end in zip(before[name], after[name])]
        if len(delta) != 8 or any(value < 0 for value in delta) or sum(delta) == 0:
            raise ValueError("CPU counters invalid or too short")
        total = sum(delta)
        idle, iowait, steal = delta[3], delta[4], delta[7]
        result[name] = {
            "delta_ticks": delta, "total_ticks": total,
            "busy_fraction": (total - idle - iowait) / total,
            "iowait_fraction": iowait / total, "steal_fraction": steal / total,
        }
    return result


def process_inventory(proc: Path = Path("/proc")) -> dict:
    matched, disappeared, unknown = [], 0, 0
    prefixes = ("hydracache", "cargo", "rustc", "timing-controls", "memory-diagnos",
                "allocator-profi", "resp-", "get-owner", "get_owner", "Runner.")
    entries = list(proc.iterdir())
    if len(entries) > 65536:
        return {"complete": False, "reason": "process-roster-limit"}
    for entry in sorted(entries, key=lambda value: value.name):
        if not entry.name.isdigit():
            continue
        try:
            name = read_text(entry / "comm").strip()
            if name.startswith(prefixes):
                matched.append({"pid": int(entry.name), "comm": name,
                                "cgroup": read_text(entry / "cgroup").strip()})
        except FileNotFoundError:
            disappeared += 1
        except (OSError, ValueError, UnicodeError):
            unknown += 1
    return {"complete": unknown == 0, "matched": matched,
            "disappeared_during_snapshot": disappeared, "unknown_processes": unknown}


def lifecycle() -> dict:
    return {
        "markers": [marker_state(path) for path in MARKERS],
        "supervisor": command(["systemctl", "show", SERVICE,
                               "--property=ActiveState,SubState,MainPID,NRestarts,ControlGroup"]),
        "units": command(["systemctl", "list-units", "--all", "--no-legend", "--plain",
                          "actions.runner.*", "hydracache*"]),
        "processes": process_inventory(),
    }


def provisioning() -> dict:
    try:
        if not directory_safe(RECEIPT.parent) or not stat.S_ISREG(RECEIPT.lstat().st_mode):
            raise ValueError("unsafe receipt")
        raw = read_text(RECEIPT).encode("utf-8")
        value = json.loads(raw)
        return {"status": "available", "raw_sha256": hashlib.sha256(raw).hexdigest(),
                "fields": {key: value[key] for key in (
                    "source_commit", "binary_sha256", "fixture_binary_sha256"
                ) if key in value}}
    except (OSError, ValueError, UnicodeError) as error:
        return {"status": "unavailable", "reason": type(error).__name__}


def allocator_inventory() -> dict:
    result = command(["ldconfig", "-p"])
    if result["status"] == "available":
        result["stdout"] = "\n".join(
            line for line in result["stdout"].splitlines()
            if any(name in line for name in ("jemalloc", "mimalloc", "tcmalloc")))
    return result


def installed_binary_hashes() -> dict:
    result = {}
    for name in ("/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074",
                 "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture"):
        path = Path(name)
        try:
            if not directory_safe(path.parent) or not stat.S_ISREG(path.lstat().st_mode):
                raise ValueError("unsafe executable")
            digest, size = hashlib.sha256(), 0
            with path.open("rb") as stream:
                while block := stream.read(65536):
                    size += len(block)
                    if size > 64 * MAX_BYTES:
                        raise ValueError("executable hash limit")
                    digest.update(block)
            result[name] = {"status": "available", "sha256": digest.hexdigest(), "bytes": size}
        except (OSError, ValueError) as error:
            result[name] = {"status": "unavailable", "reason": type(error).__name__}
    return result


def collect() -> dict:
    if platform.system() != "Linux":
        raise ValueError("Linux-only inventory")
    start = time.monotonic_ns()
    first = lifecycle()
    before = cpu_snapshot(Path("/proc/stat"))
    cpu_start = time.monotonic_ns()
    time.sleep(2)  # one fixed diagnostic interval, not a workload or noise pass.
    after = cpu_snapshot(Path("/proc/stat"))
    interval = time.monotonic_ns() - cpu_start
    last = lifecycle()
    mem = {}
    for line in read_text(Path("/proc/meminfo")).splitlines():
        fields = line.split()
        if fields[0] in ("MemTotal:", "MemAvailable:", "SwapTotal:", "SwapFree:"):
            mem[fields[0].rstrip(":")] = int(fields[1]) * 1024
    sysfs = "/sys/devices/system/cpu"
    return {
        "schema_version": "hydracache-rental-readonly-preflight-074-v1",
        "captured_at_unix_seconds": int(time.time()),
        "elapsed_ns": time.monotonic_ns() - start,
        "effective_uid": os.geteuid(), "kernel": platform.release(),
        "architecture": platform.machine(),
        "boot_id_sha256": hashlib.sha256(read_text(Path("/proc/sys/kernel/random/boot_id")).strip().encode()).hexdigest(),
        "lifecycle_before": first, "lifecycle_after": last,
        "provisioning": provisioning(),
        "installed_binary_hashes": installed_binary_hashes(),
        "cpu_sample": {"interval_ns": interval, "ticks_per_second": os.sysconf("SC_CLK_TCK"),
                       "per_cpu": cpu_deltas(before, after), "noise_gate_pass_claimed": False},
        "memory_bytes": mem,
        "placement": {name: read_value(Path(sysfs) / name) for name in
                      ("online", "offline", "isolated", "nohz_full", "smt/control")},
        "governors": {path.parent.name: read_value(path) for path in
                      sorted(Path(sysfs).glob("cpufreq/policy*/scaling_governor"))},
        "numa_online": read_value(Path("/sys/devices/system/node/online")),
        "pressure": {name: read_value(Path("/proc/pressure") / name) for name in ("cpu", "memory", "io")},
        "disk": {"free_bytes": shutil.disk_usage("/var/lib/hydracache-performance").free},
        "perf_event_paranoid": read_value(Path("/proc/sys/kernel/perf_event_paranoid")),
        "libc": command(["getconf", "GNU_LIBC_VERSION"]),
        "allocator_libraries": allocator_inventory(),
        "allocator_native_active_resident_retained_proven": False,
        "tool_paths": {name: shutil.which(name) for name in ("python3", "cc", "make", "cmake", "perf", "taskset", "cargo", "rustc")},
        "build_user_toolchain": {
            "toolchains": command(["sudo", "-n", "-u", "github-runner",
                                   "/home/github-runner/.cargo/bin/rustup", "toolchain", "list"]),
            "rustc": command(["sudo", "-n", "-u", "github-runner",
                              "/home/github-runner/.cargo/bin/rustc", "+1.94.0", "-Vv"]),
            "cargo": command(["sudo", "-n", "-u", "github-runner",
                              "/home/github-runner/.cargo/bin/cargo", "+1.94.0", "-V"]),
        },
        "read_only": True, "product_process_started": False,
        "service_operations_performed": False, "qualification_started": False,
        "promotable": False, "admission_allowed": False,
        "limitations": ["two snapshots do not reserve the host or prevent a concurrent start",
                        "two-second CPU inventory is not finite A/A noise calibration",
                        "library discovery is not telemetry capability proof or System allocator retention",
                        "root audit PATH is not the future measured build-user toolchain"],
    }


if __name__ == "__main__":
    print(json.dumps(collect(), sort_keys=True, indent=2))
