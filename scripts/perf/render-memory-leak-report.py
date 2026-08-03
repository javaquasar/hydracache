#!/usr/bin/env python3
"""Render leak/soak evidence with explicit duration and slope caveats."""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from typing import Any


def regression(points: list[tuple[float, float]]) -> tuple[float | None, float | None]:
    if len(points) < 3:
        return None, None
    x_bar = sum(x for x, _ in points) / len(points)
    y_bar = sum(y for _, y in points) / len(points)
    denominator = sum((x - x_bar) ** 2 for x, _ in points)
    if denominator == 0:
        return None, None
    slope = sum((x - x_bar) * (y - y_bar) for x, y in points) / denominator
    intercept = y_bar - slope * x_bar
    return slope, intercept


def read_samples(case_dir: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for path in sorted((case_dir / "telemetry").glob("*.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(row.get("timestamp_unix"), (int, float)):
                rows.append(row)
    return sorted(rows, key=lambda row: row["timestamp_unix"])


def fmt_bytes(value: float | None) -> str:
    return "n/a" if value is None else f"{value / (1024 * 1024):.2f} MiB"


def metric_result(rows: list[dict[str, Any]], key: str) -> dict[str, Any]:
    points = [(float(row["timestamp_unix"]), float(row[key])) for row in rows if isinstance(row.get(key), (int, float))]
    if not points:
        return {"samples": 0, "slope_bytes_per_minute": None, "first": None, "last": None, "delta": None}
    slope, _ = regression(points)
    slope_per_minute = None if slope is None else slope * 60.0
    return {
        "samples": len(points),
        "slope_bytes_per_minute": slope_per_minute,
        "first": points[0][1],
        "last": points[-1][1],
        "delta": points[-1][1] - points[0][1],
        "duration_seconds": points[-1][0] - points[0][0] if len(points) > 1 else 0,
    }


def classify(result: dict[str, Any]) -> str:
    duration = result.get("duration_seconds") or 0
    slope = result.get("slope_bytes_per_minute")
    if duration < 120 or result.get("samples", 0) < 30:
        return "insufficient-duration"
    if slope is None:
        return "unavailable"
    if slope > 1024 * 1024:
        return "possible-growth"
    return "plateau-or-noise"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    rows: list[dict[str, str]] = []
    status_path = args.input / "leak-status.tsv"
    if status_path.exists():
        with status_path.open(newline="", encoding="utf-8") as handle:
            rows = list(csv.DictReader(handle, delimiter="\t"))
    results: list[tuple[dict[str, str], dict[str, Any], dict[str, dict[str, Any]]]] = []
    for row in rows:
        case_dir = args.input / "leak-experiments" / row["experiment"] / row["target"]
        samples = read_samples(case_dir)
        metrics = {key: metric_result(samples, key) for key in (
            "vmrss_bytes", "smaps_rollup_pss_anon_bytes", "cgroup_memory_current_bytes",
            "cgroup_memory_anon_bytes", "cgroup_memory_file_bytes", "process_threads", "process_fd_count",
        )}
        results.append((row, {"samples": len(samples)}, metrics))
    lines = [
        "# Memory leak / soak stage",
        "",
        "This stage is independent exploratory evidence and is not qualification/bootstrap evidence.",
        "A possible-growth label is a screening result, not proof of a bug; repeat with a longer duration and a heap profiler before changing code.",
        "",
        "## Reproduction contract",
        "",
        f"- Source and run parameters: `{args.input / 'reproduction-command.txt'}`",
        "- One-second process/container telemetry includes RSS, smaps PSS anon/file, cgroup anon/file/slab, CPU, affinity, threads, and FD count.",
        "- JVM heap remains unavailable unless an explicit `JVM_HEAP_CMD` is configured.",
        "",
        "## Slope summary",
        "",
        "| Experiment | Target | Pattern | Status | Samples | RSS slope MiB/min | anon slope MiB/min | cgroup slope MiB/min | Duration s | Classification |",
        "|---|---|---|---|---:|---:|---:|---:|---:|---|",
    ]
    for row, meta, metrics in results:
        rss = metrics["vmrss_bytes"]
        anon = metrics["cgroup_memory_anon_bytes"]
        cgroup = metrics["cgroup_memory_current_bytes"]
        slope = rss.get("slope_bytes_per_minute")
        anon_slope = anon.get("slope_bytes_per_minute")
        cgroup_slope = cgroup.get("slope_bytes_per_minute")
        duration = rss.get("duration_seconds") or anon.get("duration_seconds") or cgroup.get("duration_seconds") or 0
        classification = "not-applicable" if row["status"] == "not_applicable" else classify(rss)
        lines.append(
            f"| {row['experiment']} | {row['target']} | {row['pattern']} | {row['status']} | {meta['samples']} | "
            f"{'n/a' if slope is None else f'{slope / (1024*1024):.3f}'} | "
            f"{'n/a' if anon_slope is None else f'{anon_slope / (1024*1024):.3f}'} | "
            f"{'n/a' if cgroup_slope is None else f'{cgroup_slope / (1024*1024):.3f}'} | {duration:.0f} | {classification} |"
        )
    lines += [
        "",
        "## Analysis guidance",
        "",
        "- A leak candidate should show a positive slope in at least two independent resident/anonymous signals, remain after expiry/reset, and reproduce across fresh runs.",
        "- RSS growth with flat cgroup anon and rising cgroup file is more consistent with page cache or mappings than live Rust objects.",
        "- Anon growth with rising thread/FD counts points to retained tasks, connections, or queues; anon growth with stable counts points to allocator/object retention.",
        "- A positive slope that flattens after a bounded keyspace is fragmentation/capacity behavior until disproven, not automatically a leak.",
        "- The expiry/reclamation and cycle-reset cases are specifically intended to reveal whether memory falls after logical data removal.",
        "- Hazelcast expiry/reclamation is marked not-applicable because this harness exercises Redis-protocol TTL; Hazelcast native expiry requires a separate client/API workload and is not silently substituted.",
        "",
        "## Recommended next actions",
        "",
        "1. Repeat any possible-growth row at 30–60 minutes and at least three fresh processes.",
        "2. If anonymous memory remains positive, capture `smaps_rollup`, allocator statistics, and application-level key/index counts at the same checkpoints.",
        "3. Compare Admin API on/off and persistence modes before changing defaults; retain only changes that reduce anon/RSS without violating latency/error SLOs.",
        "4. Treat JVM heap as a separate measurement: configure JMX/JVM_HEAP_CMD for Hazelcast and record it as unavailable otherwise.",
    ]
    args.output.write_text("\n".join(lines) + "\n", encoding="utf-8")
    (args.input / "leak-index.json").write_text(json.dumps(results, indent=2, default=str) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
