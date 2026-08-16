import importlib.util
import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("verify-memory-telemetry-coverage.py")
SPEC = importlib.util.spec_from_file_location("verify_memory_coverage", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


class MemoryTelemetryCoverageTest(unittest.TestCase):
    def write_case(self, root: Path, checkpoint: float, samples: list[float]) -> None:
        (root / "telemetry").mkdir(parents=True)
        (root / "checkpoints.tsv").write_text(
            f"timestamp\tphase\tdetail\n{checkpoint}\tfinal\tdone\n",
            encoding="utf-8",
        )
        (root / "telemetry" / "telemetry.jsonl").write_text(
            "".join(json.dumps({"timestamp_unix": value}) + "\n" for value in samples),
            encoding="utf-8",
        )

    def test_accepts_final_checkpoint_covered_within_sampling_tolerance(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_case(root, 12.0, [9.0, 10.0, 11.0])
            self.assertIsNone(MODULE.coverage_problem(root, 1.0))

    def test_rejects_checkpoint_after_collector_expired(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.write_case(root, 120.0, [9.0, 10.0, 11.0])
            problem = MODULE.coverage_problem(root, 1.0)
            self.assertIn("newer than the final telemetry sample", problem)


if __name__ == "__main__":
    unittest.main()
