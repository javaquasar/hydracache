import json
import subprocess
import tempfile
import unittest
from pathlib import Path


class SummarizeCiInstructionTest(unittest.TestCase):
    @staticmethod
    def record(name="cache_work::cache_get_hit", head=100, base=100, diff="0"):
        return {
            "module_path": name,
            "id": None,
            "version": "6",
            "profiles": [
                {
                    "tool": "Callgrind",
                    "summaries": {
                        "total": {
                            "regressions": [],
                            "summary": {
                                "Callgrind": {
                                    "Ir": {
                                        "diffs": {"diff_pct": diff, "factor": "1"},
                                        "metrics": {"Both": [{"Int": head}, {"Int": base}]},
                                    }
                                }
                            },
                        }
                    },
                }
            ],
        }

    def test_wraps_matching_benchmarks_and_preserves_non_ship_boundary(self):
        root = Path(__file__).resolve().parents[2]
        script = root / "scripts/perf/summarize-ci-instruction.py"
        record = self.record()
        with tempfile.TemporaryDirectory() as directory:
            tmp = Path(directory)
            base = tmp / "base.ndjson"
            head = tmp / "head.ndjson"
            output = tmp / "report.json"
            base.write_text("runner preamble\n" + json.dumps(record) + "\n", encoding="utf-8")
            head.write_text(json.dumps(record) + "\n", encoding="utf-8")
            subprocess.run(
                [
                    "python",
                    str(script),
                    "--base",
                    str(base),
                    "--head",
                    str(head),
                    "--base-sha",
                    "a" * 40,
                    "--head-sha",
                    "b" * 40,
                    "--status",
                    "0",
                    "--output",
                    str(output),
                ],
                check=True,
            )
            report = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual(report["verdict"], "accepted")
            self.assertFalse(report["claim_boundary"]["ship_evidence_eligible"])
            self.assertFalse(report["claim_boundary"]["latency_claim"])
            self.assertEqual(report["comparisons"][0]["base_ir"], 100)
            self.assertEqual(report["comparisons"][0]["head_ir"], 100)

    def test_rejects_identity_drift(self):
        root = Path(__file__).resolve().parents[2]
        script = root / "scripts/perf/summarize-ci-instruction.py"
        with tempfile.TemporaryDirectory() as directory:
            tmp = Path(directory)
            base = tmp / "base.ndjson"
            head = tmp / "head.ndjson"
            base.write_text(json.dumps(self.record("base")) + "\n", encoding="utf-8")
            head.write_text(json.dumps(self.record("head")) + "\n", encoding="utf-8")
            result = subprocess.run(
                [
                    "python",
                    str(script),
                    "--base",
                    str(base),
                    "--head",
                    str(head),
                    "--base-sha",
                    "a",
                    "--head-sha",
                    "b",
                    "--status",
                    "0",
                    "--output",
                    str(tmp / "report.json"),
                ],
                capture_output=True,
                text=True,
            )
            self.assertNotEqual(result.returncode, 0)
            self.assertIn("identity sets differ", result.stderr)


if __name__ == "__main__":
    unittest.main()
