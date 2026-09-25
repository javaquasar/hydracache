#!/usr/bin/env python3
"""Run and validate the preregistered local W8 allocator matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import statistics
import subprocess
import sys
from typing import Any


NO_PURGE_PHASES = ["cold", "fill", "steady_read", "delete", "refill", "post_idle"]
PURGE_PHASES = NO_PURGE_PHASES + ["pre_purge", "post_purge", "second_refill"]


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


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def validate_receipt(receipt: dict[str, Any], allocator: str, mode: str) -> None:
    require(receipt.get("schema_version") == "hydracache-w8-allocator-profile-v1", "schema changed")
    require(receipt.get("profile_id") == "w8-allocator-profile-073-v1", "profile changed")
    require(receipt.get("allocator") == allocator, "allocator identity changed")
    require(receipt.get("allocator_feature") == f"allocator-{allocator}", "feature identity changed")
    require(receipt.get("mode") == mode, "mode changed")
    require(receipt.get("cardinality") == 16_384, "cardinality changed")
    require(receipt.get("payload_bytes") == 4_096, "payload changed")
    require(receipt.get("steady_read_operations") == 65_536, "read volume changed")
    require(receipt.get("idle_milliseconds") == 2_000, "idle interval changed")
    require(receipt.get("promotable") is False, "local receipt became promotable")
    expected_phases = PURGE_PHASES if mode == "purge" else NO_PURGE_PHASES
    phases = receipt.get("phases")
    require(isinstance(phases, list), "phases are missing")
    require([phase.get("phase") for phase in phases] == expected_phases, "phase order changed")
    expected_entries = [0, 16_384, 16_384, 0, 16_384, 16_384]
    if mode == "purge":
        expected_entries += [16_384, 16_384, 32_768]
    require(
        [phase.get("logical_entries") for phase in phases] == expected_entries,
        "logical cardinality changed",
    )
    for phase in phases:
        process = phase.get("process", {})
        for field in [
            "working_set_bytes",
            "peak_working_set_bytes",
            "private_commit_bytes",
            "page_fault_count",
        ]:
            require(isinstance(process.get(field), int) and process[field] >= 0, f"process {field} missing")
        native = phase.get("allocator_native", {})
        require(native.get("provider") == allocator, "native provider changed")
        if allocator == "system":
            require(native.get("status") == "unavailable-with-reason", "system native status changed")
            for field in ["allocated_or_live", "active_or_committed", "resident", "retained_or_reserved"]:
                require(native.get(field) is None, f"system {field} was imputed")
                require(native.get("unavailable", {}).get(field), f"system {field} reason missing")
            require(native.get("raw") is None, "system native raw payload should be absent")
        else:
            require(native.get("status") == "partial-native-statistics", "mimalloc native status changed")
            require(native.get("allocated_or_live") is None, "mimalloc zero live counter was accepted")
            require(native.get("unavailable", {}).get("allocated_or_live"), "mimalloc live reason missing")
            require(native.get("resident") is None, "mimalloc process RSS was accepted as allocator resident")
            require(native.get("unavailable", {}).get("resident"), "mimalloc resident reason missing")
            for field in ["active_or_committed", "retained_or_reserved"]:
                metric = native.get(field)
                require(isinstance(metric, dict), f"mimalloc {field} missing")
                require(isinstance(metric.get("bytes"), int) and metric["bytes"] >= 0, f"mimalloc {field} units invalid")
                require(metric.get("source") and metric.get("semantics"), f"mimalloc {field} semantics missing")
            require(isinstance(native.get("raw"), dict), "mimalloc raw payload missing")
    invariants = receipt.get("invariants", {})
    require(invariants.get("exact_phase_order") is True, "phase invariant failed")
    require(invariants.get("exact_logical_cardinality") is True, "cardinality invariant failed")
    require(invariants.get("payload_shape_preserved") is True, "payload invariant failed")
    require(invariants.get("rss_used_as_native_substitute") is False, "RSS substituted a native field")
    if mode == "purge":
        require(invariants.get("purge_requested") is True, "purge request missing")
        require(invariants.get("purge_api_invoked") is True, "purge API was not invoked")
        require(invariants.get("purge_calls_delta", 0) > 0, "purge call counter did not advance")
        require(invariants.get("second_refill_completed") is True, "second refill missing")


def run_attempt(
    binary: Path,
    allocator: str,
    mode: str,
    ordinal: int,
    output: Path,
) -> dict[str, Any]:
    command = [str(binary)]
    if mode == "purge":
        command += ["--mode", "purge"]
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


def metric(receipt: dict[str, Any], phase: str, path: tuple[str, ...]) -> int:
    value: Any = next(item for item in receipt["phases"] if item["phase"] == phase)
    for part in path:
        value = value[part]
    if not isinstance(value, int):
        raise ValueError(f"metric {phase}/{'.'.join(path)} is not an integer")
    return value


def medians(receipts: list[dict[str, Any]], phases: list[str]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for phase in phases:
        phase_result: dict[str, Any] = {}
        for name, path in {
            "working_set_bytes": ("process", "working_set_bytes"),
            "peak_working_set_bytes": ("process", "peak_working_set_bytes"),
            "private_commit_bytes": ("process", "private_commit_bytes"),
            "page_fault_count": ("process", "page_fault_count"),
        }.items():
            phase_result[name] = int(statistics.median(metric(receipt, phase, path) for receipt in receipts))
        if receipts[0]["allocator"] == "mimalloc":
            for name, path in {
                "native_committed_bytes": ("allocator_native", "active_or_committed", "bytes"),
                "native_reserved_bytes": ("allocator_native", "retained_or_reserved", "bytes"),
            }.items():
                phase_result[name] = int(statistics.median(metric(receipt, phase, path) for receipt in receipts))
        result[phase] = phase_result
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--system-binary", type=Path, required=True)
    parser.add_argument("--mimalloc-binary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--tool-source", type=Path, required=True)
    parser.add_argument("--tool-lock", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=5)
    args = parser.parse_args()

    require(args.repeats == 5, "W8 requires exactly five repeats")
    require(len(args.source_commit) == 40 and all(char in "0123456789abcdef" for char in args.source_commit), "source commit must be full lowercase SHA")
    for path in [args.system_binary, args.mimalloc_binary, args.tool_source, args.tool_lock, args.contract]:
        require(path.is_file(), f"missing input: {path}")
    if args.output_dir.exists():
        require(not any(args.output_dir.iterdir()), "output directory must be new or empty")
    else:
        args.output_dir.mkdir(parents=True)

    schedule: list[tuple[str, str]] = []
    for pair in range(args.repeats):
        order = ["system", "mimalloc"] if pair % 2 == 0 else ["mimalloc", "system"]
        schedule.extend((allocator, "no-purge") for allocator in order)
    schedule.extend(("mimalloc", "purge") for _ in range(args.repeats))

    attempts = []
    receipts: dict[tuple[str, str], list[dict[str, Any]]] = {}
    binaries = {"system": args.system_binary.resolve(), "mimalloc": args.mimalloc_binary.resolve()}
    for ordinal, (allocator, mode) in enumerate(schedule, start=1):
        outcome = run_attempt(binaries[allocator], allocator, mode, ordinal, args.output_dir)
        attempts.append(outcome["record"])
        if outcome["receipt"] is not None:
            receipts.setdefault((allocator, mode), []).append(outcome["receipt"])
        write_json(args.output_dir / "attempts.json", attempts)

    metadata = {
        "schema_version": "hydracache-w8-allocator-matrix-v1",
        "source_commit": args.source_commit,
        "tool_source_sha256": sha256(args.tool_source),
        "tool_lock_sha256": sha256(args.tool_lock),
        "contract_sha256": sha256(args.contract),
        "system_binary_sha256": sha256(args.system_binary),
        "mimalloc_binary_sha256": sha256(args.mimalloc_binary),
        "repeats": args.repeats,
        "schedule": [{"allocator": allocator, "mode": mode} for allocator, mode in schedule],
        "attempts": attempts,
        "failed_attempts": sum(not attempt["valid"] for attempt in attempts),
        "promotable": False,
    }
    write_json(args.output_dir / "manifest.json", metadata)
    if metadata["failed_attempts"]:
        print(f"W8 matrix retained {metadata['failed_attempts']} failed attempts", file=sys.stderr)
        return 1

    system = receipts[("system", "no-purge")]
    mimalloc = receipts[("mimalloc", "no-purge")]
    purge = receipts[("mimalloc", "purge")]
    require(len(system) == len(mimalloc) == len(purge) == 5, "validated receipt volume changed")
    summary = {
        "schema_version": "hydracache-w8-allocator-summary-v1",
        "attempts": len(attempts),
        "failed_attempts": 0,
        "system_no_purge": medians(system, NO_PURGE_PHASES),
        "mimalloc_no_purge": medians(mimalloc, NO_PURGE_PHASES),
        "mimalloc_purge": medians(purge, PURGE_PHASES),
        "elapsed_ns_median": {
            "system_no_purge": int(statistics.median(receipt["elapsed_ns"] for receipt in system)),
            "mimalloc_no_purge": int(statistics.median(receipt["elapsed_ns"] for receipt in mimalloc)),
            "mimalloc_purge": int(statistics.median(receipt["elapsed_ns"] for receipt in purge)),
        },
        "purge_calls_delta_median": int(statistics.median(receipt["invariants"]["purge_calls_delta"] for receipt in purge)),
        "purged_bytes_delta_median": int(statistics.median(receipt["invariants"]["purged_bytes_delta"] for receipt in purge)),
        "claim_boundary": "local Windows screening; native concepts are provider-specific; no release/default claim",
    }
    write_json(args.output_dir / "summary.json", summary)
    print(json.dumps(summary, sort_keys=True))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        print(f"W8 allocator matrix: {error}", file=sys.stderr)
        raise SystemExit(2)
