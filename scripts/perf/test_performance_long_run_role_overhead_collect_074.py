import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock


MODULE_PATH = Path(__file__).with_name(
    "performance_long_run_role_overhead_collect_074.py"
)
SPEC = importlib.util.spec_from_file_location(
    "performance_long_run_role_overhead_collect_074", MODULE_PATH
)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def provisioning_receipt(source: str, binary_sha256: str) -> dict:
    receipt = {field: 1 for field in MODULE.PROVISIONING_FIELDS}
    receipt.update(
        {
            "schema_version": "hydracache-w11-host-provisioning-v1",
            "source_commit": source,
            "created_at_utc": "2026-10-06T00:00:00Z",
            "mutation_performed": True,
            "repository_id": 1,
            "allowed_actor_ids": [1],
            "runner_group_database_membership": True,
            "runner_process_group_refresh_may_be_required": False,
            "service_active": True,
            "socket_mode": 0o660,
            "binary_sha256": binary_sha256,
        }
    )
    for field in (
        "fixture_binary_sha256",
        "config_sha256",
        "service_sha256",
        "sysusers_sha256",
        "tmpfiles_sha256",
        "verification_key_sha256",
        "unit_properties_sha256",
        "machine_id_sha256",
        "boot_id_sha256",
    ):
        receipt[field] = "b" * 64
    return receipt


class RoleOverheadCollectorTests(unittest.TestCase):
    def test_schedule_is_exact_abba_for_both_roles(self):
        schedule = [
            MODULE._variant_order(pair)
            for pair in range(1, MODULE.PAIRS + 1)
        ]
        self.assertEqual(
            schedule,
            [
                ("control", "instrumented"),
                ("instrumented", "control"),
                ("control", "instrumented"),
                ("instrumented", "control"),
                ("control", "instrumented"),
            ],
        )

    def test_fixture_argv_is_fixed_and_contains_no_shell(self):
        argv = MODULE.fixture_argv(
            MODULE.INSTALLED_BINARY, "i74", "control", 1, 1, "2"
        )
        self.assertEqual(
            argv,
            [
                "/usr/bin/taskset",
                "--cpu-list",
                "2",
                "/usr/bin/nice",
                "-n",
                "0",
                str(MODULE.INSTALLED_BINARY),
                "role-overhead-fixture",
                "i74",
                "control",
                "1",
                "1",
                "2",
            ],
        )
        self.assertNotIn("/bin/sh", argv)

    def test_provisioning_receipt_binds_source_binary_and_raw_digest(self):
        source = "a" * 40
        binary_sha256 = "c" * 64
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "receipt.json"
            raw = json.dumps(
                provisioning_receipt(source, binary_sha256), sort_keys=True
            ).encode("utf-8") + b"\n"
            path.write_bytes(raw)
            self.assertEqual(
                MODULE.validate_provisioning_receipt(path, source, binary_sha256),
                hashlib.sha256(raw).hexdigest(),
            )
            with self.assertRaisesRegex(ValueError, "identity differs"):
                MODULE.validate_provisioning_receipt(path, "d" * 40, binary_sha256)

    def test_cgroup_counter_parsers_are_strict(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            cpu = root / "cpu.stat"
            io = root / "io.stat"
            cpu.write_text("usage_usec 17\nuser_usec 10\nsystem_usec 7\n")
            io.write_text("8:0 rbytes=11 wbytes=13 rios=1 wios=1\n")
            self.assertEqual(MODULE._cgroup_cpu_ns(cpu), 17_000)
            self.assertEqual(MODULE._cgroup_io_bytes(io), 24)
            io.write_text("8:0 rios=1 wios=1\n")
            with self.assertRaisesRegex(ValueError, "byte counters"):
                MODULE._cgroup_io_bytes(io)

    def test_supervisor_process_identity_binds_exact_argv_and_cgroup(self):
        binary = MODULE.INSTALLED_BINARY
        command_line = (
            str(binary).encode("utf-8")
            + b"\0serve\0/etc/hydracache-perf/supervisor-074.toml\0"
        )
        control_group = "/system.slice/hydracache-performance-supervisor-074.service"
        self.assertEqual(
            MODULE._validate_process_identity(
                binary, control_group, command_line, f"0::{control_group}\n"
            ),
            hashlib.sha256(command_line).hexdigest(),
        )
        with self.assertRaisesRegex(ValueError, "command identity"):
            MODULE._validate_process_identity(
                binary, control_group, command_line + b"extra", f"0::{control_group}\n"
            )
        with self.assertRaisesRegex(ValueError, "process cgroup"):
            MODULE._validate_process_identity(
                binary, control_group, command_line, "0::/foreign.service\n"
            )

    def test_fixture_requires_exact_checkpoint_and_uses_shell_false(self):
        receipt = {
            "schema_version": 1,
            "role": "i74",
            "variant": "instrumented",
            "pair_index": 1,
            "position": 2,
            "cpuset": "2",
            "nice": 0,
            "elapsed_ns": 10,
            "cpu_ns": 9,
            "rss_peak_bytes": 8,
            "io_bytes": 4096,
            "checkpoint_write_bytes": 4096,
            "completed_operations": MODULE.OPERATIONS,
        }

        def fake_run(argv, **kwargs):
            kwargs["stdout"].write(b"h" * 4096)
            kwargs["stdout"].flush()
            self.assertFalse(kwargs["shell"])
            self.assertEqual(argv[0], MODULE.TASKSET)
            self.assertEqual(argv[3], MODULE.NICE)
            return subprocess.CompletedProcess(
                argv, 0, stderr=json.dumps(receipt).encode("utf-8") + b"\n"
            )

        with tempfile.TemporaryDirectory() as directory, mock.patch.object(
            MODULE.subprocess, "run", side_effect=fake_run
        ):
            result = MODULE.run_fixture(
                MODULE.INSTALLED_BINARY,
                Path(directory),
                "i74",
                "instrumented",
                1,
                2,
                "2",
            )
            self.assertEqual(result["checkpoint_write_bytes"], 4096)
            self.assertEqual((Path(directory) / "i74-p1-2.checkpoint").stat().st_size, 4096)

    def test_first_cpu_rejects_ambiguous_or_unbounded_values(self):
        self.assertEqual(MODULE._first_cpu("2-7,10"), "2")
        for invalid in ("", "02", "2,", "4096", "cpu0"):
            with self.subTest(invalid=invalid):
                with self.assertRaises(ValueError):
                    MODULE._first_cpu(invalid)

    def test_idle_guard_requires_digest_bound_root_observation(self):
        source = "a" * 40
        binary_sha256 = "b" * 64

        class FakeSocket:
            def __init__(self, active_campaign_absent=True):
                self.sent = None
                self.active_campaign_absent = active_campaign_absent

            def __enter__(self):
                return self

            def __exit__(self, *_args):
                return False

            def settimeout(self, _timeout):
                pass

            def connect(self, path):
                self.path = path

            def sendall(self, value):
                self.sent = json.loads(value)

            def recvmsg(self, _maximum):
                request = self.sent["request"]
                receipt = {"supervisor_binary": {"sha256": binary_sha256}}
                result = {
                    "schema_version": 1,
                    "installed_source_commit": source,
                    "active_campaign_absent": self.active_campaign_absent,
                    "fixture_binary": {},
                    "receipt_sha256": hashlib.sha256(
                        MODULE._canonical(receipt)
                    ).hexdigest(),
                    "receipt": receipt,
                }
                body = {
                    "schema_version": 1,
                    "request_id": request["request_id"],
                    "campaign_id": MODULE.HOST_SCOPE,
                    "ok": True,
                    "state_revision": 0,
                    "server_time_unix_seconds": 1,
                    "result": result,
                    "error_code": None,
                }
                response = dict(body)
                response["response_sha256"] = hashlib.sha256(
                    MODULE._canonical(body)
                ).hexdigest()
                return MODULE._canonical(response), [], 0, None

        socket_constants = (
            mock.patch.object(MODULE.socket, "AF_UNIX", 1, create=True),
            mock.patch.object(MODULE.socket, "SOCK_SEQPACKET", 5, create=True),
            mock.patch.object(MODULE.socket, "MSG_TRUNC", 0x20, create=True),
        )
        for patcher in socket_constants:
            patcher.start()
            self.addCleanup(patcher.stop)
        with mock.patch.object(MODULE.socket, "socket", return_value=FakeSocket()):
            MODULE.observe_idle_guard(
                MODULE.SUPERVISOR_SOCKET,
                source,
                binary_sha256,
                1,
                2,
                3,
                4,
            )
        with mock.patch.object(
            MODULE.socket,
            "socket",
            return_value=FakeSocket(active_campaign_absent=False),
        ):
            with self.assertRaisesRegex(ValueError, "not an idle"):
                MODULE.observe_idle_guard(
                    MODULE.SUPERVISOR_SOCKET,
                    source,
                    binary_sha256,
                    1,
                    2,
                    3,
                    4,
                )


if __name__ == "__main__":
    unittest.main()
