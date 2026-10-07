import copy
import importlib.util
import math
import pathlib
import tomllib
import unittest
import json
import tempfile

SCRIPT = pathlib.Path(__file__).with_name("performance_get_owner_screen_074.py")
SPEC = importlib.util.spec_from_file_location("screen", SCRIPT)
SCREEN = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SCREEN)
ROOT = SCRIPT.parents[2]


class ScreenTests(unittest.TestCase):
    def setUp(self):
        self.policy = tomllib.loads((ROOT / SCREEN.POLICY).read_text(encoding="utf-8"))
        self.seal = {"source_commit": "a" * 40, "off_binary_sha256": "off", "on_binary_sha256": "on"}
        cells = {cell["id"]: cell for cell in self.policy["cell"]}
        self.attempts = []
        for mode, pair, cell_id, role in SCREEN.schedule(self.policy):
            cell = cells[cell_id]
            enabled = mode == "ab" and role == "on"
            operations = cell["pipeline"] * cell["batches"]
            gross = 80000 if enabled and cell["affected"] else 100000
            receipt = {"measured_errors": 0, "measured_dispatches": operations, "measured_mutations": operations if cell["operation"] == "set" else 0, "profile_id": SCREEN.PROFILE, "source_commit": "a" * 40, "source_clean": True, "binary_sha256": "on" if enabled else "off", "get_owner_enabled": enabled, "promotable": False, "exact_response_validation": True, "exact_final_values_and_cardinality": True, "operation": cell["operation"], "payload_bytes": cell["payload"], "alternate_payload_bytes": cell["alternate_payload"], "pipeline": cell["pipeline"], "batches": cell["batches"], "read_chunk_bytes": cell["read_chunk"], "seed": 740074, "concurrency": 1, "warmup_batches": 5, "operations": operations, "write_calls": operations, "flush_calls": operations, "next_read_boundary_samples": cell["batches"], "workload_sha256": cell_id, "allocation": {"gross_allocated_bytes": gross, "live_before_bytes": 1000000, "live_after_bytes": 1000000, "peak_live_requested_bytes": 1010000, "peak_live_above_start_bytes": 10000}, "gross_allocated_bytes_per_operation": gross / operations, "next_read_boundary_live_max_bytes": 1001000}
            self.attempts.append({"compiled_feature_enabled": enabled, "mode": mode, "pair": pair, "cell": cell_id, "role": role, "exit_code": 0, "receipt": receipt})

    def analyse(self):
        return SCREEN.analyse(self.attempts, self.policy, self.seal)

    def test_complete_screen_pass_never_promotes(self):
        summary = self.analyse()
        self.assertEqual(summary["classification"], "screen-passed-full-d3-controls-still-required")
        self.assertFalse(summary["promotable"])
        self.assertFalse(summary["accepted_product_change"])
        self.assertEqual(summary["attempts_retained"], 180)

    def test_peak_increase_rejects_even_with_twenty_percent_allocation_saving(self):
        for attempt in self.attempts:
            if attempt["mode"] == "ab" and attempt["role"] == "on" and attempt["cell"] == "get-4k-p50":
                attempt["receipt"]["allocation"]["peak_live_above_start_bytes"] = 11000
                attempt["receipt"]["allocation"]["peak_live_requested_bytes"] = 1011000
        result = self.analyse()
        self.assertEqual(result["classification"], "rejected-local-allocation-memory-screen")
        self.assertEqual(result["red_cells"], ["get-4k-p50"])

    def test_retained_owner_increase_is_not_hidden_by_subtracted_start(self):
        for attempt in self.attempts:
            if attempt["mode"] == "ab" and attempt["role"] == "on":
                attempt["receipt"]["allocation"]["live_after_bytes"] += 1
        self.assertEqual(len(self.analyse()["red_cells"]), 9)

    def test_partial_failed_reordered_and_duplicate_attempts_are_rejected(self):
        original = copy.deepcopy(self.attempts)
        for mutate in [lambda rows: rows.pop(), lambda rows: rows[0].update(exit_code=1), lambda rows: rows.reverse(), lambda rows: rows.__setitem__(1, rows[0])]:
            self.attempts = copy.deepcopy(original)
            mutate(self.attempts)
            with self.assertRaises(ValueError):
                self.analyse()

    def test_missing_nonfinite_denominator_and_identity_drift_fail(self):
        original = copy.deepcopy(self.attempts)
        for key, value in [("source_commit", "b" * 40), ("source_clean", False), ("get_owner_enabled", True), ("binary_sha256", "on"), ("operations", 1), ("seed", 1), ("warmup_batches", 0), ("write_calls", 0), ("gross_allocated_bytes_per_operation", math.nan)]:
            self.attempts = copy.deepcopy(original)
            self.attempts[0]["receipt"][key] = value
            with self.assertRaises(ValueError):
                self.analyse()
        self.attempts = copy.deepcopy(original)
        del self.attempts[0]["receipt"]["allocation"]
        with self.assertRaises(KeyError):
            self.analyse()

    def test_pair_and_between_repeat_trace_drift_fail(self):
        self.attempts[0]["receipt"]["workload_sha256"] = "drift"
        with self.assertRaises(ValueError):
            self.analyse()
        self.attempts[1]["receipt"]["workload_sha256"] = "drift"
        with self.assertRaises(ValueError):
            self.analyse()

    def test_aa_noise_invalidates_without_threshold_adaptation(self):
        receipt = self.attempts[1]["receipt"]
        receipt["allocation"]["peak_live_above_start_bytes"] *= 1.02
        receipt["allocation"]["peak_live_requested_bytes"] = 1010200
        with self.assertRaisesRegex(ValueError, "AA peak noise"):
            self.analyse()

    def test_interval_retains_every_ratio_and_does_not_select_best_pair(self):
        ratios = [0.8, 0.81, 0.82, 0.83, 0.84]
        result = SCREEN.interval(ratios)
        self.assertEqual(result["paired_ratios"], ratios)
        self.assertLess(result["lower_95"], result["geometric_mean"])
        self.assertGreater(result["upper_95"], result["geometric_mean"])
        with self.assertRaises(ValueError):
            SCREEN.interval(ratios[:-1])

    def test_changed_policy_is_rejected_before_a_measurement(self):
        for key, value in [("minimum_affected_gross_allocation_reduction", 0.19), ("maximum_peak_live_above_start_ratio", 1.01), ("independent_ab_pairs_per_cell", 4)]:
            policy = copy.deepcopy(self.policy)
            policy["phase_a"][key] = value
            with self.assertRaisesRegex(ValueError, "sealed policy changed"):
                SCREEN.validate_policy(policy)

    def test_cell_and_feature_policy_drift_is_rejected(self):
        policy = copy.deepcopy(self.policy)
        policy["cell"][0]["payload"] += 1
        with self.assertRaisesRegex(ValueError, "sealed cell matrix"):
            SCREEN.validate_policy(policy)

    def test_complete_packet_replay_hash_checks_and_rejects_extra_files(self):
        with tempfile.TemporaryDirectory() as name:
            directory = pathlib.Path(name)
            seal = {**self.seal, "profile_id": SCREEN.PROFILE, "promotable": False, "schedule": SCREEN.schedule(self.policy)}
            SCREEN.write_new(directory / "seal.json", seal)
            for ordinal, attempt in enumerate(self.attempts, 1):
                mode, pair, cell_id, role = (attempt[key] for key in ["mode", "pair", "cell", "role"])
                path = directory / f"{ordinal:03d}-{mode}-{pair}-{cell_id}-{role}.json"
                SCREEN.write_new(path, attempt["receipt"])
                archived = {key: value for key, value in attempt.items() if key != "receipt"}
                archived.update(ordinal=ordinal, raw_receipt_sha256=SCREEN.digest(path))
                SCREEN.write_new(directory / f"{ordinal:03d}.attempt.json", archived)
            summary = self.analyse()
            SCREEN.write_new(directory / "summary.json", summary)
            self.assertEqual(SCREEN.replay(directory, self.policy), summary)
            SCREEN.write_new(directory / "extra.json", {})
            with self.assertRaisesRegex(ValueError, "extra files"):
                SCREEN.replay(directory, self.policy)
            path = directory / "001-aa-1-get-4k-p50-off.json"
            path.write_bytes(path.read_bytes() + b" ")
            with self.assertRaisesRegex(ValueError, "raw receipt hash"):
                SCREEN.replay(directory, self.policy)

    def test_dispatch_mutation_error_and_attempt_feature_drift_fail(self):
        original = copy.deepcopy(self.attempts)
        for key, value in [("measured_dispatches", 0), ("measured_mutations", 1), ("measured_errors", 1)]:
            self.attempts = copy.deepcopy(original)
            self.attempts[0]["receipt"][key] = value
            with self.assertRaises(ValueError):
                self.analyse()
        self.attempts = copy.deepcopy(original)
        self.attempts[0]["compiled_feature_enabled"] = True
        with self.assertRaisesRegex(ValueError, "attempt feature"):
            self.analyse()
        policy = copy.deepcopy(self.policy)
        policy["phase_a"]["feature"] = "serial-scratch"
        with self.assertRaisesRegex(ValueError, "feature policy"):
            SCREEN.validate_policy(policy)


if __name__ == "__main__":
    unittest.main()
