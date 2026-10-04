#!/usr/bin/env python3
"""Build a frozen 0.74 campaign start manifest without starting any process."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import re
from typing import Any


SCHEMA_VERSION = 1
RELEASE = "0.74"
EXPECTED_FIELDS = {
    "schema_version",
    "repository_id",
    "authorization_identity",
    "contract_sha256",
    "tooling_sha",
    "i74_source_sha",
    "c74_source_sha",
    "scenario_sha256",
    "host_receipt_sha256",
    "lease_id",
    "random_nonce_hex",
    "machine_id",
    "boot_id",
    "mount_identity",
    "isolated_cpuset",
    "housekeeping_cpuset",
    "seed",
    "checkpoint_cadence_seconds",
    "progress_warning_gap_seconds",
    "progress_rejection_gap_seconds",
    "diagnostic_grace_seconds",
    "product_lease_deadline_unix_seconds",
    "maximum_campaign_bytes",
    "maximum_campaign_files",
    "secret_identifiers",
}
SHA256_FIELDS = {
    "contract_sha256",
    "scenario_sha256",
    "host_receipt_sha256",
    "random_nonce_hex",
}
GIT_SHA_FIELDS = {"tooling_sha", "i74_source_sha", "c74_source_sha"}
UUID_RE = re.compile(
    r"^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
)
SECRET_FIELD_RE = re.compile(r"(?:password|private|credential|token|secret)(?:_|$)", re.I)


def canonical_json(value: Any) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode(
        "utf-8"
    )


def digest_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest()


def validate_inputs(value: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    if set(value) != EXPECTED_FIELDS:
        missing = sorted(EXPECTED_FIELDS - set(value))
        unknown = sorted(set(value) - EXPECTED_FIELDS)
        problems.append(f"input fields differ: missing={missing}, unknown={unknown}")
    if value.get("schema_version") != SCHEMA_VERSION:
        problems.append("schema_version must be 1")
    for field in SHA256_FIELDS:
        if not re.fullmatch(r"[0-9a-f]{64}", str(value.get(field, ""))):
            problems.append(f"{field} must be 64 lowercase hex characters")
    for field in GIT_SHA_FIELDS:
        if not re.fullmatch(r"[0-9a-f]{40}", str(value.get(field, ""))):
            problems.append(f"{field} must be a full lowercase Git SHA")
    if not UUID_RE.fullmatch(str(value.get("lease_id", ""))):
        problems.append("lease_id must be a lowercase UUID")
    for field in [
        "repository_id",
        "seed",
        "product_lease_deadline_unix_seconds",
        "maximum_campaign_bytes",
        "maximum_campaign_files",
    ]:
        item = value.get(field)
        if isinstance(item, bool) or not isinstance(item, int) or item <= 0:
            problems.append(f"{field} must be a positive integer")
    frozen = {
        "checkpoint_cadence_seconds": 30,
        "progress_warning_gap_seconds": 90,
        "progress_rejection_gap_seconds": 180,
        "diagnostic_grace_seconds": 30,
        "maximum_campaign_bytes": 21_474_836_480,
        "maximum_campaign_files": 20_000,
    }
    for field, expected in frozen.items():
        if value.get(field) != expected:
            problems.append(f"{field} must remain {expected}")
    for field in [
        "authorization_identity",
        "machine_id",
        "boot_id",
        "mount_identity",
        "isolated_cpuset",
        "housekeeping_cpuset",
    ]:
        item = value.get(field)
        if not isinstance(item, str) or not item or len(item) > 256:
            problems.append(f"{field} must be a non-empty bounded string")
    identifiers = value.get("secret_identifiers")
    if not isinstance(identifiers, list) or any(
        not isinstance(item, str) or not item or len(item) > 128 for item in identifiers
    ):
        problems.append("secret_identifiers must be a list of bounded identifier strings")
    if _contains_float(value):
        problems.append("floating-point values are forbidden")
    for field in value:
        if field != "secret_identifiers" and SECRET_FIELD_RE.search(field):
            problems.append(f"secret-bearing field {field} is forbidden")
    return problems


def campaign_id(value: dict[str, Any]) -> str:
    digest = hashlib.sha256()
    for field in [
        "contract_sha256",
        "tooling_sha",
        "i74_source_sha",
        "c74_source_sha",
        "scenario_sha256",
        "host_receipt_sha256",
    ]:
        digest.update(bytes.fromhex(value[field]))
    digest.update(value["lease_id"].encode("ascii"))
    digest.update(bytes.fromhex(value["random_nonce_hex"]))
    return digest.hexdigest()


def build_manifest(value: dict[str, Any]) -> dict[str, Any]:
    problems = validate_inputs(value)
    if problems:
        raise ValueError("invalid campaign inputs:\n- " + "\n- ".join(problems))
    result = {key: item for key, item in value.items() if key != "random_nonce_hex"}
    result.update(
        {
            "release": RELEASE,
            "campaign_id": campaign_id(value),
            "nonce_sha256": digest_bytes(bytes.fromhex(value["random_nonce_hex"])),
            "dirty": False,
            "controller_history": [],
            "state": "PREPARED",
        }
    )
    return result


def write_manifest(output: pathlib.Path, manifest: dict[str, Any]) -> tuple[pathlib.Path, str]:
    output.mkdir(parents=True, exist_ok=False)
    path = output / "campaign-start.json"
    encoded = canonical_json(manifest) + b"\n"
    with path.open("xb") as handle:
        handle.write(encoded)
        handle.flush()
        os.fsync(handle.fileno())
    digest = digest_bytes(encoded[:-1])
    digest_path = output / "campaign-start.sha256"
    with digest_path.open("x", encoding="ascii", newline="\n") as handle:
        handle.write(digest + "\n")
        handle.flush()
        os.fsync(handle.fileno())
    return path, digest


def _contains_float(value: Any) -> bool:
    if isinstance(value, float):
        return True
    if isinstance(value, dict):
        return any(_contains_float(item) for item in value.values())
    if isinstance(value, list):
        return any(_contains_float(item) for item in value)
    return False


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--prepare", action="store_true")
    parser.add_argument("--inputs", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    options = parser.parse_args()
    if not options.prepare:
        raise SystemExit("local tooling supports --prepare only; no process execution is implemented")
    if options.output.exists():
        raise SystemExit("output directory already exists")
    value = json.loads(options.inputs.read_text(encoding="utf-8"))
    if not isinstance(value, dict):
        raise SystemExit("campaign inputs must be one JSON object")
    manifest = build_manifest(value)
    path, digest = write_manifest(options.output, manifest)
    print(f"0.74 campaign manifest prepared: {path} sha256={digest}; no process started")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
