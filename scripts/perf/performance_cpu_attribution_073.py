#!/usr/bin/env python3
"""Run the non-promotable 0.73 instrumentation CPU attribution."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import statistics
import subprocess
import sys


PROFILE = "observer-cpu-attribution-073-v1"
MODES = ["off", "counters-only", "observer-noop", "production"]
WILLIAMS_ORDERS = [
    ["off", "counters-only", "production", "observer-noop"],
    ["counters-only", "observer-noop", "off", "production"],
    ["observer-noop", "production", "counters-only", "off"],
    ["production", "off", "observer-noop", "counters-only"],
]


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def median(values: list[float]) -> float:
    return float(statistics.median(values))


def run_attempt(
    binary: pathlib.Path,
    output: pathlib.Path,
    cpu_set: str,
    mode: str,
    rate: int,
    operations: int,
    warmup_operations: int,
) -> dict:
    output.mkdir(parents=True)
    receipt_path = output / "receipt.json"
    command = [
        "taskset",
        "--cpu-list",
        cpu_set,
        str(binary),
        "--profile-id",
        PROFILE,
        "--mode",
        mode,
        "--rate",
        str(rate),
        "--operations",
        str(operations),
        "--warmup-operations",
        str(warmup_operations),
        "--output",
        str(receipt_path),
    ]
    process = subprocess.run(command, capture_output=True, check=False)
    (output / "stdout.txt").write_bytes(process.stdout)
    (output / "stderr.txt").write_bytes(process.stderr)
    attempt = {
        "mode": mode,
        "exit_code": process.returncode,
        "stdout_sha256": sha256(output / "stdout.txt"),
        "stderr_sha256": sha256(output / "stderr.txt"),
        "receipt_sha256": None,
        "result": "failed",
    }
    if process.returncode == 0 and receipt_path.is_file():
        attempt["receipt_sha256"] = sha256(receipt_path)
        attempt["result"] = "success"
    (output / "attempt.json").write_text(
        json.dumps(attempt, indent=2) + "\n", encoding="utf-8"
    )
    return attempt


def validate_receipt(receipt: dict, mode: str, rate: int, operations: int) -> None:
    observation = receipt["observation"]
    complete_mode = mode in ("off", "production")
    exact_mode = mode in ("off", "production")
    if (
        receipt["schema_version"] != 1
        or receipt["release"] != "0.73"
        or receipt["profile_id"] != PROFILE
        or receipt["instrumentation_mode"] != mode
        or receipt["offered_rate_per_second"] != rate
        or receipt["operations"] != operations
        or receipt["promotable"]
        or receipt["correctness_complete"] != complete_mode
        or receipt["reconciliation_exact"] != exact_mode
        or observation["offered"] != operations
        or observation["started"] != operations
        or observation["completed"] != operations
        or observation["successes"] != operations
        or observation["errors"] != 0
        or observation["timeouts"] != 0
        or observation["rejections"] != 0
        or not observation["backlog_drained"]
    ):
        raise ValueError(f"invalid or incomplete {mode} attribution receipt")


def summarize(receipts: list[dict]) -> dict:
    return {
        "cpu_seconds_per_operation_median": median(
            [value["cpu_seconds_per_operation"] for value in receipts]
        ),
        "allocated_bytes_per_operation_median": median(
            [value["gross_allocated_bytes_per_operation"] for value in receipts]
        ),
        "goodput_median": median(
            [value["observation"]["achieved_rate_per_second"] for value in receipts]
        ),
        "p99_us_median": median(
            [float(value["observation"]["latency"]["p99_us"]) for value in receipts]
        ),
    }


def comparison(control: dict, treatment: dict) -> dict:
    control_cpu = control["cpu_seconds_per_operation_median"]
    treatment_cpu = treatment["cpu_seconds_per_operation_median"]
    return {
        "cpu_seconds_per_operation_absolute_delta": treatment_cpu - control_cpu,
        "cpu_per_operation_relative_delta": (treatment_cpu - control_cpu) / control_cpu,
        "allocation_bytes_per_operation_absolute_delta": treatment[
            "allocated_bytes_per_operation_median"
        ]
        - control["allocated_bytes_per_operation_median"],
        "p99_us_absolute_delta": treatment["p99_us_median"] - control["p99_us_median"],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--rate", type=int, required=True)
    parser.add_argument("--repeats", type=int, required=True)
    parser.add_argument("--window-seconds", type=int, required=True)
    parser.add_argument("--warmup-operations", type=int, required=True)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--cpu-set", required=True)
    parser.add_argument("--source-sha", required=True)
    options = parser.parse_args()
    if (
        not options.binary.is_file()
        or options.output.exists()
        or options.rate <= 0
        or options.repeats < 4
        or options.window_seconds < 5
    ):
        raise SystemExit("invalid attribution binary, output, rate, repeats, or window")
    options.output.mkdir(parents=True)
    operations = options.rate * options.window_seconds
    attempts = []
    failed = []
    receipts: dict[str, list[dict]] = {mode: [] for mode in MODES}
    receipts_by_repeat: dict[int, dict[str, dict]] = {}
    offset = options.seed % len(WILLIAMS_ORDERS)
    for repeat in range(1, options.repeats + 1):
        order = WILLIAMS_ORDERS[(offset + repeat - 1) % len(WILLIAMS_ORDERS)]
        receipts_by_repeat[repeat] = {}
        for position, mode in enumerate(order, 1):
            attempt_id = f"repeat-{repeat:02}-{position}-{mode}"
            attempt_dir = options.output / "attempts" / attempt_id
            attempt = run_attempt(
                options.binary.resolve(),
                attempt_dir,
                options.cpu_set,
                mode,
                options.rate,
                operations,
                options.warmup_operations,
            )
            attempt.update(
                {"attempt_id": attempt_id, "repeat": repeat, "position": position}
            )
            attempts.append(attempt)
            if attempt["result"] != "success":
                failed.append(attempt_id)
                continue
            receipt = json.loads((attempt_dir / "receipt.json").read_text(encoding="utf-8"))
            validate_receipt(receipt, mode, options.rate, operations)
            receipts[mode].append(receipt)
            receipts_by_repeat[repeat][mode] = receipt

    summaries = {
        mode: summarize(values) if len(values) == options.repeats else None
        for mode, values in receipts.items()
    }
    comparisons = {}
    if all(summary is not None for summary in summaries.values()):
        pairs = [
            ("off_to_counters_only", "off", "counters-only"),
            ("counters_only_to_observer_noop", "counters-only", "observer-noop"),
            ("observer_noop_to_production", "observer-noop", "production"),
            ("off_to_production", "off", "production"),
        ]
        comparisons = {
            name: comparison(summaries[control], summaries[treatment])
            for name, control, treatment in pairs
        }

    paired_cpu = []
    for repeat, by_mode in receipts_by_repeat.items():
        if len(by_mode) != len(MODES):
            continue
        paired_cpu.append(
            {
                "repeat": repeat,
                "microseconds_per_operation": {
                    mode: by_mode[mode]["cpu_seconds_per_operation"] * 1_000_000
                    for mode in MODES
                },
            }
        )

    aggregate = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE,
        "evidence_class": "dedicated_host_diagnostic_only",
        "source_sha": options.source_sha,
        "binary_sha256": sha256(options.binary),
        "rate": options.rate,
        "repeats": options.repeats,
        "window_seconds": options.window_seconds,
        "warmup_operations": options.warmup_operations,
        "seed": options.seed,
        "modes": MODES,
        "attempts": attempts,
        "failed_attempts": failed,
        "mode_summaries": summaries,
        "comparisons": comparisons,
        "paired_cpu_observations": paired_cpu,
        "candidate_data_present": False,
        "candidate_measurement_authorized": False,
        "promotable": False,
        "numerical_claim_eligible": False,
        "thresholds_changed": False,
        "diagnostic_complete": not failed
        and all(len(values) == options.repeats for values in receipts.values()),
    }
    (options.output / "cpu-attribution.json").write_text(
        json.dumps(aggregate, indent=2) + "\n", encoding="utf-8"
    )
    if not aggregate["diagnostic_complete"]:
        print(f"CPU attribution retained {len(failed)} failed attempts", file=sys.stderr)
        return 1
    print("CPU attribution completed without applying an acceptance threshold", file=sys.stderr)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
