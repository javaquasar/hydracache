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
import threading
import time
from contextlib import contextmanager
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
ARTIFACT_DOWNLOAD_ATTEMPTS = 12
ARTIFACT_DOWNLOAD_RETRY_DELAY_SECONDS = 15
ARTIFACT_DOWNLOAD_MAX_RETRY_DELAY_SECONDS = 60
ARTIFACT_DOWNLOAD_TIMEOUT_SECONDS = 600
ARTIFACT_CANARY_ATTEMPTS = 3
ARTIFACT_CANARY_RETRY_DELAY_SECONDS = 15
ARTIFACT_CANARY_TIMEOUT_SECONDS = 120
ARTIFACT_CANARY_MAX_BYTES = 64 * 1024 * 1024
ARTIFACT_ARCHIVE_MAX_BYTES = 1024 * 1024 * 1024
GITHUB_CONTROL_TIMEOUT_SECONDS = 120
SAMPLE_SET_VALIDATION_TIMEOUT_SECONDS = 600
FROZEN_ACTIVATION_PATH = "target/test-evidence/0.67.1/reference-activation.json"
FROZEN_BUDGET_VERDICT_PATH = "target/test-evidence/0.67.1/perf-budget-verdict.json"
EXPECTED_0671_WORK_ITEMS = tuple(f"W{index}" for index in range(8))


class CampaignError(RuntimeError):
    """An admission or orchestration invariant failed."""


class ArtifactTransportError(CampaignError):
    """Artifact bytes are temporarily unavailable without invalidating a run."""


class ArtifactIntegrityError(CampaignError):
    """Artifact identity or bytes are permanently unsuitable as evidence."""


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
    args: Iterable[str],
    *,
    cwd: Path | None = None,
    check: bool = True,
    timeout_seconds: int | None = None,
) -> str:
    try:
        completed = subprocess.run(
            list(args),
            cwd=cwd,
            check=False,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            timeout=timeout_seconds,
        )
    except subprocess.TimeoutExpired as error:
        raise CampaignError(f"command timed out after {timeout_seconds} seconds") from error
    if check and completed.returncode != 0:
        detail = completed.stderr.strip() or completed.stdout.strip()
        raise CampaignError(f"command failed ({completed.returncode}): {detail}")
    return completed.stdout.strip()


def run_visible(
    args: Iterable[str],
    *,
    cwd: Path,
    log_path: Path,
    timeout_seconds: int | float | None = None,
) -> int:
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
        reader_error: list[BaseException] = []

        def stream_output() -> None:
            try:
                for line in process.stdout:
                    sys.stdout.write(line)
                    sys.stdout.flush()
                    log.write(line)
                    log.flush()
            except BaseException as error:
                reader_error.append(error)

        reader = threading.Thread(target=stream_output, name="hydracache-command-output")
        reader.start()
        try:
            try:
                result = process.wait(timeout=timeout_seconds)
            except subprocess.TimeoutExpired as error:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
                raise CampaignError(
                    f"command timed out after {timeout_seconds} seconds"
                ) from error
            reader.join()
            if reader_error:
                raise reader_error[0]
            return result
        except BaseException:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            reader.join(timeout=10)
            raise
        finally:
            # Popen is not used as a context manager because output is streamed
            # concurrently. Close its pipe explicitly on both success and
            # timeout so repeated controller invocations do not leak handles.
            reader.join(timeout=10)
            process.stdout.close()


def require_tools(names: Iterable[str]) -> None:
    missing = [name for name in names if shutil.which(name) is None]
    if missing:
        raise CampaignError(f"missing required tools: {', '.join(missing)}")


def require_github_dispatch_readiness() -> None:
    """Fail before freeze if installing/authenticating gh would drift the host later."""
    require_tools(["gh"])
    run_capture(
        ["gh", "auth", "status"],
        cwd=repo_root(),
        timeout_seconds=GITHUB_CONTROL_TIMEOUT_SECONDS,
    )


def sudo_prefix() -> list[str]:
    return [] if hasattr(os, "geteuid") and os.geteuid() == 0 else ["sudo", "-n"]


def sudo_command(*args: str) -> list[str]:
    return [*sudo_prefix(), *args]


@contextmanager
def sudo_lease() -> Iterable[None]:
    """Keep one explicitly authenticated sudo timestamp alive for a command.

    Reference runs can outlive sudo's default timestamp timeout. Every actual
    privileged operation remains non-interactive so an expired credential
    fails closed instead of blocking a detached campaign indefinitely.
    """
    if not sudo_prefix():
        yield
        return
    try:
        subprocess.run(["sudo", "-v"], check=True)
    except subprocess.CalledProcessError as error:
        raise CampaignError("sudo authentication failed before campaign action") from error

    stop = threading.Event()

    def refresh() -> None:
        while not stop.wait(60):
            if subprocess.run(
                ["sudo", "-n", "-v"],
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            ).returncode != 0:
                return

    keeper = threading.Thread(target=refresh, name="hydracache-sudo-lease", daemon=True)
    keeper.start()
    try:
        yield
    finally:
        stop.set()
        keeper.join(timeout=2)


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


def require_canonical_host_admission_absent(
    canonical: Path = Path("/var/lib/hydracache-perf/reference-campaign-v1"),
) -> None:
    if canonical.exists():
        raise CampaignError(
            "prepare requires the previous campaign to close and retire its canonical host "
            f"admission first: {canonical}"
        )


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
    require_canonical_host_admission_absent()
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


def gh_json(
    args: Iterable[str], *, timeout_seconds: int | None = GITHUB_CONTROL_TIMEOUT_SECONDS
) -> Any:
    output = run_capture(
        ["gh", *args], cwd=repo_root(), timeout_seconds=timeout_seconds
    )
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
        timeout_seconds=GITHUB_CONTROL_TIMEOUT_SECONDS,
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


def download_binary(
    args: list[str],
    output: Path,
    *,
    timeout_seconds: int = ARTIFACT_DOWNLOAD_TIMEOUT_SECONDS,
) -> None:
    if timeout_seconds <= 0:
        raise CampaignError("artifact download timeout must be positive")
    partial = output.with_name(f".{output.name}.partial")
    if partial.exists():
        partial.unlink()
    try:
        with partial.open("xb") as stream:
            completed = subprocess.run(
                args,
                cwd=repo_root(),
                stdout=stream,
                stderr=subprocess.PIPE,
                check=False,
                timeout=timeout_seconds,
            )
    except subprocess.TimeoutExpired as error:
        partial.unlink(missing_ok=True)
        raise ArtifactTransportError(
            f"artifact download timed out after {timeout_seconds} seconds"
        ) from error
    if completed.returncode != 0:
        partial.unlink(missing_ok=True)
        detail = completed.stderr.decode("utf-8", errors="replace").strip()
        raise ArtifactTransportError(f"artifact download failed: {detail}")
    os.replace(partial, output)
    os.chmod(output, 0o444)


def safe_artifact_filename(artifact_id: int, name: str) -> str:
    slug = re.sub(r"[^A-Za-z0-9._-]+", "-", name).strip("-.")
    if not slug:
        raise CampaignError("artifact name cannot be represented safely")
    return f"{artifact_id}-{slug}.zip"


def expected_stage_artifact_names(
    state: dict[str, Any], run_id: int, step: str
) -> set[str]:
    source = state["expected_sha"]
    if step == "qualification":
        return {f"performance-0671-qualification-{source}-{run_id}"}
    if step == "full-dress-1":
        return {
            "performance-0671-full-dress-receipt",
            f"performance-0671-full-dress-{source}-{run_id}",
        }
    if step == "full-dress-2":
        return {
            "performance-0671-full-dress-receipt",
            "performance-0671-full-dress-admission",
            f"performance-0671-full-dress-{source}-{run_id}",
        }
    if re.fullmatch(r"bootstrap-[1-5]", step):
        return {
            "performance-0671-bootstrap-receipt",
            f"performance-0671-bootstrap-{source}-{run_id}",
        }
    if step == "frozen-candidate":
        return {f"performance-0671-frozen-candidate-{source}-{run_id}"}
    raise CampaignError(f"unknown reference campaign stage: {step}")


def download_artifacts(campaign_dir: Path, state: dict[str, Any], run_id: int, step: str) -> dict[str, Path]:
    run_dir = campaign_dir / "runs" / f"{step}-{run_id}"
    manifest_path = run_dir / "artifact-manifest.json"
    if run_dir.exists():
        try:
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise ArtifactIntegrityError(
                f"incomplete prior artifact download for {step}: {error}"
            ) from error
        if (
            not isinstance(manifest, dict)
            or set(manifest) != {"schema_version", "run_id", "step", "artifacts"}
            or manifest.get("schema_version") != 1
            or manifest.get("run_id") != run_id
            or manifest.get("step") != step
            or not isinstance(manifest.get("artifacts"), list)
            or not manifest["artifacts"]
        ):
            raise ArtifactIntegrityError(f"retained artifact manifest identity mismatch for {step}")
        paths: dict[str, Path] = {}
        for artifact in manifest["artifacts"]:
            if not isinstance(artifact, dict) or set(artifact) != {
                "artifact_id",
                "name",
                "reported_size_bytes",
                "archive_file",
                "archive_size_bytes",
                "archive_sha256",
            }:
                raise ArtifactIntegrityError(f"malformed retained artifact entry for {step}")
            artifact_id = artifact.get("artifact_id")
            name = artifact.get("name")
            reported_size = artifact.get("reported_size_bytes")
            archive_size = artifact.get("archive_size_bytes")
            archive_sha256 = artifact.get("archive_sha256")
            if (
                not isinstance(artifact_id, int)
                or not isinstance(name, str)
                or not isinstance(reported_size, int)
                or reported_size <= 0
                or reported_size > ARTIFACT_ARCHIVE_MAX_BYTES
                or not isinstance(archive_size, int)
                or archive_size <= 0
                or archive_size > ARTIFACT_ARCHIVE_MAX_BYTES
                or not isinstance(archive_sha256, str)
                or not re.fullmatch(r"[0-9a-f]{64}", archive_sha256)
            ):
                raise ArtifactIntegrityError(f"invalid retained artifact identity for {step}")
            expected_relative = (
                Path("runs")
                / f"{step}-{run_id}"
                / "original-artifacts"
                / safe_artifact_filename(artifact_id, name)
            )
            if Path(artifact.get("archive_file", "")) != expected_relative:
                raise ArtifactIntegrityError(f"retained artifact path escaped its run directory for {step}")
            path = campaign_dir / expected_relative
            if (
                not path.is_file()
                or path.is_symlink()
                or path.stat().st_size != archive_size
                or sha256_file(path) != archive_sha256
                or name in paths
            ):
                raise ArtifactIntegrityError(f"retained artifact drift for {step}")
            paths[name] = path
        if set(paths) != expected_stage_artifact_names(state, run_id, step):
            raise ArtifactIntegrityError(f"retained artifact set is not exact for {step}")
        return paths

    runs_dir = campaign_dir / "runs"
    runs_dir.mkdir(mode=0o700, exist_ok=True)
    staging = runs_dir / f".{step}-{run_id}.partial"
    if staging.exists():
        shutil.rmtree(staging)
    originals = staging / "original-artifacts"
    originals.mkdir(parents=True, mode=0o700)
    try:
        response = gh_json(
            [
                "api",
                f"repos/{state['repository']}/actions/runs/{run_id}/artifacts?per_page=100",
            ],
            timeout_seconds=ARTIFACT_DOWNLOAD_TIMEOUT_SECONDS,
        )
    except CampaignError as error:
        raise ArtifactTransportError(f"artifact listing failed for run {run_id}: {error}") from error
    artifacts = response.get("artifacts") if isinstance(response, dict) else None
    total_count = response.get("total_count") if isinstance(response, dict) else None
    if not isinstance(artifacts, list) or not artifacts:
        raise ArtifactTransportError(f"run {run_id} has no downloadable artifacts yet")
    if not isinstance(total_count, int) or total_count != len(artifacts):
        raise ArtifactIntegrityError(f"artifact listing is incomplete for {step}")
    listed_names = [artifact.get("name") for artifact in artifacts if isinstance(artifact, dict)]
    if (
        len(listed_names) != len(artifacts)
        or not all(isinstance(name, str) for name in listed_names)
        or set(listed_names) != expected_stage_artifact_names(state, run_id, step)
    ):
        raise ArtifactIntegrityError(f"artifact set is not exact for {step}")
    names: set[str] = set()
    paths: dict[str, Path] = {}
    manifest: list[dict[str, Any]] = []
    for artifact in artifacts:
        artifact_id = artifact.get("id")
        name = artifact.get("name")
        if not isinstance(artifact_id, int) or not isinstance(name, str) or name in names:
            raise ArtifactIntegrityError("artifact API returned invalid or duplicate identity")
        if artifact.get("expired") is True:
            raise ArtifactIntegrityError(f"artifact already expired: {name}")
        reported_size = artifact.get("size_in_bytes")
        if (
            not isinstance(reported_size, int)
            or reported_size <= 0
            or reported_size > ARTIFACT_ARCHIVE_MAX_BYTES
        ):
            raise ArtifactIntegrityError(f"artifact size is invalid or exceeds the cap: {name}")
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
            raise ArtifactIntegrityError(f"artifact is not a valid ZIP: {name}") from error
        if corrupt is not None:
            raise ArtifactIntegrityError(f"artifact ZIP has a corrupt member: {name}:{corrupt}")
        if output.stat().st_size <= 0 or output.stat().st_size > ARTIFACT_ARCHIVE_MAX_BYTES:
            raise ArtifactIntegrityError(f"downloaded artifact size is invalid: {name}")
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


def download_artifacts_with_retry(
    campaign_dir: Path,
    state: dict[str, Any],
    run_id: int,
    step: str,
    attempts: int = ARTIFACT_DOWNLOAD_ATTEMPTS,
    initial_delay_seconds: int = ARTIFACT_DOWNLOAD_RETRY_DELAY_SECONDS,
    max_delay_seconds: int = ARTIFACT_DOWNLOAD_MAX_RETRY_DELAY_SECONDS,
) -> dict[str, Path]:
    if attempts <= 0:
        raise CampaignError("artifact download retry count must be positive")
    if initial_delay_seconds <= 0 or max_delay_seconds < initial_delay_seconds:
        raise CampaignError("artifact download retry delays are invalid")
    latest: ArtifactTransportError | None = None
    delay_seconds = initial_delay_seconds
    for attempt in range(1, attempts + 1):
        try:
            return download_artifacts(campaign_dir, state, run_id, step)
        except ArtifactTransportError as error:
            latest = error
            if attempt == attempts:
                break
            append_event(
                campaign_dir,
                "artifact-download-retry",
                step=step,
                run_id=run_id,
                attempt=attempt,
                retry_in_seconds=delay_seconds,
                detail=str(error),
            )
            time.sleep(delay_seconds)
            delay_seconds = min(delay_seconds * 2, max_delay_seconds)
    raise ArtifactTransportError(
        f"artifact download failed after {attempts} attempts: {latest}"
    ) from latest


def artifact_named(artifacts: dict[str, Path], name: str) -> Path:
    artifact = artifacts.get(name)
    if artifact is None:
        raise CampaignError(f"expected exact artifact {name}")
    return artifact


def read_unique_member(archive_path: Path, basename: str) -> bytes:
    with zipfile.ZipFile(archive_path) as archive:
        matches = [
            info
            for info in archive.infolist()
            if Path(info.filename).name == basename and not info.is_dir()
        ]
        if len(matches) != 1:
            raise CampaignError(f"expected one {basename} in {archive_path.name}, found {len(matches)}")
        info = matches[0]
        unix_mode = info.external_attr >> 16
        if stat.S_ISLNK(unix_mode) or info.flag_bits & 0x1 or info.file_size > 64 * 1024 * 1024:
            raise CampaignError(f"receipt is unsafe or unexpectedly large: {basename}")
        return archive.read(info)


def read_bounded_evidence_member(archive_path: Path, relative: str) -> bytes:
    """Read one exact bounded evidence member without trusting ZIP paths."""
    if (
        not relative.startswith("target/test-evidence/0.67/")
        or "\\" in relative
        or relative.startswith("/")
        or any(part in {"", ".", ".."} for part in relative.split("/"))
    ):
        raise CampaignError(f"unsafe bootstrap evidence identity: {relative}")
    archive_relatives = {relative, relative.removeprefix("target/")}
    with zipfile.ZipFile(archive_path) as archive:
        matches: list[zipfile.ZipInfo] = []
        for info in archive.infolist():
            name = info.filename.removeprefix("./")
            if info.is_dir() or name.startswith("/") or "\\" in name or ".." in name.split("/"):
                continue
            if any(
                name == archive_relative or name.endswith(f"/{archive_relative}")
                for archive_relative in archive_relatives
            ):
                matches.append(info)
        if len(matches) != 1:
            raise CampaignError(
                f"expected one exact {relative} in {archive_path.name}, found {len(matches)}"
            )
        info = matches[0]
        unix_mode = info.external_attr >> 16
        if stat.S_ISLNK(unix_mode) or info.flag_bits & 0x1 or info.file_size > 64 * 1024 * 1024:
            raise CampaignError(f"unsafe or oversized ZIP evidence member: {relative}")
        return archive.read(info)


def read_bounded_frozen_member(archive_path: Path, relative: str) -> bytes:
    """Read one receipt-bound final artifact from the immutable workflow ZIP."""
    allowed_prefixes = (
        "target/test-evidence/0.67/",
        "target/test-evidence/0.67.1/",
        "target/release-evidence/canaries/",
    )
    if (
        not relative.startswith(allowed_prefixes)
        or "\\" in relative
        or relative.startswith("/")
        or any(part in {"", ".", ".."} for part in relative.split("/"))
    ):
        raise CampaignError(f"unsafe frozen evidence identity: {relative}")
    archive_relatives = {relative, relative.removeprefix("target/")}
    with zipfile.ZipFile(archive_path) as archive:
        matches: list[zipfile.ZipInfo] = []
        for info in archive.infolist():
            name = info.filename.removeprefix("./")
            if info.is_dir() or name.startswith("/") or "\\" in name or ".." in name.split("/"):
                continue
            if any(
                name == archive_relative or name.endswith(f"/{archive_relative}")
                for archive_relative in archive_relatives
            ):
                matches.append(info)
        if len(matches) != 1:
            raise CampaignError(
                f"expected one exact {relative} in {archive_path.name}, found {len(matches)}"
            )
        info = matches[0]
        unix_mode = info.external_attr >> 16
        if stat.S_ISLNK(unix_mode) or info.flag_bits & 0x1 or info.file_size > 64 * 1024 * 1024:
            raise CampaignError(f"unsafe or oversized ZIP evidence member: {relative}")
        return archive.read(info)


def read_evidence_member(archive_path: Path, relative: str, expected_sha256: str) -> bytes:
    if not re.fullmatch(r"[0-9a-f]{64}", expected_sha256):
        raise CampaignError(f"unsafe bootstrap evidence identity: {relative}")
    data = read_bounded_evidence_member(archive_path, relative)
    if sha256_bytes(data) != expected_sha256:
        raise CampaignError(f"bootstrap evidence digest mismatch: {relative}")
    return data


def validate_materialized_macro_support(
    receipt: dict[str, Any],
    payloads: dict[str, bytes],
    digests: dict[str, str],
) -> None:
    marker_relative = "target/test-evidence/0.67/w7-raw/macro-publication-receipt.json"
    prebuild_relative = "target/test-evidence/0.67/prebuild-manifest.json"
    marker_data = payloads.get(marker_relative)
    if marker_data is None:
        raise CampaignError("bootstrap receipt does not bind the W7 publication marker")
    marker = json_receipt(marker_data, "W7 macro publication receipt")
    artifacts = marker.get("artifacts")
    prebuild_sha256 = marker.get("prebuild_manifest_sha256")
    if (
        marker.get("schema_version") != 1
        or marker.get("source_commit") != receipt.get("source_commit")
        or marker.get("runner_profile") != "reference-v1"
        or marker.get("runner_fingerprint") != receipt.get("runner_fingerprint")
        or not isinstance(prebuild_sha256, str)
        or not isinstance(artifacts, list)
        or not artifacts
        or len(artifacts) > 64
    ):
        raise CampaignError("W7 macro publication identity is invalid")

    prebuild_data = payloads.get(prebuild_relative)
    if prebuild_data is None or digests.get(prebuild_relative) != prebuild_sha256:
        raise CampaignError("bootstrap receipt does not bind the exact W7 prebuild manifest")

    raw_paths: set[str] = set()
    for artifact in artifacts:
        if not isinstance(artifact, dict):
            raise CampaignError("W7 macro publication artifact is malformed")
        canonical = artifact.get("canonical_path")
        envelope_sha256 = artifact.get("envelope_sha256")
        raw_relative = artifact.get("raw_sidecar_path")
        raw_sha256 = artifact.get("raw_sha256")
        if (
            not isinstance(canonical, str)
            or canonical not in payloads
            or digests.get(canonical) != envelope_sha256
            or not isinstance(raw_relative, str)
            or not raw_relative.startswith("target/test-evidence/0.67/w7-raw/")
            or raw_relative in raw_paths
            or not isinstance(raw_sha256, str)
        ):
            raise CampaignError("W7 macro publication artifact identity is invalid")
        raw_paths.add(raw_relative)
        if raw_relative not in payloads or digests.get(raw_relative) != raw_sha256:
            raise CampaignError("bootstrap receipt does not bind an exact W7 raw sidecar")

    if digests.get(marker_relative) != sha256_bytes(marker_data):
        raise CampaignError("bootstrap receipt W7 marker digest mismatch")


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

    validate_materialized_macro_support(receipt, payloads, digests)
    total_bytes = sum(len(data) for data in payloads.values())
    if total_bytes > 512 * 1024 * 1024:
        raise CampaignError(f"bootstrap sample {index} evidence exceeds 512 MiB")

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


def sealed_json_receipt_is_valid(receipt: dict[str, Any]) -> bool:
    claimed = receipt.get("receipt_sha256")
    if not isinstance(claimed, str) or not re.fullmatch(r"[0-9a-f]{64}", claimed):
        return False
    payload = dict(receipt)
    payload["receipt_sha256"] = ""
    # Rust's serde_json compact serializer preserves struct field order and
    # emits UTF-8 directly. json.loads preserves that order for the retained
    # receipt, allowing the controller to recompute the same seal.
    canonical = json.dumps(
        payload, ensure_ascii=False, separators=(",", ":")
    ).encode("utf-8")
    return sha256_bytes(canonical) == claimed


def validate_frozen_receipt_artifacts(
    receipt: dict[str, Any], diagnostic: Path
) -> None:
    expected_fields = {
        "schema_version",
        "release",
        "profile",
        "source_commit",
        "github_run_id",
        "runner_fingerprint",
        "activation_sha256",
        "budget_verdict_sha256",
        "reference_evidence_sha256",
        "canary_receipt_sha256",
        "passed",
        "ship_evidence_eligible",
        "receipt_sha256",
    }
    if set(receipt) != expected_fields or receipt.get("schema_version") != 1:
        raise CampaignError("frozen-candidate receipt has missing or unknown fields")

    for field, relative in (
        ("activation_sha256", FROZEN_ACTIVATION_PATH),
        ("budget_verdict_sha256", FROZEN_BUDGET_VERDICT_PATH),
    ):
        expected_sha256 = receipt.get(field)
        if (
            not isinstance(expected_sha256, str)
            or not re.fullmatch(r"[0-9a-f]{64}", expected_sha256)
            or sha256_bytes(read_bounded_frozen_member(diagnostic, relative))
            != expected_sha256
        ):
            raise CampaignError(f"frozen-candidate {field} does not bind its archived file")

    reference_files = receipt.get("reference_evidence_sha256")
    if not isinstance(reference_files, list) or not reference_files or len(reference_files) > 256:
        raise CampaignError("frozen-candidate reference evidence manifest is invalid")
    reference_paths: set[str] = set()
    for item in reference_files:
        if not isinstance(item, dict) or set(item) != {"id", "path", "sha256"}:
            raise CampaignError("frozen-candidate reference evidence entry is malformed")
        relative = item.get("path")
        expected_sha256 = item.get("sha256")
        if (
            not isinstance(relative, str)
            or not relative.startswith("target/test-evidence/0.67/")
            or item.get("id") != relative
            or relative in reference_paths
            or not isinstance(expected_sha256, str)
            or not re.fullmatch(r"[0-9a-f]{64}", expected_sha256)
        ):
            raise CampaignError("frozen-candidate reference evidence identity is invalid")
        reference_paths.add(relative)
        if sha256_bytes(read_bounded_frozen_member(diagnostic, relative)) != expected_sha256:
            raise CampaignError(f"frozen-candidate archived evidence digest mismatch: {relative}")

    canary_files = receipt.get("canary_receipt_sha256")
    if not isinstance(canary_files, list):
        raise CampaignError("frozen-candidate canary evidence manifest is invalid")
    expected_canaries = [
        (work_item, f"target/release-evidence/canaries/0.67.1-{work_item}.json")
        for work_item in EXPECTED_0671_WORK_ITEMS
    ]
    observed_canaries: list[tuple[str, str]] = []
    for item in canary_files:
        if not isinstance(item, dict) or set(item) != {"id", "path", "sha256"}:
            raise CampaignError("frozen-candidate canary evidence entry is malformed")
        identity = item.get("id")
        relative = item.get("path")
        expected_sha256 = item.get("sha256")
        if (
            not isinstance(identity, str)
            or not isinstance(relative, str)
            or not isinstance(expected_sha256, str)
            or not re.fullmatch(r"[0-9a-f]{64}", expected_sha256)
        ):
            raise CampaignError("frozen-candidate canary evidence identity is invalid")
        observed_canaries.append((identity, relative))
        if sha256_bytes(read_bounded_frozen_member(diagnostic, relative)) != expected_sha256:
            raise CampaignError(f"frozen-candidate archived canary digest mismatch: {relative}")
    if observed_canaries != expected_canaries:
        raise CampaignError("frozen-candidate canary evidence is not the exact W0-W7 set")


def validate_ship_aggregate(
    aggregate: dict[str, Any], state: dict[str, Any]
) -> None:
    expected_fields = {
        "schema_version",
        "release",
        "source_commit",
        "current_worktree_dirty",
        "receipts_supplied",
        "counts",
        "reasons",
        "work_items",
    }
    counts = aggregate.get("counts")
    work_items = aggregate.get("work_items")
    if (
        set(aggregate) != expected_fields
        or aggregate.get("schema_version") != 1
        or aggregate.get("release") != "0.67.1"
        or aggregate.get("source_commit") != state["expected_sha"]
        or aggregate.get("current_worktree_dirty") is not False
        or aggregate.get("receipts_supplied") is not True
        or aggregate.get("reasons") != []
        or not isinstance(counts, dict)
        or set(counts)
        != {"planned", "implemented", "fast-green", "gated-green", "ship-ready"}
        or not isinstance(work_items, list)
        or not work_items
    ):
        raise CampaignError("frozen-candidate aggregate is not an exact ship-ready report")

    work_item_ids: list[str] = []
    for item in work_items:
        if (
            not isinstance(item, dict)
            or set(item) != {"id", "stage", "reasons"}
            or not isinstance(item.get("id"), str)
            or not item["id"]
            or item["id"] in work_item_ids
            or item.get("stage") != "ship-ready"
            or item.get("reasons") != []
        ):
            raise CampaignError(
                "frozen-candidate aggregate contains a non-ship or malformed work item"
            )
        work_item_ids.append(item["id"])
    if tuple(work_item_ids) != EXPECTED_0671_WORK_ITEMS:
        raise CampaignError("frozen-candidate aggregate is not the exact W0-W7 release set")
    if counts != {
        "planned": 0,
        "implemented": 0,
        "fast-green": 0,
        "gated-green": 0,
        "ship-ready": len(work_items),
    }:
        raise CampaignError("frozen-candidate aggregate counts do not match its work items")


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


def expect_common_receipt(
    receipt: dict[str, Any],
    state: dict[str, Any],
    run_id: int,
    *,
    schema_version: int,
) -> None:
    if (
        receipt.get("schema_version") != schema_version
        or receipt.get("release") != "0.67.1"
        or receipt.get("profile") != "reference-v1"
        or receipt.get("source_commit") != state["expected_sha"]
    ):
        raise CampaignError("receipt release/profile/source identity mismatch")
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


def validate_full_dress_admission_chain(
    admission: dict[str, Any],
    first_receipt: dict[str, Any],
    second_receipt: dict[str, Any],
    expected_members: dict[str, str],
) -> dict[str, str]:
    expected_fields = {
        "schema_version",
        "release",
        "profile",
        "source_commit",
        "runner_fingerprint",
        "runner_provisioning_sha256",
        "prebuild_contract_digest",
        "scenario_contract_set_digest",
        "full_dress_runs",
        "passed",
        "bootstrap_admission_eligible",
        "bootstrap_eligible",
        "ship_evidence_eligible",
    }
    if (
        set(admission) != expected_fields
        or admission.get("schema_version") != 1
        or admission.get("release") != "0.67.1"
        or admission.get("profile") != "reference-v1"
    ):
        raise CampaignError("full-dress admission has missing or unknown fields")

    for receipt in (first_receipt, second_receipt):
        if (
            receipt.get("schema_version") != 1
            or receipt.get("release") != "0.67.1"
            or receipt.get("profile") != "reference-v1"
            or receipt.get("source_commit") != admission.get("source_commit")
            or receipt.get("runner_fingerprint") != admission.get("runner_fingerprint")
        ):
            raise CampaignError("full-dress admission mixes receipt identity")

    contract_fields = (
        "runner_provisioning_sha256",
        "prebuild_contract_digest",
        "scenario_contract_set_digest",
    )
    contracts: dict[str, str] = {}
    for field in contract_fields:
        value = admission.get(field)
        if (
            not isinstance(value, str)
            or not re.fullmatch(r"[0-9a-f]{64}", value)
            or first_receipt.get(field) != value
            or second_receipt.get(field) != value
        ):
            raise CampaignError(f"full-dress admission {field} chain is inconsistent")
        contracts[field] = value

    members = admission.get("full_dress_runs")
    if not isinstance(members, list) or len(members) != 2:
        raise CampaignError("full-dress admission must contain exactly two runs")
    observed_members: dict[str, str] = {}
    for member in members:
        if not isinstance(member, dict) or set(member) != {"github_run_id", "receipt_sha256"}:
            raise CampaignError("full-dress admission member is malformed")
        run_id = member.get("github_run_id")
        receipt_sha256 = member.get("receipt_sha256")
        if (
            not isinstance(run_id, str)
            or not RUN_ID_RE.fullmatch(run_id)
            or run_id in observed_members
            or not isinstance(receipt_sha256, str)
            or not re.fullmatch(r"[0-9a-f]{64}", receipt_sha256)
        ):
            raise CampaignError("full-dress admission member identity is invalid")
        observed_members[run_id] = receipt_sha256
    if observed_members != expected_members:
        raise CampaignError("full-dress admission run-to-receipt mapping is wrong")
    return contracts


def validate_stage_artifacts(
    campaign_dir: Path,
    state: dict[str, Any],
    spec: dict[str, Any],
    run_id: int,
    artifacts: dict[str, Path],
) -> dict[str, Any]:
    step = spec["name"]
    if spec["mode"] == "frozen-candidate":
        diagnostic = artifact_named(
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
            or not sealed_json_receipt_is_valid(receipt)
        ):
            raise CampaignError("frozen-candidate receipt identity/eligibility contract failed")
        validate_frozen_receipt_artifacts(receipt, diagnostic)
        state["stages"]["runner_fingerprint"] = fingerprint
        retained = retain_receipt(campaign_dir, "frozen-candidate.json", data)
        aggregate = read_unique_member(diagnostic, "0.67.1.json")
        aggregate_receipt = json_receipt(aggregate, "0.67.1 aggregate release evidence")
        validate_ship_aggregate(aggregate_receipt, state)
        aggregate_path = retain_receipt(campaign_dir, "release-evidence-0.67.1.json", aggregate)
        return {
            "receipt": str(retained),
            "receipt_sha256": sha256_bytes(data),
            "aggregate": str(aggregate_path),
            "aggregate_sha256": sha256_bytes(aggregate),
        }

    if spec["mode"] == "qualify":
        diagnostic = artifact_named(artifacts, f"performance-0671-qualification-{state['expected_sha']}-{run_id}")
        validate_host_admission_artifact(campaign_dir, state, diagnostic)
        data = read_unique_member(diagnostic, "qualification.json")
        receipt = json_receipt(data, "qualification receipt")
        expect_common_receipt(receipt, state, run_id, schema_version=1)
        if receipt.get("mode") != "qualification-only" or receipt.get("bootstrap_eligible") is not False:
            raise CampaignError("qualification receipt eligibility contract failed")
        retained = retain_receipt(campaign_dir, "qualification.json", data)
        return {"receipt": str(retained), "receipt_sha256": sha256_bytes(data)}

    if spec["mode"] == "full-dress":
        reusable = artifacts.get("performance-0671-full-dress-receipt")
        if reusable is None:
            raise CampaignError("full-dress reusable receipt artifact is missing")
        diagnostic = artifact_named(artifacts, f"performance-0671-full-dress-{state['expected_sha']}-{run_id}")
        validate_host_admission_artifact(campaign_dir, state, diagnostic)
        data = read_unique_member(reusable, "full-dress-receipt.json")
        if read_unique_member(diagnostic, "full-dress-receipt.json") != data:
            raise CampaignError("full-dress reusable receipt differs from diagnostic archive")
        receipt = json_receipt(data, "full-dress receipt")
        expect_common_receipt(receipt, state, run_id, schema_version=1)
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
        if read_unique_member(diagnostic, "full-dress-admission.json") != admission_data:
            raise CampaignError("full-dress reusable admission differs from diagnostic archive")
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
        first_stage = state["stages"]["full-dress-1"]
        first_receipt_path = Path(first_stage["receipt"])
        if (
            not first_receipt_path.is_file()
            or first_receipt_path.is_symlink()
            or sha256_file(first_receipt_path) != first_stage["receipt_sha256"]
        ):
            raise CampaignError("first full-dress receipt changed before admission")
        first_receipt = json_receipt(
            first_receipt_path.read_bytes(), "first full-dress receipt"
        )
        contracts = validate_full_dress_admission_chain(
            admission,
            first_receipt,
            receipt,
            {
                str(first_stage["run_id"]): first_stage["receipt_sha256"],
                str(run_id): result["receipt_sha256"],
            },
        )
        admission_path = retain_receipt(campaign_dir, "full-dress-admission.json", admission_data)
        result.update(
            {
                "admission": str(admission_path),
                "admission_sha256": sha256_bytes(admission_data),
                **contracts,
            }
        )
        return result

    reusable = artifacts.get("performance-0671-bootstrap-receipt")
    if reusable is None:
        raise CampaignError("bootstrap reusable receipt artifact is missing")
    diagnostic = artifact_named(artifacts, f"performance-0671-bootstrap-{state['expected_sha']}-{run_id}")
    validate_host_admission_artifact(campaign_dir, state, diagnostic)
    data = read_unique_member(reusable, "bootstrap-sample.json")
    if read_unique_member(diagnostic, "bootstrap-sample.json") != data:
        raise CampaignError("bootstrap reusable receipt differs from diagnostic archive")
    receipt = json_receipt(data, "bootstrap sample receipt")
    expect_common_receipt(receipt, state, run_id, schema_version=2)
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
        or any(
            receipt.get(field) != state["stages"]["full-dress-2"].get(field)
            for field in (
                "runner_provisioning_sha256",
                "prebuild_contract_digest",
                "scenario_contract_set_digest",
            )
        )
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


def check_artifact_transport_canary(
    campaign_dir: Path,
    state: dict[str, Any],
    step: str,
    *,
    attempts: int = ARTIFACT_CANARY_ATTEMPTS,
    retry_delay_seconds: int = ARTIFACT_CANARY_RETRY_DELAY_SECONDS,
) -> None:
    """Prove that GitHub artifact storage is readable before dispatching a long run."""
    if attempts <= 0 or retry_delay_seconds <= 0:
        raise CampaignError("artifact transport canary retry settings are invalid")
    output = campaign_dir / f".artifact-transport-canary-{step}.zip"
    latest: ArtifactTransportError | None = None
    for attempt in range(1, attempts + 1):
        try:
            try:
                response = gh_json(
                    ["api", f"repos/{state['repository']}/actions/artifacts?per_page=100"],
                    timeout_seconds=ARTIFACT_CANARY_TIMEOUT_SECONDS,
                )
            except CampaignError as error:
                raise ArtifactTransportError(f"artifact canary listing failed: {error}") from error
            artifacts = response.get("artifacts") if isinstance(response, dict) else None
            if not isinstance(artifacts, list):
                raise ArtifactTransportError("artifact canary listing has an invalid shape")
            candidates = [
                artifact
                for artifact in artifacts
                if isinstance(artifact, dict)
                and isinstance(artifact.get("id"), int)
                and isinstance(artifact.get("name"), str)
                and isinstance(artifact.get("size_in_bytes"), int)
                and 0 < artifact["size_in_bytes"] <= ARTIFACT_CANARY_MAX_BYTES
                and artifact.get("expired") is not True
            ]
            if not candidates:
                raise ArtifactTransportError(
                    "artifact transport canary found no non-expired artifact within the size limit"
                )
            artifact = min(candidates, key=lambda candidate: candidate["size_in_bytes"])
            download_binary(
                [
                    "gh",
                    "api",
                    f"repos/{state['repository']}/actions/artifacts/{artifact['id']}/zip",
                ],
                output,
                timeout_seconds=ARTIFACT_CANARY_TIMEOUT_SECONDS,
            )
            try:
                with zipfile.ZipFile(output) as archive:
                    corrupt = archive.testzip()
            except zipfile.BadZipFile as error:
                raise ArtifactTransportError("artifact transport canary returned a non-ZIP body") from error
            if corrupt is not None:
                raise ArtifactTransportError(
                    f"artifact transport canary ZIP has a corrupt member: {corrupt}"
                )
            receipt = {
                "schema_version": 1,
                "step": step,
                "checked_at": utc_now(),
                "artifact_id": artifact["id"],
                "artifact_name": artifact["name"],
                "reported_size_bytes": artifact["size_in_bytes"],
                "archive_size_bytes": output.stat().st_size,
                "archive_sha256": sha256_file(output),
            }
            write_json_atomic(campaign_dir / f"artifact-transport-canary-{step}.json", receipt)
            append_event(
                campaign_dir,
                "artifact-transport-canary-passed",
                step=step,
                artifact_id=artifact["id"],
            )
            return
        except ArtifactTransportError as error:
            latest = error
            if attempt == attempts:
                break
            append_event(
                campaign_dir,
                "artifact-transport-canary-retry",
                step=step,
                attempt=attempt,
                retry_in_seconds=retry_delay_seconds,
                detail=str(error),
            )
            time.sleep(retry_delay_seconds)
        finally:
            if output.exists():
                os.chmod(output, stat.S_IRUSR | stat.S_IWUSR)
                output.unlink()
            output.with_name(f".{output.name}.partial").unlink(missing_ok=True)
    raise ArtifactTransportError(
        f"artifact transport canary failed after {attempts} attempts: {latest}"
    ) from latest


def check_pre_dispatch(campaign_dir: Path, state: dict[str, Any], step: str) -> None:
    ensure_checkout(state["expected_sha"])
    if github_main_sha(state) != state["expected_sha"]:
        raise CampaignError("origin main no longer equals the qualified campaign SHA")
    ensure_github_runner_contract(state)
    ensure_runner_offline(campaign_dir)
    check_artifact_transport_canary(campaign_dir, state, step)
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
        while True:
            watch_status = run_visible(
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
            try:
                run = view_run(state, run_id)
            except Exception as error:
                append_event(
                    campaign_dir,
                    "run-watch-status-retry",
                    step=step,
                    run_id=run_id,
                    detail=str(error),
                )
                time.sleep(15)
                continue
            if run.get("status") == "completed":
                return 0 if run.get("conclusion") == "success" else (watch_status or 1)
            append_event(
                campaign_dir,
                "run-watch-transport-retry",
                step=step,
                run_id=run_id,
                watch_status=watch_status,
            )
            time.sleep(15)
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
            if len(matches) > 1:
                raise CampaignError(f"cannot recover uniquely from interrupted dispatch for {step}")
            if matches:
                run = matches[0]
            else:
                arm_runner_watchdog(campaign_dir, state, step)
                runner_online(campaign_dir)
                try:
                    run = discover_run(state, step)
                except Exception:
                    ensure_runner_offline(campaign_dir)
                    disarm_runner_watchdog(campaign_dir, state, step)
                    raise
        else:
            if matches:
                raise CampaignError(f"campaign id/step was already used: {step}")
            stage["status"] = "dispatching"
            stage["dispatch_started_at"] = utc_now()
            save_state(campaign_dir, state)
            append_event(campaign_dir, "stage-dispatch-started", step=step)
            arm_runner_watchdog(campaign_dir, state, step)
            runner_online(campaign_dir)
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
            try:
                result = run_visible(
                    command,
                    cwd=repo_root(),
                    log_path=campaign_dir / f"{step}.log",
                    timeout_seconds=GITHUB_CONTROL_TIMEOUT_SECONDS,
                )
                if result != 0:
                    raise CampaignError(f"workflow dispatch failed for {step}")
                run = discover_run(state, step)
            except Exception:
                ensure_runner_offline(campaign_dir)
                disarm_runner_watchdog(campaign_dir, state, step)
                raise
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
    artifact_transport_error: str | None = None
    artifact_integrity_error: str | None = None
    try:
        artifacts = download_artifacts_with_retry(campaign_dir, state, run_id, step)
    except ArtifactTransportError as error:
        artifact_transport_error = str(error)
    except Exception as error:  # integrity/identity failures permanently invalidate evidence
        artifact_integrity_error = str(error)

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
    if artifact_integrity_error:
        rejection_reasons.append(artifact_integrity_error)
    if post_error:
        rejection_reasons.append(post_error)
    if rejection_reasons:
        if artifact_transport_error:
            rejection_reasons.append(artifact_transport_error)
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

    if artifact_transport_error:
        waiting_since = stage.get("awaiting_artifacts_since") or utc_now()
        stage.update(
            {
                "status": "awaiting-artifacts",
                "conclusion": "success",
                "awaiting_artifacts_since": waiting_since,
                "artifact_transport_attempted_at": utc_now(),
                "artifact_transport_error": artifact_transport_error,
            }
        )
        state["phase"] = "awaiting-artifacts"
        save_state(campaign_dir, state)
        append_event(
            campaign_dir,
            "stage-awaiting-artifacts",
            step=step,
            run_id=run_id,
            detail=artifact_transport_error,
        )
        raise CampaignError(
            f"stage {step} run {run_id} succeeded and is awaiting artifact retrieval: "
            f"{artifact_transport_error}"
        )

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
    for field in (
        "awaiting_artifacts_since",
        "artifact_transport_attempted_at",
        "artifact_transport_error",
        "rejection_reasons",
    ):
        stage.pop(field, None)
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


def select_sample_set_cargo() -> list[str]:
    runner_cargo = "/home/github-runner/.cargo/bin/cargo"
    try:
        runner_probe = subprocess.run(
            runner_command("test", "-x", runner_cargo),
            cwd=repo_root(),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=30,
        )
    except (OSError, subprocess.TimeoutExpired):
        runner_probe = None
    if runner_probe is not None and runner_probe.returncode == 0:
        return runner_command(runner_cargo)

    found = shutil.which("cargo")
    if found is None or Path(found) == Path(runner_cargo):
        raise CampaignError("cargo is unavailable for final sample-set validation")
    return [found]


def cargo_sample_set(campaign_dir: Path) -> Path:
    output = (
        repo_root()
        / "target/test-evidence/0.67.1/controller-sample-sets"
        / f"{campaign_dir.name}-{time.time_ns()}.json"
    )
    command = select_sample_set_cargo()
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
            "--output",
            str(output),
        ]
    )
    result = run_visible(
        command,
        cwd=repo_root(),
        log_path=campaign_dir / "sample-set-validation.log",
        timeout_seconds=SAMPLE_SET_VALIDATION_TIMEOUT_SECONDS,
    )
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
        artifacts = download_artifacts_with_retry(
            campaign_dir, state, run_id, step
        )
        diagnostic = artifact_named(
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
    if state["phase"] not in {"ready", "running", "awaiting-artifacts"}:
        raise CampaignError(
            f"run requires ready/running/awaiting-artifacts state, found {state['phase']}"
        )
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
    if state["phase"] not in {"ready", "running", "awaiting-artifacts"}:
        raise CampaignError(
            "frozen run requires a fresh ready/running/awaiting-artifacts campaign, "
            f"found {state['phase']}"
        )
    require_tools(["gh", "git", "sudo", "systemctl"])
    run_capture(
        ["gh", "auth", "status"],
        cwd=repo_root(),
        timeout_seconds=GITHUB_CONTROL_TIMEOUT_SECONDS,
    )
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
    print("LOCAL_HOST_CLOSEOUT_COMPLETE=true")
    print(
        "SERVER_DELETION_BLOCKED=true: copy and verify the complete campaign and host-state "
        "off-host, publish/verify GitHub artifacts, commit the sanitized final report, and "
        "complete a secret scan before provider deletion."
    )


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
        if args.command == "status":
            args.handler(args)
        else:
            with sudo_lease():
                args.handler(args)
    except (CampaignError, OSError, subprocess.SubprocessError) as error:
        print(f"reference campaign rejected: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
