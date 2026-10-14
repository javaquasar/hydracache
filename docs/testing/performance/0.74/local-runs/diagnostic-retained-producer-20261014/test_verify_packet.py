"""Owned copied evidence only; never edit originals or refetch."""
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
spec = importlib.util.spec_from_file_location("retained_packet_verifier", PACKET / "verify_packet.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class PacketTests(unittest.TestCase):
    def check(self, mutation=None):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary) / "copied-retained-producer"
            shutil.copytree(PACKET, copied, ignore=shutil.ignore_patterns("__pycache__"))
            if mutation:
                mutation(copied)
            with patch.object(verifier, "PACKET", copied), redirect_stdout(io.StringIO()):
                verifier.main()

    def change_manifest(self, change):
        def mutate(copied):
            manifest = json.loads((copied / "manifest.json").read_bytes())
            change(manifest)
            (copied / "manifest.json").write_text(json.dumps(manifest))
        return mutate

    def test_control(self):
        self.check()

    def test_future_or_boolean_version_refuses(self):
        for version in (2, True):
            with self.subTest(version=version), self.assertRaises(AssertionError):
                self.check(self.change_manifest(lambda m: m.update(schema_version=version)))

    def test_raw_capture_mutation_refuses(self):
        def mutate(copied):
            (copied / "retained-producer-development.log").write_bytes(b"changed\n")
        with self.assertRaises(AssertionError):
            self.check(mutate)

    def test_bootstrap_admission_cannot_be_set_true(self):
        def change(m):
            m["admission_claims"]["signer_bootstrap_custody_authenticated"] = True
        with self.assertRaises(AssertionError):
            self.check(self.change_manifest(change))

    def test_source_digest_mutation_refuses(self):
        def change(m):
            m["source_files"][0]["sha256_lf"] = "0" * 64
        with self.assertRaises(AssertionError):
            self.check(self.change_manifest(change))


if __name__ == "__main__":
    unittest.main()
