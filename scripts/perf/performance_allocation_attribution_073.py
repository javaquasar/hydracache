#!/usr/bin/env python3
"""Run the preregistered local-only observer allocation attribution matrix."""

from __future__ import annotations

import argparse
import hashlib
import json
import statistics
import subprocess
import sys
from pathlib import Path


PROFILE_ID = "observer-allocation-attribution-073-v1"
MODES = ["off", "counters-only", "observer-noop", "production"]
SCENARIOS = [
    "mixed",
    "get",
    "replace",
    "remove-refill",
    "tag-invalidate-refill",
    "ttl-put",
]
MODE_ORDERS = [
    ["off", "counters-only", "observer-noop", "production"],
    ["counters-only", "observer-noop", "production", "off"],
    ["observer-noop", "production", "off", "counters-only"],
    ["production", "off", "counters-only", "observer-noop"],
    ["observer-noop", "off", "production", "counters-only"],
]
COMPARISONS = [
    ("off_to_counters_only", "off", "counters-only"),
    ("counters_only_to_observer_noop", "counters-only", "observer-noop"),
    ("observer_noop_to_production", "observer-noop", "production"),
    ("off_to_production", "off", "production"),
]
REPEATS = 5
OPERATIONS = 8_192
RATE = 20_000
WARMUP_OPERATIONS = 4_096
SEED = 73_074


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def summarize(receipts: list[dict]) -> list[dict]:
    summaries = []
    for scenario in SCENARIOS:
        medians = {}
        for mode in MODES:
            values = [
                receipt["gross_allocated_bytes_per_operation"]
                for receipt in receipts
                if receipt["scenario"] == scenario
                and receipt["instrumentation_mode"] == mode
            ]
            if len(values) != REPEATS:
                raise ValueError(f"{scenario}/{mode} retained {len(values)} receipts")
            medians[mode] = statistics.median(values)
        comparisons = [
            {
                "name": name,
                "allocation_bytes_per_operation_delta": medians[right] - medians[left],
            }
            for name, left, right in COMPARISONS
        ]
        summaries.append(
            {
                "scenario": scenario,
                "allocation_bytes_per_operation_median": medians,
                "comparisons": comparisons,
            }
        )
    return summaries


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-sha", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    binary = args.binary.resolve()
    output = args.output.resolve()
    if not binary.is_file():
        raise SystemExit(f"binary does not exist: {binary}")
    if output.exists():
        raise SystemExit(f"append-only output already exists: {output}")
    if len(args.source_sha) != 40 or any(ch not in "0123456789abcdef" for ch in args.source_sha):
        raise SystemExit("--source-sha must be an exact lowercase 40-character commit")

    attempts_dir = output / "attempts"
    attempts_dir.mkdir(parents=True)
    attempts = []
    receipts = []
    for scenario in SCENARIOS:
        for repeat, order in enumerate(MODE_ORDERS, start=1):
            for position, mode in enumerate(order, start=1):
                attempt_id = f"{scenario}-repeat-{repeat:02d}-{position}-{mode}"
                attempt_dir = attempts_dir / attempt_id
                attempt_dir.mkdir()
                receipt_path = attempt_dir / "receipt.json"
                command = [
                    str(binary),
                    "--profile-id",
                    PROFILE_ID,
                    "--mode",
                    mode,
                    "--scenario",
                    scenario,
                    "--allocation-only",
                    "true",
                    "--rate",
                    str(RATE),
                    "--operations",
                    str(OPERATIONS),
                    "--warmup-operations",
                    str(WARMUP_OPERATIONS),
                    "--output",
                    str(receipt_path),
                ]
                completed = subprocess.run(command, capture_output=True, text=True, check=False)
                (attempt_dir / "stdout.txt").write_text(completed.stdout, encoding="utf-8")
                (attempt_dir / "stderr.txt").write_text(completed.stderr, encoding="utf-8")
                attempt = {
                    "attempt_id": attempt_id,
                    "scenario": scenario,
                    "repeat": repeat,
                    "position": position,
                    "mode": mode,
                    "exit_code": completed.returncode,
                    "receipt_present": receipt_path.is_file(),
                }
                attempts.append(attempt)
                if receipt_path.is_file():
                    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
                    receipt["attempt_id"] = attempt_id
                    receipt["repeat"] = repeat
                    receipt["position"] = position
                    receipts.append(receipt)

    failed_attempts = [
        attempt
        for attempt in attempts
        if attempt["exit_code"] != 0 or not attempt["receipt_present"]
    ]
    result = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE_ID,
        "evidence_class": "local_diagnostic_only",
        "source_sha": args.source_sha,
        "binary_sha256": sha256(binary),
        "run_order_seed": SEED,
        "modes": MODES,
        "scenarios": SCENARIOS,
        "repeats_per_scenario_and_mode": REPEATS,
        "operations_per_attempt": OPERATIONS,
        "offered_rate_per_second": RATE,
        "warmup_operations": WARMUP_OPERATIONS,
        "attempts": attempts,
        "failed_attempts": failed_attempts,
        "scenario_results": summarize(receipts) if not failed_attempts else [],
        "allocation_measurement_only": True,
        "cpu_claims_allowed": False,
        "rss_claims_allowed": False,
        "promotable": False,
        "numerical_release_claims_allowed": False,
    }
    (output / "allocation-attribution.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    if failed_attempts:
        print(f"allocation-attribution: FAILED ({len(failed_attempts)} attempts)", file=sys.stderr)
        return 1
    print(f"allocation-attribution: OK ({len(attempts)} attempts, {output})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
