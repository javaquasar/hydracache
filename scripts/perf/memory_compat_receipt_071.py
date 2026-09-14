#!/usr/bin/env python3
import argparse
import hashlib
import json
import pathlib
import re
from typing import Any

BASELINE_COMMIT = "75719b0bf5de2250cf4eb16a30073dd7429538e3"
REQUIRED_CHECKS = {
    "baseline-create-candidate-read-mutate-restart",
    "candidate-create-candidate-restart",
    "candidate-to-baseline-compatible-rollback",
    "rolling-baseline-candidate-all-role-orders",
    "snapshot-empty-max-record-crash-upgrade",
    "unknown-future-refuse-before-mutation-and-backup-restore",
    "hc1-hc2-versioned-wire-corpus-both-binaries",
}
SHA = re.compile(r"^[0-9a-f]{64}$")
COMMIT = re.compile(r"^[0-9a-f]{40}$")


def canonical_digest(receipt: dict[str, Any]) -> str:
    payload = dict(receipt)
    payload.pop("receipt_sha256", None)
    canonical = json.dumps(payload, sort_keys=True, separators=(",", ":")).encode()
    return hashlib.sha256(canonical).hexdigest()


def problems(
    receipt: dict[str, Any], campaign_id: str, candidate_sha: str, workflow_sha: str
) -> list[str]:
    found: list[str] = []
    expected = {
        "schema_version": 1,
        "release": "0.71",
        "campaign_id": campaign_id,
        "baseline_tag": "v0.70.0",
        "baseline_commit": BASELINE_COMMIT,
        "candidate_sha": candidate_sha,
        "workflow_sha": workflow_sha,
        "result": "success",
    }
    for field, value in expected.items():
        if receipt.get(field) != value:
            found.append(f"compatibility receipt {field} mismatch")
    if not COMMIT.fullmatch(candidate_sha) or not COMMIT.fullmatch(workflow_sha):
        found.append("expected compatibility commit identity is invalid")
    checks = receipt.get("checks")
    if not isinstance(checks, list) or set(checks) != REQUIRED_CHECKS or len(checks) != len(REQUIRED_CHECKS):
        found.append("compatibility receipt has incomplete or duplicate checks")
    binaries = receipt.get("binary_sha256")
    if not isinstance(binaries, dict) or any(
        not isinstance(binaries.get(name), str) or not SHA.fullmatch(binaries[name])
        for name in ("baseline", "candidate")
    ):
        found.append("compatibility receipt has invalid binary digests")
    if binaries and binaries.get("baseline") == binaries.get("candidate"):
        found.append("compatibility receipt used identical baseline and candidate binaries")
    if not isinstance(receipt.get("driver_sha256"), str) or not SHA.fullmatch(
        receipt["driver_sha256"]
    ):
        found.append("compatibility receipt has invalid driver digest")
    if receipt.get("receipt_sha256") != canonical_digest(receipt):
        found.append("compatibility receipt seal is invalid")
    return found


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", required=True, type=pathlib.Path)
    parser.add_argument("--campaign-id", required=True)
    parser.add_argument("--candidate-sha", required=True)
    parser.add_argument("--workflow-sha", required=True)
    args = parser.parse_args()
    receipt = json.loads(args.receipt.read_text(encoding="utf-8"))
    found = problems(receipt, args.campaign_id, args.candidate_sha, args.workflow_sha)
    if found:
        raise SystemExit("\n".join(found))
    print(f"memory compatibility receipt: OK ({args.receipt})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
