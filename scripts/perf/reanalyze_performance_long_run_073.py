#!/usr/bin/env python3
"""Reanalyze the retained 0.73 long-run packet without rerunning product work."""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

import performance_long_run_073 as runner


ORIGINAL_CAMPAIGN_SHA256 = "21050af401193774e20564af989bed27a6af2ea4e2f2f8ba367abe05b8153470"
ORIGINAL_TOOLING_SHA = "8f8d44570eb984a37f38a6b8dafeaebbd54d3d11"
EXPECTED_SOURCES = {"I73": runner.I73_SHA, "C73": runner.C73_SHA}
EXPECTED_OPERATIONS = 259_200_000
MINIMUM_PERIODIC_CHECKPOINTS = 359


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def reanalyze(packet: pathlib.Path) -> dict:
    campaign_path = packet / "long-run-campaign.json"
    require(campaign_path.is_file(), "original campaign is missing")
    require(
        runner.sha256_file(campaign_path) == ORIGINAL_CAMPAIGN_SHA256,
        "original campaign digest changed",
    )
    original = json.loads(campaign_path.read_text(encoding="utf-8"))
    require(original.get("result") == "failed", "original campaign result changed")
    require(original.get("tooling_sha") == ORIGINAL_TOOLING_SHA, "original tooling changed")
    require(original.get("role_order") == ["I73", "C73"], "original role order changed")
    require(original.get("automatic_retry_allowed") is False, "retry boundary changed")
    require(original.get("confirmation_allowed") is False, "confirmation boundary changed")
    require(original.get("baseline_source_sha") == runner.I73_SHA, "I73 identity changed")
    require(original.get("candidate_source_sha") == runner.C73_SHA, "C73 identity changed")

    analyses = {}
    nested_sha256 = {}
    for role in ["I73", "C73"]:
        role_key = role.lower()
        role_dir = packet / role_key
        original_role = original["roles"][role]
        require(original_role.get("valid") is True, f"{role} was not originally valid")
        attempt = json.loads((role_dir / "attempt.json").read_text(encoding="utf-8"))
        require(attempt.get("valid") is True, f"{role} attempt changed")
        require(
            attempt.get("attempt", {}).get("exit_code") == 0
            and attempt.get("attempt", {}).get("timed_out") is False,
            f"{role} process outcome changed",
        )
        for stream in ["stdout", "stderr"]:
            digest = runner.sha256_file(role_dir / f"{stream}.txt")
            require(
                digest == attempt["attempt"][f"{stream}_sha256"],
                f"{role} {stream} digest changed",
            )
            nested_sha256[f"{role_key}_{stream}"] = digest

        analysis = runner.analyze_role_packet(
            role_dir,
            role,
            EXPECTED_OPERATIONS,
            MINIMUM_PERIODIC_CHECKPOINTS,
            host_mode=True,
        )
        require(
            analysis["receipt_sha256"] == original_role["analysis"]["receipt_sha256"],
            f"{role} receipt digest changed",
        )
        require(
            analysis["checkpoints_sha256"]
            == original_role["analysis"]["checkpoints_sha256"],
            f"{role} checkpoint digest changed",
        )
        original_role_bytes = sum(
            (role_dir / name).stat().st_size
            for name in ["receipt.json", "checkpoints.jsonl", "stdout.txt", "stderr.txt"]
        )
        require(
            original_role_bytes == original_role["analysis"]["artifact_bytes"],
            f"{role} original artifact size changed",
        )
        analysis["artifact_bytes"] = original_role["analysis"]["artifact_bytes"]
        receipt = json.loads((role_dir / "receipt.json").read_text(encoding="utf-8"))
        require(receipt.get("source_sha") == EXPECTED_SOURCES[role], f"{role} source changed")
        analyses[role] = analysis
        nested_sha256[f"{role_key}_receipt"] = analysis["receipt_sha256"]
        nested_sha256[f"{role_key}_checkpoints"] = analysis["checkpoints_sha256"]

    for name, expected in original["calibration"]["sha256"].items():
        digest = runner.sha256_file(packet / "calibration" / f"{name}.json")
        require(digest == expected, f"{name} calibration digest changed")
        nested_sha256[f"calibration_{name}"] = digest

    guards = runner.comparison_guards(analyses["I73"], analyses["C73"], include_slopes=True)
    require(guards["goodput"], "goodput guard unexpectedly changed")
    require(guards["cpu_per_operation"], "CPU guard unexpectedly changed")
    require(guards["p99"], "p99 guard unexpectedly changed")
    return {
        "schema_version": 1,
        "release": "0.73",
        "evidence_id": "w10-long-run-analyzer-correction-36278780653-v1",
        "state": "offline-reanalysis-passed-awaiting-d4-review",
        "workflow_run": 36_278_780_653,
        "original_campaign_sha256": ORIGINAL_CAMPAIGN_SHA256,
        "original_tooling_sha": ORIGINAL_TOOLING_SHA,
        "baseline_source_sha": runner.I73_SHA,
        "candidate_source_sha": runner.C73_SHA,
        "analyzer_sha256": runner.sha256_file(pathlib.Path(runner.__file__).resolve()),
        "bootstrap_method": "moving-block-adjacent-slope-v1",
        "nested_sha256_verified": True,
        "nested_sha256": nested_sha256,
        "roles": analyses,
        "guards": guards,
        "result": "passed" if all(guards.values()) else "failed",
        "rerun_performed": False,
        "raw_observations_changed": False,
        "workload_changed": False,
        "thresholds_changed": False,
        "identities_changed": False,
        "performance_claim_allowed": False,
        "confirmation_allowed": False,
        "decision": "retain-original-failure-and-review-analyzer-correction-at-d4",
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--packet", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    options = parser.parse_args()
    output = options.output.resolve()
    if output.exists():
        raise SystemExit(f"output already exists: {output}")
    try:
        result = reanalyze(options.packet.resolve())
    except (KeyError, OSError, TypeError, ValueError, json.JSONDecodeError) as error:
        print(f"long-run reanalysis rejected input: {error}", file=sys.stderr)
        return 2
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0 if result["result"] == "passed" else 1


if __name__ == "__main__":
    raise SystemExit(main())
