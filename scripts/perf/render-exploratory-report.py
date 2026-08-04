#!/usr/bin/env python3
"""Render a self-contained Markdown report and immutable artifact manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
from datetime import datetime, timezone
from pathlib import Path


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def source_commit(root: Path) -> str:
    try:
        return subprocess.check_output(
            ["git", "-C", str(root), "rev-parse", "HEAD"], text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unavailable"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-root", type=Path)
    args = parser.parse_args()
    root = args.input.resolve()
    files = []
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.name in {"report.md", "artifact-manifest.json"}:
            continue
        files.append({
            "path": path.relative_to(root).as_posix(),
            "bytes": path.stat().st_size,
            "sha256": sha256(path),
        })
    manifest = {
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "source_commit": source_commit(args.source_root.resolve() if args.source_root else root),
        "files": files,
    }
    (root / "artifact-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    hardware_path = root / "hardware-validation.txt"
    summary_path = root / "telemetry-summary.json"
    command_path = root / "reproduction-command.txt"
    hardware = hardware_path.read_text(encoding="utf-8") if hardware_path.exists() else "unavailable\n"
    summary = summary_path.read_text(encoding="utf-8") if summary_path.exists() else "{}\n"
    command = command_path.read_text(encoding="utf-8") if command_path.exists() else "unavailable\n"
    lines = [
        "# Relative eight-case telemetry report",
        "",
        "> Exploratory only. This report is not qualification/bootstrap evidence.",
        "",
        "- Generated (UTC): " + manifest["generated_utc"],
        "- Source commit: " + manifest["source_commit"],
        "- Targets: HydraCache, Redis, Hazelcast Community",
        "- Workload: 8 cases x SET/GET x configured repeats",
        "- Sampling interval: 1 second by default",
        "",
        "## Reproduction",
        "",
        "The exact command and environment used for this run:",
        "",
        "~~~text",
        command.rstrip(),
        "~~~",
        "",
        "Re-run from the recorded source commit with the same image digest, client version, affinity, request count, and repeats.",
        "",
        "## Host and validation receipt",
        "",
        "~~~text",
        hardware.rstrip(),
        "~~~",
        "",
        "## Telemetry summary",
        "",
        "The summary preserves sample counts and reports p50/p95/max. Missing JVM heap fields remain unavailable; they are never inferred from RSS.",
        "",
        "~~~json",
        summary.rstrip(),
        "~~~",
        "",
        "## Artifact index",
        "",
        "| Path | Bytes | SHA-256 |",
        "|---|---:|---|",
    ]
    lines.extend(
        "| " + item["path"] + " | " + str(item["bytes"]) + " | " + item["sha256"] + " |"
        for item in files
    )
    lines.extend([
        "",
        "Raw benchmark logs, telemetry JSONL/CSV, Docker inspect metadata, image identifiers,",
        "hardware validation, and the artifact manifest are all in this same output directory.",
        "The directory must be copied unchanged into the branch results tree after review.",
        "",
    ])
    args.output.write_text("\n".join(lines), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
