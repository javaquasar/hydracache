#!/usr/bin/env python3
"""Run the preregistered Release 0.73 integrated long-duration pair."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import pathlib
import random
import signal
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone

PROFILE_ID = "integrated-long-run-073-v1"
I73_SHA = "e757556d3a31d565f52a9561d6d4e555bb1cc373"
C73_SHA = "7e3070894aa51af96cdcb3e350eff923a309e1fa"
RATE = 12_000
WARMUP_OPERATIONS = 5_000
WEIGHTS = [35, 30, 15, 10, 5, 5]
SURFACES = ["hc2", "resp", "hc1", "direct", "tag_invalidation", "ttl_expire_refill"]
CHECKPOINT_INTERVAL_SECONDS = 60
POST_WORK_IDLE_SECONDS = 300
BLOCK_SAMPLES = 12
BOOTSTRAP_ITERATIONS = 10_000
BOOTSTRAP_SEED = 730_073
CANARY_MARKER = "HC-CANARY-RED:W10-LONG"
ROLE_ARTIFACT_LIMIT = 64 * 1024 * 1024
PACKET_ARTIFACT_LIMIT = 256 * 1024 * 1024
REGRESSION_BUDGETS = {"goodput": 0.02, "cpu_per_operation": 0.03, "p99": 0.03}
PHASES = {
    "qualification": {"seconds": 21_600, "operations": 259_200_000, "checkpoints": 360},
    "confirmation": {"seconds": 86_400, "operations": 1_036_800_000, "checkpoints": 1_440},
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
        relative = candidate.relative_to(path).as_posix().encode()
        content = candidate.read_bytes()
        digest.update(len(relative).to_bytes(8, "big"))
        digest.update(relative)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.hexdigest()


def directory_size(path: pathlib.Path) -> int:
    return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())


def relative_change(baseline: float, candidate: float) -> float:
    if baseline == 0:
        return 0.0 if candidate == 0 else math.inf
    return (candidate - baseline) / baseline


def percentile(values: list[float], probability: float) -> float:
    if not values:
        return math.inf
    ordered = sorted(values)
    index = max(0, min(len(ordered) - 1, math.ceil(probability * len(ordered)) - 1))
    return float(ordered[index])


def theil_sen(points: list[tuple[float, float]]) -> float:
    slopes = [
        (right[1] - left[1]) / (right[0] - left[0])
        for index, left in enumerate(points)
        for right in points[index + 1 :]
        if right[0] > left[0]
    ]
    if not slopes:
        raise ValueError("slope requires at least two distinct checkpoints")
    return float(statistics.median(slopes))


def moving_block_upper_bound(
    points: list[tuple[float, float]],
    *,
    block_samples: int = BLOCK_SAMPLES,
    iterations: int = BOOTSTRAP_ITERATIONS,
    seed: int = BOOTSTRAP_SEED,
) -> dict[str, float | int]:
    if len(points) < block_samples * 2:
        raise ValueError("moving-block slope requires at least two complete blocks")
    point_slope = theil_sen(points)
    # Resample contiguous adjacent-rate blocks, preserving the slope statistic's
    # time direction. Resampling absolute levels and assigning each selected
    # block a new time coordinate destroys the trend and is not a slope bound.
    adjacent_slopes = [
        (right[1] - left[1]) / (right[0] - left[0])
        for left, right in zip(points, points[1:])
        if right[0] > left[0]
    ]
    effective_block = min(block_samples, len(adjacent_slopes))
    starts = len(adjacent_slopes) - effective_block + 1
    random_source = random.Random(seed)
    bootstrap_slopes = []
    for _ in range(iterations):
        resampled = []
        while len(resampled) < len(adjacent_slopes):
            start = random_source.randrange(0, starts)
            resampled.extend(adjacent_slopes[start : start + effective_block])
        bootstrap_slopes.append(float(statistics.mean(resampled[: len(adjacent_slopes)])))
    return {
        "samples": len(points),
        "block_samples": block_samples,
        "bootstrap_iterations": iterations,
        "bootstrap_seed": seed,
        "bootstrap_method": "moving-block-adjacent-slope-v1",
        "theil_sen_bytes_per_second": point_slope,
        "upper_95_bytes_per_second": percentile(bootstrap_slopes, 0.95),
    }


def read_checkpoints(path: pathlib.Path) -> list[dict]:
    checkpoints = []
    for line_number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        try:
            checkpoints.append(json.loads(line))
        except json.JSONDecodeError as error:
            raise ValueError(f"checkpoint line {line_number} is invalid JSON: {error}") from error
    return checkpoints


def validate_checkpoints(
    path: pathlib.Path,
    *,
    minimum_periodic: int,
    resources_required: bool,
) -> tuple[list[dict], dict[str, dict[str, float | int]]]:
    checkpoints = read_checkpoints(path)
    if len(checkpoints) < minimum_periodic + 3:
        raise ValueError("checkpoint series is incomplete")
    if [item.get("sequence") for item in checkpoints] != list(range(len(checkpoints))):
        raise ValueError("checkpoint sequence is missing, duplicated, or reordered")
    if checkpoints[0].get("kind") != "pre-work":
        raise ValueError("checkpoint series does not start before work")
    if checkpoints[-2].get("kind") != "final-work":
        raise ValueError("checkpoint series has no final-work checkpoint")
    if (
        checkpoints[-1].get("kind") != "post-idle-reconciled"
        or checkpoints[-1].get("owner_reconciled") is not True
    ):
        raise ValueError("checkpoint series has no reconciled final checkpoint")
    periodic = [item for item in checkpoints if item.get("kind") == "periodic-work"]
    if len(periodic) < minimum_periodic:
        raise ValueError("checkpoint series has too few periodic samples")
    work = [checkpoints[0], *periodic, checkpoints[-2]]
    elapsed = [float(item["elapsed_seconds"]) for item in work]
    if any(right <= left for left, right in zip(elapsed, elapsed[1:])):
        raise ValueError("checkpoint elapsed time is not strictly increasing")
    gaps = [right - left for left, right in zip(elapsed, elapsed[1:])]
    if periodic and max(gaps) > 90.0:
        raise ValueError("checkpoint series contains a gap above 90 seconds")
    for item in checkpoints:
        resources = item.get("resources", {})
        if resources_required and resources.get("available") is not True:
            raise ValueError("checkpoint resources are unavailable")
        for field in [
            "cpu_seconds",
            "rss_bytes",
            "peak_rss_bytes",
            "anonymous_pss_bytes",
            "file_pss_bytes",
            "minor_faults",
            "major_faults",
            "threads",
            "file_descriptors",
        ]:
            value = resources.get(field)
            if isinstance(value, bool) or not isinstance(value, (int, float)) or value < 0:
                raise ValueError(f"checkpoint {field} is unavailable")
    signal_points = {
        "rss": [(float(item["elapsed_seconds"]), float(item["resources"]["rss_bytes"])) for item in periodic],
        "anonymous_pss": [
            (float(item["elapsed_seconds"]), float(item["resources"]["anonymous_pss_bytes"]))
            for item in periodic
        ],
    }
    bounds = {
        name: moving_block_upper_bound(points)
        for name, points in signal_points.items()
        if len(points) >= BLOCK_SAMPLES * 2
    }
    return checkpoints, bounds


def expected_surface_counts(operations: int) -> dict[str, int]:
    full, remainder = divmod(operations, 100)
    starts = [0, 35, 65, 80, 90, 95]
    return {
        name: full * width + min(max(remainder - start, 0), width)
        for name, start, width in zip(SURFACES, starts, WEIGHTS, strict=True)
    }


def validate_receipt(receipt: dict, role: str, operations: int, *, host_mode: bool) -> None:
    expected_sha = I73_SHA if role == "I73" else C73_SHA
    observation = receipt.get("observation", {})
    expected = expected_surface_counts(operations)
    surfaces = receipt.get("surfaces", {})
    if (
        receipt.get("schema_version") != 1
        or receipt.get("release") != "0.73"
        or receipt.get("profile_id") != PROFILE_ID
        or receipt.get("role") != role
        or receipt.get("source_sha") != expected_sha
        or receipt.get("offered_rate_per_second") != (RATE if host_mode else 1_000)
        or receipt.get("operations") != operations
        or receipt.get("warmup_operations") != (WARMUP_OPERATIONS if host_mode else 100)
        or receipt.get("weights_percent") != WEIGHTS
        or receipt.get("promotable") is not False
        or receipt.get("reconciliation_exact") is not True
        or receipt.get("management_truth_zero") is not True
        or receipt.get("final_checkpoint_present") is not True
        or observation.get("offered") != operations
        or observation.get("started") != operations
        or observation.get("completed") != operations
        or observation.get("successes") != operations
        or any(observation.get(field) != 0 for field in ["errors", "timeouts", "rejections"])
        or observation.get("backlog_drained") is not True
        or set(surfaces) != set(SURFACES)
    ):
        raise ValueError(f"{role} receipt identity or outcomes are incomplete")
    for name, count in expected.items():
        surface = surfaces[name]
        if surface.get("attempted") != count or surface.get("success") != count or any(
            surface.get(field) != 0 for field in ["rejected", "timeout", "late", "incomplete"]
        ):
            raise ValueError(f"{role} {name} accounting is incomplete")
    if receipt.get("events_received") != expected["hc2"]:
        raise ValueError(f"{role} HC2 event accounting is incomplete")
    durable = receipt.get("durable", {})
    if (
        durable.get("attempted") != 1_000
        or durable.get("success") != 1_000
        or durable.get("budget_rejections") != 0
        or durable.get("reopen_verified") is not True
        or durable.get("corruption_rejected") is not True
    ):
        raise ValueError(f"{role} durable companion is incomplete")
    if host_mode and receipt.get("resources", {}).get("available") is not True:
        raise ValueError(f"{role} aggregate resources are unavailable")


def command_for_role(
    harness: pathlib.Path,
    server: pathlib.Path,
    role: str,
    receipt: pathlib.Path,
    checkpoints: pathlib.Path,
    daemon_cpu_set: str,
    loadgen_cpu_set: str,
    *,
    phase: str,
    host_mode: bool,
) -> tuple[list[str], int, int]:
    spec = PHASES[phase]
    operations = int(spec["operations"]) if host_mode else 2_000
    rate = RATE if host_mode else 1_000
    checkpoint_interval = CHECKPOINT_INTERVAL_SECONDS if host_mode else 1
    post_idle = POST_WORK_IDLE_SECONDS if host_mode else 1
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
            str(receipt),
            "--checkpoint-output",
            str(checkpoints),
            "--checkpoint-interval-seconds",
            str(checkpoint_interval),
            "--post-work-idle-seconds",
            str(post_idle),
            "--allow-unavailable-resources",
            "false" if host_mode else "true",
        ]
    )
    return command, operations, checkpoint_interval


def stop_process(process: subprocess.Popen) -> None:
    if process.poll() is not None:
        return
    if os.name == "nt":
        process.terminate()
    else:
        os.killpg(process.pid, signal.SIGTERM)
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        if os.name == "nt":
            process.kill()
        else:
            os.killpg(process.pid, signal.SIGKILL)
        process.wait()


def run_role(
    command: list[str],
    output: pathlib.Path,
    *,
    timeout_seconds: int,
    env: dict[str, str] | None = None,
) -> dict:
    output.mkdir(parents=True, exist_ok=False)
    stdout_path = output / "stdout.txt"
    stderr_path = output / "stderr.txt"
    started_at = utc_now()
    started = time.monotonic()
    with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
        process = subprocess.Popen(
            command,
            stdout=stdout,
            stderr=stderr,
            env=env,
            start_new_session=os.name != "nt",
        )
        timed_out = False
        last_heartbeat = -30.0
        while process.poll() is None:
            elapsed = time.monotonic() - started
            if elapsed > timeout_seconds:
                timed_out = True
                stop_process(process)
                break
            if elapsed - last_heartbeat >= 30.0:
                print(
                    f"LONG073_RUNNER_HEARTBEAT pid={process.pid} elapsed_seconds={elapsed:.1f}",
                    flush=True,
                )
                last_heartbeat = elapsed
            time.sleep(min(1.0, max(0.1, timeout_seconds - elapsed)))
    return {
        "started_at": started_at,
        "completed_at": utc_now(),
        "duration_seconds": time.monotonic() - started,
        "exit_code": process.returncode,
        "timed_out": timed_out,
        "command": command,
        "stdout_sha256": sha256_file(stdout_path),
        "stderr_sha256": sha256_file(stderr_path),
    }


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
    i_overlay = sha256_tree(paths["i73_overlay"])
    c_overlay = sha256_tree(paths["c73_overlay"])
    if i_overlay != c_overlay:
        raise ValueError("I73 and C73 overlays are not byte-identical")
    return {
        "paths": paths,
        "overlay_sha256": i_overlay,
        "i73_harness_sha256": sha256_file(paths["i73_harness"]),
        "c73_harness_sha256": sha256_file(paths["c73_harness"]),
        "i73_server_sha256": sha256_file(paths["i73_server"]),
        "c73_server_sha256": sha256_file(paths["c73_server"]),
        "scenario_sha256": sha256_file(paths["scenario"]),
        "runner_sha256": sha256_file(pathlib.Path(__file__).resolve()),
    }


def analyze_role_packet(
    role_dir: pathlib.Path,
    role: str,
    operations: int,
    minimum_periodic: int,
    *,
    host_mode: bool,
) -> dict:
    receipt_path = role_dir / "receipt.json"
    checkpoint_path = role_dir / "checkpoints.jsonl"
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    validate_receipt(receipt, role, operations, host_mode=host_mode)
    checkpoints, bounds = validate_checkpoints(
        checkpoint_path,
        minimum_periodic=minimum_periodic,
        resources_required=host_mode,
    )
    artifact_bytes = directory_size(role_dir)
    if host_mode and artifact_bytes > ROLE_ARTIFACT_LIMIT:
        raise ValueError("role artifact exceeded 64 MiB")
    return {
        "receipt_sha256": sha256_file(receipt_path),
        "checkpoints_sha256": sha256_file(checkpoint_path),
        "checkpoint_count": len(checkpoints),
        "resource_bounds": bounds,
        "goodput": float(receipt["observation"]["achieved_rate_per_second"]),
        "cpu_per_operation": float(receipt["resources"]["cpu_seconds_per_completed_operation"] or 0),
        "p99": float(receipt["observation"]["latency"]["p99_us"]),
        "artifact_bytes": artifact_bytes,
    }


def run_host_role(options: argparse.Namespace, inputs: dict) -> int:
    role = options.role
    if role not in {"I73", "C73"}:
        raise ValueError("--role must be I73 or C73 in role mode")
    output = options.output.resolve()
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    harness = inputs["paths"][f"{role.lower()}_harness"]
    server = inputs["paths"][f"{role.lower()}_server"]
    receipt_path = output / "receipt.json"
    checkpoint_path = output / "checkpoints.jsonl"
    command, operations, _ = command_for_role(
        harness,
        server,
        role,
        receipt_path,
        checkpoint_path,
        options.daemon_cpu_set,
        options.loadgen_cpu_set,
        phase=options.phase,
        host_mode=True,
    )
    spec = PHASES[options.phase]
    attempt = run_role(
        command,
        output,
        timeout_seconds=int(spec["seconds"]) + POST_WORK_IDLE_SECONDS + 1_800,
    )
    result = {"role": role, "attempt": attempt, "valid": False}
    try:
        if attempt["exit_code"] != 0 or attempt["timed_out"]:
            raise ValueError("role process failed or timed out")
        analysis = analyze_role_packet(
            output,
            role,
            operations,
            int(spec["checkpoints"]) - 1,
            host_mode=True,
        )
        result.update({"valid": True, "analysis": analysis})
        if role == "I73":
            (output / "baseline-bounds.json").write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "release": "0.73",
                        "profile_id": PROFILE_ID,
                        "source_sha": I73_SHA,
                        "phase": options.phase,
                        "resource_bounds": analysis["resource_bounds"],
                        "sealed_before_candidate": True,
                    },
                    indent=2,
                    sort_keys=True,
                )
                + "\n"
            )
    except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        result["error"] = str(error)
    (output / "attempt.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    return 0 if result["valid"] else 1


def validate_calibrations(root: pathlib.Path, tooling_sha: str) -> dict:
    names = ["pre-i73", "post-i73", "pre-c73", "post-c73"]
    receipts = {
        name: json.loads((root / "calibration" / f"{name}.json").read_text(encoding="utf-8"))
        for name in names
    }
    for name, receipt in receipts.items():
        if (
            receipt.get("source_sha") != tooling_sha
            or receipt.get("result") != "success"
            or receipt.get("ship_evidence_eligible") is not True
            or receipt.get("calibration", {}).get("relative_spread", math.inf)
            > receipt.get("calibration_limit", -math.inf)
        ):
            raise ValueError(f"{name} calibration is not admitted")
    first = receipts[names[0]]
    for name in names[1:]:
        receipt = receipts[name]
        if (
            receipt.get("host_fingerprint") != first.get("host_fingerprint")
            or receipt.get("identity_probes") != first.get("identity_probes")
            or receipt.get("lease") != first.get("lease")
        ):
            raise ValueError(f"host identity or lease drifted at {name}")
    return {
        "host_fingerprint": first["host_fingerprint"],
        "lease": first["lease"],
        "sha256": {
            name: sha256_file(root / "calibration" / f"{name}.json") for name in names
        },
    }


def comparison_guards(baseline: dict, candidate: dict, *, include_slopes: bool) -> dict[str, bool]:
    guards = {
        "goodput": -relative_change(baseline["goodput"], candidate["goodput"])
        <= REGRESSION_BUDGETS["goodput"],
        "cpu_per_operation": relative_change(
            baseline["cpu_per_operation"], candidate["cpu_per_operation"]
        )
        <= REGRESSION_BUDGETS["cpu_per_operation"],
        "p99": relative_change(baseline["p99"], candidate["p99"])
        <= REGRESSION_BUDGETS["p99"],
    }
    if include_slopes:
        for signal_name in ["rss", "anonymous_pss"]:
            baseline_upper = max(
                0.0,
                float(baseline["resource_bounds"][signal_name]["upper_95_bytes_per_second"]),
            )
            candidate_upper = float(
                candidate["resource_bounds"][signal_name]["upper_95_bytes_per_second"]
            )
            guards[f"{signal_name}_slope"] = candidate_upper <= baseline_upper
    return guards


def seal_host_campaign(options: argparse.Namespace, inputs: dict) -> int:
    output = options.output.resolve()
    if not output.is_dir():
        raise ValueError(f"campaign output is missing: {output}")
    if not options.tooling_sha or len(options.tooling_sha) != 40:
        raise ValueError("--tooling-sha must be an exact commit")
    role_results = {
        role: json.loads((output / role.lower() / "attempt.json").read_text(encoding="utf-8"))
        for role in ["I73", "C73"]
    }
    if not all(result.get("valid") is True for result in role_results.values()):
        raise ValueError("both role packets must be valid before sealing")
    bounds = json.loads((output / "i73" / "baseline-bounds.json").read_text(encoding="utf-8"))
    if bounds.get("sealed_before_candidate") is not True or bounds.get("source_sha") != I73_SHA:
        raise ValueError("baseline bounds were not sealed before candidate execution")
    calibration = validate_calibrations(output, options.tooling_sha)
    baseline = role_results["I73"]["analysis"]
    candidate = role_results["C73"]["analysis"]
    guards = comparison_guards(baseline, candidate, include_slopes=True)
    passed = all(guards.values()) and directory_size(output) <= PACKET_ARTIFACT_LIMIT
    campaign = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE_ID,
        "phase": options.phase,
        "mode": "host",
        "result": "passed" if passed else "failed",
        "tooling_sha": options.tooling_sha,
        "baseline_source_sha": I73_SHA,
        "candidate_source_sha": C73_SHA,
        "role_order": ["I73", "C73"],
        "regression_budgets": REGRESSION_BUDGETS,
        "guards": guards,
        "roles": role_results,
        "calibration": calibration,
        "identity": {key: value for key, value in inputs.items() if key != "paths"},
        "packet_bytes": directory_size(output),
        "automatic_retry_allowed": False,
        "performance_claim_allowed": False,
        "confirmation_allowed": passed and options.phase == "qualification",
        "final_c73_allowed": passed and options.phase == "confirmation",
    }
    (output / "long-run-campaign.json").write_text(
        json.dumps(campaign, indent=2, sort_keys=True) + "\n"
    )
    return 0 if passed else 1


def run_canary(options: argparse.Namespace, inputs: dict) -> int:
    output = options.output.resolve()
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    role_dir = output / "attempt"
    receipt = role_dir / "receipt.json"
    checkpoints = role_dir / "checkpoints.jsonl"
    command, operations, _ = command_for_role(
        inputs["paths"]["c73_harness"],
        inputs["paths"]["c73_server"],
        "C73",
        receipt,
        checkpoints,
        options.daemon_cpu_set,
        options.loadgen_cpu_set,
        phase="qualification",
        host_mode=False,
    )
    env = os.environ.copy()
    env["HYDRACACHE_CANARY_DEFECT"] = "LONG073-MISSING-FINAL"
    attempt = run_role(command, role_dir, timeout_seconds=120, env=env)
    rejected = False
    rejection = None
    try:
        if attempt["exit_code"] != 0 or not receipt.is_file() or not checkpoints.is_file():
            raise ValueError("canary process did not produce its intentionally incomplete packet")
        validate_receipt(json.loads(receipt.read_text(encoding="utf-8")), "C73", operations, host_mode=False)
        validate_checkpoints(checkpoints, minimum_periodic=1, resources_required=False)
    except (KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
        rejected = True
        rejection = str(error)
    result = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE_ID,
        "result": "passed" if rejected else "failed",
        "marker": CANARY_MARKER,
        "marker_observed": rejected,
        "receipt_absent": not (output / "long-run-campaign.json").exists(),
        "rejection": rejection,
        "attempt": attempt,
        "identity": {key: value for key, value in inputs.items() if key != "paths"},
    }
    output.mkdir(parents=True, exist_ok=True)
    (output / "canary.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
    return 0 if rejected else 1


def run_campaign(options: argparse.Namespace, inputs: dict, *, host_mode: bool) -> int:
    output = options.output.resolve()
    if output.exists():
        raise ValueError(f"output already exists: {output}")
    output.mkdir(parents=True)
    phase = options.phase
    spec = PHASES[phase]
    roles = {
        "I73": (inputs["paths"]["i73_harness"], inputs["paths"]["i73_server"]),
        "C73": (inputs["paths"]["c73_harness"], inputs["paths"]["c73_server"]),
    }
    role_results = []
    analyses = {}
    for role in ["I73", "C73"]:
        role_dir = output / role.lower()
        receipt_path = role_dir / "receipt.json"
        checkpoint_path = role_dir / "checkpoints.jsonl"
        command, operations, _ = command_for_role(
            *roles[role],
            role,
            receipt_path,
            checkpoint_path,
            options.daemon_cpu_set,
            options.loadgen_cpu_set,
            phase=phase,
            host_mode=host_mode,
        )
        timeout = int(spec["seconds"]) + POST_WORK_IDLE_SECONDS + 1_800 if host_mode else 180
        attempt = run_role(command, role_dir, timeout_seconds=timeout)
        result = {"role": role, "attempt": attempt, "valid": False}
        try:
            if attempt["exit_code"] != 0 or attempt["timed_out"]:
                raise ValueError("role process failed or timed out")
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            validate_receipt(receipt, role, operations, host_mode=host_mode)
            minimum = int(spec["checkpoints"]) - 1 if host_mode else 1
            checkpoints, bounds = validate_checkpoints(
                checkpoint_path,
                minimum_periodic=minimum,
                resources_required=host_mode,
            )
            if host_mode and directory_size(role_dir) > ROLE_ARTIFACT_LIMIT:
                raise ValueError("role artifact exceeded 64 MiB")
            analysis = {
                "receipt_sha256": sha256_file(receipt_path),
                "checkpoints_sha256": sha256_file(checkpoint_path),
                "checkpoint_count": len(checkpoints),
                "resource_bounds": bounds,
                "goodput": float(receipt["observation"]["achieved_rate_per_second"]),
                "cpu_per_operation": float(receipt["resources"]["cpu_seconds_per_completed_operation"] or 0),
                "p99": float(receipt["observation"]["latency"]["p99_us"]),
                "artifact_bytes": directory_size(role_dir),
            }
            analyses[role] = analysis
            result.update({"valid": True, "analysis": analysis})
            if role == "I73":
                (output / "baseline-bounds.json").write_text(
                    json.dumps(
                        {
                            "schema_version": 1,
                            "release": "0.73",
                            "profile_id": PROFILE_ID,
                            "source_sha": I73_SHA,
                            "phase": phase,
                            "resource_bounds": bounds,
                            "sealed_before_candidate": True,
                        },
                        indent=2,
                        sort_keys=True,
                    )
                    + "\n"
                )
        except (OSError, KeyError, TypeError, ValueError, json.JSONDecodeError) as error:
            result["error"] = str(error)
        role_results.append(result)
        (role_dir / "attempt.json").write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")
        if not result["valid"]:
            break

    guards = {}
    if set(analyses) == {"I73", "C73"}:
        baseline = analyses["I73"]
        candidate = analyses["C73"]
        guards = comparison_guards(baseline, candidate, include_slopes=host_mode)
    passed = len(role_results) == 2 and all(item["valid"] for item in role_results) and all(guards.values())
    campaign = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": PROFILE_ID,
        "phase": phase,
        "mode": "host" if host_mode else "local-screen",
        "result": "passed" if passed else "failed",
        "baseline_source_sha": I73_SHA,
        "candidate_source_sha": C73_SHA,
        "role_order": ["I73", "C73"],
        "offered_rate_per_second": RATE if host_mode else 1_000,
        "duration_seconds_per_role": int(spec["seconds"]) if host_mode else 2,
        "operations_per_role": int(spec["operations"]) if host_mode else 2_000,
        "regression_budgets": REGRESSION_BUDGETS,
        "guards": guards,
        "roles": role_results,
        "identity": {key: value for key, value in inputs.items() if key != "paths"},
        "automatic_retry_allowed": False,
        "performance_claim_allowed": False,
        "confirmation_allowed": passed and phase == "qualification" and host_mode,
        "final_c73_allowed": passed and phase == "confirmation" and host_mode,
    }
    campaign_path = output / "long-run-campaign.json"
    campaign_path.write_text(json.dumps(campaign, indent=2, sort_keys=True) + "\n")
    if directory_size(output) > PACKET_ARTIFACT_LIMIT:
        campaign["result"] = "failed"
        campaign["packet_error"] = "packet exceeded 256 MiB"
        campaign_path.write_text(json.dumps(campaign, indent=2, sort_keys=True) + "\n")
        passed = False
    return 0 if passed else 1


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--mode", choices=["canary", "local-screen", "role", "seal"], required=True
    )
    parser.add_argument("--phase", choices=sorted(PHASES), default="qualification")
    parser.add_argument("--role", choices=["I73", "C73"])
    parser.add_argument("--tooling-sha")
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
        if options.mode == "canary":
            return run_canary(options, inputs)
        if options.mode == "local-screen":
            return run_campaign(options, inputs, host_mode=False)
        if options.mode == "role":
            return run_host_role(options, inputs)
        return seal_host_campaign(options, inputs)
    except (OSError, ValueError) as error:
        print(f"long-run runner rejected input: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
