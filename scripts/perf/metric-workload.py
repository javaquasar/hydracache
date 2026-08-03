#!/usr/bin/env python3
"""Small deterministic RESP workload for Stage 3 metric experiments.

The existing redis-benchmark is retained for Stage 1/2 compatibility.  This
client is used by Stage 3 where exact key length, hot-key/Zipf skew, TTL and
latency/error accounting are required.  It intentionally uses only the Python
standard library so the run has no unpinned client dependency.
"""

from __future__ import annotations

import argparse
import json
import math
import random
import socket
import statistics
import threading
import time
from concurrent.futures import ThreadPoolExecutor


def bulk_command(parts: list[bytes]) -> bytes:
    payload = [b"*" + str(len(parts)).encode() + b"\r\n"]
    for part in parts:
        payload.extend([b"$" + str(len(part)).encode() + b"\r\n", part, b"\r\n"])
    return b"".join(payload)


def read_resp(stream: socket.SocketType) -> tuple[bytes, int]:
    first = stream.recv(1)
    if not first:
        raise ConnectionError("peer closed RESP connection")
    if first in (b"+", b"-", b":"):
        line = read_line(stream)
        return first + line, len(first) + len(line)
    if first == b"$":
        line = read_line(stream)
        length = int(line)
        if length < 0:
            return first + line, len(first) + len(line)
        body = read_exact(stream, length + 2)
        return first + line + body, len(first) + len(line) + len(body)
    if first == b"*":
        line = read_line(stream)
        count = int(line)
        total = len(first) + len(line)
        for _ in range(max(count, 0)):
            _value, size = read_resp(stream)
            total += size
        return first + line, total
    raise ValueError(f"unsupported RESP prefix: {first!r}")


def read_line(stream: socket.SocketType) -> bytes:
    chunks: list[bytes] = []
    while True:
        chunk = stream.recv(1)
        if not chunk:
            raise ConnectionError("peer closed while reading RESP line")
        chunks.append(chunk)
        if b"".join(chunks).endswith(b"\r\n"):
            return b"".join(chunks)


def read_exact(stream: socket.SocketType, count: int) -> bytes:
    chunks: list[bytes] = []
    remaining = count
    while remaining:
        chunk = stream.recv(remaining)
        if not chunk:
            raise ConnectionError("peer closed while reading RESP payload")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def key_for(index: int, key_range: int, distribution: str, key_length: int, rng: random.Random) -> bytes:
    key_range = max(key_range, 1)
    if distribution == "hot":
        bucket = index % max(1, key_range // 100)
    elif distribution == "zipf":
        # Deterministic skew approximation: p=2 gives a hot head while still
        # exercising the long tail.  This is not presented as a Zipf fit.
        bucket = min(key_range - 1, int((rng.random() ** 2) * key_range))
    else:
        bucket = index % key_range
    base = f"metric-{bucket}".encode()
    if key_length <= len(base):
        return base[:key_length]
    return base + b"x" * (key_length - len(base))


def percentile(values: list[float], fraction: float) -> float | None:
    if not values:
        return None
    values = sorted(values)
    position = (len(values) - 1) * fraction
    low, high = math.floor(position), math.ceil(position)
    if low == high:
        return values[low]
    return values[low] + (values[high] - values[low]) * (position - low)


def worker(args: argparse.Namespace, worker_id: int, results: list[dict[str, float | int]]) -> None:
    count = args.requests // args.clients + (1 if worker_id < args.requests % args.clients else 0)
    rng = random.Random(args.seed + worker_id)
    latencies: list[float] = []
    errors = 0
    bytes_sent = 0
    bytes_received = 0
    start_index = sum(args.requests // args.clients + (1 if i < args.requests % args.clients else 0) for i in range(worker_id))
    try:
        with socket.create_connection((args.host, args.port), timeout=args.timeout) as stream:
            stream.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            for offset in range(0, count, args.pipeline):
                batch = range(offset, min(offset + args.pipeline, count))
                commands: list[bytes] = []
                for relative in batch:
                    index = start_index + relative
                    key = key_for(index, args.key_range, args.distribution, args.key_length, rng)
                    if args.operation == "set":
                        parts = [b"SET", key, b"x" * args.payload]
                        if args.ttl_ms:
                            parts.extend([b"PX", str(args.ttl_ms).encode()])
                    else:
                        parts = [b"GET", key]
                    commands.append(bulk_command(parts))
                payload = b"".join(commands)
                started = time.perf_counter()
                stream.sendall(payload)
                bytes_sent += len(payload)
                for _ in commands:
                    response, size = read_resp(stream)
                    bytes_received += size
                    if response.startswith(b"-"):
                        errors += 1
                elapsed_ms = (time.perf_counter() - started) * 1000.0
                latencies.extend([elapsed_ms / max(len(commands), 1)] * len(commands))
    except (OSError, ValueError, ConnectionError):
        errors += count
    results.append({
        "requests": count,
        "errors": errors,
        "bytes_sent": bytes_sent,
        "bytes_received": bytes_received,
        "latencies_ms": latencies,
    })


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--operation", choices=("set", "get"), required=True)
    parser.add_argument("--payload", type=int, default=256)
    parser.add_argument("--clients", type=int, default=10)
    parser.add_argument("--pipeline", type=int, default=1)
    parser.add_argument("--requests", type=int, required=True)
    parser.add_argument("--key-range", type=int, default=10000)
    parser.add_argument("--key-length", type=int, default=16)
    parser.add_argument("--distribution", choices=("uniform", "hot", "zipf"), default="uniform")
    parser.add_argument("--ttl-ms", type=int, default=0)
    parser.add_argument("--timeout", type=float, default=10.0)
    parser.add_argument("--seed", type=int, default=6701)
    args = parser.parse_args()
    args.clients = max(args.clients, 1)
    args.pipeline = max(args.pipeline, 1)
    args.requests = max(args.requests, 0)
    results: list[dict[str, float | int]] = []
    started = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.clients) as pool:
        futures = [pool.submit(worker, args, worker_id, results) for worker_id in range(args.clients)]
        for future in futures:
            future.result()
    elapsed = time.perf_counter() - started
    latencies = [value for result in results for value in result["latencies_ms"]]  # type: ignore[index]
    errors = sum(int(result["errors"]) for result in results)
    completed = sum(int(result["requests"]) for result in results)
    output = {
        "operation": args.operation,
        "requests": completed,
        "errors": errors,
        "elapsed_seconds": elapsed,
        "throughput_ops_per_second": completed / elapsed if elapsed else None,
        "p50_latency_ms": percentile(latencies, 0.50),
        "p95_latency_ms": percentile(latencies, 0.95),
        "p99_latency_ms": percentile(latencies, 0.99),
        "bytes_sent": sum(int(result["bytes_sent"]) for result in results),
        "bytes_received": sum(int(result["bytes_received"]) for result in results),
        "payload_bytes": args.payload,
        "clients": args.clients,
        "pipeline": args.pipeline,
        "key_range": args.key_range,
        "key_length": args.key_length,
        "distribution": args.distribution,
        "ttl_ms": args.ttl_ms or None,
        "seed": args.seed,
    }
    print(json.dumps(output, sort_keys=True))
    return 0 if errors == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
