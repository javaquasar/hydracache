"""Mutation checks against the local evidence verifier, not host authority."""
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
spec = importlib.util.spec_from_file_location("thread_packet", HERE/"verify_packet.py")
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)

class PacketTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.packet = Path(self.temp.name)/"packet"
        shutil.copytree(HERE, self.packet, ignore=shutil.ignore_patterns("__pycache__"))
        self.scope = patch.object(v, "PACKET", self.packet)
        self.scope.start()

    def tearDown(self):
        self.scope.stop()
        self.temp.cleanup()

    def manifest(self):
        return json.loads((self.packet/"manifest.json").read_bytes())

    def save(self, m):
        (self.packet/"manifest.json").write_text(json.dumps(m), encoding="utf-8")

    def refuse(self):
        with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(AssertionError):
            v.main()

    def rehash(self, name):
        m = self.manifest()
        raw = (self.packet/name).read_bytes()
        row = next(r for r in m["captures"] if r["path"] == name)
        row["sha256"], row["bytes"] = v.sha(raw), len(raw)
        self.save(m)

    def test_valid(self):
        with contextlib.redirect_stdout(io.StringIO()):
            v.main()

    def test_raw_drift(self):
        with (self.packet/"thread-exit-baseline.log").open("ab") as f:
            f.write(b"drift")
        self.refuse()

    def test_rehashed_bad_terminal(self):
        name = "thread-exit-linux-final-2.log"
        p = self.packet/name
        p.write_bytes(p.read_bytes().replace(b"code=2 status=9", b"code=1 status=9", 1))
        self.rehash(name)
        self.refuse()

    def test_rehashed_false_tgid_lifetime(self):
        name = "thread-exit-linux-final-2.log"
        p = self.packet/name
        p.write_bytes(p.read_bytes().replace(b"original_tgid_alive_at_child_terminal=true", b"original_tgid_alive_at_child_terminal=false", 1))
        self.rehash(name)
        self.refuse()

    def test_rehashed_zero_suite(self):
        name = "thread-exit-linux-final-2.log"
        p = self.packet/name
        p.write_bytes(p.read_bytes().replace(b"test result: ok. 18 passed;", b"test result: ok. 0 passed;", 1))
        self.rehash(name)
        self.refuse()

    def test_negative_cannot_be_promoted(self):
        name = "thread-exit-linux-final-2.log"
        p = self.packet/name
        p.write_bytes(p.read_bytes().replace(b"dummy_sends=0 positive=false", b"dummy_sends=0 positive=true", 1))
        self.rehash(name)
        self.refuse()

    def test_production_flag_and_source_forgery(self):
        m = self.manifest()
        m["admission_claims"]["server_entry_wired"] = True
        self.save(m)
        self.refuse()
        m["admission_claims"]["server_entry_wired"] = False
        m["source_files"][0]["sha256_lf"] = "0"*64
        self.save(m)
        self.refuse()

    def test_missing_capture_and_duplicate_json(self):
        m = self.manifest()
        m["captures"].pop()
        self.save(m)
        self.refuse()
        (self.packet/"manifest.json").write_bytes(b'{"schema_version":1,"schema_version":1}')
        self.refuse()

    def test_attributes_and_exact_committed_bytes(self):
        (self.packet/".gitattributes").write_bytes(b"* text\n")
        self.refuse()
        with patch.object(v, "ROOT", self.packet.parent), patch.object(v, "committed", return_value=b"not original bytes"), self.assertRaises(AssertionError):
            v.verify_committed(v.SOURCE)

if __name__ == "__main__":
    unittest.main()
