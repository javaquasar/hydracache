#!/usr/bin/env python3
"""Run or analyse non-promotable 0.74 A/A and A/B local pairings.

The runner applies process placement before the measured phase (the profilers have a mandatory
warm-up), samples machine-wide CPU immediately before each process, keeps every failed attempt,
and derives a minimum detectable effect from same-binary A/A deltas. It cannot produce release
evidence.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import pathlib
import statistics
import subprocess
import sys
import time
import tomllib
from typing import Any


PROFILE_ID = "local-pairing-074-v1"


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def abba_order(pairs: int, seed: int) -> list[tuple[str, str]]:
    if pairs <= 0:
        raise ValueError("pairs must be positive")
    first = ("baseline", "candidate")
    second = ("candidate", "baseline")
    if seed % 2:
        first, second = second, first
    return [first if index % 2 == 0 else second for index in range(pairs)]


def percentile(samples: list[float], quantile: float) -> float:
    if not samples or not 0.0 <= quantile <= 1.0:
        raise ValueError("invalid percentile input")
    ordered = sorted(samples)
    rank = max(0, math.ceil(quantile * len(ordered)) - 1)
    return ordered[rank]


def relative_delta(baseline: float, candidate: float) -> float:
    if baseline == 0.0:
        raise ValueError("zero baseline metric")
    return (candidate - baseline) / baseline


def nested_number(value: dict[str, Any], dotted: str) -> float:
    current: Any = value
    for segment in dotted.split("."):
        if not isinstance(current, dict) or segment not in current:
            raise ValueError(f"receipt is missing {dotted}")
        current = current[segment]
    if isinstance(current, bool) or not isinstance(current, (int, float)):
        raise ValueError(f"receipt metric {dotted} is not numeric")
    number = float(current)
    if not math.isfinite(number):
        raise ValueError(f"receipt metric {dotted} is not finite")
    return number


def workload_identity(receipt: dict[str, Any], contract: dict[str, Any]) -> dict[str, Any]:
    identity: dict[str, Any] = {}
    for field in contract["receipt"]["required_identity_fields"]:
        if field not in receipt:
            raise ValueError(f"receipt is missing workload identity field {field}")
        identity[field] = receipt[field]
    for field in contract["receipt"]["optional_identity_fields"]:
        if field in receipt:
            identity[field] = receipt[field]
    if receipt.get("warmup_operations", 0) <= 0:
        raise ValueError("measured receipt has no warm-up")
    if receipt.get("promotable") is not False:
        raise ValueError("local receipt must remain non-promotable")
    return identity


def analyse_attempts(
    attempts: list[dict[str, Any]], contract: dict[str, Any], mode: str, seed: int
) -> dict[str, Any]:
    pairs = int(contract["pairs"])
    if len(attempts) != pairs * 2:
        raise ValueError(f"expected {pairs * 2} attempts, got {len(attempts)}")
    expected = [role for pair in abba_order(pairs, seed) for role in pair]
    actual = [attempt.get("role") for attempt in attempts]
    if actual != expected:
        raise ValueError("attempt order is not the preregistered ABBA sequence")

    maximum_background = float(contract["maximum_background_cpu_percent"])
    maximum_failures = int(contract["maximum_failed_attempts"])
    failures = [attempt for attempt in attempts if attempt.get("result") != "success"]
    if len(failures) > maximum_failures:
        raise ValueError("one or more local attempts failed")
    for attempt in attempts:
        if attempt.get("affinity_applied") is not True:
            raise ValueError("attempt ran without the requested CPU affinity")
        if attempt.get("priority_applied") is not True:
            raise ValueError("attempt ran without the requested process priority")
        if float(attempt.get("background_cpu_percent", math.inf)) > maximum_background:
            raise ValueError("attempt started above the frozen background CPU ceiling")

    identities = [workload_identity(attempt["receipt"], contract) for attempt in attempts]
    if any(identity != identities[0] for identity in identities[1:]):
        raise ValueError("workload identity drifted between paired attempts")

    if mode == "aa":
        hashes = {attempt["binary_sha256"] for attempt in attempts}
        if len(hashes) != 1:
            raise ValueError("A/A calibration used more than one binary")
    elif mode != "ab":
        raise ValueError(f"unsupported mode {mode}")

    metrics = list(contract["receipt"]["required_metrics"])
    deltas: dict[str, list[float]] = {metric: [] for metric in metrics}
    by_pair: list[dict[str, Any]] = []
    for pair_index in range(pairs):
        left, right = attempts[pair_index * 2 : pair_index * 2 + 2]
        roles = {left["role"]: left, right["role"]: right}
        row: dict[str, Any] = {"pair_index": pair_index + 1}
        for metric in metrics:
            baseline = nested_number(roles["baseline"]["receipt"], metric)
            candidate = nested_number(roles["candidate"]["receipt"], metric)
            delta = relative_delta(baseline, candidate)
            deltas[metric].append(delta)
            row[metric] = delta
        by_pair.append(row)

    noise_contract = contract["noise"]
    floor_by_metric = {
        "goodput_operations_per_second": float(noise_contract["minimum_goodput_effect"]),
        "cpu_nanoseconds_per_operation": float(
            noise_contract["minimum_cpu_per_operation_effect"]
        ),
        "latency.p99_us": float(noise_contract["minimum_p99_effect"]),
    }
    quantile = float(noise_contract["percentile"])
    multiplier = float(noise_contract["noise_multiplier"])
    minimum_detectable_effect = {
        metric: max(
            floor_by_metric[metric],
            multiplier * percentile([abs(value) for value in values], quantile),
        )
        for metric, values in deltas.items()
    }
    return {
        "schema_version": 1,
        "release": "0.74",
        "profile_id": PROFILE_ID,
        "mode": mode,
        "promotable": False,
        "numerical_claim_eligible": False,
        "order": expected,
        "workload_identity": identities[0],
        "pairs": by_pair,
        "median_relative_delta": {
            metric: statistics.median(values) for metric, values in deltas.items()
        },
        "minimum_detectable_effect": minimum_detectable_effect,
        "classification": noise_contract["classification"],
    }


def cpu_times() -> tuple[int, int]:
    if sys.platform == "win32":
        import ctypes

        class FileTime(ctypes.Structure):
            _fields_ = [("low", ctypes.c_uint32), ("high", ctypes.c_uint32)]

            def integer(self) -> int:
                return (self.high << 32) | self.low

        idle, kernel, user = FileTime(), FileTime(), FileTime()
        if not ctypes.windll.kernel32.GetSystemTimes(
            ctypes.byref(idle), ctypes.byref(kernel), ctypes.byref(user)
        ):
            raise OSError("GetSystemTimes failed")
        return idle.integer(), kernel.integer() + user.integer()
    fields = pathlib.Path("/proc/stat").read_text(encoding="utf-8").splitlines()[0].split()
    values = [int(value) for value in fields[1:]]
    return values[3] + (values[4] if len(values) > 4 else 0), sum(values)


def background_cpu_percent(sample_milliseconds: int) -> float:
    idle_before, total_before = cpu_times()
    time.sleep(sample_milliseconds / 1000.0)
    idle_after, total_after = cpu_times()
    total = total_after - total_before
    idle = idle_after - idle_before
    return 100.0 * (1.0 - idle / total) if total > 0 else 100.0


def child_setup(cpu_ids: list[int], priority: str):
    if sys.platform == "win32":
        return None

    def setup() -> None:
        os.sched_setaffinity(0, set(cpu_ids))
        if priority == "normal":
            return
        if priority == "above-normal":
            os.nice(-5)
        else:
            raise ValueError(f"unsupported priority {priority}")

    return setup


def apply_windows_placement(process: subprocess.Popen[bytes], cpu_ids: list[int], priority: str) -> tuple[bool, bool]:
    if sys.platform != "win32":
        return True, True
    import ctypes

    mask = sum(1 << cpu for cpu in cpu_ids)
    affinity = bool(ctypes.windll.kernel32.SetProcessAffinityMask(int(process._handle), mask))
    priority_values = {"normal": 0x20, "above-normal": 0x8000, "high": 0x80}
    if priority not in priority_values:
        raise ValueError(f"unsupported priority {priority}")
    priority_applied = bool(
        ctypes.windll.kernel32.SetPriorityClass(int(process._handle), priority_values[priority])
    )
    return affinity, priority_applied


def command_for(role: dict[str, Any], output: pathlib.Path) -> list[str]:
    args = [str(value).replace("{output}", str(output)) for value in role["args"]]
    if not any(str(output) in arg for arg in args):
        raise ValueError("role args must contain the {output} placeholder")
    return [str(role["binary"]), *args]


def run_attempt(
    role_name: str,
    role: dict[str, Any],
    attempt_dir: pathlib.Path,
    cpu_ids: list[int],
    priority: str,
    quiet_sample_ms: int,
) -> dict[str, Any]:
    attempt_dir.mkdir(parents=True)
    receipt_path = attempt_dir / "receipt.json"
    stdout_path = attempt_dir / "stdout.txt"
    stderr_path = attempt_dir / "stderr.txt"
    background = background_cpu_percent(quiet_sample_ms)
    command = command_for(role, receipt_path)
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            command,
            stdout=stdout,
            stderr=stderr,
            preexec_fn=child_setup(cpu_ids, priority),
        )
        affinity, priority_applied = apply_windows_placement(process, cpu_ids, priority)
        exit_code = process.wait()
    attempt: dict[str, Any] = {
        "role": role_name,
        "binary_sha256": sha256(pathlib.Path(role["binary"])),
        "background_cpu_percent": background,
        "affinity_applied": affinity,
        "priority_applied": priority_applied,
        "exit_code": exit_code,
        "stdout_sha256": sha256(stdout_path),
        "stderr_sha256": sha256(stderr_path),
        "result": "failed",
    }
    if exit_code == 0 and receipt_path.is_file():
        attempt["receipt"] = json.loads(receipt_path.read_text(encoding="utf-8"))
        attempt["receipt_sha256"] = sha256(receipt_path)
        attempt["result"] = "success"
    (attempt_dir / "attempt.json").write_text(
        json.dumps(attempt, indent=2) + "\n", encoding="utf-8"
    )
    return attempt


def load_contract(path: pathlib.Path) -> dict[str, Any]:
    contract = tomllib.loads(path.read_text(encoding="utf-8"))
    if (
        contract.get("release") != "0.74"
        or contract.get("contract_id") != "local-pairing-074-v1"
        or contract.get("promotable") is not False
    ):
        raise ValueError("invalid 0.74 local harness contract")
    return contract


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--plan", type=pathlib.Path, required=True)
    parser.add_argument("--contract", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--analyse-only", type=pathlib.Path)
    options = parser.parse_args()
    contract = load_contract(options.contract)
    plan = json.loads(options.plan.read_text(encoding="utf-8"))
    if options.output.exists():
        raise SystemExit("output directory already exists")
    options.output.mkdir(parents=True)
    if plan.get("release") != "0.74" or plan.get("pairs") != contract["pairs"]:
        raise SystemExit("plan does not match the frozen 0.74 local contract")
    mode = plan.get("mode")
    seed = int(plan["seed"])
    if options.analyse_only:
        attempts = json.loads(options.analyse_only.read_text(encoding="utf-8"))
    else:
        attempts = []
        for pair_index, order in enumerate(abba_order(int(contract["pairs"]), seed), 1):
            for position, role_name in enumerate(order, 1):
                attempt = run_attempt(
                    role_name,
                    plan[role_name],
                    options.output / "attempts" / f"pair-{pair_index:02}-{position}-{role_name}",
                    [int(value) for value in plan["cpu_ids"]],
                    str(plan["priority"]),
                    int(contract["quiet_sample_milliseconds"]),
                )
                attempts.append(attempt)
        (options.output / "attempts.json").write_text(
            json.dumps(attempts, indent=2) + "\n", encoding="utf-8"
        )
    summary = analyse_attempts(attempts, contract, str(mode), seed)
    (options.output / "summary.json").write_text(
        json.dumps(summary, indent=2) + "\n", encoding="utf-8"
    )
    print(f"local pairing 0.74: OK ({mode}, non-promotable)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
