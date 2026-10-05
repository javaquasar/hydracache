#!/usr/bin/env python3
"""Run the preregistered W9c Linux scheduler/socket owner-attribution matrix."""

from __future__ import annotations

import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import statistics
import subprocess
import sys
import threading
import time
from typing import Any


PROFILE_ID = "w9c-linux-kernel-attribution-074-v1"
PROFILER_ID = "w1-w3-resp-pipeline-profile-074-v3"
SCHEMA_VERSION = "hydracache-w9c-linux-kernel-attribution-v1"
RUSAGE_SCHEMA_VERSION = "hydracache-w9c-rusage-v1"
OPERATIONS = 40_000
WARMUP_OPERATIONS = 8_000
PAYLOAD_BYTES = 64
KEY_SPACE = 4_096
SEED = 7_409
REPEATS = 5
CELLS = (
    ("get", 1, 1),
    ("get", 10, 1),
    ("get", 10, 8),
    ("set", 1, 1),
    ("set", 10, 1),
    ("set", 10, 8),
)
READ_SYSCALLS = frozenset(("read", "readv", "recvfrom", "recvmsg", "recvmmsg"))
WRITE_SYSCALLS = frozenset(("write", "writev", "sendto", "sendmsg", "sendmmsg"))
EPOLL_SYSCALLS = frozenset(("epoll_wait", "epoll_pwait", "epoll_pwait2"))
TRACE_SYSCALLS = "%network,read,write,readv,writev,epoll_wait,epoll_pwait,epoll_pwait2,futex"
TRACE_LINE = re.compile(r"^\s*(?P<timestamp>\d+\.\d+)\s+(?P<syscall>[a-zA-Z0-9_]+)\(.*\)\s+=\s+(?P<result>-?\d+)(?:\s+(?P<error>[A-Z0-9]+))?")
TCP_FLOW = re.compile(r"<TCP(?:v6)?:\[(?:127\.0\.0\.1|\[::ffff:127\.0\.0\.1\]):(?P<local>\d+)->(?:127\.0\.0\.1|\[::ffff:127\.0\.0\.1\]):(?P<peer>\d+)\]>")
LISTENER = re.compile(r"listen\([^\n]*<TCP(?:v6)?:\[(?:127\.0\.0\.1|\[::ffff:127\.0\.0\.1\]):(?P<port>\d+)\]>")


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def write_json(path: Path, value: Any) -> None:
    path.write_text(
        json.dumps(value, indent=2, sort_keys=True) + "\n",
        encoding="utf-8",
        newline="\n",
    )


def schedule_for(repeats: int) -> list[tuple[str, int, int]]:
    schedule: list[tuple[str, int, int]] = []
    for repeat in range(repeats):
        offset = repeat % len(CELLS)
        rotation = CELLS[offset:] + CELLS[:offset]
        if repeat % 2:
            rotation = tuple(reversed(rotation))
        schedule.extend(rotation)
    return schedule


def validate_receipt(
    receipt: dict[str, Any], source_commit: str, operation: str, pipeline: int, concurrency: int
) -> None:
    require(receipt.get("schema_version") == 1, "profiler schema changed")
    require(receipt.get("release") == "0.74", "release changed")
    require(receipt.get("profile_id") == PROFILER_ID, "profiler identity changed")
    require(receipt.get("source_commit") == source_commit, "source commit changed")
    require(receipt.get("promotable") is False, "attribution receipt became promotable")
    require(receipt.get("surface") == "resp-api-loopback-tcp", "surface changed")
    require(receipt.get("operation") == operation, "operation changed")
    require(receipt.get("operations") == OPERATIONS, "operations changed")
    require(receipt.get("warmup_operations") == WARMUP_OPERATIONS, "warmup changed")
    require(receipt.get("concurrency") == concurrency, "concurrency changed")
    require(receipt.get("pipeline") == pipeline, "pipeline changed")
    require(receipt.get("batch_size") == 1, "batch size changed")
    require(receipt.get("payload_bytes") == PAYLOAD_BYTES, "payload changed")
    require(receipt.get("key_space") == KEY_SPACE, "key space changed")
    require(receipt.get("seed") == SEED, "seed changed")
    require(receipt.get("instrumentation_enabled") is True, "instrumentation changed")
    require(receipt.get("transport") == "tcp", "transport changed")
    require(receipt.get("exact_response_validation") is True, "response validation failed")
    require(isinstance(receipt.get("workload_sha256"), str), "workload digest missing")
    require(receipt.get("resp", {}).get("decoded_commands") == OPERATIONS, "decoded command count changed")
    require(receipt.get("resp", {}).get("output_frames") == OPERATIONS, "output frame count changed")
    require(receipt.get("client_surface", {}).get("dispatches") == OPERATIONS, "dispatch count changed")
    socket = receipt.get("socket_io", {})
    require(socket.get("available") is True, "socket boundary counters unavailable")
    require(socket.get("written_bytes") == receipt.get("resp", {}).get("output_bytes"), "socket/app output bytes diverged")


def validate_rusage(receipt: dict[str, Any]) -> None:
    require(receipt.get("schema_version") == RUSAGE_SCHEMA_VERSION, "rusage schema changed")
    require(receipt.get("source") == "getrusage-RUSAGE_SELF", "rusage source changed")
    require(receipt.get("measurement_only") is True, "rusage escaped the measurement window")
    delta = receipt.get("delta")
    require(isinstance(delta, dict), "rusage delta missing")
    for field in (
        "user_cpu_microseconds",
        "system_cpu_microseconds",
        "minor_page_faults",
        "major_page_faults",
        "voluntary_context_switches",
        "involuntary_context_switches",
    ):
        require(isinstance(delta.get(field), int) and delta[field] >= 0, f"rusage {field} missing")


def _new_trace_metrics() -> dict[str, Any]:
    return {
        "marker_timestamp": None,
        "server_ports": [],
        "tcp": {
            side: {
                "read_calls": 0,
                "read_bytes": 0,
                "read_eagain": 0,
                "write_calls": 0,
                "write_bytes": 0,
                "write_eagain": 0,
                "zero_length_calls": 0,
            }
            for side in ("server", "client", "unknown")
        },
        "epoll_calls": 0,
        "epoll_events": 0,
        "futex_calls": 0,
        "measurement_trace_lines": 0,
    }


def parse_strace_files(paths: list[Path], ready_token: str = "w9c-ready-v1") -> dict[str, Any]:
    lines: list[str] = []
    for path in paths:
        lines.extend(path.read_text(encoding="utf-8", errors="replace").splitlines())
    marker_times = []
    listener_events: list[tuple[float, int]] = []
    for line in lines:
        parsed = TRACE_LINE.match(line)
        if ready_token in line and parsed and parsed.group("syscall") == "write":
            marker_times.append(float(parsed.group("timestamp")))
        listener = LISTENER.search(line)
        if listener and parsed:
            listener_events.append((float(parsed.group("timestamp")), int(listener.group("port"))))
    require(len(marker_times) == 1, f"expected one trace-ready marker, found {len(marker_times)}")
    server_ports = sorted({port for timestamp, port in listener_events if timestamp > marker_times[0]})
    require(server_ports, "measurement loopback listener ports missing from trace")

    metrics = _new_trace_metrics()
    metrics["marker_timestamp"] = marker_times[0]
    metrics["server_ports"] = server_ports
    for line in lines:
        parsed = TRACE_LINE.match(line)
        if not parsed or float(parsed.group("timestamp")) <= marker_times[0]:
            continue
        metrics["measurement_trace_lines"] += 1
        syscall = parsed.group("syscall")
        result = int(parsed.group("result"))
        error = parsed.group("error")
        if syscall in EPOLL_SYSCALLS:
            metrics["epoll_calls"] += 1
            metrics["epoll_events"] += max(result, 0)
        elif syscall == "futex":
            metrics["futex_calls"] += 1

        flow = TCP_FLOW.search(line)
        if flow is None or syscall not in READ_SYSCALLS | WRITE_SYSCALLS:
            continue
        local_port = int(flow.group("local"))
        peer_port = int(flow.group("peer"))
        if local_port in server_ports:
            side = "server"
        elif peer_port in server_ports:
            side = "client"
        else:
            side = "unknown"
        direction = "read" if syscall in READ_SYSCALLS else "write"
        bucket = metrics["tcp"][side]
        bucket[f"{direction}_calls"] += 1
        if result > 0:
            bucket[f"{direction}_bytes"] += result
        elif result == 0:
            bucket["zero_length_calls"] += 1
        elif error in ("EAGAIN", "EWOULDBLOCK"):
            bucket[f"{direction}_eagain"] += 1
    return metrics


def parse_ss_snapshot(output: str) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    current: dict[str, Any] | None = None
    for line in output.splitlines():
        stripped = line.strip()
        if stripped.startswith(("ESTAB ", "SYN-SENT ", "SYN-RECV ")):
            parts = stripped.split()
            if len(parts) >= 5 and "127.0.0.1:" in parts[3] and "127.0.0.1:" in parts[4]:
                current = {
                    "recv_q_bytes": int(parts[1]),
                    "send_q_bytes": int(parts[2]),
                    "local": parts[3],
                    "peer": parts[4],
                    "skmem": {},
                }
                rows.append(current)
            else:
                current = None
        elif current is not None and "skmem:(" in stripped:
            match = re.search(r"skmem:\(([^)]*)\)", stripped)
            if match:
                for token in match.group(1).split(","):
                    item = re.fullmatch(r"([a-z]+)(\d+)", token.strip())
                    if item:
                        current["skmem"][item.group(1)] = int(item.group(2))
    return rows


def socket_queue_summary(samples: list[dict[str, Any]], server_ports: list[int]) -> dict[str, Any]:
    selected = []
    suffixes = tuple(f":{port}" for port in server_ports)
    for sample in samples:
        rows = [
            row
            for row in sample["rows"]
            if row["local"].endswith(suffixes) or row["peer"].endswith(suffixes)
        ]
        selected.append({"elapsed_ns": sample["elapsed_ns"], "rows": rows})
    all_rows = [row for sample in selected for row in sample["rows"]]
    fields = ("r", "rb", "t", "tb", "f", "w", "o", "bl", "d")
    return {
        "samples": len(selected),
        "samples_with_connections": sum(bool(sample["rows"]) for sample in selected),
        "max_connections": max((len(sample["rows"]) for sample in selected), default=0),
        "max_recv_q_bytes": max((row["recv_q_bytes"] for row in all_rows), default=0),
        "max_send_q_bytes": max((row["send_q_bytes"] for row in all_rows), default=0),
        "max_skmem": {field: max((row["skmem"].get(field, 0) for row in all_rows), default=0) for field in fields},
        "selected": selected,
    }


def sample_sockets(stop: threading.Event, started_ns: int, samples: list[dict[str, Any]]) -> None:
    while not stop.is_set():
        completed = subprocess.run(
            ["ss", "-tinmH"], capture_output=True, text=True, check=False, timeout=3
        )
        samples.append(
            {
                "elapsed_ns": time.monotonic_ns() - started_ns,
                "exit_code": completed.returncode,
                "stderr": completed.stderr,
                "rows": parse_ss_snapshot(completed.stdout),
            }
        )
        stop.wait(0.02)


def compress_traces(paths: list[Path]) -> list[dict[str, Any]]:
    compressed = []
    for path in paths:
        target = path.with_suffix(path.suffix + ".gz")
        with path.open("rb") as source, gzip.open(target, "wb", compresslevel=9) as output:
            shutil.copyfileobj(source, output)
        path.unlink()
        compressed.append({"file": target.name, "sha256": sha256(target), "bytes": target.stat().st_size})
    return compressed


def run_attempt(
    binary: Path,
    source_commit: str,
    operation: str,
    pipeline: int,
    concurrency: int,
    ordinal: int,
    output: Path,
) -> dict[str, Any]:
    stem = f"{ordinal:02d}-{operation}-p{pipeline}-c{concurrency}"
    ready = output / f"{stem}.ready"
    go = output / f"{stem}.go"
    receipt_path = output / f"{stem}.receipt.json"
    rusage_path = output / f"{stem}.rusage.json"
    stdout_path = output / f"{stem}.stdout.txt"
    stderr_path = output / f"{stem}.stderr.txt"
    ss_path = output / f"{stem}.ss.json"
    trace_prefix = output / f"{stem}.strace"
    command = [
        "strace", "-ff", "-qq", "-ttt", "-yy", "-s", "256", "-e", f"trace={TRACE_SYSCALLS}",
        "-o", str(trace_prefix), str(binary),
        "--source-commit", source_commit,
        "--operation", operation,
        "--operations", str(OPERATIONS),
        "--warmup-operations", str(WARMUP_OPERATIONS),
        "--concurrency", str(concurrency),
        "--pipeline", str(pipeline),
        "--batch-size", "1",
        "--payload-bytes", str(PAYLOAD_BYTES),
        "--key-space", str(KEY_SPACE),
        "--seed", str(SEED),
        "--instrumentation", "true",
        "--transport", "tcp",
        "--output", str(receipt_path),
    ]
    environment = os.environ.copy()
    environment.update(
        {
            "HYDRACACHE_W9C_READY_FILE": str(ready),
            "HYDRACACHE_W9C_GO_FILE": str(go),
            "HYDRACACHE_W9C_RUSAGE_FILE": str(rusage_path),
        }
    )
    record: dict[str, Any] = {
        "ordinal": ordinal,
        "cell": {"operation": operation, "pipeline": pipeline, "concurrency": concurrency},
        "command": command,
        "valid": False,
        "silent_retry": False,
    }
    samples: list[dict[str, Any]] = []
    stop = threading.Event()
    sampler: threading.Thread | None = None
    process: subprocess.Popen[bytes] | None = None
    try:
        with stdout_path.open("wb") as stdout, stderr_path.open("wb") as stderr:
            process = subprocess.Popen(command, stdout=stdout, stderr=stderr, env=environment)
            deadline = time.monotonic() + 45
            while not ready.is_file() and process.poll() is None and time.monotonic() < deadline:
                time.sleep(0.01)
            require(ready.is_file(), "profiler did not reach the measurement gate")
            started_ns = time.monotonic_ns()
            sampler = threading.Thread(target=sample_sockets, args=(stop, started_ns, samples), daemon=True)
            sampler.start()
            go.touch(exist_ok=False)
            exit_code = process.wait(timeout=180)
            record["exit_code"] = exit_code
            require(exit_code == 0, f"profiler/strace exited {exit_code}")
        require(stdout_path.stat().st_size == 0, "profiler stdout is non-empty")
        require(stderr_path.stat().st_size == 0, "profiler/strace stderr is non-empty")
        stop.set()
        if sampler is not None:
            sampler.join(timeout=5)
        require(sampler is None or not sampler.is_alive(), "ss sampler did not stop")
        trace_paths = sorted(output.glob(f"{trace_prefix.name}.*"))
        require(trace_paths, "strace output missing")
        trace = parse_strace_files(trace_paths)
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        rusage = json.loads(rusage_path.read_text(encoding="utf-8"))
        validate_receipt(receipt, source_commit, operation, pipeline, concurrency)
        validate_rusage(rusage)
        require(len(trace["server_ports"]) == concurrency, "measurement listener count changed")
        require(receipt.get("binary_sha256") == f"sha256:{sha256(binary)}", "binary identity changed")
        socket_queues = socket_queue_summary(samples, trace["server_ports"])
        require(socket_queues["samples"] > 0, "ss produced no samples")
        require(socket_queues["samples_with_connections"] > 0, "ss did not observe the workload sockets")
        require(all(sample["exit_code"] == 0 for sample in samples), "ss sampling failed")
        require(trace["tcp"]["unknown"]["read_calls"] == 0, "unowned loopback TCP reads found")
        require(trace["tcp"]["unknown"]["write_calls"] == 0, "unowned loopback TCP writes found")
        require(trace["tcp"]["server"]["write_bytes"] == receipt["resp"]["output_bytes"], "kernel server writes do not reconcile to RESP output bytes")
        require(trace["tcp"]["client"]["read_bytes"] == receipt["resp"]["output_bytes"], "kernel client reads do not reconcile to RESP output bytes")
        require(trace["tcp"]["client"]["write_bytes"] == receipt["resp"]["input_bytes"], "kernel client writes do not reconcile to RESP input bytes")
        require(trace["tcp"]["server"]["read_bytes"] == receipt["resp"]["input_bytes"], "kernel server reads do not reconcile to RESP input bytes")

        operations = receipt["operations"]
        trace["per_operation"] = {
            "server_write_calls": trace["tcp"]["server"]["write_calls"] / operations,
            "server_write_bytes": trace["tcp"]["server"]["write_bytes"] / operations,
            "server_read_calls": trace["tcp"]["server"]["read_calls"] / operations,
            "client_write_calls": trace["tcp"]["client"]["write_calls"] / operations,
            "client_read_calls": trace["tcp"]["client"]["read_calls"] / operations,
            "epoll_calls": trace["epoll_calls"] / operations,
            "futex_calls": trace["futex_calls"] / operations,
        }
        rusage_delta = rusage["delta"]
        rusage_delta["context_switches_per_operation"] = (
            rusage_delta["voluntary_context_switches"] + rusage_delta["involuntary_context_switches"]
        ) / operations
        rusage_delta["page_faults_per_operation"] = (
            rusage_delta["minor_page_faults"] + rusage_delta["major_page_faults"]
        ) / operations
        record.update(
            {
                "valid": True,
                "receipt": receipt_path.name,
                "receipt_sha256": sha256(receipt_path),
                "rusage": rusage_path.name,
                "rusage_sha256": sha256(rusage_path),
                "stdout": stdout_path.name,
                "stdout_sha256": sha256(stdout_path),
                "stderr": stderr_path.name,
                "stderr_sha256": sha256(stderr_path),
                "trace": trace,
                "rusage_delta": rusage_delta,
                "socket_queues": {key: value for key, value in socket_queues.items() if key != "selected"},
                "app": {
                    "write_calls": receipt["resp"]["write_calls"],
                    "flush_calls": receipt["resp"]["flush_calls"],
                    "poll_write_attempts": receipt["socket_io"]["poll_write_attempts"],
                    "poll_write_pending": receipt["socket_io"]["poll_write_pending"],
                    "short_writes": receipt["socket_io"]["short_writes"],
                },
                "goodput_operations_per_second": receipt["goodput_operations_per_second"],
                "p99_us": receipt["latency"]["p99_us"],
                "workload_sha256": receipt["workload_sha256"],
            }
        )
        write_json(ss_path, {"summary": record["socket_queues"], "samples": socket_queues["selected"]})
        record["ss"] = ss_path.name
        record["ss_sha256"] = sha256(ss_path)
    except (json.JSONDecodeError, OSError, subprocess.SubprocessError, ValueError) as error:
        record["error"] = str(error)
        if process is not None and process.poll() is None:
            process.kill()
            process.wait(timeout=5)
    finally:
        stop.set()
        if sampler is not None:
            sampler.join(timeout=5)
        trace_paths = sorted(output.glob(f"{trace_prefix.name}.*"))
        record["raw_traces"] = compress_traces(trace_paths)
        for path, key in ((stdout_path, "stdout"), (stderr_path, "stderr"), (receipt_path, "receipt"), (rusage_path, "rusage"), (ss_path, "ss")):
            if path.is_file() and key not in record:
                record[key] = path.name
                record[f"{key}_sha256"] = sha256(path)
    return record


def median(values: list[float | int]) -> float:
    return float(statistics.median(values))


def summarize(attempts: list[dict[str, Any]]) -> dict[str, Any]:
    cells: dict[str, Any] = {}
    for operation, pipeline, concurrency in CELLS:
        rows = [
            row for row in attempts
            if row["valid"]
            and row["cell"] == {"operation": operation, "pipeline": pipeline, "concurrency": concurrency}
        ]
        key = f"{operation}-p{pipeline}-c{concurrency}"
        require(len(rows) == REPEATS, f"{key} has {len(rows)} valid repeats")
        workload_digests = {row["workload_sha256"] for row in rows}
        require(len(workload_digests) == 1, f"{key} workload digest drifted")
        cells[key] = {
            "valid_repeats": len(rows),
            "workload_sha256": next(iter(workload_digests)),
            "median": {
                "kernel_server_write_calls_per_operation": median([row["trace"]["per_operation"]["server_write_calls"] for row in rows]),
                "kernel_server_read_calls_per_operation": median([row["trace"]["per_operation"]["server_read_calls"] for row in rows]),
                "kernel_client_write_calls_per_operation": median([row["trace"]["per_operation"]["client_write_calls"] for row in rows]),
                "kernel_client_read_calls_per_operation": median([row["trace"]["per_operation"]["client_read_calls"] for row in rows]),
                "epoll_calls_per_operation": median([row["trace"]["per_operation"]["epoll_calls"] for row in rows]),
                "futex_calls_per_operation": median([row["trace"]["per_operation"]["futex_calls"] for row in rows]),
                "context_switches_per_operation": median([row["rusage_delta"]["context_switches_per_operation"] for row in rows]),
                "page_faults_per_operation": median([row["rusage_delta"]["page_faults_per_operation"] for row in rows]),
                "app_write_calls": median([row["app"]["write_calls"] for row in rows]),
                "app_flush_calls": median([row["app"]["flush_calls"] for row in rows]),
                "app_short_writes": median([row["app"]["short_writes"] for row in rows]),
                "max_send_q_bytes": median([row["socket_queues"]["max_send_q_bytes"] for row in rows]),
                "max_recv_q_bytes": median([row["socket_queues"]["max_recv_q_bytes"] for row in rows]),
                "traced_goodput_operations_per_second": median([row["goodput_operations_per_second"] for row in rows]),
                "traced_p99_us": median([row["p99_us"] for row in rows]),
            },
        }
    return {
        "schema_version": SCHEMA_VERSION,
        "profile_id": PROFILE_ID,
        "attempts": len(attempts),
        "valid_attempts": sum(row["valid"] for row in attempts),
        "failed_attempts": sum(not row["valid"] for row in attempts),
        "cells": cells,
        "claim_boundary": {
            "performance_numbers_are_traced_attribution_only": True,
            "candidate_data_present": False,
            "acceptance_decision_allowed": False,
            "promotable": False,
            "tls_record_behavior_measured": False,
            "tokio_task_wakeup_time_measured": False,
        },
    }


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--tool-source", type=Path, required=True)
    parser.add_argument("--tool-lock", type=Path, required=True)
    parser.add_argument("--contract", type=Path, required=True)
    parser.add_argument("--repeats", type=int, default=REPEATS)
    args = parser.parse_args()

    require(sys.platform.startswith("linux"), "W9c must run on Linux")
    require(args.repeats == REPEATS, f"W9c requires exactly {REPEATS} repeats")
    require(len(args.source_commit) == 40 and all(char in "0123456789abcdef" for char in args.source_commit), "source commit must be a full lowercase SHA")
    require(shutil.which("strace") is not None, "strace is required")
    require(shutil.which("ss") is not None, "ss is required")
    inputs = [args.binary, args.tool_source, args.tool_lock, args.contract]
    for path in inputs:
        require(path.is_file(), f"missing input: {path}")
    if args.output_dir.exists():
        require(not any(args.output_dir.iterdir()), "output directory must be new or empty")
    else:
        args.output_dir.mkdir(parents=True)
    output = args.output_dir.resolve()
    binary = args.binary.resolve()

    attempts: list[dict[str, Any]] = []
    for ordinal, (operation, pipeline, concurrency) in enumerate(schedule_for(args.repeats), start=1):
        attempts.append(run_attempt(binary, args.source_commit, operation, pipeline, concurrency, ordinal, output))
        write_json(output / "attempts.json", attempts)

    manifest = {
        "schema_version": SCHEMA_VERSION,
        "profile_id": PROFILE_ID,
        "source_commit": args.source_commit,
        "binary_sha256": sha256(binary),
        "tool_source_sha256": sha256(args.tool_source),
        "tool_lock_sha256": sha256(args.tool_lock),
        "contract_sha256": sha256(args.contract),
        "schedule": [
            {"ordinal": ordinal, "operation": operation, "pipeline": pipeline, "concurrency": concurrency}
            for ordinal, (operation, pipeline, concurrency) in enumerate(schedule_for(args.repeats), start=1)
        ],
        "attempts": len(attempts),
        "valid_attempts": sum(row["valid"] for row in attempts),
        "failed_attempts": sum(not row["valid"] for row in attempts),
        "silent_retry_allowed": False,
        "candidate_data_present": False,
        "acceptance_decision_allowed": False,
        "promotable": False,
    }
    write_json(output / "manifest.json", manifest)
    if manifest["failed_attempts"]:
        return 1
    write_json(output / "summary.json", summarize(attempts))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(2)
