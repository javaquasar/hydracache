#!/usr/bin/env python3
"""Run the preregistered 0.73 integrated smoke without performance claims."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time
import tomllib


ROOT = Path(__file__).resolve().parents[2]
CONTRACT = ROOT / "docs/testing/performance/0.73/w10-integrated-process-smoke-contract.toml"
SCENARIO = ROOT / "docs/testing/performance/0.73/w10-integrated-smoke-scenario.toml"
CELLS = (
    "event_delivery",
    "expiry_tag_accounting",
    "mixed_protocol",
    "durable_companion",
)
EXPECTED_CELL_IDS = tuple(name.replace("_", "-") for name in CELLS)
RECEIPT_PREFIX = '{"schema_version":1,"cell":'
CANARY_MARKER = "HC-CANARY-RED:W10"


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def read_toml(path: Path) -> dict:
    with path.open("rb") as handle:
        return tomllib.load(handle)


def check_contract() -> tuple[dict, dict]:
    contract = read_toml(CONTRACT)
    scenario = read_toml(SCENARIO)
    if contract["state"] != "preregistered-before-smoke-implementation":
        raise RuntimeError("W10 smoke contract is not preregistered")
    if contract["host_dispatch_allowed"] or contract["local_results_promotable"]:
        raise RuntimeError("local smoke cannot open promotion or host dispatch")
    if contract["cell_ids"] != list(EXPECTED_CELL_IDS):
        raise RuntimeError("runner cells differ from the preregistered matrix")
    if contract["required_processes"] != len(CELLS):
        raise RuntimeError("runner process count differs from the contract")
    if scenario["operations"] != 1_000 or scenario["payload_bytes"] != 4_096:
        raise RuntimeError("scenario volume changed")
    weights = scenario["mixed_weights_percent"]
    if [weights[key] for key in weights] != [35, 30, 15, 10, 5, 5]:
        raise RuntimeError("mixed weights changed")
    return contract, scenario


def build_test_binary() -> Path:
    command = [
        "cargo",
        "test",
        "-p",
        "hydracache-server",
        "--test",
        "performance_integrated_073",
        "--no-run",
        "--locked",
        "--message-format=json",
    ]
    result = subprocess.run(command, cwd=ROOT, capture_output=True, check=False)
    if result.returncode != 0:
        sys.stdout.buffer.write(result.stdout)
        sys.stderr.buffer.write(result.stderr)
        raise RuntimeError("failed to build the W10 integrated smoke binary")
    executable: Path | None = None
    for raw_line in result.stdout.splitlines():
        try:
            message = json.loads(raw_line)
        except json.JSONDecodeError:
            continue
        target = message.get("target", {})
        if (
            message.get("reason") == "compiler-artifact"
            and target.get("name") == "performance_integrated_073"
            and message.get("executable")
        ):
            executable = Path(message["executable"])
    if executable is None or not executable.is_file():
        raise RuntimeError("Cargo did not report the prebuilt W10 test binary")
    return executable.resolve()


def git_value(*args: str) -> str:
    return subprocess.run(
        ["git", *args],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()


def receipt_from_stdout(stdout: bytes, expected_cell: str) -> dict:
    receipts = []
    for raw_line in stdout.decode("utf-8", errors="replace").splitlines():
        marker = raw_line.find(RECEIPT_PREFIX)
        if marker >= 0:
            receipts.append(json.loads(raw_line[marker:]))
    if len(receipts) != 1:
        raise RuntimeError(f"{expected_cell}: expected one receipt, got {len(receipts)}")
    receipt = receipts[0]
    if receipt["cell"] != expected_cell:
        raise RuntimeError(f"{expected_cell}: receipt names {receipt['cell']}")
    if receipt["attempted"] != 1_000 or receipt["success"] != 1_000:
        raise RuntimeError(f"{expected_cell}: incomplete scheduled outcomes")
    for field in ("rejected", "timeout", "late", "incomplete"):
        if receipt[field] != 0:
            raise RuntimeError(f"{expected_cell}: nonzero {field}")
    return receipt


def run_process(binary: Path, test_name: str, env: dict[str, str]) -> subprocess.CompletedProcess[bytes]:
    return subprocess.run(
        [str(binary), "--exact", test_name, "--nocapture", "--test-threads=1"],
        cwd=ROOT,
        env=env,
        capture_output=True,
        check=False,
    )


def write_streams(output_dir: Path, stem: str, result: subprocess.CompletedProcess[bytes]) -> dict:
    stdout_path = output_dir / f"{stem}.stdout.log"
    stderr_path = output_dir / f"{stem}.stderr.log"
    stdout_path.write_bytes(result.stdout)
    stderr_path.write_bytes(result.stderr)
    return {
        "stdout": stdout_path.relative_to(ROOT).as_posix(),
        "stdout_sha256": sha256_bytes(result.stdout),
        "stderr": stderr_path.relative_to(ROOT).as_posix(),
        "stderr_sha256": sha256_bytes(result.stderr),
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output-dir",
        type=Path,
        help="artifact directory (default: target/performance-evidence/0.73/W10-<UTC>)",
    )
    args = parser.parse_args()
    contract, scenario = check_contract()
    timestamp = time.strftime("%Y%m%dT%H%M%SZ", time.gmtime())
    output_dir = (args.output_dir or ROOT / "target/performance-evidence/0.73" / f"W10-{timestamp}").resolve()
    output_dir.mkdir(parents=True, exist_ok=False)
    binary = build_test_binary()
    base_env = os.environ.copy()
    base_env.pop("HYDRACACHE_CANARY_DEFECT", None)
    outcomes = []
    for test_name, cell_id in zip(CELLS, EXPECTED_CELL_IDS, strict=True):
        result = run_process(binary, test_name, base_env)
        streams = write_streams(output_dir, cell_id, result)
        if result.returncode != 0:
            raise RuntimeError(f"{cell_id}: process failed; see {streams}")
        receipt = receipt_from_stdout(result.stdout, cell_id)
        outcomes.append(
            {
                "cell": cell_id,
                "exit_code": result.returncode,
                "receipt": receipt,
                **streams,
            }
        )

    canary_env = base_env.copy()
    canary_env["HYDRACACHE_CANARY_DEFECT"] = "W10"
    canary = run_process(binary, "expiry_tag_accounting", canary_env)
    canary_streams = write_streams(output_dir, "canary", canary)
    combined_canary = (canary.stdout + canary.stderr).decode("utf-8", errors="replace")
    if canary.returncode == 0 or CANARY_MARKER not in combined_canary:
        raise RuntimeError("W10 canary did not fail red with the required marker")

    manifest = {
        "schema_version": 1,
        "release": "0.73",
        "profile_id": contract["profile_id"],
        "scenario_id": scenario["scenario_id"],
        "source_commit": git_value("rev-parse", "HEAD"),
        "source_tree": git_value("write-tree"),
        "working_tree_dirty": bool(git_value("status", "--short")),
        "binary": binary.relative_to(ROOT).as_posix(),
        "binary_sha256": sha256_bytes(binary.read_bytes()),
        "processes": outcomes,
        "canary": {
            "test": "expiry_tag_accounting",
            "exit_code": canary.returncode,
            "required_marker": CANARY_MARKER,
            "marker_observed": True,
            **canary_streams,
        },
        "performance_claim": None,
        "host_dispatch_authorized": False,
        "promotion_authorized": False,
    }
    manifest_path = output_dir / "manifest.json"
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
    receipt = {
        "manifest": manifest_path.relative_to(ROOT).as_posix(),
        "manifest_sha256": sha256_bytes(manifest_path.read_bytes()),
        "cells_passed": len(outcomes),
        "canary_red": True,
        "host_dispatch_authorized": False,
    }
    print(json.dumps(receipt, separators=(",", ":")))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
