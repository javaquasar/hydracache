#!/usr/bin/env python3
"""Verify materialized D4 LFS archives, identity receipts, and public-data hygiene."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import tarfile


ROOT = Path(__file__).resolve().parent
EXPECTED_SHA = "da8d6de409a657e0260e7fbfb4ab31d8d6ad5ca8"
FORBIDDEN_NAMES = ("hc2-pki", "/builds/", ".env", "id_rsa", "authorized_keys")
FORBIDDEN_TEXT = [
    re.compile(rb"-----BEGIN (?:RSA |EC |OPENSSH |ED25519 )?PRIVATE KEY-----"),
    re.compile(rb"github_pat_[A-Za-z0-9_]+|ghp_[A-Za-z0-9]+"),
    re.compile(rb"(?i)(?:[0-9a-f]{2}:){5}[0-9a-f]{2}"),
    re.compile(rb"(?<![0-9])(?:[0-9]{1,3}\.){3}[0-9]{1,3}(?![0-9])"),
    re.compile(rb"(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"),
]


def verify_archive(path: Path) -> int:
    count = 0
    with tarfile.open(path, "r:gz") as archive:
        for member in archive:
            name = member.name.replace("\\", "/")
            parts = PurePosixPath(name).parts
            assert not name.startswith("/") and ".." not in parts, name
            assert not any(bad in f"/{name.lower()}/" for bad in FORBIDDEN_NAMES), name
            assert not member.issym() and not member.islnk(), name
            if member.isfile():
                stream = archive.extractfile(member)
                assert stream is not None, name
                payload = stream.read()
                assert not any(pattern.search(payload) for pattern in FORBIDDEN_TEXT), name
            count += 1
    return count


def verify_receipts(manifest: dict) -> None:
    for item in manifest["campaigns"]:
        root = ROOT / "campaigns" / item["id"] / "github-artifact"
        receipts = list(root.rglob("campaign-receipt.json"))
        assert len(receipts) == 1, (item["id"], receipts)
        receipt = json.loads(receipts[0].read_text(encoding="utf-8"))
        assert receipt["campaign_id"] == item["id"]
        assert receipt["source_sha"] == EXPECTED_SHA
        assert receipt["workflow_sha"] == EXPECTED_SHA
        assert receipt["result"] == "success"
        assert receipt["ship_evidence_eligible"] is True
        artifact_files = list(root.rglob("*"))
        assert any(path.is_file() for path in artifact_files), item["id"]
        for path in artifact_files:
            if path.is_file():
                payload = path.read_bytes()
                assert not any(pattern.search(payload) for pattern in FORBIDDEN_TEXT), path


def main() -> None:
    manifest = json.loads((ROOT / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["source_sha"] == EXPECTED_SHA
    assert manifest["workflow_sha"] == EXPECTED_SHA
    assert len(manifest["campaigns"]) == 4
    checksums = []
    for line in (ROOT / "SHA256SUMS").read_text(encoding="ascii").splitlines():
        digest, relative = line.split("  ", 1)
        path = ROOT / relative
        assert path.is_file(), path
        actual = hashlib.sha256(path.read_bytes()).hexdigest()
        assert actual == digest, (relative, digest, actual)
        checksums.append(relative)
        count = verify_archive(path)
        print(f"archive OK: {relative} ({count} members)")
    expected_archives = {item["archive"] for item in manifest["campaigns"]}
    expected_archives.add(manifest["host_state_archive"])
    assert set(checksums) == expected_archives
    verify_receipts(manifest)
    print("identity, hashes, archive readability, and hygiene: PASS")


if __name__ == "__main__":
    main()
