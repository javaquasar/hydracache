"""Mutate only owned copies of evidence; no key, network or host action."""
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
spec = importlib.util.spec_from_file_location("signer_packet_verifier", PACKET / "verify_packet.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class PacketTests(unittest.TestCase):
    def check(self, mutation=None):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary) / "copied-signer-model"
            shutil.copytree(PACKET, copied, ignore=shutil.ignore_patterns("__pycache__"))
            if mutation:
                mutation(copied)
            with patch.object(verifier, "PACKET", copied), redirect_stdout(io.StringIO()):
                verifier.main()

    def change(self, mutation):
        def apply(copied):
            manifest = json.loads((copied / "manifest.json").read_bytes())
            mutation(manifest)
            (copied / "manifest.json").write_text(json.dumps(manifest))
        return apply

    def test_control(self):
        self.check()

    def test_future_and_boolean_version_refuse(self):
        for version in (2, True):
            with self.subTest(version=version), self.assertRaises(AssertionError):
                self.check(self.change(lambda m: m.update(schema_version=version)))

    def test_source_hash_substitution_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m: m["source_files"][0].update(sha256_lf="0" * 64)))

    def test_capture_mutation_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(lambda p: (p / "signer-build-missing-api.log").write_bytes(b"changed\n"))

    def test_bootstrap_claim_and_empty_claims_refuse(self):
        for mutation in [lambda m: m["admission_claims"].update(bootstrap_authorized=True),
                         lambda m: m.update(admission_claims={})]:
            with self.assertRaises(AssertionError):
                self.check(self.change(mutation))

    def test_capture_exit_substitution_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m: m["captures"][0].update(observed_outer_exit=99)))


if __name__ == "__main__":
    unittest.main()
