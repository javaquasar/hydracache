"""Mutate only owned temporary packet copies."""
from contextlib import redirect_stdout
import importlib.util
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

PACKET = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("ctor_verifier",PACKET/"verify_packet.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)

class PacketTests(unittest.TestCase):
    def check(self, mutation=None, committed_change=False):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary)/"copied-constructor"
            shutil.copytree(PACKET,copied,ignore=shutil.ignore_patterns("__pycache__"))
            if mutation:
                mutation(copied)
            original = verifier.committed
            def objects(commit,path):
                data = original(commit,path)
                if committed_change and commit == verifier.BASELINE and path.endswith("diagnostic_signer_tracer_loss_channel.rs"):
                    return data+b"changed parser body\n"
                return data
            with patch.object(verifier,"PACKET",copied), patch.object(verifier,"committed",objects), redirect_stdout(io.StringIO()):
                verifier.main()

    def change(self, mutation):
        def apply(copied):
            m = json.loads((copied/"manifest.json").read_bytes())
            mutation(m)
            (copied/"manifest.json").write_text(json.dumps(m))
        return apply

    def rewrite_capture(self, name, before, after):
        def apply(copied):
            path = copied/name
            raw = path.read_bytes()
            assert before in raw
            raw = raw.replace(before,after,1)
            path.write_bytes(raw)
            m = json.loads((copied/"manifest.json").read_bytes())
            row = next(c for c in m["captures"] if c["path"] == name)
            row.update(bytes=len(raw),sha256=verifier.sha(raw))
            (copied/"manifest.json").write_text(json.dumps(m))
        return apply

    def test_control(self):
        self.check()

    def test_raw_capture_drift(self):
        with self.assertRaises(AssertionError):
            self.check(lambda p:(p/"constructor-loss-baseline.log").write_bytes(b"altered"))

    def test_self_consistent_bad_death(self):
        with self.assertRaises(AssertionError):
            self.check(self.rewrite_capture("constructor-loss-final-verified.log",b"code=2 status=9",b"code=1 status=9"))

    def test_empty_suite(self):
        with self.assertRaises(AssertionError):
            self.check(self.rewrite_capture("constructor-loss-final-verified.log",b"18 passed; 0 failed",b"0 passed; 0 failed"))

    def test_missing_capture(self):
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m:m["captures"].pop()))

    def test_source_identity(self):
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m:m.update(source_commit="0"*40)))

    def test_closed_admission(self):
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m:m["admission_claims"].update(pre_arm_creating_thread_only_loss_proved=True)))

    def test_original_parser_body(self):
        with self.assertRaises(AssertionError):
            self.check(committed_change=True)

    def test_git_attributes(self):
        with self.assertRaises(AssertionError):
            self.check(lambda p:(p/".gitattributes").write_bytes(b"* text\n"))

    def test_committed_bytes(self):
        relative = PACKET.relative_to(verifier.ROOT)
        def objects(commit,path):
            return (PACKET/Path(path).relative_to(relative)).read_bytes()
        with patch.object(verifier,"committed",objects), redirect_stdout(io.StringIO()):
            verifier.verify_committed("1"*40)
        def altered(commit,path):
            data = objects(commit,path)
            return data+b"\n" if path.endswith("README.md") else data
        with patch.object(verifier,"committed",altered), self.assertRaises(AssertionError):
            verifier.verify_committed("1"*40)

if __name__ == "__main__":
    unittest.main()
