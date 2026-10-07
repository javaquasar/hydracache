#!/usr/bin/env python3
"""Sealed, non-promotable W9b allocation/memory early-rejection screen."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import pathlib
import statistics
import subprocess
import tomllib
from typing import Any

PROFILE = "w9b-resp-serial-scratch-allocation-memory-screen-074-v1"
POLICY = "docs/testing/performance/0.74/w9b-serial-scratch-d3-contract.toml"
T4 = 2.7764451051977987


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def digest(path: pathlib.Path) -> str:
    return "sha256:" + hashlib.sha256(path.read_bytes()).hexdigest()


def write_new(path: pathlib.Path, value: Any) -> None:
    with path.open("x", encoding="utf-8") as output:
        json.dump(value, output, indent=2, allow_nan=False)
        output.write("\n")


def schedule(policy: dict[str, Any]) -> list[tuple[str, int, str, str]]:
    rows = []
    for mode in ["aa", "ab"]:
        for pair in range(1, 6):
            for cell in policy["cell"]:
                roles = ["off", "on"] if pair % 2 else ["on", "off"]
                for role in roles:
                    rows.append((mode, pair, cell["id"], role))
    return rows


def validate_policy(policy: dict[str, Any]) -> None:
    require(policy["profile_id"] == PROFILE, "contract profile mismatch")
    for key, expected in {"independent_aa_pairs_per_cell": 5, "independent_ab_pairs_per_cell": 5, "seed": 740074, "warmup_batches": 5, "minimum_affected_gross_allocation_reduction": 0.20, "maximum_unaffected_gross_allocation_ratio": 1.05, "maximum_peak_live_above_start_ratio": 1.00, "maximum_next_read_or_post_close_live_increase_bytes": 0, "maximum_aa_allocation_or_live_relative_noise": 0.01, "maximum_attempt_seconds": 30}.items():
        require(policy["phase_a"][key] == expected, f"sealed policy changed: {key}")
    require(policy["promotable"] is False and policy["product_numeric_claims_allowed"] is False, "local screen claim boundary changed")


def interval(ratios: list[float]) -> dict[str, Any]:
    require(len(ratios) == 5, "exactly five independent pairs required")
    require(all(math.isfinite(value) and value > 0 for value in ratios), "invalid ratio")
    logs = [math.log(value) for value in ratios]
    center = statistics.mean(logs)
    half = T4 * statistics.stdev(logs) / math.sqrt(5)
    return {"paired_ratios": ratios, "geometric_mean": math.exp(center), "lower_95": math.exp(center - half), "upper_95": math.exp(center + half)}


def number(receipt: dict[str, Any], *path: str) -> float:
    value: Any = receipt
    for field in path:
        value = value[field]
    require(type(value) in (int, float) and math.isfinite(value), f"invalid numeric metric {path}")
    return float(value)


def validate(receipt: dict[str, Any], cell: dict[str, Any], source: str, binary: str, enabled: bool) -> None:
    require(receipt["profile_id"] == PROFILE, "profile drift")
    require(receipt["source_commit"] == source and receipt["source_clean"] is True, "source drift")
    require(receipt["binary_sha256"] == binary, "binary drift")
    require(receipt["serial_scratch_enabled"] is enabled, "compiled feature mismatch")
    require(receipt["promotable"] is False, "screen cannot promote")
    require(receipt["exact_response_validation"] is True and receipt["exact_final_values_and_cardinality"] is True, "semantic validation missing")
    for name, key in [("operation", "operation"), ("payload_bytes", "payload"), ("alternate_payload_bytes", "alternate_payload"), ("pipeline", "pipeline"), ("batches", "batches"), ("read_chunk_bytes", "read_chunk")]:
        require(receipt[name] == cell[key], f"workload drift {name}")
    require(receipt["seed"] == 740074 and receipt["concurrency"] == 1 and receipt["warmup_batches"] == 5, "seed/concurrency/warmup drift")
    operations = cell["pipeline"] * cell["batches"]
    require(receipt["operations"] == operations and receipt["write_calls"] == operations and receipt["flush_calls"] == operations, "operation/write/flush mismatch")
    require(receipt["next_read_boundary_samples"] == cell["batches"], "missing owner boundary")
    gross = number(receipt, "allocation", "gross_allocated_bytes")
    require(gross > 0 and number(receipt, "allocation", "peak_live_above_start_bytes") > 0, "empty measured allocation window")
    require(number(receipt, "gross_allocated_bytes_per_operation") == gross / operations, "allocation denominator mismatch")
    for metric in ["live_before_bytes", "live_after_bytes", "peak_live_requested_bytes"]:
        require(number(receipt, "allocation", metric) >= 0, "negative live bytes")
    require(number(receipt, "allocation", "peak_live_requested_bytes") - number(receipt, "allocation", "live_before_bytes") == number(receipt, "allocation", "peak_live_above_start_bytes"), "peak window denominator mismatch")


def analyse(attempts: list[dict[str, Any]], policy: dict[str, Any], seal: dict[str, Any]) -> dict[str, Any]:
    validate_policy(policy)
    expected = schedule(policy)
    require(len(attempts) == len(expected), "partial matrix; no result selection")
    cells = {cell["id"]: cell for cell in policy["cell"]}
    grouped: dict[tuple[str, str], list[tuple[dict[str, Any], dict[str, Any]]]] = {}
    for index, row in enumerate(expected):
        attempt = attempts[index]
        require(tuple(attempt[name] for name in ["mode", "pair", "cell", "role"]) == row, "attempt order drift")
        require(attempt["exit_code"] == 0, "failed attempt; no retry")
        mode, pair, cell_id, role = row
        enabled = mode == "ab" and role == "on"
        binary = seal["on_binary_sha256" if enabled else "off_binary_sha256"]
        validate(attempt["receipt"], cells[cell_id], seal["source_commit"], binary, enabled)
        if index % 2:
            previous = attempts[index - 1]
            require(previous["receipt"]["workload_sha256"] == attempt["receipt"]["workload_sha256"], "pair trace drift")
            roles = {previous["role"]: previous["receipt"], role: attempt["receipt"]}
            grouped.setdefault((mode, cell_id), []).append((roles["off"], roles["on"]))
    results = []
    red = []
    for cell in policy["cell"]:
        cell_id = cell["id"]
        aa, ab = grouped[("aa", cell_id)], grouped[("ab", cell_id)]
        traces = {receipt["workload_sha256"] for pairs in [aa, ab] for pair in pairs for receipt in pair}
        require(len(traces) == 1, "trace drift between AA/AB repeats")
        metric_paths = {"gross": ("allocation", "gross_allocated_bytes"), "peak": ("allocation", "peak_live_above_start_bytes")}
        metrics = {}
        for name, path in metric_paths.items():
            aa_ratios = [number(on, *path) / number(off, *path) for off, on in aa]
            require(max(abs(value - 1) for value in aa_ratios) <= 0.01, f"AA {name} noise exceeds sealed ceiling for {cell_id}")
            metrics[name] = interval([number(on, *path) / number(off, *path) for off, on in ab])
            metrics[name]["aa_ratios"] = aa_ratios
        gross_ceiling = 0.80 if cell["affected"] else 1.05
        gross_pass = metrics["gross"]["upper_95"] <= gross_ceiling + 1e-12
        peak_pass = metrics["peak"]["upper_95"] <= 1.00 + 1e-12
        idle_deltas = [
            (number(on, "next_read_boundary_live_max_bytes") - number(on, "allocation", "live_before_bytes"))
            - (number(off, "next_read_boundary_live_max_bytes") - number(off, "allocation", "live_before_bytes"))
            for off, on in ab
        ]
        close_deltas = [
            (number(on, "allocation", "live_after_bytes") - number(on, "allocation", "live_before_bytes"))
            - (number(off, "allocation", "live_after_bytes") - number(off, "allocation", "live_before_bytes"))
            for off, on in ab
        ]
        owner_pass = max(idle_deltas + close_deltas) <= 0
        if not (gross_pass and peak_pass and owner_pass):
            red.append(cell_id)
        results.append({"cell": cell_id, "affected": cell["affected"], "gross": metrics["gross"], "peak": metrics["peak"], "gross_guard_passed": gross_pass, "peak_guard_passed": peak_pass, "next_read_live_delta_bytes_per_pair": idle_deltas, "close_live_delta_bytes_per_pair": close_deltas, "owner_guard_passed": owner_pass})
    return {"schema_version": 1, "profile_id": PROFILE, "source_commit": seal["source_commit"], "promotable": False, "accepted_product_change": False, "product_performance_claim": False, "classification": "rejected-local-allocation-memory-screen" if red else "screen-passed-full-d3-controls-still-required", "red_cells": red, "attempts_retained": len(attempts), "results": results, "pending": ["unprofiled timing and exact per-operation scheduled latency", "embedded ClientSurfaceState HC1 HC2 independent native controls", "real plaintext/mTLS transport and concurrency cohorts", "allocator active/resident/retained and timed idle/RSS proof", "feature-on hosted-CI gate evidence"]}


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def run(root: pathlib.Path, off: pathlib.Path, on: pathlib.Path, output: pathlib.Path) -> dict[str, Any]:
    policy_path = root / POLICY
    policy = tomllib.loads(policy_path.read_text(encoding="utf-8"))
    validate_policy(policy)
    require(not git(root, "status", "--porcelain", "--untracked-files=normal"), "dirty source; cannot seal")
    require(not output.exists(), "output directory exists; no retry/overwrite")
    source = git(root, "rev-parse", "HEAD")
    seal = {"schema_version": 1, "profile_id": PROFILE, "source_commit": source, "contract_sha256": digest(policy_path), "off_binary_sha256": digest(off), "on_binary_sha256": digest(on), "tool_lock_sha256": digest(root / "tools/resp-scratch-screen-074/Cargo.lock"), "rustc": subprocess.check_output(["rustc", "-Vv"], text=True), "promotable": False, "schedule": schedule(policy), "same_source_feature_off_baseline": True, "legacy_default_product_reference": policy["legacy_default_product_baseline_sha"]}
    require(seal["off_binary_sha256"] != seal["on_binary_sha256"], "AB binaries must differ")
    output.mkdir(parents=True)
    write_new(output / "seal.json", seal)
    attempts: list[dict[str, Any]] = []
    cells = {cell["id"]: cell for cell in policy["cell"]}
    try:
        for ordinal, (mode, pair, cell_id, role) in enumerate(schedule(policy), 1):
            require(git(root, "rev-parse", "HEAD") == source and not git(root, "status", "--porcelain", "--untracked-files=normal"), "source drift after seal")
            enabled = mode == "ab" and role == "on"
            binary = on if enabled else off
            require(digest(binary) == seal["on_binary_sha256" if enabled else "off_binary_sha256"], "binary drift after seal")
            cell = cells[cell_id]
            receipt_path = output / f"{ordinal:03d}-{mode}-{pair}-{cell_id}-{role}.json"
            command = [str(binary), "--source", source, "--output", str(receipt_path), "--warmup-batches", "5"]
            for key in ["operation", "payload", "alternate_payload", "pipeline", "batches", "read_chunk"]:
                command.extend(["--" + key.replace("_", "-"), str(cell[key])])
            attempt: dict[str, Any] = {"ordinal": ordinal, "mode": mode, "pair": pair, "cell": cell_id, "role": role, "compiled_feature_enabled": enabled, "command": command}
            try:
                result = subprocess.run(command, cwd=root, capture_output=True, text=True, timeout=30)
                attempt.update(exit_code=result.returncode, stdout=result.stdout, stderr=result.stderr)
            except subprocess.TimeoutExpired as error:
                attempt.update(exit_code=None, failure="timeout", stdout=str(error.stdout), stderr=str(error.stderr))
            attempts.append(attempt)
            write_new(output / f"{ordinal:03d}.attempt.json", attempt)
            require(attempt["exit_code"] == 0, f"attempt {ordinal} failed; retained; no retry")
            attempt["receipt"] = json.loads(receipt_path.read_text(encoding="utf-8"))
            validate(attempt["receipt"], cell, source, seal["on_binary_sha256" if enabled else "off_binary_sha256"], enabled)
            print(f"{ordinal}/{len(schedule(policy))} {mode}/{pair} {cell_id} {role}", flush=True)
        summary = analyse(attempts, policy, seal)
    except Exception as error:
        summary = {"schema_version": 1, "profile_id": PROFILE, "source_commit": source, "promotable": False, "classification": "invalidated-no-rerun", "reason": str(error), "attempts_retained": len(attempts)}
    write_new(output / "summary.json", summary)
    return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True, type=pathlib.Path)
    parser.add_argument("--off", required=True, type=pathlib.Path)
    parser.add_argument("--on", required=True, type=pathlib.Path)
    parser.add_argument("--output", required=True, type=pathlib.Path)
    args = parser.parse_args()
    summary = run(args.root.resolve(), args.off.resolve(), args.on.resolve(), args.output.resolve())
    print(json.dumps(summary, indent=2, allow_nan=False))
    raise SystemExit(1 if summary["classification"] == "invalidated-no-rerun" else 0)
