#!/usr/bin/env python3
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

PHASES = ["cold", "fill", "steady", "expire_or_delete", "reset", "refill", "post_idle", "shutdown"]
ROOT = Path(__file__).resolve().parent


class ProviderContractTest(unittest.TestCase):
    def test_fixture_normalizes_temporary_retained_reset_and_background_allocations(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            timeline = root / "timeline.jsonl"
            raw = root / "raw.jsonl"
            output = root / "normalized.json"
            timeline.write_text("".join(json.dumps({"phase": phase}) + "\n" for phase in PHASES), encoding="utf-8")
            samples = []
            live = [10, 110, 90, 50, 5, 65, 60, 0]
            for index, phase in enumerate(PHASES):
                samples.append({
                    "phase": phase,
                    "gross_bytes": live[index] + 20,
                    "live_bytes": live[index],
                    "peak_bytes": live[index] + 40,
                    "stacks": [
                        {"stack": "cache::retained", "bytes": live[index]},
                        {"stack": "payload=customer-secret", "bytes": 1},
                        {"stack": "[unknown]", "bytes": 2},
                    ],
                })
            raw.write_text("".join(json.dumps(sample) + "\n" for sample in samples), encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(ROOT / "system.py"), "normalize", "--input", str(raw), "--timeline", str(timeline), "--output", str(output)],
                check=False,
            )
            self.assertEqual(completed.returncode, 0)
            normalized = json.loads(output.read_text(encoding="utf-8"))
            self.assertEqual([item["phase"] for item in normalized["phases"]], PHASES)
            self.assertEqual(normalized["phases"][4]["diff_live_bytes"], -45)
            encoded = json.dumps(normalized)
            self.assertNotIn("customer-secret", encoded)
            self.assertIn("[unknown]", encoded)

    def test_reordered_timeline_fails_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            timeline = root / "timeline.jsonl"
            raw = root / "raw.jsonl"
            output = root / "normalized.json"
            reordered = PHASES.copy()
            reordered[1], reordered[2] = reordered[2], reordered[1]
            timeline.write_text("".join(json.dumps({"phase": phase}) + "\n" for phase in reordered), encoding="utf-8")
            raw.write_text("".join(json.dumps({"phase": phase, "gross_bytes": 0, "live_bytes": 0, "peak_bytes": 0}) + "\n" for phase in PHASES), encoding="utf-8")
            completed = subprocess.run(
                [sys.executable, str(ROOT / "system.py"), "normalize", "--input", str(raw), "--timeline", str(timeline), "--output", str(output)],
                check=False,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
            self.assertNotEqual(completed.returncode, 0)


if __name__ == "__main__":
    unittest.main()
