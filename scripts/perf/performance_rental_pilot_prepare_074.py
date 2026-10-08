#!/usr/bin/env python3
"""Prepare/verify an offline P0 build seal. No workload, SSH or host-lock API.

Hash verification is not compilation provenance, host reservation or permission
to execute the pilot. Workload child-tree/cgroup ownership remains unimplemented.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import stat
import subprocess
import tempfile
import time
import tomllib


BASE = Path("docs/testing/performance/0.74")
PILOT = BASE / "rental-diagnostic-pilot-contract.toml"
COORDINATOR = BASE / "rental-pilot-coordinator-contract.toml"
OBSERVER = Path("tools/get-owner-scheduled-controls-074")
BINARY = OBSERVER / "target/x86_64-unknown-linux-gnu/release/timing-controls-074"
SOURCE = "62114be0f5da3218706e30d7424acfb5d0579d07"
MAX_INPUT = 65536
MAX_BINARY = 128 * 1024 * 1024
BUILD_COMMAND = ["cargo", "+1.94.0", "build", "--manifest-path", (OBSERVER / "Cargo.toml").as_posix(),
                 "--release", "--locked", "--no-default-features", "--bin", "timing-controls-074",
                 "--target", "x86_64-unknown-linux-gnu", "--message-format=json"]
SEAL_FIELDS = {
    "schema_version", "source_commit", "source_tree", "source_clean",
    "target", "profile", "features", "counting_allocator", "binary_sha256",
    "binary_bytes", "root_lock_sha256", "observer_lock_sha256",
    "rustc_verbose", "cargo_version", "build_command", "build_log_sha256",
}


def require(ok: bool, reason: str) -> None:
    if not ok:
        raise ValueError(reason)


def safe_regular(path: Path) -> os.stat_result:
    require(path.is_absolute(), "path must be absolute")
    require(".." not in path.parts, "parent traversal in input path")
    for parent in reversed(path.parents):
        require(stat.S_ISDIR(parent.lstat().st_mode), "unsafe path ancestor")
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, "unsafe seal/input file")
    return info


def read_bounded(path: Path, limit: int) -> bytes:
    safe_regular(path)
    with path.open("rb") as stream:
        raw = stream.read(limit + 1)
    require(len(raw) <= limit, "input exceeds bound")
    return raw


def digest(path: Path, limit: int = MAX_BINARY) -> str:
    info = safe_regular(path)
    require(info.st_size <= limit, "hash input exceeds bound")
    h, size = hashlib.sha256(), 0
    with path.open("rb") as stream:
        while chunk := stream.read(65536):
            size += len(chunk)
            require(size <= limit, "hash input grew beyond bound")
            h.update(chunk)
    require(size == info.st_size, "hash input size changed")
    return h.hexdigest()


def unique_object(pairs: list[tuple]) -> dict:
    value = {}
    for key, item in pairs:
        require(key not in value, "duplicate JSON key")
        value[key] = item
    return value


def strict_json(path: Path) -> dict:
    value = json.loads(read_bounded(path, MAX_INPUT), object_pairs_hook=unique_object,
                       parse_constant=lambda _: (_ for _ in ()).throw(ValueError("nonfinite JSON")))
    require(isinstance(value, dict), "object required")
    return value


def metadata_command(args: list[str], root: Path) -> str:
    """Small git/version commands only, not an arbitrary workload deadline API.

    Outputs go to temporary files, not unbounded communicate buffers. The fixed
    allowlist does not spawn workload descendants. It is NOT a cgroup runner.
    """
    allowed = [
        ["git", "--no-optional-locks", "rev-parse", "HEAD"],
        ["git", "--no-optional-locks", "rev-parse", "HEAD^{tree}"],
        ["git", "--no-optional-locks", "status", "--porcelain", "--untracked-files=all"],
        ["rustc", "+1.94.0", "-Vv"], ["cargo", "+1.94.0", "-V"],
    ]
    require(args in allowed, "metadata command not allowlisted")
    env = {name: value for name, value in os.environ.items()
           if name in ("PATH", "SYSTEMROOT", "WINDIR", "USERPROFILE", "HOME", "RUSTUP_HOME", "CARGO_HOME")}
    # Disable Git's optional external filesystem-monitor command before spawn.
    actual = ["git", "-c", "core.fsmonitor=false", *args[1:]] if args[0] == "git" else args
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        process = subprocess.Popen(actual, cwd=root, env=env, stdin=subprocess.DEVNULL,
                                   stdout=stdout, stderr=stderr)
        deadline = time.monotonic() + 10
        try:
            while process.poll() is None:
                require(time.monotonic() < deadline, "metadata command deadline; not a workload tree receipt")
                require(os.fstat(stdout.fileno()).st_size + os.fstat(stderr.fileno()).st_size <= MAX_INPUT,
                        "metadata output exceeds bound")
                time.sleep(.01)
            code = process.returncode
        except BaseException:
            if process.poll() is None:
                process.kill()
            process.wait(timeout=1)
            raise
        stdout.seek(0)
        stderr.seek(0)
        raw, errors = stdout.read(MAX_INPUT + 1), stderr.read(MAX_INPUT + 1)
        require(len(raw) + len(errors) <= MAX_INPUT, "metadata output exceeds bound")
        require(code == 0, "metadata command failed")
        return raw.decode("utf-8").strip()


def plan(root: Path) -> dict:
    pilot = tomllib.loads(read_bounded(root / PILOT, MAX_INPUT).decode())
    coordinator = tomllib.loads(read_bounded(root / COORDINATOR, MAX_INPUT).decode())
    require(pilot["state"] == "prepared-not-executable", "pilot state drift")
    require(pilot["intended_observer_source"] == SOURCE == coordinator["source_commit"], "source drift")
    for name in ("numerical_execution_allowed", "service_mutation_allowed", "qualification_allowed"):
        require(pilot[name] is False, "pilot execution boundary opened")
    for name in ("product_or_fixture_execution_allowed", "host_reservation_allowed", "remote_build_or_upload_allowed"):
        require(coordinator[name] is False, "coordinator execution boundary opened")
    inputs = []
    p0 = pilot["p0_cpu_feasibility"]
    require(p0["features"] == [] and p0["counting_allocator"] is False and
            p0["ab_comparison_allowed"] is False and p0["secure_cells_allowed"] is False,
            "baseline-only scope drift")
    for cell in pilot["p0_cpu_feasibility"]["cells"]:
        require(cell["config"] == f'rental-pilot-draft/{cell["surface"]}.json', "config path drift")
        config = root / BASE / cell["config"]
        value = strict_json(config)
        expected = {"schema_version": 1, "profile_id": "unprofiled-timing-controls-074-v1",
                    "surface": cell["surface"], "operation": "get",
                    **{name: p0[name] for name in ("seed", "keyspace", "payload_bytes", "dataset_sha256", "slots")},
                    "pipeline_depth": cell["pipeline_depth"],
                    **{name: p0[name] for name in ("warmup_calls", "minimum_usable_cpu_ns", "minimum_usable_measurement_wall_ns")},
                    "schedule": {"operations": p0["offered_operations_per_process"],
                                 "offered_rate_per_second": p0["offered_rate_per_second"], "concurrency": p0["slots"],
                                 **{name: p0[name] for name in ("maximum_queued", "operation_timeout_ns", "drain_timeout_ns", "slo_ns", "highest_trackable_ns")}}}
        require(value == expected, "complete config/contract drift")
        for mapping in (value, value["schedule"]):
            require(all(type(item) is int for item in mapping.values() if isinstance(item, (int, float))),
                    "numeric fields must be integers, not bool/float")
        canonical = json.dumps(expected, separators=(",", ":")).encode()
        require(hashlib.sha256(canonical).hexdigest() == cell["workload_sha256"], "typed workload digest drift")
        inputs.append({"order": cell["order"], "surface": cell["surface"], "config": cell["config"],
                       "raw_config_sha256": digest(config, MAX_INPUT),
                       "workload_sha256_from_contract": cell["workload_sha256"]})
    require([row["surface"] for row in inputs] == ["embedded", "direct", "resp2", "resp3"], "cell order drift")
    return {
        "schema_version": "rental-pilot-preparation-074-v1", "state": "PREPARATION_ONLY",
        "intended_source_commit": SOURCE, "pilot_contract_sha256": digest(root / PILOT, MAX_INPUT),
        "coordinator_contract_sha256": digest(root / COORDINATOR, MAX_INPUT),
        "build_command": BUILD_COMMAND, "binary_relative_path": BINARY.as_posix(),
        "configs": inputs, "source_clean_before_after_build_required": True,
        "metadata_commands_started": False, "build_started": False,
        "linux_binary_verified": False, "workload_started": False,
        "host_reservation": "BLOCKED_EXTERNAL_FLOCK_UNSAFE",
        "workload_child_tree_deadline_implemented": False,
        "pilot_execution_allowed": False, "promotable": False, "admission_allowed": False,
    }


def verify_build(source_root: Path, seal_path: Path, build_log: Path) -> dict:
    require(platform.system() == "Linux", "build verification requires local Linux")
    seal = strict_json(seal_path)
    require(set(seal) == SEAL_FIELDS, "build seal fields differ")
    for name in ("binary_sha256", "root_lock_sha256", "observer_lock_sha256", "build_log_sha256"):
        value = seal[name]
        require(isinstance(value, str) and len(value) == 64 and all(char in "0123456789abcdef" for char in value),
                "seal digest shape drift")
    require(type(seal["binary_bytes"]) is int and 64 <= seal["binary_bytes"] <= MAX_BINARY,
            "binary size type/bound drift")
    require(isinstance(seal["rustc_verbose"], str) and isinstance(seal["cargo_version"], str),
            "toolchain field type drift")
    require(seal["schema_version"] == "rental-pilot-build-seal-074-v1", "seal schema drift")
    require(seal["source_commit"] == SOURCE and seal["source_clean"] is True, "seal source drift")
    require(seal["target"] == "x86_64-unknown-linux-gnu" and seal["profile"] == "release", "build target/profile drift")
    require(seal["features"] == [] and seal["counting_allocator"] is False, "build feature drift")
    require(seal["build_command"] == BUILD_COMMAND, "build command drift")
    source = metadata_command(["git", "--no-optional-locks", "rev-parse", "HEAD"], source_root)
    require(source == SOURCE, "checkout is not the exact observer source")
    tree = metadata_command(["git", "--no-optional-locks", "rev-parse", "HEAD^{tree}"], source_root)
    require(seal["source_tree"] == tree, "source tree drift")
    require(not metadata_command(["git", "--no-optional-locks", "status", "--porcelain", "--untracked-files=all"], source_root),
            "source checkout is dirty")
    binary = source_root / BINARY
    info = safe_regular(binary)
    require(info.st_size == seal["binary_bytes"] and info.st_mode & 0o111 != 0,
            "binary size or executable mode drift")
    with binary.open("rb") as stream:
        header = stream.read(64)
    require(len(header) == 64 and header[:6] == b"\x7fELF\x02\x01" and
            int.from_bytes(header[16:18], "little") in (2, 3) and header[18:20] == b"\x3e\x00",
            "not an x86_64 ELF executable")
    require(digest(binary) == seal["binary_sha256"], "binary hash drift")
    for field, path in (("root_lock_sha256", source_root / "Cargo.lock"),
                        ("observer_lock_sha256", source_root / OBSERVER / "Cargo.lock")):
        require(digest(path) == seal[field], "compiled lock input drift")
    rustc = metadata_command(["rustc", "+1.94.0", "-Vv"], source_root)
    cargo = metadata_command(["cargo", "+1.94.0", "-V"], source_root)
    require(rustc == seal["rustc_verbose"] and rustc.startswith("rustc 1.94.0 (4a4ef493e ") and
            "host: x86_64-unknown-linux-gnu" in rustc, "rustc identity drift")
    require(cargo == seal["cargo_version"] and cargo.startswith("cargo 1.94.0 (85eff7c80 "), "cargo identity drift")
    require(digest(build_log, 16 * 1024 * 1024) == seal["build_log_sha256"], "build log drift")
    require(metadata_command(["git", "--no-optional-locks", "rev-parse", "HEAD"], source_root) == source and
            not metadata_command(["git", "--no-optional-locks", "status", "--porcelain", "--untracked-files=all"], source_root),
            "source changed while verifying")
    return {"schema_version": "rental-pilot-build-inspection-074-v1", "source_commit": source,
            "binary_sha256": seal["binary_sha256"], "linux_binary_verified": True,
            "compilation_provenance_proven": False, "build_started": False,
            "workload_started": False, "pilot_execution_allowed": False,
            "host_reservation": "BLOCKED_EXTERNAL_FLOCK_UNSAFE", "promotable": False,
            "admission_allowed": False}


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="mode", required=True)
    prepare = sub.add_parser("plan")
    prepare.add_argument("root", type=Path)
    verify = sub.add_parser("verify-build")
    verify.add_argument("source_root", type=Path)
    verify.add_argument("seal", type=Path)
    verify.add_argument("build_log", type=Path)
    args = parser.parse_args()
    try:
        result = (plan(args.root.absolute()) if args.mode == "plan" else
                  verify_build(args.source_root.absolute(), args.seal.absolute(), args.build_log.absolute()))
    except (OSError, ValueError, KeyError, UnicodeError) as error:
        parser.exit(1, f"preparation rejected: {type(error).__name__}: {error}\n")
    print(json.dumps(result, sort_keys=True, indent=2))


if __name__ == "__main__":
    main()
