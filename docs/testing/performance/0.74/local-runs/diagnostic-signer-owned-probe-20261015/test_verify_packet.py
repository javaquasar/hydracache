"""Copied-packet mutations only; no external inputs, keys or network."""
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
SPEC = importlib.util.spec_from_file_location("owned_probe_packet", HERE / "verify_packet.py")
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

    def manifest(self):
        return json.loads((self.packet / "manifest.json").read_bytes())

    def save(self, m):
        (self.packet / "manifest.json").write_text(json.dumps(m), encoding="utf-8")

    def refuse(self):
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AssertionError):
            v.verify()

    def test_valid(self):
        with contextlib.redirect_stdout(io.StringIO()):
            v.verify()

    def test_raw_drift(self):
        with (self.packet / "linux-final.log").open("ab") as stream:
            stream.write(b"drift")
        self.refuse()

    def test_rehashed_zero_suite(self):
        p = self.packet / "root-final.log"
        p.write_bytes(p.read_bytes().replace(b"157 passed", b"0 passed"))
        m = self.manifest()
        m["files"][p.name].update(bytes=p.stat().st_size, sha256=v.sha(p.read_bytes()))
        self.save(m)
        self.refuse()

    def test_open_or_numeric_claim(self):
        for value in (True, 0):
            m = self.manifest()
            m["closed_claims"]["independent_job_or_runtime_custody"] = value
            self.save(m)
            self.refuse()

    def test_source_and_missing_capture(self):
        m = self.manifest()
        m["source_commit"] = "0" * 40
        self.save(m)
        self.refuse()
        m["source_commit"] = v.SOURCE
        m["files"].pop("missing-binary.log")
        self.save(m)
        self.refuse()

    def test_false_exit(self):
        m = self.manifest()
        m["files"]["missing-binary.log"]["outer_exit"] = 0
        self.save(m)
        self.refuse()

    def test_duplicate_keys(self):
        (self.packet / "manifest.json").write_bytes(b'{"schema_version":1,"schema_version":1}')
        self.refuse()

    def test_capture_completeness_cannot_be_downgraded(self):
        m = self.manifest()
        m["files"]["linux-final.log"]["capture_complete"] = False
        self.save(m)
        self.refuse()

    def test_rehashed_writer_control_cannot_disappear(self):
        p = self.packet / "linux-final.log"
        p.write_bytes(p.read_bytes().replace(b"held_writer_exec=ETXTBSY", b"held_writer_exec=ignored"))
        m = self.manifest()
        m["files"][p.name].update(bytes=p.stat().st_size, sha256=v.sha(p.read_bytes()))
        self.save(m)
        self.refuse()

    def test_rehashed_failure_cannot_disappear(self):
        p = self.packet / "failed-linux.log"
        p.write_bytes(p.read_bytes().replace(b"Text file busy", b"ignored failure"))
        m = self.manifest()
        m["files"][p.name].update(bytes=p.stat().st_size, sha256=v.sha(p.read_bytes()))
        self.save(m)
        self.refuse()

    def test_independent_oracle_cannot_be_reselected(self):
        m = self.manifest()
        m["independent_oracles"][v.ORACLES[0]] = "0" * 64
        self.save(m)
        self.refuse()


if __name__ == "__main__":
    unittest.main()
