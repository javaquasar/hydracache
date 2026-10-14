"""Owned copied-packet mutations only; never change retained evidence or refetch."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from contextlib import redirect_stdout

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
spec = importlib.util.spec_from_file_location("public_source_packet_verifier", PACKET / "verify_packet.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class PacketTests(unittest.TestCase):
    def check(self, mutation=None, rehash=False):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary) / "copied-public-packet"
            shutil.copytree(PACKET, copied, ignore=shutil.ignore_patterns("__pycache__"))
            if mutation:
                report = json.loads((copied / "acquisition-report.json").read_bytes())
                mutation(report)
                raw = json.dumps(report, sort_keys=True, separators=(",", ":")).encode() + b"\n"
                (copied / "acquisition-report.json").write_bytes(raw)
                if rehash:
                    manifest = json.loads((copied / "manifest.json").read_bytes())
                    entry = next(e for e in manifest["public_artifacts"] if e["path"] == "acquisition-report.json")
                    entry.update(bytes=len(raw), sha256=hashlib.sha256(raw).hexdigest())
                    status = json.loads((copied / "acquisition-status.json").read_bytes())
                    status["stdout_bytes"] = len(raw)
                    status_raw = json.dumps(status, sort_keys=True).encode()
                    (copied / "acquisition-status.json").write_bytes(status_raw)
                    status_entry = next(e for e in manifest["public_artifacts"] if e["path"] == "acquisition-status.json")
                    status_entry.update(bytes=len(status_raw), sha256=hashlib.sha256(status_raw).hexdigest())
                    (copied / "manifest.json").write_text(json.dumps(manifest))
            with redirect_stdout(io.StringIO()):
                verifier._verify(copied, ROOT)

    def test_unmodified_control(self):
        self.check()

    def test_raw_report_mutation_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(lambda r: r.update(key_selected=True))

    def test_rehashed_selection_cannot_become_admission(self):
        with self.assertRaises(AssertionError):
            self.check(lambda r: r.update(admission_allowed=True), rehash=True)

    def test_rehashed_future_schema_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(lambda r: r.update(schema_version="future"), rehash=True)

    def test_rehashed_foreign_origin_refuses(self):
        def change(r):
            r["transport"][0]["url"] = "https://foreign.invalid/source"
        with self.assertRaises(AssertionError):
            self.check(change, rehash=True)

    def test_rehashed_zero_is_not_false(self):
        def change(r):
            r["inspection"]["admission_allowed"] = 0
        with self.assertRaises(AssertionError):
            self.check(change, rehash=True)


if __name__ == "__main__":
    unittest.main()
