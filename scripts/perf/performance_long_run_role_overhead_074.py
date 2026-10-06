#!/usr/bin/env python3
"""Validate and summarize a non-product W11 role-overhead rehearsal.

The collector is intentionally separate. This analyzer accepts only a complete, counterbalanced
attempt set with exact source and host identities. Its output is always non-promotable and cannot
close the product role-overhead or release-admission gates.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import statistics
import sys
import tomllib
from typing import Any


SCHEMA_VERSION = "hydracache-w11-role-overhead-rehearsal-v1"
ATTEMPT_SCHEMA_VERSION = 1
EVIDENCE_CLASS = "non-product-role-overhead-rehearsal"
ORDER = "abba-counterbalanced-v1"
ROLES = ("i74", "c74")
VARIANTS = ("control", "instrumented")
SHA1 = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
CPUSET = re.compile(r"^[0-9]+(?:-[0-9]+)?(?:,[0-9]+(?:-[0-9]+)?)*$")
MAX_INPUT_BYTES = 4 * 1024 * 1024

ROOT_FIELDS = {
    "schema_version",
    "release",
    "evidence_class",
    "promotable",
    "seed",
    "order",
    "attempts",
}
ATTEMPT_FIELDS = {
    "schema_version",
    "role",
    "variant",
    "pair_index",
    "position",
    "identity",
    "metrics",
    "guards",
}
IDENTITY_FIELDS = {
    "source_commit",
    "binary_sha256",
    "workload_sha256",
    "payload_sha256",
    "host_receipt_sha256",
    "seed",
    "operations",
    "warmup_operations",
    "cpuset",
}
METRIC_FIELDS = {
    "elapsed_ns",
    "role_cpu_ns",
    "role_rss_peak_bytes",
    "role_io_bytes",
    "supervisor_cpu_ns",
    "supervisor_rss_peak_bytes",
    "supervisor_io_bytes",
    "checkpoint_write_bytes",
    "completed_operations",
}
GUARD_FIELDS = {
    "affinity_applied",
    "priority_applied",
    "host_identity_stable",
    "role_identity_stable",
    "supervisor_identity_stable",
    "campaign_claim_valid",
    "unexpected_errors_absent",
}
CROSS_ROLE_IDENTITY_FIELDS = {
    "workload_sha256",
    "payload_sha256",
    "host_receipt_sha256",
    "seed",
    "operations",
    "warmup_operations",
    "cpuset",
}


def _exact_fields(value: Any, expected: set[str], context: str) -> dict[str, Any]:
    if not isinstance(value, dict) or set(value) != expected:
        raise ValueError(f"{context} fields are not exact")
    return value


def _integer(value: Any, context: str, *, positive: bool = False) -> int:
    if isinstance(value, bool) or not isinstance(value, int):
        raise ValueError(f"{context} must be an integer")
    if value < (1 if positive else 0):
        raise ValueError(f"{context} is outside the admitted range")
    return value


def _finite_positive(value: Any, context: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError(f"{context} must be numeric")
    result = float(value)
    if not math.isfinite(result) or result <= 0:
        raise ValueError(f"{context} must be finite and positive")
    return result


def _variant_order(pair_index: int, seed: int) -> tuple[str, str]:
    first = VARIANTS if seed % 2 == 0 else tuple(reversed(VARIANTS))
    return first if pair_index % 2 == 1 else tuple(reversed(first))


def _read_json(path: Path) -> tuple[dict[str, Any], str]:
    metadata = path.lstat()
    if (
        path.is_symlink()
        or not path.is_file()
        or metadata.st_nlink != 1
        or metadata.st_size <= 0
        or metadata.st_size > MAX_INPUT_BYTES
    ):
        raise ValueError("attempt input is not a bounded regular file")
    raw = path.read_bytes()
    try:
        value = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("attempt input is not valid JSON") from error
    if not isinstance(value, dict):
        raise ValueError("attempt input root must be an object")
    return value, hashlib.sha256(raw).hexdigest()


def _load_budgets(contract_path: Path, statistics_path: Path) -> dict[str, Any]:
    contract = tomllib.loads(contract_path.read_text(encoding="utf-8"))
    statistics_contract = tomllib.loads(statistics_path.read_text(encoding="utf-8"))
    if (
        contract.get("release") != "0.74"
        or contract.get("contract_id") != "long-run-controller-resilience-074-v1"
        or contract.get("state") != "frozen-before-candidate-data"
        or statistics_contract.get("release") != "0.74"
        or statistics_contract.get("contract_id") != "performance-statistics-074-v1"
        or statistics_contract.get("pairing")
        != "counterbalanced-independent-processes"
    ):
        raise ValueError("the frozen 0.74 overhead contracts are invalid")
    overhead = contract.get("overhead_budget")
    if not isinstance(overhead, dict):
        raise ValueError("the frozen overhead budget is missing")
    pairs = _integer(
        statistics_contract.get("minimum_claim_pairs"),
        "minimum_claim_pairs",
        positive=True,
    )
    if pairs != 5:
        raise ValueError("the frozen role-overhead rehearsal requires five pairs")
    return {
        "pairs": pairs,
        "maximum_supervisor_cpu_percent": _finite_positive(
            overhead.get("maximum_supervisor_cpu_percent"),
            "maximum_supervisor_cpu_percent",
        ),
        "maximum_supervisor_rss_bytes": _integer(
            overhead.get("maximum_supervisor_rss_bytes"),
            "maximum_supervisor_rss_bytes",
            positive=True,
        ),
        "maximum_checkpoint_io_bytes_per_second": _integer(
            overhead.get("maximum_checkpoint_io_bytes_per_second"),
            "maximum_checkpoint_io_bytes_per_second",
            positive=True,
        ),
        "asymmetry_percent": _finite_positive(
            overhead.get("asymmetry_percent"), "asymmetry_percent"
        ),
    }


def _validate_attempt(
    attempt: Any,
    role: str,
    variant: str,
    pair_index: int,
    position: int,
    seed: int,
    expected_source: str,
    expected_host_receipt: str,
) -> dict[str, Any]:
    attempt = _exact_fields(attempt, ATTEMPT_FIELDS, "attempt")
    if (
        _integer(attempt["schema_version"], "attempt schema version", positive=True)
        != ATTEMPT_SCHEMA_VERSION
        or attempt["role"] != role
        or attempt["variant"] != variant
        or _integer(attempt["pair_index"], "pair index", positive=True)
        != pair_index
        or _integer(attempt["position"], "attempt position", positive=True)
        != position
    ):
        raise ValueError("attempt order or identity does not match the frozen schedule")
    identity = _exact_fields(attempt["identity"], IDENTITY_FIELDS, "attempt identity")
    if (
        not isinstance(identity["source_commit"], str)
        or not SHA1.fullmatch(identity["source_commit"])
        or identity["source_commit"] != expected_source
    ):
        raise ValueError("attempt source identity drifted")
    for field in (
        "binary_sha256",
        "workload_sha256",
        "payload_sha256",
        "host_receipt_sha256",
    ):
        if not isinstance(identity[field], str) or not SHA256.fullmatch(identity[field]):
            raise ValueError(f"attempt identity {field} is invalid")
    if identity["host_receipt_sha256"] != expected_host_receipt:
        raise ValueError("attempt host receipt drifted")
    if identity["seed"] != seed:
        raise ValueError("attempt seed drifted")
    _integer(identity["operations"], "operations", positive=True)
    _integer(identity["warmup_operations"], "warmup_operations", positive=True)
    if not isinstance(identity["cpuset"], str) or not CPUSET.fullmatch(identity["cpuset"]):
        raise ValueError("attempt cpuset is not canonical")

    metrics = _exact_fields(attempt["metrics"], METRIC_FIELDS, "attempt metrics")
    for field in METRIC_FIELDS:
        _integer(metrics[field], f"attempt metric {field}", positive=field in {"elapsed_ns", "role_cpu_ns", "role_rss_peak_bytes", "supervisor_rss_peak_bytes", "completed_operations"})
    if metrics["completed_operations"] != identity["operations"]:
        raise ValueError("completed operation count does not match workload identity")
    if variant == "control" and metrics["checkpoint_write_bytes"] != 0:
        raise ValueError("control attempt wrote checkpoint bytes")
    if variant == "instrumented" and metrics["checkpoint_write_bytes"] <= 0:
        raise ValueError("instrumented attempt did not write checkpoint bytes")
    if metrics["checkpoint_write_bytes"] > metrics["role_io_bytes"]:
        raise ValueError("checkpoint bytes exceed measured role I/O")

    guards = _exact_fields(attempt["guards"], GUARD_FIELDS, "attempt guards")
    if any(value is not True for value in guards.values()):
        raise ValueError("attempt did not satisfy every frozen guard")
    return attempt


def _relative_percent(control: int, instrumented: int) -> float:
    if control <= 0:
        raise ValueError("control metric must be positive")
    return (instrumented / control - 1.0) * 100.0


def analyse(
    attempt_set: dict[str, Any],
    input_sha256: str,
    budgets: dict[str, Any],
    expected_sources: dict[str, str],
    expected_host_receipt: str,
) -> dict[str, Any]:
    root = _exact_fields(attempt_set, ROOT_FIELDS, "attempt-set")
    if (
        _integer(root["schema_version"], "attempt-set schema version", positive=True)
        != ATTEMPT_SCHEMA_VERSION
        or root["release"] != "0.74"
        or root["evidence_class"] != EVIDENCE_CLASS
        or root["promotable"] is not False
        or root["order"] != ORDER
    ):
        raise ValueError("attempt-set contract identity is invalid")
    seed = _integer(root["seed"], "seed")
    attempts = root["attempts"]
    expected_count = len(ROLES) * budgets["pairs"] * len(VARIANTS)
    if not isinstance(attempts, list) or len(attempts) != expected_count:
        raise ValueError("attempt-set does not contain the frozen number of attempts")
    if set(expected_sources) != set(ROLES):
        raise ValueError("expected role sources are incomplete")
    for source in expected_sources.values():
        if not isinstance(source, str) or not SHA1.fullmatch(source):
            raise ValueError("expected role source is invalid")
    if not isinstance(expected_host_receipt, str) or not SHA256.fullmatch(
        expected_host_receipt
    ):
        raise ValueError("expected host receipt is invalid")

    validated: list[dict[str, Any]] = []
    cursor = 0
    for role in ROLES:
        for pair_index in range(1, budgets["pairs"] + 1):
            for position, variant in enumerate(_variant_order(pair_index, seed), 1):
                validated.append(
                    _validate_attempt(
                        attempts[cursor],
                        role,
                        variant,
                        pair_index,
                        position,
                        seed,
                        expected_sources[role],
                        expected_host_receipt,
                    )
                )
                cursor += 1

    role_summaries: dict[str, Any] = {}
    common_identity: dict[str, Any] | None = None
    all_resource_budgets_passed = True
    for role in ROLES:
        role_attempts = [attempt for attempt in validated if attempt["role"] == role]
        identities = [attempt["identity"] for attempt in role_attempts]
        if any(identity != identities[0] for identity in identities[1:]):
            raise ValueError(f"{role} workload identity drifted between variants")
        projected = {field: identities[0][field] for field in CROSS_ROLE_IDENTITY_FIELDS}
        if common_identity is None:
            common_identity = projected
        elif projected != common_identity:
            raise ValueError("I74 and C74 workload identities are not equivalent")

        pair_rows: list[dict[str, Any]] = []
        for pair_index in range(1, budgets["pairs"] + 1):
            pair = [
                attempt
                for attempt in role_attempts
                if attempt["pair_index"] == pair_index
            ]
            by_variant = {attempt["variant"]: attempt for attempt in pair}
            control = by_variant["control"]["metrics"]
            instrumented = by_variant["instrumented"]["metrics"]
            elapsed_seconds = instrumented["elapsed_ns"] / 1_000_000_000
            supervisor_cpu_percent = (
                instrumented["supervisor_cpu_ns"] * 100.0 / instrumented["elapsed_ns"]
            )
            control_plane_io_bytes = (
                instrumented["supervisor_io_bytes"]
                + instrumented["checkpoint_write_bytes"]
            )
            control_plane_io_bytes_per_second = control_plane_io_bytes / elapsed_seconds
            row = {
                "pair_index": pair_index,
                "order": list(_variant_order(pair_index, seed)),
                "elapsed_overhead_percent": _relative_percent(
                    control["elapsed_ns"], instrumented["elapsed_ns"]
                ),
                "role_cpu_overhead_percent": _relative_percent(
                    control["role_cpu_ns"], instrumented["role_cpu_ns"]
                ),
                "supervisor_cpu_percent": supervisor_cpu_percent,
                "supervisor_rss_peak_bytes": instrumented[
                    "supervisor_rss_peak_bytes"
                ],
                "checkpoint_write_bytes": instrumented["checkpoint_write_bytes"],
                "supervisor_io_bytes": instrumented["supervisor_io_bytes"],
                "control_plane_io_bytes_per_second": control_plane_io_bytes_per_second,
                "supervisor_cpu_budget_passed": supervisor_cpu_percent
                <= budgets["maximum_supervisor_cpu_percent"],
                "supervisor_rss_budget_passed": instrumented[
                    "supervisor_rss_peak_bytes"
                ]
                <= budgets["maximum_supervisor_rss_bytes"],
                "checkpoint_io_budget_passed": control_plane_io_bytes_per_second
                <= budgets["maximum_checkpoint_io_bytes_per_second"],
            }
            row["resource_budgets_passed"] = (
                row["supervisor_cpu_budget_passed"]
                and row["supervisor_rss_budget_passed"]
                and row["checkpoint_io_budget_passed"]
            )
            all_resource_budgets_passed &= bool(row["resource_budgets_passed"])
            pair_rows.append(row)
        role_summaries[role] = {
            "source_commit": expected_sources[role],
            "binary_sha256": identities[0]["binary_sha256"],
            "pairs": pair_rows,
            "median_elapsed_overhead_percent": statistics.median(
                row["elapsed_overhead_percent"] for row in pair_rows
            ),
            "median_role_cpu_overhead_percent": statistics.median(
                row["role_cpu_overhead_percent"] for row in pair_rows
            ),
            "resource_budgets_passed": all(
                row["resource_budgets_passed"] for row in pair_rows
            ),
        }

    elapsed_asymmetry = abs(
        role_summaries["i74"]["median_elapsed_overhead_percent"]
        - role_summaries["c74"]["median_elapsed_overhead_percent"]
    )
    cpu_asymmetry = abs(
        role_summaries["i74"]["median_role_cpu_overhead_percent"]
        - role_summaries["c74"]["median_role_cpu_overhead_percent"]
    )
    elapsed_asymmetry_passed = elapsed_asymmetry <= budgets["asymmetry_percent"]
    cpu_asymmetry_passed = cpu_asymmetry <= budgets["asymmetry_percent"]
    rehearsal_passed = (
        all_resource_budgets_passed
        and elapsed_asymmetry_passed
        and cpu_asymmetry_passed
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "release": "0.74",
        "evidence_class": EVIDENCE_CLASS,
        "promotable": False,
        "input_sha256": input_sha256,
        "order": ORDER,
        "pairs_per_role": budgets["pairs"],
        "seed": seed,
        "host_receipt_sha256": expected_host_receipt,
        "workload_identity": common_identity,
        "budgets": {key: value for key, value in budgets.items() if key != "pairs"},
        "roles": role_summaries,
        "asymmetry": {
            "elapsed_overhead_percentage_points": elapsed_asymmetry,
            "role_cpu_overhead_percentage_points": cpu_asymmetry,
            "elapsed_passed": elapsed_asymmetry_passed,
            "role_cpu_passed": cpu_asymmetry_passed,
        },
        "decision": {
            "resource_budgets_passed": all_resource_budgets_passed,
            "non_product_rehearsal_passed": rehearsal_passed,
            "role_overhead_qualification_complete": False,
            "release_admission_allowed": False,
            "reason": "Non-product attempt sets validate the frozen analyzer and collection shape but cannot qualify product I74/C74 role overhead.",
        },
    }


def _write_new(path: Path, value: dict[str, Any]) -> None:
    if path.exists():
        raise ValueError("output must not pre-exist")
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2, sort_keys=True, allow_nan=False)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--attempts", required=True, type=Path)
    parser.add_argument("--contract", required=True, type=Path)
    parser.add_argument("--statistics", required=True, type=Path)
    parser.add_argument("--expected-i74-source", required=True)
    parser.add_argument("--expected-c74-source", required=True)
    parser.add_argument("--expected-host-receipt", required=True)
    parser.add_argument("--output", required=True, type=Path)
    options = parser.parse_args()
    try:
        attempt_set, input_sha256 = _read_json(options.attempts)
        budgets = _load_budgets(options.contract, options.statistics)
        receipt = analyse(
            attempt_set,
            input_sha256,
            budgets,
            {"i74": options.expected_i74_source, "c74": options.expected_c74_source},
            options.expected_host_receipt,
        )
        _write_new(options.output, receipt)
    except (OSError, ValueError, tomllib.TOMLDecodeError) as error:
        raise SystemExit(str(error)) from error
    if not receipt["decision"]["non_product_rehearsal_passed"]:
        print("0.74 non-product role-overhead rehearsal: FAILED", file=sys.stderr)
        return 1
    print(f"0.74 non-product role-overhead rehearsal: OK ({options.output})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
