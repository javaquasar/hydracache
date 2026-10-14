"""Mutate only owned packet copies, not source or installed artifacts."""
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
spec = importlib.util.spec_from_file_location("rooted_verifier", PACKET / "verify_packet.py")
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)


class PacketTests(unittest.TestCase):
    def check(self, mutation=None):
        with tempfile.TemporaryDirectory() as temporary:
            copied = Path(temporary) / "copied-rooted-exec"
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

    def test_git_attributes_and_committed_byte_substitution_refuse(self):
        with self.assertRaises(AssertionError):
            self.check(lambda p: (p / ".gitattributes").write_bytes(b"* text\n"))
        with patch.object(verifier, "committed", return_value=b"substituted\n"):
            with self.assertRaises(AssertionError):
                verifier.verify_committed("0" * 40)

    def test_future_boolean_version_and_source_refuse(self):
        for mutation in [lambda m: m.update(schema_version=2),
                         lambda m: m.update(schema_version=True),
                         lambda m: m.update(source_commit="0" * 40)]:
            with self.assertRaises(AssertionError):
                self.check(self.change(mutation))

    def test_source_hash_and_empty_sources_refuse(self):
        for mutation in [lambda m: m["source_files"][0].update(sha256_lf="0" * 64),
                         lambda m: m.update(source_files=[])]:
            with self.assertRaises(AssertionError):
                self.check(self.change(mutation))

    def test_credential_claim_or_empty_admission_refuse(self):
        for mutation in [lambda m: m["admission_claims"].update(actual_private_key_or_token_read=True),
                         lambda m: m.update(admission_claims={})]:
            with self.assertRaises(AssertionError):
                self.check(self.change(mutation))

    def test_raw_red_log_or_exit_substitution_refuse(self):
        with self.assertRaises(AssertionError):
            self.check(lambda p: (p / "rooted-exec-linux-check-lint.log").write_bytes(b"changed\n"))
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m: m["captures"][0].update(observed_outer_exit=99)))

    def test_empty_unchanged_or_qualification_substitution_refuse(self):
        for mutation in [lambda m: m.update(unchanged_files=[]),
                         lambda m: m.update(qualification_sha256_raw="0" * 64)]:
            with self.assertRaises(AssertionError):
                self.check(self.change(mutation))

    def test_removed_frozen_binding_refuses(self):
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m: m["unchanged_files"].pop()))

    def test_matching_but_wrong_expected_negative_exit_refuses(self):
        def mutate(manifest):
            capture = next(c for c in manifest["captures"]
                           if c["path"] == "rooted-exec-linux-check-lint.log")
            capture.update(expected_outer_exit=0, observed_outer_exit=0)
        with self.assertRaises(AssertionError):
            self.check(self.change(mutate))

    def test_removed_positive_capture_or_helper_identity_refuses(self):
        def remove(copied):
            manifest = json.loads((copied / "manifest.json").read_bytes())
            name = "rooted-exec-linux-msrv.log"
            manifest["captures"] = [c for c in manifest["captures"] if c["path"] != name]
            (copied / name).unlink()
            (copied / "manifest.json").write_text(json.dumps(manifest))
        with self.assertRaises(AssertionError):
            self.check(remove)
        with self.assertRaises(AssertionError):
            self.check(self.change(lambda m: m.update(fixture_helper_sha256="0" * 64)))


if __name__ == "__main__":
    unittest.main()
