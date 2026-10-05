#!/usr/bin/env python3
"""Collect a read-only W11 systemd/provisioning capability receipt."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import sys
from typing import Any

try:
    import grp
    import pwd
except ImportError:  # pragma: no cover - exercised only by non-Unix local tooling
    grp = None
    pwd = None


SCHEMA_VERSION = "hydracache-w11-host-capability-v1"
SERVICE = "hydracache-performance-supervisor-074.service"
SUPERVISOR_USER = "hydracache-perf"
CLIENT_GROUP = "hydracache-perf-client"
PATHS = (
    "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074",
    "/etc/hydracache-perf/supervisor-074.toml",
    "/run/hydracache-perf/supervisor-v1.sock",
    "/var/lib/hydracache-performance/campaigns",
    "/var/lib/hydracache-performance/staging",
    "/var/lib/hydracache-performance/seals",
)


def command(*arguments: str) -> dict[str, Any]:
    executable = shutil.which(arguments[0])
    if executable is None:
        return {"available": False, "exit_code": None, "stdout": "", "stderr": ""}
    try:
        completed = subprocess.run(
            arguments,
            capture_output=True,
            text=True,
            check=False,
            timeout=10,
        )
        return {
            "available": True,
            "exit_code": completed.returncode,
            "stdout": completed.stdout[:16_384],
            "stderr": completed.stderr[:16_384],
        }
    except (OSError, subprocess.TimeoutExpired) as error:
        return {
            "available": True,
            "exit_code": None,
            "stdout": "",
            "stderr": f"{type(error).__name__}: {error}",
        }


def path_metadata(path: str) -> dict[str, Any]:
    try:
        value = os.lstat(path)
    except FileNotFoundError:
        return {"path": path, "status": "missing"}
    except PermissionError:
        return {"path": path, "status": "permission-denied"}
    except OSError as error:
        return {"path": path, "status": "error", "errno": error.errno}
    kind = (
        "socket"
        if stat.S_ISSOCK(value.st_mode)
        else "directory"
        if stat.S_ISDIR(value.st_mode)
        else "regular"
        if stat.S_ISREG(value.st_mode)
        else "symlink"
        if stat.S_ISLNK(value.st_mode)
        else "other"
    )
    return {
        "path": path,
        "status": "present",
        "kind": kind,
        "uid": value.st_uid,
        "gid": value.st_gid,
        "mode": stat.S_IMODE(value.st_mode),
        "device": value.st_dev,
        "inode": value.st_ino,
        "size": value.st_size,
    }


def account(name: str, group: bool = False) -> dict[str, Any]:
    if grp is None or pwd is None:
        return {"status": "unsupported-platform"}
    try:
        if group:
            value = grp.getgrnam(name)
            return {"status": "present", "gid": value.gr_gid, "members": sorted(value.gr_mem)}
        value = pwd.getpwnam(name)
        return {
            "status": "present",
            "uid": value.pw_uid,
            "gid": value.pw_gid,
            "home": value.pw_dir,
            "shell": value.pw_shell,
        }
    except KeyError:
        return {"status": "missing"}


def mount_for(path: str) -> dict[str, Any] | None:
    try:
        lines = Path("/proc/self/mountinfo").read_text(encoding="utf-8").splitlines()
    except OSError:
        return None
    target = Path(path)
    matches = []
    for line in lines:
        left, separator, right = line.partition(" - ")
        if not separator:
            continue
        fields = left.split()
        tail = right.split()
        if len(fields) < 6 or len(tail) < 3:
            continue
        mount_point = Path(fields[4].replace("\\040", " "))
        try:
            target.relative_to(mount_point)
        except ValueError:
            continue
        matches.append(
            {
                "mount_id": int(fields[0]),
                "device_major_minor": fields[2],
                "root": fields[3],
                "mount_point": str(mount_point),
                "mount_options": fields[5],
                "filesystem_type": tail[0],
                "source": tail[1],
                "super_options": tail[2],
            }
        )
    return max(matches, key=lambda item: len(item["mount_point"]), default=None)


def runner_groups() -> list[int]:
    return sorted(set(os.getgroups()) | {os.getgid()})


def collect(source_commit: str) -> dict[str, Any]:
    paths = {path: path_metadata(path) for path in PATHS}
    supervisor_user = account(SUPERVISOR_USER)
    client_group = account(CLIENT_GROUP, group=True)
    systemctl_state = command("systemctl", "is-system-running")
    unit = command(
        "systemctl",
        "show",
        SERVICE,
        "--property=LoadState,ActiveState,SubState,FragmentPath,MainPID,User,Group,Type,KillMode,ControlGroup",
        "--no-pager",
    )
    bus = command("busctl", "--system", "status", "org.freedesktop.systemd1")
    pid1 = ""
    try:
        pid1 = Path("/proc/1/comm").read_text(encoding="utf-8").strip()
    except OSError:
        pass
    cgroup_controllers = ""
    try:
        cgroup_controllers = Path("/sys/fs/cgroup/cgroup.controllers").read_text(encoding="utf-8").strip()
    except OSError:
        pass
    groups = runner_groups()
    socket = paths["/run/hydracache-perf/supervisor-v1.sock"]
    client_gid = client_group.get("gid")
    service_loaded = "LoadState=loaded" in unit["stdout"]
    service_active = "ActiveState=active" in unit["stdout"]
    expected_paths_present = all(value["status"] == "present" for value in paths.values())
    runner_can_reach_socket = (
        socket.get("kind") == "socket"
        and socket.get("mode") == 0o660
        and isinstance(client_gid, int)
        and socket.get("gid") == client_gid
        and client_gid in groups
    )
    systemd_available = (
        pid1 == "systemd"
        and systemctl_state["available"]
        and systemctl_state["exit_code"] in (0, 1)
        and bus["exit_code"] == 0
        and bool(cgroup_controllers)
    )
    provisioned = (
        supervisor_user.get("status") == "present"
        and client_group.get("status") == "present"
        and expected_paths_present
        and service_loaded
    )
    return {
        "schema_version": SCHEMA_VERSION,
        "source_commit": source_commit,
        "read_only": True,
        "mutation_performed": False,
        "host": {
            "node": platform.node(),
            "kernel": platform.release(),
            "machine": platform.machine(),
            "cpu_count": os.cpu_count(),
            "machine_id_sha256": hashlib.sha256(Path("/etc/machine-id").read_bytes().strip()).hexdigest(),
            "boot_id_sha256": hashlib.sha256(Path("/proc/sys/kernel/random/boot_id").read_bytes().strip()).hexdigest(),
        },
        "runner": {"uid": os.getuid(), "gid": os.getgid(), "groups": groups},
        "systemd": {
            "pid1_comm": pid1,
            "cgroup_v2_controllers": cgroup_controllers.split(),
            "system_state": systemctl_state,
            "manager_bus": bus,
            "service": unit,
            "systemd_run": command("systemd-run", "--version"),
        },
        "accounts": {"supervisor_user": supervisor_user, "client_group": client_group},
        "paths": paths,
        "mounts": {
            "/var/lib/hydracache-performance": mount_for("/var/lib/hydracache-performance"),
            "/run/hydracache-perf": mount_for("/run/hydracache-perf"),
        },
        "verdict": {
            "systemd_available": systemd_available,
            "provisioned": provisioned,
            "service_active": service_active,
            "runner_client_socket_access": runner_can_reach_socket,
            "host_rehearsal_ready": systemd_available
            and provisioned
            and service_active
            and runner_can_reach_socket,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if not sys.platform.startswith("linux"):
        raise SystemExit("W11 host capability collection requires Linux")
    if len(args.source_commit) != 40 or any(char not in "0123456789abcdef" for char in args.source_commit):
        raise SystemExit("source commit must be a full lowercase SHA")
    if args.output.exists():
        raise SystemExit("output must not pre-exist")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(
        json.dumps(collect(args.source_commit), indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
