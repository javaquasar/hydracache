"""Relay an immutable GitHub artifact to the isolated AX42 observer."""

import argparse
from dataclasses import dataclass
from hashlib import sha256
import os
from pathlib import Path
import re
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
from zipfile import ZIP_DEFLATED, ZIP_STORED, ZipFile

from . import w11_guest_artifact_transfer_074 as transfer
from . import w11_guest_observer_074 as driver


build, receipt, workflow = transfer.build, transfer.receipt, transfer.workflow
require = build.require
COMMAND = "observe-v2"
SSH_LOGIN = "hydracache-admin"
ACCOUNT = "hydracache-w11-observer"
PERMIT = Path("/etc/hydracache-w11-observer/permit.json")
RUNTIME = Path("/opt/hydracache-w11-observer/runtime")
STATE_DIRECTORY = Path("/var/lib/hydracache-w11-observer")
WORK = Path("/var/lib/private/hydracache-w11-observer")
SOCKET = Path("/run/hydracache-perf/supervisor-v1.sock")
SCHEMA = "hydracache-w11-ssh-observer-074-v1"
PERMIT_SCHEMA = "hydracache-w11-ssh-permit-074-v1"
MAX_ARCHIVE = 1024**3


@dataclass(frozen=True)
class Submission:
    request: bytes
    subject: dict
    artifact: dict
    remaining_ns: int


def artifact_spec(item):
    require(isinstance(item, dict) and set(item) == {"id", "sha256", "bytes"}
            and isinstance(item["id"], str) and re.fullmatch(r"[1-9][0-9]{0,19}", item["id"])
            and receipt.is_hex(item["sha256"], 64) and type(item["bytes"]) is int
            and 0 < item["bytes"] <= MAX_ARCHIVE, "invalid artifact identity or size")
    return item


def lookup_artifact(document, role, binding):
    require(isinstance(document, dict) and document.get("total_count") == 1
            and isinstance(document.get("artifacts"), list) and len(document["artifacts"]) == 1,
            "expected one build artifact")
    row = document["artifacts"][0]
    suffix = "-".join(binding[key] for key in ("workflow_commit", "run_id", "run_attempt"))
    require(isinstance(row, dict) and row.get("name") == f"product-w11-build-074-{role}-{suffix}"
            and row.get("expired") is False and isinstance(row.get("workflow_run"), dict)
            and isinstance(row.get("digest"), str) and row["digest"].startswith("sha256:"),
            "wrong or expired build artifact")
    expected_run = {"id": int(binding["run_id"]), "repository_id": int(build.REPOSITORY_ID),
                    "head_repository_id": int(build.REPOSITORY_ID),
                    "head_branch": "main", "head_sha": binding["workflow_commit"]}
    require(all(row["workflow_run"].get(key) == value for key, value in expected_run.items()),
            "artifact belongs to another repository or workflow run")
    return artifact_spec({"id": str(row.get("id")), "sha256": row["digest"][7:], "bytes": row.get("size_in_bytes")})


def require_archive(path, artifact):
    info = path.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size == artifact["bytes"],
            "build archive size or type differs")
    require(driver.digest(path) == artifact["sha256"], "build archive SHA-256 differs")


def unpack_artifact(archive, role, directory, binary_bytes=None):
    directory.mkdir(mode=0o700)
    with ZipFile(archive) as packet:
        members = packet.infolist()
        require(len(members) == 2 and {item.filename for item in members} == {workflow.SUBJECT, f"{role}-server"},
                "unexpected build archive member")
        for item in members:
            limit = receipt.MAX_JSON_BYTES if item.filename == workflow.SUBJECT else MAX_ARCHIVE
            require(0 < item.file_size <= limit and item.compress_type in (ZIP_STORED, ZIP_DEFLATED)
                    and stat.S_IFMT(item.external_attr >> 16) in (0, stat.S_IFREG),
                    "unexpected build archive member size or type")
            if binary_bytes is not None and item.filename != workflow.SUBJECT:
                require(item.file_size == binary_bytes, "archive ELF size differs from the permit")
            with packet.open(item) as incoming, (directory / item.filename).open("xb") as out:
                receive_bytes(incoming, out, item.file_size)


def receive_bytes(stream, output, length):
    remaining = length
    while remaining:
        chunk = stream.read(min(remaining, 1024**2))
        require(bool(chunk), "short artifact transfer")
        output.write(chunk)
        remaining -= len(chunk)
    require(stream.read(1) == b"", "unexpected trailing artifact bytes")


def make_header(artifact, request, remaining_ns):
    raw = receipt.canonical({"schema": SCHEMA, "artifact": artifact, "request": build.decode_json(request),
                             "remaining_ns": remaining_ns}) + b"\n"
    require(0 < len(raw) <= receipt.MAX_JSON_BYTES, "oversized observer header")
    return len(raw).to_bytes(4, "big") + raw


def read_header(stream):
    prefix = stream.read(4)
    require(len(prefix) == 4, "missing observer header")
    length = int.from_bytes(prefix, "big")
    require(0 < length <= receipt.MAX_JSON_BYTES, "invalid observer header size")
    raw = stream.read(length)
    require(len(raw) == length, "partial observer header")
    item = build.decode_json(raw)
    require(raw == receipt.canonical(item) + b"\n", "noncanonical observer header")
    return item


def authorized_submission(header, permit, now):
    require(isinstance(permit, dict) and set(permit) == {"schema", "expires_unix", "roles"}
            and permit["schema"] == PERMIT_SCHEMA and type(permit["expires_unix"]) is int
            and now < permit["expires_unix"] and isinstance(permit["roles"], dict)
            and set(permit["roles"]) == set(build.ROLES), "missing or expired AX42 observer permit")
    require(isinstance(header, dict) and set(header) == {"schema", "artifact", "request", "remaining_ns"}
            and header["schema"] == SCHEMA and type(header["remaining_ns"]) is int
            and 0 < header["remaining_ns"] <= driver.JOB_SECONDS * 10**9, "invalid observer request or job window")
    request = header["request"]
    require(isinstance(request, dict) and set(request) == {"schema", "scope", "nonce", "subject"}
            and request["schema"] == transfer.SCHEMA and request["scope"] == "protected"
            and receipt.is_hex(request["nonce"], 64) and isinstance(request["subject"], dict),
            "protected guest request required")
    role = request["subject"].get("role")
    require(isinstance(role, str) and role in permit["roles"], "unpermitted role")
    entry = permit["roles"][role]
    artifact = artifact_spec(header["artifact"])
    require(isinstance(entry, dict) and set(entry) == {"artifact", "subject"}
            and artifact == artifact_spec(entry["artifact"]) and request["subject"] == entry["subject"],
            "archive or build subject differs from the AX42 permit")
    return Submission(receipt.canonical(request) + b"\n", request["subject"], artifact, header["remaining_ns"])


def stage_input(stream, submission, source, binding, workspace):
    archive = workspace / "build.zip"
    with archive.open("xb") as output:
        receive_bytes(stream, output, submission.artifact["bytes"])
    require_archive(archive, submission.artifact)
    directory = workspace / "incoming"
    unpack_artifact(archive, source.role, directory, submission.subject["binary"]["bytes"])
    transfer.check_request(submission.request, sha256(submission.request).hexdigest(), source, directory,
                           binding, "protected")
    return directory


def hold_claim(directory, submission, binding, role):
    parts = [binding["run_id"], binding["run_attempt"]]
    require(all(re.fullmatch(r"[1-9][0-9]{0,19}", item) for item in parts) and role in build.ROLES,
            "invalid claim identity")
    claim = directory / f"run-{parts[0]}-{parts[1]}-{role}"
    try:
        claim.mkdir(mode=0o700)
    except FileExistsError as error:
        raise ValueError("observer already claimed this role and attempt") from error
    store_leaf(claim / "guest-request.json", submission.request)
    sync_directory(directory)
    return claim


def sync_directory(path):
    if os.name == "posix":
        with_dir = os.open(path, os.O_RDONLY | os.O_DIRECTORY)
        try:
            os.fsync(with_dir)
        finally:
            os.close(with_dir)


def store_leaf(path, data):
    with path.open("xb") as output:
        output.write(data)
        output.flush()
        os.fsync(output.fileno())
    sync_directory(path.parent)


def root_owned(path):
    require(path.resolve(strict=True) == path, "observer installation path contains a symlink")
    for part in (path, *path.parents):
        info = part.lstat()
        require(info.st_uid == 0 and info.st_mode & 0o022 == 0, "observer installation must be root-owned")


def installed_policy():
    import pwd

    user = pwd.getpwnam(ACCOUNT)
    require(61184 <= user.pw_uid <= 65519 and 61184 <= user.pw_gid <= 65519
            and os.getuid() == os.geteuid() == user.pw_uid and os.getgid() == os.getegid() == user.pw_gid
            and set(os.getgroups()) <= {user.pw_gid}, "unexpected observer account or groups")
    require(all(line.partition(":")[0] != ACCOUNT for line in Path("/etc/passwd").read_text().splitlines()),
            "observer identity must be transient")
    status = dict(line.split(":", 1) for line in Path("/proc/self/status").read_text().splitlines())
    require(status["NoNewPrivs"].strip() == "1" and int(status["CapEff"], 16) == 0,
            "observer process can acquire additional privileges")
    require(not os.access(SOCKET, os.W_OK), "observer account can access the supervisor socket")
    require(os.environ.get("SSH_ORIGINAL_COMMAND") == COMMAND, "unexpected SSH command")
    for path in (PERMIT, workflow.APPROVAL, RUNTIME, WORK.parent, STATE_DIRECTORY.parent):
        root_owned(path)
    require(os.environ.get("STATE_DIRECTORY") == str(STATE_DIRECTORY) and STATE_DIRECTORY.is_symlink()
            and STATE_DIRECTORY.lstat().st_uid == 0 and STATE_DIRECTORY.resolve(strict=True) == WORK,
            "unexpected systemd state directory")
    workflow.require_directory(WORK)
    info = WORK.stat()
    require(info.st_uid == user.pw_uid and info.st_gid == user.pw_gid
            and stat.S_IMODE(info.st_mode) == 0o700, "observer workspace is not private")
    roles, pin = workflow.approved_plan()
    driver.check_runtime(RUNTIME, pin)
    require(driver.host_guard(WORK) >= driver.probe.DISK_FLOOR + driver.MIN_STAGING,
            "insufficient staging space above the AX42 reserve")
    return roles, pin, workflow.read_canonical(PERMIT)


def original_context(subject):
    return {"GITHUB_REPOSITORY": build.REPOSITORY, "GITHUB_REPOSITORY_ID": build.REPOSITORY_ID,
            "GITHUB_REF": "refs/heads/main", "GITHUB_WORKFLOW_REF": build.WORKFLOW,
            "GITHUB_SHA": subject["workflow_commit"], "GITHUB_RUN_ID": subject["run_id"],
            "GITHUB_RUN_ATTEMPT": subject["run_attempt"]}


def serve(stream, output, began):
    import fcntl

    roles, pin, permit = installed_policy()
    header = read_header(stream)
    submission = authorized_submission(header, permit, time.time())
    deadline = min(began + submission.remaining_ns / 10**9, time.monotonic() + permit["expires_unix"] - time.time())
    os.environ.update(original_context(submission.subject))
    binding = build.workflow_binding()
    role = submission.subject["role"]
    binary_bytes = submission.subject["binary"]["bytes"]
    require(type(binary_bytes) is int and 0 < binary_bytes <= MAX_ARCHIVE, "unpermitted binary size")
    with (WORK / "observer.lock").open("ab") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        need = driver.MIN_STAGING + submission.artifact["bytes"] + 3 * binary_bytes
        require(driver.host_guard(WORK) >= driver.probe.DISK_FLOOR + need, "insufficient space for the observer copies")
        driver.can_boot(deadline, time.monotonic())
        with tempfile.TemporaryDirectory(prefix="w11-relay-", dir=WORK) as directory:
            workspace = Path(directory)
            incoming = stage_input(stream, submission, roles[role], binding, workspace)
            media = workspace / "media"
            request, request_digest = driver.prepare_seed(role, incoming, media, submission.request)
            driver.can_boot(deadline, time.monotonic())
            claim = hold_claim(WORK, submission, binding, role)
            raw, _ = driver.run_vm(RUNTIME, workspace, media, request_digest, deadline)
            driver.check_runtime(RUNTIME, pin)
            transfer.protected_receive_result(role, request, request_digest, incoming, raw, workspace / "observations")
            store_leaf(claim / "serial.log", raw)
            output.write(raw)
            output.flush()


def ssh_command(host, identity, known_hosts):
    require(re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9.-]{0,252}", host) is not None, "unexpected observer hostname")
    require(all(path.is_file() for path in (identity, known_hosts)), "missing SSH identity or host pin")
    return ["/usr/bin/ssh", "-F", "/dev/null", "-T", "-o", "BatchMode=yes", "-o", "IdentitiesOnly=yes",
            "-o", "StrictHostKeyChecking=yes", "-o", "GlobalKnownHostsFile=/dev/null",
            "-o", f"UserKnownHostsFile={known_hosts}", "-o", "ClearAllForwardings=yes",
            "-o", "ConnectTimeout=8", "-i", str(identity), f"{SSH_LOGIN}@{host}", COMMAND]


def submit(args):
    deadline = driver.job_deadline(args.clock)
    binding = build.workflow_binding()
    artifact = lookup_artifact(build.load(args.metadata), args.role, binding)
    files = list(args.download.iterdir())
    require(len(files) == 1, "expected a single raw GitHub build archive")
    archive = files[0]
    require_archive(archive, artifact)
    command = ssh_command(os.environ["W11_OBSERVER_SSH_HOST"], args.identity, args.known_hosts)
    with tempfile.TemporaryDirectory(prefix="w11-relay-client-", dir=args.observations.parent) as directory:
        incoming = Path(directory) / "incoming"
        unpack_artifact(archive, args.role, incoming)
        request = transfer.protected_request(args.role, incoming)
        driver.can_boot(deadline, time.monotonic())
        with tempfile.TemporaryFile(dir=directory) as packet, archive.open("rb") as raw:
            packet.write(make_header(artifact, request, int((deadline - time.monotonic()) * 10**9)))
            shutil.copyfileobj(raw, packet, length=1024**2)
            packet.seek(0)
            response = subprocess.run(command, stdin=packet, stdout=subprocess.PIPE,
                                      env={"PATH": "/usr/bin:/bin", "LANG": "C.UTF-8"}, check=True,
                                      timeout=deadline - time.monotonic() - driver.UPLOAD_SECONDS).stdout
        verified = transfer.protected_receive_result(args.role, request, sha256(request).hexdigest(),
                                                      incoming, response, args.observations)
    print(f"verified archive: {verified['raw_sha256']}")


def main():
    parser = argparse.ArgumentParser()
    commands = parser.add_subparsers(dest="operation", required=True)
    which = commands.add_parser("artifact")
    which.add_argument("role", choices=build.ROLES)
    which.add_argument("metadata", type=Path)
    send = commands.add_parser("submit")
    send.add_argument("role", choices=build.ROLES)
    for field in ("clock", "metadata", "download", "observations", "identity", "known_hosts"):
        send.add_argument(field, type=Path)
    commands.add_parser("serve")
    args = parser.parse_args()
    os.umask(0o077)
    if args.operation == "artifact":
        print("id=" + lookup_artifact(build.load(args.metadata), args.role, build.workflow_binding())["id"])
    elif args.operation == "submit":
        submit(args)
    else:
        started = time.monotonic()
        for event in (signal.SIGTERM, signal.SIGHUP, signal.SIGALRM):
            signal.signal(event, driver.interrupted)
        signal.setitimer(signal.ITIMER_REAL, driver.JOB_SECONDS)
        serve(sys.stdin.buffer, sys.stdout.buffer, started)


if __name__ == "__main__":
    main()
