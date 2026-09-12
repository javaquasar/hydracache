#!/usr/bin/env python3
"""Measure and freeze the 0.71 memory-instrumentation overhead envelope."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import statistics
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


RELEASE = "0.71"
MODES = ("off", "production", "profile")
WORKLOADS: tuple[tuple[str, dict[str, Any], str], ...] = (
    ("cold", {}, "post_idle"),
    ("small-hot", {"keys": 10_000, "value_bytes": 64}, "steady"),
    (
        "tag-heavy",
        {
            "distribution": "high-fanout",
            "tags_per_entry": 16,
            "tag_pool": 16,
        },
        "steady",
    ),
    (
        "hc2-1000",
        {"connections": 1_000, "tls": True, "slow_consumers": 100},
        "steady",
    ),
    ("reset", {"cycles": 60}, "shutdown"),
)
CASE_IDS = {
    "cold": "M0-cold",
    "small-hot": "M1-shape",
    "tag-heavy": "M5-tags",
    "hc2-1000": "M6-connections",
    "reset": "M4-reset",
}


class OverheadError(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    with os.fdopen(os.open(temporary, flags, 0o600), "w", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    if path.exists():
        temporary.unlink()
        raise OverheadError(f"refusing to overwrite immutable receipt: {path}")
    os.replace(temporary, path)


def finite_number(value: Any, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise OverheadError(f"{label} must be numeric")
    result = float(value)
    if not math.isfinite(result):
        raise OverheadError(f"{label} must be finite")
    return result


def selected_sample(report: dict[str, Any], phase: str) -> dict[str, Any]:
    checkpoints = [item for item in report.get("checkpoints", []) if item.get("phase") == phase]
    if len(checkpoints) != 1:
        raise OverheadError(f"report must contain exactly one {phase} checkpoint")
    checkpoint = checkpoints[0]
    performance = checkpoint.get("performance", {})
    process = checkpoint.get("process", {})
    requests = int(performance.get("request_count", 0))
    cpu = finite_number(performance.get("cpu_seconds"), "cpu_seconds")
    return {
        "rss_bytes": int(process["vm_rss_bytes"]),
        "rps": finite_number(performance.get("rps"), "rps"),
        "p99_ns": int(performance["p99_ns"]),
        "cpu_seconds": cpu,
        "cpu_seconds_per_request": None if requests == 0 else cpu / requests,
        "context_switches": int(performance["context_switches"]),
        "request_count": requests,
        "errors": int(performance["errors"]),
    }


def median(values: list[float]) -> float:
    if not values:
        raise OverheadError("cannot summarize an empty sample set")
    return float(statistics.median(values))


def regression(candidate: float, baseline: float, *, lower_is_better: bool) -> float | None:
    if baseline == 0:
        return None
    return (candidate - baseline) / baseline if lower_is_better else (baseline - candidate) / baseline


def summarize(samples: list[dict[str, Any]], repetitions: int) -> tuple[list[dict[str, Any]], dict[str, Any]]:
    expected = len(WORKLOADS) * len(MODES) * repetitions
    if len(samples) != expected:
        raise OverheadError(f"expected {expected} samples, observed {len(samples)}")
    comparisons: list[dict[str, Any]] = []
    for workload, _dimensions, _phase in WORKLOADS:
        by_mode: dict[str, list[dict[str, Any]]] = {}
        for mode in MODES:
            selected = [item for item in samples if item["workload"] == workload and item["mode"] == mode]
            if len(selected) != repetitions or any(item["metrics"]["errors"] != 0 for item in selected):
                raise OverheadError(f"incomplete or failed {workload}/{mode} sample set")
            by_mode[mode] = selected
        medians: dict[str, dict[str, float | None]] = {}
        for mode, selected in by_mode.items():
            metrics = [item["metrics"] for item in selected]
            cpu_values = [item["cpu_seconds_per_request"] for item in metrics if item["cpu_seconds_per_request"] is not None]
            medians[mode] = {
                "rss_bytes": median([float(item["rss_bytes"]) for item in metrics]),
                "rps": median([item["rps"] for item in metrics]),
                "p99_ns": median([float(item["p99_ns"]) for item in metrics]),
                "cpu_seconds_per_request": median(cpu_values) if cpu_values else None,
                "context_switches": median([float(item["context_switches"]) for item in metrics]),
            }
        off = medians["off"]
        production = medians["production"]
        comparisons.append(
            {
                "workload": workload,
                "medians": medians,
                "production_vs_off": {
                    "rss_delta_bytes": production["rss_bytes"] - off["rss_bytes"],
                    "rss_regression_fraction": regression(float(production["rss_bytes"]), float(off["rss_bytes"]), lower_is_better=True),
                    "rps_regression_fraction": regression(float(production["rps"]), float(off["rps"]), lower_is_better=False),
                    "p99_regression_fraction": regression(float(production["p99_ns"]), float(off["p99_ns"]), lower_is_better=True),
                    "cpu_per_request_regression_fraction": (
                        None
                        if production["cpu_seconds_per_request"] is None or off["cpu_seconds_per_request"] is None
                        else regression(float(production["cpu_seconds_per_request"]), float(off["cpu_seconds_per_request"]), lower_is_better=True)
                    ),
                },
            }
        )
    fields = (
        "rss_delta_bytes",
        "rss_regression_fraction",
        "rps_regression_fraction",
        "p99_regression_fraction",
        "cpu_per_request_regression_fraction",
    )
    envelope = {
        field: max(
            (item["production_vs_off"][field] for item in comparisons if item["production_vs_off"][field] is not None),
            default=None,
        )
        for field in fields
    }
    return comparisons, envelope


def mode_order(repetition: int) -> tuple[str, ...]:
    offset = repetition % len(MODES)
    return MODES[offset:] + MODES[:offset]


def execute(args: argparse.Namespace) -> None:
    if args.repetitions < 3:
        raise OverheadError("evidence overhead measurement requires at least three repetitions")
    manifest = json.loads(args.build_manifest.read_text(encoding="utf-8"))
    host = json.loads(args.host_preflight.read_text(encoding="utf-8"))
    binary = Path(manifest["binary"])
    if not binary.is_file() or sha256(binary) != manifest["binary_sha256"]:
        raise OverheadError("B1 binary is missing or drifted")
    if host.get("result") != "success" or host.get("ship_evidence_eligible") is not True:
        raise OverheadError("host is not admitted for ship evidence")
    if args.work_root.exists():
        raise OverheadError(f"work root already exists: {args.work_root}")
    args.work_root.mkdir(parents=True)
    executor = Path(__file__).with_name("memory_case_executor_071.py")
    samples: list[dict[str, Any]] = []
    for repetition in range(args.repetitions):
        for mode in mode_order(repetition):
            for workload, dimensions, phase in WORKLOADS:
                output = args.work_root / f"r{repetition + 1}-{mode}-{workload}"
                job_path = output.with_suffix(".job.json")
                job = {
                    "job_id": f"s5-r{repetition + 1}-{mode}-{workload}",
                    "case_id": CASE_IDS[workload],
                    "cohort": "B1-instrumented",
                    "dimensions": dimensions,
                    "repetition": repetition + 1,
                }
                job_path.write_text(json.dumps(job, sort_keys=True) + "\n", encoding="utf-8")
                command = [
                    sys.executable,
                    str(executor),
                    "--job", str(job_path),
                    "--build-manifest", str(args.build_manifest),
                    "--output", str(output),
                    "--scenario-digest", args.scenario_digest,
                    "--host-preflight", str(args.host_preflight),
                    "--instrumentation-mode", mode,
                ]
                if workload == "hc2-1000":
                    if not args.hc2_helper_manifest:
                        raise OverheadError("HC/2 helper manifest is required")
                    command.extend(["--hc2-helper-manifest", str(args.hc2_helper_manifest)])
                completed = subprocess.run(command, check=False)
                if completed.returncode != 0:
                    raise OverheadError(f"executor failed for {job['job_id']} with {completed.returncode}")
                report_path = output / "memory-baseline-report.json"
                report = json.loads(report_path.read_text(encoding="utf-8"))
                if (
                    report.get("source_sha") != manifest.get("source_sha")
                    or report.get("binary_sha256") != manifest.get("binary_sha256")
                    or report.get("host_fingerprint") != host.get("host_fingerprint")
                    or report.get("instrumentation_mode") != mode
                    or report.get("ship_evidence_eligible") is not True
                ):
                    raise OverheadError(f"identity mismatch in {job['job_id']}")
                samples.append(
                    {
                        "workload": workload,
                        "mode": mode,
                        "repetition": repetition + 1,
                        "report": str(report_path),
                        "report_sha256": sha256(report_path),
                        "metrics": selected_sample(report, phase),
                    }
                )
    comparisons, envelope = summarize(samples, args.repetitions)
    atomic_json(
        args.output,
        {
            "schema_version": 1,
            "release": RELEASE,
            "source_sha": manifest["source_sha"],
            "binary_sha256": manifest["binary_sha256"],
            "host_fingerprint": host["host_fingerprint"],
            "scenario_digest": args.scenario_digest,
            "measurement_design": {
                "modes": list(MODES),
                "workloads": [item[0] for item in WORKLOADS],
                "repetitions": args.repetitions,
                "mode_order": "cyclic-alternating",
                "candidate_data_used": False,
            },
            "samples": samples,
            "comparisons": comparisons,
            "frozen_envelope": envelope,
            "passed": True,
            "ship_evidence_eligible": True,
            "created_at": datetime.now(timezone.utc).isoformat(),
        },
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--build-manifest", type=Path, required=True)
    parser.add_argument("--host-preflight", type=Path, required=True)
    parser.add_argument("--hc2-helper-manifest", type=Path, required=True)
    parser.add_argument("--scenario-digest", required=True)
    parser.add_argument("--work-root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--repetitions", type=int, default=3)
    return parser.parse_args()


def main() -> int:
    try:
        execute(parse_args())
        return 0
    except (OverheadError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"memory instrumentation overhead 0.71: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
