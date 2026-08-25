#!/usr/bin/env python3
"""Fail-closed controller for one HydraCache 0.67.1 reference campaign.

The controller intentionally starts after a human has rented and installed a
server. It automates the mutable host preparation, the explicit post-reboot
freeze/burn-in boundary, and the strictly serialized GitHub Actions chain. It
never creates, deletes, powers off, or resizes provider resources.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import stat
import subprocess
import sys
import time
from typing import Any, Iterable
import zipfile


SCHEMA_VERSION = 1
EXPECTED_REPOSITORY = "javaquasar/hydracache"
EXPECTED_BRANCH = "main"
WORKFLOW = "ci.yml"
EXPECTED_RUNNER_NAME = "hydracache-perf-v1"
EXPECTED_RUNNER_LABEL = "hydracache-perf-v1"
PROFILE_RELATIVE = Path("docs/testing/perf-host-profiles/ubuntu-24.04-reference-v1.json")
STATE_FILE = "campaign-state.json"
CAMPAIGN_RE = re.compile(r"^hc0671-[a-z0-9][a-z0-9-]{5,55}$")
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
RUN_ID_RE = re.compile(r"^[1-9][0-9]*$")
ACTIVE_PERF_TITLE_RE = re.compile(
    r"^CI dispatch (?:hc0671-[a-z0-9-]+:)?(?:qualification|qualify|full-dress(?:-[12])?|bootstrap(?:-[1-5])?|frozen-candidate)$"
)


class CampaignError(RuntimeError):
    """An admission or orchestration invariant failed."""


def utc_now() -> str:
    return dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat()


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def run_capture(
    args: Iterable[str], *, cwd: Path | None = None, check: bool = True
) -> str:
    completed = subprocess.run(
        list(args),
        cwd=cwd,
        check=False,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
    )
    if check and completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise CampaignError(f"command failed ({completed.returncode}): {detail}")
    return completed.stdout.strip()


def run_visible(args: Iterable[str], *, cwd: Path, log_path: Path) -> int:
    command = list(args)
    with log_path.open("a", encoding="utf-8", newline="\n") as log:
        log.write(f"[{utc_now()}] command: {' '.join(command)}\n")
        log.flush()
        process = subprocess.Popen(
            command,
            cwd=cwd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            encoding="utf-8",
            errors="replace",
        )
        assert process.stdout is not None
        try:
            for line in process.stdout:
                sys.stdout.write(line)
                sys.stdout.flush()
                log.write(line)
                log.flush()
            return process.wait()
        except BaseException:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.wait()
            raise


def require_tools(names: Iterable[str]) -> None:
    missing = [name for name in names if shutil.which(name) is None]
    if missing:
        raise CampaignError(f"missing required tools: {', '.join(missing)}")


def require_github_dispatch_readiness() -> None:
    """Fail before freeze if installing/authenticating gh would drift the host later."""
    require_tools(["gh"])
    run_capture(["gh", "auth", "status"], cwd=repo_root())


def sudo_prefix() -> list[str]:
    return [] if hasattr(os, "geteuid") and os.geteuid() == 0 else ["sudo"]


def sudo_command(*args: str) -> list[str]:
    return [*sudo_prefix(), *args]


def runner_command(*args: str) -> list[str]:
    return ["sudo", "-u", "github-runner", "-H", *args]


def ensure_checkout(expected_sha: str) -> None:
    root = repo_root()
    head = run_capture(["git", "rev-parse", "HEAD"], cwd=root)
    if head != expected_sha:
        raise CampaignError(f"checkout SHA drift: expected {expected_sha}, found {head}")
    status = run_capture(
        ["git", "status", "--porcelain=v1", "--untracked-files=normal", "--ignore-submodules=none"],
        cwd=root,
    )
    if status:
        raise CampaignError("campaign requires an exactly clean checkout")


def ensure_external_campaign_dir(path: Path) -> Path:
    resolved = path.expanduser().resolve()
    root = repo_root().resolve()
    if resolved == root or root in resolved.parents:
        raise CampaignError("campaign output must remain outside the Git worktree")
    return resolved


def validate_host_state_dir(path: str) -> str:
    resolved = Path(path).resolve()
    allowed = Path("/var/lib/hydracache-perf").resolve()
    if resolved == allowed or allowed not in resolved.parents:
        raise CampaignError("host state directory must be a child of /var/lib/hydracache-perf")
    if not re.fullmatch(r"host-tuning-[A-Za-z0-9._-]+", resolved.name):
        raise CampaignError("host state directory needs a host-tuning-* basename")
    return str(resolved)


def state_path(campaign_dir: Path) -> Path:
    return campaign_dir / STATE_FILE


def write_json_atomic(path: Path, value: Any, *, create: bool = False) -> None:
    payload = (json.dumps(value, indent=2, sort_keys=True) + "\n").encode("utf-8")
    if create:
        with path.open("xb") as stream:
            stream.write(payload)
        os.chmod(path, 0o600)
        return
    temporary = path.with_name(f".{path.name}.tmp-{os.getpid()}")
    try:
        with temporary.open("xb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, 0o600)
        os.replace(temporary, path)
    finally:
        if temporary.exists():
            temporary.unlink()


def load_state(campaign_dir: Path) -> dict[str, Any]:
    path = state_path(campaign_dir)
    try:
        state = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"cannot read campaign state: {error}") from error
    required = {
        "schema_version",
        "campaign_id",
        "repository",
        "branch",
        "workflow",
        "expected_sha",
        "profile",
        "profile_sha256",
        "host_state_dir",
        "phase",
        "created_at",
        "boot_id_before_prepare",
        "stages",
    }
    if set(state) != required:
        raise CampaignError("campaign state has missing or unknown top-level fields")
    if state["schema_version"] != SCHEMA_VERSION:
        raise CampaignError("unsupported campaign state schema")
    if not CAMPAIGN_RE.fullmatch(state["campaign_id"]):
        raise CampaignError("invalid campaign id in state")
    if state["repository"] != EXPECTED_REPOSITORY or state["branch"] != EXPECTED_BRANCH:
        raise CampaignError("campaign repository/branch contract drift")
    if state["workflow"] != WORKFLOW or not COMMIT_RE.fullmatch(state["expected_sha"]):
        raise CampaignError("campaign workflow/SHA contract drift")
    return state


def save_state(campaign_dir: Path, state: dict[str, Any]) -> None:
    write_json_atomic(state_path(campaign_dir), state)


def append_event(campaign_dir: Path, event: str, **fields: Any) -> None:
    record = {"timestamp": utc_now(), "event": event, **fields}
    with (campaign_dir / "campaign-events.jsonl").open("a", encoding="utf-8", newline="\n") as stream:
        stream.write(json.dumps(record, sort_keys=True, separators=(",", ":")) + "\n")
        stream.flush()
        os.fsync(stream.fileno())


def profile_path(state: dict[str, Any]) -> Path:
    path = Path(state["profile"])
    if path != repo_root() / PROFILE_RELATIVE:
        raise CampaignError("campaign profile path drift")
    if sha256_file(path) != state["profile_sha256"]:
        raise CampaignError("campaign profile digest drift")
    return path


def expand_cpu_list(value: str) -> set[int]:
    cpus: set[int] = set()
    for segment in value.split(","):
        bounds = segment.split("-", maxsplit=1)
        if not all(bound.isdigit() for bound in bounds):
            raise CampaignError(f"invalid CPU list: {value}")
        first = int(bounds[0])
        last = int(bounds[-1])
        if first > last:
            raise CampaignError(f"invalid CPU range: {segment}")
        cpus.update(range(first, last + 1))
    if not cpus:
        raise CampaignError("CPU list is empty")
    return cpus


def pin_controller_to_housekeeping(state: dict[str, Any]) -> None:
    profile = json.loads(profile_path(state).read_text(encoding="utf-8"))
    housekeeping = profile.get("cpu_contract", {}).get("housekeeping_cpus")
    if housekeeping != "0,5-7":
        raise CampaignError("campaign only supports the reviewed housekeeping CPU contract")
    if not hasattr(os, "sched_setaffinity") or not hasattr(os, "sched_getaffinity"):
        raise CampaignError("campaign controller requires Linux CPU affinity APIs")
    expected = expand_cpu_list(housekeeping)
    os.sched_setaffinity(0, expected)
    if os.sched_getaffinity(0) != expected:
        raise CampaignError("campaign controller effective affinity is not housekeeping-only")


def host_prepare_command(action: str, state: dict[str, Any]) -> list[str]:
    script = repo_root() / "scripts/perf/prepare-reference-host.sh"
    return sudo_command(
        str(script),
        action,
        "--profile",
        str(profile_path(state)),
        "--state-dir",
        state["host_state_dir"],
    )


def ensure_runner_offline(campaign_dir: Path) -> None:
    root = repo_root()
    log = campaign_dir / "host-lifecycle.log"
    runner_service = root / "scripts/perf/runner-service.sh"
    rootless = root / "scripts/perf/rootless-docker.sh"
    if run_visible(sudo_command(str(runner_service), "offline"), cwd=root, log_path=log) != 0:
        raise CampaignError("could not take the GitHub runner offline")
    if run_visible(runner_command(str(rootless), "stop"), cwd=root, log_path=log) != 0:
        raise CampaignError("could not stop rootless Docker")


def run_host_action(campaign_dir: Path, state: dict[str, Any], action: str) -> None:
    result = run_visible(
        host_prepare_command(action, state),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    if result != 0:
        raise CampaignError(f"host action failed: {action}")


def current_boot_id() -> str:
    return Path("/proc/sys/kernel/random/boot_id").read_text(encoding="ascii").strip()


def command_prepare(args: argparse.Namespace) -> None:
    if not args.confirm_host_mutation:
        raise CampaignError("prepare requires --confirm-host-mutation")
    if not CAMPAIGN_RE.fullmatch(args.campaign_id):
        raise CampaignError("campaign id must match hc0671-[a-z0-9-] and be 13..63 characters")
    if not COMMIT_RE.fullmatch(args.expected_sha):
        raise CampaignError("expected SHA must be exactly 40 lowercase hexadecimal characters")
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    if campaign_dir.exists():
        raise CampaignError(f"prepare refuses to reuse campaign directory: {campaign_dir}")
    host_state_dir = validate_host_state_dir(args.host_state_dir)
    ensure_checkout(args.expected_sha)
    campaign_dir.mkdir(parents=True, mode=0o700)
    profile = repo_root() / PROFILE_RELATIVE
    state: dict[str, Any] = {
        "schema_version": SCHEMA_VERSION,
        "campaign_id": args.campaign_id,
        "repository": EXPECTED_REPOSITORY,
        "branch": EXPECTED_BRANCH,
        "workflow": WORKFLOW,
        "expected_sha": args.expected_sha,
        "profile": str(profile),
        "profile_sha256": sha256_file(profile),
        "host_state_dir": host_state_dir,
        "phase": "preparing",
        "created_at": utc_now(),
        "boot_id_before_prepare": current_boot_id(),
        "stages": {},
    }
    write_json_atomic(state_path(campaign_dir), state, create=True)
    append_event(campaign_dir, "campaign-created", expected_sha=args.expected_sha)
    try:
        pin_controller_to_housekeeping(state)
        ensure_runner_offline(campaign_dir)
        for action in ("preflight", "apply-services", "install-isolation"):
            append_event(campaign_dir, "host-action-started", action=action)
            run_host_action(campaign_dir, state, action)
            append_event(campaign_dir, "host-action-completed", action=action)
        state["phase"] = "reboot-required"
        save_state(campaign_dir, state)
    except Exception:
        state["phase"] = "preparation-failed"
        save_state(campaign_dir, state)
        append_event(campaign_dir, "campaign-rejected", phase="prepare")
        raise
    print(f"REBOOT_REQUIRED=true campaign_dir={campaign_dir}")


def archive_host_state(campaign_dir: Path, state: dict[str, Any], name: str) -> Path:
    host_state = Path(state["host_state_dir"])
    archive = campaign_dir / name
    if archive.exists():
        raise CampaignError(f"refusing to overwrite host-state archive: {archive}")
    result = run_visible(
        sudo_command(
            "tar",
            "--create",
            "--gzip",
            "--numeric-owner",
            "--file",
            str(archive),
            "--directory",
            str(host_state.parent),
            host_state.name,
        ),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    if result != 0:
        raise CampaignError("could not archive the immutable host state")
    result = run_visible(
        sudo_command("chmod", "0444", str(archive)),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    if result != 0:
        raise CampaignError("could not make host-state archive read-only")
    return archive


def publish_host_admission(
    campaign_dir: Path,
    state: dict[str, Any],
    host_archive: Path,
    burn_receipt: Path,
) -> tuple[Path, Path]:
    bundle = campaign_dir / "reference-campaign-host-admission.tar.gz"
    receipt_path = campaign_dir / "reference-campaign-admission.json"
    canonical = Path("/var/lib/hydracache-perf/reference-campaign-v1")
    for path in (bundle, receipt_path):
        if path.exists():
            raise CampaignError(f"refusing to overwrite host admission output: {path}")
    result = run_visible(
        sudo_command(
            "tar",
            "--create",
            "--gzip",
            "--numeric-owner",
            "--file",
            str(bundle),
            "--directory",
            str(campaign_dir),
            host_archive.name,
            "irq-burn-in",
        ),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    if result != 0:
        raise CampaignError("could not build the host admission bundle")
    run_visible(
        sudo_command("chmod", "0444", str(bundle)),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    burn = validate_burn_receipt(burn_receipt, state)
    receipt = {
        "schema_version": 1,
        "release": "0.67.1",
        "stage": "reference-campaign-host-admission",
        "campaign_id": state["campaign_id"],
        "source_commit": state["expected_sha"],
        "profile_sha256": state["profile_sha256"],
        "host_state_archive_sha256": sha256_file(host_archive),
        "irq_burn_in_receipt_sha256": sha256_file(burn_receipt),
        "irq_baseline_sha256": burn["irq_baseline_sha256"],
        "host_admission_bundle_sha256": sha256_file(bundle),
        "host_frozen": True,
        "irq_burn_in_passed": True,
        "passed": True,
        "qualification_evidence": False,
        "bootstrap_evidence": False,
        "ship_evidence_eligible": False,
    }
    write_json_atomic(receipt_path, receipt, create=True)
    os.chmod(receipt_path, 0o444)
    exists = subprocess.run(
        sudo_command("test", "-e", str(canonical)),
        cwd=repo_root(),
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode == 0
    if exists:
        raise CampaignError(f"canonical host admission already exists: {canonical}")
    commands = [
        sudo_command("install", "--directory", "--mode=0755", str(canonical)),
        sudo_command(
            "install",
            "--mode=0444",
            "--owner=root",
            "--group=root",
            str(bundle),
            str(canonical / bundle.name),
        ),
        sudo_command(
            "install",
            "--mode=0444",
            "--owner=root",
            "--group=root",
            str(receipt_path),
            str(canonical / receipt_path.name),
        ),
        sudo_command("chmod", "0555", str(canonical)),
    ]
    for command in commands:
        if run_visible(command, cwd=repo_root(), log_path=campaign_dir / "host-lifecycle.log") != 0:
            raise CampaignError("could not publish the canonical host admission")
    return bundle, receipt_path


def canonical_admission_matches(canonical: Path, state: dict[str, Any]) -> bool:
    host_admission = state.get("stages", {}).get("host_admission", {})
    expected_receipt = host_admission.get("host_admission_receipt_sha256")
    expected_bundle = host_admission.get("host_admission_bundle_sha256")
    receipt_path = canonical / "reference-campaign-admission.json"
    bundle_path = canonical / "reference-campaign-host-admission.tar.gz"
    if not expected_receipt or not expected_bundle:
        return False
    try:
        receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
        return (
            receipt.get("campaign_id") == state["campaign_id"]
            and receipt.get("source_commit") == state["expected_sha"]
            and sha256_file(receipt_path) == expected_receipt
            and sha256_file(bundle_path) == expected_bundle
        )
    except (OSError, json.JSONDecodeError):
        return False


def retire_canonical_host_admission(campaign_dir: Path, state: dict[str, Any]) -> Path | None:
    canonical = Path("/var/lib/hydracache-perf/reference-campaign-v1")
    if not canonical.exists():
        return None
    if not canonical_admission_matches(canonical, state):
        raise CampaignError("canonical host admission belongs to another campaign or has drifted")
    retired = campaign_dir / "retired-reference-campaign-v1"
    if retired.exists():
        raise CampaignError(f"retired canonical admission already exists: {retired}")
    if run_visible(
        sudo_command("mv", str(canonical), str(retired)),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    ) != 0:
        raise CampaignError("could not retire canonical host admission")
    return retired


def validate_runner_provisioning_receipt(path: Path, state: dict[str, Any]) -> dict[str, Any]:
    try:
        receipt = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"invalid runner provisioning receipt: {error}") from error
    if (
        receipt.get("schema_version") != 4
        or receipt.get("release") != "0.67.1"
        or receipt.get("stage") != "runner-provisioned"
        or receipt.get("source_commit") != state["expected_sha"]
        or receipt.get("runner_name") != EXPECTED_RUNNER_NAME
        or receipt.get("runner_online") is not False
        or receipt.get("ship_evidence_eligible") is not False
    ):
        raise CampaignError("runner provisioning receipt does not match the frozen campaign")
    return receipt


def publish_runner_provisioning_receipt(
    campaign_dir: Path, state: dict[str, Any]
) -> tuple[Path, Path | None]:
    source = repo_root() / "target/test-evidence/0.67.1/runner-provisioned.json"
    validate_runner_provisioning_receipt(source, state)
    canonical = Path("/var/lib/hydracache-perf/runner-provisioned.json")
    if canonical.exists() and sha256_file(canonical) == sha256_file(source):
        return canonical, None
    previous = campaign_dir / "previous-runner-provisioned.json"
    if canonical.exists():
        if previous.exists():
            raise CampaignError(f"previous runner receipt archive already exists: {previous}")
        if run_visible(
            sudo_command("mv", str(canonical), str(previous)),
            cwd=repo_root(),
            log_path=campaign_dir / "host-lifecycle.log",
        ) != 0:
            raise CampaignError("could not archive the previous runner provisioning receipt")
    temporary = canonical.with_name(f".runner-provisioned-{state['campaign_id']}.tmp")
    if subprocess.run(
        sudo_command("test", "-e", str(temporary)),
        cwd=repo_root(),
        check=False,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    ).returncode == 0:
        raise CampaignError(f"temporary runner receipt already exists: {temporary}")
    commands = [
        sudo_command(
            "install", "--mode=0444", "--owner=root", "--group=root", str(source), str(temporary)
        ),
        sudo_command("mv", str(temporary), str(canonical)),
    ]
    for command in commands:
        if run_visible(command, cwd=repo_root(), log_path=campaign_dir / "host-lifecycle.log") != 0:
            raise CampaignError("could not publish the current runner provisioning receipt")
    return canonical, previous if previous.exists() else None


def validate_burn_receipt(path: Path, state: dict[str, Any]) -> dict[str, Any]:
    try:
        receipt = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CampaignError(f"invalid IRQ burn-in receipt: {error}") from error
    if receipt.get("schema_version") != 2 or receipt.get("stage") != "reference-host-irq-burn-in":
        raise CampaignError("wrong IRQ burn-in receipt schema/stage")
    if receipt.get("source_commit") != state["expected_sha"]:
        raise CampaignError("IRQ burn-in source SHA mismatch")
    if receipt.get("profile_sha256") != state["profile_sha256"]:
        raise CampaignError("IRQ burn-in profile digest mismatch")
    if (
        receipt.get("measurement_cpus") != "1-4"
        or receipt.get("storage_io_cpus") != "0,5-7"
        or receipt.get("duration_seconds", 0) < 600
    ):
        raise CampaignError("IRQ burn-in measurement/duration contract mismatch")
    if receipt.get("passed") is not True or receipt.get("failure_step") is not None:
        raise CampaignError("IRQ burn-in did not pass")
    if receipt.get("qualification_evidence") is not False or receipt.get("bootstrap_evidence") is not False:
        raise CampaignError("IRQ burn-in must remain non-evidence")
    if receipt.get("ship_evidence_eligible") is not False:
        raise CampaignError("IRQ burn-in cannot be ship evidence")
    for field in ("irq_baseline_sha256", "interrupts_before_sha256", "interrupts_after_sha256"):
        if not re.fullmatch(r"[0-9a-f]{64}", receipt.get(field, "")):
            raise CampaignError(f"IRQ burn-in lacks {field}")
    return receipt


def command_freeze(args: argparse.Namespace) -> None:
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    state = load_state(campaign_dir)
    if state["phase"] != "reboot-required":
        raise CampaignError(f"freeze requires reboot-required state, found {state['phase']}")
    if current_boot_id() == state["boot_id_before_prepare"]:
        raise CampaignError("the required reboot has not occurred")
    ensure_checkout(state["expected_sha"])
    require_github_dispatch_readiness()
    try:
        pin_controller_to_housekeeping(state)
        ensure_runner_offline(campaign_dir)
        for action in ("verify", "freeze", "check-frozen"):
            append_event(campaign_dir, "host-action-started", action=action)
            run_host_action(campaign_dir, state, action)
            append_event(campaign_dir, "host-action-completed", action=action)
        archive = archive_host_state(campaign_dir, state, "host-state-after-freeze.tar.gz")
        burn_dir = campaign_dir / "irq-burn-in"
        burn = repo_root() / "scripts/perf/reference-host-irq-burn-in.sh"
        burn_command = sudo_command(
            str(burn),
            "--output-dir",
            str(burn_dir),
            "--profile",
            str(profile_path(state)),
            "--duration-seconds",
            str(args.duration_seconds),
            "--read-mebibytes",
            str(args.read_mebibytes),
        )
        if args.network_target:
            burn_command.extend(["--network-target", args.network_target])
        append_event(campaign_dir, "irq-burn-in-started")
        result = run_visible(
            burn_command,
            cwd=repo_root(),
            log_path=campaign_dir / "host-lifecycle.log",
        )
        if result != 0:
            raise CampaignError("IRQ burn-in rejected this host allocation")
        burn_receipt = burn_dir / "irq-burn-in.json"
        validate_burn_receipt(burn_receipt, state)
        run_host_action(campaign_dir, state, "check-frozen")
        runner_receipt, previous_runner_receipt = publish_runner_provisioning_receipt(
            campaign_dir, state
        )
        admission_bundle, admission_receipt = publish_host_admission(
            campaign_dir, state, archive, burn_receipt
        )
        state["phase"] = "ready"
        state["stages"] = {
            "host_admission": {
                "status": "completed",
                "host_archive": str(archive),
                "host_archive_sha256": sha256_file(archive),
                "irq_burn_in_receipt": str(burn_receipt),
                "irq_burn_in_receipt_sha256": sha256_file(burn_receipt),
                "host_admission_bundle": str(admission_bundle),
                "host_admission_bundle_sha256": sha256_file(admission_bundle),
                "host_admission_receipt": str(admission_receipt),
                "host_admission_receipt_sha256": sha256_file(admission_receipt),
                "runner_provisioning_receipt": str(runner_receipt),
                "runner_provisioning_receipt_sha256": sha256_file(runner_receipt),
                "previous_runner_provisioning_receipt": (
                    str(previous_runner_receipt) if previous_runner_receipt is not None else None
                ),
            }
        }
        save_state(campaign_dir, state)
        append_event(campaign_dir, "host-admission-completed")
    except Exception:
        state["phase"] = "host-admission-failed"
        save_state(campaign_dir, state)
        append_event(campaign_dir, "campaign-rejected", phase="freeze")
        raise
    print(f"HOST_ADMISSION_PASSED=true campaign_dir={campaign_dir}")


def gh_json(args: Iterable[str]) -> Any:
    output = run_capture(["gh", *args], cwd=repo_root())
    try:
        return json.loads(output)
    except json.JSONDecodeError as error:
        raise CampaignError(f"GitHub CLI returned malformed JSON: {error}") from error


def github_main_sha(state: dict[str, Any]) -> str:
    return run_capture(
        [
            "gh",
            "api",
            f"repos/{state['repository']}/commits/{state['branch']}",
            "--jq",
            ".sha",
        ],
        cwd=repo_root(),
    )


def ensure_github_runner_contract(state: dict[str, Any]) -> None:
    value = gh_json(["api", f"repos/{state['repository']}/actions/runners"])
    if not isinstance(value, dict) or not isinstance(value.get("runners"), list):
        raise CampaignError("GitHub runner listing is not an object with runners")
    matches = [runner for runner in value["runners"] if runner.get("name") == EXPECTED_RUNNER_NAME]
    if len(matches) != 1:
        raise CampaignError(f"expected exactly one GitHub runner named {EXPECTED_RUNNER_NAME}")
    runner = matches[0]
    labels = {
        label.get("name")
        for label in runner.get("labels", [])
        if isinstance(label, dict) and isinstance(label.get("name"), str)
    }
    if EXPECTED_RUNNER_LABEL not in labels:
        raise CampaignError(
            f"GitHub runner {EXPECTED_RUNNER_NAME} lacks required label {EXPECTED_RUNNER_LABEL}"
        )
    if runner.get("busy") is not False:
        raise CampaignError(f"GitHub runner {EXPECTED_RUNNER_NAME} is already busy")


def expected_title(state: dict[str, Any], step: str) -> str:
    return f"CI dispatch {state['campaign_id']}:{step}"


def list_dispatch_runs(state: dict[str, Any], status: str | None = None) -> list[dict[str, Any]]:
    command = [
        "run",
        "list",
        "--repo",
        state["repository"],
        "--workflow",
        state["workflow"],
        "--event",
        "workflow_dispatch",
        "--branch",
        state["branch"],
        "--limit",
        "100",
        "--json",
        "databaseId,displayTitle,headSha,status,conclusion,createdAt,url",
    ]
    if status:
        command.extend(["--status", status])
    result = gh_json(command)
    if not isinstance(result, list):
        raise CampaignError("GitHub run listing is not an array")
    return result


def matching_runs(state: dict[str, Any], step: str) -> list[dict[str, Any]]:
    title = expected_title(state, step)
    return [
        run
        for run in list_dispatch_runs(state)
        if run.get("displayTitle") == title and run.get("headSha") == state["expected_sha"]
    ]


def assert_no_foreign_reference_runs(state: dict[str, Any], allowed_run_id: int | None = None) -> None:
    active: list[dict[str, Any]] = []
    for status in ("queued", "in_progress"):
        active.extend(list_dispatch_runs(state, status))
    foreign = [
        run
        for run in active
        if ACTIVE_PERF_TITLE_RE.fullmatch(str(run.get("displayTitle", "")))
        and run.get("databaseId") != allowed_run_id
    ]
    if foreign:
        details = ", ".join(f"{run['databaseId']}:{run['displayTitle']}" for run in foreign)
        raise CampaignError(f"foreign reference performance run is active or queued: {details}")


def stage_specs(state: dict[str, Any]) -> list[dict[str, Any]]:
    stages = state["stages"]

    def completed_run(name: str) -> str:
        stage = stages.get(name, {})
        run_id = str(stage.get("run_id", ""))
        if stage.get("status") != "completed" or not RUN_ID_RE.fullmatch(run_id):
            raise CampaignError(f"stage prerequisite is not complete: {name}")
        return run_id

    specs: list[dict[str, Any]] = [{"name": "qualification", "mode": "qualify"}]
    if stages.get("qualification", {}).get("status") == "completed":
        specs.append({"name": "full-dress-1", "mode": "full-dress", "full_predecessor": ""})
    if stages.get("full-dress-1", {}).get("status") == "completed":
        specs.append(
            {
                "name": "full-dress-2",
                "mode": "full-dress",
                "full_predecessor": completed_run("full-dress-1"),
            }
        )
    if stages.get("full-dress-2", {}).get("status") == "completed":
        admission_run = completed_run("full-dress-2")
        for index in range(1, 6):
            predecessor = "" if index == 1 else completed_run(f"bootstrap-{index - 1}")
            specs.append(
                {
                    "name": f"bootstrap-{index}",
                    "mode": "bootstrap",
                    "sample_index": str(index),
                    "admission_run": admission_run,
                    "bootstrap_predecessor": predecessor,
                }
            )
            if stages.get(f"bootstrap-{index}", {}).get("status") != "completed":
                break
    return specs


def dispatch_fields(state: dict[str, Any], spec: dict[str, Any]) -> list[str]:
    fields = {
        "candidate_release": "0.67.1",
        "performance_0671_mode": spec["mode"],
        "performance_0671_campaign": f"{state['campaign_id']}:{spec['name']}",
    }
    if spec["mode"] == "full-dress":
        fields["full_dress_predecessor_run_id"] = spec["full_predecessor"]
    if spec["mode"] == "bootstrap":
        fields["full_dress_admission_run_id"] = spec["admission_run"]
        fields["bootstrap_sample_index"] = spec["sample_index"]
        fields["bootstrap_predecessor_run_id"] = spec["bootstrap_predecessor"]
    result: list[str] = []
    for key, value in fields.items():
        result.extend(["--field", f"{key}={value}"])
    return result


def discover_run(state: dict[str, Any], step: str, timeout_seconds: int = 120) -> dict[str, Any]:
    deadline = time.monotonic() + timeout_seconds
    while time.monotonic() < deadline:
        matches = matching_runs(state, step)
        if len(matches) == 1:
            return matches[0]
        if len(matches) > 1:
            raise CampaignError(f"campaign step correlation is ambiguous: {step}")
        time.sleep(3)
    raise CampaignError(f"timed out discovering dispatched run for {step}")


def runner_online(campaign_dir: Path) -> None:
    script = repo_root() / "scripts/perf/runner-service.sh"
    if run_visible(sudo_command(str(script), "online"), cwd=repo_root(), log_path=campaign_dir / "host-lifecycle.log") != 0:
        raise CampaignError("could not bring the runner online")


def watchdog_units(state: dict[str, Any], step: str) -> tuple[str, str]:
    stem = f"hydracache-runner-lease-{state['campaign_id']}-{step}"
    return f"{stem}.service", f"{stem}.timer"


def arm_runner_watchdog(campaign_dir: Path, state: dict[str, Any], step: str) -> None:
    service, _ = watchdog_units(state, step)
    command = sudo_command(
        "systemd-run",
        "--collect",
        f"--unit={service.removesuffix('.service')}",
        "--on-active=370m",
        str(repo_root() / "scripts/perf/runner-service.sh"),
        "offline",
    )
    if run_visible(command, cwd=repo_root(), log_path=campaign_dir / "host-lifecycle.log") != 0:
        raise CampaignError("could not arm the runner offline watchdog")


def disarm_runner_watchdog(campaign_dir: Path, state: dict[str, Any], step: str) -> None:
    service, timer = watchdog_units(state, step)
    run_visible(
        sudo_command("systemctl", "stop", timer),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    for unit in (service, timer):
        run_visible(
            sudo_command("systemctl", "reset-failed", unit),
            cwd=repo_root(),
            log_path=campaign_dir / "host-lifecycle.log",
        )
    run_visible(
        sudo_command("systemctl", "daemon-reload"),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    for _ in range(20):
        load_states = [
            run_capture(
                sudo_command("systemctl", "show", "--property=LoadState", "--value", unit),
                cwd=repo_root(),
                check=False,
            )
            for unit in (service, timer)
        ]
        if all(value in {"", "not-found"} for value in load_states):
            return
        time.sleep(0.5)
    raise CampaignError("runner watchdog transient units did not unload")


def download_binary(args: list[str], output: Path) -> None:
    partial = output.with_name(f".{output.name}.partial")
    if partial.exists():
        partial.unlink()
    with partial.open("xb") as stream:
        completed = subprocess.run(args, cwd=repo_root(), stdout=stream, stderr=subprocess.PIPE, check=False)
    if completed.returncode != 0:
        partial.unlink(missing_ok=True)
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise CampaignError(f"artifact download failed: {detail}")
    os.replace(partial, output)
    os.chmod(output, 0o444)


def safe_artifact_filename(artifact_id: int, name: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-.")
    if not slug:
        raise CampaignError("artifact name cannot be represented safely")
    return f"{artifact_id}-{slug}.zip"


def download_artifacts(campaign_dir: Path, state: dict[str, Any], run_id: int, step: str) -> dict[str, Path]:
    run_dir = campaign_dir / "runs" / f"{step}-{run_id}"
    manifest_path = run_dir / "artifact-manifest.json"
    if run_dir.exists():
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise CampaignError(f"incomplete prior artifact download for {step}: {error}") from error
        paths: dict[str, Path] = {}
        for artifact in manifest.get("artifacts", []):
            path = campaign_dir / artifact.get("archive_file", "")
            if (
                not path.is_file()
                or sha256_file(path) != artifact.get("archive_sha256")
                or artifact.get("name") in paths
            ):
                raise CampaignError(f"retained artifact drift for {step}")
            paths[artifact["name"]] = path
        if not paths:
            raise CampaignError(f"empty retained artifact manifest for {step}")
        return paths

    runs_dir = campaign_dir / "runs"
    runs_dir.mkdir(mode=0o700, exist_ok=True)
    staging = runs_dir / f".{step}-{run_id}.partial"
    if staging.exists():
        shutil.rmtree(staging)
    originals = staging / "original-artifacts"
    originals.mkdir(parents=True, mode=0o700)
    response = gh_json(
        [
            "api",
            f"repos/{state['repository']}/actions/runs/{run_id}/artifacts?per_page=100",
        ]
    )
    artifacts = response.get("artifacts") if isinstance(response, dict) else None
    if not isinstance(artifacts, list) or not artifacts:
        raise CampaignError(f"run {run_id} has no downloadable artifacts")
    names: set[str] = set()
    paths: dict[str, Path] = {}
    manifest: list[dict[str, Any]] = []
    for artifact in artifacts:
        artifact_id = artifact.get("id")
        name = artifact.get("name")
        if not isinstance(artifact_id, int) or not isinstance(name, str) or name in names:
            raise CampaignError("artifact API returned invalid or duplicate identity")
        if artifact.get("expired") is True:
            raise CampaignError(f"artifact already expired: {name}")
        names.add(name)
        output = originals / safe_artifact_filename(artifact_id, name)
        download_binary(
            [
                "gh",
                "api",
                f"repos/{state['repository']}/actions/artifacts/{artifact_id}/zip",
            ],
            output,
        )
        try:
            with zipfile.ZipFile(output) as archive:
                corrupt = archive.testzip()
        except zipfile.BadZipFile as error:
            raise CampaignError(f"artifact is not a valid ZIP: {name}") from error
        if corrupt is not None:
            raise CampaignError(f"artifact ZIP has a corrupt member: {name}:{corrupt}")
        paths[name] = output
        manifest.append(
            {
                "artifact_id": artifact_id,
                "name": name,
                "reported_size_bytes": artifact.get("size_in_bytes"),
                "archive_file": str((run_dir / "original-artifacts" / output.name).relative_to(campaign_dir)),
                "archive_size_bytes": output.stat().st_size,
                "archive_sha256": sha256_file(output),
            }
        )
    write_json_atomic(
        staging / "artifact-manifest.json",
        {"schema_version": 1, "run_id": run_id, "step": step, "artifacts": manifest},
        create=True,
    )
    os.replace(staging, run_dir)
    return {name: run_dir / "original-artifacts" / path.name for name, path in paths.items()}


def artifact_by_prefix(artifacts: dict[str, Path], prefix: str) -> Path:
    matches = [path for name, path in artifacts.items() if name.startswith(prefix)]
    if len(matches) != 1:
        raise CampaignError(f"expected exactly one artifact with prefix {prefix}, found {len(matches)}")
    return matches[0]


def read_unique_member(archive_path: Path, basename: str) -> bytes:
    with zipfile.ZipFile(archive_path) as archive:
        matches = [name for name in archive.namelist() if Path(name).name == basename and not name.endswith("/")]
        if len(matches) != 1:
            raise CampaignError(f"expected one {basename} in {archive_path.name}, found {len(matches)}")
        info = archive.getinfo(matches[0])
        if info.file_size > 64 * 1024 * 1024:
            raise CampaignError(f"receipt is unexpectedly large: {basename}")
        return archive.read(info)


def read_evidence_member(archive_path: Path, relative: str, expected_sha256: str) -> bytes:
    """Read one exact bounded evidence member without trusting ZIP paths."""
    if (
        not relative.startswith("target/test-evidence/0.67/")
        or "\\" in relative
        or relative.startswith("/")
        or any(part in {"", ".", ".."} for part in relative.split("/"))
        or not re.fullmatch(r"[0-9a-f]{64}", expected_sha256)
    ):
        raise CampaignError(f"unsafe bootstrap evidence identity: {relative}")
    with zipfile.ZipFile(archive_path) as archive:
        matches: list[zipfile.ZipInfo] = []
        for info in archive.infolist():
            name = info.filename.removeprefix("./")
            if info.is_dir() or name.startswith("/") or "\\" in name or ".." in name.split("/"):
                continue
            if name == relative or name.endswith(f"/{relative}"):
                matches.append(info)
        if len(matches) != 1:
            raise CampaignError(
                f"expected one exact {relative} in {archive_path.name}, found {len(matches)}"
            )
        info = matches[0]
        unix_mode = info.external_attr >> 16
        if stat.S_ISLNK(unix_mode) or info.flag_bits & 0x1 or info.file_size > 64 * 1024 * 1024:
            raise CampaignError(f"unsafe or oversized ZIP evidence member: {relative}")
        data = archive.read(info)
    if sha256_bytes(data) != expected_sha256:
        raise CampaignError(f"bootstrap evidence digest mismatch: {relative}")
    return data


def materialize_bootstrap_input(
    campaign_dir: Path,
    index: int,
    receipt_data: bytes,
    diagnostic_archive: Path,
) -> Path:
    receipt = json_receipt(receipt_data, f"bootstrap sample {index}")
    if receipt.get("sample_index") != index:
        raise CampaignError(f"bootstrap input index mismatch: {index}")
    evidence = receipt.get("evidence_files")
    if not isinstance(evidence, list) or not evidence:
        raise CampaignError(f"bootstrap sample {index} has no evidence manifest")
    if len(evidence) > 256:
        raise CampaignError(f"bootstrap sample {index} evidence manifest is oversized")
    payloads: dict[str, bytes] = {"bootstrap-sample.json": receipt_data}
    digests: dict[str, str] = {"bootstrap-sample.json": sha256_bytes(receipt_data)}
    total_bytes = len(receipt_data)
    for item in evidence:
        if not isinstance(item, dict):
            raise CampaignError(f"bootstrap sample {index} has malformed evidence identity")
        relative = item.get("path")
        digest = item.get("sha256")
        if not isinstance(relative, str) or not isinstance(digest, str) or relative in payloads:
            raise CampaignError(f"bootstrap sample {index} has duplicate/malformed evidence")
        data = read_evidence_member(diagnostic_archive, relative, digest)
        total_bytes += len(data)
        if total_bytes > 512 * 1024 * 1024:
            raise CampaignError(f"bootstrap sample {index} evidence exceeds 512 MiB")
        payloads[relative] = data
        digests[relative] = digest

    output = campaign_dir / "reference-inputs" / f"sample-{index}"
    if output.exists():
        for relative, digest in digests.items():
            path = output / relative
            if not path.is_file() or path.is_symlink() or sha256_file(path) != digest:
                raise CampaignError(f"materialized bootstrap input drift: sample-{index}/{relative}")
        manifest = json.loads((output / "materialization.json").read_text(encoding="utf-8"))
        if manifest.get("files") != digests:
            raise CampaignError(f"materialized bootstrap manifest drift: sample-{index}")
        return output

    parent = output.parent
    parent.mkdir(mode=0o700, exist_ok=True)
    staging = parent / f".sample-{index}.partial"
    if staging.exists():
        shutil.rmtree(staging)
    staging.mkdir(mode=0o700)
    try:
        for relative, data in payloads.items():
            path = staging / relative
            path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
            with path.open("xb") as stream:
                stream.write(data)
            os.chmod(path, 0o444)
        write_json_atomic(
            staging / "materialization.json",
            {
                "schema_version": 1,
                "sample_index": index,
                "diagnostic_archive_sha256": sha256_file(diagnostic_archive),
                "files": digests,
            },
            create=True,
        )
        os.replace(staging, output)
    finally:
        if staging.exists():
            shutil.rmtree(staging)
    return output


def validate_host_admission_artifact(
    campaign_dir: Path,
    state: dict[str, Any],
    diagnostic: Path,
) -> None:
    host = state["stages"].get("host_admission", {})
    expected_receipt = Path(host.get("host_admission_receipt", ""))
    expected_bundle = Path(host.get("host_admission_bundle", ""))
    if not expected_receipt.is_file() or not expected_bundle.is_file():
        raise CampaignError("controller host admission files are missing")
    imported_receipt = read_unique_member(diagnostic, "reference-campaign-admission.json")
    imported_bundle = read_unique_member(diagnostic, "reference-campaign-host-admission.tar.gz")
    if imported_receipt != expected_receipt.read_bytes():
        raise CampaignError("GitHub artifact host admission receipt differs from the frozen original")
    if sha256_bytes(imported_bundle) != host.get("host_admission_bundle_sha256"):
        raise CampaignError("GitHub artifact host admission bundle differs from the frozen original")
    receipt = json_receipt(imported_receipt, "imported host admission")
    if (
        receipt.get("campaign_id") != state["campaign_id"]
        or receipt.get("source_commit") != state["expected_sha"]
        or receipt.get("passed") is not True
        or receipt.get("ship_evidence_eligible") is not False
    ):
        raise CampaignError("imported host admission identity/eligibility mismatch")


def json_receipt(data: bytes, label: str) -> dict[str, Any]:
    try:
        value = json.loads(data)
    except json.JSONDecodeError as error:
        raise CampaignError(f"{label} is malformed JSON: {error}") from error
    if not isinstance(value, dict):
        raise CampaignError(f"{label} is not a JSON object")
    return value


def retain_receipt(campaign_dir: Path, name: str, data: bytes) -> Path:
    accepted = campaign_dir / "accepted-receipts"
    accepted.mkdir(mode=0o700, exist_ok=True)
    relative = Path(name)
    if relative.is_absolute() or ".." in relative.parts:
        raise CampaignError(f"unsafe retained receipt path: {name}")
    output = accepted / relative
    output.parent.mkdir(mode=0o700, exist_ok=True)
    if output.exists():
        if output.read_bytes() != data:
            raise CampaignError(f"accepted receipt changed on resume: {name}")
        return output
    with output.open("xb") as stream:
        stream.write(data)
    os.chmod(output, 0o444)
    return output


def expect_common_receipt(receipt: dict[str, Any], state: dict[str, Any], run_id: int) -> None:
    if receipt.get("source_commit") != state["expected_sha"]:
        raise CampaignError("receipt source commit mismatch")
    if receipt.get("github_run_id") != str(run_id):
        raise CampaignError("receipt GitHub run identity mismatch")
    if receipt.get("passed") is not True or receipt.get("ship_evidence_eligible") is not False:
        raise CampaignError("receipt did not pass or incorrectly claims ship eligibility")
    fingerprint = receipt.get("runner_fingerprint")
    if not isinstance(fingerprint, str) or not re.fullmatch(r"[0-9a-f]{64}", fingerprint):
        raise CampaignError("receipt has no valid runner fingerprint")
    known = state["stages"].get("runner_fingerprint")
    if known is not None and known != fingerprint:
        raise CampaignError("runner fingerprint changed inside the campaign")
    state["stages"]["runner_fingerprint"] = fingerprint


def validate_stage_artifacts(
    campaign_dir: Path,
    state: dict[str, Any],
    spec: dict[str, Any],
    run_id: int,
    artifacts: dict[str, Path],
) -> dict[str, Any]:
    step = spec["name"]
    if spec["mode"] == "frozen-candidate":
        diagnostic = artifact_by_prefix(
            artifacts,
            f"performance-0671-frozen-candidate-{state['expected_sha']}-{run_id}",
        )
        validate_host_admission_artifact(campaign_dir, state, diagnostic)
        data = read_unique_member(diagnostic, "frozen-candidate.json")
        receipt = json_receipt(data, "frozen candidate receipt")
        fingerprint = receipt.get("runner_fingerprint")
        known_fingerprint = state["stages"].get("runner_fingerprint")
        if (
            receipt.get("release") != "0.67.1"
            or receipt.get("profile") != "reference-v1"
            or receipt.get("source_commit") != state["expected_sha"]
            or receipt.get("github_run_id") != str(run_id)
            or not isinstance(fingerprint, str)
            or not re.fullmatch(r"[0-9a-f]{64}", fingerprint)
            or (known_fingerprint is not None and fingerprint != known_fingerprint)
            or receipt.get("passed") is not True
            or receipt.get("ship_evidence_eligible") is not True
            or not re.fullmatch(r"[0-9a-f]{64}", str(receipt.get("receipt_sha256", "")))
        ):
            raise CampaignError("frozen-candidate receipt identity/eligibility contract failed")
        state["stages"]["runner_fingerprint"] = fingerprint
        retained = retain_receipt(campaign_dir, "frozen-candidate.json", data)
        aggregate = read_unique_member(diagnostic, "0.67.1.json")
        aggregate_receipt = json_receipt(aggregate, "0.67.1 aggregate release evidence")
        if (
            aggregate_receipt.get("release") != "0.67.1"
            or aggregate_receipt.get("source_commit") != state["expected_sha"]
        ):
            raise CampaignError("frozen-candidate aggregate evidence identity mismatch")
        aggregate_path = retain_receipt(campaign_dir, "release-evidence-0.67.1.json", aggregate)
        return {
            "receipt": str(retained),
            "receipt_sha256": sha256_bytes(data),
            "aggregate": str(aggregate_path),
            "aggregate_sha256": sha256_bytes(aggregate),
        }

    if spec["mode"] == "qualify":
        diagnostic = artifact_by_prefix(artifacts, f"performance-0671-qualification-{state['expected_sha']}-{run_id}")
        validate_host_admission_artifact(campaign_dir, state, diagnostic)
        data = read_unique_member(diagnostic, "qualification.json")
        receipt = json_receipt(data, "qualification receipt")
        expect_common_receipt(receipt, state, run_id)
        if receipt.get("mode") != "qualification-only" or receipt.get("bootstrap_eligible") is not False:
            raise CampaignError("qualification receipt eligibility contract failed")
        retained = retain_receipt(campaign_dir, "qualification.json", data)
        return {"receipt": str(retained), "receipt_sha256": sha256_bytes(data)}

    if spec["mode"] == "full-dress":
        reusable = artifacts.get("performance-0671-full-dress-receipt")
        if reusable is None:
            raise CampaignError("full-dress reusable receipt artifact is missing")
        diagnostic = artifact_by_prefix(artifacts, f"performance-0671-full-dress-{state['expected_sha']}-{run_id}")
        validate_host_admission_artifact(campaign_dir, state, diagnostic)
        data = read_unique_member(reusable, "full-dress-receipt.json")
        receipt = json_receipt(data, "full-dress receipt")
        expect_common_receipt(receipt, state, run_id)
        if (
            receipt.get("mode") != "full-dress-qualification-only"
            or receipt.get("qualification_only") is not True
            or receipt.get("bootstrap_eligible") is not False
        ):
            raise CampaignError("full-dress receipt eligibility contract failed")
        retained = retain_receipt(campaign_dir, f"{step}.json", data)
        result: dict[str, Any] = {"receipt": str(retained), "receipt_sha256": sha256_bytes(data)}
        if step == "full-dress-1":
            if "performance-0671-full-dress-admission" in artifacts:
                raise CampaignError("first full-dress run unexpectedly published admission")
            return result
        admission_archive = artifacts.get("performance-0671-full-dress-admission")
        if admission_archive is None:
            raise CampaignError("second full-dress run did not publish admission")
        admission_data = read_unique_member(admission_archive, "full-dress-admission.json")
        admission = json_receipt(admission_data, "full-dress admission")
        if (
            admission.get("source_commit") != state["expected_sha"]
            or admission.get("runner_fingerprint") != state["stages"]["runner_fingerprint"]
            or admission.get("passed") is not True
            or admission.get("bootstrap_admission_eligible") is not True
            or admission.get("bootstrap_eligible") is not False
            or admission.get("ship_evidence_eligible") is not False
        ):
            raise CampaignError("full-dress admission contract failed")
        members = admission.get("full_dress_runs")
        expected_ids = {str(state["stages"]["full-dress-1"]["run_id"]), str(run_id)}
        if not isinstance(members, list) or {str(member.get("github_run_id")) for member in members} != expected_ids:
            raise CampaignError("full-dress admission member identities are wrong")
        expected_hashes = {
            state["stages"]["full-dress-1"]["receipt_sha256"],
            result["receipt_sha256"],
        }
        if {str(member.get("receipt_sha256")) for member in members} != expected_hashes:
            raise CampaignError("full-dress admission member digests are wrong")
        admission_path = retain_receipt(campaign_dir, "full-dress-admission.json", admission_data)
        result.update(
            {
                "admission": str(admission_path),
                "admission_sha256": sha256_bytes(admission_data),
            }
        )
        return result

    reusable = artifacts.get("performance-0671-bootstrap-receipt")
    if reusable is None:
        raise CampaignError("bootstrap reusable receipt artifact is missing")
    diagnostic = artifact_by_prefix(artifacts, f"performance-0671-bootstrap-{state['expected_sha']}-{run_id}")
    validate_host_admission_artifact(campaign_dir, state, diagnostic)
    data = read_unique_member(reusable, "bootstrap-sample.json")
    receipt = json_receipt(data, "bootstrap sample receipt")
    expect_common_receipt(receipt, state, run_id)
    index = int(spec["sample_index"])
    expected_predecessor = None if index == 1 else spec["bootstrap_predecessor"]
    expected_predecessor_hash = (
        None if index == 1 else state["stages"][f"bootstrap-{index - 1}"]["receipt_sha256"]
    )
    if (
        receipt.get("sample_index") != index
        or receipt.get("bootstrap_eligible") is not True
        or receipt.get("admission_sha256") != state["stages"]["full-dress-2"]["admission_sha256"]
        or receipt.get("predecessor_github_run_id") != expected_predecessor
        or receipt.get("predecessor_receipt_sha256") != expected_predecessor_hash
    ):
        raise CampaignError(f"bootstrap sample {index} chain contract failed")
    retained = retain_receipt(campaign_dir, f"bootstrap-samples/sample-{index}.json", data)
    return {"receipt": str(retained), "receipt_sha256": sha256_bytes(data)}


def view_run(state: dict[str, Any], run_id: int) -> dict[str, Any]:
    value = gh_json(
        [
            "run",
            "view",
            str(run_id),
            "--repo",
            state["repository"],
            "--json",
            "databaseId,displayTitle,headSha,status,conclusion,url,createdAt,updatedAt",
        ]
    )
    if not isinstance(value, dict):
        raise CampaignError("GitHub run view is not an object")
    return value


def check_pre_dispatch(campaign_dir: Path, state: dict[str, Any], step: str) -> None:
    ensure_checkout(state["expected_sha"])
    if github_main_sha(state) != state["expected_sha"]:
        raise CampaignError("origin main no longer equals the qualified campaign SHA")
    ensure_github_runner_contract(state)
    ensure_runner_offline(campaign_dir)
    run_host_action(campaign_dir, state, "check-frozen")
    guard = repo_root() / "scripts/perf/reference-runtime-irq-guard.sh"
    result = run_visible(
        sudo_command(str(guard), f"campaign-pre-{step}"),
        cwd=repo_root(),
        log_path=campaign_dir / "host-lifecycle.log",
    )
    if result != 0:
        raise CampaignError(f"pre-dispatch IRQ guard rejected {step}")


def wait_for_run(campaign_dir: Path, state: dict[str, Any], run_id: int, step: str) -> int:
    disarm_runner_watchdog(campaign_dir, state, step)
    arm_runner_watchdog(campaign_dir, state, step)
    try:
        runner_online(campaign_dir)
        return run_visible(
            [
                "gh",
                "run",
                "watch",
                str(run_id),
                "--repo",
                state["repository"],
                "--interval",
                "30",
                "--exit-status",
            ],
            cwd=repo_root(),
            log_path=campaign_dir / f"{step}.log",
        )
    finally:
        ensure_runner_offline(campaign_dir)
        disarm_runner_watchdog(campaign_dir, state, step)


def execute_stage(campaign_dir: Path, state: dict[str, Any], spec: dict[str, Any]) -> None:
    step = spec["name"]
    stage = state["stages"].setdefault(step, {"status": "pending"})
    if stage.get("status") == "completed":
        return
    if stage.get("status") == "rejected":
        raise CampaignError(f"campaign is permanently stopped at rejected stage {step}")

    run_id_value = stage.get("run_id")
    run_id = int(run_id_value) if isinstance(run_id_value, int) else None

    if run_id is None:
        check_pre_dispatch(campaign_dir, state, step)
        assert_no_foreign_reference_runs(state)
        matches = matching_runs(state, step)
        if stage.get("status") == "dispatching":
            if len(matches) != 1:
                raise CampaignError(f"cannot recover uniquely from interrupted dispatch for {step}")
            run = matches[0]
        else:
            if matches:
                raise CampaignError(f"campaign id/step was already used: {step}")
            stage["status"] = "dispatching"
            stage["dispatch_started_at"] = utc_now()
            save_state(campaign_dir, state)
            append_event(campaign_dir, "stage-dispatch-started", step=step)
            command = [
                "gh",
                "workflow",
                "run",
                state["workflow"],
                "--repo",
                state["repository"],
                "--ref",
                state["branch"],
                *dispatch_fields(state, spec),
            ]
            result = run_visible(command, cwd=repo_root(), log_path=campaign_dir / f"{step}.log")
            if result != 0:
                raise CampaignError(f"workflow dispatch failed for {step}")
            run = discover_run(state, step)
        run_id = int(run["databaseId"])
        stage.update({"run_id": run_id, "run_url": run.get("url"), "status": "queued"})
        save_state(campaign_dir, state)
        append_event(campaign_dir, "stage-correlated", step=step, run_id=run_id)
    run = view_run(state, run_id)
    if run.get("headSha") != state["expected_sha"] or run.get("displayTitle") != expected_title(state, step):
        raise CampaignError(f"persisted GitHub run identity mismatch for {step}")
    assert_no_foreign_reference_runs(state, allowed_run_id=run_id)
    stage["status"] = "running"
    save_state(campaign_dir, state)
    if run.get("status") == "completed":
        ensure_runner_offline(campaign_dir)
        disarm_runner_watchdog(campaign_dir, state, step)
        watch_status = 0 if run.get("conclusion") == "success" else 1
    else:
        watch_status = wait_for_run(campaign_dir, state, run_id, step)
        run = view_run(state, run_id)
    artifacts: dict[str, Path] = {}
    artifact_error: str | None = None
    try:
        artifacts = download_artifacts(campaign_dir, state, run_id, step)
    except Exception as error:  # retain the run failure separately from download failure
        artifact_error = str(error)

    post_error: str | None = None
    try:
        run_host_action(campaign_dir, state, "check-frozen")
        guard = repo_root() / "scripts/perf/reference-runtime-irq-guard.sh"
        if run_visible(
            sudo_command(str(guard), f"campaign-post-{step}"),
            cwd=repo_root(),
            log_path=campaign_dir / "host-lifecycle.log",
        ) != 0:
            raise CampaignError("post-dispatch IRQ guard failed")
    except Exception as error:
        post_error = str(error)

    rejection_reasons: list[str] = []
    if watch_status != 0 or run.get("status") != "completed" or run.get("conclusion") != "success":
        rejection_reasons.append(f"GitHub run conclusion={run.get('conclusion')} watch_status={watch_status}")
    if run.get("headSha") != state["expected_sha"] or run.get("displayTitle") != expected_title(state, step):
        rejection_reasons.append("GitHub run identity mismatch")
    if artifact_error:
        rejection_reasons.append(artifact_error)
    if post_error:
        rejection_reasons.append(post_error)
    if rejection_reasons:
        stage.update(
            {
                "status": "rejected",
                "completed_at": utc_now(),
                "conclusion": run.get("conclusion"),
                "rejection_reasons": rejection_reasons,
            }
        )
        state["phase"] = "rejected"
        save_state(campaign_dir, state)
        append_event(campaign_dir, "stage-rejected", step=step, run_id=run_id, reasons=rejection_reasons)
        raise CampaignError(f"stage {step} rejected: {'; '.join(rejection_reasons)}")

    try:
        retained = validate_stage_artifacts(campaign_dir, state, spec, run_id, artifacts)
    except Exception as error:
        reason = str(error)
        stage.update(
            {
                "status": "rejected",
                "completed_at": utc_now(),
                "conclusion": run.get("conclusion"),
                "rejection_reasons": [reason],
            }
        )
        state["phase"] = "rejected"
        save_state(campaign_dir, state)
        append_event(campaign_dir, "stage-rejected", step=step, run_id=run_id, reasons=[reason])
        raise CampaignError(f"stage {step} artifact validation rejected: {reason}") from error
    stage.update(
        {
            "status": "completed",
            "completed_at": utc_now(),
            "conclusion": "success",
            **retained,
        }
    )
    save_state(campaign_dir, state)
    append_event(campaign_dir, "stage-accepted", step=step, run_id=run_id)
    print(f"STAGE_ACCEPTED={step} RUN_ID={run_id}")


def validate_host_admission_state(campaign_dir: Path, state: dict[str, Any]) -> None:
    admission = state["stages"].get("host_admission")
    if not isinstance(admission, dict) or admission.get("status") != "completed":
        raise CampaignError("host admission is missing")
    for path_field, digest_field in (
        ("host_archive", "host_archive_sha256"),
        ("irq_burn_in_receipt", "irq_burn_in_receipt_sha256"),
        ("host_admission_bundle", "host_admission_bundle_sha256"),
        ("host_admission_receipt", "host_admission_receipt_sha256"),
    ):
        path = Path(admission.get(path_field, ""))
        if not path.is_file() or sha256_file(path) != admission.get(digest_field):
            raise CampaignError(f"host admission artifact drift: {path_field}")
    validate_burn_receipt(Path(admission["irq_burn_in_receipt"]), state)
    canonical = Path("/var/lib/hydracache-perf/reference-campaign-v1")
    if canonical.stat().st_uid != 0 or canonical.stat().st_mode & 0o777 != 0o555:
        raise CampaignError("canonical host admission directory owner/mode drift")
    for name, digest_field in (
        ("reference-campaign-host-admission.tar.gz", "host_admission_bundle_sha256"),
        ("reference-campaign-admission.json", "host_admission_receipt_sha256"),
    ):
        path = canonical / name
        if (
            not path.is_file()
            or path.is_symlink()
            or path.stat().st_uid != 0
            or path.stat().st_mode & 0o777 != 0o444
            or sha256_file(path) != admission.get(digest_field)
        ):
            raise CampaignError(f"canonical host admission drift: {name}")


def cargo_sample_set(campaign_dir: Path) -> Path:
    output = repo_root() / "target/test-evidence/0.67.1/bootstrap-sample-set.json"
    if output.exists():
        raise CampaignError("sample-set output already exists before final validation")
    cargo = Path("/home/github-runner/.cargo/bin/cargo")
    if not cargo.is_file():
        found = shutil.which("cargo")
        if found is None:
            raise CampaignError("cargo is unavailable for final sample-set validation")
        command = [found]
    else:
        command = runner_command(str(cargo))
    command.extend(
        [
            "run",
            "-p",
            "xtask",
            "--locked",
            "--offline",
            "--",
            "perf-bootstrap",
            "--release",
            "0.67.1",
            "--profile",
            "reference-v1",
            "--phase",
            "sample-set",
            "--samples-dir",
            str(campaign_dir / "accepted-receipts/bootstrap-samples"),
        ]
    )
    result = run_visible(command, cwd=repo_root(), log_path=campaign_dir / "sample-set-validation.log")
    if result != 0 or not output.is_file():
        raise CampaignError("Rust sample-set validator rejected the five-sample chain")
    data = output.read_bytes()
    return retain_receipt(campaign_dir, "bootstrap-sample-set.json", data)


def prepare_reference_inputs(campaign_dir: Path, state: dict[str, Any]) -> Path:
    sample_set = Path(state["stages"].get("sample_set", ""))
    expected_sample_set_sha = state["stages"].get("sample_set_sha256")
    if (
        not sample_set.is_file()
        or not isinstance(expected_sample_set_sha, str)
        or sha256_file(sample_set) != expected_sample_set_sha
    ):
        raise CampaignError("accepted bootstrap sample set is absent or changed")
    inputs = campaign_dir / "reference-inputs"
    inputs.mkdir(mode=0o700, exist_ok=True)
    samples: list[dict[str, Any]] = []
    for index in range(1, 6):
        step = f"bootstrap-{index}"
        stage = state["stages"].get(step, {})
        run_id = stage.get("run_id")
        receipt_path = Path(stage.get("receipt", ""))
        if (
            stage.get("status") != "completed"
            or not isinstance(run_id, int)
            or not receipt_path.is_file()
            or sha256_file(receipt_path) != stage.get("receipt_sha256")
        ):
            raise CampaignError(f"accepted {step} receipt is absent or changed")
        artifacts = download_artifacts(campaign_dir, state, run_id, step)
        diagnostic = artifact_by_prefix(
            artifacts,
            f"performance-0671-bootstrap-{state['expected_sha']}-{run_id}",
        )
        sample_dir = materialize_bootstrap_input(
            campaign_dir,
            index,
            receipt_path.read_bytes(),
            diagnostic,
        )
        samples.append(
            {
                "sample_index": index,
                "github_run_id": run_id,
                "receipt_sha256": stage["receipt_sha256"],
                "directory": str(sample_dir),
                "diagnostic_archive_sha256": sha256_file(diagnostic),
            }
        )
    sample_set_copy = inputs / "bootstrap-sample-set.json"
    if sample_set_copy.exists():
        if sample_set_copy.read_bytes() != sample_set.read_bytes():
            raise CampaignError("materialized bootstrap sample set changed")
    else:
        with sample_set_copy.open("xb") as stream:
            stream.write(sample_set.read_bytes())
        os.chmod(sample_set_copy, 0o444)
    manifest_path = inputs / "reference-inputs.json"
    manifest = {
        "schema_version": 1,
        "release": "0.67.1",
        "profile": "reference-v1",
        "source_commit": state["expected_sha"],
        "runner_fingerprint": state["stages"].get("runner_fingerprint"),
        "sample_set_sha256": expected_sample_set_sha,
        "samples": samples,
    }
    if manifest_path.exists():
        if json.loads(manifest_path.read_text(encoding="utf-8")) != manifest:
            raise CampaignError("materialized reference-input manifest changed")
    else:
        write_json_atomic(manifest_path, manifest, create=True)
    return manifest_path


def write_summary(campaign_dir: Path, state: dict[str, Any]) -> None:
    rows: list[str] = []
    names = ["qualification", "full-dress-1", "full-dress-2", *[f"bootstrap-{i}" for i in range(1, 6)]]
    if "frozen-candidate" in state["stages"]:
        names.append("frozen-candidate")
    for name in names:
        stage = state["stages"].get(name, {})
        rows.append(
            f"| {name} | {stage.get('status', 'not-run')} | {stage.get('run_id', '')} | "
            f"{stage.get('receipt_sha256', '')} |"
        )
    host = state["stages"].get("host_admission", {})
    summary = {
        "schema_version": 1,
        "campaign_id": state["campaign_id"],
        "repository": state["repository"],
        "source_commit": state["expected_sha"],
        "phase": state["phase"],
        "runner_fingerprint": state["stages"].get("runner_fingerprint"),
        "host_archive_sha256": host.get("host_archive_sha256"),
        "irq_burn_in_receipt_sha256": host.get("irq_burn_in_receipt_sha256"),
        "sample_set_sha256": state["stages"].get("sample_set_sha256"),
        "generated_at": utc_now(),
        "stages": {
            key: value
            for key, value in state["stages"].items()
            if key not in {"host_admission", "runner_fingerprint"}
        },
    }
    write_json_atomic(campaign_dir / "campaign-summary.json", summary)
    markdown = "\n".join(
        [
            "# HydraCache 0.67.1 reference campaign",
            "",
            f"- Campaign: `{state['campaign_id']}`",
            f"- Source: `{state['expected_sha']}`",
            f"- State: `{state['phase']}`",
            f"- Runner fingerprint: `{state['stages'].get('runner_fingerprint', 'unavailable')}`",
            f"- Host archive SHA-256: `{host.get('host_archive_sha256', 'unavailable')}`",
            f"- IRQ burn-in SHA-256: `{host.get('irq_burn_in_receipt_sha256', 'unavailable')}`",
            f"- Sample-set SHA-256: `{state['stages'].get('sample_set_sha256', 'unavailable')}`",
            "",
            "| Stage | Status | GitHub run | Receipt SHA-256 |",
            "|---|---|---:|---|",
            *rows,
            "",
            "Original GitHub artifact ZIP files are retained under `runs/*/original-artifacts/`;",
            "their byte sizes and SHA-256 digests are recorded beside each run.",
            "The exact W5 inputs are materialized without changing those ZIPs under",
            "`reference-inputs/sample-{1..5}` and sealed by `reference-inputs/reference-inputs.json`.",
            "",
            "Bootstrap campaigns remain non-ship inputs; only the separate post-activation",
            "frozen-candidate campaign may produce ship-eligible evidence.",
            "",
        ]
    )
    (campaign_dir / "campaign-summary.md").write_text(markdown, encoding="utf-8", newline="\n")


def command_run(args: argparse.Namespace) -> None:
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    state = load_state(campaign_dir)
    if state["phase"] not in {"ready", "running"}:
        raise CampaignError(f"run requires ready/running state, found {state['phase']}")
    require_tools(["gh", "git", "sudo", "systemctl"])
    require_github_dispatch_readiness()
    ensure_checkout(state["expected_sha"])
    validate_host_admission_state(campaign_dir, state)
    pin_controller_to_housekeeping(state)
    state["phase"] = "running"
    save_state(campaign_dir, state)
    while True:
        specs = stage_specs(state)
        pending = next(
            (spec for spec in specs if state["stages"].get(spec["name"], {}).get("status") != "completed"),
            None,
        )
        if pending is None:
            if state["stages"].get("bootstrap-5", {}).get("status") == "completed":
                break
            continue
        execute_stage(campaign_dir, state, pending)
    ensure_runner_offline(campaign_dir)
    run_host_action(campaign_dir, state, "check-frozen")
    sample_set = cargo_sample_set(campaign_dir)
    state["stages"]["sample_set"] = str(sample_set)
    state["stages"]["sample_set_sha256"] = sha256_file(sample_set)
    reference_inputs = prepare_reference_inputs(campaign_dir, state)
    state["stages"]["reference_inputs"] = str(reference_inputs)
    state["stages"]["reference_inputs_sha256"] = sha256_file(reference_inputs)
    state["phase"] = "complete"
    save_state(campaign_dir, state)
    append_event(campaign_dir, "campaign-completed")
    write_summary(campaign_dir, state)
    print(f"CAMPAIGN_COMPLETE=true campaign_dir={campaign_dir}")


def command_prepare_review(args: argparse.Namespace) -> None:
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    state = load_state(campaign_dir)
    if state["phase"] not in {"complete", "closed"}:
        raise CampaignError(f"prepare-review requires complete/closed state, found {state['phase']}")
    require_tools(["gh", "git"])
    ensure_checkout(state["expected_sha"])
    manifest = prepare_reference_inputs(campaign_dir, state)
    state["stages"]["reference_inputs"] = str(manifest)
    state["stages"]["reference_inputs_sha256"] = sha256_file(manifest)
    save_state(campaign_dir, state)
    write_summary(campaign_dir, state)
    print(f"REFERENCE_INPUTS_READY={manifest}")
    print(
        "NEXT=perf-reference --phase propose --sample-set "
        f"{campaign_dir / 'reference-inputs/bootstrap-sample-set.json'} "
        f"--samples-dir {campaign_dir / 'reference-inputs'}"
    )


def command_frozen(args: argparse.Namespace) -> None:
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    state = load_state(campaign_dir)
    if state["phase"] not in {"ready", "running"}:
        raise CampaignError(f"frozen run requires a fresh ready/running campaign, found {state['phase']}")
    require_tools(["gh", "git", "sudo", "systemctl"])
    run_capture(["gh", "auth", "status"], cwd=repo_root())
    ensure_checkout(state["expected_sha"])
    validate_host_admission_state(campaign_dir, state)
    pin_controller_to_housekeeping(state)
    state["phase"] = "running"
    save_state(campaign_dir, state)
    execute_stage(
        campaign_dir,
        state,
        {"name": "frozen-candidate", "mode": "frozen-candidate"},
    )
    ensure_runner_offline(campaign_dir)
    run_host_action(campaign_dir, state, "check-frozen")
    state["phase"] = "complete"
    save_state(campaign_dir, state)
    append_event(campaign_dir, "frozen-candidate-completed")
    write_summary(campaign_dir, state)
    print(f"FROZEN_CANDIDATE_COMPLETE=true campaign_dir={campaign_dir}")


def command_status(args: argparse.Namespace) -> None:
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    state = load_state(campaign_dir)
    print(json.dumps(state, indent=2, sort_keys=True))


def command_close(args: argparse.Namespace) -> None:
    campaign_dir = ensure_external_campaign_dir(Path(args.campaign_dir))
    state = load_state(campaign_dir)
    require_tools(["gh", "git", "sudo", "systemctl"])
    pin_controller_to_housekeeping(state)
    ensure_runner_offline(campaign_dir)
    assert_no_foreign_reference_runs(state)
    if state["phase"] == "complete":
        run_host_action(campaign_dir, state, "check-frozen")
    archive = archive_host_state(campaign_dir, state, "host-state-final.tar.gz")
    state["stages"]["final_host_archive"] = str(archive)
    state["stages"]["final_host_archive_sha256"] = sha256_file(archive)
    retired = retire_canonical_host_admission(campaign_dir, state)
    if retired is not None:
        state["stages"]["retired_canonical_host_admission"] = str(retired)
    state["phase"] = "closed"
    save_state(campaign_dir, state)
    append_event(campaign_dir, "campaign-closed")
    write_summary(campaign_dir, state)
    print("SAFE_TO_DELETE_SERVER=true")
    print("Provider deletion, runner credential revocation, and billing confirmation remain explicit.")


def parser() -> argparse.ArgumentParser:
    root = argparse.ArgumentParser(description=__doc__)
    subparsers = root.add_subparsers(dest="command", required=True)

    prepare = subparsers.add_parser("prepare", help="apply reviewed host policy and stop at reboot")
    prepare.add_argument("--campaign-id", required=True)
    prepare.add_argument("--campaign-dir", required=True)
    prepare.add_argument("--host-state-dir", required=True)
    prepare.add_argument("--expected-sha", required=True)
    prepare.add_argument("--confirm-host-mutation", action="store_true")
    prepare.set_defaults(handler=command_prepare)

    freeze = subparsers.add_parser("freeze", help="verify reboot, freeze host, and run IRQ burn-in")
    freeze.add_argument("--campaign-dir", required=True)
    freeze.add_argument("--duration-seconds", type=int, default=900)
    freeze.add_argument("--read-mebibytes", type=int, default=256)
    freeze.add_argument("--network-target")
    freeze.set_defaults(handler=command_freeze)

    run = subparsers.add_parser("run", help="execute qualification, full-dress, and bootstrap chain")
    run.add_argument("--campaign-dir", required=True)
    run.set_defaults(handler=command_run)

    prepare_review = subparsers.add_parser(
        "prepare-review",
        help="materialize digest-verified W5 inputs from retained immutable artifact ZIPs",
    )
    prepare_review.add_argument("--campaign-dir", required=True)
    prepare_review.set_defaults(handler=command_prepare_review)

    frozen = subparsers.add_parser(
        "run-frozen",
        help="execute the separate post-activation frozen-candidate campaign",
    )
    frozen.add_argument("--campaign-dir", required=True)
    frozen.set_defaults(handler=command_frozen)

    status = subparsers.add_parser("status", help="show the durable campaign state")
    status.add_argument("--campaign-dir", required=True)
    status.set_defaults(handler=command_status)

    close = subparsers.add_parser("close", help="stop local services and produce the deletion handoff")
    close.add_argument("--campaign-dir", required=True)
    close.set_defaults(handler=command_close)
    return root


def main() -> int:
    args = parser().parse_args()
    try:
        args.handler(args)
    except (CampaignError, OSError, subprocess.SubprocessError) as error:
        print(f"reference campaign rejected: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
