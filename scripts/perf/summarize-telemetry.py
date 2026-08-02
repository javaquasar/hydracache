#!/usr/bin/env python3
"""Summarize telemetry JSONL files without hiding missing measurements."""

from __future__ import annotations

import argparse
import json
import math
from collections import defaultdict
from pathlib import Path
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


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    buckets: dict[str, dict[str, list[float]]] = defaultdict(lambda: defaultdict(list))
    for path in sorted(args.input.rglob("*.jsonl")):
        for line in path.read_text().splitlines():
            row: dict[str, Any] = json.loads(line)
            bucket = path.stem
            for metric in (
                "container_cpu_percent", "process_cpu_percent", "process_cpu_ticks",
                "vmrss_bytes", "vmhwm_bytes",
                "cgroup_memory_current_bytes", "cgroup_memory_peak_bytes",
                "jvm_heap_used_bytes", "jvm_heap_committed_bytes", "jvm_heap_max_bytes",
            ):
                value = row.get(metric)
                if isinstance(value, (int, float)):
                    buckets[bucket][metric].append(float(value))
    summary: dict[str, Any] = {}
    for bucket, metrics in buckets.items():
        summary[bucket] = {
            metric: {
                "samples": len(values),
                "p50": percentile(values, 0.50),
                "p95": percentile(values, 0.95),
                "max": max(values),
            }
            for metric, values in metrics.items()
        }
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
