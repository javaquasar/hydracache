"""Owned copied-packet integrity cases only; no refetch or retained packet edits."""
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
spec = importlib.util.spec_from_file_location("roster_packet_verifier", PACKET / "verify_packet.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class PacketTests(unittest.TestCase):
    def check(self, mutation=None):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary) / "copied-roster-packet"
            shutil.copytree(PACKET, copied, ignore=shutil.ignore_patterns("__pycache__"))
            if mutation:
                mutation(copied)
            with patch.object(verifier, "PACKET", copied), redirect_stdout(io.StringIO()):
                verifier.main()

    def mutate_manifest(self, change):
        def mutate(copied):
            manifest = json.loads((copied / "manifest.json").read_bytes())
            change(manifest)
            (copied / "manifest.json").write_text(json.dumps(manifest))
        return mutate

    def test_unmodified_control(self):
        self.check()

    def test_changed_raw_capture_refuses(self):
        def mutate(copied):
            (copied / "oidc-roster-development.log").write_bytes(b"forged\n")
        with self.assertRaises(AssertionError):
            self.check(mutate)

    def test_future_manifest_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(self.mutate_manifest(lambda m: m.update(schema_version=2)))

    def test_boolean_is_not_manifest_version_one(self):
        with self.assertRaises(AssertionError):
            self.check(self.mutate_manifest(lambda m: m.update(schema_version=True)))

    def test_admission_flag_cannot_become_true(self):
        def change(m):
            m["admission_claims"]["authenticated_original_start"] = True
        with self.assertRaises(AssertionError):
            self.check(self.mutate_manifest(change))

    def test_source_digest_change_refuses(self):
        def change(m):
            m["source_files"][0]["sha256_lf"] = "0" * 64
        with self.assertRaises(AssertionError):
            self.check(self.mutate_manifest(change))


if __name__ == "__main__":
    unittest.main()
