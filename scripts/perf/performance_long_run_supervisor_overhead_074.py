#!/usr/bin/env python3
"""Collect a bounded read-only idle-overhead receipt for the W11 supervisor."""

from __future__ import annotations

import argparse
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
import json
import os
from pathlib import Path
import re
import sys
import time
from typing import Callable


SCHEMA_VERSION = "hydracache-w11-supervisor-idle-overhead-v1"
MAX_DURATION_SECONDS = 300.0
MIN_INTERVAL_SECONDS = 0.05
MAX_INTERVAL_SECONDS = 10.0
HASH = re.compile(r"^[0-9a-f]{40}$")


@dataclass(frozen=True)
class SupervisorSnapshot:
    monotonic_ns: int
    start_ticks: int
    cpu_usage_usec: int
    cpu_user_usec: int
    cpu_system_usec: int
    rss_bytes: int
    memory_current_bytes: int
    memory_peak_bytes: int
    read_bytes: int | None
    write_bytes: int | None
    cpu_pressure_some_usec: int
    cpu_pressure_full_usec: int
    voluntary_context_switches: int
    involuntary_context_switches: int
    threads: int
    pids_current: int
    cpuset: str


def _fields(path: Path, separator: str = ":") -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, found, value = line.partition(separator)
        if found:
            result[key.strip()] = value.strip()
    return result


def _kilobytes(value: str) -> int:
    match = re.fullmatch(r"([0-9]+) kB", value)
    if match is None:
        raise ValueError(f"invalid proc memory value: {value}")
    return int(match.group(1)) * 1024


def _single_integer(path: Path) -> int:
    value = path.read_text(encoding="utf-8").strip()
    if not value.isdecimal():
        raise ValueError(f"invalid integer counter: {path}")
    return int(value)


def _cgroup_io(path: Path) -> tuple[int, int]:
    read_bytes = 0
    write_bytes = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.split()
        if len(fields) < 2:
            raise ValueError("cgroup io.stat is malformed")
        counters = dict(field.split("=", 1) for field in fields[1:])
        read_bytes += int(counters.get("rbytes", "0"))
        write_bytes += int(counters.get("wbytes", "0"))
    return read_bytes, write_bytes


def _pressure(path: Path) -> tuple[int, int]:
    totals: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.split()
        if not fields:
            continue
        values = dict(field.split("=", 1) for field in fields[1:])
        if "total" not in values:
            raise ValueError("cgroup pressure record lacks total")
        totals[fields[0]] = int(values["total"])
    if "some" not in totals:
        raise ValueError("cgroup pressure data lacks some total")
    return totals["some"], totals.get("full", 0)


def read_supervisor_snapshot(
    proc_root: Path,
    cgroup: Path,
    pid: int,
    monotonic_ns: Callable[[], int] = time.monotonic_ns,
) -> SupervisorSnapshot:
    process = proc_root / str(pid)
    stat_line = (process / "stat").read_text(encoding="utf-8").strip()
    closing = stat_line.rfind(")")
    if closing < 0 or not stat_line.startswith(f"{pid} ("):
        raise ValueError("process stat identity is malformed")
    stat_fields = stat_line[closing + 2 :].split()
    if len(stat_fields) < 20:
        raise ValueError("process stat is truncated")
    status = _fields(process / "status")
    cpu = _fields(cgroup / "cpu.stat", separator=" ")
    io_stat = cgroup / "io.stat"
    read_bytes, write_bytes = _cgroup_io(io_stat) if io_stat.is_file() else (None, None)
    pressure_some, pressure_full = _pressure(cgroup / "cpu.pressure")
    threads = int(status["Threads"])
    pids_current = _single_integer(cgroup / "pids.current")
    if threads <= 0 or pids_current <= 0:
        raise ValueError("supervisor task counters are invalid")
    return SupervisorSnapshot(
        monotonic_ns=monotonic_ns(),
        start_ticks=int(stat_fields[19]),
        cpu_usage_usec=int(cpu["usage_usec"]),
        cpu_user_usec=int(cpu["user_usec"]),
        cpu_system_usec=int(cpu["system_usec"]),
        rss_bytes=_kilobytes(status["VmRSS"]),
        memory_current_bytes=_single_integer(cgroup / "memory.current"),
        memory_peak_bytes=_single_integer(cgroup / "memory.peak"),
        read_bytes=read_bytes,
        write_bytes=write_bytes,
        cpu_pressure_some_usec=pressure_some,
        cpu_pressure_full_usec=pressure_full,
        voluntary_context_switches=int(status["voluntary_ctxt_switches"]),
        involuntary_context_switches=int(status["nonvoluntary_ctxt_switches"]),
        threads=threads,
        pids_current=pids_current,
        cpuset=status["Cpus_allowed_list"],
    )


def _delta(first: SupervisorSnapshot, last: SupervisorSnapshot, field: str) -> int:
    value = getattr(last, field) - getattr(first, field)
    if value < 0:
        raise ValueError(f"supervisor counter decreased: {field}")
    return value


def _optional_delta(
    first: SupervisorSnapshot, last: SupervisorSnapshot, field: str
) -> int | None:
    first_value = getattr(first, field)
    last_value = getattr(last, field)
    if first_value is None or last_value is None:
        return None
    value = last_value - first_value
    if value < 0:
        raise ValueError(f"supervisor counter decreased: {field}")
    return value


def summarize(
    samples: list[SupervisorSnapshot],
    maximum_cpu_percent: float,
    maximum_rss_bytes: int,
    maximum_io_bytes_per_second: int,
) -> dict[str, object]:
    if len(samples) < 2:
        raise ValueError("at least two valid samples are required")
    first, last = samples[0], samples[-1]
    if any(sample.start_ticks != first.start_ticks for sample in samples):
        raise ValueError("supervisor process identity changed during observation")
    if any(sample.cpuset != first.cpuset for sample in samples):
        raise ValueError("supervisor cpuset changed during observation")
    elapsed_seconds = (last.monotonic_ns - first.monotonic_ns) / 1_000_000_000
    if elapsed_seconds <= 0:
        raise ValueError("observation clock did not advance")
    cpu_seconds = _delta(first, last, "cpu_usage_usec") / 1_000_000
    read_bytes = _optional_delta(first, last, "read_bytes")
    write_bytes = _optional_delta(first, last, "write_bytes")
    io_bytes = (
        read_bytes + write_bytes
        if read_bytes is not None and write_bytes is not None
        else None
    )
    cpu_pressure_some = _delta(first, last, "cpu_pressure_some_usec")
    cpu_pressure_full = _delta(first, last, "cpu_pressure_full_usec")
    cpu_percent = cpu_seconds * 100.0 / elapsed_seconds
    io_bytes_per_second = io_bytes / elapsed_seconds if io_bytes is not None else None
    maximum_observed_rss = max(sample.rss_bytes for sample in samples)
    cpu_budget_passed = cpu_percent <= maximum_cpu_percent
    rss_budget_passed = maximum_observed_rss <= maximum_rss_bytes
    io_budget_passed = (
        io_bytes_per_second <= maximum_io_bytes_per_second
        if io_bytes_per_second is not None
        else None
    )
    within_screen = cpu_budget_passed and rss_budget_passed and io_budget_passed is True
    return {
        "sample_count": len(samples),
        "elapsed_seconds": elapsed_seconds,
        "stable_start_ticks": first.start_ticks,
        "stable_cpuset": first.cpuset,
        "cpu_seconds": cpu_seconds,
        "cpu_percent": cpu_percent,
        "cpu_user_usec_delta": _delta(first, last, "cpu_user_usec"),
        "cpu_system_usec_delta": _delta(first, last, "cpu_system_usec"),
        "rss_bytes_max": maximum_observed_rss,
        "cgroup_memory_current_bytes_max": max(
            sample.memory_current_bytes for sample in samples
        ),
        "cgroup_memory_peak_bytes_max": max(sample.memory_peak_bytes for sample in samples),
        "read_bytes_delta": read_bytes,
        "write_bytes_delta": write_bytes,
        "io_bytes_per_second": io_bytes_per_second,
        "io_counter_source": "cgroup-v2-io.stat" if io_bytes is not None else None,
        "cpu_pressure_some_usec_delta": cpu_pressure_some,
        "cpu_pressure_full_usec_delta": cpu_pressure_full,
        "cpu_pressure_some_ratio": cpu_pressure_some / (elapsed_seconds * 1_000_000),
        "cpu_pressure_full_ratio": cpu_pressure_full / (elapsed_seconds * 1_000_000),
        "voluntary_context_switches_delta": _delta(
            first, last, "voluntary_context_switches"
        ),
        "involuntary_context_switches_delta": _delta(
            first, last, "involuntary_context_switches"
        ),
        "threads_max": max(sample.threads for sample in samples),
        "cgroup_pids_max": max(sample.pids_current for sample in samples),
        "cpu_budget_passed": cpu_budget_passed,
        "rss_budget_passed": rss_budget_passed,
        "io_budget_evaluated": io_budget_passed is not None,
        "io_budget_passed": io_budget_passed,
        "cpu_rss_partial_screen_passed": cpu_budget_passed and rss_budget_passed,
        "idle_screen_passed": within_screen,
    }


def collect(
    pid: int,
    cgroup: Path,
    duration_seconds: float,
    interval_seconds: float,
    proc_root: Path = Path("/proc"),
) -> list[SupervisorSnapshot]:
    deadline = time.monotonic() + duration_seconds
    samples = [read_supervisor_snapshot(proc_root, cgroup, pid)]
    while time.monotonic() < deadline:
        time.sleep(min(interval_seconds, max(0.0, deadline - time.monotonic())))
        samples.append(read_supervisor_snapshot(proc_root, cgroup, pid))
    return samples


def resolve_cgroup(cgroup_root: Path, control_group: str) -> Path:
    if not control_group.startswith("/") or ".." in Path(control_group).parts:
        raise ValueError("control group must be an absolute canonical cgroup path")
    root = cgroup_root.resolve(strict=True)
    candidate = (root / control_group.lstrip("/")).resolve(strict=True)
    if root not in candidate.parents:
        raise ValueError("control group escaped the cgroup root")
    if not candidate.is_dir():
        raise ValueError("control group is not a directory")
    return candidate


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--installed-source-commit", required=True)
    parser.add_argument("--pid", required=True, type=int)
    parser.add_argument("--control-group", required=True)
    parser.add_argument("--duration-seconds", required=True, type=float)
    parser.add_argument("--interval-seconds", required=True, type=float)
    parser.add_argument("--maximum-cpu-percent", required=True, type=float)
    parser.add_argument("--maximum-rss-bytes", required=True, type=int)
    parser.add_argument("--maximum-io-bytes-per-second", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        raise SystemExit("W11 supervisor overhead collection requires Linux")
    if not HASH.fullmatch(args.source_commit) or not HASH.fullmatch(
        args.installed_source_commit
    ):
        raise SystemExit("source commits must be full lowercase SHAs")
    if args.pid <= 1:
        raise SystemExit("supervisor PID must be greater than one")
    if not 0 < args.duration_seconds <= MAX_DURATION_SECONDS:
        raise SystemExit("duration is outside the bounded screen")
    if not MIN_INTERVAL_SECONDS <= args.interval_seconds <= MAX_INTERVAL_SECONDS:
        raise SystemExit("interval is outside the bounded screen")
    if args.interval_seconds >= args.duration_seconds:
        raise SystemExit("interval must be shorter than duration")
    if (
        args.maximum_cpu_percent <= 0
        or args.maximum_rss_bytes <= 0
        or args.maximum_io_bytes_per_second <= 0
    ):
        raise SystemExit("screen budgets must be positive")
    if args.output.exists():
        raise SystemExit("output must not pre-exist")

    cgroup = resolve_cgroup(Path("/sys/fs/cgroup"), args.control_group)
    started_at = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    samples = collect(args.pid, cgroup, args.duration_seconds, args.interval_seconds)
    summary = summarize(
        samples,
        args.maximum_cpu_percent,
        args.maximum_rss_bytes,
        args.maximum_io_bytes_per_second,
    )
    receipt = {
        "schema_version": SCHEMA_VERSION,
        "source_commit": args.source_commit,
        "installed_source_commit": args.installed_source_commit,
        "started_at_utc": started_at,
        "completed_at_utc": datetime.now(timezone.utc)
        .isoformat()
        .replace("+00:00", "Z"),
        "read_only": True,
        "mutation_performed": False,
        "product_candidate_started": False,
        "workload_started": False,
        "pid": args.pid,
        "control_group": args.control_group,
        "requested_duration_seconds": args.duration_seconds,
        "requested_interval_seconds": args.interval_seconds,
        "budgets": {
            "maximum_cpu_percent": args.maximum_cpu_percent,
            "maximum_rss_bytes": args.maximum_rss_bytes,
            "maximum_io_bytes_per_second": args.maximum_io_bytes_per_second,
        },
        "summary": summary,
        "samples": [asdict(sample) for sample in samples],
        "decision": {
            "idle_screen_only": True,
            "role_overhead_qualification_complete": False,
            "release_admission_allowed": False,
        },
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(args.output, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as output:
            json.dump(receipt, output, indent=2, sort_keys=True)
            output.write("\n")
            output.flush()
            os.fsync(output.fileno())
    finally:
        try:
            os.close(descriptor)
        except OSError:
            pass
    if not summary["cpu_rss_partial_screen_passed"] or summary["io_budget_passed"] is False:
        raise SystemExit("supervisor idle overhead exceeded the frozen screen budget")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
