#!/usr/bin/env python3
"""Create a stable envelope around Gungraun's versioned NDJSON summaries."""

import argparse
import json
from pathlib import Path


def read_ndjson(path: Path) -> list[dict]:
    records = []
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        if not line.lstrip().startswith("{"):
            continue
        value = json.loads(line)
        if not isinstance(value, dict):
            raise ValueError(f"{path}:{number}: expected an object")
        records.append(value)
    if not records:
        raise ValueError(f"{path}: no benchmark summaries")
    return records


def metric_integer(value: dict) -> int:
    if not isinstance(value, dict) or set(value) != {"Int"} or not isinstance(value["Int"], int):
        raise ValueError("expected an integer Gungraun metric")
    return value["Int"]


def instruction_comparisons(rows: list[dict]) -> list[dict]:
    comparisons = []
    for row in rows:
        callgrind_profiles = [item for item in row.get("profiles", []) if item.get("tool") == "Callgrind"]
        if len(callgrind_profiles) != 1:
            raise ValueError(f"{row.get('module_path')}: expected exactly one Callgrind profile")
        metrics = callgrind_profiles[0]["summaries"]["total"]["summary"]["Callgrind"]
        instruction = metrics["Ir"]
        pair = instruction["metrics"].get("Both")
        if not isinstance(pair, list) or len(pair) != 2:
            raise ValueError(f"{row.get('module_path')}: expected paired Ir metrics")
        head_ir, base_ir = (metric_integer(item) for item in pair)
        comparisons.append(
            {
                "benchmark": row["module_path"],
                "base_ir": base_ir,
                "head_ir": head_ir,
                "diff_percent": float(instruction["diffs"]["diff_pct"]),
                "regressed": bool(callgrind_profiles[0]["summaries"]["total"].get("regressions")),
            }
        )
    return sorted(comparisons, key=lambda item: item["benchmark"])


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", type=Path, required=True)
    parser.add_argument("--head", type=Path, required=True)
    parser.add_argument("--base-sha", required=True)
    parser.add_argument("--head-sha", required=True)
    parser.add_argument("--status", type=int, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()

    base = read_ndjson(args.base)
    head = read_ndjson(args.head)
    names = lambda rows: sorted(f"{row.get('module_path')}::{row.get('id') or ''}" for row in rows)
    if names(base) != names(head):
        raise ValueError("base/head benchmark identity sets differ")

    report = {
        "schema_version": 1,
        "profile": "ci-instruction-v1",
        "verdict": "accepted" if args.status == 0 else "rejected",
        "runner_exit_status": args.status,
        "base_sha": args.base_sha,
        "head_sha": args.head_sha,
        "claim_boundary": {
            "relative_work_regression": True,
            "qualification_evidence": False,
            "bootstrap_evidence": False,
            "ship_evidence_eligible": False,
            "latency_claim": False,
            "throughput_claim": False,
            "capacity_claim": False,
        },
        "comparisons": instruction_comparisons(head),
        "base": base,
        "head": head,
    }
    args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
