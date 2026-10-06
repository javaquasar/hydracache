#!/usr/bin/env python3
"""Assemble the W11 protected-start fixture bundle without starting a process."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import pathlib
import re
import secrets
import stat
import subprocess
import tempfile
import time
import uuid
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parents[2]
FIXTURE_CONTRACT = (
    ROOT / "docs" / "testing" / "performance" / "0.74" / "non-product-start-fixture.toml"
)
CONTROLLER_CONTRACT = (
    ROOT
    / "docs"
    / "testing"
    / "performance"
    / "0.74"
    / "long-run-controller-resilience-contract.toml"
)
SCHEMAS = ROOT / "docs" / "testing" / "performance" / "0.74" / "schemas"
BUILDER = pathlib.Path(__file__).with_name("performance_long_run_074.py")
FIXTURE_PATH = "/opt/hydracache-performance/0.74/campaign-lifecycle-fixture"
OBSERVATION_FILES = {
    "evidence.json",
    "host-observation.json",
    "host-observation.sha256",
    "request.json",
    "response.json",
}
EVIDENCE_FIELDS = {
    "schema_version",
    "source_commit",
    "installed_source_commit",
    "installed_supervisor_binary_sha256",
    "installed_fixture_binary",
    "request_sha256",
    "response_sha256",
    "host_receipt_sha256",
    "peer_admission_required",
    "signed_authorization_used",
    "arbitrary_output_path_accepted",
    "campaign_state_mutated",
    "product_candidate_started",
    "promotable",
}
REQUEST_FIELDS = {
    "schema_version",
    "request_id",
    "operation",
    "campaign_id",
    "expected_state_revision",
    "manifest_path",
    "manifest_sha256",
    "controller",
    "abort_reason",
    "approval_nonce_sha256",
}
RESPONSE_FIELDS = {
    "schema_version",
    "request_id",
    "campaign_id",
    "ok",
    "state_revision",
    "server_time_unix_seconds",
    "result",
    "error_code",
    "response_sha256",
}
MAX_OBSERVATION_FILE_BYTES = 1024 * 1024


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def digest_file(path: pathlib.Path) -> str:
    return digest_bytes(path.read_bytes())


def domain_digest(label: str, fixture_contract: bytes) -> str:
    return digest_bytes(label.encode("ascii") + b"\0" + fixture_contract)


def read_safe_regular(path: pathlib.Path) -> bytes:
    metadata = path.lstat()
    if (
        not stat.S_ISREG(metadata.st_mode)
        or metadata.st_nlink != 1
        or metadata.st_size <= 0
        or metadata.st_size > MAX_OBSERVATION_FILE_BYTES
    ):
        raise ValueError(f"unsafe host observation file: {path.name}")
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        current = os.fstat(descriptor)
        if (
            current.st_dev != metadata.st_dev
            or current.st_ino != metadata.st_ino
            or current.st_size != metadata.st_size
        ):
            raise ValueError(f"host observation file changed while reading: {path.name}")
        value = os.read(descriptor, MAX_OBSERVATION_FILE_BYTES + 1)
    finally:
        os.close(descriptor)
    if len(value) != metadata.st_size:
        raise ValueError(f"host observation file size changed: {path.name}")
    return value


def strict_json_object(
    value: bytes, label: str, *, sort_keys: bool
) -> dict[str, Any]:
    encoded = value[:-1] if value.endswith(b"\n") else value

    def reject_duplicates(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
        result: dict[str, Any] = {}
        for key, item in pairs:
            if key in result:
                raise ValueError(f"{label} contains a duplicate key")
            result[key] = item
        return result

    parsed = json.loads(encoded, object_pairs_hook=reject_duplicates)
    rendered = json.dumps(
        parsed, sort_keys=sort_keys, separators=(",", ":")
    ).encode("utf-8")
    if not isinstance(parsed, dict) or rendered != encoded:
        raise ValueError(f"{label} is not one strict JSON object")
    return parsed


def load_builder():
    spec = importlib.util.spec_from_file_location("performance_long_run_074", BUILDER)
    if spec is None or spec.loader is None:
        raise ValueError("campaign builder module is unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load_observation(directory: pathlib.Path, source_sha: str) -> tuple[dict, dict]:
    if (
        not directory.is_dir()
        or directory.is_symlink()
        or {path.name for path in directory.iterdir()} != OBSERVATION_FILES
    ):
        raise ValueError("host observation artifact differs from the fixed five-file layout")
    evidence_bytes = read_safe_regular(directory / "evidence.json")
    request_bytes = read_safe_regular(directory / "request.json")
    response_bytes = read_safe_regular(directory / "response.json")
    receipt_bytes = read_safe_regular(directory / "host-observation.json")
    evidence = strict_json_object(evidence_bytes, "evidence.json", sort_keys=True)
    request = strict_json_object(request_bytes, "request.json", sort_keys=True)
    response = strict_json_object(response_bytes, "response.json", sort_keys=False)
    receipt = strict_json_object(receipt_bytes, "host-observation.json", sort_keys=True)
    canonical_receipt = canonical_json(receipt)
    response_body = {key: value for key, value in response.items() if key != "response_sha256"}
    result = response.get("result", {})
    fixture = evidence.get("installed_fixture_binary")
    request_body = request.get("request", {})
    if (
        not re.fullmatch(r"[0-9a-f]{40}", source_sha)
        or not isinstance(evidence, dict)
        or set(evidence) != EVIDENCE_FIELDS
        or evidence.get("schema_version") != "hydracache-w11-host-observation-socket-v1"
        or not isinstance(request, dict)
        or set(request) != {"request", "authorization"}
        or request.get("authorization") is not None
        or not isinstance(request_body, dict)
        or set(request_body) != REQUEST_FIELDS
        or not isinstance(response, dict)
        or set(response) != RESPONSE_FIELDS
        or not isinstance(result, dict)
        or set(result)
        != {
            "schema_version",
            "installed_source_commit",
            "fixture_binary",
            "receipt_sha256",
            "receipt",
        }
        or evidence.get("source_commit") != source_sha
        or evidence.get("installed_source_commit") != source_sha
        or evidence.get("request_sha256") != digest_bytes(request_bytes)
        or evidence.get("response_sha256") != digest_bytes(canonical_json(response_body))
        or response.get("response_sha256") != evidence.get("response_sha256")
        or evidence.get("host_receipt_sha256") != digest_bytes(canonical_receipt)
        or result.get("receipt_sha256") != evidence.get("host_receipt_sha256")
        or result.get("receipt") != receipt
        or result.get("installed_source_commit") != source_sha
        or result.get("fixture_binary") != fixture
        or response.get("request_id") != request_body.get("request_id")
        or response.get("campaign_id") != request_body.get("campaign_id")
        or response.get("ok") is not True
        or response.get("state_revision") != 0
        or response.get("error_code") is not None
        or result.get("schema_version") != 1
        or request_body.get("operation") != "host_observation"
        or request_body.get("expected_state_revision") != 0
        or request_body.get("manifest_path") is not None
        or evidence.get("peer_admission_required") is not True
        or evidence.get("signed_authorization_used") is not False
        or evidence.get("arbitrary_output_path_accepted") is not False
        or evidence.get("campaign_state_mutated") is not False
        or evidence.get("product_candidate_started") is not False
        or evidence.get("promotable") is not False
    ):
        raise ValueError("host observation is not an exact non-product bundle input")
    head = read_safe_regular(directory / "host-observation.sha256")
    if head != (evidence["host_receipt_sha256"] + "\n").encode("ascii"):
        raise ValueError("host observation digest sidecar differs")
    expected_fixture_fields = {
        "path",
        "sha256",
        "size",
        "inode",
        "device",
        "uid",
        "gid",
        "mode",
    }
    if (
        not isinstance(fixture, dict)
        or set(fixture) != expected_fixture_fields
        or fixture.get("path") != FIXTURE_PATH
        or not re.fullmatch(r"[0-9a-f]{64}", str(fixture.get("sha256", "")))
        or any(
            type(fixture.get(field)) is not int or fixture[field] <= 0
            for field in ["size", "inode", "device"]
        )
        or type(fixture.get("uid")) is not int
        or type(fixture.get("gid")) is not int
        or type(fixture.get("mode")) is not int
        or fixture.get("uid") != 0
        or fixture.get("gid") != 0
        or fixture.get("mode") != 0o755
    ):
        raise ValueError("fixed non-product fixture identity differs")
    return evidence, receipt


def source_tree(source_sha: str) -> str:
    head = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
    ).strip()
    dirty = subprocess.check_output(
        ["git", "status", "--porcelain=v1", "--untracked-files=normal"],
        cwd=ROOT,
        text=True,
    )
    if head != source_sha or dirty:
        raise ValueError("bundle assembler checkout is not the exact clean source commit")
    return subprocess.check_output(
        ["git", "rev-parse", "HEAD^{tree}"], cwd=ROOT, text=True
    ).strip()


def assemble(
    observation_directory: pathlib.Path,
    output: pathlib.Path,
    evidence_output: pathlib.Path,
    source_sha: str,
    repository_id: int,
    *,
    now_unix_seconds: int | None = None,
    lease_id: str | None = None,
    nonce_hex: str | None = None,
) -> dict[str, Any]:
    if output.exists() or evidence_output.exists():
        raise ValueError("output path already exists")
    if repository_id <= 0:
        raise ValueError("repository id must be positive")
    observation, receipt = load_observation(observation_directory, source_sha)
    fixture = observation["installed_fixture_binary"]
    fixture_contract = FIXTURE_CONTRACT.read_bytes()
    tree = source_tree(source_sha)
    lock_digest = digest_file(ROOT / "Cargo.lock")
    fixture_i74 = dict(fixture, role="i74")
    fixture_c74 = dict(fixture, role="c74")
    now = int(time.time()) if now_unix_seconds is None else now_unix_seconds
    values = {
        "schema_version": 1,
        "repository_id": repository_id,
        "authorization_identity": "performance-reference-074/non-product-start-v1",
        "contract_sha256": digest_file(CONTROLLER_CONTRACT),
        "tooling_sha": source_sha,
        "i74_source_sha": source_sha,
        "c74_source_sha": source_sha,
        "i74_tree_sha": tree,
        "c74_tree_sha": tree,
        "i74_cargo_lock_sha256": lock_digest,
        "c74_cargo_lock_sha256": lock_digest,
        "i74_dirty": False,
        "c74_dirty": False,
        "scenario_sha256": domain_digest("scenario", fixture_contract),
        "workload_sha256": domain_digest("workload", fixture_contract),
        "offered_load_sha256": domain_digest("offered-load", fixture_contract),
        "estimator_sha256": domain_digest("estimator", fixture_contract),
        "thresholds_sha256": domain_digest("thresholds", fixture_contract),
        "host_receipt_sha256": observation["host_receipt_sha256"],
        "lease_id": lease_id or str(uuid.uuid4()),
        "random_nonce_hex": nonce_hex or secrets.token_hex(32),
        "machine_id": receipt["machine_id"],
        "boot_id": receipt["boot_id"],
        "mount_identity": receipt["mount_identity"],
        "isolated_cpuset": receipt["isolated_cpuset"],
        "housekeeping_cpuset": receipt["housekeeping_cpuset"],
        "seed": 740074,
        "checkpoint_cadence_seconds": 30,
        "progress_warning_gap_seconds": 90,
        "progress_rejection_gap_seconds": 180,
        "diagnostic_grace_seconds": 30,
        "product_lease_deadline_unix_seconds": now + 3600,
        "maximum_campaign_bytes": 21_474_836_480,
        "maximum_campaign_files": 20_000,
        "installed_binaries": [fixture_i74, fixture_c74],
        "argv_templates": {
            "i74": [FIXTURE_PATH, "campaign-start-rehearsal-harness"],
            "c74": [FIXTURE_PATH, "campaign-start-rehearsal-harness"],
        },
        "command_environment_sha256": "0" * 64,
        "role_order": ["i74", "c74"],
        "phase_durations_seconds": {
            "warmup": 55,
            "measured": 55,
            "drain": 55,
            "durable_companion": 55,
            "post_work_idle": 55,
            "reconciliation": 55,
        },
        "output_limits": {
            "stdout_bytes": 1_048_576,
            "stderr_bytes": 1_048_576,
            "diagnostic_bytes": 1_048_576,
            "final_artifact_bytes": 1_048_576,
            "files": 100,
        },
        "expected_output_schema_sha256s": {
            "checkpoint": digest_file(SCHEMAS / "checkpoint-envelope.schema.json"),
            "measurement": domain_digest("measurement-schema", fixture_contract),
            "reconciliation": domain_digest("reconciliation-schema", fixture_contract),
            "raw_manifest": digest_file(SCHEMAS / "raw-manifest.schema.json"),
            "packet_manifest": digest_file(SCHEMAS / "campaign-packet-manifest.schema.json"),
        },
        "required_final_guards": [
            "non-product-protected-start-rehearsal-only",
            "product-candidate-started-false",
        ],
        "secret_identifiers": [],
    }
    builder = load_builder()
    campaign_id = builder.campaign_id(values)
    values["command_environment_sha256"] = builder.expected_command_environment_sha256(
        campaign_id, values["isolated_cpuset"], values["housekeeping_cpuset"]
    )
    manifest = builder.build_manifest(values)
    with tempfile.TemporaryDirectory(prefix="hydracache-074-non-product-manifest-") as temporary:
        manifest_directory = pathlib.Path(temporary) / "manifest"
        _, manifest_sha256 = builder.write_manifest(manifest_directory, manifest)
        _, bundle_sha256 = builder.assemble_start_bundle(
            manifest_directory, observation_directory, output
        )
    verified_campaign, verified_digest = builder.verify_start_bundle(output)
    if verified_campaign != campaign_id or verified_digest != bundle_sha256:
        raise ValueError("assembled start bundle did not independently re-verify")
    evidence = {
        "schema_version": "hydracache-w11-non-product-start-bundle-v1",
        "source_commit": source_sha,
        "campaign_id": campaign_id,
        "manifest_sha256": manifest_sha256,
        "host_receipt_sha256": observation["host_receipt_sha256"],
        "start_bundle_sha256": bundle_sha256,
        "installed_fixture_binary": fixture,
        "host_observation_request_sha256": observation["request_sha256"],
        "host_observation_response_sha256": observation["response_sha256"],
        "signed_start_dispatched": False,
        "campaign_state_mutated": False,
        "product_candidate_started": False,
        "promotable": False,
    }
    evidence_output.mkdir(mode=0o700, parents=True, exist_ok=False)
    (evidence_output / "evidence.json").write_bytes(canonical_json(evidence) + b"\n")
    return evidence


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--observation-directory", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    parser.add_argument("--evidence-output", type=pathlib.Path, required=True)
    parser.add_argument("--source-sha", required=True)
    parser.add_argument("--repository-id", type=int, required=True)
    options = parser.parse_args()
    evidence = assemble(
        options.observation_directory,
        options.output,
        options.evidence_output,
        options.source_sha,
        options.repository_id,
    )
    print(
        "0.74 non-product start bundle assembled: "
        f"campaign={evidence['campaign_id']} "
        f"manifest={evidence['manifest_sha256']} "
        f"bundle={evidence['start_bundle_sha256']}; no start dispatched"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
