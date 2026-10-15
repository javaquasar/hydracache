"""Mutate copied evidence only; no runtime inputs, keys or network access."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("catalog_packet", HERE / "verify_packet.py")
v = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(v)


class PacketTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.packet = Path(self.temp.name) / "packet"
        shutil.copytree(HERE, self.packet, ignore=shutil.ignore_patterns("__pycache__"))
        self.scope = patch.object(v, "PACKET", self.packet)
        self.scope.start()

    def tearDown(self):
        self.scope.stop()
        self.temp.cleanup()

    def mutate(self, name, action):
        path = self.packet / name
        value = json.loads(path.read_bytes())
        action(value)
        path.write_text(json.dumps(value), encoding="utf-8")
        manifest_path = self.packet / "manifest.json"
        manifest = json.loads(manifest_path.read_bytes())
        raw = path.read_bytes()
        manifest["files"][name] = {"bytes": len(raw), "sha256": v.sha(raw)}
        manifest_path.write_text(json.dumps(manifest), encoding="utf-8")

    def refuse(self):
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AssertionError):
            v.verify()

    def test_valid(self):
        with contextlib.redirect_stdout(io.StringIO()):
            v.verify()

    def test_raw_drift(self):
        with (self.packet / "baseline-linux.log").open("ab") as stream:
            stream.write(b"drift")
        self.refuse()

    def test_rehashed_open_or_numeric_flag(self):
        for value in (True, 0):
            self.mutate("actual-catalog.json", lambda c: c.update(signing_allowed=value))
            self.refuse()

    def test_rehashed_role(self):
        self.mutate("actual-catalog.json", lambda c: c["images"][1].update(role="libc.so.6"))
        self.refuse()

    def test_rehashed_digest(self):
        self.mutate("actual-catalog.json", lambda c: c["images"][0].update(sha256="0" * 64))
        self.refuse()

    def test_rehashed_size(self):
        self.mutate("actual-catalog.json", lambda c: c["images"][0].update(bytes=64))
        self.refuse()

    def test_rehashed_source(self):
        self.mutate("actual-status.json", lambda s: s.update(tooling_source="0" * 40))
        self.refuse()

    def test_rehashed_exit(self):
        self.mutate("actual-status.json", lambda s: s.update(negative_native_exit_code=0))
        self.refuse()

    def test_rehashed_inspector(self):
        self.mutate("actual-status.json", lambda s: s.update(inspector_sha256="0" * 64))
        self.refuse()

    def test_rehashed_schema(self):
        self.mutate("actual-catalog.json", lambda c: c.update(schema_version="runtime-authority"))
        self.refuse()


if __name__ == "__main__":
    unittest.main()
