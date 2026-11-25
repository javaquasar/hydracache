import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from . import maintenance_i74_receipt_074 as maintenance


class MaintenanceReceiptTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.source = self.root / "source"
        self.directory = self.root / "artifact"
        self.source.mkdir()
        self.directory.mkdir()
        self.binary = self.directory / "I74-server"
        self.binary.write_bytes(b"\x7fELF\x02\x01\x01" + bytes(9) + b"\x03\x00\x3e\x00" + bytes(44))
        (self.source / "Cargo.lock").write_bytes(b"controlled lock\n")
        self.git("init", "--quiet")
        self.git("add", "Cargo.lock")
        self.git("-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid",
                 "-c", "commit.gpgSign=false", "commit", "--quiet", "-m", "test input")
        self.identity = (self.git("rev-parse", "HEAD"), self.git("rev-parse", "HEAD^{tree}"))
        self.lock = maintenance.build.sha256(self.source / "Cargo.lock")
        self.enterContext(patch.object(maintenance, "SOURCE", self.identity))
        self.enterContext(patch.object(maintenance, "LOCK", self.lock))
        self.enterContext(patch.dict(os.environ, {
            "GITHUB_REPOSITORY": maintenance.build.REPOSITORY, "GITHUB_REPOSITORY_ID": maintenance.build.REPOSITORY_ID,
            "GITHUB_REF": "refs/heads/main", "GITHUB_WORKFLOW_REF": maintenance.build.WORKFLOW,
            "GITHUB_SHA": "a" * 40, "GITHUB_RUN_ID": "123", "GITHUB_RUN_ATTEMPT": "2",
        }))
        self.binding = maintenance.build.workflow_binding()
        self.digest = maintenance.write_receipt(self.source, self.directory)

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.source), "-c", "core.autocrlf=false", *args],
                                       text=True).strip()

    def check(self):
        maintenance.verify(self.directory, self.binding, built_sha256=self.digest)

    def test_exact_source_image_purpose_and_run(self):
        self.check()
        self.assertEqual(self.digest, hashlib.sha256(self.binary.read_bytes()).hexdigest())
        doc = json.loads((self.directory / maintenance.RECEIPT).read_bytes())
        self.assertEqual((doc["schema"], doc["purpose"], doc["role"]),
                         ("hydracache-product-maintenance-074-v1", "maintenance-release-candidate", "I74"))
        self.assertEqual((doc["source_commit"], doc["source_tree"], doc["cargo_lock_sha256"]),
                         (*self.identity, self.lock))
        self.assertEqual((doc["workflow_commit"], doc["run_id"], doc["run_attempt"]), ("a" * 40, "123", "2"))

    def test_wrong_source_and_dirty_checkout_refused_before_writing(self):
        for source, lock, error in [(("0" * 40, self.identity[1]), self.lock, "commit or tree"),
                                    ((self.identity[0], "f" * 40), self.lock, "commit or tree"),
                                    (self.identity, "f" * 64, "Cargo.lock")]:
            with self.subTest(expected=error), patch.object(maintenance, "SOURCE", source), \
                 patch.object(maintenance, "LOCK", lock), self.assertRaisesRegex(ValueError, error):
                maintenance.write_receipt(self.source, self.directory)
        (self.source / "untracked").touch()
        with self.assertRaisesRegex(ValueError, "dirty role"):
            maintenance.write_receipt(self.source, self.directory)

    def test_source_run_downgrade_and_receipt_substitution_refused(self):
        file = self.directory / maintenance.RECEIPT
        original = json.loads(file.read_bytes())
        for fields in ({"purpose": "qualification"}, {"schema": "hydracache-product-build-074-v2"},
                       {"role": "C74"}, {"source_commit": "0" * 40}, {"source_tree": "0" * 40},
                       {"cargo_lock_sha256": "1" * 64}, {"run_id": "124"}, {"run_attempt": "1"},
                       {"workflow_commit": "b" * 40}):
            with self.subTest(fields=fields), self.assertRaisesRegex(ValueError, "receipt differs"):
                file.write_bytes(maintenance.receipt_bytes({**original, **fields}))
                self.check()

    def test_only_the_built_image_and_inventory_are_accepted(self):
        for pin in ("", "invalid", "0" * 64):
            with self.subTest(pin=pin), self.assertRaisesRegex(ValueError, "build job output"):
                maintenance.verify(self.directory, self.binding, built_sha256=pin)
        extra = self.directory / "C74-server"
        extra.touch()
        with self.assertRaisesRegex(ValueError, "inventory"):
            self.check()
        extra.unlink()
        self.binary.write_bytes(self.binary.read_bytes() + b"changed")
        with self.assertRaisesRegex(ValueError, "receipt differs"):
            self.check()
        self.binary.write_bytes(b"X" * 64)
        with self.assertRaisesRegex(ValueError, "ELF"):
            self.check()

    def test_signed_verification_uses_maintenance_predicate_and_protected_identity(self):
        (self.directory / maintenance.BUNDLE).write_bytes(b"controlled bundle")
        doc = json.loads((self.directory / maintenance.RECEIPT).read_bytes())
        def reply(value):
            return json.dumps([{"verificationResult": {"statement": {"predicate": value}}}]).encode()

        with patch.object(maintenance.build.subprocess, "check_output", return_value=reply(doc)) as gh:
            maintenance.verify(self.directory, self.binding, signed=True)
            args = gh.call_args.args[0]
            for flag, expected in (("--predicate-type", maintenance.PREDICATE), ("--repo", maintenance.build.REPOSITORY),
                                   ("--signer-workflow", maintenance.build.WORKFLOW.split("@")[0]),
                                   ("--signer-digest", "a" * 40), ("--source-ref", "refs/heads/main"),
                                   ("--source-digest", "a" * 40)):
                self.assertEqual(args[args.index(flag) + 1], expected)
            self.assertIn("--deny-self-hosted-runners", args)
        with patch.object(maintenance.build.subprocess, "check_output", return_value=reply({**doc, "role": "C74"})), \
             self.assertRaisesRegex(ValueError, "signed predicate differs"):
            maintenance.verify(self.directory, self.binding, signed=True)


if __name__ == "__main__":
    unittest.main()
