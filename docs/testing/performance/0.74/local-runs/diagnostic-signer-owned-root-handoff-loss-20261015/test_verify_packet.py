"""Mutation checks validate only the unsigned consistency verifier."""
import copy
import json
import unittest
from unittest.mock import patch
import verify_packet as v

class PacketTests(unittest.TestCase):
    def test_real_packet(self):
        v.verify()

    def test_manifest_pin_rejects_changed_bytes(self):
        raw = (v.PACKET / "manifest.json").read_bytes()
        with self.assertRaises(ValueError):
            v.manifest(raw + b" ")

    def test_rehashed_manifest_cannot_open_claims_or_change_identity(self):
        data = json.loads((v.PACKET / "manifest.json").read_bytes())
        for key, value in [("source_commit", "0" * 40), ("msrv_linux_lint_executed", True),
                           ("msrv_windows_lint_passed", True),
                           ("all_checks_passed", True), ("execution_attested", True),
                           ("amendment", "0" * 40)]:
            bad = copy.deepcopy(data)
            bad[key] = value
            raw = json.dumps(bad).encode()
            with patch.object(v, "MANIFEST_SHA256", v.digest(raw)), self.assertRaises(ValueError):
                v.manifest(raw)
        for value in (True, 0, "false"):
            bad = copy.deepcopy(data)
            bad["closed_claims"]["ship_allowed"] = value
            raw = json.dumps(bad).encode()
            with patch.object(v, "MANIFEST_SHA256", v.digest(raw)), self.assertRaises(ValueError):
                v.manifest(raw)

    def test_rehashed_log_cannot_promote_negative_cleanup_or_drop_cut(self):
        data = json.loads((v.PACKET / "manifest.json").read_bytes())
        original = (v.PACKET / "current-1.log").read_bytes()
        for before, after in [
            (b"kind=no-exitkill cut=installed sends_at_cut=0 automatic_death=false", b"kind=no-exitkill cut=installed sends_at_cut=0 automatic_death=true"),
            (b"kind=loss cut=receive sends_at_cut=0", b"kind=loss cut=receive sends_at_cut=1"),
            (b"kind=loss cut=closed", b"kind=loss cut=acked"),
            (b"43 passed; 0 failed", b"42 passed; 0 failed"),
        ]:
            raw = original.replace(before, after)
            self.assertNotEqual(raw, original)
            expected = copy.deepcopy(data["captures"]["current-1.log"])
            expected.update(sha256=v.digest(raw), bytes=len(raw), results=v.results(raw), required_markers=[])
            with self.assertRaises(ValueError):
                v.capture("current-1.log", raw, expected)

    def test_capture_completeness_and_bytes_are_checked(self):
        data = json.loads((v.PACKET / "manifest.json").read_bytes())
        raw = (v.PACKET / "current-1.log").read_bytes()
        for key, value in [("capture_complete", False), ("bytes", 0), ("sha256", "0" * 64)]:
            expected = copy.deepcopy(data["captures"]["current-1.log"])
            expected[key] = value
            with self.assertRaises(ValueError):
                v.capture("current-1.log", raw, expected)

    def test_msrv_refusal_cannot_be_relabelled_as_runtime_acceptance(self):
        data = json.loads((v.PACKET / "manifest.json").read_bytes())
        raw = (v.PACKET / "msrv.log").read_bytes() + v.transfer_markers()[0].encode() + b"\n"
        expected = copy.deepcopy(data["captures"]["msrv.log"])
        expected.update(sha256=v.digest(raw), bytes=len(raw))
        with self.assertRaises(ValueError):
            v.capture("msrv.log", raw, expected)

    def test_body_extraction_keeps_original_guard(self):
        self.assertEqual(v.body("fn output() { if x { y } z }", "fn output("), " if x { y } z ")

if __name__ == "__main__":
    unittest.main()
