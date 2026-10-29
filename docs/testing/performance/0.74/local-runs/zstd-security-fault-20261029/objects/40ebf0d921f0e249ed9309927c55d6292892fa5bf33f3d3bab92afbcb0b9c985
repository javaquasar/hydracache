#!/usr/bin/env python3
"""Compare compact summaries without turning noisy hosted RSS into a release threshold."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any


def comparable_fingerprint(summary: dict[str, Any]) -> dict[str, Any]:
    fingerprint = summary.get("fingerprint", {})
    return {key: fingerprint.get(key) for key in ("kernel", "online_cpus", "affinity", "redis_image", "targets")}


def metric_last(case: dict[str, Any], metric: str) -> float | None:
    value = case.get("metrics", {}).get(metric, {}).get("last")
    return float(value) if isinstance(value, (int, float)) else None


def compare(candidate: dict[str, Any], baseline: dict[str, Any] | None) -> dict[str, Any]:
    result: dict[str, Any] = {
        "schema_version": 1,
        "release": candidate.get("release"),
        "candidate_source_commit": candidate.get("provenance", {}).get("source_commit"),
        "baseline_available": baseline is not None,
        "comparable_fingerprint": False,
        "gating": False,
        "deltas": [],
    }
    if baseline is None:
        return result
    result["baseline_source_commit"] = baseline.get("provenance", {}).get("source_commit")
    result["comparable_fingerprint"] = comparable_fingerprint(candidate) == comparable_fingerprint(baseline)
    old_cases = {(row.get("experiment"), row.get("target")): row for row in baseline.get("cases", [])}
    for current in candidate.get("cases", []):
        key = (current.get("experiment"), current.get("target"))
        previous = old_cases.get(key)
        if previous is None:
            continue
        metrics = {}
        for metric in ("vmrss_bytes", "smaps_rollup_pss_anon_bytes"):
            old = metric_last(previous, metric)
            new = metric_last(current, metric)
            metrics[metric] = {
                "baseline_last": old,
                "candidate_last": new,
                "delta_bytes": None if old is None or new is None else new - old,
                "relative_delta": None if old in (None, 0) or new is None else (new - old) / old,
            }
        result["deltas"].append({"experiment": key[0], "target": key[1], "metrics": metrics})
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    candidate = json.loads(args.candidate.read_text(encoding="utf-8"))
    baseline = json.loads(args.baseline.read_text(encoding="utf-8")) if args.baseline else None
    result = compare(candidate, baseline)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
