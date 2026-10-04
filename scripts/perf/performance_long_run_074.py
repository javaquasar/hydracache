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
    "i74_tree_sha",
    "c74_tree_sha",
    "i74_cargo_lock_sha256",
    "c74_cargo_lock_sha256",
    "i74_dirty",
    "c74_dirty",
    "scenario_sha256",
    "workload_sha256",
    "offered_load_sha256",
    "estimator_sha256",
    "thresholds_sha256",
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
    "installed_binaries",
    "argv_templates",
    "command_environment_sha256",
    "role_order",
    "phase_durations_seconds",
    "output_limits",
    "expected_output_schema_sha256s",
    "required_final_guards",
    "secret_identifiers",
}
SHA256_FIELDS = {
    "contract_sha256",
    "scenario_sha256",
    "host_receipt_sha256",
    "random_nonce_hex",
    "i74_cargo_lock_sha256",
    "c74_cargo_lock_sha256",
    "workload_sha256",
    "offered_load_sha256",
    "estimator_sha256",
    "thresholds_sha256",
    "command_environment_sha256",
}
GIT_SHA_FIELDS = {
    "tooling_sha",
    "i74_source_sha",
    "c74_source_sha",
    "i74_tree_sha",
    "c74_tree_sha",
}
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
    if value.get("i74_dirty") is not False or value.get("c74_dirty") is not False:
        problems.append("both source roles must declare dirty=false")
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
    if not isinstance(identifiers, list) or len(identifiers) > 32 or any(
        not isinstance(item, str)
        or not item
        or len(item) > 128
        or "=" in item
        or "\n" in item
        or "\r" in item
        for item in identifiers
    ):
        problems.append("secret_identifiers must be a list of bounded identifier strings")
    binaries = value.get("installed_binaries")
    if not isinstance(binaries, list) or len(binaries) != 2:
        problems.append("installed_binaries must contain exact i74 and c74 rows")
    else:
        roles: list[str] = []
        for binary in binaries:
            if not isinstance(binary, dict) or set(binary) != {
                "role", "path", "sha256", "size", "inode", "device", "uid", "gid", "mode"
            }:
                problems.append("installed binary fields differ from the frozen schema")
                continue
            roles.append(str(binary.get("role")))
            path = binary.get("path")
            if (
                not isinstance(path, str)
                or not path.startswith("/opt/hydracache-performance/0.74/")
                or len(path) <= len("/opt/hydracache-performance/0.74/")
                or ".." in pathlib.PurePosixPath(path).parts
                or "\0" in path
                or "\n" in path
                or "\r" in path
                or len(path) > 512
            ):
                problems.append("installed binary path is outside the fixed 0.74 root")
            if not re.fullmatch(r"[0-9a-f]{64}", str(binary.get("sha256", ""))):
                problems.append("installed binary sha256 is invalid")
            for field in ["size", "inode", "device"]:
                item = binary.get(field)
                if isinstance(item, bool) or not isinstance(item, int) or item <= 0:
                    problems.append(f"installed binary {field} must be positive")
            for field in ["uid", "gid", "mode"]:
                item = binary.get(field)
                if isinstance(item, bool) or not isinstance(item, int) or item < 0:
                    problems.append(f"installed binary {field} must be non-negative")
            if isinstance(binary.get("mode"), int) and binary["mode"] > 0o7777:
                problems.append("installed binary mode exceeds the Unix permission mask")
        if roles != ["i74", "c74"]:
            problems.append("installed binary role order must be i74,c74")
    argv = value.get("argv_templates")
    if not isinstance(argv, dict) or set(argv) != {"i74", "c74"}:
        problems.append("argv_templates must contain exact i74 and c74 arrays")
    else:
        binary_rows = binaries if isinstance(binaries, list) else []
        by_role = {
            item.get("role"): item.get("path")
            for item in binary_rows
            if isinstance(item, dict)
        }
        for role in ["i74", "c74"]:
            args = argv.get(role)
            if (
                not isinstance(args, list)
                or not args
                or len(args) > 128
                or any(not isinstance(arg, str) or not arg or "\0" in arg or len(arg) > 1024 for arg in args)
                or args[0] != by_role.get(role)
            ):
                problems.append(f"argv_templates.{role} is invalid or not bound to its binary")
    if value.get("role_order") != ["i74", "c74"]:
        problems.append("role_order must remain i74,c74")
    _validate_exact_positive_map(
        value.get("phase_durations_seconds"),
        {"warmup", "measured", "drain", "durable_companion", "post_work_idle", "reconciliation"},
        "phase_durations_seconds",
        problems,
    )
    _validate_exact_positive_map(
        value.get("output_limits"),
        {"stdout_bytes", "stderr_bytes", "diagnostic_bytes", "final_artifact_bytes", "files"},
        "output_limits",
        problems,
    )
    output_limits = value.get("output_limits")
    if isinstance(output_limits, dict):
        final_artifact_bytes = output_limits.get("final_artifact_bytes")
        maximum_campaign_bytes = value.get("maximum_campaign_bytes")
        files = output_limits.get("files")
        maximum_campaign_files = value.get("maximum_campaign_files")
        if (
            isinstance(final_artifact_bytes, int)
            and not isinstance(final_artifact_bytes, bool)
            and isinstance(maximum_campaign_bytes, int)
            and not isinstance(maximum_campaign_bytes, bool)
            and final_artifact_bytes > maximum_campaign_bytes
        ):
            problems.append("final artifact limit exceeds the campaign byte limit")
        if (
            isinstance(files, int)
            and not isinstance(files, bool)
            and isinstance(maximum_campaign_files, int)
            and not isinstance(maximum_campaign_files, bool)
            and files > maximum_campaign_files
        ):
            problems.append("output file limit exceeds the campaign file limit")
    schemas = value.get("expected_output_schema_sha256s")
    if (
        not isinstance(schemas, dict)
        or set(schemas) != {"checkpoint", "measurement", "reconciliation", "packet_manifest"}
        or any(not re.fullmatch(r"[0-9a-f]{64}", str(item)) for item in schemas.values())
    ):
        problems.append("expected_output_schema_sha256s is invalid")
    guards = value.get("required_final_guards")
    guards_are_strings = isinstance(guards, list) and all(
        isinstance(item, str) for item in guards
    )
    if (
        not isinstance(guards, list)
        or not guards
        or len(guards) > 128
        or not guards_are_strings
        or (guards_are_strings and len(set(guards)) != len(guards))
        or (
            guards_are_strings
            and any(not item or len(item) > 128 or "\0" in item for item in guards)
        )
    ):
        problems.append("required_final_guards must be unique bounded identifiers")
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


def _validate_exact_positive_map(
    value: Any, fields: set[str], label: str, problems: list[str]
) -> None:
    if not isinstance(value, dict) or set(value) != fields:
        problems.append(f"{label} fields differ from the frozen schema")
        return
    if any(isinstance(item, bool) or not isinstance(item, int) or item <= 0 for item in value.values()):
        problems.append(f"{label} values must be positive integers")


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
