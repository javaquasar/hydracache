"""Offline preparation fixtures; fake ELF metadata is never a built product."""

import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


SPEC = importlib.util.spec_from_file_location(
    "pilot", Path(__file__).with_name("performance_rental_pilot_prepare_074.py"))
pilot = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(pilot)
ROOT = Path(__file__).resolve().parents[2]
RUSTC = "rustc 1.94.0 (4a4ef493e 2026-03-02)\nhost: x86_64-unknown-linux-gnu"
CARGO = "cargo 1.94.0 (85eff7c80 2026-01-15)"


def metadata(args, root):
    if args[-1] == "HEAD":
        return pilot.SOURCE
    if args[-1] == "HEAD^{tree}":
        return "a" * 40
    if "status" in args:
        return ""
    return RUSTC if args[0] == "rustc" else CARGO


def fixture(root):
    binary = root / pilot.BINARY
    binary.parent.mkdir(parents=True)
    header = bytearray(64)
    header[:6], header[16:18], header[18:20] = b"\x7fELF\x02\x01", b"\x03\x00", b"\x3e\x00"
    binary.write_bytes(header + b"FAKE-ELF-NOT-A-PRODUCT")
    binary.chmod(0o755)
    (root / "Cargo.lock").write_bytes(b"root-lock-fixture\n")
    (root / pilot.OBSERVER / "Cargo.lock").write_bytes(b"tool-lock-fixture\n")
    log = root / "build.log"
    log.write_bytes(b"fake build log, no compiler was started\n")
    seal = {
        "schema_version": "rental-pilot-build-seal-074-v1", "source_commit": pilot.SOURCE,
        "source_tree": "a" * 40, "source_clean": True, "target": "x86_64-unknown-linux-gnu",
        "profile": "release", "features": [], "counting_allocator": False,
        "binary_sha256": pilot.digest(binary), "binary_bytes": binary.stat().st_size,
        "root_lock_sha256": pilot.digest(root / "Cargo.lock"),
        "observer_lock_sha256": pilot.digest(root / pilot.OBSERVER / "Cargo.lock"),
        "rustc_verbose": RUSTC, "cargo_version": CARGO, "build_command": pilot.BUILD_COMMAND,
        "build_log_sha256": pilot.digest(log),
    }
    path = root / "seal.json"
    path.write_text(json.dumps(seal))
    return path, log, seal


class PilotPreparationTests(unittest.TestCase):
    def setUp(self):
        # The positive fixture explicitly simulates Linux metadata on Windows;
        # NT chmod cannot set POSIX execute bits. It is never a real build proof.
        if pilot.os.name == "nt":
            original = pilot.safe_regular
            def linux_fixture_metadata(path):
                info = original(path)
                if path.as_posix().endswith(pilot.BINARY.as_posix()):
                    return pilot.os.stat_result((info.st_mode | 0o111, *info[1:]))
                return info
            patcher = patch.object(pilot, "safe_regular", side_effect=linux_fixture_metadata)
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_real_plan_binds_four_complete_inputs_and_starts_nothing(self):
        with patch.object(pilot.subprocess, "Popen") as spawn:
            result = pilot.plan(ROOT)
        spawn.assert_not_called()
        self.assertEqual(len(result["configs"]), 4)
        self.assertFalse(result["pilot_execution_allowed"])
        self.assertFalse(result["build_started"])
        self.assertEqual(result["host_reservation"], "BLOCKED_EXTERNAL_FLOCK_UNSAFE")
        self.assertFalse(result["workload_child_tree_deadline_implemented"])

    def test_config_numeric_unknown_and_workload_drift_are_rejected(self):
        original = pilot.strict_json
        for field, new in (("warmup_calls", 63), ("slots", 8.0), ("seed", True), ("unexpected", 1)):
            def changed(path):
                value = original(path)
                value[field] = new
                return value
            with patch.object(pilot, "strict_json", side_effect=changed):
                with self.assertRaises(ValueError):
                    pilot.plan(ROOT)

    def test_duplicate_nested_nonfinite_and_oversized_json_fail(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "input.json"
            for raw in (b'{"a":1,"a":2}', b'{"a":{"b":1,"b":2}}', b'{"a":NaN}', b'[]', b' ' * 65537):
                path.write_bytes(raw)
                with self.assertRaises(ValueError):
                    pilot.strict_json(path)

    def test_fake_build_hash_inspection_is_not_compilation_provenance_or_execution(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            seal, log, _ = fixture(root)
            with patch.object(pilot.platform, "system", return_value="Linux"), \
                    patch.object(pilot, "metadata_command", side_effect=metadata):
                result = pilot.verify_build(root, seal, log)
            self.assertTrue(result["linux_binary_verified"])
            for field in ("compilation_provenance_proven", "build_started", "workload_started",
                          "pilot_execution_allowed", "promotable", "admission_allowed"):
                self.assertFalse(result[field])

    def test_every_build_identity_drift_refuses_before_any_workload(self):
        mutations = {
            "schema_version": "wrong", "source_commit": "b" * 40, "source_tree": "b" * 40,
            "source_clean": False, "target": "windows", "profile": "debug", "features": ["get-owner"],
            "counting_allocator": True, "binary_sha256": "0" * 64, "binary_bytes": 1,
            "root_lock_sha256": "0" * 64, "observer_lock_sha256": "0" * 64,
            "rustc_verbose": "other", "cargo_version": "other", "build_command": ["cargo", "run"],
            "build_log_sha256": "0" * 64, "unknown": False,
        }
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            path, log, original = fixture(root)
            for field, value in mutations.items():
                changed = copy.deepcopy(original)
                changed[field] = value
                path.write_text(json.dumps(changed))
                with patch.object(pilot.platform, "system", return_value="Linux"), \
                        patch.object(pilot, "metadata_command", side_effect=metadata):
                    with self.subTest(field=field), self.assertRaises(ValueError):
                        pilot.verify_build(root, path, log)

    def test_non_linux_dirty_source_and_mid_verification_drift_fail(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            path, log, _ = fixture(root)
            with patch.object(pilot.platform, "system", return_value="Windows"):
                with self.assertRaises(ValueError):
                    pilot.verify_build(root, path, log)
            for changed in (lambda args, root: "M Cargo.lock" if "status" in args else metadata(args, root),
                            lambda args, root: "b" * 40 if args[-1] == "HEAD" else metadata(args, root)):
                with patch.object(pilot.platform, "system", return_value="Linux"), \
                        patch.object(pilot, "metadata_command", side_effect=changed):
                    with self.assertRaises(ValueError):
                        pilot.verify_build(root, path, log)
            heads = iter([pilot.SOURCE, "b" * 40])
            with patch.object(pilot.platform, "system", return_value="Linux"), \
                    patch.object(pilot, "metadata_command", side_effect=lambda args, root:
                                 next(heads) if args[-1] == "HEAD" else metadata(args, root)):
                with self.assertRaises(ValueError):
                    pilot.verify_build(root, path, log)

    def test_binary_format_wrong_arch_corruption_and_links_are_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            path, log, seal = fixture(root)
            binary = root / pilot.BINARY
            for header in (b"MZ" + bytes(62), b"\x7fELF\x01\x01" + bytes(58), bytes(64)):
                binary.write_bytes(header)
                seal["binary_bytes"] = len(header)
                seal["binary_sha256"] = pilot.digest(binary)
                path.write_text(json.dumps(seal))
                with patch.object(pilot.platform, "system", return_value="Linux"), \
                        patch.object(pilot, "metadata_command", side_effect=metadata):
                    with self.assertRaises(ValueError):
                        pilot.verify_build(root, path, log)
            link = root / "linked.json"
            pilot.os.link(path, link)
            with self.assertRaises(ValueError):
                pilot.strict_json(path)

    def test_metadata_argv_is_allowlisted_without_any_run_build_or_service(self):
        for args in (["cargo", "run"], ["cargo", "build"], ["ssh", "host"], ["systemctl", "stop"],
                     ["git", "status"], ["rustc", "-Vv"]):
            with patch.object(pilot.subprocess, "Popen") as spawn:
                with self.assertRaises(ValueError):
                    pilot.metadata_command(args, ROOT)
            spawn.assert_not_called()

    def test_metadata_failure_overflow_and_deadline_cleanup(self):
        with patch.object(pilot.subprocess, "Popen") as spawn:
            spawn.return_value.poll.return_value = 1
            spawn.return_value.returncode = 1
            with self.assertRaises(ValueError):
                pilot.metadata_command(["cargo", "+1.94.0", "-V"], ROOT)
        with patch.object(pilot.subprocess, "Popen") as spawn, \
                patch.object(pilot.time, "monotonic", side_effect=[0, 11]):
            spawn.return_value.poll.return_value = None
            with self.assertRaises(ValueError):
                pilot.metadata_command(["cargo", "+1.94.0", "-V"], ROOT)
            spawn.return_value.kill.assert_called_once()
            spawn.return_value.wait.assert_called_once_with(timeout=1)
        with patch.object(pilot.subprocess, "Popen") as spawn, \
                patch.object(pilot.os, "fstat") as info:
            spawn.return_value.poll.return_value = None
            info.return_value.st_size = 65537
            with self.assertRaises(ValueError):
                pilot.metadata_command(["cargo", "+1.94.0", "-V"], ROOT)
            spawn.return_value.kill.assert_called_once()

    def test_metadata_success_preserves_raw_output_and_disables_git_fsmonitor(self):
        def spawned(args, **kwargs):
            kwargs["stdout"].write(b"fixture-source\n")
            process = unittest.mock.Mock()
            process.poll.return_value = 0
            process.returncode = 0
            self.assertEqual(args[:3], ["git", "-c", "core.fsmonitor=false"])
            self.assertEqual(kwargs["stdin"], pilot.subprocess.DEVNULL)
            return process
        with patch.object(pilot.subprocess, "Popen", side_effect=spawned):
            self.assertEqual(pilot.metadata_command(["git", "--no-optional-locks", "rev-parse", "HEAD"], ROOT),
                             "fixture-source")

    def test_current_supervisor_busy_path_is_not_safe_external_reservation(self):
        server = (ROOT / "tools/long-run-supervisor-074/src/server.rs").read_text()
        self.assertIn("self.maintain_lease_expiry_with_backend(unix_seconds(), &mut lease_backend)?;", server)
        self.assertIn(".map_err(|error| ServerError::LeaseExpiry(error.to_string()))?", server)
        host = (ROOT / "tools/long-run-supervisor-074/src/host_execution.rs").read_text()
        self.assertIn("FileExt::try_lock_exclusive(&lock).map_err(map_lock_error)?;", host)
        service = (ROOT / "scripts/perf/long-run-supervisor-074/hydracache-performance-supervisor-074.service").read_text()
        self.assertIn("Restart=on-failure", service)
        code = Path(pilot.__file__).read_text()
        for forbidden in ("fcntl", "flock(", "os.killpg", 'sub.add_parser("run")', '"systemctl",', '"ssh",'):
            self.assertNotIn(forbidden, code)


if __name__ == "__main__":
    unittest.main()
