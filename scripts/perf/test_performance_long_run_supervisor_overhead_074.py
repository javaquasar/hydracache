import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest


MODULE_PATH = Path(__file__).with_name("performance_long_run_supervisor_overhead_074.py")
SPEC = importlib.util.spec_from_file_location(
    "performance_long_run_supervisor_overhead_074", MODULE_PATH
)
assert SPEC and SPEC.loader
overhead = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = overhead
SPEC.loader.exec_module(overhead)


def snapshot(**overrides: object) -> object:
    values = {
        "monotonic_ns": 1_000_000_000,
        "start_ticks": 700,
        "cpu_usage_usec": 10_000,
        "cpu_user_usec": 7_000,
        "cpu_system_usec": 3_000,
        "rss_bytes": 4_096,
        "memory_current_bytes": 8_192,
        "memory_peak_bytes": 16_384,
        "read_bytes": 100,
        "write_bytes": 200,
        "cpu_pressure_some_usec": 1_000,
        "cpu_pressure_full_usec": 100,
        "voluntary_context_switches": 3,
        "involuntary_context_switches": 1,
        "threads": 1,
        "pids_current": 1,
        "cpuset": "0-1",
    }
    values.update(overrides)
    return overhead.SupervisorSnapshot(**values)


class SupervisorOverheadTests(unittest.TestCase):
    def test_snapshot_parses_process_and_cgroup_v2_counters(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            process = root / "proc" / "42"
            cgroup = root / "cgroup" / "system.slice" / "supervisor.service"
            process.mkdir(parents=True)
            cgroup.mkdir(parents=True)
            fields = ["0"] * 20
            fields[0] = "S"
            fields[19] = "1234"
            (process / "stat").write_text(
                "42 (supervisor worker) " + " ".join(fields) + "\n", encoding="utf-8"
            )
            (process / "status").write_text(
                "VmRSS:\t5 kB\nThreads:\t2\n"
                "voluntary_ctxt_switches:\t8\nnonvoluntary_ctxt_switches:\t2\n",
                encoding="utf-8",
            )
            (cgroup / "cpu.stat").write_text(
                "usage_usec 100\nuser_usec 70\nsystem_usec 30\n", encoding="utf-8"
            )
            (cgroup / "io.stat").write_text(
                "8:0 rbytes=1000 wbytes=2000 rios=1 wios=2\n"
                "8:16 rbytes=500 wbytes=700 rios=1 wios=1\n",
                encoding="utf-8",
            )
            (cgroup / "cpu.pressure").write_text(
                "some avg10=0.00 avg60=0.00 avg300=0.00 total=20\n"
                "full avg10=0.00 avg60=0.00 avg300=0.00 total=4\n",
                encoding="utf-8",
            )
            (cgroup / "memory.current").write_text("8192\n", encoding="utf-8")
            (cgroup / "memory.peak").write_text("16384\n", encoding="utf-8")
            (cgroup / "pids.current").write_text("2\n", encoding="utf-8")
            (cgroup / "cpuset.cpus.effective").write_text("0-1\n", encoding="utf-8")

            observed = overhead.read_supervisor_snapshot(
                root / "proc", cgroup, 42, monotonic_ns=lambda: 99
            )
            self.assertEqual(observed.start_ticks, 1234)
            self.assertEqual(observed.cpu_usage_usec, 100)
            self.assertEqual(observed.rss_bytes, 5_120)
            self.assertEqual(observed.read_bytes, 1_500)
            self.assertEqual(observed.write_bytes, 2_700)
            self.assertEqual(observed.cpu_pressure_full_usec, 4)
            self.assertEqual(observed.cpuset, "0-1")

    def test_summary_reports_exact_deltas_and_passes_frozen_idle_screen(self) -> None:
        result = overhead.summarize(
            [
                snapshot(),
                snapshot(
                    monotonic_ns=11_000_000_000,
                    cpu_usage_usec=40_000,
                    cpu_user_usec=25_000,
                    cpu_system_usec=15_000,
                    rss_bytes=8_192,
                    memory_current_bytes=12_288,
                    memory_peak_bytes=20_480,
                    read_bytes=1_100,
                    write_bytes=1_200,
                    cpu_pressure_some_usec=11_000,
                    cpu_pressure_full_usec=2_100,
                    voluntary_context_switches=13,
                    involuntary_context_switches=3,
                    threads=2,
                    pids_current=2,
                ),
            ],
            maximum_cpu_percent=0.5,
            maximum_rss_bytes=64 * 1024 * 1024,
            maximum_io_bytes_per_second=1024 * 1024,
        )
        self.assertEqual(result["elapsed_seconds"], 10.0)
        self.assertAlmostEqual(result["cpu_percent"], 0.3)
        self.assertEqual(result["read_bytes_delta"], 1_000)
        self.assertEqual(result["write_bytes_delta"], 1_000)
        self.assertEqual(result["cpu_pressure_some_ratio"], 0.001)
        self.assertTrue(result["idle_screen_passed"])

    def test_identity_drift_and_budget_failure_are_fail_closed(self) -> None:
        with self.assertRaisesRegex(ValueError, "identity changed"):
            overhead.summarize(
                [snapshot(), snapshot(monotonic_ns=2_000_000_000, start_ticks=701)],
                0.5,
                64 * 1024 * 1024,
                1024 * 1024,
            )
        result = overhead.summarize(
            [
                snapshot(),
                snapshot(monotonic_ns=2_000_000_000, cpu_usage_usec=20_000),
            ],
            0.5,
            64 * 1024 * 1024,
            1024 * 1024,
        )
        self.assertFalse(result["idle_screen_passed"])

    def test_control_group_cannot_escape_the_cgroup_root(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "safe").mkdir()
            self.assertEqual(
                overhead.resolve_cgroup(root, "/safe"), (root / "safe").resolve()
            )
            with self.assertRaisesRegex(ValueError, "canonical"):
                overhead.resolve_cgroup(root, "/../outside")


if __name__ == "__main__":
    unittest.main()
