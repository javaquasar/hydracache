#!/usr/bin/env python3
"""Run the baseline-only 0.73 observer rate/window pilot."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import statistics
import subprocess
import sys


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def median(samples: list[float]) -> float:
    return float(statistics.median(samples))


def relative_spread(samples: list[float]) -> float:
    center = median(samples)
    return (max(samples) - min(samples)) / center if center else float("inf")


def relative_regression(control: float, treatment: float) -> float:
    return (treatment - control) / control if control else float("inf")


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
        "rate": rate,
        "operations": operations,
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
    if (
        receipt["schema_version"] != 1
        or receipt["release"] != "0.73"
        or receipt["profile_id"] != "observer-baseline-pilot-073-v1"
        or receipt["instrumentation_mode"] != mode
        or receipt["offered_rate_per_second"] != rate
        or receipt["operations"] != operations
        or receipt["promotable"]
        or observation["offered"] != operations
        or observation["started"] != operations
        or observation["completed"] != operations
        or observation["successes"] != operations
        or observation["errors"] != 0
        or observation["timeouts"] != 0
        or observation["rejections"] != 0
        or not observation["backlog_drained"]
        or not receipt["reconciliation_exact"]
    ):
        raise ValueError(f"invalid or incomplete {mode} receipt at rate {rate}")


def summarize(receipts: list[dict]) -> dict:
    goodput = [value["observation"]["achieved_rate_per_second"] for value in receipts]
    p99 = [float(value["observation"]["latency"]["p99_us"]) for value in receipts]
    cpu = [value["cpu_seconds_per_operation"] for value in receipts]
    allocation = [value["gross_allocated_bytes_per_operation"] for value in receipts]
    rss_delta = [
        float(value["rss_after_bytes"] - value["rss_before_bytes"]) for value in receipts
    ]
    peak_delta = [
        float(value["peak_rss_bytes"] - value["rss_before_bytes"]) for value in receipts
    ]
    return {
        "goodput_median": median(goodput),
        "goodput_relative_spread": relative_spread(goodput),
        "p99_us_median": median(p99),
        "cpu_seconds_per_operation_median": median(cpu),
        "allocated_bytes_per_operation_median": median(allocation),
        "rss_delta_bytes_median": median(rss_delta),
        "peak_rss_delta_bytes_median": median(peak_delta),
        "achieved_ratio_minimum": min(
            value["observation"]["achieved_rate_per_second"]
            / value["offered_rate_per_second"]
            for value in receipts
        ),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--rates", required=True)
    parser.add_argument("--repeats", type=int, required=True)
    parser.add_argument("--window-seconds", type=int, required=True)
    parser.add_argument("--warmup-operations", type=int, required=True)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--cpu-set", required=True)
    parser.add_argument("--source-sha", required=True)
    options = parser.parse_args()
    rates = [int(value) for value in options.rates.split(",")]
    if (
        not options.binary.is_file()
        or options.output.exists()
        or options.repeats < 3
        or options.window_seconds < 5
        or len(rates) < 4
        or rates != sorted(set(rates))
    ):
        raise SystemExit("invalid pilot binary, output, repeats, window, or rates")
    options.output.mkdir(parents=True)

    attempts = []
    failed = []
    receipts: dict[tuple[int, str], list[dict]] = {}
    for rate_index, rate in enumerate(rates):
        operations = rate * options.window_seconds
        for repeat in range(1, options.repeats + 1):
            off_first = (options.seed + rate_index + repeat) % 2 == 0
            order = ["off", "production"] if off_first else ["production", "off"]
            for position, mode in enumerate(order, 1):
                attempt_id = f"rate-{rate}-repeat-{repeat:02}-{position}-{mode}"
                attempt_dir = options.output / "attempts" / attempt_id
                attempt = run_attempt(
                    options.binary.resolve(),
                    attempt_dir,
                    options.cpu_set,
                    mode,
                    rate,
                    operations,
                    options.warmup_operations,
                )
                attempt.update(
                    {
                        "attempt_id": attempt_id,
                        "repeat": repeat,
                        "position": position,
                    }
                )
                attempts.append(attempt)
                if attempt["result"] != "success":
                    failed.append(attempt_id)
                    continue
                receipt = json.loads((attempt_dir / "receipt.json").read_text(encoding="utf-8"))
                validate_receipt(receipt, mode, rate, operations)
                receipts.setdefault((rate, mode), []).append(receipt)

    rate_results = []
    stable_rates = []
    for rate in rates:
        off_receipts = receipts.get((rate, "off"), [])
        production_receipts = receipts.get((rate, "production"), [])
        complete = len(off_receipts) == options.repeats and len(production_receipts) == options.repeats
        if complete:
            off = summarize(off_receipts)
            production = summarize(production_receipts)
            overhead = {
                "goodput_relative_regression": relative_regression(
                    off["goodput_median"], production["goodput_median"]
                )
                * -1.0,
                "cpu_per_operation_relative_regression": relative_regression(
                    off["cpu_seconds_per_operation_median"],
                    production["cpu_seconds_per_operation_median"],
                ),
                "p99_relative_regression": relative_regression(
                    off["p99_us_median"], production["p99_us_median"]
                ),
                "allocation_absolute_overhead": production[
                    "allocated_bytes_per_operation_median"
                ]
                - off["allocated_bytes_per_operation_median"],
                "rss_delta_absolute_overhead": production["rss_delta_bytes_median"]
                - off["rss_delta_bytes_median"],
                "peak_rss_delta_absolute_overhead": production[
                    "peak_rss_delta_bytes_median"
                ]
                - off["peak_rss_delta_bytes_median"],
            }
        else:
            off = None
            production = None
            overhead = None
        stable = (
            complete
            and off is not None
            and production is not None
            and overhead is not None
            and off["achieved_ratio_minimum"] >= 0.98
            and production["achieved_ratio_minimum"] >= 0.98
            and off["goodput_relative_spread"] <= 0.15
            and production["goodput_relative_spread"] <= 0.15
            and off["p99_us_median"] <= 10_000
            and production["p99_us_median"] <= 10_000
            and overhead["goodput_relative_regression"] <= 0.02
            and overhead["cpu_per_operation_relative_regression"] <= 0.03
            and overhead["p99_relative_regression"] <= 0.03
        )
        if stable:
            stable_rates.append(rate)
        rate_results.append(
            {
                "offered_rate_per_second": rate,
                "off": off,
                "production": production,
                "overhead": overhead,
                "stable": stable,
            }
        )

    selected_knee = max(stable_rates) if stable_rates else None
    frozen_rates = (
        [
            max(100, round(selected_knee * fraction / 100) * 100)
            for fraction in (0.25, 0.60, 0.85)
        ]
        if selected_knee
        else []
    )
    aggregate = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": "observer-baseline-pilot-073-v1",
        "evidence_class": "dedicated_host_baseline_only",
        "source_sha": options.source_sha,
        "binary_sha256": sha256(options.binary),
        "seed": options.seed,
        "rates": rates,
        "repeats": options.repeats,
        "window_seconds": options.window_seconds,
        "warmup_operations": options.warmup_operations,
        "attempts": attempts,
        "failed_attempts": failed,
        "rate_results": rate_results,
        "stable_rates": stable_rates,
        "selected_knee_rate_per_second": selected_knee,
        "frozen_d3_rates_per_second": frozen_rates,
        "promotable": False,
        "numerical_claim_eligible": False,
        "candidate_data_present": False,
        "i73_freeze_eligible": not failed and len(stable_rates) >= 3,
        "thresholds_changed": False,
    }
    (options.output / "baseline-pilot.json").write_text(
        json.dumps(aggregate, indent=2) + "\n", encoding="utf-8"
    )
    if failed or len(stable_rates) < 3:
        print(
            f"baseline pilot retained {len(failed)} failures and {len(stable_rates)} stable rates",
            file=sys.stderr,
        )
        return 1
    print(
        f"baseline pilot selected knee={selected_knee} rates={frozen_rates}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
