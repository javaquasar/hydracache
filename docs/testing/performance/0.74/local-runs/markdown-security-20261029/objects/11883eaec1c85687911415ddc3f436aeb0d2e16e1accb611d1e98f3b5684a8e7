#!/usr/bin/env python3
"""Create a compact, durable, non-promotable summary of one memory diagnostic."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path
from typing import Any


def read_contract(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value
    return values


def build_summary(root: Path) -> dict[str, Any]:
    contract = read_contract(root / "reproduction-command.txt")
    if contract.get("ship_evidence_eligible") != "false":
        raise ValueError("diagnostic must explicitly record ship_evidence_eligible=false")
    with (root / "leak-status.tsv").open(newline="", encoding="utf-8") as handle:
        status_rows = list(csv.DictReader(handle, delimiter="\t"))
    index_path = root / "leak-index.json"
    index_rows = json.loads(index_path.read_text(encoding="utf-8")) if index_path.exists() else []
    metrics_by_case = {
        (row[0]["experiment"], row[0]["target"]): {"sample_count": row[1]["samples"], "metrics": row[2]}
        for row in index_rows
    }
    cases = []
    for row in status_rows:
        key = (row["experiment"], row["target"])
        cases.append({**row, **metrics_by_case.get(key, {"sample_count": 0, "metrics": {}})})
    complete = bool(cases) and all(row["status"] in {"complete", "not_applicable"} for row in cases)
    return {
        "schema_version": 1,
        "release": "0.70",
        "non_promotable": True,
        "complete": complete,
        "provenance": {
            "source_commit": contract.get("source_commit", ""),
            "source_tree_clean": contract.get("source_tree_clean") == "true",
            "hydracache_binary_sha256": contract.get("hydracache_binary_sha256", ""),
            "environment": contract.get("diagnostic_environment", ""),
        },
        "fingerprint": {
            "kernel": contract.get("kernel", ""),
            "online_cpus": contract.get("online_cpus", ""),
            "affinity": contract.get("affinity", ""),
            "redis_image": contract.get("redis_image", ""),
            "targets": contract.get("targets", ""),
        },
        "workload": {
            "interval_seconds": contract.get("interval_seconds", ""),
            "duration_seconds": contract.get("duration_seconds", ""),
            "cycles": contract.get("cycles", ""),
            "batch_requests": contract.get("batch_requests", ""),
        },
        "cases": cases,
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    summary = build_summary(args.input)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if not summary["complete"]:
        raise SystemExit("memory diagnostic summary contains incomplete cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
