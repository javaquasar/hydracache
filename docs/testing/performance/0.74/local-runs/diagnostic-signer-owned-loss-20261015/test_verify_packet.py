import contextlib
import io
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch

import verify_packet as packet


class PacketTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.path = Path(self.temp.name) / "packet"
        shutil.copytree(packet.PACKET, self.path)
        self.manifest = json.loads((self.path / "manifest.json").read_bytes())

    def write_manifest(self):
        (self.path / "manifest.json").write_text(json.dumps(self.manifest))

    def replace(self, name, before, after, rehash=True):
        raw = (self.path / name).read_bytes()
        assert before in raw
        raw = raw.replace(before, after)
        (self.path / name).write_bytes(raw)
        if rehash:
            self.manifest["files"][name].update(bytes=len(raw), sha256=packet.sha(raw))
        self.write_manifest()

    def verify(self):
        with patch.object(packet, "PACKET", self.path), contextlib.redirect_stdout(io.StringIO()):
            packet.verify()

    def refused(self):
        with self.assertRaises(AssertionError):
            self.verify()

    def test_original_packet_with_explicit_red_regression(self):
        self.verify()

    def test_raw_log_drift(self):
        self.replace("linux-final.log", b"owner_loss_matrix=4", b"owner_loss_matrix=0", False)
        self.refused()

    def test_rehashed_missing_owner_loss(self):
        self.replace("linux-final.log", b"owner_loss first_exec=true healthy=false", b"owner_loss first_exec=false healthy=false")
        self.refused()

    def test_rehashed_child_signal(self):
        self.replace("linux-msrv.log", b"monitor_child_signaled=false", b"monitor_child_signaled=true")
        self.refused()

    def test_rehashed_zero_unit_tests(self):
        self.replace("linux-final.log", b"test result: ok. 12 passed;", b"test result: ok. 0 passed;")
        self.refused()

    def test_erased_legacy_failure_is_not_a_green_gate(self):
        self.replace("creator-parallel-failed.log", b"test result: FAILED. 13 passed; 1 failed;", b"test result: ok. 14 passed; 0 failed;")
        self.refused()

    def test_false_all_regressions_passed(self):
        self.manifest["closed_claims"]["all_regressions_passed"] = True
        self.write_manifest()
        self.refused()

    def test_false_server_admission(self):
        self.manifest["closed_claims"]["server_or_github_mutated"] = True
        self.write_manifest()
        self.refused()

    def test_changed_failed_exit(self):
        self.manifest["files"]["creator-parallel-failed.log"]["outer_exit"] = 0
        self.write_manifest()
        self.refused()

    def test_lost_negative_capture(self):
        del self.manifest["files"]["failed-self-stop-diagnostic.log"]
        self.write_manifest()
        self.refused()

    def test_reselected_normal_source(self):
        del self.manifest["unchanged_sources"]["tools/long-run-supervisor-074/src/diagnostic_signer_owned_probe.rs"]
        self.write_manifest()
        self.refused()

    def test_duplicate_manifest_key(self):
        raw = json.dumps(self.manifest)
        (self.path / "manifest.json").write_text('{"schema_version":"forged",' + raw[1:])
        self.refused()


if __name__ == "__main__":
    unittest.main()
