#!/usr/bin/env python3
"""Run the frozen Release 0.73 integrated I73/C73 host comparison."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone

PROFILE_ID = "integrated-focused-host-073-v1"
I73_SHA = "e757556d3a31d565f52a9561d6d4e555bb1cc373"
C73_SHA = "7e3070894aa51af96cdcb3e350eff923a309e1fa"
RATES = [5_000, 12_000, 17_000]
PAIRS_PER_RATE = 5
WINDOW_SECONDS = 10
WARMUP_OPERATIONS = 5_000
ORDER_SEED = 731_073
WEIGHTS = [35, 30, 15, 10, 5, 5]
SURFACES = ["hc2", "resp", "hc1", "direct", "tag_invalidation", "ttl_expire_refill"]
CANARY_MARKER = "HC-CANARY-RED:W10-HOST"
REGRESSION_BUDGETS = {
    "goodput_relative_regression": 0.02,
    "cpu_per_operation_relative_regression": 0.03,
    "p99_relative_regression": 0.03,
}


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_file(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def sha256_tree(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    files = sorted(
        candidate
        for candidate in path.rglob("*")
        if candidate.is_file()
        and "target" not in candidate.relative_to(path).parts
        and "__pycache__" not in candidate.relative_to(path).parts
    )
    if not files:
        raise ValueError(f"overlay tree is empty: {path}")
    for candidate in files:
        relative = candidate.relative_to(path).as_posix().encode("utf-8")
        content = candidate.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def hodges_lehmann(samples: list[float]) -> float:
    if not samples:
        return float("inf")
    walsh_averages = [
        (left + right) / 2.0
        for index, left in enumerate(samples)
        for right in samples[index:]
    ]
    return float(statistics.median(walsh_averages))


def relative_change(baseline: float, candidate: float) -> float:
    if baseline == 0:
        return 0.0 if candidate == 0 else float("inf")
    return (candidate - baseline) / baseline


def pair_order(rate_index: int, repeat: int) -> list[str]:
    i73_first = (ORDER_SEED + rate_index + repeat) % 2 == 0
    return ["I73", "C73"] if i73_first else ["C73", "I73"]


def expected_surface_counts(operations: int) -> dict[str, int]:
    full, remainder = divmod(operations, 100)
    starts = [0, 35, 65, 80, 90, 95]
    return {
        name: full * width + min(max(remainder - start, 0), width)
        for name, start, width in zip(SURFACES, starts, WEIGHTS, strict=True)
    }


def require_number(value: object, label: str, *, positive: bool = False) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{label} is not numeric")
    number = float(value)
    if not number >= 0 or (positive and not number > 0):
        raise ValueError(f"{label} is outside its valid range")
    return number


def validate_receipt(
    receipt: dict,
    role: str,
    rate: int,
    operations: int,
    daemon_cpu_set: str,
    loadgen_cpu_set: str,
) -> None:
    source_sha = I73_SHA if role == "I73" else C73_SHA
    observation = receipt.get("observation", {})
    if (
        receipt.get("schema_version") != 1
        or receipt.get("release") != "0.73"
        or receipt.get("profile_id") != PROFILE_ID
        or receipt.get("role") != role
        or receipt.get("source_sha") != source_sha
        or receipt.get("offered_rate_per_second") != rate
        or receipt.get("operations") != operations
        or receipt.get("warmup_operations") != WARMUP_OPERATIONS
        or receipt.get("weights_percent") != WEIGHTS
        or receipt.get("daemon_cpu_set") != daemon_cpu_set
        or receipt.get("loadgen_cpu_set") != loadgen_cpu_set
        or receipt.get("promotable") is not False
        or receipt.get("reconciliation_exact") is not True
        or receipt.get("management_truth_zero") is not True
        or observation.get("offered") != operations
        or observation.get("started") != operations
        or observation.get("completed") != operations
        or observation.get("successes") != operations
        or observation.get("errors") != 0
        or observation.get("timeouts") != 0
        or observation.get("rejections") != 0
        or observation.get("backlog_drained") is not True
    ):
        raise ValueError(f"{role} receipt has incomplete identity, outcomes, or owners")

    expected = expected_surface_counts(operations)
    surfaces = receipt.get("surfaces", {})
    if set(surfaces) != set(SURFACES):
        raise ValueError(f"{role} receipt has an unexpected surface set")
    for name, count in expected.items():
        surface = surfaces[name]
        if (
            surface.get("attempted") != count
            or surface.get("success") != count
            or surface.get("rejected") != 0
            or surface.get("timeout") != 0
            or surface.get("late") != 0
            or surface.get("incomplete") != 0
        ):
            raise ValueError(f"{role} receipt has incomplete {name} accounting")
    if receipt.get("events_received") != expected["hc2"]:
        raise ValueError(f"{role} receipt has incomplete HC2 event accounting")

    durable = receipt.get("durable", {})
    if (
        durable.get("attempted") != 1_000
        or durable.get("success") != 1_000
        or durable.get("budget_rejections") != 0
        or durable.get("reopen_verified") is not True
        or durable.get("corruption_rejected") is not True
    ):
        raise ValueError(f"{role} durable companion did not reconcile")
    require_number(durable.get("reclaimed_bytes"), "reclaimed_bytes", positive=True)

    resources = receipt.get("resources", {})
    if resources.get("available") is not True:
        raise ValueError(f"{role} combined process resources are unavailable")
    require_number(resources.get("cpu_seconds"), "cpu_seconds", positive=True)
    require_number(
        resources.get("cpu_seconds_per_completed_operation"),
        "cpu_seconds_per_completed_operation",
        positive=True,
    )
    require_number(resources.get("rss_before_bytes"), "rss_before_bytes", positive=True)
    require_number(resources.get("rss_after_bytes"), "rss_after_bytes", positive=True)
    require_number(resources.get("peak_rss_after_bytes"), "peak_rss_after_bytes", positive=True)
    require_number(
        observation.get("achieved_rate_per_second"),
        "achieved_rate_per_second",
        positive=True,
    )
    require_number(observation.get("latency", {}).get("p99_us"), "p99_us", positive=True)


def paired_regressions(i73: dict, c73: dict) -> dict[str, float]:
    i_observation = i73["observation"]
    c_observation = c73["observation"]
    i_resources = i73["resources"]
    c_resources = c73["resources"]
    goodput_change = relative_change(
        float(i_observation["achieved_rate_per_second"]),
        float(c_observation["achieved_rate_per_second"]),
    )
    return {
        "goodput_relative_regression": -goodput_change,
        "cpu_per_operation_relative_regression": relative_change(
            float(i_resources["cpu_seconds_per_completed_operation"]),
            float(c_resources["cpu_seconds_per_completed_operation"]),
        ),
        "p99_relative_regression": relative_change(
            float(i_observation["latency"]["p99_us"]),
            float(c_observation["latency"]["p99_us"]),
        ),
        "rss_after_relative_change": relative_change(
            float(i_resources["rss_after_bytes"]), float(c_resources["rss_after_bytes"])
        ),
        "peak_rss_relative_change": relative_change(
            float(i_resources["peak_rss_after_bytes"]),
            float(c_resources["peak_rss_after_bytes"]),
        ),
    }


def command_for_attempt(
    harness: pathlib.Path,
    server: pathlib.Path,
    role: str,
    rate: int,
    operations: int,
    output: pathlib.Path,
    daemon_cpu_set: str,
    loadgen_cpu_set: str,
    *,
    host_mode: bool,
) -> list[str]:
    command = []
    if host_mode:
        command.extend(["taskset", "--cpu-list", loadgen_cpu_set])
    command.extend(
        [
            str(harness),
            "--profile-id",
            PROFILE_ID,
            "--role",
            role,
            "--source-sha",
            I73_SHA if role == "I73" else C73_SHA,
            "--rate",
            str(rate),
            "--operations",
            str(operations),
            "--warmup-operations",
            str(WARMUP_OPERATIONS if host_mode else 100),
            "--daemon-cpu-set",
            daemon_cpu_set,
            "--loadgen-cpu-set",
            loadgen_cpu_set,
            "--server-binary",
            str(server),
            "--output",
            str(output),
            "--allow-unavailable-resources",
            "false" if host_mode else "true",
        ]
    )
    return command


def run_process(
    command: list[str], output: pathlib.Path, *, env: dict[str, str] | None = None
) -> dict:
    output.mkdir(parents=True, exist_ok=False)
    started_at = utc_now()
    start = time.monotonic()
    process = subprocess.run(command, capture_output=True, check=False, env=env)
    duration = time.monotonic() - start
    stdout_path = output / "stdout.txt"
    stderr_path = output / "stderr.txt"
    stdout_path.write_bytes(process.stdout)
    stderr_path.write_bytes(process.stderr)
    return {
        "started_at": started_at,
        "completed_at": utc_now(),
        "duration_seconds": duration,
        "exit_code": process.returncode,
        "command": command,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }


def run_attempt(
    harness: pathlib.Path,
    server: pathlib.Path,
    role: str,
    rate: int,
    operations: int,
    attempt_dir: pathlib.Path,
    daemon_cpu_set: str,
    loadgen_cpu_set: str,
) -> tuple[dict, dict | None]:
    receipt_path = attempt_dir / "receipt.json"
    attempt = run_process(
        command_for_attempt(
            harness,
            server,
            role,
            rate,
            operations,
            receipt_path,
            daemon_cpu_set,
            loadgen_cpu_set,
            host_mode=True,
        ),
        attempt_dir,
    )
    receipt = None
    attempt["result"] = "process-failed"
    attempt["receipt_sha256"] = None
    if receipt_path.is_file():
        attempt["receipt_sha256"] = sha256_file(receipt_path)
    if attempt["exit_code"] == 0 and receipt_path.is_file():
        try:
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            validate_receipt(
                receipt,
                role,
                rate,
                operations,
                daemon_cpu_set,
                loadgen_cpu_set,
            )
            attempt["result"] = "valid"
        except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            attempt["result"] = "invalid-receipt"
            attempt["validation_error"] = str(error)
            receipt = None
    elif attempt["exit_code"] == 0:
        attempt["result"] = "missing-receipt"
    (attempt_dir / "attempt.json").write_text(
        json.dumps(attempt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return attempt, receipt


def verify_inputs(options: argparse.Namespace) -> dict:
    paths = {
        "i73_harness": options.i73_harness.resolve(),
        "c73_harness": options.c73_harness.resolve(),
        "i73_server": options.i73_server.resolve(),
        "c73_server": options.c73_server.resolve(),
        "i73_overlay": options.i73_overlay.resolve(),
        "c73_overlay": options.c73_overlay.resolve(),
        "scenario": options.scenario.resolve(),
    }
    for name in ["i73_harness", "c73_harness", "i73_server", "c73_server", "scenario"]:
        if not paths[name].is_file():
            raise ValueError(f"missing {name}: {paths[name]}")
    for name in ["i73_overlay", "c73_overlay"]:
        if not paths[name].is_dir():
            raise ValueError(f"missing {name}: {paths[name]}")
    i_overlay_sha = sha256_tree(paths["i73_overlay"])
    c_overlay_sha = sha256_tree(paths["c73_overlay"])
    if i_overlay_sha != c_overlay_sha:
        raise ValueError("I73 and C73 harness overlays are not byte-identical")
    return {
        "paths": paths,
        "overlay_sha256": i_overlay_sha,
        "i73_harness_sha256": sha256_file(paths["i73_harness"]),
        "c73_harness_sha256": sha256_file(paths["c73_harness"]),
        "i73_server_sha256": sha256_file(paths["i73_server"]),
        "c73_server_sha256": sha256_file(paths["c73_server"]),
        "scenario_sha256": sha256_file(paths["scenario"]),
        "runner_sha256": sha256_file(pathlib.Path(__file__).resolve()),
    }


def run_canary(options: argparse.Namespace, inputs: dict) -> int:
    output = options.output.resolve()
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    output.mkdir(parents=True)
    receipt_path = output / "receipt.json"
    env = os.environ.copy()
    env["HYDRACACHE_CANARY_DEFECT"] = "HOST073"
    attempt = run_process(
        command_for_attempt(
            inputs["paths"]["c73_harness"],
            inputs["paths"]["c73_server"],
            "C73",
            1_000,
            1_000,
            receipt_path,
            options.daemon_cpu_set,
            options.loadgen_cpu_set,
            host_mode=False,
        ),
        output / "attempt",
        env=env,
    )
    stderr = (output / "attempt" / "stderr.txt").read_text(
        encoding="utf-8", errors="replace"
    )
    passed = attempt["exit_code"] != 0 and CANARY_MARKER in stderr and not receipt_path.exists()
    result = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE_ID,
        "mode": "canary",
        "result": "passed" if passed else "failed",
        "marker": CANARY_MARKER,
        "marker_observed": CANARY_MARKER in stderr,
        "receipt_absent": not receipt_path.exists(),
        "attempt": attempt,
        "identity": {key: value for key, value in inputs.items() if key != "paths"},
    }
    (output / "canary.json").write_text(
        json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return 0 if passed else 1


def run_campaign(options: argparse.Namespace, inputs: dict) -> int:
    output = options.output.resolve()
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    output.mkdir(parents=True)
    attempts = []
    receipts: dict[tuple[int, int, str], dict] = {}
    roles = {
        "I73": (inputs["paths"]["i73_harness"], inputs["paths"]["i73_server"]),
        "C73": (inputs["paths"]["c73_harness"], inputs["paths"]["c73_server"]),
    }
    for rate_index, rate in enumerate(RATES):
        operations = rate * WINDOW_SECONDS
        for repeat in range(1, PAIRS_PER_RATE + 1):
            for position, role in enumerate(pair_order(rate_index, repeat), 1):
                attempt_id = f"rate-{rate}-pair-{repeat:02}-{position}-{role.lower()}"
                harness, server = roles[role]
                attempt, receipt = run_attempt(
                    harness,
                    server,
                    role,
                    rate,
                    operations,
                    output / "attempts" / attempt_id,
                    options.daemon_cpu_set,
                    options.loadgen_cpu_set,
                )
                attempt.update(
                    {
                        "attempt_id": attempt_id,
                        "rate": rate,
                        "operations": operations,
                        "pair": repeat,
                        "position": position,
                        "role": role,
                    }
                )
                attempts.append(attempt)
                if receipt is not None:
                    receipts[(rate, repeat, role)] = receipt

    rate_results = []
    all_rates_passed = True
    for rate in RATES:
        pair_results = []
        for repeat in range(1, PAIRS_PER_RATE + 1):
            i73 = receipts.get((rate, repeat, "I73"))
            c73 = receipts.get((rate, repeat, "C73"))
            if i73 is not None and c73 is not None:
                pair_results.append(
                    {"pair": repeat, **paired_regressions(i73, c73)}
                )
        complete = len(pair_results) == PAIRS_PER_RATE
        estimates = {
            metric: round(hodges_lehmann([pair[metric] for pair in pair_results]), 6)
            for metric in [*REGRESSION_BUDGETS, "rss_after_relative_change", "peak_rss_relative_change"]
        }
        guards = {
            metric: complete and estimates[metric] <= limit
            for metric, limit in REGRESSION_BUDGETS.items()
        }
        passed = complete and all(guards.values())
        all_rates_passed = all_rates_passed and passed
        rate_results.append(
            {
                "rate": rate,
                "operations_per_role": rate * WINDOW_SECONDS,
                "complete_pairs": len(pair_results),
                "pair_results": pair_results,
                "hodges_lehmann": estimates,
                "primary_guards": guards,
                "allocation_diagnostic": "unavailable-combined-process-counter-not-instrumented",
                "rss_diagnostics_promotable": False,
                "passed": passed,
            }
        )

    failed_attempts = [attempt["attempt_id"] for attempt in attempts if attempt["result"] != "valid"]
    passed = all_rates_passed and not failed_attempts and len(attempts) == 30
    campaign = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE_ID,
        "result": "passed" if passed else "failed",
        "baseline_source_sha": I73_SHA,
        "candidate_source_sha": C73_SHA,
        "rates_per_second": RATES,
        "pairs_per_rate": PAIRS_PER_RATE,
        "window_seconds": WINDOW_SECONDS,
        "warmup_operations": WARMUP_OPERATIONS,
        "run_order_seed": ORDER_SEED,
        "weights_percent": WEIGHTS,
        "daemon_cpu_set": options.daemon_cpu_set,
        "loadgen_cpu_set": options.loadgen_cpu_set,
        "paired_estimator": "hodges-lehmann-v1",
        "numeric_precision_decimals": 6,
        "regression_budgets": REGRESSION_BUDGETS,
        "allocation_and_rss_promotable": False,
        "silent_retry_allowed": False,
        "attempt_count": len(attempts),
        "failed_attempts": failed_attempts,
        "attempts": attempts,
        "rate_results": rate_results,
        "identity": {key: value for key, value in inputs.items() if key != "paths"},
    }
    (output / "integrated-host-campaign.json").write_text(
        json.dumps(campaign, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    return 0 if passed else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=["campaign", "canary"], required=True)
    parser.add_argument("--i73-harness", type=pathlib.Path, required=True)
    parser.add_argument("--c73-harness", type=pathlib.Path, required=True)
    parser.add_argument("--i73-server", type=pathlib.Path, required=True)
    parser.add_argument("--c73-server", type=pathlib.Path, required=True)
    parser.add_argument("--i73-overlay", type=pathlib.Path, required=True)
    parser.add_argument("--c73-overlay", type=pathlib.Path, required=True)
    parser.add_argument("--scenario", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--daemon-cpu-set", required=True)
    parser.add_argument("--loadgen-cpu-set", required=True)
    options = parser.parse_args()
    if (
        not options.daemon_cpu_set
        or not options.loadgen_cpu_set
        or options.daemon_cpu_set == options.loadgen_cpu_set
    ):
        raise SystemExit("daemon and loadgen CPU sets must be non-empty and distinct")
    try:
        inputs = verify_inputs(options)
        return run_canary(options, inputs) if options.mode == "canary" else run_campaign(options, inputs)
    except (OSError, ValueError) as error:
        print(f"integrated host runner rejected input: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
