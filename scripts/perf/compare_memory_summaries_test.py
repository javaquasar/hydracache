import importlib.util
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("compare-memory-summaries.py")
SPEC = importlib.util.spec_from_file_location("compare_memory_summaries", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def summary(commit: str, rss: int) -> dict:
    return {
        "release": "0.70",
        "provenance": {"source_commit": commit},
        "fingerprint": {"kernel": "Linux", "online_cpus": "4", "affinity": "0", "redis_image": "pinned", "targets": "hydra redis"},
        "cases": [{"experiment": "01", "target": "hydra", "metrics": {"vmrss_bytes": {"last": rss}}}],
    }


class CompareMemorySummariesTest(unittest.TestCase):
    def test_reports_relative_delta_without_gating(self) -> None:
        result = MODULE.compare(summary("new", 120), summary("old", 100))
        self.assertTrue(result["comparable_fingerprint"])
        self.assertFalse(result["gating"])
        self.assertAlmostEqual(result["deltas"][0]["metrics"]["vmrss_bytes"]["relative_delta"], 0.2)

    def test_first_run_is_explicitly_unavailable(self) -> None:
        result = MODULE.compare(summary("new", 120), None)
        self.assertFalse(result["baseline_available"])
        self.assertEqual(result["deltas"], [])


if __name__ == "__main__":
    unittest.main()
