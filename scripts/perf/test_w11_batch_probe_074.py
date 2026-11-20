"""Native end-to-end controls with the pinned synthetic RESP daemon."""

from contextlib import contextmanager
from hashlib import sha256
import os
from pathlib import Path
import select
import shutil
import socket
import subprocess
import tempfile
import time
import unittest
from unittest.mock import patch

from scripts.perf import w11_batch_probe_074 as joined
from scripts.perf import w11_product_build_receipt_074 as receipt


WORKFLOW = receipt.build.approved_workflow_run("a" * 40, "123", "1")
ENVIRONMENT = {
    "GITHUB_REPOSITORY": receipt.build.REPOSITORY,
    "GITHUB_REPOSITORY_ID": receipt.build.REPOSITORY_ID,
    "GITHUB_REF": "refs/heads/main",
    "GITHUB_WORKFLOW_REF": receipt.build.WORKFLOW,
    "GITHUB_SHA": WORKFLOW["workflow_commit"],
    "GITHUB_RUN_ID": WORKFLOW["run_id"],
    "GITHUB_RUN_ATTEMPT": WORKFLOW["run_attempt"],
}
SOURCE = ("1" * 40, "2" * 40)
LOCK = "0" * 64
VERSION = b"0.74.0-rc.1"


@unittest.skipUnless(os.environ.get("W11_JOINED_FIXTURE") == "1", "isolated Linux fixture")
class JoinedBatchProofTests(unittest.TestCase):
    def setUp(self):
        self.stage = Path.cwd().resolve()
        temporary = tempfile.TemporaryDirectory(prefix="jp-", dir=self.stage)
        self.root = Path(temporary.name).resolve()
        self.assertEqual(self.root.parent, self.stage)
        self.addCleanup(temporary.cleanup)
        self.material = self.root / "material"
        self.token = joined.probe.fixture(self.material)
        self.boot = joined.readback.BOOT_ID.read_text().strip()
        self.namespace = joined.namespace_identity("thread-self")
        self.archives = self.root / "archives"
        self.archives.mkdir()
        self.artifact = self.root / "artifact"
        self.artifact.mkdir()
        self.image = self.artifact / "I74-server"
        shutil.copy2(Path(os.readlink("/proc/self/exe")), self.image)
        self.subject = receipt.build.receipt_for_source("I74", self.artifact, SOURCE, LOCK, WORKFLOW)
        self.subject["schema"] = receipt.RECEIPT_SCHEMA
        self.emitter = self.stage / "fixture-emitter.so"
        self.daemon_script = self.stage / "fixture-daemon.py"

    @contextmanager
    def original(self, selector):
        name = selector or "default"
        run_directory = self.root / name
        run_directory.mkdir(mode=0o700)
        environment = {
            "PATH": "/usr/bin:/bin", "LANG": "C", "HOME": str(run_directory),
            "PYTHONPATH": str(self.stage), "HYDRACACHE_W11_CONFIG_READBACK": "1",
        }
        limit = 128
        if selector:
            environment["HYDRACACHE_CLIENT_BATCH_PROFILE"] = selector
            limit = 256
        with joined.readback.private_channel(run_directory) as channel:
            started = time.monotonic()
            deadline = started + joined.probe.PROCESS_LIMIT
            argv = [str(self.image), "-B", str(self.daemon_script), str(self.material), str(self.emitter), str(limit)]
            with subprocess.Popen(
                argv, cwd=run_directory, env=environment,
                stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            ) as child:
                try:
                    events = select.poll()
                    events.register(child.stdout, select.POLLIN)
                    self.assertTrue(events.poll(5000), "fixture readiness timed out")
                    ready = child.stdout.readline(256)
                    self.assertEqual(ready, joined.readback.READY)
                    port = int((run_directory / "fixture-port").read_text())
                    address = ("127.0.0.1", port)
                    yield child, channel, ready, address, deadline
                finally:
                    if child.poll() is None:
                        try:
                            remaining = max(0.001, min(5, deadline - time.monotonic()))
                            _, error = child.communicate(b"x", timeout=remaining)
                        except subprocess.TimeoutExpired:
                            child.kill()
                            child.communicate(timeout=5)
                            raise
                        self.assertEqual(child.returncode, 0, error.decode(errors="replace"))
                    self.assertLess(time.monotonic() - started, joined.probe.PROCESS_LIMIT)

    def collect(self, selector, child, channel, ready, address, deadline):
        session = joined.batch.RespSession(self.material, address, self.token, deadline, VERSION)
        root = self.archives / (selector or "default")
        return joined.collect_generation(self.subject, child, self.image, self.boot, self.namespace,
                                         channel, ready, selector, session, root)

    def test_joined_readback_mtls_and_batch_proof_reaches_the_v2_receipt(self):
        observed = {}
        for selector in (None, "w11-256"):
            name = selector or "default"
            with self.original(selector) as (child, channel, ready, address, deadline):
                record = self.collect(selector, child, channel, ready, address, deadline)
                self.assertEqual(record["before"]["pid"], child.pid)
                self.assertEqual(record["before"], record["after"])
                self.assertEqual(record["before"]["elf_sha256"], self.subject["binary"]["sha256"])
                self.assertEqual(len(record["cases"]), 4 if selector is None else 8)
                observed[name] = record
        self.assertNotEqual(observed["default"]["before"], observed["w11-256"]["before"])
        proof = joined.batch.publish_evidence(self.subject, observed["default"], observed["w11-256"],
                                               self.archives, self.artifact)
        approved = receipt.ApprovedRole("I74", *SOURCE, LOCK, proof)
        with patch.dict(os.environ, ENVIRONMENT), patch.object(receipt.build, "verify_source_identity"):
            receipt.write_receipt(approved, self.root, self.artifact)
            receipt.verify_receipt(approved, self.artifact)
        data = self.artifact / receipt.RECEIPT
        verified = receipt.build.load(data)
        self.assertEqual(verified["w11_profile"]["boundary_evidence_sha256"], proof)
        self.assertEqual(verified["binary"], self.subject["binary"])
        output = self.stage / "controlled-joined-proof"
        output.mkdir(mode=0o700)
        shutil.copytree(self.archives, output / "archives")
        for name in (receipt.EVIDENCE, receipt.RECEIPT):
            shutil.copyfile(self.artifact / name, output / name)
        with (output / "scope.json").open("xb") as out:
            out.write(receipt.canonical({"controlled_only": True, "promotable": False,
                                         "proof_sha256": proof,
                                         "receipt_sha256": sha256(data.read_bytes()).hexdigest()}) + b"\n")

    def test_foreign_listener_is_refused_without_opening_a_connection(self):
        with self.original(None) as (child, channel, ready, _, deadline):
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as other:
                other.bind(("127.0.0.1", 0))
                other.listen()
                with self.assertRaisesRegex(ValueError, "original daemon in the process pair"):
                    self.collect(None, child, channel, ready, other.getsockname(), deadline)
                ready = select.poll()
                ready.register(other, select.POLLIN)
                self.assertFalse(ready.poll(0))
                self.assertFalse(any(self.archives.iterdir()))
