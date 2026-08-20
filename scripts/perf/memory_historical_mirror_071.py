#!/usr/bin/env python3
"""Create and restore-verify the protected 0.71 historical input mirror."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


COMMIT = "dbc2f82f7f303528b3cca7842818730c82232b9c"
TAG = "explore-0.67-telemetry-20260803"
BRANCH = "explore/0.67-telemetry-hazelcast"
ARCHIVE_PATHS = ["results", "docs/testing/perf-scenarios/0.67"]


class MirrorError(RuntimeError):
    pass


def sha256_bytes(value: bytes) -> str:
    return "sha256:" + hashlib.sha256(value).hexdigest()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return "sha256:" + digest.hexdigest()


def manifest_from_archive(path: Path) -> list[dict[str, Any]]:
    files = []
    with tarfile.open(path, "r:gz") as archive:
        for member in sorted(archive.getmembers(), key=lambda item: item.name):
            if not member.isfile():
                continue
            stream = archive.extractfile(member)
            if stream is None:
                raise MirrorError(f"archive member cannot be read: {member.name}")
            content = stream.read()
            if not content:
                raise MirrorError(f"historical archive contains an empty raw file: {member.name}")
            files.append({"path": member.name, "bytes": len(content), "sha256": sha256_bytes(content)})
    if not files:
        raise MirrorError("historical archive contains no files")
    return files


def manifest_digest(files: list[dict[str, Any]]) -> str:
    canonical = json.dumps(files, separators=(",", ":"), sort_keys=True).encode()
    return sha256_bytes(canonical)


def restored_manifest(path: Path) -> list[dict[str, Any]]:
    with tempfile.TemporaryDirectory(prefix="hydracache-memory-history-") as directory:
        root = Path(directory)
        with tarfile.open(path, "r:gz") as archive:
            for member in archive.getmembers():
                destination = (root / member.name).resolve()
                if not destination.is_relative_to(root.resolve()):
                    raise MirrorError(f"unsafe archive member: {member.name}")
            archive.extractall(root, filter="data")
        files = []
        for item in sorted(root.rglob("*")):
            if item.is_file():
                content = item.read_bytes()
                files.append(
                    {
                        "path": str(item.relative_to(root)).replace("\\", "/"),
                        "bytes": len(content),
                        "sha256": sha256_bytes(content),
                    }
                )
        return files


def git(root: Path, *arguments: str) -> str:
    return subprocess.run(
        ["git", *arguments], cwd=root, check=True, capture_output=True, text=True
    ).stdout.strip()


def atomic_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    if path.exists():
        temporary.unlink()
        raise MirrorError(f"refusing to overwrite receipt: {path}")
    os.replace(temporary, path)


def create(args: argparse.Namespace) -> None:
    root = Path(git(Path.cwd(), "rev-parse", "--show-toplevel")).resolve()
    mirror_root = args.mirror_root.resolve()
    if mirror_root == root or mirror_root.is_relative_to(root):
        raise MirrorError("protected mirror must be outside the ordinary Git worktree")
    if not args.approve_protected_mirror:
        raise MirrorError("creation requires explicit --approve-protected-mirror")
    if git(root, "rev-list", "-n", "1", TAG) != COMMIT:
        raise MirrorError("historical tag does not resolve to the frozen commit")
    if git(root, "rev-parse", f"origin/{BRANCH}") != COMMIT:
        raise MirrorError("historical remote branch does not resolve to the frozen commit")
    mirror_root.mkdir(parents=True, exist_ok=True)
    archive = mirror_root / f"hydracache-memory-history-{COMMIT}.tar.gz"
    if archive.exists():
        raise MirrorError(f"refusing to overwrite protected object: {archive}")
    staging = mirror_root / f".{archive.name}.{os.getpid()}.partial"
    command = ["git", "archive", "--format=tar.gz", "-o", str(staging), COMMIT, "--", *ARCHIVE_PATHS]
    subprocess.run(command, cwd=root, check=True)
    files = manifest_from_archive(staging)
    first_manifest = manifest_digest(files)
    restored = restored_manifest(staging)
    restored_digest = manifest_digest(restored)
    if files != restored or first_manifest != restored_digest:
        staging.unlink(missing_ok=True)
        raise MirrorError("restored historical mirror manifest differs from the source archive")
    os.replace(staging, archive)
    archive.chmod(0o444)
    receipt = {
        "schema_version": 1,
        "release": "0.71",
        "branch": BRANCH,
        "tag": TAG,
        "commit": COMMIT,
        "checkout_clean": True,
        "files": files,
        "mirror": {
            "provider": args.provider,
            "object_id": str(archive),
            "archive_sha256": sha256_file(archive),
            "byte_length": archive.stat().st_size,
            "manifest_sha256": first_manifest,
            "restored_manifest_sha256": restored_digest,
            "verified_at": datetime.now(timezone.utc).isoformat(),
            "retention_deadline": args.retention_deadline,
        },
    }
    atomic_json(args.output, receipt)
    print(f"historical mirror verified: {archive}")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--mirror-root", type=Path, required=True)
    parser.add_argument("--provider", required=True)
    parser.add_argument("--retention-deadline", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--approve-protected-mirror", action="store_true")
    return parser.parse_args()


def main() -> int:
    try:
        create(parse_args())
        return 0
    except (MirrorError, OSError, ValueError, subprocess.CalledProcessError, tarfile.TarError) as error:
        print(f"memory historical mirror 0.71: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
