"""No SSH/workloads: parsing and fail-closed read-only inventory checks."""

import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch


spec = importlib.util.spec_from_file_location(
    "rental", Path(__file__).with_name("performance_rental_preflight_074.py"))
rental = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rental)


class RentalPreflightTests(unittest.TestCase):
    def test_markers_absent_present_and_dangling_symlink_are_distinct(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "marker"
            self.assertEqual(rental.marker_state(path)["status"], "absent")
            path.write_text("not read by inventory")
            self.assertEqual(rental.marker_state(path)["status"], "present")
            with patch.object(Path, "lstat", side_effect=PermissionError):
                self.assertEqual(rental.marker_state(path)["status"], "unknown")
            with patch.object(rental, "directory_safe", return_value=False):
                self.assertEqual(rental.marker_state(path)["status"], "unknown")
            path.unlink()
            try:
                path.symlink_to(Path(folder) / "missing")
            except OSError:
                # Windows account may not have symlink privilege. Unix live guard remains explicit.
                return
            self.assertTrue(rental.marker_state(path)["symlink"])
            self.assertEqual(rental.marker_state(path)["status"], "present")

    def test_missing_parent_does_not_prove_marker_absence(self):
        with tempfile.TemporaryDirectory() as folder:
            self.assertEqual(rental.marker_state(Path(folder) / "missing" / "marker")["status"], "unknown")

    def test_cpu_guest_is_not_double_counted_and_iowait_is_separate(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "stat"
            path.write_text("cpu 1 2 3 4 5 6 7 8 99 99\ncpu0 1 2 3 4 5 6 7 8 99 99\n")
            before = rental.cpu_snapshot(path)
            self.assertEqual(len(before["cpu"]), 8)
            after = {name: [a + b for a, b in zip(values, [1, 0, 1, 6, 1, 0, 0, 1])]
                     for name, values in before.items()}
            delta = rental.cpu_deltas(before, after)["cpu"]
            self.assertEqual(delta["total_ticks"], 10)
            self.assertAlmostEqual(delta["busy_fraction"], .3)
            self.assertAlmostEqual(delta["iowait_fraction"], .1)
            self.assertAlmostEqual(delta["steal_fraction"], .1)

    def test_cpu_hotplug_backwards_missing_and_zero_counters_fail(self):
        before = {"cpu": [1] * 8}
        for after in ({"cpu0": [2] * 8}, {"cpu": [0] * 8}, before):
            with self.assertRaises(ValueError):
                rental.cpu_deltas(before, after)
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "stat"
            for text in ("cpu 1 2\n", "cpu0 1 2 3 4 5 6 7 8\n", "cpu -1 2 3 4 5 6 7 8\n"):
                path.write_text(text)
                with self.assertRaises(ValueError):
                    rental.cpu_snapshot(path)

    def test_reads_are_bounded_and_unknown_processes_fail_closed(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "value"
            path.write_bytes(b"x" * (rental.MAX_BYTES + 1))
            self.assertEqual(rental.read_value(path)["status"], "unavailable")
            proc = Path(folder) / "123"
            proc.mkdir()
            (proc / "comm").write_text("timing-controls\n")
            (proc / "cgroup").write_text("0::/test\n")
            result = rental.process_inventory(Path(folder))
            self.assertTrue(result["complete"])
            self.assertEqual(result["matched"][0]["pid"], 123)
            with patch.object(rental, "read_text", side_effect=PermissionError):
                self.assertFalse(rental.process_inventory(Path(folder))["complete"])

    def test_commands_are_read_only_and_failures_not_success(self):
        with patch.object(rental.shutil, "which", return_value=None):
            self.assertEqual(rental.command(["missing"])["status"], "unavailable")
        with patch.object(rental.shutil, "which", return_value="/bin/test"), \
                patch.object(rental.subprocess, "run", side_effect=rental.subprocess.TimeoutExpired("test", 10)):
            self.assertEqual(rental.command(["test"])["reason"], "TimeoutExpired")
        source = Path(rental.__file__).read_text()
        for forbidden in ("stop", "restart", "kill", "install", "start"):
            self.assertNotIn(f'"systemctl", "{forbidden}"', source)
        for forbidden in ("/cmdline", "/environ", "verification_key_hex", "write_text(", "write_bytes("):
            self.assertNotIn(forbidden, source)

    def test_allocator_discovery_failure_is_not_an_empty_success(self):
        with patch.object(rental, "command", return_value={"status": "command-failed", "exit_code": 1}):
            self.assertEqual(rental.allocator_inventory()["status"], "command-failed")
        with patch.object(rental, "command", return_value={"status": "available", "stdout": "libc.so\nlibjemalloc.so\n"}):
            self.assertEqual(rental.allocator_inventory()["stdout"], "libjemalloc.so")

    def test_protected_receipt_is_bounded_hashed_and_secret_fields_excluded(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / "receipt"
            path.write_text('{"source_commit":"abc", "secret":"must-not-retain"}')
            with patch.object(rental, "RECEIPT", path):
                result = rental.provisioning()
            self.assertEqual(result["fields"], {"source_commit": "abc"})
            self.assertEqual(result["raw_sha256"], rental.hashlib.sha256(path.read_bytes()).hexdigest())

    def test_no_non_linux_fallback_or_product_admission(self):
        with patch.object(rental.platform, "system", return_value="Windows"):
            with self.assertRaises(ValueError):
                rental.collect()
        source = Path(rental.__file__).read_text()
        for name in ("product_process_started", "service_operations_performed", "qualification_started",
                     "promotable", "admission_allowed", "allocator_native_active_resident_retained_proven"):
            self.assertIn(f'"{name}": False', source)


if __name__ == "__main__":
    unittest.main()
