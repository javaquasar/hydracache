#!/usr/bin/env python3
"""Collect process/container telemetry for the exploratory comparison only.

The collector uses only the Python standard library. It deliberately records
unavailable JVM heap data as unavailable instead of confusing RSS with heap.
"""

from __future__ import annotations

import argparse
import csv
import json
import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

STOP = False


def stop(_signum: int, _frame: Any) -> None:
    global STOP
    STOP = True


def command_json(command: list[str]) -> Any | None:
    try:
        completed = subprocess.run(command, check=True, capture_output=True, text=True)
        return json.loads(completed.stdout)
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError):
        return None


def proc_status(pid: int) -> dict[str, str]:
    result: dict[str, str] = {}
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            if ":" in line:
                key, value = line.split(":", 1)
                result[key] = value.strip()
    except OSError:
        pass
    return result


def proc_cpu_ticks(pid: int) -> int | None:
    try:
        fields = Path(f"/proc/{pid}/stat").read_text().split()
        return int(fields[13]) + int(fields[14])
    except (OSError, IndexError, ValueError):
        return None


def cgroup_dir(pid: int) -> Path | None:
    try:
        lines = Path(f"/proc/{pid}/cgroup").read_text().splitlines()
    except OSError:
        return None
    for line in lines:
        hierarchy, _controllers, path = line.split(":", 2)
        if hierarchy == "0":
            candidate = Path("/sys/fs/cgroup") / path.lstrip("/")
            return candidate if candidate.exists() else None
    return None


def read_int(path: Path) -> int | None:
    try:
        value = path.read_text().strip()
        if value == "max":
            return None
        return int(value)
    except (OSError, ValueError):
        return None


def cgroup_memory(directory: Path | None) -> dict[str, int | None]:
    if directory is None:
        return {
            "current_bytes": None,
            "peak_bytes": None,
            "limit_bytes": None,
            "anon_bytes": None,
            "file_bytes": None,
            "slab_bytes": None,
        }
    values: dict[str, int | None] = {
        "current_bytes": read_int(directory / "memory.current"),
        "peak_bytes": read_int(directory / "memory.peak"),
        "limit_bytes": read_int(directory / "memory.max"),
        "anon_bytes": None,
        "file_bytes": None,
        "slab_bytes": None,
    }
    try:
        for line in (directory / "memory.stat").read_text().splitlines():
            key, value = line.split()[:2]
            if key in {"anon", "file", "slab"}:
                values[f"{key}_bytes"] = int(value)
    except (OSError, ValueError):
        pass
    return values


def smaps_rollup(pid: int | None) -> dict[str, int | None]:
    values = {"rss_bytes": None, "pss_anon_bytes": None, "pss_file_bytes": None}
    if pid is None:
        return values
    try:
        for line in Path(f"/proc/{pid}/smaps_rollup").read_text().splitlines():
            key, value = line.split(":", 1)
            amount = int(value.strip().split()[0]) * 1024
            if key == "Rss":
                values["rss_bytes"] = amount
            elif key == "Pss_Anon":
                values["pss_anon_bytes"] = amount
            elif key == "Pss_File":
                values["pss_file_bytes"] = amount
    except (OSError, ValueError, IndexError):
        pass
    return values


def process_threads(pid: int | None) -> int | None:
    if pid is None:
        return None
    try:
        return len(list(Path(f"/proc/{pid}/task").iterdir()))
    except OSError:
        return None


def process_fd_count(pid: int | None) -> int | None:
    if pid is None:
        return None
    try:
        return len(list(Path(f"/proc/{pid}/fd").iterdir()))
    except OSError:
        return None


def cgroup_cpu_usec(directory: Path | None) -> int | None:
    if directory is None:
        return None
    try:
        for line in (directory / "cpu.stat").read_text().splitlines():
            key, value = line.split()
            if key == "usage_usec":
                return int(value)
    except (OSError, ValueError):
        pass
    return None


def affinity(status: dict[str, str]) -> str | None:
    return status.get("Cpus_allowed_list") or None


def effective_cpus(cpuset: str | None) -> float:
    if not cpuset:
        return float(os.cpu_count() or 1)
    count = 0
    for part in cpuset.split(","):
        bounds = part.split("-")
        try:
            count += int(bounds[-1]) - int(bounds[0]) + 1
        except ValueError:
            return float(os.cpu_count() or 1)
    return float(max(count, 1))


def jvm_heap(pid: int | None) -> dict[str, Any]:
    """Optional heap telemetry; never treats RSS as heap."""
    command = os.environ.get("JVM_HEAP_CMD")
    if not command or pid is None:
        return {"available": False, "reason": "no JVM_HEAP_CMD configured"}
    try:
        output = subprocess.check_output(
            command.replace("$PID", str(pid)), shell=True, text=True, stderr=subprocess.STDOUT
        )
        payload = json.loads(output)
        payload["available"] = True
        return payload
    except (OSError, subprocess.CalledProcessError, json.JSONDecodeError) as error:
        return {"available": False, "reason": f"JVM_HEAP_CMD failed: {error}"}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--pid", type=int)
    parser.add_argument("--container")
    parser.add_argument("--interval", type=float, default=1.0)
    parser.add_argument("--duration", type=float, default=0.0)
    args = parser.parse_args()

    signal.signal(signal.SIGTERM, stop)
    signal.signal(signal.SIGINT, stop)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    metadata: dict[str, Any] = {
        "target": args.target,
        "container": args.container,
        "collector_interval_seconds": args.interval,
        "host": os.uname().nodename,
        "host_cpu_count": os.cpu_count(),
    }
    pid = args.pid
    if args.container:
        inspected = command_json(["docker", "inspect", args.container])
        if inspected:
            item = inspected[0]
            metadata["container_metadata"] = item
            pid = (item.get("State") or {}).get("Pid") or pid
    metadata["pid"] = pid
    args.output.with_suffix(".metadata.json").write_text(
        json.dumps(metadata, indent=2, sort_keys=True) + "\n"
    )

    fields = [
        "timestamp_unix", "target", "pid", "container_cpu_percent", "process_cpu_percent",
        "process_cpu_ticks",
        "vmrss_bytes", "vmhwm_bytes", "effective_cpu_affinity", "cgroup_memory_current_bytes",
        "cgroup_memory_peak_bytes", "cgroup_memory_limit_bytes", "cgroup_memory_anon_bytes",
        "cgroup_memory_file_bytes", "cgroup_memory_slab_bytes", "smaps_rollup_rss_bytes",
        "smaps_rollup_pss_anon_bytes", "smaps_rollup_pss_file_bytes", "process_threads",
        "process_fd_count", "jvm_heap_available",
        "jvm_heap_used_bytes", "jvm_heap_committed_bytes", "jvm_heap_max_bytes",
    ]
    started = time.monotonic()
    previous_group = cgroup_dir(pid) if pid else None
    previous_cpu = cgroup_cpu_usec(previous_group)
    previous_process_ticks = proc_cpu_ticks(pid) if pid else None
    previous_time = time.monotonic()
    clock_ticks = float(os.sysconf("SC_CLK_TCK"))
    with args.output.open("w", encoding="utf-8") as json_file, args.output.with_suffix(".csv").open(
        "w", newline="", encoding="utf-8"
    ) as csv_file:
        writer = csv.DictWriter(csv_file, fieldnames=fields)
        writer.writeheader()
        while not STOP and (not args.duration or time.monotonic() - started < args.duration):
            status = proc_status(pid) if pid else {}
            group = cgroup_dir(pid) if pid else None
            current_cpu = cgroup_cpu_usec(group)
            elapsed = max(time.monotonic() - previous_time, 1e-6)
            cpu_delta = None if current_cpu is None or previous_cpu is None else current_cpu - previous_cpu
            current_process_ticks = proc_cpu_ticks(pid) if pid else None
            process_tick_delta = (
                None
                if current_process_ticks is None or previous_process_ticks is None
                else max(current_process_ticks - previous_process_ticks, 0)
            )
            process_cpu_percent = (
                None
                if process_tick_delta is None
                else process_tick_delta / clock_ticks / elapsed
                / effective_cpus(affinity(status)) * 100
            )
            # This is a container metric. For host Hydra, leave container CPU
            # unavailable rather than exposing an unrelated cgroup aggregate.
            cpu_percent = None
            if args.container and cpu_delta is not None:
                cpu_percent = cpu_delta / 1_000_000 / elapsed / effective_cpus(affinity(status)) * 100
            memory = cgroup_memory(group)
            smaps = smaps_rollup(pid)
            heap = jvm_heap(pid)
            rss = int(status["VmRSS"].split()[0]) * 1024 if "VmRSS" in status else None
            hwm = int(status["VmHWM"].split()[0]) * 1024 if "VmHWM" in status else None
            row: dict[str, Any] = {
                "timestamp_unix": time.time(),
                "target": args.target,
                "pid": pid,
                "container_cpu_percent": round(cpu_percent, 4) if cpu_percent is not None else None,
                "process_cpu_percent": round(process_cpu_percent, 4)
                if process_cpu_percent is not None
                else None,
                "process_cpu_ticks": current_process_ticks,
                "vmrss_bytes": rss,
                "vmhwm_bytes": hwm,
                "effective_cpu_affinity": affinity(status),
                "cgroup_memory_current_bytes": memory["current_bytes"],
                "cgroup_memory_peak_bytes": memory["peak_bytes"],
                "cgroup_memory_limit_bytes": memory["limit_bytes"],
                "cgroup_memory_anon_bytes": memory["anon_bytes"],
                "cgroup_memory_file_bytes": memory["file_bytes"],
                "cgroup_memory_slab_bytes": memory["slab_bytes"],
                "smaps_rollup_rss_bytes": smaps["rss_bytes"],
                "smaps_rollup_pss_anon_bytes": smaps["pss_anon_bytes"],
                "smaps_rollup_pss_file_bytes": smaps["pss_file_bytes"],
                "process_threads": process_threads(pid),
                "process_fd_count": process_fd_count(pid),
                "jvm_heap_available": heap.get("available", False),
                "jvm_heap_used_bytes": heap.get("used_bytes"),
                "jvm_heap_committed_bytes": heap.get("committed_bytes"),
                "jvm_heap_max_bytes": heap.get("max_bytes"),
            }
            json_file.write(json.dumps(row, sort_keys=True) + "\n")
            json_file.flush()
            writer.writerow(row)
            csv_file.flush()
            previous_cpu, previous_process_ticks, previous_time = (
                current_cpu,
                current_process_ticks,
                time.monotonic(),
            )
            time.sleep(max(args.interval - (time.monotonic() - previous_time), 0.0))
    return 0


if __name__ == "__main__":
    sys.exit(main())
