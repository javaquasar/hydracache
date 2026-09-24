import importlib.util
import pathlib
import unittest


SCRIPT = pathlib.Path(__file__).with_name("performance_baseline_pilot_073.py")
SPEC = importlib.util.spec_from_file_location("performance_baseline_pilot_073", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
PILOT = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PILOT)


class BaselinePilotV2Tests(unittest.TestCase):
    def test_hodges_lehmann_uses_walsh_averages(self) -> None:
        self.assertEqual(PILOT.hodges_lehmann([1.0, 2.0, 3.0, 4.0, 5.0]), 3.0)

    def test_paired_overhead_keeps_regression_signs_and_absolute_deltas(self) -> None:
        off = self.receipt(goodput=100.0, cpu=2.0, p99=10.0, allocation=50.0, rss=100)
        production = self.receipt(
            goodput=98.0, cpu=2.2, p99=10.3, allocation=54.0, rss=112
        )
        overhead = PILOT.paired_overhead(off, production)
        self.assertAlmostEqual(overhead["goodput_relative_regression"], 0.02)
        self.assertAlmostEqual(overhead["cpu_per_operation_relative_regression"], 0.10)
        self.assertAlmostEqual(overhead["p99_relative_regression"], 0.03)
        self.assertEqual(overhead["allocation_absolute_overhead"], 4.0)
        self.assertEqual(overhead["rss_delta_absolute_overhead"], 12)

    @staticmethod
    def receipt(
        *, goodput: float, cpu: float, p99: float, allocation: float, rss: int
    ) -> dict:
        return {
            "observation": {
                "achieved_rate_per_second": goodput,
                "latency": {"p99_us": p99},
            },
            "cpu_seconds_per_operation": cpu,
            "gross_allocated_bytes_per_operation": allocation,
            "rss_before_bytes": 1_000,
            "rss_after_bytes": 1_000 + rss,
            "peak_rss_bytes": 1_000 + rss,
        }


if __name__ == "__main__":
    unittest.main()
