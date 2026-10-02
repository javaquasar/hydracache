#!/usr/bin/env python3
"""Validate and render the 0.74 release-qualification plan without executing it."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import tomllib
from typing import Any


FROZEN_PREDECESSOR = "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
PHASES = [
    "Q0-identity-and-contract",
    "Q1-build-and-semantic",
    "Q2-same-box-redis",
    "Q3-native-non-regression",
    "Q4-soak-and-retention",
    "Q5-finalize-evidence",
]


def digest(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_manifest(root: pathlib.Path, manifest: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    expected_scalars = {
        "schema_version": 1,
        "release": "0.74",
        "manifest_id": "release-qualification-074-v1",
        "state": "prepared-not-authorized",
        "expensive_runs_enabled": False,
        "explicit_authorization_required": True,
        "predecessor_source_sha": FROZEN_PREDECESSOR,
        "candidate_source_sha": "UNRESOLVED",
        "seed": 740074,
        "artifact_name_format": "hc074-{source12}-{host_id}-{run_id}",
        "same_box_redis_required": True,
        "redis_server_version": "7.2.5",
        "redis_benchmark_version": "7.2.5",
    }
    for field, expected in expected_scalars.items():
        if manifest.get(field) != expected:
            problems.append(f"{field} must remain {expected!r}")

    inputs = manifest.get("contract_inputs", [])
    if not isinstance(inputs, list) or len(inputs) < 4:
        problems.append("at least four hashed contract inputs are required")
    else:
        for item in inputs:
            path = root / str(item.get("path", ""))
            if not path.is_file():
                problems.append(f"missing contract input {path}")
            elif digest(path) != item.get("sha256"):
                problems.append(f"contract input digest mismatch for {item.get('path')}")

    phases = manifest.get("phases", [])
    ids = [phase.get("id") for phase in phases] if isinstance(phases, list) else []
    if ids != PHASES:
        problems.append("qualification phases are missing or reordered")
    for phase in phases if isinstance(phases, list) else []:
        command = str(phase.get("command", ""))
        if not command or "&&" in command or ";" in command:
            problems.append(f"phase {phase.get('id')} must contain one inspectable command")
        if phase.get("expensive") is True and phase.get("expected_before_authorization") != "not-run":
            problems.append(f"expensive phase {phase.get('id')} must remain not-run")

    unresolved = manifest.get("unresolved", {})
    required_unresolved = {
        "predecessor_annotated_tag",
        "predecessor_confirmation",
        "candidate_source",
        "admitted_host",
        "redis_binary_digests",
        "qualification_runner",
        "authorization",
    }
    if set(unresolved) != required_unresolved or not all(unresolved.values()):
        problems.append("all pre-authorization qualification blockers must remain explicit")
    if not re.fullmatch(r"hc074-\{source12\}-\{host_id\}-\{run_id\}", str(manifest.get("artifact_name_format"))):
        problems.append("artifact naming format drifted")
    return problems


def dry_run_receipt(manifest_path: pathlib.Path, manifest: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": 1,
        "release": "0.74",
        "mode": "dry-run",
        "promotable": False,
        "executed_commands": [],
        "manifest_sha256": digest(manifest_path),
        "phase_order": [phase["id"] for phase in manifest["phases"]],
        "expensive_phases": [
            phase["id"] for phase in manifest["phases"] if phase["expensive"]
        ],
        "unresolved": sorted(key for key, value in manifest["unresolved"].items() if value),
        "ready_to_execute": False,
        "reason": "0.73 publication, admitted host, exact candidate, runner identities and explicit authorization are unresolved",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path.cwd())
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--dry-run", action="store_true")
    options = parser.parse_args()
    if not options.dry_run:
        raise SystemExit("this pre-authorization tool supports --dry-run only")
    root = options.root.resolve()
    manifest_path = options.manifest if options.manifest.is_absolute() else root / options.manifest
    output = options.output if options.output.is_absolute() else root / options.output
    if output.exists():
        raise SystemExit("dry-run output already exists")
    manifest = tomllib.loads(manifest_path.read_text(encoding="utf-8"))
    problems = check_manifest(root, manifest)
    if problems:
        raise SystemExit("invalid qualification manifest:\n- " + "\n- ".join(problems))
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(
        json.dumps(dry_run_receipt(manifest_path, manifest), indent=2) + "\n",
        encoding="utf-8",
    )
    print(f"0.74 qualification dry-run: OK ({output}, no commands executed)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
