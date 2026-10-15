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

    def save(self):
        (self.path / "manifest.json").write_text(json.dumps(self.manifest))

    def replace(self, name, old, new, rehash=True):
        raw = (self.path / name).read_bytes()
        assert old in raw
        raw = raw.replace(old, new)
        (self.path / name).write_bytes(raw)
        if rehash:
            self.manifest["files"][name].update(bytes=len(raw), sha256=packet.sha(raw))
        self.save()

    def verify(self):
        with patch.object(packet, "PACKET", self.path), contextlib.redirect_stdout(io.StringIO()):
            packet.verify()

    def refused(self):
        with self.assertRaises(AssertionError):
            self.verify()

    def test_original(self):
        self.verify()

    def test_raw_drift(self):
        self.replace("joint-parallel-final.log", b"classification_cases=25", b"classification_cases=0", False)
        self.refused()

    def test_rehashed_zero_tests(self):
        self.replace("joint-parallel-final.log", b"test result: ok. 17 passed;", b"test result: ok. 0 passed;")
        self.refused()

    def test_rehashed_missing_joint_model(self):
        self.replace("joint-parallel-final.log", b"running_never_accepted=true", b"running_never_accepted=false")
        self.refused()

    def test_negative_cleanup_promoted(self):
        self.replace("linux-msrv-final.log", b"child_signaled=true dummy_sends=0 positive=false", b"child_signaled=true dummy_sends=0 positive=true")
        self.refused()

    def test_erased_intermediate_failure(self):
        self.replace("group-only-repeats.log", b"test result: FAILED. 15 passed; 1 failed;", b"test result: ok. 16 passed; 0 failed;")
        self.refused()

    def test_missing_negative_capture(self):
        del self.manifest["files"]["pending-signal-exploration.log"]
        self.save()
        self.refused()

    def test_false_admission(self):
        self.manifest["closed_claims"]["ship_allowed"] = True
        self.save()
        self.refused()

    def test_wrong_failed_exit(self):
        self.manifest["files"]["group-only-repeats.log"]["outer_exit"] = 0
        self.save()
        self.refused()

    def test_changed_source_digest(self):
        first = next(iter(self.manifest["implementation_files"]))
        self.manifest["implementation_files"][first] = "0" * 64
        self.save()
        self.refused()

    def test_missing_historical_negative(self):
        del self.manifest["prior_negative_evidence"][packet.PRIOR[0]]
        self.save()
        self.refused()

    def test_duplicate_manifest_field(self):
        raw = json.dumps(self.manifest)
        (self.path / "manifest.json").write_text('{"schema_version":"forged",' + raw[1:])
        self.refused()


if __name__ == "__main__":
    unittest.main()
