#!/usr/bin/env python3
"""Bounded, resumable orchestration for the HydraCache 0.71 memory campaign.

The controller deliberately separates campaign scheduling from the measured
executor.  Rehearsal mode exercises the complete orchestration with bounded
durations; evidence mode fails closed until an evidence-capable executor is
selected explicitly.
"""

from __future__ import annotations

import argparse
import hashlib
import itertools
import json
import os
import platform
import shutil
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


RELEASE = "0.71"
SCHEMA_VERSION = 1
SCENARIO_RELATIVE = Path("docs/testing/perf-scenarios/0.71/memory-efficiency-v1.toml")
IDENTITIES_RELATIVE = Path("docs/testing/memory/0.71/baseline-identities.toml")
PROFILE_RELATIVE = Path("docs/testing/perf-host-profiles/memory-reference-071-v1.json")
DEFAULT_OUTPUT = Path("target/memory-evidence/0.71/campaigns")
BUILD_TIMEOUT_SECONDS = 3600


class CampaignError(RuntimeError):
    pass


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat()


def repository_root() -> Path:
    completed = subprocess.run(
        ["git", "rev-parse", "--show-toplevel"],
        check=True,
        capture_output=True,
        text=True,
    )
    return Path(completed.stdout.strip()).resolve()


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def atomic_json(path: Path, value: Any, *, create: bool = False) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
    with os.fdopen(os.open(temporary, flags, 0o600), "w", encoding="utf-8", newline="\n") as stream:
        json.dump(value, stream, indent=2, sort_keys=True)
        stream.write("\n")
        stream.flush()
        os.fsync(stream.fileno())
    if create and path.exists():
        temporary.unlink()
        raise CampaignError(f"refusing to overwrite {path}")
    os.replace(temporary, path)


def append_event(path: Path, event: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    record = {"at": utc_now(), **event}
    with path.open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, sort_keys=True) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


@dataclass(frozen=True)
class Cell:
    case_id: str
    dimensions: dict[str, Any]

    @property
    def cell_id(self) -> str:
        if not self.dimensions:
            return self.case_id
        suffix = "__".join(
            f"{key}-{str(value).lower()}" for key, value in sorted(self.dimensions.items())
        )
        return f"{self.case_id}__{suffix}"


def expand_case(case: dict[str, Any]) -> list[Cell]:
    case_id = str(case["id"])
    if case_id == "M1-shape":
        return [
            Cell(case_id, {"keys": keys, "value_bytes": value_bytes})
            for keys, value_bytes in itertools.product(case["keys"], case["value_bytes"])
        ]
    if case_id == "M2-rewrite":
        return [Cell(case_id, {"cycles": cycles}) for cycles in case["cycles"]]
    if case_id in {"M3-ttl", "M4-reset"}:
        return [Cell(case_id, {"cycles": cycles}) for cycles in case["cycles"]]
    if case_id == "M5-tags":
        cells = [
            Cell(case_id, {"distribution": "uniform", "tags_per_entry": count})
            for count in case["tags_per_entry"]
        ]
        cells.extend(
            Cell(case_id, {"distribution": distribution, "tags_per_entry": "canonical"})
            for distribution in case["distributions"]
            if distribution != "uniform"
        )
        return cells
    if case_id == "M6-connections":
        cells = [
            Cell(case_id, {"connections": connections, "tls": tls, "slow_consumers": 0})
            for connections, tls in itertools.product(case["connections"], case["tls"])
        ]
        cells.extend(
            Cell(case_id, {"connections": count, "tls": False, "slow_consumers": count})
            for count in case["slow_consumers"]
        )
        return cells
    if case_id == "M7-persistence":
        return [Cell(case_id, {"persistence": mode}) for mode in case["modes"]]
    if case_id == "M8-60m":
        return [
            Cell(case_id, {"sequence": sequence})
            for sequence in ("fixed-keyspace", "ttl", "reset", "hc2-churn")
        ]
    return [Cell(case_id, {})]


def selected_cases(scenario: dict[str, Any], requested: list[str]) -> list[dict[str, Any]]:
    cases = scenario["case"]
    if not requested:
        return cases
    requested_set = set(requested)
    selected = [case for case in cases if case["id"] in requested_set]
    missing = requested_set - {case["id"] for case in selected}
    if missing:
        raise CampaignError(f"unknown cases: {', '.join(sorted(missing))}")
    return selected


def build_plan(
    root: Path,
    cases: list[str],
    cohorts: list[str],
    repetition_override: int | None,
    rehearsal: bool,
) -> dict[str, Any]:
    scenario_path = root / SCENARIO_RELATIVE
    scenario = tomllib.loads(scenario_path.read_text(encoding="utf-8"))
    identities = tomllib.loads((root / IDENTITIES_RELATIVE).read_text(encoding="utf-8"))
    selected = selected_cases(scenario, cases)
    if "B0-release" in cohorts and any(case["id"] != "M0-cold" for case in selected):
        raise CampaignError("B0-release is admitted only for the M0 D0 external-signal cohort")
    source_shas = {
        "B0-release": str(identities["b0_release"]["source_sha"]),
        "B1-instrumented": str(identities["b1_instrumented"]["source_sha"]),
        "C-candidate": git(root, "rev-parse", "HEAD"),
    }
    jobs: list[dict[str, Any]] = []
    row_caps: dict[str, int] = {}
    for case in selected:
        case_id = str(case["id"])
        repetitions = repetition_override or int(case["d0_repetitions"])
        row_caps[case_id] = 30 if rehearsal else int(case["host_time_cap_seconds"])
        for cell in expand_case(case):
            for repetition in range(1, repetitions + 1):
                for cohort in cohorts:
                    job_id = f"{cell.cell_id}__{cohort}__r{repetition}"
                    jobs.append(
                        {
                            "job_id": job_id,
                            "case_id": case_id,
                            "cell_id": cell.cell_id,
                            "dimensions": cell.dimensions,
                            "cohort": cohort,
                            "repetition": repetition,
                            "status": "pending",
                            "attempts": [],
                        }
                    )
    return {
        "schema_version": SCHEMA_VERSION,
        "release": RELEASE,
        "scenario": str(SCENARIO_RELATIVE).replace("\\", "/"),
        "scenario_digest": sha256(scenario_path),
        "mode": "rehearsal" if rehearsal else "evidence",
        "created_at": utc_now(),
        "cohorts": cohorts,
        "source_shas": {cohort: source_shas[cohort] for cohort in cohorts},
        "row_time_caps_seconds": row_caps,
        "admitted_host_cap_seconds": sum(row_caps.values()),
        "job_count": len(jobs),
        "jobs": jobs,
    }


def git(root: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True, text=True
    ).stdout.strip()


def doctor(root: Path, evidence: bool) -> dict[str, Any]:
    scenario = root / SCENARIO_RELATIVE
    identities = root / IDENTITIES_RELATIVE
    profile = root / PROFILE_RELATIVE
    checks: dict[str, dict[str, Any]] = {}

    def record(name: str, ok: bool, detail: str) -> None:
        checks[name] = {"ok": ok, "detail": detail}

    for tool in ("git", "cargo", "rustc", "python3" if os.name != "nt" else "python"):
        location = shutil.which(tool)
        record(f"tool:{tool}", location is not None, location or "not found")
    for path in (scenario, identities, profile):
        record(f"file:{path.name}", path.is_file(), str(path))
    try:
        dirty = git(root, "status", "--porcelain=v1", "--untracked-files=all")
        record("clean-worktree", not dirty, "clean" if not dirty else "dirty")
    except subprocess.CalledProcessError as error:
        record("clean-worktree", False, str(error))
    if identities.is_file():
        values = tomllib.loads(identities.read_text(encoding="utf-8"))
        for name, table in (("B0", values["b0_release"]), ("B1", values["b1_instrumented"])):
            expected = str(table["source_sha"])
            present = subprocess.run(
                ["git", "cat-file", "-e", f"{expected}^{{commit}}"], cwd=root
            ).returncode == 0
            record(f"commit:{name}", present, expected)
    linux = platform.system() == "Linux"
    record("platform:linux", linux, platform.platform())
    for tool in ("perf", "numactl"):
        location = shutil.which(tool)
        record(f"evidence-tool:{tool}", location is not None, location or "not found")
    eligible = all(item["ok"] for item in checks.values())
    if not evidence:
        eligible = all(
            item["ok"]
            for name, item in checks.items()
            if not name.startswith("evidence-tool:") and name != "platform:linux"
        )
    return {
        "schema_version": SCHEMA_VERSION,
        "release": RELEASE,
        "mode": "evidence" if evidence else "rehearsal",
        "checked_at": utc_now(),
        "eligible": eligible,
        "checks": checks,
    }


class CampaignLock:
    def __init__(self, path: Path):
        self.path = path
        self.fd: int | None = None

    def __enter__(self) -> "CampaignLock":
        try:
            self.fd = os.open(self.path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            os.write(self.fd, f"{os.getpid()}\n".encode("ascii"))
            os.fsync(self.fd)
        except FileExistsError as error:
            raise CampaignError(f"campaign is already locked: {self.path}") from error
        return self

    def __exit__(self, *_args: Any) -> None:
        if self.fd is not None:
            os.close(self.fd)
        self.path.unlink(missing_ok=True)


def ensure_external_build_root(root: Path, build_root: Path) -> Path:
    resolved = build_root.resolve()
    if resolved == root or resolved.is_relative_to(root):
        raise CampaignError("evidence build root must be outside the Git worktree")
    resolved.mkdir(parents=True, exist_ok=True)
    return resolved


def executable_name() -> str:
    return "hydracache-server.exe" if os.name == "nt" else "hydracache-server"


def prepare_builds(root: Path, campaign_dir: Path, build_root: Path) -> None:
    state_path = campaign_dir / "state.json"
    if not state_path.is_file():
        raise CampaignError(f"campaign state is missing: {state_path}")
    with CampaignLock(campaign_dir / ".lock"):
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state["mode"] != "evidence":
            raise CampaignError("prepare is only required for evidence campaigns")
        if git(root, "status", "--porcelain=v1", "--untracked-files=all"):
            raise CampaignError("evidence builds require a clean controller worktree")
        build_root = ensure_external_build_root(root, build_root)
        manifests: dict[str, Any] = dict(state.get("builds", {}))
        for cohort, source_sha in state["source_shas"].items():
            if cohort in manifests:
                manifest_path = campaign_dir / manifests[cohort]["manifest"]
                if manifest_path.is_file() and sha256(manifest_path) == manifests[cohort]["manifest_sha256"]:
                    continue
                raise CampaignError(f"retained build manifest drift for {cohort}")
            slug = cohort.lower().replace("-", "_")
            worktree = build_root / f"source-{slug}"
            target_dir = build_root / f"target-{slug}"
            if worktree.exists() or target_dir.exists():
                raise CampaignError(f"refusing to reuse incomplete build paths for {cohort}")
            log_dir = campaign_dir / "builds" / cohort
            log_dir.mkdir(parents=True, exist_ok=False)
            build_log = log_dir / "build.log"
            try:
                subprocess.run(
                    ["git", "worktree", "add", "--detach", str(worktree), source_sha],
                    cwd=root,
                    check=True,
                    stdout=subprocess.PIPE,
                    stderr=subprocess.STDOUT,
                    text=True,
                )
                if git(worktree, "rev-parse", "HEAD") != source_sha:
                    raise CampaignError(f"worktree source mismatch for {cohort}")
                if git(worktree, "status", "--porcelain=v1", "--untracked-files=all"):
                    raise CampaignError(f"new build worktree is dirty for {cohort}")
                command = ["cargo", "build", "--release", "--locked", "-p", "hydracache-server"]
                environment = os.environ.copy()
                environment.update(
                    {
                        "CARGO_INCREMENTAL": "0",
                        "CARGO_TARGET_DIR": str(target_dir),
                        "SOURCE_DATE_EPOCH": git(worktree, "show", "-s", "--format=%ct", source_sha),
                    }
                )
                with build_log.open("w", encoding="utf-8", newline="\n") as log:
                    completed = subprocess.run(
                        command,
                        cwd=worktree,
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=BUILD_TIMEOUT_SECONDS,
                        check=False,
                    )
                if completed.returncode != 0:
                    raise CampaignError(f"build failed for {cohort}; see {build_log}")
                built_binary = target_dir / "release" / executable_name()
                if not built_binary.is_file():
                    raise CampaignError(f"build did not produce {built_binary}")
                retained_binary = log_dir / executable_name()
                shutil.copy2(built_binary, retained_binary)
                retained_binary.chmod(0o555)
                manifest = {
                    "schema_version": SCHEMA_VERSION,
                    "release": RELEASE,
                    "cohort": cohort,
                    "source_sha": source_sha,
                    "cargo_lock_sha256": sha256(worktree / "Cargo.lock"),
                    "binary": str(retained_binary.resolve()),
                    "binary_sha256": sha256(retained_binary),
                    "build_profile": "release",
                    "features": [],
                    "allocator": "system",
                    "exact_command": command,
                    "environment": {
                        "CARGO_INCREMENTAL": "0",
                        "SOURCE_DATE_EPOCH": environment["SOURCE_DATE_EPOCH"],
                    },
                    "built_at": utc_now(),
                }
                manifest_path = log_dir / "build-manifest.json"
                atomic_json(manifest_path, manifest, create=True)
                manifests[cohort] = {
                    "manifest": str(manifest_path.relative_to(campaign_dir)).replace("\\", "/"),
                    "manifest_sha256": sha256(manifest_path),
                }
                state["builds"] = manifests
                atomic_json(state_path, state)
            finally:
                if worktree.exists():
                    subprocess.run(
                        ["git", "worktree", "remove", "--force", str(worktree)],
                        cwd=root,
                        check=False,
                        stdout=subprocess.DEVNULL,
                        stderr=subprocess.DEVNULL,
                    )
                shutil.rmtree(target_dir, ignore_errors=True)


def execute_rehearsal(root: Path, campaign_dir: Path, job: dict[str, Any]) -> tuple[str, list[str]]:
    output = campaign_dir / "jobs" / job["job_id"]
    command = [
        "cargo",
        "run",
        "-p",
        "hydracache-loadgen",
        "--locked",
        "--",
        "memory-efficiency",
        "--instrumentation-mode",
        "profile",
        "--provider",
        "system",
        "--output-dir",
        str(output),
    ]
    log_path = output / "executor.log"
    output.mkdir(parents=True, exist_ok=True)
    with log_path.open("w", encoding="utf-8", newline="\n") as log:
        try:
            completed = subprocess.run(
                command,
                cwd=root,
                stdout=log,
                stderr=subprocess.STDOUT,
                # Rehearsal includes a locked development build when no warm
                # binary exists. The measured smoke itself remains bounded by
                # the much smaller row cap recorded in the plan.
                timeout=180,
                check=False,
            )
        except subprocess.TimeoutExpired:
            return "timeout", command
        except OSError:
            return "tool-unavailable", command
    return ("success" if completed.returncode == 0 else "product-failure"), command


def run_campaign(root: Path, campaign_dir: Path) -> None:
    state_path = campaign_dir / "state.json"
    events_path = campaign_dir / "events.jsonl"
    if not state_path.is_file():
        raise CampaignError(f"campaign state is missing: {state_path}")
    with CampaignLock(campaign_dir / ".lock"):
        state = json.loads(state_path.read_text(encoding="utf-8"))
        if state["mode"] != "rehearsal":
            raise CampaignError(
                "evidence execution is fail-closed until the daemon executor is configured"
            )
        for job in state["jobs"]:
            if job["status"] == "success":
                continue
            attempt = len(job["attempts"]) + 1
            append_event(events_path, {"event": "job-started", "job_id": job["job_id"], "attempt": attempt})
            started = time.monotonic_ns()
            status, command = execute_rehearsal(root, campaign_dir, job)
            job["status"] = status
            job["attempts"].append(
                {
                    "attempt": attempt,
                    "started_at": utc_now(),
                    "elapsed_ns": time.monotonic_ns() - started,
                    "status": status,
                    "exact_command": command,
                }
            )
            atomic_json(state_path, state)
            append_event(events_path, {"event": "job-finished", "job_id": job["job_id"], "attempt": attempt, "status": status})
            if status != "success":
                raise CampaignError(f"job {job['job_id']} ended as {status}")
        state["completed_at"] = utc_now()
        state["status"] = "success"
        atomic_json(state_path, state)
        append_event(events_path, {"event": "campaign-finished", "status": "success"})


def finalize(campaign_dir: Path) -> dict[str, Any]:
    state_path = campaign_dir / "state.json"
    state = json.loads(state_path.read_text(encoding="utf-8"))
    incomplete = [job["job_id"] for job in state["jobs"] if job["status"] != "success"]
    if incomplete:
        raise CampaignError(f"campaign has {len(incomplete)} incomplete jobs")
    artifacts = []
    for path in sorted((campaign_dir / "jobs").rglob("*")):
        if path.is_file():
            artifacts.append(
                {
                    "path": str(path.relative_to(campaign_dir)).replace("\\", "/"),
                    "sha256": sha256(path),
                    "bytes": path.stat().st_size,
                }
            )
    receipt = {
        "schema_version": SCHEMA_VERSION,
        "release": RELEASE,
        "mode": state["mode"],
        "scenario_digest": state["scenario_digest"],
        "job_count": state["job_count"],
        "completed_jobs": len(state["jobs"]),
        "finalized_at": utc_now(),
        "ship_evidence_eligible": False,
        "artifacts": artifacts,
    }
    atomic_json(campaign_dir / "campaign-receipt.json", receipt)
    return receipt


def campaign_path(root: Path, output: Path, campaign_id: str) -> Path:
    base = output if output.is_absolute() else root / output
    return (base / campaign_id).resolve()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output-root", type=Path, default=DEFAULT_OUTPUT)
    commands = parser.add_subparsers(dest="command", required=True)
    doctor_parser = commands.add_parser("doctor")
    doctor_parser.add_argument("--evidence", action="store_true")
    plan_parser = commands.add_parser("plan")
    plan_parser.add_argument("--campaign-id", required=True)
    plan_parser.add_argument("--case", action="append", default=[])
    plan_parser.add_argument("--cohort", action="append", choices=["B0-release", "B1-instrumented", "C-candidate"])
    plan_parser.add_argument("--repetitions", type=int)
    plan_parser.add_argument("--rehearsal", action="store_true")
    prepare_parser = commands.add_parser("prepare")
    prepare_parser.add_argument("--campaign-id", required=True)
    prepare_parser.add_argument("--build-root", type=Path, required=True)
    for name in ("run", "resume", "finalize"):
        command = commands.add_parser(name)
        command.add_argument("--campaign-id", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    root = repository_root()
    try:
        if args.command == "doctor":
            result = doctor(root, args.evidence)
            print(json.dumps(result, indent=2, sort_keys=True))
            return 0 if result["eligible"] else 2
        campaign_dir = campaign_path(root, args.output_root, args.campaign_id)
        if args.command == "plan":
            if args.repetitions is not None and args.repetitions < 1:
                raise CampaignError("--repetitions must be positive")
            if campaign_dir.exists():
                raise CampaignError(f"campaign already exists: {campaign_dir}")
            cohorts = args.cohort or ["B1-instrumented"]
            plan = build_plan(root, args.case, cohorts, args.repetitions, args.rehearsal)
            if not args.rehearsal:
                resolved_output = campaign_path(root, args.output_root, "_").parent
                if resolved_output == root or resolved_output.is_relative_to(root):
                    raise CampaignError("evidence campaign output must be outside the Git worktree")
            campaign_dir.mkdir(parents=True)
            atomic_json(campaign_dir / "state.json", plan, create=True)
            append_event(campaign_dir / "events.jsonl", {"event": "campaign-planned", "job_count": plan["job_count"]})
            print(json.dumps({key: plan[key] for key in ("mode", "job_count", "admitted_host_cap_seconds")}, indent=2))
        elif args.command == "prepare":
            prepare_builds(root, campaign_dir, args.build_root)
        elif args.command in {"run", "resume"}:
            run_campaign(root, campaign_dir)
        elif args.command == "finalize":
            print(json.dumps(finalize(campaign_dir), indent=2, sort_keys=True))
        return 0
    except (CampaignError, OSError, ValueError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        print(f"memory campaign 0.71: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
