#!/usr/bin/env python3
"""Finite phase B0 native/TCP controls. No product admission or expensive dispatch."""
from __future__ import annotations

import argparse
import json
import math
import os
import pathlib
import subprocess
import sys
import time
import tomllib
from typing import Any

from performance_get_owner_screen_074 import digest, interval, require, write_new
from performance_local_pairing_074 import apply_windows_placement, background_cpu_percent

PROFILE = "get-owner-local-controls-074-v1"
POLICY = "docs/testing/performance/0.74/get-response-owner-phase-b0-contract.toml"
FIELDS = ["surface", "operation", "payload", "concurrency", "pipeline", "operations", "warmup"]
CELLS = [
    ("embedded-get-4k-c1", "embedded", "get", 4096, 1, 1, 1000000, 100000),
    ("embedded-get-4k-c8", "embedded", "get", 4096, 8, 1, 1000000, 100000),
    ("client-get-4k-c1", "client-surface", "get", 4096, 1, 1, 500000, 50000),
    ("client-get-4k-c8", "client-surface", "get", 4096, 8, 1, 500000, 50000),
    ("client-set-4k-c1", "client-surface", "set", 4096, 1, 1, 100000, 10000),
    ("client-set-4k-c8", "client-surface", "set", 4096, 8, 1, 100000, 10000),
    ("resp-get-4k-p1-c1", "resp-tcp", "get", 4096, 1, 1, 20000, 2000),
    ("resp-get-4k-p1-c8", "resp-tcp", "get", 4096, 8, 1, 20000, 2000),
    ("resp-get-4k-p50-c8", "resp-tcp", "get", 4096, 8, 50, 80000, 8000),
    ("resp-get-1m-p1-c1", "resp-tcp", "get", 1048576, 1, 1, 2000, 200),
]


def policy_check(policy: dict[str, Any]) -> None:
    require(policy["profile_id"] == PROFILE and policy["proposal_id"] == "p74-get-response-owner-v1", "policy identity drift")
    for key in ["promotable", "product_numeric_claims_allowed", "product_mutation_allowed", "expensive_workloads_allowed", "qualification_allowed", "prior_rejections_reopened"]:
        require(policy[key] is False, f"closed boundary changed: {key}")
    expected = {"seed": 740074, "runtime_workers": 2, "os": "windows", "key_space": 16, "cpu_ids": [0, 1], "priority": "normal", "quiet_sample_milliseconds": 500, "maximum_background_cpu_percent": 10.0, "independent_aa_pairs_per_cell_per_lane": 5, "independent_ab_pairs_per_cell_per_lane": 5, "total_fresh_process_attempts": 400, "maximum_attempt_seconds": 60, "maximum_failed_attempts": 0, "minimum_timing_elapsed_seconds": 1.0, "minimum_timing_cpu_seconds": 1.0, "instrumentation": False, "feature": "get-owner", "product_feature": "experimental-resp-get-owner-074", "allocation_feature": "allocation-profile"}
    for key, value in expected.items():
        require(policy["screen"][key] == value, f"sealed screen changed: {key}")
    for key, value in {"minimum_goodput_ratio": 0.98, "maximum_cpu_ratio": 1.03, "maximum_p99_ratio": 1.03, "maximum_gross_allocation_ratio": 1.05, "maximum_aa_goodput_relative_noise": 0.03, "maximum_aa_cpu_relative_noise": 0.03, "maximum_aa_p99_relative_noise": 0.10, "maximum_aa_gross_relative_noise": 0.01}.items():
        require(policy["guards"][key] == value, f"sealed guard changed: {key}")
    require([tuple(cell[key] for key in ["id", *FIELDS]) for cell in policy["cell"]] == CELLS, "sealed matrix changed")
    for key in ["complete_d3_allowed", "accepted_product_change", "default_enabled"]:
        require(policy["pending"][key] is False, f"admission drift: {key}")
    for key in ["hc1_hc2_controls_required", "matched_mtls_resp3_required", "concurrency_32_128_required", "scheduled_per_operation_latency_required", "miss_error_slow_reader_size_transition_required", "allocator_active_resident_retained_idle_refill_required", "feature_on_hosted_ci_receipt_required"]:
        require(policy["pending"][key] is True, f"required guard waived: {key}")
    require(policy["pending"]["integrated_c74"] == "UNRESOLVED", "integrated C74 changed")


def schedule(policy: dict[str, Any]) -> list[tuple[str, str, int, str, str]]:
    return [(lane, mode, pair, cell["id"], role)
            for lane in ["timing", "allocation"] for mode in ["aa", "ab"]
            for pair in range(1, 6) for cell in policy["cell"]
            for role in (["off", "on"] if pair % 2 else ["on", "off"])]


def variant(lane: str, mode: str, role: str) -> str:
    return f"{lane}-{'on' if mode == 'ab' and role == 'on' else 'off'}"


def numeric(receipt: dict[str, Any], key: str) -> float:
    value = receipt[key]
    require(type(value) in (int, float) and math.isfinite(value) and value > 0, f"invalid metric {key}")
    return float(value)


def validate(receipt: dict[str, Any], cell: dict[str, Any], lane: str, enabled: bool, seal: dict[str, Any]) -> None:
    require(receipt["profile_id"] == PROFILE and receipt["source_commit"] == seal["source_commit"] and receipt["source_clean"] is True, "source/profile mismatch")
    binary = seal["binaries"][f"{lane}-{'on' if enabled else 'off'}"]
    require(receipt["binary_sha256"] == binary["sha256"], "binary mismatch")
    require(receipt["get_owner_enabled"] is enabled and receipt["allocation_profile_enabled"] is (lane == "allocation"), "compiled variant mismatch")
    require(receipt["instrumentation_enabled"] is False and receipt["promotable"] is False, "profiling/admission drift")
    require(receipt["exact_result_validation"] is True and receipt["exact_final_values"] is True, "semantic proof missing")
    expected = {key: cell[key] for key in FIELDS} | {"key_space": 16, "seed": 740074}
    require(receipt["workload"] == expected, "workload drift")
    require(receipt["runtime_workers"] == 2, "runtime topology drift")
    require(receipt["latency_samples"] == cell["operations"] // cell["pipeline"], "latency denominator drift")
    require(receipt["latency_unit"] == ("closed_loop_operation" if cell["pipeline"] == 1 else "closed_loop_pipeline_batch"), "batch/per-operation latency conflation")
    for key in ["p50_us", "p95_us", "p99_us", "elapsed_seconds", "cpu_seconds", "cpu_nanoseconds_per_operation", "goodput_operations_per_second"]:
        numeric(receipt, key)
    require(receipt["p50_us"] <= receipt["p95_us"] <= receipt["p99_us"], "non-monotonic quantiles")
    require(math.isclose(receipt["goodput_operations_per_second"], cell["operations"] / receipt["elapsed_seconds"], rel_tol=1e-12), "goodput denominator drift")
    require(math.isclose(receipt["cpu_nanoseconds_per_operation"], receipt["cpu_seconds"] * 1e9 / cell["operations"], rel_tol=1e-12), "CPU denominator drift")
    if lane == "timing":
        require(receipt["gross_allocated_bytes_per_operation"] is None and "allocation" not in receipt, "timing must not contain counting allocator evidence")
        require(receipt["elapsed_seconds"] >= 1.0 and receipt["cpu_seconds"] >= 1.0, "timing window too short for frozen precision guard")
    else:
        gross = numeric(receipt["allocation"], "gross_allocated_bytes")
        require(receipt["gross_allocated_bytes_per_operation"] == gross / cell["operations"], "gross denominator drift")


def metrics(lane: str) -> list[str]:
    return ["goodput_operations_per_second", "cpu_nanoseconds_per_operation", "p99_us"] if lane == "timing" else ["gross_allocated_bytes_per_operation"]


def aa_guard(left: dict[str, Any], right: dict[str, Any], lane: str, policy: dict[str, Any]) -> None:
    ceilings = [0.03, 0.03, 0.10] if lane == "timing" else [0.01]
    for metric, ceiling in zip(metrics(lane), ceilings):
        require(abs(numeric(right, metric) / numeric(left, metric) - 1) <= ceiling + 1e-12, f"AA noise invalidates {lane}/{metric}; no tuning or retry")


def analyse(attempts: list[dict[str, Any]], policy: dict[str, Any], seal: dict[str, Any]) -> dict[str, Any]:
    policy_check(policy)
    expected = schedule(policy)
    require(len(attempts) == 400, "partial matrix cannot produce a comparison")
    grouped: dict[tuple[str, str, str], list[dict[str, Any]]] = {}
    cells = {cell["id"]: cell for cell in policy["cell"]}
    traces: dict[str, set[str]] = {}
    for row, attempt in zip(expected, attempts):
        require(tuple(attempt[key] for key in ["lane", "mode", "pair", "cell", "role"]) == row, "attempt order drift")
        lane, mode, _, cell_id, role = row
        require(attempt["exit_code"] == 0 and attempt["affinity_applied"] is True and attempt["priority_applied"] is True and attempt["placement_before_warmup"] is True, "attempt failed or placement missing")
        require(type(attempt["background_cpu_percent"]) in (int, float) and 0 <= attempt["background_cpu_percent"] <= 10, "background CPU outside frozen ceiling")
        validate(attempt["receipt"], cells[cell_id], lane, mode == "ab" and role == "on", seal)
        traces.setdefault(cell_id, set()).add(attempt["receipt"]["workload_sha256"])
        grouped.setdefault((lane, mode, cell_id), []).append(attempt)
    require(all(len(values) == 1 for values in traces.values()), "AA/AB/lane trace drift")
    results = []
    for lane in ["timing", "allocation"]:
        for cell in policy["cell"]:
            ratios = {metric: [] for metric in metrics(lane)}
            aa = grouped[(lane, "aa", cell["id"])]
            for i in range(0, 10, 2):
                aa_guard(aa[i]["receipt"], aa[i + 1]["receipt"], lane, policy)
            ab = grouped[(lane, "ab", cell["id"])]
            for i in range(0, 10, 2):
                roles = {item["role"]: item["receipt"] for item in ab[i:i + 2]}
                for metric in metrics(lane):
                    ratios[metric].append(numeric(roles["on"], metric) / numeric(roles["off"], metric))
            intervals = {key: interval(values) for key, values in ratios.items()}
            passed = all(value["lower_95"] >= 0.98 if key == "goodput_operations_per_second" else value["upper_95"] <= (1.05 if lane == "allocation" else 1.03) for key, value in intervals.items())
            results.append({"lane": lane, "cell": cell["id"], "guard_passed": passed, "intervals": intervals})
    return {"profile_id": PROFILE, "source_commit": seal["source_commit"], "attempts_retained": 400, "promotable": False, "accepted_product_change": False, "product_performance_claim": False, "classification": "local-b0-guards-passed-full-d3-still-required" if all(row["guard_passed"] for row in results) else "local-b0-guard-red", "results": results, "pending": policy["pending"]}


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def replay(packet: pathlib.Path, policy: dict[str, Any], contract_hash: str) -> dict[str, Any]:
    seal = json.loads((packet / "seal.json").read_text(encoding="utf-8"))
    require(seal["contract_sha256"] == contract_hash and seal["policy"] == policy, "contract seal drift")
    require(seal["schedule"] == [list(row) for row in schedule(policy)], "sealed schedule drift")
    attempts = []
    expected_files = {"seal.json", "summary.json"}
    for ordinal in range(1, 401):
        prefix = f"attempt-{ordinal:04}"
        expected_files.update(f"{prefix}/{name}" for name in ["raw.json", "attempt.json", "ready", "go"])
        directory = packet / prefix
        attempt = json.loads((directory / "attempt.json").read_text(encoding="utf-8"))
        require(attempt["raw_sha256"] == digest(directory / "raw.json"), "raw bytes changed")
        require(attempt["receipt"] == json.loads((directory / "raw.json").read_text(encoding="utf-8")), "raw receipt mismatch")
        require((directory / "ready").read_bytes() == b"" and (directory / "go").read_bytes() == b"", "placement marker drift")
        attempts.append(attempt)
    require({path.relative_to(packet).as_posix() for path in packet.rglob("*") if path.is_file()} == expected_files, "packet file selection/drift")
    summary = analyse(attempts, policy, seal)
    require(summary == json.loads((packet / "summary.json").read_text(encoding="utf-8")), "summary differs from independent replay")
    return summary


def run(root: pathlib.Path, binaries: dict[str, pathlib.Path], output: pathlib.Path) -> dict[str, Any]:
    require(sys.platform == "win32", "local contract requires Windows")
    policy_path = root / POLICY
    policy = tomllib.loads(policy_path.read_text(encoding="utf-8")); policy_check(policy)
    source = git(root, "rev-parse", "HEAD")
    require(not git(root, "status", "--porcelain"), "source is dirty")
    require(output.is_relative_to(root / "target") and not output.exists(), "new packet must be beneath ignored root target")
    output.mkdir(parents=True)
    lock = root / "tools/get-owner-controls-074/Cargo.lock"
    seal = {"profile_id": PROFILE, "source_commit": source, "contract_sha256": digest(policy_path), "lock_sha256": digest(lock), "rustc": subprocess.check_output(["rustc", "-Vv"], text=True), "policy": policy, "schedule": [list(row) for row in schedule(policy)], "binaries": {name: {"path": str(path), "sha256": digest(path)} for name, path in binaries.items()}, "promotable": False}
    write_new(output / "seal.json", seal)
    cells = {cell["id"]: cell for cell in policy["cell"]}
    attempts = []
    try:
        for ordinal, row in enumerate(schedule(policy), 1):
            lane, mode, pair, cell_id, role = row
            require(git(root, "rev-parse", "HEAD") == source and not git(root, "status", "--porcelain"), "source drift")
            require(digest(policy_path) == seal["contract_sha256"] and digest(lock) == seal["lock_sha256"], "contract/lock drift")
            binary = binaries[variant(lane, mode, role)]
            require(digest(binary) == seal["binaries"][variant(lane, mode, role)]["sha256"], "binary drift")
            directory = output / f"attempt-{ordinal:04}"; directory.mkdir()
            ready, go, raw = [directory / name for name in ["ready", "go", "raw.json"]]
            cell = cells[cell_id]
            command = [str(binary), "--source", source, "--output", str(raw)]
            for key, value in ({name: cell[name] for name in FIELDS} | {"key_space": 16, "seed": 740074}).items():
                command += ["--" + key.replace("_", "-"), str(value)]
            attempt = dict(zip(["lane", "mode", "pair", "cell", "role"], row))
            attempt.update(command=command, exit_code=None, affinity_applied=False, priority_applied=False, placement_before_warmup=False, stdout="", stderr="")
            attempts.append(attempt)
            process = None
            try:
                attempt["background_cpu_percent"] = background_cpu_percent(500)
                require(0 <= attempt["background_cpu_percent"] <= 10, "background CPU outside frozen ceiling")
                env = os.environ | {"HYDRACACHE_GET_OWNER_CONTROL_READY": str(ready), "HYDRACACHE_GET_OWNER_CONTROL_GO": str(go)}
                started = time.monotonic()
                process = subprocess.Popen(command, cwd=root, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
                deadline = started + 30
                while not ready.is_file() and process.poll() is None and time.monotonic() < deadline:
                    time.sleep(0.01)
                require(ready.is_file() and process.poll() is None, "prepare/placement gate failed")
                affinity, priority = apply_windows_placement(process, [0, 1], "normal")
                attempt.update(affinity_applied=affinity, priority_applied=priority)
                require(affinity and priority, "placement failed")
                with go.open("xb"):
                    pass
                attempt["placement_before_warmup"] = True
                stdout, stderr = process.communicate(timeout=max(0.01, 60 - (time.monotonic() - started)))
                attempt.update(stdout=stdout, stderr=stderr, exit_code=process.returncode)
                require(process.returncode == 0 and raw.is_file(), "attempt execution failed")
                attempt["raw_sha256"] = digest(raw)
                attempt["receipt"] = json.loads(raw.read_text(encoding="utf-8"))
                validate(attempt["receipt"], cell, lane, mode == "ab" and role == "on", seal)
                require(git(root, "rev-parse", "HEAD") == source and not git(root, "status", "--porcelain"), "source drift during attempt")
                require(digest(binary) == seal["binaries"][variant(lane, mode, role)]["sha256"] and digest(policy_path) == seal["contract_sha256"] and digest(lock) == seal["lock_sha256"], "sealed artifact drift during attempt")
                if ordinal % 2 == 0:
                    require(attempts[-2]["receipt"]["workload_sha256"] == attempt["receipt"]["workload_sha256"], "pair trace drift")
                    if mode == "aa":
                        aa_guard(attempts[-2]["receipt"], attempt["receipt"], lane, policy)
            except Exception as error:
                if process is not None:
                    if process.poll() is None:
                        process.kill()
                    stdout, stderr = process.communicate()
                    attempt.update(exit_code=process.returncode, stdout=stdout, stderr=stderr)
                if raw.exists(): attempt["raw_sha256"] = digest(raw)
                attempt["failure"] = str(error)
                raise
            finally:
                write_new(directory / "attempt.json", attempt)
            print(f"retained {ordinal}/400 {lane} {mode} {cell_id} {role}", flush=True)
        summary = analyse(attempts, policy, seal)
    except Exception as error:
        summary = {"profile_id": PROFILE, "classification": "invalidated", "reason": str(error), "attempts_retained": len(attempts), "promotable": False, "accepted_product_change": False, "product_performance_claim": False}
    write_new(output / "summary.json", summary)
    return summary


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[2])
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--replay", type=pathlib.Path)
    for lane in ["timing", "allocation"]:
        for role in ["off", "on"]:
            parser.add_argument(f"--{lane}-{role}", type=pathlib.Path)
    options = parser.parse_args(); root = options.root.resolve()
    policy_path = root / POLICY
    policy = tomllib.loads(policy_path.read_text(encoding="utf-8")); policy_check(policy)
    if options.replay:
        summary = replay(options.replay.resolve(), policy, digest(policy_path))
    else:
        binaries = {f"{lane}-{role}": getattr(options, f"{lane}_{role}") for lane in ["timing", "allocation"] for role in ["off", "on"]}
        require(options.output is not None and all(binaries.values()), "output and four binaries required")
        summary = run(root, {key: value.resolve() for key, value in binaries.items()}, options.output.resolve())
    print(json.dumps(summary, indent=2, allow_nan=False))
    return 1 if summary["classification"] == "invalidated" else 0


if __name__ == "__main__":
    raise SystemExit(main())
