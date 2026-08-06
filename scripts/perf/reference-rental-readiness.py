#!/usr/bin/env python3
"""Create a non-evidence operator admission receipt before renting bare metal."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
from datetime import datetime, timezone
from decimal import Decimal, InvalidOperation
from pathlib import Path


GIT_SHA_RE = re.compile(r"^[0-9a-f]{40}$")
GITHUB_RUN_RE = re.compile(
    r"^https://github\.com/[^/]+/[^/]+/actions/runs/(?P<run_id>[0-9]+)$"
)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def git(repo: Path, *args: str) -> str:
    return subprocess.check_output(
        ["git", "-C", str(repo), *args], text=True, stderr=subprocess.DEVNULL
    ).strip()


def nonempty(name: str, value: str) -> str:
    value = value.strip()
    if not value:
        raise ValueError(f"{name} must not be empty")
    return value


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--policy", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--rental-operator", required=True)
    parser.add_argument("--billing-owner", required=True)
    parser.add_argument("--deletion-owner", required=True)
    parser.add_argument("--provider", required=True)
    parser.add_argument("--sku", required=True)
    parser.add_argument("--hourly-rate-eur", required=True)
    parser.add_argument("--authorized-hours", type=int, required=True)
    parser.add_argument("--main-sha", required=True)
    parser.add_argument("--main-ci-run-url", required=True)
    parser.add_argument("--decision-reference", required=True)
    parser.add_argument("--approve", action="store_true")
    args = parser.parse_args()

    repo = Path(git(Path.cwd(), "rev-parse", "--show-toplevel"))
    policy_path = (
        args.policy
        if args.policy
        else repo / "docs/testing/perf-procurement/0.67.1.json"
    ).resolve()
    policy = json.loads(policy_path.read_text(encoding="utf-8"))
    if policy.get("schema_version") != 1 or policy.get("release") != "0.67.1":
        raise ValueError("unsupported procurement policy")
    if not args.approve:
        raise ValueError("rental admission requires the explicit --approve decision")
    if args.authorized_hours < 1 or args.authorized_hours > int(
        policy["maximum_authorized_hours"]
    ):
        raise ValueError("authorized hours exceed the reviewed policy boundary")
    try:
        hourly_rate = Decimal(args.hourly_rate_eur)
    except InvalidOperation as error:
        raise ValueError("hourly rate must be a decimal number") from error
    if hourly_rate <= 0:
        raise ValueError("hourly rate must be positive")
    main_sha = args.main_sha.lower()
    if not GIT_SHA_RE.fullmatch(main_sha):
        raise ValueError("main SHA must be 40 lowercase hexadecimal characters")
    if main_sha != git(repo, "rev-parse", "origin/main"):
        raise ValueError("main SHA differs from the locally fetched origin/main")
    if main_sha != git(repo, "rev-parse", "HEAD"):
        raise ValueError("rental admission must be created from exact origin/main")
    if git(repo, "status", "--porcelain=v1", "--untracked-files=all"):
        raise ValueError("rental admission requires a clean exact-main checkout")
    run_match = GITHUB_RUN_RE.fullmatch(args.main_ci_run_url)
    if not run_match:
        raise ValueError("main CI run URL must identify one GitHub Actions run")
    run = json.loads(
        subprocess.check_output(
            [
                "gh",
                "run",
                "view",
                run_match.group("run_id"),
                "--json",
                "status,conclusion,headSha,url",
            ],
            cwd=repo,
            text=True,
        )
    )
    if (
        run.get("status") != "completed"
        or run.get("conclusion") != "success"
        or run.get("headSha") != main_sha
        or run.get("url") != args.main_ci_run_url
    ):
        raise ValueError("main CI run is not a successful exact-main run")
    if args.output.exists():
        raise FileExistsError(f"refusing to overwrite rental admission: {args.output}")

    receipt = {
        "schema_version": 1,
        "stage": "reference-rental-procurement-admission",
        "release": "0.67.1",
        "policy_id": policy["policy_id"],
        "policy_sha256": sha256(policy_path),
        "decision": "approve",
        "decision_reference": nonempty("decision reference", args.decision_reference),
        "approved_at": datetime.now(timezone.utc).isoformat(),
        "rental_operator": nonempty("rental operator", args.rental_operator),
        "billing_owner": nonempty("billing owner", args.billing_owner),
        "deletion_owner": nonempty("deletion owner", args.deletion_owner),
        "provider": nonempty("provider", args.provider),
        "resource_kind": policy["required_resource_kind"],
        "sku": nonempty("SKU", args.sku),
        "operating_system": policy["required_operating_system"],
        "hourly_rate_eur": str(hourly_rate),
        "authorized_hours": args.authorized_hours,
        "maximum_authorized_cost_eur": str(hourly_rate * args.authorized_hours),
        "main_sha": main_sha,
        "main_ci_run_url": args.main_ci_run_url,
        "main_ci_conclusion": run["conclusion"],
        "delete_resource_to_stop_billing": policy[
            "provider_resource_must_be_deleted_to_stop_billing"
        ],
        "contains_provider_credentials": False,
        "qualification_evidence": False,
        "bootstrap_evidence": False,
        "ship_evidence_eligible": False,
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open("x", encoding="utf-8") as stream:
        stream.write(json.dumps(receipt, indent=2, sort_keys=True) + "\n")
    print(f"reference rental admission created: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
