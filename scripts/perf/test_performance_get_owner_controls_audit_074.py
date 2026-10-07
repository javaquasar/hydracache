import json
import pathlib
import tempfile
import unittest
from unittest import mock

import performance_get_owner_controls_audit_074 as auditor

ROOT = pathlib.Path(__file__).resolve().parents[2]


class Audit(unittest.TestCase):
    def test_retained_prelaunch_refusal_has_no_native_or_candidate_result(self):
        with mock.patch.object(auditor.runner.subprocess, "Popen", side_effect=AssertionError("offline audit must not spawn")):
            result = auditor.audit(ROOT, ROOT / auditor.PACKET)
        self.assertEqual(result["background_cpu_percent"], 57.8125)
        self.assertEqual(result["benchmark_processes_started"], 0)
        self.assertEqual(result["numerical_pairs_completed"], 0)
        self.assertFalse(result["compiled_feature_execution_confirmed"])
        self.assertFalse(result["product_performance_claim"])

    def test_tampered_witness_placement_summary_binary_and_file_selection_rejected(self):
        source = ROOT / auditor.PACKET
        mutations = [
            ("attempt-0001/attempt.json", lambda value: value.update(background_cpu_percent=9.0)),
            ("attempt-0001/attempt.json", lambda value: value.update(exit_code=0)),
            ("attempt-0001/attempt.json", lambda value: value.update(placement_before_warmup=True)),
            ("attempt-0001/attempt.json", lambda value: value.update(receipt={})),
            ("summary.json", lambda value: value.update(promotable=True)),
            ("seal.json", lambda value: value.update(schedule=[])),
            ("seal.json", lambda value: value["binaries"].pop("timing-on")),
        ]
        for file, mutate in mutations:
            with tempfile.TemporaryDirectory() as directory:
                packet = pathlib.Path(directory)
                for path in source.rglob("*"):
                    if path.is_file():
                        target = packet / path.relative_to(source); target.parent.mkdir(parents=True, exist_ok=True)
                        value = json.loads(path.read_text(encoding="utf-8"))
                        if path.relative_to(source).as_posix() == file: mutate(value)
                        target.write_text(json.dumps(value), encoding="utf-8")
                with self.assertRaises(ValueError): auditor.audit(ROOT, packet)
        with tempfile.TemporaryDirectory() as directory:
            packet = pathlib.Path(directory)
            for path in source.rglob("*"):
                if path.is_file():
                    target = packet / path.relative_to(source); target.parent.mkdir(parents=True, exist_ok=True); target.write_bytes(path.read_bytes())
            (packet / "attempt-0001/raw.json").touch()
            with self.assertRaises(ValueError): auditor.audit(ROOT, packet)


if __name__ == "__main__": unittest.main()
