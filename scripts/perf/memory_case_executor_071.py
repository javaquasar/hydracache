#!/usr/bin/env python3
"""Execute one isolated 0.71 memory cell against a real daemon process."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import signal
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path
from typing import Any


PHASES = ["cold", "fill", "steady", "expire_or_delete", "reset", "refill", "post_idle", "shutdown"]
ADMIN_HEADERS = {
    "x-hydracache-client-id": "memory-campaign-071",
    "x-hydracache-tenant": "memory-campaign-071",
    "x-hydracache-admin": "true",
}


class ExecutionError(RuntimeError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def free_port() -> int:
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return int(listener.getsockname()[1])


def resp_command(stream: socket.socket, *parts: bytes) -> Any:
    payload = b"*" + str(len(parts)).encode() + b"\r\n"
    for part in parts:
        payload += b"$" + str(len(part)).encode() + b"\r\n" + part + b"\r\n"
    stream.sendall(payload)
    return read_resp(stream)


def read_line(stream: socket.socket) -> bytes:
    value = bytearray()
    while not value.endswith(b"\r\n"):
        block = stream.recv(1)
        if not block:
            raise ExecutionError("daemon closed the RESP connection")
        value.extend(block)
    return bytes(value[:-2])


def read_resp(stream: socket.socket) -> Any:
    prefix = stream.recv(1)
    if prefix in {b"+", b":", b"-"}:
        value = read_line(stream)
        if prefix == b"-":
            raise ExecutionError(f"RESP error: {value.decode(errors='replace')}")
        return int(value) if prefix == b":" else value
    if prefix == b"$":
        length = int(read_line(stream))
        if length == -1:
            return None
        value = bytearray()
        while len(value) < length + 2:
            value.extend(stream.recv(length + 2 - len(value)))
        return bytes(value[:-2])
    if prefix == b"*":
        return [read_resp(stream) for _ in range(int(read_line(stream)))]
    raise ExecutionError(f"unsupported RESP prefix: {prefix!r}")


def http_json(port: int, path: str, method: str = "GET") -> dict[str, Any] | None:
    request = urllib.request.Request(
        f"http://127.0.0.1:{port}{path}", headers=ADMIN_HEADERS, method=method
    )
    try:
        with urllib.request.urlopen(request, timeout=5) as response:
            return json.loads(response.read())
    except (urllib.error.HTTPError, urllib.error.URLError, json.JSONDecodeError):
        return None


def wait_ready(process: subprocess.Popen[Any], port: int) -> None:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if process.poll() is not None:
            raise ExecutionError(f"daemon exited during readiness with {process.returncode}")
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=0.2) as stream:
                if resp_command(stream, b"PING") == b"PONG":
                    return
        except (OSError, ExecutionError):
            time.sleep(0.1)
    raise ExecutionError("daemon did not become RESP-ready")


def kib_value(value: str | None) -> int:
    return 0 if not value else int(value.split()[0]) * 1024


def key_values(path: Path) -> dict[str, str]:
    values: dict[str, str] = {}
    if path.is_file():
        for line in path.read_text(encoding="utf-8").splitlines():
            if ":" in line:
                key, value = line.split(":", 1)
                values[key] = value.strip()
    return values


def read_optional_int(path: Path) -> int | None:
    try:
        value = path.read_text(encoding="ascii").strip()
        return None if value == "max" else int(value)
    except (OSError, ValueError):
        return None


def process_snapshot(pid: int) -> dict[str, Any]:
    status = key_values(Path(f"/proc/{pid}/status"))
    smaps = key_values(Path(f"/proc/{pid}/smaps_rollup"))
    try:
        fds = len(list(Path(f"/proc/{pid}/fd").iterdir()))
    except OSError:
        fds = 0
    return {
        "vm_rss_bytes": kib_value(status.get("VmRSS")),
        "vm_hwm_bytes": kib_value(status.get("VmHWM")),
        "smaps_rss_bytes": kib_value(smaps.get("Rss")),
        "smaps_pss_bytes": kib_value(smaps.get("Pss")),
        "smaps_anon_bytes": kib_value(smaps.get("Pss_Anon") or smaps.get("Anonymous")),
        "smaps_file_bytes": kib_value(smaps.get("Pss_File")),
        "threads": int(status.get("Threads", "0")),
        "fds": fds,
    }


def cgroup_directory(pid: int) -> Path | None:
    path = Path(f"/proc/{pid}/cgroup")
    if not path.is_file():
        return None
    for line in path.read_text(encoding="ascii").splitlines():
        hierarchy, _controllers, relative = line.split(":", 2)
        if hierarchy == "0":
            candidate = Path("/sys/fs/cgroup") / relative.lstrip("/")
            return candidate if candidate.is_dir() else None
    return None


def available(value: int | None, reason: str) -> dict[str, Any]:
    return {"value": value, "unavailable_reason": None if value is not None else reason}


def cgroup_snapshot(pid: int) -> dict[str, Any]:
    directory = cgroup_directory(pid)
    stats = key_values(directory / "memory.stat") if directory else {}
    reason = "cgroup-v2 memory controller unavailable"
    return {
        "memory_current_bytes": available(read_optional_int(directory / "memory.current") if directory else None, reason),
        "memory_peak_bytes": available(read_optional_int(directory / "memory.peak") if directory else None, reason),
        "anon_bytes": available(int(stats["anon"]) if "anon" in stats else None, reason),
        "file_bytes": available(int(stats["file"]) if "file" in stats else None, reason),
        "slab_bytes": available(int(stats["slab"]) if "slab" in stats else None, reason),
    }


def percentile(values: list[int], fraction: float) -> int:
    if not values:
        return 0
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, max(0, math.ceil(len(ordered) * fraction) - 1))]


class Workload:
    def __init__(self, stream: socket.socket, job: dict[str, Any], rehearsal: bool):
        self.stream = stream
        self.job = job
        self.rehearsal = rehearsal
        self.dimensions = job["dimensions"]
        self.live: set[int] = set()
        self.latencies: list[int] = []
        self.requests = 0
        self.errors = 0
        self.value_bytes = int(self.dimensions.get("value_bytes", 256))
        requested_keys = int(self.dimensions.get("keys", 10_000))
        self.keys = min(requested_keys, 32) if rehearsal else requested_keys

    def call(self, *parts: bytes) -> Any:
        started = time.monotonic_ns()
        try:
            return resp_command(self.stream, *parts)
        except Exception:
            self.errors += 1
            raise
        finally:
            self.latencies.append(time.monotonic_ns() - started)
            self.requests += 1

    def key(self, index: int) -> bytes:
        return f"memory:{index:016d}".encode()

    def run_phase(self, phase: str, admin_port: int) -> None:
        case_id = self.job["case_id"]
        if case_id == "M0-cold":
            time.sleep(0.01 if self.rehearsal else (300 if phase == "cold" else 0))
            return
        if phase in {"cold", "post_idle", "shutdown"}:
            if case_id == "M3-ttl" and phase == "post_idle":
                time.sleep(0.3 if self.rehearsal else 61)
                self.live.clear()
                return
            if case_id == "M4-reset" and phase == "shutdown":
                if http_json(admin_port, "/admin/diagnostics/reset", "POST") is None:
                    for index in list(self.live):
                        self.call(b"DEL", self.key(index))
                self.live.clear()
                return
            delay = 0.01 if self.rehearsal else (300 if phase in {"cold", "post_idle"} else 0)
            time.sleep(delay)
            return
        if phase == "fill":
            for index in range(self.keys):
                args = [b"SET", self.key(index), bytes([index % 251]) * self.value_bytes]
                if case_id == "M3-ttl":
                    args.extend([b"PX", b"250"] if self.rehearsal else [b"EX", b"60"])
                self.call(*args)
                self.live.add(index)
                tags = self.dimensions.get("tags_per_entry", 0)
                if isinstance(tags, int) and tags:
                    self.call(b"HC.SETTAGS", self.key(index), *[f"tag-{item}".encode() for item in range(tags)])
            return
        if phase == "steady":
            cycles = min(int(self.dimensions.get("cycles", 1)), 2) if self.rehearsal else int(self.dimensions.get("cycles", 1))
            for _cycle in range(cycles):
                for index in range(self.keys):
                    if case_id == "M2-rewrite":
                        self.call(b"SET", self.key(index), bytes([(_cycle + index) % 251]) * self.value_bytes)
                    else:
                        self.call(b"GET", self.key(index))
            return
        if phase == "expire_or_delete":
            if case_id == "M3-ttl":
                time.sleep(0.3 if self.rehearsal else 61)
            else:
                for index in range(0, self.keys, 64):
                    removed = range(index, min(self.keys, index + 64))
                    self.call(b"DEL", *[self.key(item) for item in removed])
            self.live.clear()
            return
        if phase == "reset":
            reset = http_json(admin_port, "/admin/diagnostics/reset", "POST")
            if reset is None:
                for index in list(self.live):
                    self.call(b"DEL", self.key(index))
            self.live.clear()
            return
        if phase == "refill":
            for index in range(max(1, self.keys // 2)):
                arguments = [b"SET", self.key(index), bytes([index % 251]) * self.value_bytes]
                if case_id == "M3-ttl":
                    arguments.extend([b"PX", b"250"] if self.rehearsal else [b"EX", b"60"])
                self.call(*arguments)
                self.live.add(index)

    def performance(self, elapsed_ns: int) -> dict[str, Any]:
        return {
            "rps": 0.0 if elapsed_ns <= 0 else self.requests / (elapsed_ns / 1_000_000_000),
            "p50_ns": percentile(self.latencies, 0.50),
            "p95_ns": percentile(self.latencies, 0.95),
            "p99_ns": percentile(self.latencies, 0.99),
            "max_ns": max(self.latencies, default=0),
            "errors": self.errors,
            "timeouts": 0,
            "retries": 0,
            "cpu_seconds": 0.0,
            "context_switches": 0,
        }


def logical_snapshot(document: dict[str, Any] | None, workload: Workload) -> dict[str, int]:
    footprint = (document or {}).get("embedded_cache", {})
    external = (document or {}).get("client_surface") or {}
    entries = int(footprint.get("live_entries", len(workload.live)))
    return {
        "entries": entries,
        "key_bytes": int(footprint.get("logical_key_bytes", sum(len(workload.key(item)) for item in workload.live))),
        "value_bytes": int(footprint.get("logical_value_bytes", entries * workload.value_bytes)),
        "tag_records": int(footprint.get("tag_memberships", 0)),
        "tag_bytes": 0,
        "generation_records": int(footprint.get("tag_generation_records", 0)) + int(footprint.get("key_generation_records", 0)),
        "generation_bytes": 0,
        "event_records": int(footprint.get("event_ring_occupancy", 0)),
        "event_bytes": int(footprint.get("event_ring_bytes") or 0),
        "idempotency_records": int(external.get("idempotency_outcomes", 0)),
        "idempotency_bytes": int(external.get("idempotency_identity_bytes", 0)),
        "audit_records": int(external.get("audit_events", 0)),
        "audit_bytes": 0,
        "pending": int(footprint.get("pending_loads", 0)),
        "subscriptions": int(((document or {}).get("hc2") or {}).get("active_subscriptions", 0)),
        "sessions": int(((document or {}).get("hc2") or {}).get("active_sessions", 0)),
    }


def provider_command(adapter: Path, *arguments: str, allow_unavailable: bool = False) -> None:
    completed = subprocess.run([sys.executable, str(adapter), *arguments], check=False)
    if completed.returncode != 0 and not (allow_unavailable and completed.returncode == 3):
        raise ExecutionError(f"provider command failed ({completed.returncode}): {' '.join(arguments)}")


def stop_process(process: subprocess.Popen[Any]) -> None:
    if process.poll() is not None:
        return
    if os.name == "nt":
        process.send_signal(signal.CTRL_BREAK_EVENT)
    else:
        os.killpg(process.pid, signal.SIGINT)
    try:
        process.wait(timeout=15)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def execute(args: argparse.Namespace) -> None:
    job = json.loads(args.job.read_text(encoding="utf-8"))
    manifest = json.loads(args.build_manifest.read_text(encoding="utf-8"))
    binary = Path(manifest["binary"])
    if not binary.is_file():
        raise ExecutionError("retained daemon binary is missing")
    if sha256(binary) != manifest["binary_sha256"]:
        raise ExecutionError("retained daemon binary digest differs from its build manifest")
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    redis_port, admin_port = free_port(), free_port()
    environment = os.environ.copy()
    environment.update(
        {
            "HYDRACACHE_ROLE": "local",
            "HYDRACACHE_REDIS_API_ENABLED": "true",
            "HYDRACACHE_REDIS_ADDR": f"127.0.0.1:{redis_port}",
            "HYDRACACHE_ADMIN_API_ENABLED": "true",
            "HYDRACACHE_ADMIN_ADDR": f"127.0.0.1:{admin_port}",
            "HYDRACACHE_DIAGNOSTIC_RESET_ENABLED": "true",
        }
    )
    daemon_log = (output / "daemon.log").open("w", encoding="utf-8", newline="\n")
    process = subprocess.Popen(
        [str(binary)],
        env=environment,
        stdout=daemon_log,
        stderr=subprocess.STDOUT,
        start_new_session=True,
        creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0,
    )
    adapter = Path(__file__).with_name("memory-providers") / f"{args.provider}.py"
    provider_state = output / "provider-state.json"
    provider_raw = output / "provider-raw.jsonl"
    timeline_path = output / "phase-timeline.jsonl"
    checkpoints: list[dict[str, Any]] = []
    completed = False
    try:
        wait_ready(process, redis_port)
        provider_command(adapter, "probe", "--output", str(output / "provider-probe.json"))
        provider_command(
            adapter,
            "start",
            "--state",
            str(provider_state),
            "--raw",
            str(provider_raw),
            "--pid",
            str(process.pid),
            "--binary",
            str(binary),
        )
        with socket.create_connection(("127.0.0.1", redis_port), timeout=5) as stream:
            stream.settimeout(30)
            workload = Workload(stream, job, args.rehearsal)
            started = time.monotonic_ns()
            for sequence, phase in enumerate(PHASES, 1):
                workload.run_phase(phase, admin_port)
                provider_command(adapter, "mark", "--state", str(provider_state), "--phase", phase)
                provider_command(adapter, "snapshot", "--state", str(provider_state), "--phase", phase)
                monotonic_ns = time.monotonic_ns() - started
                owner = http_json(admin_port, "/admin/memory-footprint")
                write_json(output / "owner-snapshots" / f"{phase}.json", owner or {"unavailable": "B0 has no W1 admin footprint"})
                checkpoint = {
                    "phase": phase,
                    "sequence": sequence,
                    "monotonic_ns": max(1, monotonic_ns),
                    "logical": logical_snapshot(owner, workload),
                    "process": process_snapshot(process.pid),
                    "cgroup": cgroup_snapshot(process.pid),
                    "allocator": {
                        name: available(None, f"{args.provider} adapter does not expose comparable {name}")
                        for name in ("allocated_bytes", "active_bytes", "resident_bytes", "retained_bytes", "mapped_bytes")
                    },
                    "performance": workload.performance(monotonic_ns),
                }
                checkpoints.append(checkpoint)
                with timeline_path.open("a", encoding="utf-8", newline="\n") as timeline:
                    timeline.write(json.dumps({"phase": phase, "sequence": sequence, "monotonic_ns": checkpoint["monotonic_ns"]}, sort_keys=True) + "\n")
        provider_command(adapter, "stop", "--state", str(provider_state))
        provider_command(
            adapter,
            "normalize",
            "--input",
            str(provider_raw),
            "--timeline",
            str(timeline_path),
            "--output",
            str(output / "provider-normalized.json"),
        )
        host = json.loads(args.host_preflight.read_text(encoding="utf-8")) if args.host_preflight else {}
        eligible = not args.rehearsal and host.get("ship_evidence_eligible") is True
        report = {
            "schema_version": 1,
            "release": "0.71",
            "cohort": job["cohort"],
            "source_sha": manifest["source_sha"],
            "binary_sha256": manifest["binary_sha256"],
            "scenario_digest": args.scenario_digest,
            "host_fingerprint": host.get("host_fingerprint", "diagnostic-unadmitted-host"),
            "build": {
                "profile": manifest["build_profile"],
                "features": manifest["features"],
                "allocator": manifest["allocator"],
                "image_digest": None,
                "kernel": os.uname().release if hasattr(os, "uname") else sys.platform,
                "service_profile": "memory-reference-071-v1",
                "affinity": environment.get("HYDRACACHE_MEMORY_DAEMON_CPUSET", "unbound-diagnostic"),
                "cgroup_limit": None,
            },
            "allocator": {"name": manifest["allocator"], "provider": args.provider, "provider_version": "provider-protocol-v1"},
            "exact_command": [
                f"HYDRACACHE_ROLE={environment['HYDRACACHE_ROLE']}",
                f"HYDRACACHE_REDIS_API_ENABLED={environment['HYDRACACHE_REDIS_API_ENABLED']}",
                f"HYDRACACHE_REDIS_ADDR={environment['HYDRACACHE_REDIS_ADDR']}",
                f"HYDRACACHE_ADMIN_API_ENABLED={environment['HYDRACACHE_ADMIN_API_ENABLED']}",
                f"HYDRACACHE_ADMIN_ADDR={environment['HYDRACACHE_ADMIN_ADDR']}",
                f"HYDRACACHE_DIAGNOSTIC_RESET_ENABLED={environment['HYDRACACHE_DIAGNOSTIC_RESET_ENABLED']}",
                str(binary),
            ],
            "unique_keys": 0 if job["case_id"] == "M0-cold" else workload.keys,
            "unique_key_verification": {
                "method": "workload-key-ledger",
                "observed": 0 if job["case_id"] == "M0-cold" else workload.keys,
            },
            "request_count": workload.requests,
            "diagnostic_only": not eligible,
            "ship_evidence_eligible": eligible,
            "checkpoints": checkpoints,
        }
        write_json(output / "memory-baseline-report.json", report)
        completed = True
    finally:
        stop_process(process)
        daemon_log.close()
    if completed:
        write_json(
            output / "executor-receipt.json",
            {
                "schema_version": 1,
                "release": "0.71",
                "job_id": job["job_id"],
                "cohort": job["cohort"],
                "pid": process.pid,
                "fresh_process": True,
                "binary_sha256": manifest["binary_sha256"],
                "stopped": process.poll() is not None,
                "exit_code": process.returncode,
            },
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--job", type=Path, required=True)
    parser.add_argument("--build-manifest", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--scenario-digest", required=True)
    parser.add_argument("--provider", choices=["system", "jemalloc", "mimalloc"], default="system")
    parser.add_argument("--host-preflight", type=Path)
    parser.add_argument("--rehearsal", action="store_true")
    return parser.parse_args()


def main() -> int:
    try:
        execute(parse_args())
        return 0
    except (ExecutionError, OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"memory case executor 0.71: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
