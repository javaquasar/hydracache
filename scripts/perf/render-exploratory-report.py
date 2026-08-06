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


def reproduction_value(path: Path, key: str) -> str:
    if not path.exists():
        return "unavailable"
    prefix = key + "="
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.startswith(prefix):
            return line[len(prefix):]
    return "unavailable"


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-root", type=Path)
    parser.add_argument("--policy", type=Path, required=True)
    args = parser.parse_args()
    root = args.input.resolve()
    if args.output.resolve() != root / "report.md":
        raise ValueError("indicative report output must be the canonical input/report.md")
    policy_path = args.policy.resolve()
    policy = json.loads(policy_path.read_text(encoding="utf-8"))
    if policy.get("policy_id") != "indicative-exploratory-v1":
        raise ValueError("unsupported exploratory reporting policy")
    if any(policy.get(field) is not False for field in [
        "authoritative",
        "capacity_bearing",
        "qualification_evidence",
        "bootstrap_evidence",
        "ship_evidence_eligible",
    ]):
        raise ValueError("indicative policy must remain non-authoritative and non-promotable")
    command_path = root / "reproduction-command.txt"
    hardware_path = root / "hardware-validation.txt"
    summary_path = root / "telemetry-summary.json"
    prerequisite_paths = [command_path, hardware_path, summary_path]
    missing_prerequisites = [path.name for path in prerequisite_paths if not path.is_file()]
    if missing_prerequisites:
        raise ValueError(
            "indicative report is missing prerequisites: "
            + ", ".join(missing_prerequisites)
        )
    storage_mode = reproduction_value(command_path, "exploratory_storage_mode")
    if storage_mode not in policy["allowed_storage_modes"]:
        raise ValueError("run used a storage mode outside the indicative policy")
    targets = reproduction_value(command_path, "targets").split(",")
    if targets != policy["required_targets"]:
        raise ValueError("run targets differ from the indicative policy")
    commit = source_commit(args.source_root.resolve() if args.source_root else root)
    if len(commit) != 40 or any(character not in "0123456789abcdef" for character in commit):
        raise ValueError("indicative report requires an exact Git source commit")
    receipt = {
        "schema_version": 1,
        "stage": "indicative-exploratory-report",
        "policy_id": policy["policy_id"],
        "policy_sha256": sha256(policy_path),
        "evidence_class": policy["evidence_class"],
        "claim_scope": policy["claim_scope"],
        "source_commit": commit,
        "storage_mode": storage_mode,
        "input_sha256": {
            "hardware_validation": sha256(hardware_path),
            "reproduction_command": sha256(command_path),
            "telemetry_summary": sha256(summary_path),
        },
        "authoritative": False,
        "capacity_bearing": False,
        "qualification_evidence": False,
        "bootstrap_evidence": False,
        "ship_evidence_eligible": False,
    }
    (root / "indicative-receipt.json").write_text(
        json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
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
        "source_commit": commit,
        "evidence_class": policy["evidence_class"],
        "policy_id": policy["policy_id"],
        "policy_sha256": sha256(policy_path),
        "authoritative": False,
        "capacity_bearing": False,
        "ship_evidence_eligible": False,
        "files": files,
    }
    (root / "artifact-manifest.json").write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    hardware = hardware_path.read_text(encoding="utf-8") if hardware_path.exists() else "unavailable\n"
    summary = summary_path.read_text(encoding="utf-8") if summary_path.exists() else "{}\n"
    command = command_path.read_text(encoding="utf-8") if command_path.exists() else "unavailable\n"
    lines = [
        "# Relative eight-case telemetry report",
        "",
        "> Indicative exploratory data only. This report is not authoritative, capacity-bearing, qualification/bootstrap evidence, production sizing guidance, or ship evidence.",
        "",
        "- Generated (UTC): " + manifest["generated_utc"],
        "- Source commit: " + manifest["source_commit"],
        "- Targets: HydraCache, Redis, Hazelcast Community",
        "- Workload: 8 cases x SET/GET x configured repeats",
        "- Sampling interval: 1 second by default",
        "- Evidence class: " + policy["evidence_class"],
        "- Storage mode: " + storage_mode,
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
    missing = [name for name in policy["required_artifacts"] if not (root / name).is_file()]
    if missing:
        raise ValueError("indicative report is missing required artifacts: " + ", ".join(missing))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
