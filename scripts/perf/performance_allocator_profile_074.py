#!/usr/bin/env python3
"""Run and validate the preregistered W9e dedicated-Linux allocator matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys
from typing import Any


PROFILE_ID = "w9e-linux-allocator-profile-074-v1"
SCHEMA_VERSION = "hydracache-w9e-linux-allocator-profile-v1"
ALLOCATORS = ("system", "mimalloc", "jemalloc")
NO_PURGE_PHASES = ("cold", "fill", "steady_read", "delete", "refill", "post_idle")
PURGE_PHASES = NO_PURGE_PHASES + ("pre_purge", "post_purge", "second_refill")
EXPECTED_OPERATIONS = (0, 16_384, 81_920, 98_304, 114_688, 114_688)
EXPECTED_ENTRIES = (0, 16_384, 16_384, 0, 16_384, 16_384)
PROCESS_FIELDS = (
    "working_set_bytes",
    "peak_working_set_bytes",
    "pss_bytes",
    "private_clean_bytes",
    "private_dirty_bytes",
    "page_fault_count",
    "voluntary_context_switches",
    "involuntary_context_switches",
    "thread_count",
    "user_cpu_ns",
    "system_cpu_ns",
)
NATIVE_FIELDS = (
    "allocated_or_live",
    "active_or_committed",
    "resident",
    "retained_or_reserved",
)


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def schedule_for(repeats: int) -> list[tuple[str, str]]:
    schedule: list[tuple[str, str]] = []
    for repeat in range(repeats):
        rotation = ALLOCATORS[repeat % len(ALLOCATORS) :] + ALLOCATORS[: repeat % len(ALLOCATORS)]
        if repeat % 2:
            rotation = tuple(reversed(rotation))
        schedule.extend((allocator, "no-purge") for allocator in rotation)
    schedule.extend(("mimalloc", "purge") for _ in range(repeats))
    return schedule


def validate_metric(metric: Any, label: str) -> None:
    require(isinstance(metric, dict), f"{label} metric missing")
    require(isinstance(metric.get("bytes"), int) and metric["bytes"] >= 0, f"{label} units invalid")
    require(bool(metric.get("source")), f"{label} source missing")
    require(bool(metric.get("semantics")), f"{label} semantics missing")


def validate_native(native: dict[str, Any], allocator: str) -> None:
    unavailable = native.get("unavailable")
    require(isinstance(unavailable, dict), "native unavailable map missing")
    if allocator == "system":
        require(native.get("provider") == "system-glibc-mallinfo2", "glibc provider changed")
        require(native.get("status") == "partial-native-statistics", "glibc status changed")
        validate_metric(native.get("allocated_or_live"), "glibc allocated")
        validate_metric(native.get("active_or_committed"), "glibc active")
        validate_metric(native.get("retained_or_reserved"), "glibc retained")
        require(native.get("resident") is None and unavailable.get("resident"), "glibc resident was imputed")
        require(isinstance(native.get("raw"), dict), "glibc mallinfo2 payload missing")
    elif allocator == "mimalloc":
        require(native.get("provider") == "mimalloc", "mimalloc provider changed")
        require(native.get("status") == "partial-native-statistics", "mimalloc status changed")
        require(native.get("allocated_or_live") is None and unavailable.get("allocated_or_live"), "mimalloc live bytes were imputed")
        validate_metric(native.get("active_or_committed"), "mimalloc committed")
        validate_metric(native.get("retained_or_reserved"), "mimalloc reserved")
        require(native.get("resident") is None and unavailable.get("resident"), "mimalloc resident was imputed")
        require(isinstance(native.get("raw"), dict), "mimalloc raw payload missing")
    else:
        require(native.get("provider") == "jemalloc", "jemalloc provider changed")
        require(native.get("status") == "available", "jemalloc status changed")
        for field in NATIVE_FIELDS:
            validate_metric(native.get(field), f"jemalloc {field}")
        require(isinstance(native.get("arenas"), int) and native["arenas"] > 0, "jemalloc arena count missing")
    require(native.get("thread_caches") is None and unavailable.get("thread_caches"), "thread-cache limitation not explicit")


def validate_receipt(receipt: dict[str, Any], allocator: str, mode: str) -> None:
    require(receipt.get("schema_version") == SCHEMA_VERSION, "schema changed")
    require(receipt.get("profile_id") == PROFILE_ID, "profile changed")
    require(receipt.get("allocator") == allocator, "allocator identity changed")
    require(receipt.get("allocator_feature") == f"allocator-{allocator}", "feature identity changed")
    require(receipt.get("target") == "x86_64-linux", "dedicated Linux target changed")
    require(receipt.get("mode") == mode, "mode changed")
    require(receipt.get("workload_seed") == 74_009, "seed changed")
    require(receipt.get("cardinality") == 16_384, "cardinality changed")
    require(receipt.get("payload_bytes") == 4_096, "payload changed")
    require(receipt.get("steady_read_operations") == 65_536, "read volume changed")
    require(receipt.get("idle_milliseconds") == 2_000, "idle interval changed")
    require(isinstance(receipt.get("executable_bytes"), int) and receipt["executable_bytes"] > 0, "binary size missing")
    require(receipt.get("promotable") is False, "attribution receipt became promotable")
    expected_phases = PURGE_PHASES if mode == "purge" else NO_PURGE_PHASES
    expected_operations = EXPECTED_OPERATIONS + ((114_688, 114_688, 131_072) if mode == "purge" else ())
    expected_entries = EXPECTED_ENTRIES + ((16_384, 16_384, 32_768) if mode == "purge" else ())
    phases = receipt.get("phases")
    require(isinstance(phases, list), "phases missing")
    require(tuple(phase.get("phase") for phase in phases) == expected_phases, "phase order changed")
    require(tuple(phase.get("completed_operations") for phase in phases) == expected_operations, "operation trace changed")
    require(tuple(phase.get("logical_entries") for phase in phases) == expected_entries, "logical cardinality changed")
    for phase in phases:
        process = phase.get("process")
        require(isinstance(process, dict), "process snapshot missing")
        require(process.get("source") == "linux-procfs-and-getrusage", "process source changed")
        for field in PROCESS_FIELDS:
            require(isinstance(process.get(field), int) and process[field] >= 0, f"process {field} missing")
        require(process.get("private_commit_bytes") is None, "Windows commit was fabricated on Linux")
        require(process.get("unavailable", {}).get("private_commit_bytes"), "commit limitation missing")
        validate_native(phase.get("allocator_native", {}), allocator)
    invariants = receipt.get("invariants", {})
    for field in ("exact_phase_order", "exact_logical_cardinality", "payload_shape_preserved"):
        require(invariants.get(field) is True, f"{field} invariant failed")
    require(invariants.get("rss_used_as_native_substitute") is False, "RSS substituted a native field")
    if mode == "purge":
        require(allocator == "mimalloc", "purge allocator changed")
        require(invariants.get("purge_requested") is True, "purge request missing")
        require(invariants.get("purge_api_invoked") is True, "purge API was not invoked")
        require(invariants.get("purge_calls_delta", 0) > 0, "purge counter did not advance")
        require(invariants.get("second_refill_completed") is True, "second refill missing")


def run_attempt(binary: Path, allocator: str, mode: str, ordinal: int, output: Path) -> dict[str, Any]:
    command = [str(binary)] + (["--mode", "purge"] if mode == "purge" else [])
    completed = subprocess.run(command, capture_output=True, check=False)
    stem = f"{ordinal:02d}-{allocator}-{mode}"
    stdout_path = output / f"{stem}.stdout.json"
    stderr_path = output / f"{stem}.stderr.txt"
    stdout_path.write_bytes(completed.stdout)
    stderr_path.write_bytes(completed.stderr)
    record: dict[str, Any] = {
        "ordinal": ordinal,
        "allocator": allocator,
        "mode": mode,
        "command": command,
        "exit_code": completed.returncode,
        "stdout": stdout_path.name,
        "stdout_sha256": sha256(stdout_path),
        "stderr": stderr_path.name,
        "stderr_sha256": sha256(stderr_path),
        "stderr_bytes": len(completed.stderr),
        "valid": False,
    }
    try:
        require(completed.returncode == 0, f"process exited {completed.returncode}")
        require(not completed.stderr, "stderr is non-empty")
        receipt = json.loads(completed.stdout)
        require(isinstance(receipt, dict), "receipt is not an object")
        validate_receipt(receipt, allocator, mode)
        record["valid"] = True
        record["elapsed_ns"] = receipt["elapsed_ns"]
        return {"record": record, "receipt": receipt}
    except (UnicodeDecodeError, json.JSONDecodeError, ValueError) as error:
        record["error"] = str(error)
        return {"record": record, "receipt": None}


def nested(receipt: dict[str, Any], phase: str, path: tuple[str, ...]) -> int:
    value: Any = next(item for item in receipt["phases"] if item["phase"] == phase)
    for part in path:
        value = value[part]
    if not isinstance(value, int):
        raise ValueError(f"metric {phase}/{'.'.join(path)} is not an integer")
    return value


def phase_medians(receipts: list[dict[str, Any]], phases: tuple[str, ...]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    previous_phase: str | None = None
    for phase in phases:
        values: dict[str, Any] = {}
        for name in PROCESS_FIELDS:
            values[name] = int(statistics.median(nested(item, phase, ("process", name)) for item in receipts))
        values["completed_operations"] = int(statistics.median(nested(item, phase, ("completed_operations",)) for item in receipts))
        values["elapsed_ns"] = int(statistics.median(nested(item, phase, ("elapsed_ns",)) for item in receipts))
        if previous_phase is not None:
            operations = values["completed_operations"] - result[previous_phase]["completed_operations"]
            cpu_delta = (
                values["user_cpu_ns"] + values["system_cpu_ns"]
                - result[previous_phase]["user_cpu_ns"]
                - result[previous_phase]["system_cpu_ns"]
            )
            elapsed_delta = values["elapsed_ns"] - result[previous_phase]["elapsed_ns"]
            values["cpu_ns_per_operation"] = None if operations == 0 else cpu_delta / operations
            values["elapsed_ns_per_operation"] = None if operations == 0 else elapsed_delta / operations
        result[phase] = values
        previous_phase = phase
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    for allocator in ALLOCATORS:
        parser.add_argument(f"--{allocator}-binary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--tool-source", type=Path, required=True)
    parser.add_argument("--tool-lock", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=5)
    args = parser.parse_args()

    require(sys.platform.startswith("linux"), "W9e must run on Linux")
    require(args.repeats == 5, "W9e requires exactly five repeats")
    require(len(args.source_commit) == 40 and all(char in "0123456789abcdef" for char in args.source_commit), "source commit must be a full lowercase SHA")
    binaries = {allocator: getattr(args, f"{allocator}_binary").resolve() for allocator in ALLOCATORS}
    for path in [*binaries.values(), args.tool_source, args.tool_lock, args.contract]:
        require(path.is_file(), f"missing input: {path}")
    if args.output_dir.exists():
        require(not any(args.output_dir.iterdir()), "output directory must be new or empty")
    else:
        args.output_dir.mkdir(parents=True)

    schedule = schedule_for(args.repeats)
    attempts: list[dict[str, Any]] = []
    receipts: dict[tuple[str, str], list[dict[str, Any]]] = {}
    for ordinal, (allocator, mode) in enumerate(schedule, start=1):
        outcome = run_attempt(binaries[allocator], allocator, mode, ordinal, args.output_dir)
        attempts.append(outcome["record"])
        if outcome["receipt"] is not None:
            receipts.setdefault((allocator, mode), []).append(outcome["receipt"])
        write_json(args.output_dir / "attempts.json", attempts)

    manifest = {
        "schema_version": "hydracache-w9e-linux-allocator-matrix-v1",
        "profile_id": PROFILE_ID,
        "source_commit": args.source_commit,
        "tool_source_sha256": sha256(args.tool_source),
        "tool_lock_sha256": sha256(args.tool_lock),
        "contract_sha256": sha256(args.contract),
        "binary_sha256": {allocator: sha256(path) for allocator, path in binaries.items()},
        "binary_bytes": {allocator: path.stat().st_size for allocator, path in binaries.items()},
        "repeats": args.repeats,
        "schedule": [{"allocator": allocator, "mode": mode} for allocator, mode in schedule],
        "attempts": attempts,
        "failed_attempts": sum(not attempt["valid"] for attempt in attempts),
        "candidate_data_present": False,
        "acceptance_decision_allowed": False,
        "promotable": False,
    }
    write_json(args.output_dir / "manifest.json", manifest)
    if manifest["failed_attempts"]:
        print(f"W9e matrix retained {manifest['failed_attempts']} failed attempts", file=sys.stderr)
        return 1

    require(all(len(receipts[(allocator, "no-purge")]) == 5 for allocator in ALLOCATORS), "allocator receipt volume changed")
    require(len(receipts[("mimalloc", "purge")]) == 5, "purge receipt volume changed")
    summary = {
        "schema_version": "hydracache-w9e-linux-allocator-summary-v1",
        "profile_id": PROFILE_ID,
        "attempts": len(attempts),
        "failed_attempts": 0,
        "allocators": {
            allocator: phase_medians(receipts[(allocator, "no-purge")], NO_PURGE_PHASES)
            for allocator in ALLOCATORS
        },
        "mimalloc_purge": phase_medians(receipts[("mimalloc", "purge")], PURGE_PHASES),
        "elapsed_ns_median": {
            allocator: int(statistics.median(item["elapsed_ns"] for item in receipts[(allocator, "no-purge")]))
            for allocator in ALLOCATORS
        },
        "executable_bytes": {
            allocator: int(statistics.median(item["executable_bytes"] for item in receipts[(allocator, "no-purge")]))
            for allocator in ALLOCATORS
        },
        "claim_boundary": "dedicated-Linux owner attribution only; no allocator winner, default change, product candidate, or release claim",
    }
    write_json(args.output_dir / "summary.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"W9e allocator matrix: {error}", file=sys.stderr)
        raise SystemExit(2)
