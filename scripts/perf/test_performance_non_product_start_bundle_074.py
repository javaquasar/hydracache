import hashlib
import importlib.util
import json
import os
import pathlib
import tempfile
import unittest
from unittest import mock


SCRIPT = pathlib.Path(__file__).with_name("performance_non_product_start_bundle_074.py")
SPEC = importlib.util.spec_from_file_location(
    "performance_non_product_start_bundle_074", SCRIPT
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)
SOURCE_SHA = "a" * 40


def canonical_json(value: object) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":")).encode("utf-8")


def write_observation(directory: pathlib.Path, *, source_sha: str = SOURCE_SHA) -> dict:
    directory.mkdir()
    fixture = {
        "path": MODULE.FIXTURE_PATH,
        "sha256": "9" * 64,
        "size": 8192,
        "inode": 4102,
        "device": 2049,
        "uid": 0,
        "gid": 0,
        "mode": 0o755,
    }
    receipt = {
        "schema_version": 1,
        "machine_id": "machine-a",
        "boot_id": "boot-a",
        "kernel_release": "test-kernel",
        "kernel_command_line_sha256": "0" * 64,
        "campaign_mount": {
            "mount_id": 1,
            "device_major_minor": "8:1",
            "root": "/",
            "mount_point": "/var/lib/hydracache-performance",
            "mount_options": ["rw"],
            "filesystem_type": "ext4",
            "source": "/dev/test",
            "super_options": ["rw"],
        },
        "mount_identity": "dev=1;opts=rw",
        "online_cpuset": "0-7",
        "isolated_cpuset": "2-7",
        "housekeeping_cpuset": "0-1",
        "cpu_governors": {f"cpu{cpu}": "performance" for cpu in range(8)},
        "kernel_tunables": {},
        "supervisor_binary": {
            "path": "/opt/hydracache-perf/bin/hydracache-long-run-supervisor-074",
            "sha256": "8" * 64,
            "size": 16384,
            "inode": 4101,
            "device": 2049,
            "uid": 0,
            "gid": 0,
            "mode": 0o755,
        },
        "reference_host_freeze_sha256": "7" * 64,
    }
    request = {
        "request": {
            "schema_version": 1,
            "request_id": "123e4567-e89b-42d3-a456-426614174000",
            "operation": "host_observation",
            "campaign_id": "2" * 64,
            "expected_state_revision": 0,
            "manifest_path": None,
            "manifest_sha256": "3" * 64,
            "controller": {
                "repository_id": 123,
                "run_id": 456,
                "run_attempt": 1,
                "actor_id": 789,
                "authorization_sha256": "0" * 64,
            },
            "abort_reason": None,
            "approval_nonce_sha256": None,
        },
        "authorization": None,
    }
    receipt_bytes = canonical_json(receipt)
    receipt_sha256 = hashlib.sha256(receipt_bytes).hexdigest()
    response_body = {
        "schema_version": 1,
        "request_id": request["request"]["request_id"],
        "campaign_id": request["request"]["campaign_id"],
        "ok": True,
        "state_revision": 0,
        "server_time_unix_seconds": 2_000_000_000,
        "result": {
            "schema_version": 1,
            "installed_source_commit": source_sha,
            "fixture_binary": fixture,
            "receipt_sha256": receipt_sha256,
            "receipt": receipt,
        },
        "error_code": None,
    }
    response = dict(
        response_body, response_sha256=hashlib.sha256(canonical_json(response_body)).hexdigest()
    )
    request_bytes = canonical_json(request)
    evidence = {
        "schema_version": "hydracache-w11-host-observation-socket-v1",
        "source_commit": source_sha,
        "installed_source_commit": source_sha,
        "installed_supervisor_binary_sha256": receipt["supervisor_binary"]["sha256"],
        "installed_fixture_binary": fixture,
        "request_sha256": hashlib.sha256(request_bytes).hexdigest(),
        "response_sha256": response["response_sha256"],
        "host_receipt_sha256": receipt_sha256,
        "peer_admission_required": True,
        "signed_authorization_used": False,
        "arbitrary_output_path_accepted": False,
        "campaign_state_mutated": False,
        "product_candidate_started": False,
        "promotable": False,
    }
    (directory / "request.json").write_bytes(request_bytes)
    response_bytes = json.dumps(response, separators=(",", ":")).encode("utf-8")
    (directory / "response.json").write_bytes(response_bytes + b"\n")
    (directory / "host-observation.json").write_bytes(receipt_bytes + b"\n")
    (directory / "host-observation.sha256").write_text(
        receipt_sha256 + "\n", encoding="ascii", newline="\n"
    )
    (directory / "evidence.json").write_bytes(canonical_json(evidence) + b"\n")
    return evidence


class PerformanceNonProductStartBundle074Tests(unittest.TestCase):
    def test_source_tree_requires_exact_clean_checkout(self) -> None:
        with mock.patch.object(
            MODULE.subprocess,
            "check_output",
            side_effect=[SOURCE_SHA + "\n", " M changed\n"],
        ):
            with self.assertRaisesRegex(ValueError, "exact clean source commit"):
                MODULE.source_tree(SOURCE_SHA)
        with mock.patch.object(
            MODULE.subprocess,
            "check_output",
            side_effect=[SOURCE_SHA + "\n", "", "b" * 40 + "\n"],
        ):
            self.assertEqual(MODULE.source_tree(SOURCE_SHA), "b" * 40)

    def test_exact_observation_builds_only_bundle_and_non_promotable_evidence(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            observation = write_observation(root / "observation")
            with mock.patch.object(MODULE, "source_tree", return_value="b" * 40):
                evidence = MODULE.assemble(
                    root / "observation",
                    root / "bundle",
                    root / "evidence",
                    SOURCE_SHA,
                    123,
                    now_unix_seconds=2_000_000_000,
                    lease_id="123e4567-e89b-42d3-a456-426614174001",
                    nonce_hex="4" * 64,
                )
            builder = MODULE.load_builder()
            campaign, bundle_digest = builder.verify_start_bundle(root / "bundle")
            self.assertEqual(campaign, evidence["campaign_id"])
            self.assertEqual(bundle_digest, evidence["start_bundle_sha256"])
            self.assertEqual(
                evidence["installed_fixture_binary"],
                observation["installed_fixture_binary"],
            )
            self.assertFalse(evidence["signed_start_dispatched"])
            self.assertFalse(evidence["campaign_state_mutated"])
            self.assertFalse(evidence["product_candidate_started"])
            self.assertFalse(evidence["promotable"])
            self.assertEqual(
                {path.name for path in (root / "bundle").iterdir()},
                {
                    "campaign-start.json",
                    "campaign-start.sha256",
                    "host-observation.json",
                    "host-observation.sha256",
                    "start-bundle.json",
                    "start-bundle.sha256",
                },
            )

    def test_source_fixture_and_response_tamper_fail_closed(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            write_observation(root / "observation")
            with self.assertRaisesRegex(ValueError, "exact non-product bundle input"):
                MODULE.load_observation(root / "observation", "f" * 40)

            evidence_path = root / "observation" / "evidence.json"
            evidence = json.loads(evidence_path.read_text(encoding="utf-8"))
            evidence["installed_fixture_binary"]["uid"] = 1000
            evidence_path.write_bytes(canonical_json(evidence) + b"\n")
            with self.assertRaisesRegex(ValueError, "exact non-product bundle input|identity differs"):
                MODULE.load_observation(root / "observation", SOURCE_SHA)

            write_observation(root / "observation-clean")
            response_path = root / "observation-clean" / "response.json"
            response = json.loads(response_path.read_text(encoding="utf-8"))
            response["server_time_unix_seconds"] += 1
            response_path.write_bytes(canonical_json(response) + b"\n")
            with self.assertRaisesRegex(ValueError, "exact non-product bundle input"):
                MODULE.load_observation(root / "observation-clean", SOURCE_SHA)

    def test_layout_and_create_new_guards_reject_ambiguous_input_or_output(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            write_observation(root / "observation")
            (root / "observation" / "unexpected").write_text("extra", encoding="ascii")
            with self.assertRaisesRegex(ValueError, "five-file layout"):
                MODULE.load_observation(root / "observation", SOURCE_SHA)

            write_observation(root / "observation-clean")
            request = root / "observation-clean" / "request.json"
            alias = root / "request-alias.json"
            request.replace(alias)
            os.link(alias, request)
            with self.assertRaisesRegex(ValueError, "unsafe host observation file"):
                MODULE.load_observation(root / "observation-clean", SOURCE_SHA)

            write_observation(root / "observation-output")
            (root / "bundle").mkdir()
            with self.assertRaisesRegex(ValueError, "output path already exists"):
                MODULE.assemble(
                    root / "observation-output",
                    root / "bundle",
                    root / "evidence",
                    SOURCE_SHA,
                    123,
                )


if __name__ == "__main__":
    unittest.main()
