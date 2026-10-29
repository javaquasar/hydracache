#!/usr/bin/env python3
"""Fail-closed published-0.72 compatibility orchestrator for release 0.73."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import tomllib
from typing import Any


PROFILE_ID = "published-072-compatibility-073-v1"
CANARY_MARKER = "HC-CANARY-RED:W10-COMPAT"
WIRE_ROLES = (
    "B72-client--B72-server",
    "B72-client--C73-server",
    "C73-client--B72-server",
    "C73-client--C73-server",
)
REQUIRED_COMMON_CASES = {
    "hc1-empty-read",
    "hc1-put-read-structured-key-and-binary-value",
    "hc1-tenant-isolation",
    "hc1-ttl-expiry",
    "hc1-malformed-frame-rejected",
    "hc2-empty-read",
    "hc2-put-read-binary-key-and-value",
    "hc2-tenant-isolation",
    "hc2-ttl-expiry",
    "hc2-malformed-input-rejected-before-mutation",
    "resp-empty-read",
    "resp-put-read-binary-key-and-value",
    "resp-tagged-put-read-invalidate",
    "resp-ttl-expiry",
    "resp-malformed-frame-rejected",
    "management-routes-schema-v1",
    "console-index-hashed-assets-and-404",
    "drain-reconciles-live-owners",
}
REQUIRED_DURABLE_CASES = {
    "b72-create-live-and-tombstone",
    "b72-flush-and-reopen",
    "c73-read-old-write-new-and-reopen",
    "repair-confirmed-tombstone-gc",
    "budget-rejection-before-mutation",
    "checksum-corruption-loud-refusal-and-restore",
    "b72-read-old-and-candidate-writes",
    "b72-same-disk-reopen-after-candidate",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mode", choices=("canary", "campaign"), required=True)
    parser.add_argument("--b72-harness", type=pathlib.Path, required=True)
    parser.add_argument("--c73-harness", type=pathlib.Path, required=True)
    parser.add_argument("--b72-server", type=pathlib.Path, required=True)
    parser.add_argument("--c73-server", type=pathlib.Path, required=True)
    parser.add_argument("--b72-root", type=pathlib.Path, required=True)
    parser.add_argument("--c73-root", type=pathlib.Path, required=True)
    parser.add_argument("--scenario", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    return parser.parse_args()


def sha256(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(root), *args], text=True, encoding="utf-8"
    ).strip()


def read_json(path: pathlib.Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8"))


def write_json(path: pathlib.Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )


def require_fresh_output(path: pathlib.Path) -> None:
    if path.exists():
        raise RuntimeError(f"output path already exists; retries must be retained: {path}")
    path.mkdir(parents=True)


def validate_identity(args: argparse.Namespace, scenario: dict[str, Any]) -> dict[str, str]:
    expected_b72 = scenario["published_source_commit"]
    expected_c73 = scenario["candidate_source_commit"]
    expected_tree = scenario["candidate_tree_oid"]
    actual_b72 = git(args.b72_root, "rev-parse", "HEAD")
    actual_c73 = git(args.c73_root, "rev-parse", "HEAD")
    actual_tree = git(args.c73_root, "rev-parse", "HEAD^{tree}")
    tag_commit = git(
        args.b72_root, "rev-parse", f"{scenario['published_tag']}^{{commit}}"
    )
    tag_object = git(args.b72_root, "rev-parse", scenario["published_tag"])
    expected = (expected_b72, expected_c73, expected_tree, scenario["published_tag_object"])
    actual = (actual_b72, actual_c73, actual_tree, tag_object)
    if actual != expected or tag_commit != expected_b72:
        raise RuntimeError(
            "exact source identity mismatch: "
            f"expected={expected}, actual={actual}, tag_commit={tag_commit}"
        )
    if git(args.b72_root, "status", "--porcelain=v1", "--untracked-files=normal"):
        raise RuntimeError("B72 source root is dirty")
    if git(args.c73_root, "status", "--porcelain=v1", "--untracked-files=normal"):
        raise RuntimeError("C73 source root is dirty")
    binaries = {
        "b72_harness": args.b72_harness,
        "c73_harness": args.c73_harness,
        "b72_server": args.b72_server,
        "c73_server": args.c73_server,
    }
    for label, path in binaries.items():
        if not path.is_file():
            raise RuntimeError(f"{label} is not a file: {path}")
    return {label: sha256(path) for label, path in binaries.items()}


def run_harness(
    harness: pathlib.Path,
    *,
    mode: str,
    role: str,
    source_sha: str,
    output: pathlib.Path,
    server: pathlib.Path | None = None,
    store: pathlib.Path | None = None,
) -> dict[str, Any]:
    command = [
        str(harness),
        "--mode",
        mode,
        "--role",
        role,
        "--source-sha",
        source_sha,
        "--output",
        str(output),
    ]
    if server is not None:
        command.extend(("--server-binary", str(server)))
    if store is not None:
        command.extend(("--store", str(store)))
    completed = subprocess.run(command, text=True, encoding="utf-8", capture_output=True)
    log = output.with_suffix(".log")
    log.parent.mkdir(parents=True, exist_ok=True)
    log.write_text(
        f"exit_code={completed.returncode}\nstdout:\n{completed.stdout}\nstderr:\n{completed.stderr}",
        encoding="utf-8",
    )
    if completed.returncode != 0 or not output.is_file():
        raise RuntimeError(
            f"{role}/{mode} failed with exit {completed.returncode}; see {log}"
        )
    receipt = read_json(output)
    if (
        receipt.get("profile_id") != PROFILE_ID
        or receipt.get("mode") != mode
        or receipt.get("role") != role
        or receipt.get("source_sha") != source_sha
        or receipt.get("result") != "passed"
    ):
        raise RuntimeError(f"invalid receipt identity for {role}/{mode}: {receipt}")
    return receipt


def validate_wire(receipts: list[dict[str, Any]]) -> None:
    roles = [receipt.get("role") for receipt in receipts]
    if tuple(roles) != WIRE_ROLES:
        raise RuntimeError(
            f"{CANARY_MARKER}: exact wire matrix missing or reordered; got {roles}"
        )
    for receipt in receipts:
        cases = set(receipt.get("cases", ()))
        if cases != REQUIRED_COMMON_CASES:
            missing = sorted(REQUIRED_COMMON_CASES - cases)
            extra = sorted(cases - REQUIRED_COMMON_CASES)
            raise RuntimeError(
                f"wire case coverage mismatch for {receipt['role']}: missing={missing}, extra={extra}"
            )
        if receipt.get("mutation_count") != 8:
            raise RuntimeError(f"wire mutation accounting mismatch for {receipt['role']}")


def validate_durable(receipts: list[dict[str, Any]]) -> None:
    modes = tuple(receipt.get("mode") for receipt in receipts)
    if modes != ("durable-write", "durable-transition", "durable-rollback"):
        raise RuntimeError(f"durable transition order mismatch: {modes}")
    cases = {case for receipt in receipts for case in receipt.get("cases", ())}
    if cases != REQUIRED_DURABLE_CASES:
        raise RuntimeError(
            "durable coverage mismatch: "
            f"missing={sorted(REQUIRED_DURABLE_CASES - cases)}, "
            f"extra={sorted(cases - REQUIRED_DURABLE_CASES)}"
        )
    if [receipt.get("mutation_count") for receipt in receipts] != [2, 1, 0]:
        raise RuntimeError("durable mutation accounting mismatch")


def canary(args: argparse.Namespace, scenario: dict[str, Any]) -> None:
    require_fresh_output(args.output)
    marker_observed = False
    message = ""
    defective = [
        {"role": role, "cases": sorted(REQUIRED_COMMON_CASES)}
        for role in WIRE_ROLES
        if role != "C73-client--B72-server"
    ]
    try:
        validate_wire(defective)
    except RuntimeError as error:
        message = str(error)
        marker_observed = CANARY_MARKER in message
    pass_receipt = args.output / "compatibility-campaign.json"
    receipt_absent = not pass_receipt.exists()
    if not marker_observed or not receipt_absent:
        raise RuntimeError("compatibility matrix canary did not fail closed")
    write_json(
        args.output / "canary.json",
        {
            "schema_version": 1,
            "release": "0.73",
            "profile_id": PROFILE_ID,
            "contract_id": scenario["contract_id"],
            "defect": "removed-C73-client--B72-server",
            "marker": CANARY_MARKER,
            "marker_observed": marker_observed,
            "receipt_absent": receipt_absent,
            "failure": message,
            "result": "passed",
        },
    )


def campaign(args: argparse.Namespace, scenario: dict[str, Any]) -> None:
    require_fresh_output(args.output)
    binary_sha256 = validate_identity(args, scenario)
    b72_sha = scenario["published_source_commit"]
    c73_sha = scenario["candidate_source_commit"]
    cells = (
        (WIRE_ROLES[0], args.b72_harness, args.b72_server, b72_sha, b72_sha),
        (WIRE_ROLES[1], args.b72_harness, args.c73_server, b72_sha, c73_sha),
        (WIRE_ROLES[2], args.c73_harness, args.b72_server, c73_sha, b72_sha),
        (WIRE_ROLES[3], args.c73_harness, args.c73_server, c73_sha, c73_sha),
    )
    if os.environ.get("HYDRACACHE_CANARY_DEFECT") == "W10-COMPAT":
        cells = tuple(cell for cell in cells if cell[0] != "C73-client--B72-server")
    wire_receipts: list[dict[str, Any]] = []
    wire_artifacts: list[dict[str, str]] = []
    for role, harness, server, client_sha, server_sha in cells:
        path = args.output / "wire" / f"{role}.json"
        receipt = run_harness(
            harness,
            mode="wire",
            role=role,
            source_sha=client_sha,
            output=path,
            server=server,
        )
        wire_receipts.append(receipt)
        wire_artifacts.append(
            {
                "role": role,
                "client_source_sha": client_sha,
                "server_source_sha": server_sha,
                "receipt_sha256": sha256(path),
            }
        )
    validate_wire(wire_receipts)

    store = args.output / "durable-store"
    durable_specs = (
        ("durable-write", "B72", args.b72_harness, b72_sha),
        ("durable-transition", "C73", args.c73_harness, c73_sha),
        ("durable-rollback", "B72", args.b72_harness, b72_sha),
    )
    durable_receipts: list[dict[str, Any]] = []
    durable_artifacts: list[dict[str, str]] = []
    for mode, role, harness, source_sha in durable_specs:
        path = args.output / "durable" / f"{mode}.json"
        receipt = run_harness(
            harness,
            mode=mode,
            role=role,
            source_sha=source_sha,
            output=path,
            store=store,
        )
        durable_receipts.append(receipt)
        durable_artifacts.append(
            {
                "mode": mode,
                "role": role,
                "source_sha": source_sha,
                "receipt_sha256": sha256(path),
            }
        )
    validate_durable(durable_receipts)
    write_json(
        args.output / "compatibility-campaign.json",
        {
            "schema_version": 1,
            "release": "0.73",
            "profile_id": PROFILE_ID,
            "contract_id": scenario["contract_id"],
            "published_tag": scenario["published_tag"],
            "published_source_commit": b72_sha,
            "candidate_source_commit": c73_sha,
            "candidate_tree_oid": scenario["candidate_tree_oid"],
            "binary_sha256": binary_sha256,
            "wire_cells": wire_artifacts,
            "durable_transitions": durable_artifacts,
            "wire_cells_passed": len(wire_receipts),
            "durable_transitions_passed": len(durable_receipts),
            "rolling_scenarios_passed": 0,
            "rolling_complete": False,
            "long_run_allowed": False,
            "host_performance_claim_allowed": False,
            "result": "passed-local-wire-and-durable-only",
        },
    )


def main() -> int:
    args = parse_args()
    scenario = tomllib.loads(args.scenario.read_text(encoding="utf-8"))
    if scenario.get("profile_id", PROFILE_ID) != PROFILE_ID:
        raise RuntimeError("unexpected compatibility profile")
    if args.mode == "canary":
        canary(args, scenario)
    else:
        campaign(args, scenario)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # noqa: BLE001 - top-level fail-closed boundary
        print(error, file=sys.stderr)
        raise SystemExit(1)
