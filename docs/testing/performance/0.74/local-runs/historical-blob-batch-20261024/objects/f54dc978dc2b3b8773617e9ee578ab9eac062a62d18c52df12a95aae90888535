"""Finite immutable-source prototype. Not imported by any runtime or guard."""
from dataclasses import dataclass
import hashlib
import os
from pathlib import Path
import re
import subprocess
import tempfile
import time

MAX_REQUESTS = 256
MAX_INPUT = 256 * 1024
MAX_OUTPUT = 16 * 1024 * 1024
MAX_STDERR = 64 * 1024
BOUND_SECONDS = 30
OID = re.compile(r"[0-9a-f]{40}\Z")


class Refused(RuntimeError):
    """A prototype failure, never an empty historical source or cached verdict."""


@dataclass(frozen=True)
class Request:
    revision: str
    path: str

    def spec(self):
        if not isinstance(self.revision, str) or not OID.fullmatch(self.revision):
            raise Refused("full lowercase SHA1 commit required")
        if (not isinstance(self.path, str) or not self.path or len(self.path) > 4096
                or any(c in self.path for c in "\0\r\n\\")
                or any(part in ("", ".", "..") for part in self.path.split("/"))):
            raise Refused("explicit repository-relative path required")
        try:
            return f"{self.revision}:{self.path}".encode("utf-8", errors="strict")
        except UnicodeError as error:
            raise Refused("UTF8 path required") from error


@dataclass(frozen=True)
class Output:
    stdout: bytes
    stderr: bytes
    returncode: int


def run_git(root, args, data, deadline):
    """Spool output, cap in-memory reads, kill/wait this exact child on refusal.

    File-size sampling is a refusal guard, not a kernel-enforced disk quota.
    No pipes need concurrent draining and no descendant is intentionally spawned.
    """
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise Refused("batch deadline")
    # This candidate is local-only. A partial clone must not spawn a fetch.
    if any(name in os.environ for name in
           ("GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY",
            "GIT_ALTERNATE_OBJECT_DIRECTORIES", "GIT_NAMESPACE")):
        raise Refused("repository-overriding environment")
    with tempfile.TemporaryFile() as source, tempfile.TemporaryFile() as stdout, \
            tempfile.TemporaryFile() as stderr:
        source.write(data)
        source.seek(0)
        child = subprocess.Popen(
            ["git", "--no-replace-objects", *args], cwd=root,
            stdin=source, stdout=stdout, stderr=stderr,
            env=dict(os.environ, GIT_NO_LAZY_FETCH="1"),
        )
        try:
            while True:
                if (os.fstat(stdout.fileno()).st_size > MAX_OUTPUT
                        or os.fstat(stderr.fileno()).st_size > MAX_STDERR):
                    raise Refused("Git output budget")
                if child.poll() is not None:
                    break
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise Refused("batch deadline")
                try:
                    child.wait(timeout=min(0.02, remaining))
                except subprocess.TimeoutExpired:
                    pass
            stdout.seek(0)
            stderr.seek(0)
            out, err = stdout.read(MAX_OUTPUT + 1), stderr.read(MAX_STDERR + 1)
            if len(out) > MAX_OUTPUT or len(err) > MAX_STDERR:
                raise Refused("Git output budget")
            return Output(out, err, child.returncode)
        finally:
            if child.poll() is None:
                child.kill()
            child.wait(timeout=5)


def _record(raw, offset, spec):
    end = raw.find(b"\n", offset, offset + MAX_INPUT + 100)
    if end < 0:
        raise Refused("truncated or oversized batch header")
    header = raw[offset:end]
    if header == spec + b" missing":
        return None, end + 1
    parts = header.split(b" ")
    if (len(parts) != 3 or not re.fullmatch(b"[0-9a-f]{40}", parts[0])
            or parts[1] not in (b"blob", b"tree", b"commit", b"tag")
            or not re.fullmatch(b"0|[1-9][0-9]*", parts[2])
            or len(parts[2]) > 8):
        raise Refused("unexpected batch header")
    count = int(parts[2])
    start, stop = end + 1, end + 1 + count
    if count > MAX_OUTPUT or stop >= len(raw) or raw[stop:stop + 1] != b"\n":
        raise Refused("truncated, oversized or unterminated batch body")
    body = raw[start:stop]
    digest = hashlib.sha1(parts[1] + b" " + parts[2] + b"\0" + body).hexdigest()
    if digest.encode("ascii") != parts[0]:
        raise Refused("object content mismatch")
    return (parts[0], parts[1], body), stop + 1


def read_batch(root, requests, *, run=run_git):
    """Return complete ordered show-shaped results for one preplanned batch.

    Every request is preserved, including duplicates. Commit metadata is checked
    in the same batch process; there is no revision resolution or mutable-ref cache.
    Missing/non-blob requests delegate to the exact original show invocation.
    Protocol/process failures refuse the whole call without fallback or retry.
    """
    if not isinstance(requests, (tuple, list)) or len(requests) > MAX_REQUESTS:
        raise Refused("request budget")
    requests = tuple(requests)
    if any(not isinstance(request, Request) for request in requests):
        raise Refused("Request required")
    specs = [request.spec() for request in requests]
    if not specs:
        return []
    revisions = list(dict.fromkeys(request.revision.encode("ascii") for request in requests))
    data = b"\n".join([*revisions, *specs]) + b"\n"
    if len(data) > MAX_INPUT:
        raise Refused("input budget")
    root = Path(root).resolve(strict=True)
    if not root.is_dir():
        raise Refused("repository directory required")
    deadline = time.monotonic() + BOUND_SECONDS
    result = run(root, ["cat-file", "--batch"], data, deadline)
    if result.returncode != 0 or result.stderr:
        raise Refused("batch process failed or emitted diagnostics")
    if len(result.stdout) > MAX_OUTPUT:
        raise Refused("batch output budget")
    offset = 0
    for revision in revisions:
        record, offset = _record(result.stdout, offset, revision)
        if record is None or record[:2] != (revision, b"commit"):
            raise Refused("revision is not the exact existing commit")
    records = []
    for spec in specs:
        record, offset = _record(result.stdout, offset, spec)
        records.append(record)
    if offset != len(result.stdout):
        raise Refused("extra batch output")
    outputs = []
    output_bytes, error_bytes = 0, 0
    for spec, record in zip(specs, records):
        if time.monotonic() >= deadline:
            raise Refused("batch deadline")
        if record is not None and record[1] == b"blob":
            output = Output(record[2], b"", 0)
        else:
            output = run(root, ["show", spec.decode("utf-8")], b"", deadline)
        output_bytes += len(output.stdout)
        error_bytes += len(output.stderr)
        if output_bytes > MAX_OUTPUT or error_bytes > MAX_STDERR:
            raise Refused("aggregate result budget")
        outputs.append(output)
    return outputs
