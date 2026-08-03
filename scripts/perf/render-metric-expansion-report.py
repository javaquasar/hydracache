#!/usr/bin/env python3
"""Render the reproducible Stage 3 metric-expansion evidence bundle.

The renderer intentionally keeps missing values visible as ``N/A``.  It never
interprets RSS as JVM heap and does not turn a failed workload into a passing
case merely because telemetry was collected.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
import statistics
from pathlib import Path
from typing import Any


def percentile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    ordered = sorted(values)
    position = (len(ordered) - 1) * fraction
    low, high = math.floor(position), math.ceil(position)
    if low == high:
        return ordered[low]
    return ordered[low] + (ordered[high] - ordered[low]) * (position - low)


def number(value: Any) -> float | None:
    return float(value) if isinstance(value, (int, float)) and not isinstance(value, bool) else None


def fmt(value: Any, digits: int = 2) -> str:
    if value is None:
        return "N/A"
    if isinstance(value, bool):
        return "yes" if value else "no"
    if isinstance(value, (int, float)):
        return f"{value:.{digits}f}"
    return str(value)


def load_jsonl(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    if not path.exists():
        return rows
    for line in path.read_text(encoding="utf-8", errors="replace").splitlines():
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(value, dict):
            rows.append(value)
    return rows


def metric_stats(rows: list[dict[str, Any]], field: str) -> dict[str, Any]:
    values = [value for row in rows if (value := number(row.get(field))) is not None]
    return {
        "samples": len(values),
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "max": max(values) if values else None,
        "min": min(values) if values else None,
    }


def slope_per_minute(rows: list[dict[str, Any]], field: str) -> float | None:
    points = [
        (number(row.get("timestamp_unix")), number(row.get(field)))
        for row in rows
    ]
    points = [(x, y) for x, y in points if x is not None and y is not None]
    if len(points) < 2:
        return None
    x0, y0 = points[0]
    x1, y1 = points[-1]
    if x1 == x0:
        return None
    return (y1 - y0) / (x1 - x0) * 60.0


def workload_stats(rows: list[dict[str, Any]]) -> dict[str, Any]:
    latency: list[float] = []
    throughputs: list[float] = []
    errors = 0
    requests = 0
    for row in rows:
        value = number(row.get("p50_latency_ms"))
        if value is not None:
            latency.append(value)
        value = number(row.get("throughput_ops_per_second"))
        if value is not None:
            throughputs.append(value)
        errors += int(number(row.get("errors")) or 0)
        requests += int(number(row.get("requests")) or 0)
    return {
        "records": len(rows),
        "requests": requests,
        "errors": errors,
        "error_rate": errors / requests if requests else None,
        "p50_latency_ms": percentile(latency, 0.50),
        "p95_latency_ms": percentile(latency, 0.95),
        "max_latency_ms": max(latency) if latency else None,
        "p50_throughput_ops_per_second": percentile(throughputs, 0.50),
        "p95_throughput_ops_per_second": percentile(throughputs, 0.95),
    }


def read_status(root: Path) -> list[dict[str, str]]:
    path = root / "case-status.tsv"
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as stream:
        return list(csv.DictReader(stream, delimiter="\t"))


def summarize_case(root: Path, row: dict[str, str]) -> dict[str, Any]:
    case_root = root / "metric-experiments" / row["experiment"] / row["target"] / row["case"]
    telemetry = load_jsonl(case_root / "telemetry" / "telemetry.jsonl")
    workload = load_jsonl(case_root / "raw" / "workload.jsonl")
    key_metrics = {
        "cpu_percent": metric_stats(telemetry, "container_cpu_percent"),
        "process_cpu_percent": metric_stats(telemetry, "process_cpu_percent"),
        "rss_bytes": metric_stats(telemetry, "vmrss_bytes"),
        "hwm_bytes": metric_stats(telemetry, "vmhwm_bytes"),
        "cgroup_current_bytes": metric_stats(telemetry, "cgroup_memory_current_bytes"),
        "cgroup_peak_bytes": metric_stats(telemetry, "cgroup_memory_peak_bytes"),
        "heap_used_bytes": metric_stats(telemetry, "jvm_heap_used_bytes"),
        "heap_max_bytes": metric_stats(telemetry, "jvm_heap_max_bytes"),
        "threads": metric_stats(telemetry, "process_threads"),
        "fd_count": metric_stats(telemetry, "process_fd_count"),
        "minor_faults": metric_stats(telemetry, "process_minor_faults"),
        "major_faults": metric_stats(telemetry, "process_major_faults"),
        "read_bytes": metric_stats(telemetry, "process_read_bytes"),
        "write_bytes": metric_stats(telemetry, "process_write_bytes"),
        "context_switches": metric_stats(telemetry, "voluntary_context_switches"),
        "oom_kills": metric_stats(telemetry, "cgroup_memory_oom_kill_events"),
        "memory_reclaim": metric_stats(telemetry, "cgroup_memory_reclaim_events"),
        "psi_memory_avg10": metric_stats(telemetry, "psi_memory_some_avg10"),
        "psi_cpu_avg10": metric_stats(telemetry, "psi_cpu_some_avg10"),
        "psi_io_avg10": metric_stats(telemetry, "psi_io_some_avg10"),
    }
    metadata: dict[str, Any] = {}
    metadata_path = case_root / "telemetry" / "telemetry.metadata.json"
    if metadata_path.exists():
        try:
            metadata = json.loads(metadata_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            metadata = {}
    return {
        **row,
        "path": str(case_root.relative_to(root)),
        "telemetry_samples": len(telemetry),
        "workload": workload_stats(workload),
        "metrics": key_metrics,
        "rss_slope_bytes_per_minute": slope_per_minute(telemetry, "vmrss_bytes"),
        "cgroup_slope_bytes_per_minute": slope_per_minute(telemetry, "cgroup_memory_current_bytes"),
        "heap_slope_bytes_per_minute": slope_per_minute(telemetry, "jvm_heap_used_bytes"),
        "jvm_heap_available_samples": sum(1 for sample in telemetry if sample.get("jvm_heap_available") is True),
        "effective_affinity": next((sample.get("effective_cpu_affinity") for sample in telemetry if sample.get("effective_cpu_affinity")), None),
        "metadata": metadata,
    }


def write_report(root: Path, cases: list[dict[str, Any]], report_path: Path, analysis_path: Path) -> None:
    complete = sum(case["status"] == "complete" for case in cases)
    failed = sum(case["status"] == "failed" for case in cases)
    by_target: dict[str, list[dict[str, Any]]] = {}
    for case in cases:
        by_target.setdefault(case["target"], []).append(case)

    lines = [
        "# HydraCache 0.67 Stage 3 metric-expansion report",
        "",
        "> Exploratory evidence only. This output is intentionally separate from qualification/bootstrap evidence.",
        "",
        f"- Output root: `{root}`",
        f"- Cases: {len(cases)} total; {complete} complete; {failed} failed",
        f"- Source commit: `{(root / 'reproduction-command.txt').read_text(encoding='utf-8', errors='replace').split('source_commit=', 1)[-1].splitlines()[0] if (root / 'reproduction-command.txt').exists() and 'source_commit=' in (root / 'reproduction-command.txt').read_text(encoding='utf-8', errors='replace') else 'unavailable'}`",
        "- Sampling interval: one second unless the run metadata says otherwise.",
        "",
        "## Case summary",
        "",
        "| Experiment | Target | Case | Status | Telemetry | RSS p50/p95/max | Cgroup current p95/max | CPU p95/max | Latency p95 | Errors | RSS slope/min |",
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for case in cases:
        rss = case["metrics"]["rss_bytes"]
        cg = case["metrics"]["cgroup_current_bytes"]
        cpu = case["metrics"]["cpu_percent"]
        workload = case["workload"]
        lines.append(
            f"| {case['experiment']} | {case['target']} | {case['case']} | {case['status']} | {case['telemetry_samples']} "
            f"| {fmt(rss['p50'], 0)}/{fmt(rss['p95'], 0)}/{fmt(rss['max'], 0)} "
            f"| {fmt(cg['p95'], 0)}/{fmt(cg['max'], 0)} | {fmt(cpu['p95'])}/{fmt(cpu['max'])} "
            f"| {fmt(workload['p95_latency_ms'])} | {workload['errors']} | {fmt(case['rss_slope_bytes_per_minute'], 0)} |"
        )
    lines.extend([
        "",
        "## Target-level reading",
        "",
        "The table is a screening view, not a causal attribution. Compare like-for-like rows (same payload, key length, clients, pipeline, request count and affinity).",
        "",
    ])
    for target, target_cases in sorted(by_target.items()):
        valid = [case for case in target_cases if case["status"] == "complete"]
        lines.append(f"### {target}")
        lines.append("")
        if not valid:
            lines.append("No complete cases.")
            lines.append("")
            continue
        max_rss = max((number(case["metrics"]["rss_bytes"]["max"]) or 0 for case in valid), default=0)
        max_cpu = max((number(case["metrics"]["cpu_percent"]["p95"]) or 0 for case in valid), default=0)
        lines.append(f"- Complete cases: {len(valid)}; largest sampled RSS: {fmt(max_rss, 0)} bytes; highest sampled CPU p95: {fmt(max_cpu)}%.")
        lines.append("- JVM heap is reported independently; `N/A` means the probe was unavailable, not zero heap.")
        lines.append("")
    lines.extend([
        "## Metric definitions and limitations",
        "",
        "- `container_cpu_percent` is cgroup CPU normalized by effective affinity; Hydra's host process has no container CPU field and uses `process_cpu_percent`.",
        "- `vmrss_bytes`/`vmhwm_bytes` are process RSS/high-water RSS. Cgroup current/peak/limit are separate accounting domains.",
        "- `jvm_heap_*` comes from `jcmd GC.heap_info` when available; slim images may show unavailable. Never substitute RSS for heap.",
        "- PSI, faults, I/O, context switches and host network counters are host/kernel signals sampled with the target. They are supporting evidence and may include unrelated host activity.",
        "- A failed workload or failed target start remains failed even if telemetry files exist. Missing values are preserved as `N/A`.",
        "",
        "## Reproduction",
        "",
        "Run from the exact source checkout after installing the pinned Hazelcast client and using pinned image digests:",
        "",
        "```bash",
        "export DOCKER_HOST=unix:///run/user/1002/docker.sock",
        "export HAZELCAST_IMAGE='hazelcast/hazelcast:5.7.0-slim-jdk21@sha256:d9939853200b70cfd52115a9f1e905ef37cd3d98e1f966ce67c8d5e1c9e21e90'",
        "export HAZELCAST_CLIENT_PYTHON=/home/hydracache-admin/.venvs/hazelcast/bin/python",
        "export HAZELCAST_CLIENT_VERSION=5.5.0",
        "export MEASUREMENT_AFFINITY=4",
        "bash scripts/perf/run-metric-expansion-stage.sh /dev/shm/hydracache-metric-expansion-<timestamp>",
        "```",
        "",
    ])
    report_path.write_text("\n".join(lines) + "\n", encoding="utf-8")

    recommendations = [
        "# Stage 3 metric-expansion analysis",
        "",
        "> Exploratory analysis for optimization planning; it is not a qualification decision.",
        "",
        "## Decision rules",
        "",
        "1. Treat RSS/HWM, cgroup memory and JVM heap as different quantities.",
        "2. Investigate only comparisons with zero workload errors and matching matrix controls.",
        "3. Use slopes and peak/current deltas to prioritize, then confirm with a dedicated controlled run.",
        "4. Treat PSI, host network and host-wide I/O as confounder indicators, not target-attributed cost.",
        "",
        "## Prioritized follow-ups",
        "",
        "- If a target's cgroup current or RSS rises while JVM heap is flat, inspect native buffers, allocator behavior, persistence and off-heap structures.",
        "- If RSS rises only under hot/Zipf-like keys, inspect cache index/eviction metadata and admission behavior.",
        "- If RSS rises with long keys or larger payloads, separate key metadata, value storage and serialization overhead.",
        "- If CPU p95 rises with client/pipeline changes while throughput does not, inspect contention, batching and connection handling.",
        "- If OOM/reclaim counters or PSI rise in pressure cases, compare degradation and fail-closed behavior before increasing limits.",
        "- If the Hazelcast JVM probe is unavailable, repeat only the JVM subset with a non-slim image or a deliberately enabled JDK diagnostic tool; do not infer heap from RSS.",
        "",
        "## Per-target screening",
        "",
    ]
    for target, target_cases in sorted(by_target.items()):
        complete_cases = [case for case in target_cases if case["status"] == "complete"]
        slopes = [case["rss_slope_bytes_per_minute"] for case in complete_cases if case["rss_slope_bytes_per_minute"] is not None]
        errors = sum(case["workload"]["errors"] for case in complete_cases)
        recommendations.append(f"### {target}")
        recommendations.append("")
        recommendations.append(f"- Complete cases: {len(complete_cases)}; workload errors across complete cases: {errors}.")
        recommendations.append(f"- Observed RSS slopes (bytes/min): {fmt(statistics.median(slopes), 0) if slopes else 'N/A'} median; inspect outliers in `case-index.json`.")
        recommendations.append("- Compare the target against the corresponding rows in the report before attributing a difference.")
        recommendations.append("")
    recommendations.extend([
        "## Raw evidence index",
        "",
        "Every case directory contains `case-metadata.txt`, collector JSONL/CSV and metadata, workload JSONL/logs, target logs, and inspect snapshots when available. The generated `case-index.json` is the machine-readable index.",
        "",
    ])
    analysis_path.write_text("\n".join(recommendations), encoding="utf-8")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--analysis", type=Path, required=True)
    args = parser.parse_args()
    cases = [summarize_case(args.input, row) for row in read_status(args.input)]
    (args.input / "case-index.json").write_text(json.dumps(cases, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    write_report(args.input, cases, args.output, args.analysis)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
