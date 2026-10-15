"""Mutation checks of public local evidence only; never touch private keys."""
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
spec = importlib.util.spec_from_file_location("custody_packet",HERE/"verify_packet.py")
v = importlib.util.module_from_spec(spec)
spec.loader.exec_module(v)
class PacketTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.packet = Path(self.temp.name)/"packet"
        shutil.copytree(HERE,self.packet,ignore=shutil.ignore_patterns("__pycache__"))
        self.scope = patch.object(v,"PACKET",self.packet)
        self.scope.start()
    def tearDown(self):
        self.scope.stop()
        self.temp.cleanup()
    def manifest(self):
        return json.loads((self.packet/"manifest.json").read_bytes())
    def save(self,m):
        (self.packet/"manifest.json").write_text(json.dumps(m),encoding="utf-8")
    def refuse(self):
        with contextlib.redirect_stdout(io.StringIO()),self.assertRaises(AssertionError):
            v.main()
    def rehash(self,name):
        m = self.manifest()
        row = next(r for r in m["captures"] if r["path"] == name)
        raw = (self.packet/name).read_bytes()
        row["sha256"],row["bytes"] = v.sha(raw),len(raw)
        self.save(m)
    def test_valid(self):
        with contextlib.redirect_stdout(io.StringIO()):
            v.main()
    def test_raw_drift(self):
        with (self.packet/"signer-custody-baseline.log").open("ab") as f:
            f.write(b"drift")
        self.refuse()
    def test_rehashed_zero_contract_suite(self):
        name = "signer-custody-contracts.log"
        p = self.packet/name
        p.write_bytes(p.read_bytes().replace(b"149 passed",b"0 passed",1))
        self.rehash(name)
        self.refuse()
    def test_rehashed_lost_public_proof(self):
        name = "signer-custody-public-proof-linux.log"
        p = self.packet/name
        p.write_bytes(p.read_bytes().replace(b"selfcheck=verified",b"selfcheck=refused",1))
        self.rehash(name)
        self.refuse()
    def test_generation_is_not_public_pin_approval(self):
        name = "signer-custody-generation.json"
        p = self.packet/name
        raw = p.read_bytes().replace(b'"public_pin_approved":false',b'"public_pin_approved":true',1)
        p.write_bytes(raw)
        self.rehash(name)
        self.refuse()
    def test_admission_and_source_forgery(self):
        m = self.manifest()
        m["admission_claims"]["production_trust_enrolled"] = True
        self.save(m)
        self.refuse()
        m["admission_claims"]["production_trust_enrolled"] = False
        m["source_files"][0]["sha256_lf"] = "0"*64
        self.save(m)
        self.refuse()
    def test_missing_capture(self):
        m = self.manifest()
        m["captures"].pop()
        self.save(m)
        self.refuse()
    def test_duplicate_json_and_attributes(self):
        original = (self.packet/"manifest.json").read_bytes()
        (self.packet/"manifest.json").write_bytes(b'{"schema_version":1,"schema_version":1}')
        self.refuse()
        (self.packet/"manifest.json").write_bytes(original)
        (self.packet/".gitattributes").write_bytes(b"* text\n")
        self.refuse()
    def test_exact_committed_bytes_refuse(self):
        with patch.object(v,"ROOT",self.packet.parent),patch.object(v,"committed",return_value=b"substitution"),self.assertRaises(AssertionError):
            v.verify_committed(v.SOURCE)
if __name__ == "__main__":
    unittest.main()
