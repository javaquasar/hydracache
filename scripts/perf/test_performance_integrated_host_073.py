import importlib.util
import pathlib
import unittest


SCRIPT = pathlib.Path(__file__).with_name("performance_integrated_host_073.py")
SPEC = importlib.util.spec_from_file_location("performance_integrated_host_073", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
RUNNER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(RUNNER)


class IntegratedHostRunnerTests(unittest.TestCase):
    def test_surface_counts_preserve_remainder_and_total(self) -> None:
        counts = RUNNER.expected_surface_counts(123)
        self.assertEqual(counts, {
            "hc2": 58,
            "resp": 30,
            "hc1": 15,
            "direct": 10,
            "tag_invalidation": 5,
            "ttl_expire_refill": 5,
        })
        self.assertEqual(sum(counts.values()), 123)

    def test_order_is_counterbalanced_and_deterministic(self) -> None:
        orders = [RUNNER.pair_order(0, repeat) for repeat in range(1, 6)]
        self.assertEqual(orders.count(["I73", "C73"]), 3)
        self.assertEqual(orders.count(["C73", "I73"]), 2)
        self.assertNotEqual(orders[0], orders[1])

    def test_hodges_lehmann_uses_walsh_averages(self) -> None:
        self.assertEqual(RUNNER.hodges_lehmann([1, 2, 3, 4, 5]), 3.0)

    def test_validate_receipt_accepts_exact_complete_accounting(self) -> None:
        receipt = self.receipt("I73", 5_000, 50_000)
        RUNNER.validate_receipt(receipt, "I73", 5_000, 50_000, "5-6", "7")

    def test_validate_receipt_rejects_hidden_surface_loss(self) -> None:
        receipt = self.receipt("C73", 5_000, 50_000)
        receipt["surfaces"]["hc2"]["success"] -= 1
        with self.assertRaisesRegex(ValueError, "incomplete hc2"):
            RUNNER.validate_receipt(receipt, "C73", 5_000, 50_000, "5-6", "7")

    def test_regression_signs_match_guard_semantics(self) -> None:
        i73 = self.receipt("I73", 5_000, 50_000)
        c73 = self.receipt("C73", 5_000, 50_000)
        i73["observation"]["achieved_rate_per_second"] = 100.0
        c73["observation"]["achieved_rate_per_second"] = 98.0
        i73["resources"]["cpu_seconds_per_completed_operation"] = 2.0
        c73["resources"]["cpu_seconds_per_completed_operation"] = 2.06
        i73["observation"]["latency"]["p99_us"] = 10.0
        c73["observation"]["latency"]["p99_us"] = 10.3
        result = RUNNER.paired_regressions(i73, c73)
        self.assertAlmostEqual(result["goodput_relative_regression"], 0.02)
        self.assertAlmostEqual(result["cpu_per_operation_relative_regression"], 0.03)
        self.assertAlmostEqual(result["p99_relative_regression"], 0.03)

    @staticmethod
    def receipt(role: str, rate: int, operations: int) -> dict:
        expected = RUNNER.expected_surface_counts(operations)
        return {
            "schema_version": 1,
            "release": "0.73",
            "profile_id": RUNNER.PROFILE_ID,
            "role": role,
            "source_sha": RUNNER.I73_SHA if role == "I73" else RUNNER.C73_SHA,
            "offered_rate_per_second": rate,
            "operations": operations,
            "warmup_operations": RUNNER.WARMUP_OPERATIONS,
            "weights_percent": RUNNER.WEIGHTS,
            "daemon_cpu_set": "5-6",
            "loadgen_cpu_set": "7",
            "observation": {
                "offered": operations,
                "started": operations,
                "completed": operations,
                "successes": operations,
                "errors": 0,
                "timeouts": 0,
                "rejections": 0,
                "backlog_drained": True,
                "achieved_rate_per_second": float(rate),
                "latency": {"p99_us": 100.0},
            },
            "surfaces": {
                name: {
                    "attempted": count,
                    "success": count,
                    "rejected": 0,
                    "timeout": 0,
                    "late": 0,
                    "incomplete": 0,
                }
                for name, count in expected.items()
            },
            "resources": {
                "available": True,
                "cpu_seconds": 2.0,
                "cpu_seconds_per_completed_operation": 0.00004,
                "rss_before_bytes": 100_000,
                "rss_after_bytes": 110_000,
                "peak_rss_after_bytes": 120_000,
            },
            "events_received": expected["hc2"],
            "reconciliation_exact": True,
            "management_truth_zero": True,
            "durable": {
                "attempted": 1_000,
                "success": 1_000,
                "budget_rejections": 0,
                "reclaimed_bytes": 10,
                "reopen_verified": True,
                "corruption_rejected": True,
            },
            "promotable": False,
        }


if __name__ == "__main__":
    unittest.main()
