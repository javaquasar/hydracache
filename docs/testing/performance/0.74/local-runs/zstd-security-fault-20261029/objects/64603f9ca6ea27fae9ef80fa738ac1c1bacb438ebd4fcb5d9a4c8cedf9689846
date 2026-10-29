#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("memory_instrumentation_overhead_071.py")
SPEC = importlib.util.spec_from_file_location("memory_instrumentation_overhead_071", SCRIPT)
assert SPEC and SPEC.loader
overhead = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = overhead
SPEC.loader.exec_module(overhead)


class MemoryInstrumentationOverhead071Tests(unittest.TestCase):
    def samples(self) -> list[dict[str, object]]:
        samples = []
        for workload, _dimensions, _phase in overhead.WORKLOADS:
            for mode in overhead.MODES:
                for repetition in range(1, 4):
                    factor = {"off": 1.0, "production": 1.1, "profile": 1.2}[mode]
                    samples.append(
                        {
                            "workload": workload,
                            "mode": mode,
                            "repetition": repetition,
                            "metrics": {
                                "rss_bytes": int(1000 * factor),
                                "rps": 0.0 if workload == "cold" else 100 / factor,
                                "p99_ns": int(100 * factor),
                                "cpu_seconds_per_request": None if workload == "cold" else factor,
                                "context_switches": int(10 * factor),
                                "errors": 0,
                            },
                        }
                    )
        return samples

    def test_summary_freezes_production_vs_off_without_candidate_data(self) -> None:
        comparisons, envelope = overhead.summarize(self.samples(), 3)
        self.assertEqual(len(comparisons), 5)
        self.assertEqual(envelope["rss_delta_bytes"], 100.0)
        self.assertAlmostEqual(envelope["rss_regression_fraction"], 0.1)
        self.assertIsNone(comparisons[0]["production_vs_off"]["rps_regression_fraction"])

    def test_summary_rejects_missing_or_failed_sample(self) -> None:
        with self.assertRaises(overhead.OverheadError):
            overhead.summarize(self.samples()[:-1], 3)
        samples = self.samples()
        samples[0]["metrics"]["errors"] = 1  # type: ignore[index]
        with self.assertRaises(overhead.OverheadError):
            overhead.summarize(samples, 3)

    def test_mode_order_rotates_to_reduce_monotonic_drift(self) -> None:
        self.assertEqual(overhead.mode_order(0), ("off", "production", "profile"))
        self.assertEqual(overhead.mode_order(1), ("production", "profile", "off"))
        self.assertEqual(overhead.mode_order(2), ("profile", "off", "production"))


if __name__ == "__main__":
    unittest.main()
