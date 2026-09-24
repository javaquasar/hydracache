import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("performance_allocation_attribution_073.py")
SPEC = importlib.util.spec_from_file_location("allocation_attribution_073", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class AllocationAttributionTests(unittest.TestCase):
    def test_matrix_is_the_preregistered_full_volume(self) -> None:
        self.assertEqual(MODULE.REPEATS, 5)
        self.assertEqual(MODULE.OPERATIONS, 8_192)
        self.assertEqual(MODULE.RATE, 20_000)
        self.assertEqual(MODULE.WARMUP_OPERATIONS, 4_096)
        self.assertEqual(len(MODULE.MODES) * len(MODULE.SCENARIOS) * MODULE.REPEATS, 120)
        for mode in MODULE.MODES:
            self.assertEqual(sum(order.count(mode) for order in MODULE.MODE_ORDERS), 5)

    def test_summary_uses_adjacent_mode_medians(self) -> None:
        receipts = []
        for scenario_index, scenario in enumerate(MODULE.SCENARIOS):
            base = float(scenario_index * 100)
            for repeat in range(MODULE.REPEATS):
                for mode_index, mode in enumerate(MODULE.MODES):
                    receipts.append(
                        {
                            "scenario": scenario,
                            "instrumentation_mode": mode,
                            "gross_allocated_bytes_per_operation": base
                            + mode_index * 10
                            + repeat,
                        }
                    )
        results = MODULE.summarize(receipts)
        self.assertEqual(len(results), len(MODULE.SCENARIOS))
        mixed = results[0]
        self.assertEqual(mixed["allocation_bytes_per_operation_median"]["off"], 2.0)
        deltas = {
            item["name"]: item["allocation_bytes_per_operation_delta"]
            for item in mixed["comparisons"]
        }
        self.assertEqual(deltas["off_to_counters_only"], 10.0)
        self.assertEqual(deltas["counters_only_to_observer_noop"], 10.0)
        self.assertEqual(deltas["observer_noop_to_production"], 10.0)
        self.assertEqual(deltas["off_to_production"], 30.0)


if __name__ == "__main__":
    unittest.main()
