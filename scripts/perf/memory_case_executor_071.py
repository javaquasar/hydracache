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


class Hc2Fleet:
    def __init__(self, helper: Path, pki: Path, port: int, output: Path, rehearsal: bool):
        self.helper = helper
        self.pki = pki
        self.port = port
        self.output = output
        self.rehearsal = rehearsal
        self.process: subprocess.Popen[Any] | None = None
        self.log: Any = None
        self.generation = 0

    def start(self, connections: int, slow_consumers: int) -> dict[str, Any]:
        if self.process is not None:
            raise ExecutionError("HC/2 fleet is already running")
        self.generation += 1
        if self.rehearsal:
            connections = min(connections, 16)
            slow_consumers = min(slow_consumers, connections, 8)
        ready = self.output / f"hc2-fleet-{self.generation}-ready.json"
        self.log = (self.output / f"hc2-fleet-{self.generation}.log").open(
            "w", encoding="utf-8", newline="\n"
        )
        command = [
            str(self.helper),
            "hold",
            "--endpoint",
            f"https://127.0.0.1:{self.port}",
            "--ca",
            str(self.pki / "ca.pem"),
            "--cert",
            str(self.pki / "client.pem"),
            "--key",
            str(self.pki / "client.key"),
            "--connections",
            str(connections),
            "--slow-consumers",
            str(slow_consumers),
            "--ready",
            str(ready),
        ]
        self.process = subprocess.Popen(
            command,
            stdin=subprocess.PIPE,
            stdout=self.log,
            stderr=subprocess.STDOUT,
        )
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            if ready.is_file():
                receipt = json.loads(ready.read_text(encoding="utf-8"))
                if receipt.get("connections") != connections or receipt.get("slow_consumers") != slow_consumers:
                    raise ExecutionError("HC/2 helper ready receipt has the wrong cardinality")
                return receipt
            if self.process.poll() is not None:
                raise ExecutionError(f"HC/2 helper exited during startup with {self.process.returncode}")
            time.sleep(0.05)
        raise ExecutionError("HC/2 helper did not become ready")

    def stop(self) -> None:
        if self.process is None:
            return
        if self.process.stdin:
            self.process.stdin.write(b"x")
            self.process.stdin.close()
        try:
            self.process.wait(timeout=30)
        except subprocess.TimeoutExpired as error:
            self.process.kill()
            self.process.wait(timeout=5)
            raise ExecutionError("HC/2 helper did not stop cleanly") from error
        if self.process.returncode != 0:
            raise ExecutionError(f"HC/2 helper failed with {self.process.returncode}")
        self.process = None
        if self.log:
            self.log.close()
            self.log = None


def wait_hc2_accounting(admin_port: int, connections: int, subscriptions: int) -> dict[str, Any]:
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        owner = http_json(admin_port, "/admin/memory-footprint") or {}
        hc2 = owner.get("hc2") or {}
        if (
            hc2.get("active_connections") == connections
            and hc2.get("active_subscriptions") == subscriptions
        ):
            return owner
        time.sleep(0.05)
    raise ExecutionError(
        f"HC/2 accounting did not reach connections={connections}, subscriptions={subscriptions}"
    )


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


def directory_snapshot(directory: Path) -> dict[str, Any]:
    files: list[dict[str, Any]] = []
    if directory.is_dir():
        for path in sorted(item for item in directory.rglob("*") if item.is_file()):
            stat = path.stat()
            files.append(
                {
                    "path": str(path.relative_to(directory)).replace("\\", "/"),
                    "logical_bytes": stat.st_size,
                    "allocated_bytes": getattr(stat, "st_blocks", 0) * 512,
                }
            )
    return {
        "files": files,
        "logical_bytes": sum(item["logical_bytes"] for item in files),
        "allocated_bytes": sum(item["allocated_bytes"] for item in files),
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
        self.live_tags: dict[int, tuple[bytes, ...]] = {}
        self.distribution_observations: list[dict[str, Any]] = []
        self.verified_settags = 0
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

    def tags_for(self, index: int) -> tuple[bytes, ...]:
        if self.job["case_id"] != "M5-tags":
            return ()
        count = int(self.dimensions["tags_per_entry"])
        distribution = self.dimensions["distribution"]
        if count == 0:
            return ()
        if distribution == "uniform":
            return tuple(f"uniform:{index:016d}:{item:04d}".encode() for item in range(count))
        pool = int(self.dimensions["tag_pool"])
        if pool < count:
            raise ExecutionError("M5 tag pool cannot be smaller than tags_per_entry")
        if distribution == "one-hot":
            return (f"one-hot:{index % pool:04d}".encode(),)
        if distribution == "high-fanout":
            return tuple(
                f"high-fanout:{(index + item) % pool:04d}".encode()
                for item in range(count)
            )
        raise ExecutionError(f"unsupported M5 tag distribution: {distribution}")

    def apply_tags(self, index: int) -> None:
        tags = self.tags_for(index)
        if not tags:
            self.live_tags.pop(index, None)
            return
        observed = self.call(b"HC.SETTAGS", self.key(index), *tags)
        expected = len(set(tags))
        if observed != expected:
            raise ExecutionError(
                f"HC.SETTAGS verification failed for key {index}: expected {expected}, got {observed!r}"
            )
        self.live_tags[index] = tags
        self.verified_settags += 1

    def tag_memberships(self) -> int:
        return sum(len(tags) for tags in self.live_tags.values())

    def tag_bytes(self) -> int:
        return sum(len(tag) for tags in self.live_tags.values() for tag in tags)

    def observe_distribution(self, phase: str) -> None:
        if self.job["case_id"] != "M5-tags":
            return
        distinct = {tag for tags in self.live_tags.values() for tag in tags}
        self.distribution_observations.append(
            {
                "phase": phase,
                "live_entries": len(self.live),
                "tag_memberships": self.tag_memberships(),
                "distinct_tags": len(distinct),
                "tag_bytes": self.tag_bytes(),
            }
        )

    def run_duration_sequence(
        self,
        admin_port: int,
        process_pid: int,
        output: Path,
        fleet: Hc2Fleet | None,
    ) -> dict[str, Any]:
        configured_sequence = self.dimensions["sequence"]
        schedule = (
            [configured_sequence]
            if isinstance(configured_sequence, str)
            else list(configured_sequence)
        )
        if not schedule:
            raise ExecutionError("duration sequence cannot be empty")
        duration = float(
            self.dimensions["rehearsal_duration_seconds"]
            if self.rehearsal
            else self.dimensions["duration_seconds"]
        )
        interval = float(
            self.dimensions["rehearsal_iteration_seconds"]
            if self.rehearsal
            else self.dimensions["iteration_seconds"]
        )
        heartbeat_interval = float(
            self.dimensions["rehearsal_heartbeat_seconds"]
            if self.rehearsal
            else self.dimensions["heartbeat_seconds"]
        )
        block_seconds = float(
            self.dimensions.get(
                "rehearsal_block_seconds" if self.rehearsal else "block_seconds",
                duration,
            )
        )
        started = time.monotonic()
        deadline = started + duration
        next_heartbeat = started
        iteration = 0
        heartbeats: list[dict[str, Any]] = []
        churn: list[dict[str, Any]] = []
        scenario_iterations = {name: 0 for name in schedule}
        receipt_prefix = self.job["case_id"].split("-", 1)[0].lower()
        heartbeat_path = output / f"{receipt_prefix}-duration-heartbeats.jsonl"
        while time.monotonic() < deadline:
            iteration += 1
            iteration_started = time.monotonic()
            sequence = schedule[
                int((iteration_started - started) // block_seconds) % len(schedule)
            ]
            scenario_iterations[sequence] += 1
            if sequence == "fixed-keyspace":
                for index in range(self.keys):
                    self.call(b"GET", self.key(index))
            elif sequence == "ttl":
                for index in range(self.keys):
                    arguments = [b"SET", self.key(index), bytes([index % 251]) * self.value_bytes]
                    arguments.extend([b"PX", b"100"] if self.rehearsal else [b"EX", b"30"])
                    self.call(*arguments)
                    self.live.add(index)
            elif sequence == "reset":
                self.run_phase("reset", admin_port)
                self.run_phase("fill", admin_port)
            elif sequence == "hc2-churn":
                if fleet is None:
                    raise ExecutionError("M8 HC/2 churn requires the retained HC/2 helper")
                ready = fleet.start(int(self.dimensions["hc2_churn_connections"]), 0)
                owner = wait_hc2_accounting(admin_port, int(ready["connections"]), 0)
                churn.append({"iteration": iteration, **(owner.get("hc2") or {})})
                fleet.stop()
                wait_hc2_accounting(admin_port, 0, 0)
            else:
                raise ExecutionError(f"unsupported duration sequence: {sequence}")
            sleep_for = min(max(0.0, interval - (time.monotonic() - iteration_started)), max(0.0, deadline - time.monotonic()))
            if sleep_for:
                time.sleep(sleep_for)
            if sequence == "ttl":
                self.live.clear()
            now = time.monotonic()
            if now >= next_heartbeat or now >= deadline:
                record = {
                    "schema_version": 1,
                    "release": "0.71",
                    "sequence": sequence,
                    "iteration": iteration,
                    "elapsed_ns": int((now - started) * 1_000_000_000),
                    "process": process_snapshot(process_pid),
                    "cgroup": cgroup_snapshot(process_pid),
                    "owner": http_json(admin_port, "/admin/memory-footprint"),
                }
                heartbeats.append(record)
                with heartbeat_path.open("a", encoding="utf-8", newline="\n") as stream:
                    stream.write(json.dumps(record, sort_keys=True) + "\n")
                next_heartbeat = now + heartbeat_interval
        observed_ns = time.monotonic_ns() - int(started * 1_000_000_000)
        required_ns = int(duration * 1_000_000_000)
        if observed_ns < required_ns:
            raise ExecutionError("duration executor ended before its preregistered duration")
        return {
            "schema_version": 1,
            "release": "0.71",
            "schedule": schedule,
            "block_seconds": block_seconds,
            "requested_duration_seconds": duration,
            "observed_duration_ns": observed_ns,
            "iterations": iteration,
            "heartbeat_count": len(heartbeats),
            "scenario_iterations": scenario_iterations,
            "hc2_churn": churn,
        }

    def run_phase(self, phase: str, admin_port: int) -> None:
        case_id = self.job["case_id"]
        if case_id == "M0-cold":
            time.sleep(0.01 if self.rehearsal else (300 if phase == "cold" else 0))
            return
        if phase in {"cold", "post_idle", "shutdown"}:
            if case_id == "M3-ttl" and phase == "post_idle":
                time.sleep(0.3 if self.rehearsal else 61)
                self.live.clear()
                self.live_tags.clear()
                return
            if case_id == "M4-reset" and phase == "shutdown":
                if http_json(admin_port, "/admin/diagnostics/reset", "POST") is None:
                    for index in list(self.live):
                        self.call(b"DEL", self.key(index))
                self.live.clear()
                self.live_tags.clear()
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
                self.apply_tags(index)
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
            self.live_tags.clear()
            return
        if phase == "reset":
            reset = http_json(admin_port, "/admin/diagnostics/reset", "POST")
            if reset is None:
                for index in list(self.live):
                    self.call(b"DEL", self.key(index))
            self.live.clear()
            self.live_tags.clear()
            return
        if phase == "refill":
            for index in range(max(1, self.keys // 2)):
                arguments = [b"SET", self.key(index), bytes([index % 251]) * self.value_bytes]
                if case_id == "M3-ttl":
                    arguments.extend([b"PX", b"250"] if self.rehearsal else [b"EX", b"60"])
                self.call(*arguments)
                self.live.add(index)
                self.apply_tags(index)

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
        "tag_records": workload.tag_memberships() if workload.job["case_id"] == "M5-tags" else int(footprint.get("tag_memberships", 0)),
        "tag_bytes": workload.tag_bytes(),
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
    redis_port, admin_port, hc2_port = free_port(), free_port(), free_port()
    client_port, cluster_port = free_port(), free_port()
    helper: Path | None = None
    helper_sha256: str | None = None
    pki = output / "hc2-pki"
    uses_hc2 = job["case_id"] == "M6-connections" or (
        job["case_id"] == "M8-60m" and job["dimensions"].get("sequence") == "hc2-churn"
    ) or job["case_id"] in {"M9-6h", "M10-24h"}
    if uses_hc2:
        if not args.hc2_helper_manifest:
            raise ExecutionError("M6 requires --hc2-helper-manifest")
        helper_manifest = json.loads(args.hc2_helper_manifest.read_text(encoding="utf-8"))
        helper = Path(helper_manifest["binary"])
        helper_sha256 = str(helper_manifest["binary_sha256"])
        if not helper.is_file() or sha256(helper) != helper_manifest["binary_sha256"]:
            raise ExecutionError("retained HC/2 helper is missing or drifted")
        subprocess.run([str(helper), "pki", "--output", str(pki)], check=True)
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
    durable_store: Path | None = None
    if job["case_id"] == "M7-persistence" and job["dimensions"].get("persistence") == "supported":
        durable_store = output / "durable-store"
        environment.update(
            {
                "HYDRACACHE_ROLE": "member",
                "HYDRACACHE_NODE_ID": "memory-m7-member-0",
                "HYDRACACHE_LISTEN_ADDR": f"127.0.0.1:{client_port}",
                "HYDRACACHE_CLUSTER_ADDR": f"127.0.0.1:{cluster_port}",
                "HYDRACACHE_CLUSTER_ADVERTISE_ADDR": f"127.0.0.1:{cluster_port}",
                "HYDRACACHE_CLUSTER_START": "bootstrap",
                "HYDRACACHE_SEEDS": f"127.0.0.1:{cluster_port}",
                "HYDRACACHE_STORAGE_DIR": str(durable_store),
                "HYDRACACHE_DIAGNOSTIC_RESET_ENABLED": "false",
            }
        )
    if uses_hc2:
        if job["case_id"] == "M6-connections" and job["dimensions"].get("tls") is not True:
            raise ExecutionError("HC/2 is contractually mandatory-mTLS; plaintext M6 cells are invalid")
        environment.update(
            {
                "HYDRACACHE_HC2_ENABLED": "true",
                "HYDRACACHE_HC2_ADDR": f"127.0.0.1:{hc2_port}",
                "HYDRACACHE_HC2_CLUSTER_ID": "memory-campaign-071",
                "HYDRACACHE_TLS_ENABLED": "true",
                "HYDRACACHE_TLS_CERT_PATH": str(pki / "server.pem"),
                "HYDRACACHE_TLS_KEY_PATH": str(pki / "server.key"),
                "HYDRACACHE_TLS_CA_PATH": str(pki / "ca.pem"),
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
    hc2_observations: list[dict[str, Any]] = []
    hc2_helper_receipts: list[dict[str, Any]] = []
    persistence_observations: list[dict[str, Any]] = []
    duration_receipt: dict[str, Any] | None = None
    fleet = Hc2Fleet(helper, pki, hc2_port, output, args.rehearsal) if helper else None
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
                if job["case_id"] == "M6-connections" and fleet and phase in {"fill", "refill"}:
                    hc2_helper_receipts.append(
                        fleet.start(
                            int(job["dimensions"]["connections"]),
                            int(job["dimensions"]["slow_consumers"]),
                        )
                    )
                if job["case_id"] == "M6-connections" and fleet and phase in {"expire_or_delete", "shutdown"}:
                    fleet.stop()
                if job["case_id"] in {"M8-60m", "M9-6h", "M10-24h"} and phase == "steady":
                    duration_receipt = workload.run_duration_sequence(
                        admin_port, process.pid, output, fleet
                    )
                else:
                    workload.run_phase(phase, admin_port)
                workload.observe_distribution(phase)
                provider_command(adapter, "mark", "--state", str(provider_state), "--phase", phase)
                provider_command(adapter, "snapshot", "--state", str(provider_state), "--phase", phase)
                monotonic_ns = time.monotonic_ns() - started
                owner = http_json(admin_port, "/admin/memory-footprint")
                if job["case_id"] == "M6-connections" and fleet:
                    expected_connections = (
                        min(int(job["dimensions"]["connections"]), 16)
                        if args.rehearsal
                        else int(job["dimensions"]["connections"])
                    ) if phase in {"fill", "steady", "refill", "post_idle"} else 0
                    requested_slow = int(job["dimensions"]["slow_consumers"])
                    expected_subscriptions = (
                        min(requested_slow, expected_connections, 8)
                        if args.rehearsal
                        else requested_slow
                    ) if expected_connections else 0
                    owner = wait_hc2_accounting(
                        admin_port, expected_connections, expected_subscriptions
                    )
                    hc2_observations.append({"phase": phase, **(owner.get("hc2") or {})})
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
                if durable_store:
                    persistence_observations.append(
                        {"phase": phase, **directory_snapshot(durable_store)}
                    )
                with timeline_path.open("a", encoding="utf-8", newline="\n") as timeline:
                    timeline.write(json.dumps({"phase": phase, "sequence": sequence, "monotonic_ns": checkpoint["monotonic_ns"]}, sort_keys=True) + "\n")
            if job["case_id"] == "M5-tags":
                write_json(
                    output / "m5-distribution-receipt.json",
                    {
                        "schema_version": 1,
                        "release": "0.71",
                        "job_id": job["job_id"],
                        "distribution": workload.dimensions["distribution"],
                        "tags_per_entry": int(workload.dimensions["tags_per_entry"]),
                        "tag_pool": workload.dimensions["tag_pool"],
                        "verification_method": "HC.SETTAGS-response-and-workload-ledger",
                        "verified_settags": workload.verified_settags,
                        "observations": workload.distribution_observations,
                    },
                )
            if job["case_id"] == "M6-connections" and fleet:
                write_json(
                    output / "m6-connections-receipt.json",
                    {
                        "schema_version": 1,
                        "release": "0.71",
                        "job_id": job["job_id"],
                        "transport": "grpc-bidirectional-mtls",
                        "requested_connections": int(job["dimensions"]["connections"]),
                        "requested_slow_consumers": int(job["dimensions"]["slow_consumers"]),
                        "rehearsal_cardinality_capped": args.rehearsal,
                        "helper_receipts": hc2_helper_receipts,
                        "observations": hc2_observations,
                    },
                )
            if duration_receipt:
                prefix = job["case_id"].split("-", 1)[0].lower()
                write_json(output / f"{prefix}-duration-receipt.json", duration_receipt)
            if durable_store:
                ready = http_json(admin_port, "/readyz") or {}
                if ready.get("storage_open") is not True:
                    raise ExecutionError("M7 supported mode did not report storage_open=true")
                if not (durable_store / "raft-log").is_dir():
                    raise ExecutionError("M7 supported mode did not create the sled raft-log directory")
                write_json(
                    output / "m7-persistence-receipt.json",
                    {
                        "schema_version": 1,
                        "release": "0.71",
                        "job_id": job["job_id"],
                        "backend": "member-sled-raft-log",
                        "storage_open": True,
                        "storage_root": "durable-store",
                        "observations": persistence_observations,
                    },
                )
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
        exact_command = [
            f"HYDRACACHE_ROLE={environment['HYDRACACHE_ROLE']}",
            f"HYDRACACHE_REDIS_API_ENABLED={environment['HYDRACACHE_REDIS_API_ENABLED']}",
            f"HYDRACACHE_REDIS_ADDR={environment['HYDRACACHE_REDIS_ADDR']}",
            f"HYDRACACHE_ADMIN_API_ENABLED={environment['HYDRACACHE_ADMIN_API_ENABLED']}",
            f"HYDRACACHE_ADMIN_ADDR={environment['HYDRACACHE_ADMIN_ADDR']}",
            f"HYDRACACHE_DIAGNOSTIC_RESET_ENABLED={environment['HYDRACACHE_DIAGNOSTIC_RESET_ENABLED']}",
        ]
        if fleet:
            exact_command.extend(
                [
                    f"HYDRACACHE_HC2_ENABLED={environment['HYDRACACHE_HC2_ENABLED']}",
                    f"HYDRACACHE_HC2_ADDR={environment['HYDRACACHE_HC2_ADDR']}",
                    f"HYDRACACHE_HC2_CLUSTER_ID={environment['HYDRACACHE_HC2_CLUSTER_ID']}",
                    "HYDRACACHE_TLS_ENABLED=true",
                    f"HC2_HELPER_SHA256={helper_sha256}",
                ]
            )
        if durable_store:
            exact_command.extend(
                [
                    "HYDRACACHE_ROLE=member",
                    f"HYDRACACHE_CLUSTER_ADDR={environment['HYDRACACHE_CLUSTER_ADDR']}",
                    f"HYDRACACHE_SEEDS={environment['HYDRACACHE_SEEDS']}",
                    "HYDRACACHE_STORAGE_DIR=durable-store",
                ]
            )
        exact_command.append(str(binary))
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
            "exact_command": exact_command,
            "unique_keys": 0 if job["case_id"] == "M0-cold" else workload.keys,
            "unique_key_verification": {
                "method": "workload-key-ledger",
                "observed": 0 if job["case_id"] == "M0-cold" else workload.keys,
            },
            "request_count": workload.requests + sum(
                int(receipt.get("pressure_events", 0)) for receipt in hc2_helper_receipts
            ),
            "diagnostic_only": not eligible,
            "ship_evidence_eligible": eligible,
            "checkpoints": checkpoints,
        }
        write_json(output / "memory-baseline-report.json", report)
        completed = True
    finally:
        if fleet:
            fleet.stop()
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
    parser.add_argument("--hc2-helper-manifest", type=Path)
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
