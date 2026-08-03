#!/usr/bin/env python3
"""Render the ten-experiment memory investigation bundle.

The report is intentionally descriptive: unavailable JVM heap telemetry is
reported as unavailable, and no RSS value is presented as a heap measurement.
"""

from __future__ import annotations

import argparse
import csv
import json
import math
from pathlib import Path
from statistics import median
from typing import Any


def percentile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    values = sorted(values)
    index = (len(values) - 1) * fraction
    low, high = math.floor(index), math.ceil(index)
    if low == high:
        return values[low]
    return values[low] + (values[high] - values[low]) * (index - low)


def fmt_bytes(value: float | None) -> str:
    if value is None:
        return "n/a"
    return f"{value / (1024 * 1024):.2f} MiB"


def load_rows(root: Path) -> list[dict[str, str]]:
    path = root / "case-status.tsv"
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle, delimiter="\t"))


def telemetry_metrics(case_dir: Path) -> dict[str, Any]:
    rows: list[dict[str, Any]] = []
    for path in sorted((case_dir / "telemetry").glob("*.jsonl")):
        for line in path.read_text(encoding="utf-8").splitlines():
            try:
                rows.append(json.loads(line))
            except json.JSONDecodeError:
                continue
    metrics: dict[str, Any] = {"samples": len(rows)}
    for key in (
        "container_cpu_percent", "process_cpu_percent", "vmrss_bytes", "vmhwm_bytes",
        "smaps_rollup_rss_bytes", "smaps_rollup_pss_anon_bytes",
        "smaps_rollup_pss_file_bytes", "cgroup_memory_current_bytes",
        "cgroup_memory_peak_bytes", "cgroup_memory_anon_bytes", "cgroup_memory_file_bytes",
        "cgroup_memory_slab_bytes", "process_threads", "process_fd_count",
        "jvm_heap_used_bytes", "jvm_heap_committed_bytes", "jvm_heap_max_bytes",
    ):
        values = [float(row[key]) for row in rows if isinstance(row.get(key), (int, float))]
        if values:
            metrics[key] = {
                "p50": percentile(values, 0.50),
                "p95": percentile(values, 0.95),
                "max": max(values),
                "min": min(values),
            }
    heap_available = [row.get("jvm_heap_available") for row in rows]
    metrics["jvm_heap_available"] = bool(heap_available and any(heap_available))
    return metrics


def case_dir(root: Path, row: dict[str, str]) -> Path:
    return root / "experiments" / row["experiment"] / row["target"] / row["case"]


def metric_text(metrics: dict[str, Any], key: str, bytes_value: bool = False) -> str:
    value = (metrics.get(key) or {}).get("p50")
    if bytes_value:
        return fmt_bytes(value)
    return "n/a" if value is None else f"{value:.2f}"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-root", type=Path, required=True)
    args = parser.parse_args()
    rows = load_rows(args.input)
    enriched: list[tuple[dict[str, str], dict[str, Any]]] = []
    for row in rows:
        metrics = telemetry_metrics(case_dir(args.input, row))
        row["metrics"] = metrics  # type: ignore[index]
        enriched.append((row, metrics))

    complete = sum(row["status"] == "complete" for row, _ in enriched)
    failed = sum(row["status"] == "failed" for row, _ in enriched)
    not_applicable = sum(row["status"] == "not_applicable" for row, _ in enriched)
    lines = [
        "# Memory investigation stage (10 experiments)",
        "",
        "This bundle is exploratory evidence only; it is not qualification/bootstrap evidence.",
        "Every applicable case starts a fresh target process/container. Raw files are retained next to this report.",
        "",
        "## Reproduction contract",
        "",
        f"- Source commit: `{(args.input / 'reproduction-command.txt').read_text(encoding='utf-8').split('source_commit=', 1)[-1].splitlines()[0] if (args.input / 'reproduction-command.txt').exists() else 'unknown'}`",
        f"- Measurement affinity: `{(args.input / 'reproduction-command.txt').read_text(encoding='utf-8').split('measurement_affinity=', 1)[-1].splitlines()[0] if (args.input / 'reproduction-command.txt').exists() and 'measurement_affinity=' in (args.input / 'reproduction-command.txt').read_text(encoding='utf-8') else 'unknown'}`",
        "- Sampling interval: 1 second by default; each raw sample contains process, cgroup, smaps-rollup, affinity, thread/FD, and optional JVM fields.",
        "- JVM heap: marked unavailable unless `JVM_HEAP_CMD` was explicitly configured; RSS is never substituted for heap.",
        "",
        "## Outcome",
        "",
        f"- Applicable cases complete: **{complete}**",
        f"- Failed cases: **{failed}**",
        f"- Not applicable cases: **{not_applicable}**",
        f"- Total recorded rows: **{len(enriched)}**",
        "",
        "## Case-level summary",
        "",
        "| Experiment | Target | Case | Status | Samples | RSS p50 | RSS p95 | cgroup current p50 | anon p50 | file p50 | CPU p50 | threads p50 | FDs p50 |",
        "|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for row, metrics in enriched:
        lines.append(
            f"| {row['experiment']} | {row['target']} | {row['case']} | {row['status']} | "
            f"{metrics.get('samples', 0)} | {metric_text(metrics, 'vmrss_bytes', True)} | "
            f"{fmt_bytes((metrics.get('vmrss_bytes') or {}).get('p95'))} | "
            f"{metric_text(metrics, 'cgroup_memory_current_bytes', True)} | "
            f"{metric_text(metrics, 'cgroup_memory_anon_bytes', True)} | "
            f"{metric_text(metrics, 'cgroup_memory_file_bytes', True)} | "
            f"{metric_text(metrics, 'process_cpu_percent')} | "
            f"{metric_text(metrics, 'process_threads')} | {metric_text(metrics, 'process_fd_count')} |"
        )
    lines += [
        "",
        "## Interpretation rules",
        "",
        "1. `VmRSS` and smaps-rollup RSS describe resident process memory; cgroup values include the container's charged memory and can include file-backed pages.",
        "2. `memory.stat` anon/file/slab fields are reported separately so allocator growth is not confused with page cache or kernel slab.",
        "3. A single case is descriptive, not a leak proof. Leak conclusions are produced only by the separate soak stage, which computes slopes across checkpoints.",
        "4. Failed or unavailable cases remain visible and are not silently removed from the denominator.",
        "",
        "## Experiment definitions",
        "",
        "1. Cold start idle footprint; 2. keyspace scaling (1k/10k/50k); 3. fixed versus random key range; 4. persistence/storage modes; 5. Admin API ablation; 6. TTL residual memory; 7. SET/GET mix; 8. client concurrency; 9. restart observation; 10. payload scaling.",
        "",
        "## Raw evidence",
        "",
        "Each case directory contains `case-metadata.txt`, target logs, container metadata where applicable, `telemetry/*.jsonl` and CSV, and `telemetry-summary.json`. The root contains `hardware-validation.txt`, `reproduction-command.txt`, `case-status.tsv`, and Docker metadata.",
        "",
    ]
    args.output.write_text("\n".join(lines), encoding="utf-8")

    analysis = args.output.with_name("memory-optimization-analysis.md")
    target_aggregates: dict[str, list[float]] = {}
    for row, metrics in enriched:
        value = (metrics.get("vmrss_bytes") or {}).get("p50")
        if isinstance(value, (int, float)):
            target_aggregates.setdefault(row["target"], []).append(float(value))
    analysis_lines = [
        "# Memory optimization analysis (stage 1)",
        "",
        "This is a hypothesis list grounded in the ten-case bundle. It does not claim a leak without the separate soak stage.",
        "",
        "## What to compare",
        "",
        "- Compare fresh-process cold-start RSS with keyspace/payload/concurrency cases. A large cold-start delta points to baseline runtime or enabled services; growth correlated with keys/payload points to retained data/metadata.",
        "- Compare `cgroup_memory_anon_bytes` with `cgroup_memory_file_bytes`. Anon growth is the stronger allocator/object-retention signal; file growth can be page cache or mapped files.",
        "- Compare `smaps_rollup_pss_anon_bytes` and `smaps_rollup_pss_file_bytes` against VmRSS to identify shared/runtime mappings.",
        "- Compare thread and FD counts across concurrency and restart cases. Growth without workload growth is an operational leak candidate.",
        "",
        "## Target medians observed in this bundle",
        "",
    ]
    for target, values in sorted(target_aggregates.items()):
        analysis_lines.append(f"- `{target}` case-level median RSS p50: **{fmt_bytes(median(values))}**")
    analysis_lines += [
        "",
        "## Improvement candidates",
        "",
        "1. Keep Admin API, RESP API, persistence, and any diagnostics disabled in production profiles unless required; use the ablation experiment to quantify each service.",
        "2. Bound keyspace and value retention, and verify expiry/delete paths with the TTL and workload-mix experiments.",
        "3. Separate allocator fragmentation from live objects using post-load idle and soak checkpoints; do not optimize from cgroup peak alone.",
        "4. If anonymous memory grows while logical key count is stable, inspect cache/index capacity, allocator arenas, and background task queues before changing SLOs.",
        "5. If file-backed memory dominates, inspect storage layout and page-cache behavior rather than reducing object allocations blindly.",
        "",
        "The leak stage must be completed before labeling any slope as a confirmed memory leak.",
    ]
    analysis.write_text("\n".join(analysis_lines) + "\n", encoding="utf-8")
    (args.input / "case-index.json").write_text(json.dumps(enriched, indent=2, default=str) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
