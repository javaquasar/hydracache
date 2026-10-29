#!/usr/bin/env python3
"""Offline audit of a B0 background-CPU refusal. Never starts or retries a workload."""
from __future__ import annotations

import argparse
import json
import math
import pathlib
import tomllib

import performance_get_owner_controls_074 as runner

PACKET = "docs/testing/performance/0.74/local-runs/get-owner-b0-18ee8cdc"


def audit(root: pathlib.Path, packet: pathlib.Path) -> dict:
    policy_path = root / runner.POLICY
    policy = tomllib.loads(policy_path.read_text(encoding="utf-8"))
    runner.policy_check(policy)
    files = ["seal.json", "summary.json", "attempt-0001/attempt.json"]
    runner.require({path.relative_to(packet).as_posix() for path in packet.rglob("*") if path.is_file()} == set(files), "prelaunch packet has extra/missing files")
    seal = json.loads((packet / files[0]).read_text(encoding="utf-8"))
    summary = json.loads((packet / files[1]).read_text(encoding="utf-8"))
    attempt = json.loads((packet / files[2]).read_text(encoding="utf-8"))
    runner.require(seal["profile_id"] == runner.PROFILE and seal["policy"] == policy and seal["contract_sha256"] == runner.digest(policy_path), "contract seal drift")
    runner.require(seal["lock_sha256"] == runner.digest(root / "tools/get-owner-controls-074/Cargo.lock"), "lock seal drift")
    runner.require(seal["schedule"] == [list(row) for row in runner.schedule(policy)], "finite schedule drift")
    runner.require(len(seal["source_commit"]) == 40 and all(c in "0123456789abcdef" for c in seal["source_commit"]), "source SHA malformed")
    names = {f"{lane}-{role}" for lane in ["timing", "allocation"] for role in ["off", "on"]}
    runner.require(set(seal["binaries"]) == names, "four-binary seal missing")
    for binary in seal["binaries"].values():
        value = binary["sha256"]
        runner.require(len(value) == 71 and value.startswith("sha256:") and all(c in "0123456789abcdef" for c in value[7:]), "binary SHA malformed")
    first = runner.schedule(policy)[0]
    runner.require(tuple(attempt[key] for key in ["lane", "mode", "pair", "cell", "role"]) == first, "not first sealed attempt")
    cpu = attempt["background_cpu_percent"]
    runner.require(type(cpu) in (float, int) and math.isfinite(cpu) and 10 < cpu <= 100, "no frozen background-CPU refusal witness")
    runner.require(attempt["exit_code"] is None and attempt["stdout"] == "" and attempt["stderr"] == "", "benchmark process may have started")
    runner.require(all(attempt[key] is False for key in ["affinity_applied", "priority_applied", "placement_before_warmup"]), "not a prelaunch refusal")
    runner.require("receipt" not in attempt and "raw_sha256" not in attempt, "prelaunch must not contain measured receipt")
    reason = "background CPU outside frozen ceiling"
    runner.require(attempt["failure"] == reason, "failure reason drift")
    expected_summary = {"profile_id": runner.PROFILE, "classification": "invalidated", "reason": reason, "attempts_retained": 1, "promotable": False, "accepted_product_change": False, "product_performance_claim": False}
    runner.require(summary == expected_summary, "summary overstates refused attempt")
    cell = policy["cell"][0]
    command = [seal["binaries"]["timing-off"]["path"], "--source", seal["source_commit"], "--output", attempt["command"][4]]
    for key, value in ({name: cell[name] for name in runner.FIELDS} | {"key_space": 16, "seed": 740074}).items():
        command += ["--" + key.replace("_", "-"), str(value)]
    runner.require(command == attempt["command"] and command[4].endswith("attempt-0001\\raw.json"), "command identity drift")
    return {"profile_id": runner.PROFILE, "source_commit": seal["source_commit"], "classification": "verified-prelaunch-background-cpu-invalidation", "planned_attempts": 400, "attempts_retained": 1, "benchmark_processes_started": 0, "numerical_pairs_completed": 0, "background_cpu_percent": cpu, "maximum_background_cpu_percent": 10.0, "raw_file_sha256": {name: runner.digest(packet / name) for name in files}, "promotable": False, "accepted_product_change": False, "product_performance_claim": False, "compiled_feature_execution_confirmed": False}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", type=pathlib.Path, default=pathlib.Path(__file__).resolve().parents[2])
    parser.add_argument("--packet", type=pathlib.Path)
    options = parser.parse_args(); root = options.root.resolve()
    print(json.dumps(audit(root, options.packet or root / PACKET), indent=2))
    return 0


if __name__ == "__main__": raise SystemExit(main())
