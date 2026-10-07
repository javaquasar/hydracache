"""Finite local D1 runner/replay. Every attempt retained; no candidate admission."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tomllib

PROFILE = "response-reduction-owner-d1-074-v1"
CONTRACT = "docs/testing/performance/0.74/response-reduction-attribution-contract.toml"
LOCK = "tools/resp-response-owner-074/Cargo.lock"
CELLS = [
    ("get-empty", "get", True, 0, 10000),
    ("get-64", "get", True, 64, 10000),
    ("get-4096", "get", True, 4096, 10000),
    ("get-1048576", "get", True, 1048576, 500),
    ("get-miss", "get", False, 4096, 10000),
    ("set-4096", "set", True, 4096, 10000),
]
STAGES = ["dispatch_only", "reducer_only", "dispatch_and_reduce"]


def digest(path):
    return "sha256:" + hashlib.sha256(Path(path).read_bytes()).hexdigest()


def corpus_hashes(length):
    seed = (740074).to_bytes(8, "little")
    key = b"hc074-response-owner:\x00\xff:" + seed
    payload = (seed * ((length + 7) // 8))[:length]
    return tuple("sha256:" + hashlib.sha256(value).hexdigest() for value in [key, payload])


def write_new(path, value):
    with Path(path).open("x", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, allow_nan=False)
        stream.write("\n")


def git(root, *args):
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def order():
    return [(repeat, CELLS[(index + repeat) % 6])
            for repeat in range(3) for index in range(6)]


def require(condition, message):
    if not condition:
        raise ValueError(message)


def policy(root):
    contract = tomllib.loads((root / CONTRACT).read_text(encoding="utf-8"))
    require(contract["profile_id"] == PROFILE, "profile drift")
    for flag in ["product_mutation_allowed", "promotable", "product_numeric_claims_allowed",
                 "prior_rejections_reopened", "expensive_workloads_allowed"]:
        require(contract[flag] is False, "D1 boundary drift")
    screen = contract["screen"]
    for key, value in [("seed", 740074), ("warmup_operations_per_dispatch_control", 100),
                       ("fresh_process_repeats", 3), ("maximum_attempts", 18),
                       ("maximum_attempt_seconds", 30), ("cache_time_ms", 1000000)]:
        require(screen[key] == value, "screen policy drift")
    actual = [(c["id"], c["operation"], c["hit"], c["payload_bytes"], c["iterations"])
              for c in contract["cell"]]
    require(actual == CELLS, "cell drift")
    return contract


def integer(value):
    return type(value) is int and value >= 0


def analyse(directory):
    directory = Path(directory)
    seal = json.loads((directory / "seal.json").read_text())
    require(seal["profile_id"] == PROFILE and seal["promotable"] is False, "seal drift")
    require(len(list(directory.glob("*.attempt.json"))) == 18, "incomplete or duplicate attempts")
    require(len(list(directory.glob("*.raw.json"))) == 18, "incomplete or duplicate raw receipts")
    rows = {cell[0]: [] for cell in CELLS}
    corpus = {}
    for index, (repeat, cell) in enumerate(order()):
        stem = f"{index:02d}-{cell[0]}"
        attempt = json.loads((directory / f"{stem}.attempt.json").read_text())
        require(attempt["index"] == index and attempt["repeat"] == repeat
                and attempt["cell_id"] == cell[0] and attempt["returncode"] == 0
                and attempt["failure"] is None, "failed/reordered attempt")
        raw_path = directory / f"{stem}.raw.json"
        require(attempt["raw_sha256"] == digest(raw_path), "raw digest drift")
        raw = json.loads(raw_path.read_text())
        require(raw["profile_id"] == PROFILE and raw["tier"] == "local-d1-owner-attribution"
                and raw["schema_version"] == 1 and raw["promotable"] is False
                and raw["product_mutation"] is False, "receipt scope drift")
        for key in ["source_commit", "binary_sha256", "tool_lock_sha256", "contract_sha256"]:
            require(raw[key] == seal[key], "identity drift: " + key)
        require((raw["cell_id"], raw["operation"], raw["hit"], raw["payload_bytes"], raw["iterations"]) == cell,
                "workload drift")
        require(raw["seed"] == 740074 and raw["warmup_operations"] == 100
                and raw["cache_time_ms"] == 1000000
                and raw["exact_result_pointer_and_state_validation"] is True, "semantic drift")
        hashes = tuple(raw[k] for k in ["key_sha256", "payload_sha256", "request_plan_sha256"])
        require(hashes[:2] == corpus_hashes(cell[3]), "preregistered corpus differs")
        require(corpus.setdefault(cell[0], hashes) == hashes, "corpus drift")
        values = {}
        for stage in STAGES:
            record = raw[stage]
            memory = record["memory"]
            require(all(integer(v) for v in memory.values()), "invalid memory count")
            require(memory["live_after_bytes"] == memory["live_before_bytes"], "unreleased window owner")
            require(memory["peak_live_requested_bytes"] - memory["live_before_bytes"]
                    == memory["peak_live_above_start_bytes"], "inconsistent peak")
            for key in ["maximum_response_live_increment_bytes", "maximum_response_and_reduced_live_increment_bytes"]:
                require(integer(record[key]) and record[key] <= memory["peak_live_above_start_bytes"],
                        "invalid checkpoint")
            values[stage] = {"gross_bytes_per_operation": memory["gross_allocated_bytes"] / cell[4],
                             "allocation_calls_per_operation": memory["successful_allocation_calls"] / cell[4],
                             "peak_live_above_start_bytes": memory["peak_live_above_start_bytes"],
                             "response_live_bytes": record["maximum_response_live_increment_bytes"],
                             "response_and_reduced_live_bytes": record["maximum_response_and_reduced_live_increment_bytes"]}
        # Attribution consistency checks, not a product acceptance threshold.
        expected = cell[3] if cell[1] == "get" and cell[2] else 0
        isolated = raw["reducer_only"]["memory"]
        require(isolated["gross_allocated_bytes"] == expected * cell[4]
                and isolated["successful_allocation_calls"] == int(expected > 0) * cell[4]
                and isolated["peak_live_above_start_bytes"] == expected, "reducer attribution mismatch")
        delta = (raw["dispatch_and_reduce"]["memory"]["gross_allocated_bytes"]
                 - raw["dispatch_only"]["memory"]["gross_allocated_bytes"])
        require(delta == isolated["gross_allocated_bytes"], "unassigned dispatch/reducer residual")
        values["reduction_increment_bytes_per_operation"] = delta / cell[4]
        values["repeat"] = repeat
        rows[cell[0]].append(values)
    return {"schema_version": 1, "profile_id": PROFILE, "source_commit": seal["source_commit"],
            "classification": "d1-response-reduction-owner-attributed", "promotable": False,
            "product_numeric_claim": False, "attempts": 18, "all_attempts_retained": True,
            "candidate_authorized": False, "cells": rows,
            "limitations": ["not a product candidate or speedup", "fixed operation budget with injected clock",
                            "no encode/IO/concurrency/native/RSS/CPU/latency qualification"]}


def run(root, binary, source, output):
    root, binary, output = Path(root).resolve(), Path(binary).resolve(), Path(output).resolve()
    policy(root)
    require(git(root, "rev-parse", "HEAD") == source and not git(root, "status", "--porcelain"), "dirty/source drift")
    output.mkdir(parents=True, exist_ok=False)
    seal = {"profile_id": PROFILE, "promotable": False, "source_commit": source,
            "binary_sha256": digest(binary), "tool_lock_sha256": digest(root / LOCK),
            "contract_sha256": digest(root / CONTRACT),
            "toolchain": subprocess.check_output(["rustc", "-Vv"], cwd=root, text=True),
            "order": [{"index": i, "repeat": r, "cell_id": c[0]} for i, (r, c) in enumerate(order())]}
    write_new(output / "seal.json", seal)
    for index, (repeat, cell) in enumerate(order()):
        stem = f"{index:02d}-{cell[0]}"
        raw = output / f"{stem}.raw.json"
        attempt = {"index": index, "repeat": repeat, "cell_id": cell[0], "returncode": None,
                   "failure": None, "stdout": "", "stderr": "", "raw_sha256": None}
        try:
            require(git(root, "rev-parse", "HEAD") == source and not git(root, "status", "--porcelain"), "mid-run source drift")
            for name, path in [("binary_sha256", binary), ("tool_lock_sha256", root / LOCK),
                               ("contract_sha256", root / CONTRACT)]:
                require(digest(path) == seal[name], "mid-run identity drift: " + name)
            result = subprocess.run([str(binary), source, cell[0], str(raw)], cwd=root,
                                    text=True, capture_output=True, timeout=30, check=False)
            attempt.update(returncode=result.returncode, stdout=result.stdout, stderr=result.stderr)
            if result.returncode != 0:
                attempt["failure"] = "nonzero-exit"
        except (OSError, ValueError, subprocess.TimeoutExpired) as error:
            attempt["failure"] = str(error)
            if isinstance(error, subprocess.TimeoutExpired):
                for field in ["stdout", "stderr"]:
                    value = getattr(error, field, None) or ""
                    attempt[field] = value.decode("utf-8", errors="replace") if isinstance(value, bytes) else value
        if raw.exists():
            attempt["raw_sha256"] = digest(raw)
        write_new(output / f"{stem}.attempt.json", attempt)
        require(attempt["failure"] is None, "failed attempt retained; no retry authorized")
    try:
        summary = analyse(output)
    except (ValueError, KeyError, OSError) as error:
        write_new(output / "analysis-rejection.json", {"promotable": False, "failure": str(error),
                                                       "attempts_retained": True, "retry_authorized": False})
        raise
    write_new(output / "summary.json", summary)
    return summary


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--replay", type=Path)
    parser.add_argument("--root", type=Path)
    parser.add_argument("--binary", type=Path)
    parser.add_argument("--source")
    parser.add_argument("--output", type=Path)
    args = parser.parse_args()
    if args.replay:
        result = analyse(args.replay)
    else:
        require(all([args.root, args.binary, args.source, args.output]), "runner arguments required")
        result = run(args.root, args.binary, args.source, args.output)
    print(json.dumps(result, indent=2, allow_nan=False))
