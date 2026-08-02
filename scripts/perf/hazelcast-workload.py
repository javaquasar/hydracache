#!/usr/bin/env python3
"""Matched SET/GET workload for Hazelcast Community exploratory runs."""

from __future__ import annotations

import argparse
import json
import sys
import time
from concurrent.futures import ThreadPoolExecutor


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=5701)
    parser.add_argument("--payload", type=int, required=True)
    parser.add_argument("--clients", type=int, required=True)
    parser.add_argument("--pipeline", type=int, required=True)
    parser.add_argument("--requests", type=int, required=True)
    parser.add_argument("--key-range", type=int, default=10000)
    parser.add_argument("--operation", choices=("set", "get"), required=True)
    args = parser.parse_args()
    try:
        import hazelcast  # type: ignore
    except ImportError as error:
        raise SystemExit("hazelcast-python-client is required for the Hazelcast target") from error

    client = hazelcast.HazelcastClient(cluster_members=[f"{args.host}:{args.port}"])
    # Keep the async proxy so pipeline depth is represented by outstanding
    # client futures rather than by a blocking call followed by a fake flush.
    cache = client.get_map("exploratory-067")
    value = b"x" * args.payload

    def worker(worker_id: int) -> int:
        count = args.requests // args.clients + (1 if worker_id < args.requests % args.clients else 0)
        for offset in range(0, count, args.pipeline):
            futures = []
            for index in range(offset, min(offset + args.pipeline, count)):
                key = f"key-{(worker_id * count + index) % args.key_range}"
                futures.append(cache.set(key, value) if args.operation == "set" else cache.get(key))
            for future in futures:
                future.result()
        return count

    start = time.perf_counter()
    with ThreadPoolExecutor(max_workers=args.clients) as pool:
        completed = sum(pool.map(worker, range(args.clients)))
    elapsed = time.perf_counter() - start
    client.shutdown()
    print(json.dumps({
        "operation": args.operation,
        "requests": completed,
        "elapsed_seconds": elapsed,
        "throughput_ops_per_second": completed / elapsed if elapsed else None,
    }, sort_keys=True))
    return 0


if __name__ == "__main__":
    sys.exit(main())
