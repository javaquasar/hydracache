import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location("response_owner", Path(__file__).with_name("performance_response_owner_074.py"))
owner = importlib.util.module_from_spec(spec)
spec.loader.exec_module(owner)
ROOT = Path(__file__).resolve().parents[2]


class OwnerTests(unittest.TestCase):
    def fixture(self, directory, mutate=None):
        seal = {"profile_id": owner.PROFILE, "promotable": False, "source_commit": "source",
                "binary_sha256": "binary", "tool_lock_sha256": "lock", "contract_sha256": "contract"}
        owner.write_new(directory / "seal.json", seal)
        for index, (repeat, cell) in enumerate(owner.order()):
            expected = cell[3] if cell[1] == "get" and cell[2] else 0
            raw = {**seal, "schema_version": 1, "tier": "local-d1-owner-attribution", "product_mutation": False,
                   "cell_id": cell[0], "operation": cell[1], "hit": cell[2], "payload_bytes": cell[3],
                   "iterations": cell[4], "seed": 740074, "warmup_operations": 100, "cache_time_ms": 1000000,
                   "exact_result_pointer_and_state_validation": True,
                   "key_sha256": owner.corpus_hashes(cell[3])[0],
                   "payload_sha256": owner.corpus_hashes(cell[3])[1], "request_plan_sha256": cell[0]}
            for stage, amount in zip(owner.STAGES, [0, expected, expected]):
                raw[stage] = {"memory": {"gross_allocated_bytes": amount * cell[4],
                                        "successful_allocation_calls": int(amount > 0) * cell[4],
                                        "live_before_bytes": 100, "live_after_bytes": 100,
                                        "peak_live_requested_bytes": 100 + amount,
                                        "peak_live_above_start_bytes": amount},
                              "maximum_response_live_increment_bytes": 0,
                              "maximum_response_and_reduced_live_increment_bytes": amount}
            if mutate and index == 0:
                mutate(raw)
            stem = f"{index:02d}-{cell[0]}"
            path = directory / f"{stem}.raw.json"
            owner.write_new(path, raw)
            owner.write_new(directory / f"{stem}.attempt.json", {
                "index": index, "repeat": repeat, "cell_id": cell[0], "returncode": 0,
                "failure": None, "raw_sha256": owner.digest(path)})

    def test_policy_and_finite_order(self):
        owner.policy(ROOT)
        self.assertEqual(len(owner.order()), 18)
        for repeat in range(3):
            self.assertEqual(len({c[0] for r, c in owner.order() if r == repeat}), 6)

    def test_complete_archive_is_attribution_not_candidate(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)
            self.fixture(path)
            summary = owner.analyse(path)
            self.assertFalse(summary["candidate_authorized"])
            self.assertFalse(summary["promotable"])
            self.assertEqual(len(summary["cells"]["get-4096"]), 3)

    def test_scope_identity_result_workload_owner_and_residual_drift_rejected(self):
        original = [lambda r: r.update(source_commit="different"),
                    lambda r: r.update(promotable=True),
                    lambda r: r.update(iterations=0),
                    lambda r: r.update(payload_sha256="wrong"),
                    lambda r: r.update(exact_result_pointer_and_state_validation=False),
                    lambda r: r["reducer_only"]["memory"].update(live_after_bytes=101),
                    lambda r: r["dispatch_and_reduce"]["memory"].update(gross_allocated_bytes=1),
                    lambda r: r["dispatch_only"]["memory"].update(peak_live_above_start_bytes=1)]
        for mutate in original:
            with self.subTest(mutate=mutate), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp)
                self.fixture(path, mutate)
                with self.assertRaises(ValueError):
                    owner.analyse(path)

    def test_missing_failed_duplicate_reordered_and_corrupt_attempts_rejected(self):
        for mode in ["missing", "failed", "duplicate", "reordered", "corrupt"]:
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp)
                self.fixture(path)
                first = path / "00-get-empty.attempt.json"
                attempt = json.loads(first.read_text())
                if mode == "missing":
                    first.unlink()
                elif mode == "duplicate":
                    owner.write_new(path / "extra.attempt.json", attempt)
                elif mode == "corrupt":
                    (path / "00-get-empty.raw.json").write_text("{}")
                else:
                    attempt.update(returncode=1) if mode == "failed" else attempt.update(index=1)
                    first.write_text(json.dumps(attempt))
                with self.assertRaises(ValueError):
                    owner.analyse(path)

    def test_existing_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "receipt.json"
            owner.write_new(path, {"first": True})
            with self.assertRaises(FileExistsError):
                owner.write_new(path, {"second": True})
            self.assertEqual(json.loads(path.read_text()), {"first": True})


if __name__ == "__main__":
    unittest.main()
