#!/usr/bin/env python3
"""Run the complete non-promotable 0.71 Linux/Docker rehearsal."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


class RehearsalError(RuntimeError):
    pass


def run(command: list[str], root: Path) -> None:
    completed = subprocess.run(command, cwd=root, check=False)
    if completed.returncode != 0:
        raise RehearsalError(f"command failed ({completed.returncode}): {' '.join(command)}")


def output(command: list[str], root: Path) -> str:
    return subprocess.run(command, cwd=root, check=True, capture_output=True, text=True).stdout.strip()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def execute(args: argparse.Namespace) -> None:
    root = Path(output(["git", "rev-parse", "--show-toplevel"], Path.cwd())).resolve()
    evidence = args.output.resolve()
    if evidence.exists() and any(evidence.iterdir()):
        raise RehearsalError(f"output directory must be empty: {evidence}")
    evidence.mkdir(parents=True, exist_ok=True)
    commands = [
        [sys.executable, "scripts/perf/memory_campaign_071_test.py"],
        [sys.executable, "scripts/perf/memory_case_executor_071_test.py"],
        [sys.executable, "scripts/perf/memory_historical_mirror_071_test.py"],
        [
            "cargo",
            "test",
            "-p",
            "xtask",
            "--test",
            "memory_baseline_071",
            "--locked",
            "--",
            "--skip",
            "b0_b1_are_distinct_cohorts",
        ],
        ["cargo", "build", "--release", "--locked", "-p", "hydracache-server"],
        [
            "cargo",
            "build",
            "--release",
            "--locked",
            "-p",
            "hydracache-loadgen",
            "--bin",
            "memory-hc2-connections-071",
        ],
    ]
    for command in commands:
        run(command, root)

    binary = root / "target/release/hydracache-server"
    manifest = {
        "schema_version": 1,
        "release": "0.71",
        "cohort": "B1-instrumented",
        "source_sha": output(["git", "rev-parse", "HEAD"], root),
        "binary": str(binary),
        "binary_sha256": sha256(binary),
        "build_profile": "release",
        "features": [],
        "allocator": "system",
    }
    jobs = [
        {
            "job_id": "docker-real-daemon-m3-ttl-r1",
            "case_id": "M3-ttl",
            "cell_id": "M3-ttl__cycles-60",
            "dimensions": {"cycles": 60},
            "cohort": "B1-instrumented",
            "repetition": 1,
        },
        {
            "job_id": "docker-real-daemon-m5-high-fanout-r1",
            "case_id": "M5-tags",
            "cell_id": "M5-tags__distribution-high-fanout__tag_pool-16__tags_per_entry-16",
            "dimensions": {
                "distribution": "high-fanout",
                "tag_pool": 16,
                "tags_per_entry": 16,
            },
            "cohort": "B1-instrumented",
            "repetition": 1,
        },
        {
            "job_id": "docker-real-daemon-m6-slow-consumers-r1",
            "case_id": "M6-connections",
            "cell_id": "M6-connections__connections-100__slow_consumers-100__tls-true",
            "dimensions": {"connections": 100, "slow_consumers": 100, "tls": True},
            "cohort": "B1-instrumented",
            "repetition": 1,
        },
        {
            "job_id": "docker-real-daemon-m7-supported-r1",
            "case_id": "M7-persistence",
            "cell_id": "M7-persistence__persistence-supported",
            "dimensions": {"persistence": "supported"},
            "cohort": "B1-instrumented",
            "repetition": 1,
        },
    ]
    for sequence in ("fixed-keyspace", "ttl", "reset", "hc2-churn"):
        jobs.append(
            {
                "job_id": f"docker-real-daemon-m8-{sequence}-r1",
                "case_id": "M8-60m",
                "cell_id": f"M8-60m__sequence-{sequence}",
                "dimensions": {
                    "sequence": sequence,
                    "duration_seconds": 3600,
                    "iteration_seconds": 60,
                    "heartbeat_seconds": 300,
                    "hc2_churn_connections": 100,
                    "rehearsal_duration_seconds": 2.0,
                    "rehearsal_iteration_seconds": 0.25,
                    "rehearsal_heartbeat_seconds": 0.5,
                },
                "cohort": "B1-instrumented",
                "repetition": 1,
            }
        )
    manifest_path = evidence / "real-daemon" / "build-manifest.json"
    write_json(manifest_path, manifest)
    helper = root / "target/release/memory-hc2-connections-071"
    helper_manifest_path = evidence / "real-daemon" / "hc2-helper-manifest.json"
    write_json(
        helper_manifest_path,
        {
            "schema_version": 1,
            "release": "0.71",
            "source_sha": manifest["source_sha"],
            "binary": str(helper),
            "binary_sha256": sha256(helper),
        },
    )
    scenario = root / "docs/testing/perf-scenarios/0.71/memory-efficiency-v1.toml"
    reports: list[Path] = []
    for job in jobs:
        case_label = (
            f"M8-60m-{job['dimensions']['sequence']}"
            if job["case_id"] == "M8-60m"
            else job["case_id"]
        )
        case_root = evidence / "real-daemon" / case_label
        job_path = case_root / "job.json"
        write_json(job_path, job)
        executor_command = [
                sys.executable,
                "scripts/perf/memory_case_executor_071.py",
                "--job",
                str(job_path),
                "--build-manifest",
                str(manifest_path),
                "--output",
                str(case_root / "run"),
                "--scenario-digest",
                sha256(scenario),
                "--provider",
                "system",
                "--rehearsal",
            ]
        if job["case_id"] == "M6-connections" or (
            job["case_id"] == "M8-60m" and job["dimensions"]["sequence"] == "hc2-churn"
        ):
            executor_command.extend(["--hc2-helper-manifest", str(helper_manifest_path)])
        run(executor_command, root)
        report = case_root / "run" / "memory-baseline-report.json"
        reports.append(report)
        run(
            [
                "cargo",
                "run",
                "-p",
                "xtask",
                "--locked",
                "--",
                "memory-baseline-report-check",
                "--release",
                "0.71",
                "--report",
                str(report),
                "--allow-diagnostic-source",
            ],
            root,
        )
    m5_receipt = evidence / "real-daemon" / "M5-tags" / "run" / "m5-distribution-receipt.json"
    if not m5_receipt.is_file():
        raise RehearsalError("real M5 run did not produce its distribution receipt")
    m6_receipt = evidence / "real-daemon" / "M6-connections" / "run" / "m6-connections-receipt.json"
    if not m6_receipt.is_file():
        raise RehearsalError("real M6 run did not produce its connection receipt")
    m7_receipt = evidence / "real-daemon" / "M7-persistence" / "run" / "m7-persistence-receipt.json"
    if not m7_receipt.is_file():
        raise RehearsalError("real M7 run did not produce its persistence receipt")
    for sequence in ("fixed-keyspace", "ttl", "reset", "hc2-churn"):
        receipt = evidence / "real-daemon" / f"M8-60m-{sequence}" / "run" / "m8-duration-receipt.json"
        if not receipt.is_file():
            raise RehearsalError(f"real M8 {sequence} run did not produce its duration receipt")

    campaign_id = "docker-full-matrix"
    campaign_root = evidence / "campaigns"
    controller = [sys.executable, "scripts/perf/memory_campaign_071.py", "--output-root", str(campaign_root)]
    run(controller + ["plan", "--campaign-id", campaign_id, "--repetitions", "1", "--rehearsal"], root)
    run(controller + ["run", "--campaign-id", campaign_id], root)
    run(controller + ["resume", "--campaign-id", campaign_id], root)
    run(controller + ["finalize", "--campaign-id", campaign_id], root)
    campaign_state = json.loads((campaign_root / campaign_id / "state.json").read_text(encoding="utf-8"))
    for report in reports:
        typed_report = json.loads(report.read_text(encoding="utf-8"))
        if typed_report.get("ship_evidence_eligible") is not False or typed_report.get("diagnostic_only") is not True:
            raise RehearsalError("Docker typed report incorrectly became promotable")
    receipt = {
        "schema_version": 1,
        "release": "0.71",
        "kind": "linux-docker-rehearsal",
        "source_sha": manifest["source_sha"],
        "platform": platform.platform(),
        "cgroup_v2": Path("/sys/fs/cgroup/cgroup.controllers").is_file(),
        "real_daemon_cells": [job["cell_id"] for job in jobs],
        "real_daemon_report_sha256": [sha256(report) for report in reports],
        "matrix_jobs": campaign_state["job_count"],
        "matrix_status": campaign_state["status"],
        "completed_at": datetime.now(timezone.utc).isoformat(),
        "diagnostic_only": True,
        "ship_evidence_eligible": False,
    }
    write_json(evidence / "docker-rehearsal-receipt.json", receipt)
    print(json.dumps(receipt, indent=2, sort_keys=True))


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, required=True)
    return parser.parse_args()


def main() -> int:
    try:
        execute(parse_args())
        return 0
    except (RehearsalError, OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"memory Docker rehearsal 0.71: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
