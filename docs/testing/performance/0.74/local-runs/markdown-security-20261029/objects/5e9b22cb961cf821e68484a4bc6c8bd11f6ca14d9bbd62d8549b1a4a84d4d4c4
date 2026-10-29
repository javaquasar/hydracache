#!/usr/bin/env python3
"""Fail when a memory case's final checkpoint is not covered by telemetry."""

from __future__ import annotations

import argparse
import csv
import json
from pathlib import Path


def coverage_problem(case_dir: Path, interval_seconds: float) -> str | None:
    checkpoint_path = case_dir / "checkpoints.tsv"
    try:
        with checkpoint_path.open(newline="", encoding="utf-8") as handle:
            checkpoints = list(csv.DictReader(handle, delimiter="\t"))
    except OSError as error:
        return f"cannot read {checkpoint_path}: {error}"
    if not checkpoints:
        return "case has no checkpoints"

    try:
        final_checkpoint = float(checkpoints[-1]["timestamp"])
    except (KeyError, TypeError, ValueError):
        return "final checkpoint timestamp is invalid"

    sample_times: list[float] = []
    for path in sorted((case_dir / "telemetry").glob("*.jsonl")):
        try:
            lines = path.read_text(encoding="utf-8").splitlines()
        except OSError as error:
            return f"cannot read {path}: {error}"
        for line in lines:
            try:
                timestamp = json.loads(line).get("timestamp_unix")
            except json.JSONDecodeError:
                continue
            if isinstance(timestamp, (int, float)):
                sample_times.append(float(timestamp))
    if len(sample_times) < 3:
        return f"case has only {len(sample_times)} timestamped telemetry samples"

    maximum_gap = max(3.0, interval_seconds * 3.0)
    gap = final_checkpoint - max(sample_times)
    if gap > maximum_gap:
        return (
            f"final checkpoint is {gap:.3f}s newer than the final telemetry sample "
            f"(maximum allowed gap {maximum_gap:.3f}s)"
        )
    return None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--case-dir", type=Path, required=True)
    parser.add_argument("--interval-seconds", type=float, required=True)
    args = parser.parse_args()
    problem = coverage_problem(args.case_dir, args.interval_seconds)
    if problem is not None:
        print(f"memory telemetry coverage failed for {args.case_dir}: {problem}")
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
