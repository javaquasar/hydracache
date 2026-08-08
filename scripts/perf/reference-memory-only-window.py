#!/usr/bin/env python3
"""Run one non-ship performance window and prove that it stayed memory-only."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import time
from typing import Any


class GuardError(RuntimeError):
    pass


NVME_NAMESPACE = re.compile(r"^nvme[0-9]+n[0-9]+$")
GIT_SHA = re.compile(r"^[0-9a-f]{40}$")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def tree_digest(root: Path, excluded: Path) -> str:
    hasher = hashlib.sha256()
    for path in sorted(root.rglob("*")):
        try:
            path.relative_to(excluded)
            continue
        except ValueError:
            pass
        if path.is_symlink():
            raise GuardError(f"runtime tree contains a symlink: {path}")
        if not path.is_file():
            continue
        relative = path.relative_to(root).as_posix().encode()
        stat = path.stat()
        hasher.update(len(relative).to_bytes(8, "big"))
        hasher.update(relative)
        hasher.update(stat.st_mode.to_bytes(8, "big"))
        hasher.update(stat.st_size.to_bytes(8, "big"))
        with path.open("rb") as source:
            while block := source.read(1024 * 1024):
                hasher.update(block)
    return hasher.hexdigest()


def parse_cpu_list(value: str) -> set[int]:
    cpus: set[int] = set()
    for item in value.split(","):
        if not item:
            raise GuardError("empty CPU-list component")
        if "-" in item:
            start_text, end_text = item.split("-", 1)
            start, end = int(start_text), int(end_text)
            if start > end:
                raise GuardError("descending CPU range")
            cpus.update(range(start, end + 1))
        else:
            cpus.add(int(item))
    if not cpus:
        raise GuardError("measurement CPU set is empty")
    return cpus


def is_below(path: Path, root: Path) -> bool:
    try:
        path.resolve(strict=True).relative_to(root.resolve(strict=True))
        return True
    except (FileNotFoundError, ValueError):
        return False


def is_below_lexical(path: Path, root: Path) -> bool:
    try:
        path.resolve(strict=False).relative_to(root.resolve(strict=True))
        return True
    except ValueError:
        return False


def mount_type(path: Path, mountinfo: Path) -> str | None:
    resolved = path.resolve(strict=True)
    selected: tuple[int, str] | None = None
    for line in mountinfo.read_text(encoding="utf-8").splitlines():
        left, separator, right = line.partition(" - ")
        if not separator:
            continue
        fields = left.split()
        right_fields = right.split()
        if len(fields) < 5 or not right_fields:
            continue
        mount_point = Path(fields[4].replace("\\040", " "))
        try:
            resolved.relative_to(mount_point)
        except ValueError:
            continue
        candidate = (len(str(mount_point)), right_fields[0])
        if selected is None or candidate[0] > selected[0]:
            selected = candidate
    return None if selected is None else selected[1]


def read_diskstats(path: Path) -> dict[str, list[int]]:
    result: dict[str, list[int]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.split()
        if len(fields) >= 7 and NVME_NAMESPACE.fullmatch(fields[2]):
            result[fields[2]] = [int(value) for value in fields[3:]]
    if not result:
        raise GuardError("no NVMe namespace counters found")
    return result


def read_io_stat(path: Path) -> dict[str, dict[str, int]]:
    result: dict[str, dict[str, int]] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        fields = line.split()
        if not fields:
            continue
        counters: dict[str, int] = {}
        for field in fields[1:]:
            key, separator, value = field.partition("=")
            if not separator:
                raise GuardError("malformed cgroup io.stat")
            counters[key] = int(value)
        result[fields[0]] = counters
    return result


def read_nvme_irqs(path: Path, measurement_cpus: set[int]) -> dict[str, dict[str, int]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if not lines:
        raise GuardError("empty interrupts snapshot")
    headers = lines[0].split()
    cpu_columns = {int(item.removeprefix("CPU")): index for index, item in enumerate(headers)}
    missing = measurement_cpus.difference(cpu_columns)
    if missing:
        raise GuardError(f"interrupts snapshot lacks measurement CPUs: {sorted(missing)}")
    result: dict[str, dict[str, int]] = {}
    for line in lines[1:]:
        prefix, separator, suffix = line.partition(":")
        if not separator or "nvme" not in suffix.lower():
            continue
        counters = suffix.split()
        irq = prefix.strip()
        result[irq] = {
            str(cpu): int(counters[cpu_columns[cpu]]) for cpu in sorted(measurement_cpus)
        }
    if not result:
        raise GuardError("no NVMe IRQ counters found")
    return result


def nonzero_delta(before: Any, after: Any, prefix: str = "") -> list[str]:
    failures: list[str] = []
    if isinstance(before, dict) and isinstance(after, dict):
        for key in sorted(set(before) | set(after)):
            child = f"{prefix}.{key}" if prefix else str(key)
            if key not in before or key not in after:
                failures.append(f"{child}=mapping-changed")
            else:
                failures.extend(nonzero_delta(before[key], after[key], child))
        return failures
    if isinstance(before, list) and isinstance(after, list):
        if len(before) != len(after):
            return [f"{prefix}=counter-layout-changed"]
        for index, (old, new) in enumerate(zip(before, after)):
            if new != old:
                failures.append(f"{prefix}[{index}]={new - old:+d}")
        return failures
    if before != after:
        failures.append(f"{prefix}={after - before:+d}")
    return failures


def current_cgroup_io_path(proc_root: Path, cgroup_root: Path) -> Path:
    unified: str | None = None
    for line in (proc_root / "self/cgroup").read_text(encoding="utf-8").splitlines():
        hierarchy, controllers, path = line.split(":", 2)
        if hierarchy == "0" and controllers == "":
            unified = path
            break
    if unified is None:
        raise GuardError("unified cgroup v2 membership not found")
    candidate = cgroup_root / unified.lstrip("/") / "io.stat"
    if not candidate.is_file():
        raise GuardError(f"cgroup io.stat is unavailable: {candidate}")
    return candidate


def write_json(path: Path, value: Any) -> None:
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", required=True, type=Path)
    parser.add_argument("--runtime-root", required=True, type=Path)
    parser.add_argument("--output-dir", required=True, type=Path)
    parser.add_argument("--working-directory", type=Path)
    parser.add_argument("command", nargs=argparse.REMAINDER)
    args = parser.parse_args()
    if args.command and args.command[0] == "--":
        args.command = args.command[1:]
    if not args.command:
        parser.error("a command is required after --")
    return args


def main() -> int:
    args = parse_args()
    testing = os.environ.get("HC_LOCAL_ORCHESTRATION_TESTING") == "1" and Path(
        "/.dockerenv"
    ).exists()
    proc_root = Path(os.environ.get("HC_MEMORY_ONLY_PROC_ROOT", "/proc"))
    cgroup_root = Path(os.environ.get("HC_MEMORY_ONLY_CGROUP_ROOT", "/sys/fs/cgroup"))
    if not testing and (proc_root != Path("/proc") or cgroup_root != Path("/sys/fs/cgroup")):
        raise GuardError("fixture roots are forbidden outside local orchestration testing")

    profile = json.loads(args.profile.read_text(encoding="utf-8"))
    contract = profile.get("measurement_window_contract", {})
    expected_profile_id = (
        "ubuntu-24.04-memory-only-local-test" if testing else "ubuntu-24.04-memory-only-v1"
    )
    if (
        profile.get("schema_version") != 1
        or profile.get("profile_id") != expected_profile_id
        or contract.get("mode") != "memory-only-v1"
    ):
        raise GuardError("unsupported memory-only host profile")
    for field in ("qualification_evidence", "bootstrap_evidence", "ship_evidence_eligible"):
        if contract.get(field) is not False:
            raise GuardError(f"memory-only profile must keep {field}=false")
    for field in (
        "maximum_major_faults",
        "maximum_nvme_counter_delta",
        "maximum_cgroup_io_counter_delta",
        "maximum_measurement_cpu_nvme_irq_delta",
    ):
        if contract.get(field) != 0:
            raise GuardError(f"memory-only profile must keep {field}=0")
    for field in (
        "require_command_below_runtime_root",
        "require_working_directory_below_runtime_root",
        "require_swap_disabled",
        "require_nvme_devices",
    ):
        if contract.get(field) is not True:
            raise GuardError(f"memory-only profile must keep {field}=true")

    runtime_root = args.runtime_root.resolve(strict=True)
    working_directory = (args.working_directory or runtime_root).resolve(strict=True)
    executable = Path(args.command[0]).resolve(strict=True)
    if not is_below(executable, runtime_root):
        raise GuardError("command executable must be below the runtime root")
    if executable.read_bytes()[:4] != b"\x7fELF":
        raise GuardError("measured command must be a directly executable ELF binary")
    if not is_below(working_directory, runtime_root):
        raise GuardError("working directory must be below the runtime root")
    for argument in args.command[1:]:
        candidate_text = argument.partition("=")[2] if "=" in argument else argument
        if candidate_text.startswith("/"):
            if not is_below_lexical(Path(candidate_text), runtime_root):
                raise GuardError(f"command input escapes the runtime root: {candidate_text}")
    if not testing and mount_type(runtime_root, proc_root / "self/mountinfo") != "tmpfs":
        raise GuardError("runtime root must be on tmpfs")
    swaps = (proc_root / "swaps").read_text(encoding="utf-8").splitlines()
    if len(swaps) != 1:
        raise GuardError("swap must be disabled before a memory-only window")

    output_dir = args.output_dir
    if output_dir.exists():
        raise GuardError(f"refusing to overwrite output directory: {output_dir}")
    output_dir.parent.mkdir(parents=True, exist_ok=True)
    runtime_digest_before = tree_digest(runtime_root, output_dir)
    output_dir.mkdir(mode=0o755)
    if not is_below(output_dir, runtime_root):
        raise GuardError("output directory must be below the runtime root")

    measurement_cpus = parse_cpu_list(profile["cpu_contract"]["measurement_cpus"])
    online = set(range(os.cpu_count() or 0)) if not testing else measurement_cpus
    if not measurement_cpus.issubset(online):
        raise GuardError("measurement CPU set is not online")
    io_stat_path = current_cgroup_io_path(proc_root, cgroup_root)
    repo_root = Path(__file__).resolve().parents[2]
    source_commit = subprocess.run(
        ["git", "-C", str(repo_root), "rev-parse", "HEAD"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout.strip()
    if not GIT_SHA.fullmatch(source_commit):
        raise GuardError("could not bind the window to an exact source commit")
    boot_id = (proc_root / "sys/kernel/random/boot_id").read_text(encoding="utf-8").strip()

    before = {
        "diskstats": read_diskstats(proc_root / "diskstats"),
        "cgroup_io": read_io_stat(io_stat_path),
        "nvme_irqs": read_nvme_irqs(proc_root / "interrupts", measurement_cpus),
    }
    write_json(output_dir / "before.json", before)

    started_at = utc_now()
    started_ns = time.monotonic_ns()
    pid = os.fork()
    if pid == 0:
        try:
            os.chdir(working_directory)
            os.sched_setaffinity(0, measurement_cpus)
            os.execv(str(executable), [str(executable), *args.command[1:]])
        except BaseException as error:
            print(f"memory-only child launch failed: {error}", file=sys.stderr)
            os._exit(127)
    _, wait_status, usage = os.wait4(pid, 0)
    finished_ns = time.monotonic_ns()
    finished_at = utc_now()
    exit_code = os.waitstatus_to_exitcode(wait_status)

    after = {
        "diskstats": read_diskstats(proc_root / "diskstats"),
        "cgroup_io": read_io_stat(io_stat_path),
        "nvme_irqs": read_nvme_irqs(proc_root / "interrupts", measurement_cpus),
    }
    write_json(output_dir / "after.json", after)
    runtime_digest_after = tree_digest(runtime_root, output_dir)
    violations = {
        "nvme_counters": nonzero_delta(before["diskstats"], after["diskstats"]),
        "cgroup_io": nonzero_delta(before["cgroup_io"], after["cgroup_io"]),
        "measurement_cpu_nvme_irqs": nonzero_delta(before["nvme_irqs"], after["nvme_irqs"]),
        "major_faults": [] if usage.ru_majflt == 0 else [f"major_faults={usage.ru_majflt}"],
        "command": [] if exit_code == 0 else [f"exit_code={exit_code}"],
        "runtime_tree": []
        if runtime_digest_before == runtime_digest_after
        else ["runtime_tree=digest-changed"],
    }
    passed = not any(violations.values())
    receipt = {
        "schema_version": 1,
        "stage": "reference-memory-only-window",
        "source_commit": source_commit,
        "profile_id": profile["profile_id"],
        "profile_sha256": digest(args.profile),
        "runtime_root": str(runtime_root),
        "runtime_root_digest": runtime_digest_before,
        "working_directory": str(working_directory),
        "command": [str(executable), *args.command[1:]],
        "measurement_cpus": profile["cpu_contract"]["measurement_cpus"],
        "cgroup_io_stat": str(io_stat_path),
        "boot_id": boot_id,
        "kernel_release": os.uname().release,
        "started_at": started_at,
        "finished_at": finished_at,
        "major_faults": usage.ru_majflt,
        "exit_code": exit_code,
        "duration_ns": finished_ns - started_ns,
        "violations": violations,
        "passed": passed,
        "qualification_evidence": False,
        "bootstrap_evidence": False,
        "ship_evidence_eligible": False,
    }
    receipt_path = output_dir / "memory-only-window.json"
    write_json(receipt_path, receipt)
    for path in output_dir.iterdir():
        path.chmod(0o444)
    output_dir.chmod(0o555)
    print(f"MEMORY_ONLY_WINDOW_PASSED={str(passed).lower()} receipt={receipt_path}")
    return 0 if passed else 1


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (GuardError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"memory-only window rejected: {error}", file=sys.stderr)
        raise SystemExit(1)
