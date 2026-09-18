#!/usr/bin/env python3
"""Generate the bounded 0.71 release claim set from accepted D4 evidence."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import tomllib


EXPECTED_ROWS = {"M3": ("M3-ttl", 10), "M8": ("M8-60m", 8),
                 "M9": ("M9-6h", 2), "M10": ("M10-24h", 2)}
ALLOWED_DISPOSITIONS = {"deferred", "measured-no-win", "not-applicable"}


def derive_claims(policy: dict, d3: dict, manifest: dict, receipts: dict[str, dict], acceptance: dict) -> dict:
    if policy.get("release") != "0.71" or manifest.get("release") != "0.71":
        raise ValueError("release identity mismatch")
    if d3.get("release") != "0.71" or d3.get("result") != "accepted-for-exact-candidate-qualification":
        raise ValueError("D3 decision is not accepted")
    if d3.get("proposals") is None or any(p.get("numerical_claim_authorized") for p in d3["proposals"]):
        raise ValueError("unexpected numerical proposal; review a new claims contract")
    optional = policy.get("optional_work", [])
    if not optional or any(item.get("disposition") not in ALLOWED_DISPOSITIONS for item in optional):
        raise ValueError("optional work has an unreviewed disposition")
    if any(item.get("disposition") == "deferred" and not item.get("reason") for item in optional):
        raise ValueError("deferred work lacks a reason")
    source_sha = manifest.get("source_sha")
    if not isinstance(source_sha, str) or len(source_sha) != 40 or source_sha != manifest.get("workflow_sha"):
        raise ValueError("invalid source/workflow identity")
    campaigns = manifest.get("campaigns", [])
    if len(campaigns) != len(EXPECTED_ROWS) or {item.get("stage") for item in campaigns} != set(EXPECTED_ROWS):
        raise ValueError("incomplete D4 stage set")
    if (acceptance.get("release") != "0.71"
            or acceptance.get("measured_source_sha") != source_sha
            or acceptance.get("workflow_sha") != source_sha
            or acceptance.get("evidence_branch") != "evidence/0.71/ax42/d4"
            or not isinstance(acceptance.get("evidence_commit"), str)
            or len(acceptance["evidence_commit"]) != 40
            or acceptance.get("campaigns") != [
                {key: item[key] for key in ("stage", "id", "github_run_id", "github_artifact_id")}
                for item in campaigns
            ]):
        raise ValueError("D4 acceptance and evidence manifest disagree")
    if set(receipts) != {item["id"] for item in campaigns}:
        raise ValueError("missing or extra campaign receipt")
    for item in campaigns:
        receipt = receipts[item["id"]]
        case_id, jobs = EXPECTED_ROWS[item["stage"]]
        if (receipt.get("campaign_id") != item["id"]
                or receipt.get("case_ids") != [case_id]
                or receipt.get("source_sha") != source_sha
                or receipt.get("workflow_sha") != source_sha
                or receipt.get("result") != "success"
                or receipt.get("ship_evidence_eligible") is not True
                or receipt.get("job_count") != jobs
                or receipt.get("completed_jobs") != jobs):
            raise ValueError(f"unaccepted campaign receipt: {item['id']}")
    return {
        "schema_version": 1,
        "release": "0.71",
        "measured_source_sha": source_sha,
        "evidence_branch": "evidence/0.71/ax42/d4",
        "evidence_commit": acceptance["evidence_commit"],
        "campaign_ids": [item["id"] for item in campaigns],
        "numeric_memory_improvement_claims": [],
        "permitted_scope": [
            "retained-byte accounting on shipped surfaces",
            "explicit owner bounds and active-expiry cleanup",
            "exact-host D4 memory qualification for the measured source and workflow",
        ],
        "negative_result": "No optional numerical RSS, allocator, Redis, Hazelcast, or cross-host improvement is claimed.",
        "optional_dispositions": [
            {"id": item["id"], "disposition": item["disposition"],
             "reason": item["reason"], "next_evidence": item["next_evidence"]}
            for item in optional
        ],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--evidence-root", type=Path, required=True)
    parser.add_argument("--campaigns", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    policy = tomllib.loads((args.root / "docs/testing/memory/0.71/release-policy.toml").read_text(encoding="utf-8"))
    d3 = json.loads((args.root / "docs/testing/memory/0.71/d3-implementation-acceptance.json").read_text(encoding="utf-8"))
    acceptance = json.loads((args.root / "docs/testing/memory/0.71/d4-acceptance.json").read_text(encoding="utf-8"))
    manifest = json.loads((args.evidence_root / "manifest.json").read_text(encoding="utf-8"))
    receipts = {}
    for item in manifest["campaigns"]:
        path = args.campaigns / item["id"] / "campaign-receipt.json"
        receipts[item["id"]] = json.loads(path.read_text(encoding="utf-8"))
    claims = derive_claims(policy, d3, manifest, receipts, acceptance)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(claims, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"release claims: {args.output} (no numerical memory improvement claim)")


if __name__ == "__main__":
    main()
