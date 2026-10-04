#!/usr/bin/env python3
"""Read-only 0.74 campaign observer; never starts, attaches, seals, or aborts."""

from __future__ import annotations

import argparse
import hashlib
import json
import pathlib
import re
import time
from typing import Any


HASH_RE = re.compile(r"^[0-9a-f]{64}$")
LIVE_STATES = {"I74_STARTING", "I74_RUNNING", "C74_STARTING", "C74_RUNNING"}
TERMINAL_STATES = {
    "I74_TERMINAL",
    "I74_SEALED",
    "C74_TERMINAL",
    "COMPLETE_SEALED",
    "FAILED_INCOMPLETE",
    "ABORTED_INCOMPLETE",
    "LEASE_EXPIRED_INCOMPLETE",
    "CORRUPT_QUARANTINED",
}
REQUIRED_TOP_LEVEL = {
    "revision",
    "campaign_state",
    "identity",
    "harness",
    "daemon",
    "checkpoint",
    "controller_lease",
    "recorded_failure",
    "duplicate_executor",
    "durable_history_corrupt",
}
REQUIRED_IDENTITY = {
    "campaign_id",
    "manifest_sha256",
    "contract_sha256",
    "scenario_sha256",
    "tooling_sha256",
    "source_bundle_sha256",
    "binary_bundle_sha256",
    "workload_bundle_sha256",
    "machine_id",
    "boot_id",
    "host_receipt_sha256",
    "mount_identity",
    "isolated_cpuset",
    "housekeeping_cpuset",
    "command_environment_sha256",
    "lease_id",
    "lease_deadline_unix_seconds",
}
REQUIRED_PROCESS = {
    "boot_id",
    "pid",
    "start_ticks",
    "process_group",
    "cgroup_path",
    "cgroup_inode",
    "unit_name",
}
REQUIRED_CHECKPOINT = {
    "sequence",
    "record_sha256",
    "useful_progress_unix_seconds",
}
REQUIRED_CONTROLLER_LEASE = {
    "holder_request_id",
    "authorization_sha256",
    "expires_unix_seconds",
}
MAX_STATE_BYTES = 65_536


def is_integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def require_exact_object(
    value: Any, required: set[str], label: str, problems: list[str]
) -> dict[str, Any]:
    if not isinstance(value, dict):
        problems.append(f"{label} is not an object")
        return {}
    if set(value) != required:
        problems.append(f"{label} fields differ from the frozen schema")
    return value


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def inspect_state(
    state: dict[str, Any],
    *,
    expected_campaign_id: str,
    expected_manifest_sha256: str,
    now_unix_seconds: int,
    progress_rejection_gap_seconds: int,
) -> dict[str, Any]:
    problems: list[str] = []
    if set(state) != REQUIRED_TOP_LEVEL:
        problems.append("state fields differ from the frozen schema")
    identity = require_exact_object(
        state.get("identity"), REQUIRED_IDENTITY, "identity", problems
    )
    checkpoint = require_exact_object(
        state.get("checkpoint"), REQUIRED_CHECKPOINT, "checkpoint", problems
    )
    harness = require_exact_object(
        state.get("harness"), REQUIRED_PROCESS, "harness", problems
    )
    daemon = require_exact_object(
        state.get("daemon"), REQUIRED_PROCESS, "daemon", problems
    )
    lease_value = state.get("controller_lease")
    lease = (
        {}
        if lease_value is None
        else require_exact_object(
            lease_value, REQUIRED_CONTROLLER_LEASE, "controller lease", problems
        )
    )
    if identity.get("campaign_id") != expected_campaign_id:
        problems.append("campaign id drift")
    if identity.get("manifest_sha256") != expected_manifest_sha256:
        problems.append("manifest digest drift")
    for field in [
        "contract_sha256",
        "scenario_sha256",
        "tooling_sha256",
        "source_bundle_sha256",
        "binary_bundle_sha256",
        "workload_bundle_sha256",
        "host_receipt_sha256",
        "command_environment_sha256",
    ]:
        if not HASH_RE.fullmatch(str(identity.get(field, ""))):
            problems.append(f"identity {field} is invalid")
    if not is_integer(state.get("revision")) or state.get("revision", -1) < 0:
        problems.append("state revision is invalid")
    for label, process in [("harness", harness), ("daemon", daemon)]:
        for field in ["pid", "start_ticks", "cgroup_inode"]:
            if not is_integer(process.get(field)) or process.get(field, 0) <= 0:
                problems.append(f"{label} {field} is invalid")
        if not is_integer(process.get("process_group")):
            problems.append(f"{label} process_group is invalid")
        if process.get("boot_id") != identity.get("boot_id"):
            problems.append(f"{label} boot id drift")
    for field in ["sequence", "useful_progress_unix_seconds"]:
        if not is_integer(checkpoint.get(field)) or checkpoint.get(field, -1) < 0:
            problems.append(f"checkpoint {field} is invalid")
    for field in ["record_sha256"]:
        if not HASH_RE.fullmatch(str(checkpoint.get(field, ""))):
            problems.append(f"checkpoint {field} is invalid")
    state_name = state.get("campaign_state")
    if state_name not in LIVE_STATES | TERMINAL_STATES | {"PREPARED"}:
        problems.append("unknown campaign state")
    useful_progress = checkpoint.get("useful_progress_unix_seconds")
    if isinstance(useful_progress, bool) or not isinstance(useful_progress, int):
        problems.append("useful progress time is invalid")
        progress_age = None
    else:
        progress_age = now_unix_seconds - useful_progress
        if progress_age < 0:
            problems.append("useful progress time is in the future")
    if state.get("durable_history_corrupt") or state_name == "CORRUPT_QUARANTINED":
        classification = "evidence-corruption"
    elif state.get("recorded_failure") or state.get("duplicate_executor"):
        classification = "measurement-loss"
    elif state_name in LIVE_STATES and progress_age is not None:
        if progress_age > progress_rejection_gap_seconds:
            classification = "progress-loss"
        else:
            lease_expiry = lease.get("expires_unix_seconds")
            if lease and (not is_integer(lease_expiry) or lease_expiry < 0):
                problems.append("controller lease expiry is invalid")
            lease_live = bool(lease) and is_integer(lease_expiry) and lease_expiry >= now_unix_seconds
            classification = "healthy" if lease_live else "controller-loss"
    elif state_name in TERMINAL_STATES:
        classification = "terminal"
    else:
        classification = "prepared"
    if problems:
        classification = "invalid-state"
    return {
        "schema_version": 1,
        "release": "0.74",
        "mode": "read-only-status",
        "promotable": False,
        "campaign_id": expected_campaign_id,
        "manifest_sha256": expected_manifest_sha256,
        "state_revision": state.get("revision"),
        "campaign_state": state_name,
        "classification": classification,
        "checkpoint_sequence": checkpoint.get("sequence"),
        "checkpoint_head_sha256": checkpoint.get("record_sha256"),
        "useful_progress_age_seconds": progress_age,
        "harness_pid": harness.get("pid"),
        "harness_start_ticks": harness.get("start_ticks"),
        "daemon_pid": daemon.get("pid"),
        "daemon_start_ticks": daemon.get("start_ticks"),
        "problems": problems,
        "mutations": [],
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--state", type=pathlib.Path, required=True)
    parser.add_argument("--campaign-id", required=True)
    parser.add_argument("--manifest-sha256", required=True)
    parser.add_argument("--now-unix-seconds", type=int, default=int(time.time()))
    parser.add_argument("--progress-rejection-gap-seconds", type=int, default=180)
    parser.add_argument("--output", type=pathlib.Path, required=True)
    options = parser.parse_args()
    if not HASH_RE.fullmatch(options.campaign_id) or not HASH_RE.fullmatch(
        options.manifest_sha256
    ):
        raise SystemExit("campaign and manifest identities must be lowercase SHA-256")
    if options.progress_rejection_gap_seconds != 180:
        raise SystemExit("progress rejection gap must remain frozen at 180 seconds")
    if options.output.exists():
        raise SystemExit("output already exists")
    if options.state.stat().st_size > MAX_STATE_BYTES:
        raise SystemExit("state exceeds the frozen 65,536-byte limit")
    state = json.loads(options.state.read_text(encoding="utf-8"))
    if not isinstance(state, dict):
        raise SystemExit("state must be one JSON object")
    receipt = inspect_state(
        state,
        expected_campaign_id=options.campaign_id,
        expected_manifest_sha256=options.manifest_sha256,
        now_unix_seconds=options.now_unix_seconds,
        progress_rejection_gap_seconds=options.progress_rejection_gap_seconds,
    )
    receipt["state_sha256"] = sha256(options.state)
    options.output.parent.mkdir(parents=True, exist_ok=True)
    with options.output.open("x", encoding="utf-8", newline="\n") as output:
        output.write(json.dumps(receipt, indent=2) + "\n")
    print(f"0.74 campaign monitor: {receipt['classification']} ({options.output}); no mutation")
    return 0 if not receipt["problems"] else 9


if __name__ == "__main__":
    raise SystemExit(main())
