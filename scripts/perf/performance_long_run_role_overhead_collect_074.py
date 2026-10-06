#!/usr/bin/env python3
"""Collect the fixed unprivileged W11 non-product role-overhead attempt set."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import socket
import subprocess
import sys
import uuid
from typing import Any

try:
    import resource
except ImportError:  # pragma: no cover - production collector requires Linux
    resource = None  # type: ignore[assignment]


SCHEMA_VERSION = 1
EVIDENCE_CLASS = "non-product-role-overhead-rehearsal"
ORDER = "abba-counterbalanced-v1"
SEED = 740_074
PAIRS = 5
OPERATIONS = 100_000_000
WARMUP_OPERATIONS = 1_000_000
CHECKPOINT_BYTES = 4_096
MAX_RECEIPT_BYTES = 1024 * 1024
MAX_BINARY_BYTES = 128 * 1024 * 1024
MAX_PACKET_BYTES = 65_536
FIXTURE_TIMEOUT_SECONDS = 10
INSTALLED_BINARY = Path(
    "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074"
)
SUPERVISOR_SOCKET = Path("/run/hydracache-perf/supervisor-v1.sock")
TASKSET = "/usr/bin/taskset"
NICE = "/usr/bin/nice"
HOST_SCOPE = "2dd3f4aa960289c385aad3988d988533197269368f851e5834f535a753cd92ba"
HOST_MANIFEST_SCOPE = (
    "baa6e451a7248643e7a96fc5908ac07e0c97d38255e31b1bb1494ea5ac26c6ec"
)
WORKLOAD_DEFINITION = (
    b"hydracache-0.74-non-product-role-overhead-xorshift64-v1;"
    b"operations=100000000;warmup=1000000;delay_ms=300"
)
PAYLOAD_DEFINITION = (
    b"hydracache-0.74-non-product-role-overhead-checkpoint-v1;bytes=4096"
)
SHA1 = re.compile(r"^[0-9a-f]{40}$")
SHA256 = re.compile(r"^[0-9a-f]{64}$")
CPUSET = re.compile(r"^[0-9]+(?:-[0-9]+)?(?:,[0-9]+(?:-[0-9]+)?)*$")
PROVISIONING_FIELDS = {
    "schema_version",
    "source_commit",
    "created_at_utc",
    "mutation_performed",
    "repository_id",
    "allowed_actor_ids",
    "runner_uid",
    "runner_gid",
    "supervisor_uid",
    "client_gid",
    "runner_group_database_membership",
    "runner_process_group_refresh_may_be_required",
    "service_active",
    "socket_mode",
    "binary_sha256",
    "fixture_binary_sha256",
    "config_sha256",
    "service_sha256",
    "sysusers_sha256",
    "tmpfiles_sha256",
    "verification_key_sha256",
    "unit_properties_sha256",
    "machine_id_sha256",
    "boot_id_sha256",
}
FIXTURE_FIELDS = {
    "schema_version",
    "role",
    "variant",
    "pair_index",
    "position",
    "cpuset",
    "nice",
    "elapsed_ns",
    "cpu_ns",
    "rss_peak_bytes",
    "io_bytes",
    "checkpoint_write_bytes",
    "completed_operations",
}


def _safe_regular(path: Path, maximum: int, *, nonempty: bool = True) -> bytes:
    metadata = path.lstat()
    if (
        path.is_symlink()
        or not path.is_file()
        or metadata.st_nlink != 1
        or metadata.st_size > maximum
        or (nonempty and metadata.st_size == 0)
    ):
        raise ValueError(f"bounded regular file required: {path}")
    raw = path.read_bytes()
    if len(raw) != metadata.st_size:
        raise ValueError(f"file changed while read: {path}")
    return raw


def _sha256_file(path: Path, maximum: int) -> str:
    return hashlib.sha256(_safe_regular(path, maximum)).hexdigest()


def validate_provisioning_receipt(
    path: Path, source_commit: str, binary_sha256: str
) -> str:
    raw = _safe_regular(path, MAX_RECEIPT_BYTES)
    try:
        receipt = json.loads(raw)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("provisioning receipt is not valid JSON") from error
    if not isinstance(receipt, dict) or set(receipt) != PROVISIONING_FIELDS:
        raise ValueError("provisioning receipt fields differ")
    if (
        receipt["schema_version"] != "hydracache-w11-host-provisioning-v1"
        or receipt["source_commit"] != source_commit
        or receipt["binary_sha256"] != binary_sha256
        or receipt["mutation_performed"] is not True
        or receipt["service_active"] is not True
        or receipt["socket_mode"] != 0o660
    ):
        raise ValueError("provisioning receipt identity differs")
    for field in (
        "fixture_binary_sha256",
        "config_sha256",
        "service_sha256",
        "sysusers_sha256",
        "tmpfiles_sha256",
        "verification_key_sha256",
        "unit_properties_sha256",
        "machine_id_sha256",
        "boot_id_sha256",
    ):
        if not isinstance(receipt[field], str) or not SHA256.fullmatch(receipt[field]):
            raise ValueError(f"provisioning receipt {field} is invalid")
    return hashlib.sha256(raw).hexdigest()


def _fields(path: Path) -> dict[str, str]:
    result: dict[str, str] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        key, found, value = line.partition(":")
        if found:
            result[key.strip()] = value.strip()
    return result


def _first_cpu(value: str) -> str:
    if not CPUSET.fullmatch(value):
        raise ValueError("allowed CPU list is not canonical")
    first = value.split(",", 1)[0].split("-", 1)[0]
    cpu = int(first)
    if first != str(cpu) or cpu >= 4096:
        raise ValueError("first allowed CPU is outside the admitted range")
    return first


def _kilobytes(value: str) -> int:
    match = re.fullmatch(r"([0-9]+) kB", value)
    if match is None or int(match.group(1)) == 0:
        raise ValueError("invalid process peak RSS")
    return int(match.group(1)) * 1024


def _cgroup_cpu_ns(path: Path) -> int:
    values: dict[str, str] = {}
    for line in path.read_text(encoding="ascii").splitlines():
        fields = line.split()
        if len(fields) != 2 or fields[0] in values:
            raise ValueError("cgroup cpu.stat is malformed")
        values[fields[0]] = fields[1]
    usage = values.get("usage_usec")
    if usage is None or not usage.isdecimal():
        raise ValueError("cgroup CPU usage is absent")
    return int(usage) * 1000


def _cgroup_io_bytes(path: Path) -> int:
    total = 0
    lines = path.read_text(encoding="ascii").splitlines()
    if not lines:
        raise ValueError("cgroup io.stat is empty")
    for line in lines:
        fields = line.split()
        if len(fields) < 2:
            raise ValueError("cgroup io.stat is malformed")
        counters: dict[str, str] = {}
        for item in fields[1:]:
            name, found, value = item.partition("=")
            if not found or name in counters or not value.isdecimal():
                raise ValueError("cgroup I/O counter is malformed")
            counters[name] = value
        if "rbytes" not in counters or "wbytes" not in counters:
            raise ValueError("cgroup byte counters are absent")
        total += int(counters["rbytes"]) + int(counters["wbytes"])
    return total


def resolve_cgroup(root: Path, control_group: str) -> Path:
    if not control_group.startswith("/") or ".." in Path(control_group).parts:
        raise ValueError("control group is not canonical")
    resolved_root = root.resolve(strict=True)
    candidate = (resolved_root / control_group.lstrip("/")).resolve(strict=True)
    if resolved_root not in candidate.parents or not candidate.is_dir():
        raise ValueError("control group escaped the cgroup root")
    return candidate


def _validate_process_identity(
    binary: Path, relative_cgroup: str, command_line: bytes, process_cgroup: str
) -> str:
    expected_command_line = (
        str(binary).encode("utf-8")
        + b"\0serve\0/etc/hydracache-perf/supervisor-074.toml\0"
    )
    if command_line != expected_command_line:
        raise ValueError("supervisor command identity differs")
    if process_cgroup != f"0::{relative_cgroup}\n":
        raise ValueError("supervisor process cgroup differs")
    return hashlib.sha256(command_line).hexdigest()


def supervisor_snapshot(
    pid: int, cgroup: Path, binary: Path, proc_root: Path = Path("/proc")
) -> dict[str, Any]:
    process = proc_root / str(pid)
    stat_line = (process / "stat").read_text(encoding="utf-8").strip()
    closing = stat_line.rfind(")")
    if closing < 0 or not stat_line.startswith(f"{pid} ("):
        raise ValueError("supervisor stat identity is malformed")
    stat_fields = stat_line[closing + 2 :].split()
    if len(stat_fields) < 20:
        raise ValueError("supervisor stat identity is truncated")
    status = _fields(process / "status")
    command_line = (process / "cmdline").read_bytes()
    relative_cgroup = "/" + cgroup.relative_to("/sys/fs/cgroup").as_posix()
    process_cgroup = (process / "cgroup").read_text(encoding="ascii")
    command_line_sha256 = _validate_process_identity(
        binary, relative_cgroup, command_line, process_cgroup
    )
    return {
        "pid": pid,
        "start_ticks": int(stat_fields[19]),
        "command_line_sha256": command_line_sha256,
        "process_cgroup": relative_cgroup,
        "cpuset": status["Cpus_allowed_list"],
        "cpu_ns": _cgroup_cpu_ns(cgroup / "cpu.stat"),
        "rss_peak_bytes": _kilobytes(status["VmHWM"]),
        "io_bytes": _cgroup_io_bytes(cgroup / "io.stat"),
    }


def _canonical(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def observe_idle_guard(
    socket_path: Path,
    source_commit: str,
    binary_sha256: str,
    repository_id: int,
    run_id: int,
    run_attempt: int,
    actor_id: int,
) -> None:
    request = {
        "schema_version": 1,
        "request_id": str(uuid.uuid4()),
        "operation": "host_observation",
        "campaign_id": HOST_SCOPE,
        "expected_state_revision": 0,
        "manifest_path": None,
        "manifest_sha256": HOST_MANIFEST_SCOPE,
        "controller": {
            "repository_id": repository_id,
            "run_id": run_id,
            "run_attempt": run_attempt,
            "actor_id": actor_id,
            "authorization_sha256": "0" * 64,
        },
        "abort_reason": None,
        "approval_nonce_sha256": None,
    }
    wire = _canonical({"request": request, "authorization": None})
    with socket.socket(socket.AF_UNIX, socket.SOCK_SEQPACKET) as client:
        client.settimeout(5)
        client.connect(str(socket_path))
        client.sendall(wire)
        response_bytes, _, flags, _ = client.recvmsg(MAX_PACKET_BYTES + 1)
    if flags & socket.MSG_TRUNC or not response_bytes or len(response_bytes) > MAX_PACKET_BYTES:
        raise ValueError("host observation response is outside the packet bound")
    try:
        response = json.loads(response_bytes)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("host observation response is invalid JSON") from error
    response_fields = {
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
    if not isinstance(response, dict) or set(response) != response_fields:
        raise ValueError("host observation response fields differ")
    body = {key: value for key, value in response.items() if key != "response_sha256"}
    if (
        not isinstance(response["response_sha256"], str)
        or hashlib.sha256(_canonical(body)).hexdigest() != response["response_sha256"]
        or response["request_id"] != request["request_id"]
        or response["campaign_id"] != HOST_SCOPE
        or response["ok"] is not True
        or response["state_revision"] != 0
        or response["error_code"] is not None
    ):
        raise ValueError("host observation response identity differs")
    result = response["result"]
    result_fields = {
        "schema_version",
        "installed_source_commit",
        "active_campaign_absent",
        "fixture_binary",
        "receipt_sha256",
        "receipt",
    }
    if (
        not isinstance(result, dict)
        or set(result) != result_fields
        or result["schema_version"] != 1
        or result["installed_source_commit"] != source_commit
        or result["active_campaign_absent"] is not True
    ):
        raise ValueError("host is not an idle source-matched installation")
    receipt = result["receipt"]
    if (
        not isinstance(receipt, dict)
        or not isinstance(result["receipt_sha256"], str)
        or hashlib.sha256(_canonical(receipt)).hexdigest() != result["receipt_sha256"]
        or receipt.get("supervisor_binary", {}).get("sha256") != binary_sha256
    ):
        raise ValueError("host observation binary receipt differs")


def _variant_order(pair_index: int) -> tuple[str, str]:
    variants = ("control", "instrumented")
    return variants if pair_index % 2 == 1 else tuple(reversed(variants))


def fixture_argv(
    binary: Path, role: str, variant: str, pair_index: int, position: int, cpu: str
) -> list[str]:
    return [
        TASKSET,
        "--cpu-list",
        cpu,
        NICE,
        "-n",
        "0",
        str(binary),
        "role-overhead-fixture",
        role,
        variant,
        str(pair_index),
        str(position),
        cpu,
    ]


def _fixture_limits() -> None:
    if resource is None:
        raise RuntimeError("POSIX resource limits are unavailable")
    resource.setrlimit(resource.RLIMIT_AS, (64 * 1024 * 1024, 64 * 1024 * 1024))
    resource.setrlimit(resource.RLIMIT_NOFILE, (64, 64))
    resource.setrlimit(resource.RLIMIT_FSIZE, (8192, 8192))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def run_fixture(
    binary: Path,
    directory: Path,
    role: str,
    variant: str,
    pair_index: int,
    position: int,
    cpu: str,
) -> dict[str, Any]:
    stem = f"{role}-p{pair_index}-{position}"
    checkpoint = directory / f"{stem}.checkpoint"
    receipt_path = directory / f"{stem}.json"
    descriptor = os.open(checkpoint, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "wb") as output:
            completed = subprocess.run(
                fixture_argv(binary, role, variant, pair_index, position, cpu),
                stdin=subprocess.DEVNULL,
                stdout=output,
                stderr=subprocess.PIPE,
                env={"LANG": "C.UTF-8", "LC_ALL": "C.UTF-8", "PATH": "/usr/bin:/bin", "TZ": "UTC"},
                check=False,
                timeout=FIXTURE_TIMEOUT_SECONDS,
                shell=False,
                preexec_fn=_fixture_limits,
            )
    except Exception:
        try:
            os.close(descriptor)
        except OSError:
            pass
        raise
    if completed.returncode != 0:
        raise ValueError(f"fixture failed with status {completed.returncode}")
    if len(completed.stderr) == 0 or len(completed.stderr) > 64 * 1024:
        raise ValueError("fixture receipt is outside the bound")
    receipt_descriptor = os.open(
        receipt_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600
    )
    with os.fdopen(receipt_descriptor, "wb") as receipt_output:
        receipt_output.write(completed.stderr)
        receipt_output.flush()
        os.fsync(receipt_output.fileno())
    try:
        receipt = json.loads(completed.stderr)
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        raise ValueError("fixture receipt is invalid JSON") from error
    checkpoint_size = checkpoint.stat().st_size
    expected_size = CHECKPOINT_BYTES if variant == "instrumented" else 0
    if (
        not isinstance(receipt, dict)
        or set(receipt) != FIXTURE_FIELDS
        or receipt["schema_version"] != SCHEMA_VERSION
        or receipt["role"] != role
        or receipt["variant"] != variant
        or receipt["pair_index"] != pair_index
        or receipt["position"] != position
        or receipt["cpuset"] != cpu
        or receipt["nice"] != 0
        or receipt["completed_operations"] != OPERATIONS
        or receipt["checkpoint_write_bytes"] != expected_size
        or checkpoint_size != expected_size
        or receipt["io_bytes"] < expected_size
        or any(receipt[field] <= 0 for field in ("elapsed_ns", "cpu_ns", "rss_peak_bytes"))
    ):
        raise ValueError("fixture identity, metrics, or checkpoint differ")
    return receipt


def collect_attempts(
    source_commit: str,
    host_receipt_sha256: str,
    binary: Path,
    binary_sha256: str,
    supervisor_pid: int,
    cgroup: Path,
    socket_path: Path,
    output_directory: Path,
    controller: tuple[int, int, int, int],
) -> dict[str, Any]:
    status = _fields(Path("/proc/self/status"))
    cpu = _first_cpu(status["Cpus_allowed_list"])
    boot_id = Path("/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()
    if not re.fullmatch(r"[0-9a-f-]{36}", boot_id):
        raise ValueError("host boot identity is invalid")
    identity = {
        "source_commit": source_commit,
        "binary_sha256": binary_sha256,
        "workload_sha256": hashlib.sha256(WORKLOAD_DEFINITION).hexdigest(),
        "payload_sha256": hashlib.sha256(PAYLOAD_DEFINITION).hexdigest(),
        "host_receipt_sha256": host_receipt_sha256,
        "seed": SEED,
        "operations": OPERATIONS,
        "warmup_operations": WARMUP_OPERATIONS,
        "cpuset": cpu,
    }
    guard_args = (socket_path, source_commit, binary_sha256, *controller)
    attempts: list[dict[str, Any]] = []
    for role in ("i74", "c74"):
        for pair_index in range(1, PAIRS + 1):
            for position, variant in enumerate(_variant_order(pair_index), 1):
                observe_idle_guard(*guard_args)
                before = supervisor_snapshot(supervisor_pid, cgroup, binary)
                fixture = run_fixture(
                    binary,
                    output_directory,
                    role,
                    variant,
                    pair_index,
                    position,
                    cpu,
                )
                after = supervisor_snapshot(supervisor_pid, cgroup, binary)
                observe_idle_guard(*guard_args)
                stable_fields = (
                    "pid",
                    "start_ticks",
                    "command_line_sha256",
                    "process_cgroup",
                    "cpuset",
                )
                supervisor_stable = all(before[field] == after[field] for field in stable_fields)
                if not supervisor_stable:
                    raise ValueError("supervisor identity changed during fixture")
                current_boot = Path("/proc/sys/kernel/random/boot_id").read_text(
                    encoding="ascii"
                ).strip()
                if current_boot != boot_id:
                    raise ValueError("host boot identity changed during fixture")
                if after["cpu_ns"] < before["cpu_ns"] or after["io_bytes"] < before["io_bytes"]:
                    raise ValueError("supervisor counters decreased")
                attempts.append(
                    {
                        "schema_version": SCHEMA_VERSION,
                        "role": role,
                        "variant": variant,
                        "pair_index": pair_index,
                        "position": position,
                        "identity": dict(identity),
                        "metrics": {
                            "elapsed_ns": fixture["elapsed_ns"],
                            "role_cpu_ns": fixture["cpu_ns"],
                            "role_rss_peak_bytes": fixture["rss_peak_bytes"],
                            "role_io_bytes": fixture["io_bytes"],
                            "supervisor_cpu_ns": after["cpu_ns"] - before["cpu_ns"],
                            "supervisor_rss_peak_bytes": max(
                                before["rss_peak_bytes"], after["rss_peak_bytes"]
                            ),
                            "supervisor_io_bytes": after["io_bytes"] - before["io_bytes"],
                            "checkpoint_write_bytes": fixture["checkpoint_write_bytes"],
                            "completed_operations": fixture["completed_operations"],
                        },
                        "guards": {
                            "affinity_applied": fixture["cpuset"] == cpu,
                            "priority_applied": fixture["nice"] == 0,
                            "host_identity_stable": current_boot == boot_id,
                            "role_identity_stable": True,
                            "supervisor_identity_stable": supervisor_stable,
                            "campaign_claim_valid": True,
                            "unexpected_errors_absent": True,
                        },
                    }
                )
    return {
        "schema_version": SCHEMA_VERSION,
        "release": "0.74",
        "evidence_class": EVIDENCE_CLASS,
        "promotable": False,
        "seed": SEED,
        "order": ORDER,
        "attempts": attempts,
    }


def _write_new(path: Path, value: dict[str, Any]) -> None:
    if path.exists():
        raise ValueError("output must not pre-exist")
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(descriptor, "w", encoding="utf-8", newline="\n") as output:
        json.dump(value, output, indent=2, sort_keys=True, allow_nan=False)
        output.write("\n")
        output.flush()
        os.fsync(output.fileno())


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--provisioning-receipt", required=True, type=Path)
    parser.add_argument("--installed-binary", required=True, type=Path)
    parser.add_argument("--supervisor-pid", required=True, type=int)
    parser.add_argument("--control-group", required=True)
    parser.add_argument("--socket", required=True, type=Path)
    parser.add_argument("--repository-id", required=True, type=int)
    parser.add_argument("--run-id", required=True, type=int)
    parser.add_argument("--run-attempt", required=True, type=int)
    parser.add_argument("--actor-id", required=True, type=int)
    parser.add_argument("--output", required=True, type=Path)
    options = parser.parse_args()
    try:
        if not sys.platform.startswith("linux") or os.geteuid() == 0:
            raise ValueError("role-overhead collection requires unprivileged Linux")
        if not SHA1.fullmatch(options.source_commit):
            raise ValueError("source commit must be a full lowercase SHA")
        if options.installed_binary != INSTALLED_BINARY or options.socket != SUPERVISOR_SOCKET:
            raise ValueError("installed binary or supervisor socket path differs")
        if options.supervisor_pid <= 1 or any(
            value <= 0
            for value in (
                options.repository_id,
                options.run_id,
                options.run_attempt,
                options.actor_id,
            )
        ):
            raise ValueError("process or controller identity is invalid")
        binary_sha256 = _sha256_file(options.installed_binary, MAX_BINARY_BYTES)
        host_receipt_sha256 = validate_provisioning_receipt(
            options.provisioning_receipt, options.source_commit, binary_sha256
        )
        cgroup = resolve_cgroup(Path("/sys/fs/cgroup"), options.control_group)
        fixture_directory = options.output.parent / "fixtures"
        fixture_directory.mkdir(mode=0o700, parents=False, exist_ok=False)
        attempts = collect_attempts(
            options.source_commit,
            host_receipt_sha256,
            options.installed_binary,
            binary_sha256,
            options.supervisor_pid,
            cgroup,
            options.socket,
            fixture_directory,
            (
                options.repository_id,
                options.run_id,
                options.run_attempt,
                options.actor_id,
            ),
        )
        _write_new(options.output, attempts)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        raise SystemExit(str(error)) from error
    print(f"0.74 non-product role-overhead attempts: OK ({options.output})")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
