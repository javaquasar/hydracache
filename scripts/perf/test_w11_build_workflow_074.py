import json
from pathlib import Path
import tempfile
import time
import unittest
from unittest.mock import patch

from scripts.perf import test_w11_batch_boundary_074 as model
from scripts.perf import w11_build_workflow_074 as workflow


class WorkflowArtifactTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="w11-artifacts-", dir=Path.cwd())
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name).resolve()
        self.binding = model.WORKFLOW

    def make_packet(self, role="I74"):
        source = workflow.SourceRole(role, *model.SOURCE, workflow.build.LOCK)
        incoming, observations, final = [self.root / (role + name) for name in ("-build", "-raw", "-final")]
        incoming.mkdir()
        observations.mkdir()
        archives = observations / "archives"
        archives.mkdir()
        (incoming / f"{role}-server").write_bytes(model.IMAGE + role.encode())
        subject = workflow.subject_for(source, incoming, self.binding)
        (incoming / workflow.SUBJECT).write_bytes(workflow.receipt.canonical(subject) + b"\n")
        (archives / workflow.SUBJECT).write_bytes(workflow.receipt.canonical(subject) + b"\n")
        records = []
        with patch.object(workflow.probe, "disk_guard", return_value=workflow.probe.DISK_FLOOR + 1024**2):
            for selector, count, pid in ((None, 128, 300), ("w11-256", 256, 301)):
                directory = archives / (selector or "default")
                daemon = model.DaemonModel(count)
                generation = {"boot_id": model.BOOT, "pid": pid, "start_ticks": pid * 8,
                              "elf_sha256": subject["binary"]["sha256"]}
                session = workflow.batch.RespSession(self.root, ("127.0.0.1", 1), model.TOKEN,
                                                     time.monotonic() + 60, model.VERSION)
                with patch.object(workflow.probe, "exchange", daemon.exchange):
                    observed = workflow.batch.collect_profile(subject, selector, workflow.receipt.config(selector),
                                                              lambda: generation, session, directory)
                (directory / "effective-config.json").write_bytes(workflow.receipt.canonical(observed["effective_config"]))
                (directory / "ready.json").write_bytes(workflow.readback.READY)
                listener = {"network_namespace_inode": 7, "socket_inode": 11 + pid, "daemon_pid": pid, "port": 9999}
                (directory / "listener.json").write_bytes(workflow.receipt.canonical(listener) + b"\n")
                run = {"daemon_pid": pid, "client_cpu": 1, "daemon_cpu": 2, "elapsed_ns": 9000, "exit": 0}
                (directory / "run.json").write_bytes(workflow.receipt.canonical(run) + b"\n")
                records.append(observed)
            workflow.batch.publish_evidence(subject, *records, archives, observations)
        pins = workflow.finalize(source, incoming, observations, final, self.binding)
        return source, incoming, final, archives, pins

    def test_replay_both_roles_against_the_independent_pins(self):
        for name in ("I74", "C74"):
            with self.subTest(role=name):
                source, _, final, archives, pins = self.make_packet(name)
                result = workflow.verify_packet(source, final, archives, *pins, self.binding)
                self.assertEqual(result["role"], name)
                self.assertEqual(result["w11_profile"]["boundary_evidence_sha256"], pins[0])
                self.assertEqual(result["w11_profile"]["accepted"], [128, 129, 256])

    def test_unsigned_image_replaced_after_build_is_rejected(self):
        source, incoming, _, _, _ = self.make_packet()
        with (incoming / "I74-server").open("ab") as image:
            image.write(b"changed")
        with self.assertRaisesRegex(ValueError, "unsigned build identity differs"):
            workflow.check_build(source, incoming, self.binding)

    def test_signer_refuses_old_attempt_and_changed_independent_pins(self):
        source, _, final, archives, pins = self.make_packet()
        old_binding = {**self.binding, "run_attempt": "2"}
        for given, bind, reason in (
            (pins, old_binding, "different build, role or workflow run"),
            (("f" * 64, pins[1]), self.binding, "admission pin"),
            ((pins[0], "f" * 64), self.binding, "independent verification job"),
        ):
            with self.subTest(reason=reason), self.assertRaisesRegex(ValueError, reason):
                workflow.verify_packet(source, final, archives, *given, bind)

    def test_signer_matches_exact_response_and_inventory(self):
        source, _, final, archives, pins = self.make_packet()
        leaf = archives / "w11-256/MSET-257/after-0.response.resp"
        original = leaf.read_bytes()
        leaf.write_bytes(b"?" + original[1:])
        with self.assertRaisesRegex(ValueError, "after-0 differs"):
            workflow.verify_packet(source, final, archives, *pins, self.binding)
        leaf.write_bytes(original)
        (archives / "w11-256" / "unexpected").mkdir()
        with self.assertRaisesRegex(ValueError, "unexpected files"):
            workflow.verify_packet(source, final, archives, *pins, self.binding)

    def test_signer_revalidates_authentication_and_listener(self):
        source, _, final, archives, pins = self.make_packet()
        leaf = archives / "default/auth/no-certificate.resp"
        leaf.write_bytes(b"+OK\r\n")
        with self.assertRaisesRegex(ValueError, "certificate or AUTH"):
            workflow.verify_packet(source, final, archives, *pins, self.binding)
        leaf.write_bytes(b"")
        leaf = archives / "default/listener.json"
        listener = json.loads(leaf.read_bytes())
        listener["daemon_pid"] += 1
        leaf.write_bytes(workflow.receipt.canonical(listener) + b"\n")
        with self.assertRaisesRegex(ValueError, "listener does not belong"):
            workflow.verify_packet(source, final, archives, *pins, self.binding)

    def test_signer_rechecks_the_original_process_limit(self):
        source, _, final, archives, pins = self.make_packet()
        leaf = archives / "w11-256/run.json"
        row = json.loads(leaf.read_bytes())
        row["elapsed_ns"] = 60 * 10**9 + 1
        leaf.write_bytes(workflow.receipt.canonical(row) + b"\n")
        with self.assertRaisesRegex(ValueError, "drain, placement or proof deadline"):
            workflow.verify_packet(source, final, archives, *pins, self.binding)

    def test_signer_rejects_crossed_roles_and_extra_artifact(self):
        i74, _, final, i74_archives, pins = self.make_packet()
        _, _, _, c74_archives, _ = self.make_packet("C74")
        with self.assertRaisesRegex(ValueError, "raw archive build identity differs"):
            workflow.verify_packet(i74, final, c74_archives, *pins, self.binding)
        (final / "scope.json").write_text('{"controlled_only":true}')
        with self.assertRaisesRegex(ValueError, "unexpected files"):
            workflow.verify_packet(i74, final, i74_archives, *pins, self.binding)

    def test_draft_and_legacy_roles_cannot_enable_the_workflow(self):
        draft = json.loads(workflow.APPROVAL.read_bytes())
        path = self.root / "approval.json"
        path.write_bytes(workflow.receipt.canonical(draft))
        with self.assertRaisesRegex(ValueError, "protected main must record approval"):
            workflow.approved_plan(path)
        draft["status"] = "approved"
        draft["observer_transport"] = "unapproved"
        path.write_bytes(workflow.receipt.canonical(draft))
        with self.assertRaisesRegex(ValueError, "approved SSH observer"):
            workflow.approved_plan(path)
        draft["observer_transport"] = "ssh"
        path.write_bytes(workflow.receipt.canonical(draft))
        with self.assertRaisesRegex(ValueError, "approved offline guest runtime"):
            workflow.approved_plan(path)
        draft["observer_runtime_sha256"] = "f" * 64
        path.write_bytes(workflow.receipt.canonical(draft))
        roles, runtime_sha256 = workflow.approved_plan(path)
        self.assertEqual(set(roles), {"I74", "C74"})
        self.assertEqual(runtime_sha256, draft["observer_runtime_sha256"])
        draft["roles"]["I74"]["commit"] = workflow.build.ROLES["I74"][0]
        path.write_bytes(workflow.receipt.canonical(draft))
        with self.assertRaisesRegex(ValueError, "approved successor source"):
            workflow.approved_plan(path)


if __name__ == "__main__":
    unittest.main()
