#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import io
import sys
import tarfile
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("memory_historical_mirror_071.py")
SPEC = importlib.util.spec_from_file_location("memory_historical_mirror_071", SCRIPT)
assert SPEC and SPEC.loader
mirror = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = mirror
SPEC.loader.exec_module(mirror)


class HistoricalMirror071Tests(unittest.TestCase):
    def test_archive_and_restored_manifests_match(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive_path = Path(directory) / "history.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                content = b"raw-telemetry\n"
                member = tarfile.TarInfo("results/raw.jsonl")
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
            source = mirror.manifest_from_archive(archive_path)
            restored = mirror.restored_manifest(archive_path)
            self.assertEqual(source, restored)
            self.assertEqual(mirror.manifest_digest(source), mirror.manifest_digest(restored))

    def test_empty_raw_file_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive_path = Path(directory) / "history.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                member = tarfile.TarInfo("results/empty.jsonl")
                member.size = 0
                archive.addfile(member, io.BytesIO())
            with self.assertRaises(mirror.MirrorError):
                mirror.manifest_from_archive(archive_path)

    def test_path_traversal_is_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            archive_path = Path(directory) / "history.tar.gz"
            with tarfile.open(archive_path, "w:gz") as archive:
                content = b"escape\n"
                member = tarfile.TarInfo("../outside.jsonl")
                member.size = len(content)
                archive.addfile(member, io.BytesIO(content))
            with self.assertRaises(mirror.MirrorError):
                mirror.restored_manifest(archive_path)


if __name__ == "__main__":
    unittest.main()
