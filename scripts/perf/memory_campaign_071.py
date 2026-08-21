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
import re
import shutil
import subprocess
import sys
import tarfile
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
FULL_SHA = re.compile(r"^[0-9a-f]{40}$")
CAMPAIGN_ROLES = {"baseline", "candidate"}


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
        if self.case_id == "M8-60m":
            return f"{self.case_id}__sequence-{self.dimensions['sequence']}"
        if self.case_id in {"M9-6h", "M10-24h"}:
            return self.case_id

        def slug(value: Any) -> str:
            if isinstance(value, list):
                return "+".join(str(item).lower() for item in value)
            return str(value).lower()

        suffix = "__".join(
            f"{key}-{slug(value)}" for key, value in sorted(self.dimensions.items())
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
            Cell(
                case_id,
                {
                    "distribution": "uniform",
                    "tags_per_entry": count,
                    "tag_pool": case["uniform_tag_pool"],
                },
            )
            for count in case["tags_per_entry"]
        ]
        cells.append(
            Cell(
                case_id,
                {
                    "distribution": "one-hot",
                    "tags_per_entry": int(case["one_hot_tags_per_entry"]),
                    "tag_pool": int(case["one_hot_tag_pool"]),
                },
            )
        )
        cells.append(
            Cell(
                case_id,
                {
                    "distribution": "high-fanout",
                    "tags_per_entry": int(case["high_fanout_tags_per_entry"]),
                    "tag_pool": int(case["high_fanout_tag_pool"]),
                },
            )
        )
        return cells
    if case_id == "M6-connections":
        cells = [
            Cell(case_id, {"connections": connections, "tls": tls, "slow_consumers": 0})
            for connections, tls in itertools.product(case["connections"], case["tls"])
        ]
        cells.extend(
            Cell(case_id, {"connections": count, "tls": True, "slow_consumers": count})
            for count in case["slow_consumers"]
        )
        return cells
    if case_id == "M7-persistence":
        return [Cell(case_id, {"persistence": mode}) for mode in case["modes"]]
    if case_id == "M8-60m":
        return [
            Cell(
                case_id,
                {
                    "sequence": sequence,
                    "duration_seconds": int(case["duration_seconds"]),
                    "iteration_seconds": int(case["iteration_seconds"]),
                    "heartbeat_seconds": int(case["heartbeat_seconds"]),
                    "hc2_churn_connections": int(case["hc2_churn_connections"]),
                    "rehearsal_duration_seconds": float(case["rehearsal_duration_seconds"]),
                    "rehearsal_iteration_seconds": float(case["rehearsal_iteration_seconds"]),
                    "rehearsal_heartbeat_seconds": float(case["rehearsal_heartbeat_seconds"]),
                },
            )
            for sequence in ("fixed-keyspace", "ttl", "reset", "hc2-churn")
        ]
    if case_id in {"M9-6h", "M10-24h"}:
        return [
            Cell(
                case_id,
                {
                    "sequence": list(case["sequence"]),
                    "duration_seconds": int(case["duration_seconds"]),
                    "block_seconds": int(case["block_seconds"]),
                    "iteration_seconds": int(case["iteration_seconds"]),
                    "heartbeat_seconds": int(case["heartbeat_seconds"]),
                    "hc2_churn_connections": int(case["hc2_churn_connections"]),
                    "rehearsal_duration_seconds": float(case["rehearsal_duration_seconds"]),
                    "rehearsal_block_seconds": float(case["rehearsal_block_seconds"]),
                    "rehearsal_iteration_seconds": float(case["rehearsal_iteration_seconds"]),
                    "rehearsal_heartbeat_seconds": float(case["rehearsal_heartbeat_seconds"]),
                },
            )
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
    workflow_sha: str | None = None,
    source_sha: str | None = None,
    campaign_role: str = "baseline",
) -> dict[str, Any]:
    scenario_path = root / SCENARIO_RELATIVE
    scenario = tomllib.loads(scenario_path.read_text(encoding="utf-8"))
    identities = tomllib.loads((root / IDENTITIES_RELATIVE).read_text(encoding="utf-8"))
    selected = selected_cases(scenario, cases)
    if "B0-release" in cohorts and any(case["id"] != "M0-cold" for case in selected):
        raise CampaignError("B0-release is admitted only for the M0 D0 external-signal cohort")
    if campaign_role not in CAMPAIGN_ROLES:
        raise CampaignError(f"unsupported campaign role: {campaign_role}")
    if not rehearsal:
        cohort_set = set(cohorts)
        if campaign_role == "candidate" and cohort_set != {"B1-instrumented", "C-candidate"}:
            raise CampaignError("candidate campaign requires exactly the frozen B1/C cohorts")
        if campaign_role == "baseline" and not cohort_set.issubset({"B0-release", "B1-instrumented"}):
            raise CampaignError("baseline campaign admits only the frozen B0/B1 cohorts")
    resolved_workflow_sha = resolve_commit(root, workflow_sha or git(root, "rev-parse", "HEAD"), "workflow")
    if source_sha is None:
        source_sha = git(root, "rev-parse", "HEAD") if rehearsal else str(
            identities["b1_instrumented"]["source_sha"]
        )
    resolved_source_sha = resolve_commit(root, source_sha, "candidate source")
    if (
        not rehearsal
        and campaign_role == "baseline"
        and resolved_source_sha != str(identities["b1_instrumented"]["source_sha"])
    ):
        raise CampaignError("baseline campaign source_sha must equal the frozen B1-instrumented SHA")
    source_shas = {
        "B0-release": str(identities["b0_release"]["source_sha"]),
        "B1-instrumented": str(identities["b1_instrumented"]["source_sha"]),
        "C-candidate": resolved_source_sha,
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
        "workflow_sha": resolved_workflow_sha,
        "source_sha": resolved_source_sha,
        "campaign_role": campaign_role,
        "controller_sha": resolved_workflow_sha,
        "case_ids": sorted({str(case["id"]) for case in selected}),
        "row_time_caps_seconds": row_caps,
        "admitted_host_cap_seconds": sum(row_caps.values()),
        "job_count": len(jobs),
        "jobs": jobs,
    }


def git(root: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True, text=True
    ).stdout.strip()


def require_full_sha(value: str, label: str) -> str:
    if not FULL_SHA.fullmatch(value):
        raise CampaignError(f"{label} must be a lowercase 40-character commit SHA")
    return value


def resolve_commit(root: Path, value: str, label: str) -> str:
    requested = require_full_sha(value, label)
    try:
        resolved = git(root, "rev-parse", f"{requested}^{{commit}}")
    except subprocess.CalledProcessError as error:
        raise CampaignError(f"{label} commit is unavailable in the canonical checkout: {requested}") from error
    if resolved != requested:
        raise CampaignError(f"{label} did not resolve to the exact requested commit")
    return resolved


def campaign_identity(plan: dict[str, Any]) -> dict[str, Any]:
    return {
        "schema_version": SCHEMA_VERSION,
        "release": RELEASE,
        "campaign_id": plan["campaign_id"],
        "workflow_sha": plan["workflow_sha"],
        "source_sha": plan["source_sha"],
        "campaign_role": plan["campaign_role"],
        "controller_sha": plan["controller_sha"],
        "scenario_digest": plan["scenario_digest"],
        "case_ids": plan["case_ids"],
        "source_shas": plan["source_shas"],
    }


def retained_campaign_identity(campaign_dir: Path, state: dict[str, Any]) -> dict[str, Any]:
    reference = state.get("identity", {})
    relative = reference.get("path")
    expected_digest = reference.get("sha256")
    if not isinstance(relative, str) or not isinstance(expected_digest, str):
        raise CampaignError("campaign identity reference is missing")
    path = campaign_dir / relative
    if not path.is_file() or sha256(path) != expected_digest:
        raise CampaignError("campaign identity receipt is missing or drifted")
    identity = json.loads(path.read_text(encoding="utf-8"))
    for field in (
        "campaign_id",
        "workflow_sha",
        "source_sha",
        "campaign_role",
        "controller_sha",
        "scenario_digest",
        "case_ids",
        "source_shas",
    ):
        if identity.get(field) != state.get(field):
            raise CampaignError(f"campaign state disagrees with immutable identity field {field}")
    return identity


def verify_campaign_identity(
    campaign_dir: Path,
    workflow_sha: str,
    source_sha: str,
    campaign_role: str,
    case_id: str,
) -> dict[str, Any]:
    require_full_sha(workflow_sha, "workflow_sha")
    require_full_sha(source_sha, "source_sha")
    if campaign_role not in CAMPAIGN_ROLES:
        raise CampaignError(f"unsupported campaign role: {campaign_role}")
    state_path = campaign_dir / "state.json"
    if not state_path.is_file():
        raise CampaignError(f"campaign state is missing: {state_path}")
    state = json.loads(state_path.read_text(encoding="utf-8"))
    identity = retained_campaign_identity(campaign_dir, state)
    expected = {
        "workflow_sha": workflow_sha,
        "source_sha": source_sha,
        "campaign_role": campaign_role,
    }
    for field, value in expected.items():
        if identity.get(field) != value:
            raise CampaignError(f"campaign identity mismatch for {field}")
    if case_id not in identity.get("case_ids", []):
        raise CampaignError(f"campaign identity does not contain requested case {case_id}")
    return identity


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


def hc2_helper_name() -> str:
    return "memory-hc2-connections-071.exe" if os.name == "nt" else "memory-hc2-connections-071"


def prepare_builds(root: Path, campaign_dir: Path, build_root: Path) -> None:
    state_path = campaign_dir / "state.json"
    if not state_path.is_file():
        raise CampaignError(f"campaign state is missing: {state_path}")
    with CampaignLock(campaign_dir / ".lock"):
        state = json.loads(state_path.read_text(encoding="utf-8"))
        identity = retained_campaign_identity(campaign_dir, state)
        if state["mode"] != "evidence":
            raise CampaignError("prepare is only required for evidence campaigns")
        if git(root, "status", "--porcelain=v1", "--untracked-files=all"):
            raise CampaignError("evidence builds require a clean controller worktree")
        if git(root, "rev-parse", "HEAD") != state["controller_sha"]:
            raise CampaignError("controller source drifted from the planned SHA")
        build_root = ensure_external_build_root(root, build_root)
        if not state.get("controller_tools"):
            tool_dir = campaign_dir / "builds" / "controller-tools"
            target_dir = build_root / f"target-controller-{campaign_dir.name}"
            if tool_dir.exists() or target_dir.exists():
                raise CampaignError("refusing to reuse incomplete controller-tool build paths")
            tool_dir.mkdir(parents=True)
            command = [
                "cargo",
                "build",
                "--release",
                "--locked",
                "-p",
                "hydracache-loadgen",
                "--bin",
                "memory-hc2-connections-071",
            ]
            environment = os.environ.copy()
            environment.update({"CARGO_INCREMENTAL": "0", "CARGO_TARGET_DIR": str(target_dir)})
            try:
                with (tool_dir / "build.log").open("w", encoding="utf-8", newline="\n") as log:
                    completed = subprocess.run(
                        command,
                        cwd=root,
                        env=environment,
                        stdout=log,
                        stderr=subprocess.STDOUT,
                        timeout=BUILD_TIMEOUT_SECONDS,
                        check=False,
                    )
                if completed.returncode != 0:
                    raise CampaignError("controller HC/2 helper build failed")
                built = target_dir / "release" / hc2_helper_name()
                retained = tool_dir / hc2_helper_name()
                shutil.copy2(built, retained)
                retained.chmod(0o555)
                manifest = {
                    "schema_version": SCHEMA_VERSION,
                    "release": RELEASE,
                    "source_sha": identity["workflow_sha"],
                    "binary": str(retained.resolve()),
                    "binary_sha256": sha256(retained),
                    "exact_command": command,
                }
                manifest_path = tool_dir / "hc2-helper-manifest.json"
                atomic_json(manifest_path, manifest, create=True)
                state["controller_tools"] = {
                    "hc2_helper_manifest": str(manifest_path.relative_to(campaign_dir)).replace("\\", "/"),
                    "hc2_helper_manifest_sha256": sha256(manifest_path),
                }
                atomic_json(state_path, state)
            finally:
                shutil.rmtree(target_dir, ignore_errors=True)
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
            if log_dir.exists():
                failures = campaign_dir / "build-failures"
                failures.mkdir(exist_ok=True)
                retained_failure = failures / f"{slug}-{time.time_ns()}"
                log_dir.rename(retained_failure)
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
                if cohort == "B1-instrumented":
                    command.extend(["-p", "hydracache-loadgen"])
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
                retained_loadgen = None
                if cohort == "B1-instrumented":
                    loadgen_name = "hydracache-loadgen.exe" if os.name == "nt" else "hydracache-loadgen"
                    built_loadgen = target_dir / "release" / loadgen_name
                    if not built_loadgen.is_file():
                        raise CampaignError(f"build did not produce {built_loadgen}")
                    retained_loadgen = log_dir / loadgen_name
                    shutil.copy2(built_loadgen, retained_loadgen)
                    retained_loadgen.chmod(0o555)
                manifest = {
                    "schema_version": SCHEMA_VERSION,
                    "release": RELEASE,
                    "cohort": cohort,
                    "source_sha": source_sha,
                    "cargo_lock_sha256": sha256(worktree / "Cargo.lock"),
                    "binary": str(retained_binary.resolve()),
                    "binary_sha256": sha256(retained_binary),
                    "loadgen_binary": str(retained_loadgen.resolve()) if retained_loadgen else None,
                    "loadgen_binary_sha256": sha256(retained_loadgen) if retained_loadgen else None,
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


def validate_admission_receipts(receipts: dict[str, dict[str, Any]], state: dict[str, Any]) -> None:
    host = receipts["host-preflight"]
    if (
        host.get("schema_version") != 1
        or host.get("release") != RELEASE
        or host.get("profile_id") != "memory-reference-071-v1"
        or host.get("protected_environment") != "memory-reference-071"
        or host.get("result") != "success"
        or host.get("ship_evidence_eligible") is not True
    ):
        raise CampaignError("host preflight receipt is not an admitted 0.71 reference host")
    bootstrap = receipts["reference-activation"]
    if (
        bootstrap.get("schema_version") != 1
        or bootstrap.get("release") != "0.67.1"
        or bootstrap.get("profile") != "reference-v1"
        or bootstrap.get("passed") is not True
        or bootstrap.get("ship_evidence_eligible") is not True
    ):
        raise CampaignError("0.67.1 reference activation receipt is invalid")
    historical = receipts["historical-input-receipt"]
    mirror = historical.get("mirror", {})
    if (
        historical.get("schema_version") != 1
        or historical.get("release") != RELEASE
        or historical.get("commit") != "dbc2f82f7f303528b3cca7842818730c82232b9c"
        or historical.get("checkout_clean") is not True
        or not historical.get("files")
        or mirror.get("manifest_sha256") != mirror.get("restored_manifest_sha256")
    ):
        raise CampaignError("historical protected-mirror receipt is invalid")
    overhead = receipts["instrumentation-overhead"]
    if (
        overhead.get("schema_version") != 1
        or overhead.get("release") != RELEASE
        or overhead.get("source_sha") != state["source_shas"].get("B1-instrumented")
        or overhead.get("host_fingerprint") != host.get("host_fingerprint")
        or overhead.get("passed") is not True
        or overhead.get("ship_evidence_eligible") is not True
    ):
        raise CampaignError("S5 instrumentation overhead receipt is invalid or cross-host")


def admit_campaign(
    campaign_dir: Path,
    host_preflight: Path,
    bootstrap: Path,
    historical: Path,
    overhead: Path,
) -> None:
    state_path = campaign_dir / "state.json"
    if not state_path.is_file():
        raise CampaignError(f"campaign state is missing: {state_path}")
    sources = {
        "host-preflight": host_preflight,
        "reference-activation": bootstrap,
        "historical-input-receipt": historical,
        "instrumentation-overhead": overhead,
    }
    documents: dict[str, dict[str, Any]] = {}
    for name, source in sources.items():
        if not source.is_file() or source.stat().st_size > 10 * 1024 * 1024:
            raise CampaignError(f"admission input is missing or oversized: {source}")
        documents[name] = json.loads(source.read_text(encoding="utf-8"))
    with CampaignLock(campaign_dir / ".lock"):
        state = json.loads(state_path.read_text(encoding="utf-8"))
        identity = retained_campaign_identity(campaign_dir, state)
        if state["mode"] != "evidence":
            raise CampaignError("admission receipts apply only to evidence campaigns")
        validate_admission_receipts(documents, state)
        admission_dir = campaign_dir / "admission"
        if admission_dir.exists():
            raise CampaignError("campaign admission is immutable and already exists")
        admission_dir.mkdir()
        manifest = []
        for name, source in sources.items():
            destination = admission_dir / f"{name}.json"
            shutil.copy2(source, destination)
            destination.chmod(0o444)
            manifest.append(
                {"id": name, "path": destination.name, "sha256": sha256(destination), "bytes": destination.stat().st_size}
            )
        atomic_json(
            admission_dir / "admission-manifest.json",
            {
                "schema_version": 1,
                "release": RELEASE,
                "campaign_identity": state["identity"],
                "workflow_sha": identity["workflow_sha"],
                "source_sha": identity["source_sha"],
                "campaign_role": identity["campaign_role"],
                "receipts": manifest,
            },
            create=True,
        )
        state["admission"] = {
            "manifest": "admission/admission-manifest.json",
            "manifest_sha256": sha256(admission_dir / "admission-manifest.json"),
            "admitted_at": utc_now(),
        }
        atomic_json(state_path, state)


def verify_live_host(campaign_dir: Path, host_preflight: Path) -> None:
    state = json.loads((campaign_dir / "state.json").read_text(encoding="utf-8"))
    retained_campaign_identity(campaign_dir, state)
    retained_path = campaign_dir / "admission" / "host-preflight.json"
    if not retained_path.is_file() or not state.get("admission"):
        raise CampaignError("campaign has no retained host admission")
    retained = json.loads(retained_path.read_text(encoding="utf-8"))
    observed = json.loads(host_preflight.read_text(encoding="utf-8"))
    if (
        observed.get("result") != "success"
        or observed.get("ship_evidence_eligible") is not True
        or observed.get("host_fingerprint") != retained.get("host_fingerprint")
        or observed.get("profile_id") != retained.get("profile_id")
    ):
        raise CampaignError("live host fingerprint or eligibility drifted from campaign admission")


def execute_rehearsal(root: Path, campaign_dir: Path, job: dict[str, Any]) -> tuple[str, list[str]]:
    output = campaign_dir / "jobs" / job["job_id"]
    command = [
        "cargo",
        "run",
        "-p",
        "hydracache-loadgen",
        "--bin",
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


def unsupported_evidence_reason(job: dict[str, Any]) -> str | None:
    case_id = job["case_id"]
    dimensions = job["dimensions"]
    return None


def execute_evidence(root: Path, campaign_dir: Path, state: dict[str, Any], job: dict[str, Any]) -> tuple[str, list[str]]:
    unsupported = unsupported_evidence_reason(job)
    if unsupported:
        raise CampaignError(unsupported)
    build = state.get("builds", {}).get(job["cohort"])
    if not build:
        raise CampaignError(f"prepared build is missing for {job['cohort']}")
    build_manifest = campaign_dir / build["manifest"]
    host_preflight = campaign_dir / "admission" / "host-preflight.json"
    admission = state.get("admission")
    if not admission:
        raise CampaignError("campaign has no immutable admission manifest")
    admission_manifest = campaign_dir / admission["manifest"]
    if not admission_manifest.is_file() or sha256(admission_manifest) != admission["manifest_sha256"]:
        raise CampaignError("campaign admission manifest drifted")
    if not host_preflight.is_file():
        raise CampaignError(f"admitted host receipt is missing: {host_preflight}")
    host = json.loads(host_preflight.read_text(encoding="utf-8"))
    if host.get("result") != "success" or host.get("ship_evidence_eligible") is not True:
        raise CampaignError("host preflight is not eligible for evidence execution")
    output = campaign_dir / "jobs" / job["job_id"]
    job_path = campaign_dir / "job-specs" / f"{job['job_id']}.json"
    atomic_json(job_path, job)
    executor = root / "scripts/perf/memory_case_executor_071.py"
    command = [
        sys.executable,
        str(executor),
        "--job",
        str(job_path),
        "--build-manifest",
        str(build_manifest),
        "--output",
        str(output),
        "--scenario-digest",
        state["scenario_digest"],
        "--provider",
        "system",
        "--host-preflight",
        str(host_preflight),
    ]
    if job["case_id"] == "M6-connections" or (
        job["case_id"] == "M8-60m" and job["dimensions"].get("sequence") == "hc2-churn"
    ) or job["case_id"] in {"M9-6h", "M10-24h"}:
        tools = state.get("controller_tools", {})
        helper_manifest = campaign_dir / tools.get("hc2_helper_manifest", "missing")
        if (
            not helper_manifest.is_file()
            or sha256(helper_manifest) != tools.get("hc2_helper_manifest_sha256")
        ):
            raise CampaignError("retained HC/2 helper manifest is missing or drifted")
        command.extend(["--hc2-helper-manifest", str(helper_manifest)])
    try:
        completed = subprocess.run(command, cwd=root, timeout=state["row_time_caps_seconds"][job["case_id"]], check=False)
    except subprocess.TimeoutExpired:
        return "timeout", command
    except OSError:
        return "tool-unavailable", command
    if completed.returncode != 0:
        return "product-failure", command
    report = output / "memory-baseline-report.json"
    validation = subprocess.run(
        [
            "cargo",
            "run",
            "-p",
            "xtask",
            "--locked",
            "--",
            "memory-baseline-report-check",
            "--release",
            RELEASE,
            "--report",
            str(report),
        ],
        cwd=root,
        check=False,
    )
    return ("success" if validation.returncode == 0 else "product-failure"), command


def publish_job(campaign_dir: Path, state: dict[str, Any], job: dict[str, Any]) -> None:
    mirror_value = state.get("mirror_root")
    if not mirror_value:
        if state["mode"] == "evidence":
            raise CampaignError("evidence campaign has no protected mirror root")
        return
    mirror_root = Path(mirror_value)
    destination_dir = mirror_root / state["campaign_id"] / "jobs"
    destination_dir.mkdir(parents=True, exist_ok=True)
    archive = destination_dir / f"{job['job_id']}.tar.gz"
    receipt = destination_dir / f"{job['job_id']}.json"
    retained = state.setdefault("published_jobs", {}).get(job["job_id"])
    if retained:
        if archive.is_file() and sha256(archive) == retained["archive_sha256"] and receipt.is_file():
            return
        raise CampaignError(f"protected mirror drift for {job['job_id']}")
    if archive.is_file() and receipt.is_file():
        recovered = json.loads(receipt.read_text(encoding="utf-8"))
        if (
            recovered.get("campaign_id") == state["campaign_id"]
            and recovered.get("job_id") == job["job_id"]
            and recovered.get("archive_sha256") == sha256(archive)
        ):
            state.setdefault("published_jobs", {})[job["job_id"]] = {
                "archive": str(archive),
                "archive_sha256": recovered["archive_sha256"],
                "receipt": str(receipt),
            }
            return
        raise CampaignError(f"protected mirror recovery receipt drift for {job['job_id']}")
    if archive.exists() or receipt.exists():
        raise CampaignError(f"refusing to overwrite unbound protected mirror object for {job['job_id']}")
    source = campaign_dir / "jobs" / job["job_id"]
    temporary = destination_dir / f".{job['job_id']}.{os.getpid()}.partial"
    with tarfile.open(temporary, "w:gz") as bundle:
        bundle.add(source, arcname=job["job_id"], recursive=True)
    archive_digest = sha256(temporary)
    os.replace(temporary, archive)
    mirror_receipt = {
        "schema_version": 1,
        "release": RELEASE,
        "campaign_id": state["campaign_id"],
        "workflow_sha": state["workflow_sha"],
        "source_sha": state["source_sha"],
        "campaign_role": state["campaign_role"],
        "job_id": job["job_id"],
        "archive": archive.name,
        "archive_sha256": archive_digest,
        "bytes": archive.stat().st_size,
        "published_at": utc_now(),
    }
    atomic_json(receipt, mirror_receipt, create=True)
    state["published_jobs"][job["job_id"]] = {
        "archive": str(archive),
        "archive_sha256": archive_digest,
        "receipt": str(receipt),
    }


def run_campaign(root: Path, campaign_dir: Path) -> None:
    state_path = campaign_dir / "state.json"
    events_path = campaign_dir / "events.jsonl"
    if not state_path.is_file():
        raise CampaignError(f"campaign state is missing: {state_path}")
    with CampaignLock(campaign_dir / ".lock"):
        state = json.loads(state_path.read_text(encoding="utf-8"))
        retained_campaign_identity(campaign_dir, state)
        for job in state["jobs"]:
            if job["status"] == "success":
                continue
            attempt = len(job["attempts"]) + 1
            append_event(events_path, {"event": "job-started", "job_id": job["job_id"], "attempt": attempt})
            started = time.monotonic_ns()
            status, command = (
                execute_rehearsal(root, campaign_dir, job)
                if state["mode"] == "rehearsal"
                else execute_evidence(root, campaign_dir, state, job)
            )
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
            if status == "success":
                publish_job(campaign_dir, state, job)
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
    identity = retained_campaign_identity(campaign_dir, state)
    incomplete = [job["job_id"] for job in state["jobs"] if job["status"] != "success"]
    if incomplete:
        raise CampaignError(f"campaign has {len(incomplete)} incomplete jobs")
    process_ids: set[int] = set()
    if state["mode"] == "evidence":
        for job in state["jobs"]:
            receipt_path = campaign_dir / "jobs" / job["job_id"] / "executor-receipt.json"
            if not receipt_path.is_file():
                raise CampaignError(f"executor receipt is missing for {job['job_id']}")
            executor = json.loads(receipt_path.read_text(encoding="utf-8"))
            pid = executor.get("pid")
            if (
                not isinstance(pid, int)
                or pid in process_ids
                or executor.get("fresh_process") is not True
                or executor.get("stopped") is not True
            ):
                raise CampaignError(f"fresh stopped process proof is invalid for {job['job_id']}")
            process_ids.add(pid)
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
        "campaign_id": state["campaign_id"],
        "workflow_sha": identity["workflow_sha"],
        "source_sha": identity["source_sha"],
        "campaign_role": identity["campaign_role"],
        "campaign_identity_sha256": state["identity"]["sha256"],
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
    plan_parser.add_argument("--mirror-root", type=Path)
    plan_parser.add_argument("--workflow-sha")
    plan_parser.add_argument("--source-sha")
    plan_parser.add_argument("--campaign-role", choices=sorted(CAMPAIGN_ROLES), default="baseline")
    identity_parser = commands.add_parser("verify-identity")
    identity_parser.add_argument("--campaign-id", required=True)
    identity_parser.add_argument("--workflow-sha", required=True)
    identity_parser.add_argument("--source-sha", required=True)
    identity_parser.add_argument("--campaign-role", choices=sorted(CAMPAIGN_ROLES), required=True)
    identity_parser.add_argument("--case", required=True)
    prepare_parser = commands.add_parser("prepare")
    prepare_parser.add_argument("--campaign-id", required=True)
    prepare_parser.add_argument("--build-root", type=Path, required=True)
    admit_parser = commands.add_parser("admit")
    admit_parser.add_argument("--campaign-id", required=True)
    admit_parser.add_argument("--host-preflight", type=Path, required=True)
    admit_parser.add_argument("--bootstrap", type=Path, required=True)
    admit_parser.add_argument("--historical", type=Path, required=True)
    admit_parser.add_argument("--overhead", type=Path, required=True)
    verify_host_parser = commands.add_parser("verify-host")
    verify_host_parser.add_argument("--campaign-id", required=True)
    verify_host_parser.add_argument("--host-preflight", type=Path, required=True)
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
            if not args.rehearsal and (not args.workflow_sha or not args.source_sha):
                raise CampaignError("evidence campaign requires --workflow-sha and --source-sha")
            cohorts = args.cohort or ["B1-instrumented"]
            plan = build_plan(
                root,
                args.case,
                cohorts,
                args.repetitions,
                args.rehearsal,
                args.workflow_sha,
                args.source_sha,
                args.campaign_role,
            )
            plan["campaign_id"] = args.campaign_id
            plan["mirror_root"] = str(args.mirror_root.resolve()) if args.mirror_root else None
            if not args.rehearsal:
                resolved_output = campaign_path(root, args.output_root, "_").parent
                if resolved_output == root or resolved_output.is_relative_to(root):
                    raise CampaignError("evidence campaign output must be outside the Git worktree")
                if not args.mirror_root:
                    raise CampaignError("evidence campaign requires --mirror-root")
                mirror_root = args.mirror_root.resolve()
                if mirror_root == root or mirror_root.is_relative_to(root):
                    raise CampaignError("protected mirror root must be outside the Git worktree")
            campaign_dir.mkdir(parents=True)
            identity_path = campaign_dir / "campaign-identity.json"
            atomic_json(identity_path, campaign_identity(plan), create=True)
            identity_path.chmod(0o444)
            plan["identity"] = {
                "path": identity_path.name,
                "sha256": sha256(identity_path),
            }
            atomic_json(campaign_dir / "state.json", plan, create=True)
            append_event(
                campaign_dir / "events.jsonl",
                {
                    "event": "campaign-planned",
                    "job_count": plan["job_count"],
                    "workflow_sha": plan["workflow_sha"],
                    "source_sha": plan["source_sha"],
                    "campaign_role": plan["campaign_role"],
                },
            )
            print(
                json.dumps(
                    {
                        key: plan[key]
                        for key in (
                            "mode",
                            "campaign_role",
                            "workflow_sha",
                            "source_sha",
                            "job_count",
                            "admitted_host_cap_seconds",
                        )
                    },
                    indent=2,
                )
            )
        elif args.command == "verify-identity":
            print(
                json.dumps(
                    verify_campaign_identity(
                        campaign_dir,
                        args.workflow_sha,
                        args.source_sha,
                        args.campaign_role,
                        args.case,
                    ),
                    indent=2,
                    sort_keys=True,
                )
            )
        elif args.command == "prepare":
            prepare_builds(root, campaign_dir, args.build_root)
        elif args.command == "admit":
            admit_campaign(
                campaign_dir,
                args.host_preflight,
                args.bootstrap,
                args.historical,
                args.overhead,
            )
        elif args.command == "verify-host":
            verify_live_host(campaign_dir, args.host_preflight)
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
