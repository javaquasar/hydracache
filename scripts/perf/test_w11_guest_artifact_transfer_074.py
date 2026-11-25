from hashlib import sha256
from io import BytesIO
from pathlib import Path
import shutil
import tempfile
import unittest
from zipfile import ZIP_STORED, ZipFile

from . import test_w11_build_workflow_074 as controlled
from . import w11_guest_artifact_transfer_074 as transfer


def prepare_fixture(root):
    trial = controlled.WorkflowArtifactTests()
    trial.setUp()
    try:
        source, incoming, _, archives, _ = trial.make_packet()
        shutil.copytree(incoming, root / "incoming")
        shutil.copytree(archives.parent, root / "observations")
        return source, trial.binding
    finally:
        trial.doCleanups()


class GuestTransferTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        directory = tempfile.TemporaryDirectory(prefix="w11-transfer-fixture-", dir=Path.cwd())
        cls.addClassCleanup(directory.cleanup)
        cls.root = Path(directory.name).resolve()
        cls.source, cls.binding = prepare_fixture(cls.root)
        cls.incoming = cls.root / "incoming"
        cls.raw = cls.root / "observations"
        cls.subject = transfer.workflow.check_build(cls.source, cls.incoming, cls.binding)
        cls.request = transfer.request_bytes(cls.source, cls.incoming, cls.binding, "fixture")
        cls.digest = sha256(cls.request).hexdigest()
        cls.body = transfer.result_bytes(cls.raw, cls.subject)
        cls.serial = transfer.frame_response(cls.request, cls.body)

    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="case-", dir=self.root)
        self.addCleanup(temp.cleanup)
        self.case = Path(temp.name).resolve()

    def test_copies_exact_input_and_replays_raw_output(self):
        target = self.case / "incoming"
        subject = transfer.copy_input(self.request, self.digest, self.source, self.incoming,
                                      self.binding, "fixture", target)
        self.assertEqual(subject, self.subject)
        for leaf in self.incoming.iterdir():
            self.assertEqual(leaf.read_bytes(), (target / leaf.name).read_bytes())
        output = self.case / "replayed"
        copied = transfer.receive_result(self.request, self.serial, subject, output)
        self.assertEqual(copied["files"], len(transfer.leaf_inventory()))
        for leaf in transfer.leaf_inventory():
            self.assertEqual((self.raw / leaf).read_bytes(), (output / leaf).read_bytes())
        with self.assertRaisesRegex(ValueError, "destination already exists"):
            transfer.receive_result(self.request, self.serial, subject, output)

    def test_foreign_attempt_and_fixture_scope_are_refused(self):
        for binding, scope in ((self.binding, "protected"), ({**self.binding, "run_attempt": "9"}, "fixture")):
            with self.subTest(binding=binding, scope=scope), self.assertRaises(ValueError):
                transfer.copy_input(self.request, self.digest, self.source, self.incoming,
                                    binding, scope, self.case / "must-not-exist")
        self.assertFalse((self.case / "must-not-exist").exists())
        with self.assertRaisesRegex(ValueError, "protected main must record approval"):
            transfer.protected_request("I74", self.incoming)

    def test_changed_artifact_or_request_refused_before_publication(self):
        artifact = shutil.copytree(self.incoming, self.case / "changed")
        image = artifact / "I74-server"
        image.write_bytes(image.read_bytes() + b"x")
        for raw, original in ((self.request + b" ", self.incoming), (self.request, artifact)):
            with self.subTest(raw_length=len(raw)), self.assertRaises(ValueError):
                transfer.copy_input(raw, self.digest, self.source, original,
                                    self.binding, "fixture", self.case / "output")
        self.assertFalse((self.case / "output").exists())

    def test_wrong_nonce_incomplete_duplicate_and_changed_frames_are_refused(self):
        other = transfer.request_bytes(self.source, self.incoming, self.binding, "fixture")
        rows = self.serial.splitlines(keepends=True)
        changed = bytearray(rows[1])
        offset = len(transfer.PREFIX) + len(b"DATA ")
        changed[offset] = ord("A") if changed[offset] != ord("A") else ord("B")
        for request, frame in ((other, self.serial), (self.request, b"".join(rows[:-1])),
                               (self.request, self.serial * 2), (self.request, b"".join([rows[0], changed, *rows[2:]]))):
            with self.subTest(bytes=len(frame)), self.assertRaises(ValueError):
                transfer.receive_result(request, frame, self.subject, self.case / "output")

    def test_fresh_transport_digests_cannot_hide_a_changed_response(self):
        changed = BytesIO()
        with ZipFile(BytesIO(self.body)) as original, ZipFile(changed, "w", compression=ZIP_STORED) as altered:
            for info in original.infolist():
                data = original.read(info)
                if info.filename == "archives/w11-256/MSET-257/after-0.response.resp":
                    data = b"?" + data[1:]
                altered.writestr(info, data)
        wire = transfer.frame_response(self.request, changed.getvalue())
        with self.assertRaisesRegex(ValueError, "after-0 differs"):
            transfer.receive_result(self.request, wire, self.subject, self.case / "output")
        self.assertFalse((self.case / "output").exists())


if __name__ == "__main__":
    unittest.main()
