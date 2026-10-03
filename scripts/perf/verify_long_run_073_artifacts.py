#!/usr/bin/env python3
"""Independently verify the two-stage HydraCache 0.73 confirmation artifacts."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import tempfile
import zipfile
from datetime import datetime, timezone

import performance_long_run_073 as long_run


RUN_ID = 36839197349
TOOLING_SHA = "aa853c6abc37b51974a0d45a56b13d9d8c1fcb5f"
I73_SHA = "e757556d3a31d565f52a9561d6d4e555bb1cc373"
C73_SHA = "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
OPERATIONS = 1_036_800_000
EXPECTED_SURFACES = {
    "hc2": 362_880_000,
    "resp": 311_040_000,
    "hc1": 155_520_000,
    "direct": 103_680_000,
    "tag_invalidation": 51_840_000,
    "ttl_expire_refill": 51_840_000,
}
SUM_LINE = re.compile(r"^([0-9a-f]{64})  ([^\\\r\n]+)$")
SECRET_MARKERS = [
    b"ghp_",
    b"github_pat_",
    b"Authorization:",
    b"Bearer ",
    b"BEGIN PRIVATE KEY",
    b"BEGIN OPENSSH PRIVATE KEY",
    b"password=",
    b"token=",
]


def sha256_file(path: pathlib.Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def safe_extract(archive_path: pathlib.Path, destination: pathlib.Path) -> set[str]:
    names: set[str] = set()
    with zipfile.ZipFile(archive_path) as archive:
        if archive.testzip() is not None:
            raise ValueError(f"corrupt ZIP member in {archive_path}")
        for member in archive.infolist():
            member_path = pathlib.PurePosixPath(member.filename)
            if (
                member.is_dir()
                or member_path.is_absolute()
                or ".." in member_path.parts
                or member.filename in names
            ):
                raise ValueError(f"unsafe or duplicate ZIP member: {member.filename}")
            names.add(member.filename)
        archive.extractall(destination)
    return names


def continuation_entries(root: pathlib.Path) -> dict[str, str]:
    manifest = root / "continuation-SHA256SUMS"
    entries: dict[str, str] = {}
    for line in manifest.read_text(encoding="utf-8").splitlines():
        match = SUM_LINE.fullmatch(line)
        if match is None:
            raise ValueError(f"invalid continuation hash line: {line!r}")
        digest, relative = match.groups()
        relative_path = pathlib.PurePosixPath(relative)
        if relative_path.is_absolute() or ".." in relative_path.parts or relative in entries:
            raise ValueError(f"unsafe or duplicate continuation member: {relative}")
        path = root.joinpath(*relative_path.parts)
        if not path.is_file() or sha256_file(path) != digest:
            raise ValueError(f"continuation hash mismatch: {relative}")
        entries[relative] = digest
    if not entries:
        raise ValueError("empty continuation manifest")
    return entries


def verify_receipt(receipt: dict, role: str, source_sha: str) -> None:
    expected = {
        "release": "0.73",
        "profile_id": "integrated-long-run-073-v1",
        "role": role,
        "source_sha": source_sha,
        "offered_rate_per_second": 12_000,
        "operations": OPERATIONS,
        "warmup_operations": 5_000,
        "checkpoint_interval_seconds": 60,
        "post_work_idle_seconds": 300,
        "final_checkpoint_present": True,
        "reconciliation_exact": True,
        "management_truth_zero": True,
    }
    for field, value in expected.items():
        if receipt.get(field) != value:
            raise ValueError(f"{role} receipt {field} mismatch")
    observation = receipt.get("observation", {})
    for field in ["offered", "started", "completed", "successes"]:
        if observation.get(field) != OPERATIONS:
            raise ValueError(f"{role} receipt observation.{field} mismatch")
    for field in ["errors", "timeouts", "rejections"]:
        if observation.get(field) != 0:
            raise ValueError(f"{role} receipt observation.{field} is non-zero")
    if observation.get("backlog_drained") is not True:
        raise ValueError(f"{role} backlog did not drain")
    surfaces = receipt.get("surfaces", {})
    for surface, attempted in EXPECTED_SURFACES.items():
        counters = surfaces.get(surface, {})
        if counters.get("attempted") != attempted or counters.get("success") != attempted:
            raise ValueError(f"{role} surface accounting mismatch: {surface}")
        for field in ["rejected", "timeout", "late", "incomplete"]:
            if counters.get(field) != 0:
                raise ValueError(f"{role} surface {surface}.{field} is non-zero")
    if sum(item["attempted"] for item in surfaces.values()) != OPERATIONS:
        raise ValueError(f"{role} surface total mismatch")
    if receipt.get("events_received") != EXPECTED_SURFACES["hc2"]:
        raise ValueError(f"{role} event reconciliation mismatch")
    if receipt.get("checkpoint_count", 0) < 1_440:
        raise ValueError(f"{role} checkpoint count is below the contract")
    durable = receipt.get("durable", {})
    if durable.get("attempted") != 1_000 or durable.get("success") != 1_000:
        raise ValueError(f"{role} durable accounting mismatch")
    if durable.get("reopen_verified") is not True or durable.get("corruption_rejected") is not True:
        raise ValueError(f"{role} durable safety checks failed")


def tree_digest(root: pathlib.Path) -> tuple[str, int]:
    digest = hashlib.sha256()
    files = sorted(path for path in root.rglob("*") if path.is_file())
    for path in files:
        relative = path.relative_to(root).as_posix()
        digest.update(relative.encode("utf-8"))
        digest.update(b"\0")
        digest.update(bytes.fromhex(sha256_file(path)))
    return digest.hexdigest(), len(files)


def reject_secret_bearing_files(root: pathlib.Path) -> int:
    files = [path for path in root.rglob("*") if path.is_file()]
    for path in files:
        content = path.read_bytes()
        for marker in SECRET_MARKERS:
            if marker in content:
                raise ValueError(f"secret-like marker in {path.relative_to(root).as_posix()}")
    return len(files)


def toml_string(value: str) -> str:
    return json.dumps(value, ensure_ascii=False)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--final-zip", type=pathlib.Path, required=True)
    parser.add_argument("--i73-stage-zip", type=pathlib.Path, required=True)
    parser.add_argument("--final-provider-sha256", required=True)
    parser.add_argument("--i73-provider-sha256", required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    options = parser.parse_args()

    outer = {
        "final": sha256_file(options.final_zip),
        "i73_stage": sha256_file(options.i73_stage_zip),
    }
    expected_outer = {
        "final": options.final_provider_sha256.removeprefix("sha256:").lower(),
        "i73_stage": options.i73_provider_sha256.removeprefix("sha256:").lower(),
    }
    if outer != expected_outer:
        raise ValueError("provider download SHA-256 mismatch")

    with tempfile.TemporaryDirectory(prefix="hydracache-073-verify-") as temporary:
        temporary_root = pathlib.Path(temporary)
        final_root = temporary_root / "final"
        stage_root = temporary_root / "i73-stage"
        final_root.mkdir()
        stage_root.mkdir()
        final_members = safe_extract(options.final_zip, final_root)
        stage_members = safe_extract(options.i73_stage_zip, stage_root)
        final_entries = continuation_entries(final_root)
        stage_entries = continuation_entries(stage_root)
        if final_entries != stage_entries:
            raise ValueError("I73 continuation manifests differ")
        if stage_members != set(stage_entries) | {"continuation-SHA256SUMS"}:
            raise ValueError("I73 stage contains unsealed or missing files")
        for relative in stage_entries:
            final_path = final_root.joinpath(*pathlib.PurePosixPath(relative).parts)
            stage_path = stage_root.joinpath(*pathlib.PurePosixPath(relative).parts)
            if final_path.read_bytes() != stage_path.read_bytes():
                raise ValueError(f"I73 continuation handoff changed {relative}")
        if final_members != stage_members | {
            "c73/attempt.json",
            "c73/checkpoints.jsonl",
            "c73/receipt.json",
            "c73/stderr.txt",
            "c73/stdout.txt",
            "calibration/pre-c73.json",
            "calibration/post-c73.json",
            "long-run-campaign.json",
        }:
            raise ValueError("final archive is missing required members")
        secret_scan_file_count = reject_secret_bearing_files(final_root)
        reject_secret_bearing_files(stage_root)

        campaign = json.loads((final_root / "long-run-campaign.json").read_text(encoding="utf-8"))
        expected_campaign = {
            "release": "0.73",
            "profile_id": "integrated-long-run-073-v1",
            "phase": "confirmation",
            "mode": "host",
            "result": "passed",
            "tooling_sha": TOOLING_SHA,
            "baseline_source_sha": I73_SHA,
            "candidate_source_sha": C73_SHA,
            "role_order": ["I73", "C73"],
            "automatic_retry_allowed": False,
            "performance_claim_allowed": False,
            "confirmation_allowed": False,
            "final_c73_allowed": True,
        }
        for field, value in expected_campaign.items():
            if campaign.get(field) != value:
                raise ValueError(f"campaign {field} mismatch")
        if campaign.get("regression_budgets") != long_run.REGRESSION_BUDGETS:
            raise ValueError("campaign regression budgets mismatch")
        if campaign.get("identity", {}).get("runner_sha256") != sha256_file(pathlib.Path(long_run.__file__)):
            raise ValueError("runner identity mismatch")
        scenario = pathlib.Path(__file__).resolve().parents[2] / "docs/testing/performance/0.73/w10-long-run-qualification-v2-contract.toml"
        if campaign.get("identity", {}).get("scenario_sha256") != sha256_file(scenario):
            raise ValueError("scenario identity mismatch")

        canary = json.loads((final_root / "canary/canary.json").read_text(encoding="utf-8"))
        if not (
            canary.get("result") == "passed"
            and canary.get("marker_observed") is True
            and canary.get("receipt_absent") is True
            and canary.get("expected_rejection") == "missing-post-idle-reconciled-checkpoint"
            and canary.get("rejection") == "checkpoint series is incomplete"
        ):
            raise ValueError("negative canary did not fail for the intended defect")

        analyses: dict[str, dict] = {}
        for role, source_sha in [("I73", I73_SHA), ("C73", C73_SHA)]:
            role_key = role.lower()
            role_root = final_root / role_key
            attempt = json.loads((role_root / "attempt.json").read_text(encoding="utf-8"))
            if not (
                attempt.get("role") == role
                and attempt.get("valid") is True
                and attempt.get("attempt", {}).get("exit_code") == 0
                and attempt.get("attempt", {}).get("timed_out") is False
            ):
                raise ValueError(f"{role} attempt did not complete cleanly")
            for stream in ["stdout", "stderr"]:
                if sha256_file(role_root / f"{stream}.txt") != attempt["attempt"][f"{stream}_sha256"]:
                    raise ValueError(f"{role} {stream} hash mismatch")
            receipt = json.loads((role_root / "receipt.json").read_text(encoding="utf-8"))
            verify_receipt(receipt, role, source_sha)
            analysis = long_run.analyze_role_packet(
                role_root,
                role,
                OPERATIONS,
                int(long_run.PHASES["confirmation"]["checkpoints"]) - 1,
                host_mode=True,
            )
            embedded_analysis = attempt.get("analysis", {})
            campaign_analysis = campaign["roles"][role]["analysis"]
            comparable = {key: value for key, value in analysis.items() if key != "artifact_bytes"}
            embedded_comparable = {
                key: value for key, value in embedded_analysis.items() if key != "artifact_bytes"
            }
            campaign_comparable = {
                key: value for key, value in campaign_analysis.items() if key != "artifact_bytes"
            }
            # The producer measures artifact_bytes immediately before writing
            # attempt.json. Reanalysis necessarily includes that final file, so
            # compare every semantic/statistical field exactly and check the
            # byte delta independently.
            expected_current_bytes = embedded_analysis.get("artifact_bytes", -1) + (
                role_root / "attempt.json"
            ).stat().st_size
            if role == "I73":
                expected_current_bytes += (role_root / "baseline-bounds.json").stat().st_size
            if (
                comparable != embedded_comparable
                or comparable != campaign_comparable
                or analysis["artifact_bytes"] != expected_current_bytes
                or analysis["artifact_bytes"] > long_run.ROLE_ARTIFACT_LIMIT
            ):
                raise ValueError(f"{role} independent reanalysis mismatch")
            analyses[role] = analysis

        bounds = json.loads((final_root / "i73/baseline-bounds.json").read_text(encoding="utf-8"))
        if not (
            bounds.get("sealed_before_candidate") is True
            and bounds.get("source_sha") == I73_SHA
            and bounds.get("resource_bounds") == analyses["I73"]["resource_bounds"]
        ):
            raise ValueError("sealed I73 bounds mismatch")
        calibrations = long_run.validate_calibrations(final_root, TOOLING_SHA)
        if calibrations != campaign.get("calibration"):
            raise ValueError("calibration reanalysis mismatch")
        guards = long_run.comparison_guards(analyses["I73"], analyses["C73"], include_slopes=True)
        if guards != campaign.get("guards") or not all(guards.values()):
            raise ValueError("comparison guards failed independent reanalysis")
        final_tree_sha256, final_file_count = tree_digest(final_root)
        stage_tree_sha256, stage_file_count = tree_digest(stage_root)

    output = "\n".join(
        [
            "schema_version = 1",
            'release = "0.73.0"',
            f"source_run = {RUN_ID}",
            'result = "passed"',
            f"verified_at = {toml_string(datetime.now(timezone.utc).isoformat())}",
            f"tooling_sha = {toml_string(TOOLING_SHA)}",
            f"baseline_source_sha = {toml_string(I73_SHA)}",
            f"candidate_source_sha = {toml_string(C73_SHA)}",
            'provider_download_sha256 = "passed"',
            'nested_sha256 = "passed"',
            'i73_c73_continuation_handoff = "passed"',
            'identity_guards = "passed"',
            'workload_guards = "passed"',
            'duration_guards = "passed"',
            'estimator_and_threshold_guards = "passed"',
            'secret_and_path_allowlist = "passed"',
            f"final_provider_sha256 = {toml_string(outer['final'])}",
            f"i73_stage_provider_sha256 = {toml_string(outer['i73_stage'])}",
            f"final_tree_sha256 = {toml_string(final_tree_sha256)}",
            f"i73_stage_tree_sha256 = {toml_string(stage_tree_sha256)}",
            f"final_file_count = {final_file_count}",
            f"i73_stage_file_count = {stage_file_count}",
            f"continuation_file_count = {len(final_entries)}",
            f"secret_scan_file_count = {secret_scan_file_count}",
            f"i73_operations = {OPERATIONS}",
            f"c73_operations = {OPERATIONS}",
            f"i73_checkpoint_count = {analyses['I73']['checkpoint_count']}",
            f"c73_checkpoint_count = {analyses['C73']['checkpoint_count']}",
            f"i73_goodput = {analyses['I73']['goodput']:.12f}",
            f"c73_goodput = {analyses['C73']['goodput']:.12f}",
            f"i73_cpu_seconds_per_operation = {analyses['I73']['cpu_per_operation']:.15f}",
            f"c73_cpu_seconds_per_operation = {analyses['C73']['cpu_per_operation']:.15f}",
            f"i73_p99_us = {analyses['I73']['p99']:.0f}",
            f"c73_p99_us = {analyses['C73']['p99']:.0f}",
            f"i73_rss_upper_95_bytes_per_second = {analyses['I73']['resource_bounds']['rss']['upper_95_bytes_per_second']:.12f}",
            f"c73_rss_upper_95_bytes_per_second = {analyses['C73']['resource_bounds']['rss']['upper_95_bytes_per_second']:.12f}",
            f"i73_anonymous_pss_upper_95_bytes_per_second = {analyses['I73']['resource_bounds']['anonymous_pss']['upper_95_bytes_per_second']:.12f}",
            f"c73_anonymous_pss_upper_95_bytes_per_second = {analyses['C73']['resource_bounds']['anonymous_pss']['upper_95_bytes_per_second']:.12f}",
            "",
        ]
    )
    options.output.parent.mkdir(parents=True, exist_ok=True)
    options.output.write_text(output, encoding="utf-8")
    print(f"verified release 0.73 confirmation run {RUN_ID}: all guards passed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
