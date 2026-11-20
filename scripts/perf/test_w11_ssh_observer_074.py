from copy import deepcopy
from hashlib import sha256
from io import BytesIO
import os
from pathlib import Path
import sys
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import Mock, patch
from zipfile import ZIP_DEFLATED, ZipFile

from . import test_w11_build_workflow_074 as fixture
from . import w11_ssh_observer_074 as relay


class SshObserverTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        case = fixture.WorkflowArtifactTests()
        case.setUp()
        cls.addClassCleanup(case.doCleanups)
        cls.source, cls.incoming, _, archives, _ = case.make_packet()
        cls.binding = case.binding
        cls.root = case.root
        cls.subject = relay.workflow.check_build(cls.source, cls.incoming, cls.binding)
        cls.request = relay.transfer.request_bytes(cls.source, cls.incoming, cls.binding, "protected")
        body = BytesIO()
        with ZipFile(body, "w", compression=ZIP_DEFLATED) as archive:
            for leaf in sorted(cls.incoming.iterdir()):
                archive.write(leaf, leaf.name)
        cls.archive = body.getvalue()
        cls.artifact = {"id": "42", "sha256": sha256(cls.archive).hexdigest(), "bytes": len(cls.archive)}
        cls.header = relay.make_header(cls.artifact, cls.request, 299 * 10**9)
        cls.result = relay.transfer.frame_response(cls.request, relay.transfer.result_bytes(archives.parent, cls.subject))

    def setUp(self):
        temp = tempfile.TemporaryDirectory(prefix="wire-", dir=self.root)
        self.addCleanup(temp.cleanup)
        self.directory = Path(temp.name).resolve()
        self.permit = {"schema": relay.PERMIT_SCHEMA, "expires_unix": 2000,
                       "roles": {"I74": {"artifact": deepcopy(self.artifact), "subject": deepcopy(self.subject)},
                                 "C74": {}}}

    def test_original_archive_and_build_are_preserved(self):
        stream = BytesIO(self.header + self.archive)
        given = relay.authorized_submission(relay.read_header(stream), self.permit, 1000)
        self.assertEqual(stream.tell(), len(self.header))
        self.assertEqual(given.request, self.request)
        stage = relay.stage_input(stream, given, self.source, self.binding, self.directory)
        for leaf in self.incoming.iterdir():
            self.assertEqual(leaf.read_bytes(), (stage / leaf.name).read_bytes())
        self.assertEqual((self.directory / "build.zip").read_bytes(), self.archive)

    def test_changed_artifact_role_and_attempt_fail_before_body(self):
        known = relay.read_header(BytesIO(self.header))
        for mutation in ("id", "sha256", "run_attempt", "role", "scope", "remaining_ns"):
            header = deepcopy(known)
            if mutation in ("id", "sha256"):
                header["artifact"][mutation] = "8" if mutation == "id" else "0" * 64
            elif mutation in ("run_attempt", "role"):
                header["request"]["subject"][mutation] = "9" if mutation == "run_attempt" else "C74"
            elif mutation == "scope":
                header["request"][mutation] = "fixture"
            else:
                header[mutation] = 301 * 10**9
            with self.subTest(mutation=mutation), self.assertRaises(ValueError):
                relay.authorized_submission(header, self.permit, 1000)
        with self.assertRaisesRegex(ValueError, "expired"):
            relay.authorized_submission(known, self.permit, 2000)
        self.assertEqual(list(self.directory.iterdir()), [])

    def test_truncated_trailing_and_modified_archives_fail_before_extraction(self):
        given = relay.authorized_submission(relay.read_header(BytesIO(self.header)), self.permit, 1000)
        changed = bytes([self.archive[0] ^ 1]) + self.archive[1:]
        for n, raw in enumerate((self.archive[:-1], self.archive + b"extra", changed)):
            case = self.directory / str(n)
            case.mkdir()
            with self.subTest(n=n), self.assertRaises(ValueError):
                relay.stage_input(BytesIO(raw), given, self.source, self.binding, case)
            self.assertFalse((case / "incoming").exists())

    def test_noncanonical_header_and_oversize_are_refused(self):
        for raw in (b"x", (relay.receipt.MAX_JSON_BYTES + 1).to_bytes(4, "big"), self.header[:-2],
                    (7).to_bytes(4, "big") + b"{ }  \r\n"):
            with self.subTest(raw=raw), self.assertRaises(ValueError):
                relay.read_header(BytesIO(raw))

    def test_one_claim_per_role_and_attempt_survives_another_nonce(self):
        given = relay.authorized_submission(relay.read_header(BytesIO(self.header)), self.permit, 1000)
        claim = relay.hold_claim(self.directory, given, self.binding, self.source.role)
        self.assertEqual((claim / "guest-request.json").read_bytes(), self.request)
        retry = relay.transfer.request_bytes(self.source, self.incoming, self.binding, "protected")
        self.assertNotEqual(retry, self.request)
        header = relay.make_header(self.artifact, retry, 299 * 10**9)
        repeat = relay.authorized_submission(relay.read_header(BytesIO(header)), self.permit, 1000)
        with self.assertRaisesRegex(ValueError, "already claimed"):
            relay.hold_claim(self.directory, repeat, self.binding, self.source.role)
        self.assertEqual((claim / "guest-request.json").read_bytes(), self.request)

    def test_artifact_identity_comes_from_this_github_run(self):
        suffix = "-".join(self.binding[key] for key in ("workflow_commit", "run_id", "run_attempt"))
        project = int(relay.build.REPOSITORY_ID)
        row = {"id": int(self.artifact["id"]), "size_in_bytes": self.artifact["bytes"],
               "digest": "sha256:" + self.artifact["sha256"], "expired": False,
               "name": "product-w11-build-074-I74-" + suffix,
               "workflow_run": {"id": int(self.binding["run_id"]), "repository_id": project,
                                "head_repository_id": project, "head_branch": "main",
                                "head_sha": self.binding["workflow_commit"]}}
        doc = {"total_count": 1, "artifacts": [row]}
        self.assertEqual(relay.lookup_artifact(doc, "I74", self.binding), self.artifact)
        row["workflow_run"]["head_repository_id"] += 1
        with self.assertRaisesRegex(ValueError, "another repository"):
            relay.lookup_artifact(doc, "I74", self.binding)

    def test_connection_loss_retains_verified_result_and_replay_cannot_boot(self):
        roles, pin = {"I74": self.source}, "f" * 64
        boot = Mock(return_value=(self.result, (70, 150, 5)))
        closed = Mock()
        closed.write.side_effect = BrokenPipeError("controlled connection loss")
        locks = SimpleNamespace(LOCK_EX=1, LOCK_NB=2, flock=Mock())
        with (patch.dict(sys.modules, {"fcntl": locks}), patch.dict(os.environ),
              patch.object(relay, "WORK", self.directory),
              patch.object(relay.time, "time", return_value=1000),
              patch.object(relay, "installed_policy", return_value=(roles, pin, self.permit)),
              patch.object(relay.workflow, "approved_plan", return_value=(roles, pin)),
              patch.object(relay.driver, "check_runtime"),
              patch.object(relay.driver, "host_guard", return_value=70 * 1024**3),
              patch.object(relay.driver, "run_vm", boot)):
            with self.assertRaises(BrokenPipeError):
                relay.serve(BytesIO(self.header + self.archive), closed, time.monotonic())
            claim = self.directory / f"run-{self.binding['run_id']}-{self.binding['run_attempt']}-I74"
            self.assertEqual((claim / "serial.log").read_bytes(), self.result)
            with self.assertRaisesRegex(ValueError, "already claimed"):
                relay.serve(BytesIO(self.header + self.archive), BytesIO(), time.monotonic())
            boot.assert_called_once()

    def test_ssh_uses_fixed_command_and_pinned_host(self):
        key, hosts = self.directory / "key", self.directory / "known-hosts"
        for path in (key, hosts):
            path.touch()
        command = relay.ssh_command("observer.example", key, hosts)
        self.assertEqual(command[-2:], [relay.ACCOUNT + "@observer.example", relay.COMMAND])
        self.assertIn("StrictHostKeyChecking=yes", command)
        self.assertIn("ClearAllForwardings=yes", command)
        with self.assertRaisesRegex(ValueError, "hostname"):
            relay.ssh_command("-F", key, hosts)


if __name__ == "__main__":
    unittest.main()
