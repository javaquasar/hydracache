import copy
import json
import pathlib
import tempfile
import tomllib
import unittest
from unittest import mock

import performance_get_owner_controls_074 as runner

ROOT = pathlib.Path(__file__).resolve().parents[2]


class Controls(unittest.TestCase):
    def setUp(self):
        self.policy = tomllib.loads((ROOT / runner.POLICY).read_text(encoding="utf-8"))
        self.seal = {"source_commit": "1" * 40, "binaries": {name: {"sha256": name} for name in ["timing-off", "timing-on", "allocation-off", "allocation-on"]}}
        self.cells = {cell["id"]: cell for cell in self.policy["cell"]}

    def receipt(self, row):
        lane, mode, pair, cell_id, role = row
        cell = self.cells[cell_id]
        enabled = mode == "ab" and role == "on"
        result = {
            "profile_id": runner.PROFILE, "source_commit": "1" * 40, "source_clean": True,
            "binary_sha256": runner.variant(lane, mode, role), "get_owner_enabled": enabled,
            "allocation_profile_enabled": lane == "allocation", "instrumentation_enabled": False,
            "promotable": False, "exact_result_validation": True, "exact_final_values": True,
            "workload": {key: cell[key] for key in runner.FIELDS} | {"key_space": 16, "seed": 740074},
            "workload_sha256": cell_id, "runtime_workers": 2,
            "latency_samples": cell["operations"] // cell["pipeline"],
            "latency_unit": "closed_loop_operation" if cell["pipeline"] == 1 else "closed_loop_pipeline_batch",
            "p50_us": 10, "p95_us": 20, "p99_us": 30,
            "elapsed_seconds": 2.0, "cpu_seconds": 2.0,
            "goodput_operations_per_second": cell["operations"] / 2,
            "cpu_nanoseconds_per_operation": 2e9 / cell["operations"],
            "gross_allocated_bytes_per_operation": None,
        }
        if lane == "allocation":
            result["allocation"] = {"gross_allocated_bytes": cell["operations"] * 1000}
            result["gross_allocated_bytes_per_operation"] = 1000.0
        return result

    def attempts(self):
        return [dict(zip(["lane", "mode", "pair", "cell", "role"], row)) | {"receipt": self.receipt(row), "exit_code": 0, "affinity_applied": True, "priority_applied": True, "placement_before_warmup": True, "background_cpu_percent": 1.0} for row in runner.schedule(self.policy)]

    def test_sealed_matrix_schedule_and_roles(self):
        runner.policy_check(self.policy)
        rows = runner.schedule(self.policy)
        self.assertEqual(len(rows), 400)
        self.assertEqual(rows[0], ("timing", "aa", 1, "embedded-get-4k-c1", "off"))
        self.assertEqual(rows[20][-1], "on")
        self.assertEqual(runner.variant("allocation", "aa", "on"), "allocation-off")

    def test_policy_cannot_relax_boundaries_or_matrix(self):
        for section, key, value in [("guards", "maximum_cpu_ratio", 1.04), ("pending", "hc1_hc2_controls_required", False), ("screen", "minimum_timing_cpu_seconds", 0.01), ("screen", "cpu_ids", [2, 3])]:
            changed = copy.deepcopy(self.policy); changed[section][key] = value
            with self.assertRaises(ValueError): runner.policy_check(changed)
        changed = copy.deepcopy(self.policy); changed["cell"][0]["operations"] *= 2
        with self.assertRaises(ValueError): runner.policy_check(changed)

    def test_complete_unchanged_controls_still_cannot_promote(self):
        result = runner.analyse(self.attempts(), self.policy, self.seal)
        self.assertEqual(result["classification"], "local-b0-guards-passed-full-d3-still-required")
        self.assertFalse(result["accepted_product_change"])
        self.assertFalse(result["product_performance_claim"])
        self.assertEqual(len(result["results"]), 20)
        self.assertEqual(len(result["results"][0]["intervals"]["p99_us"]["paired_ratios"]), 5)

    def test_partial_reordered_failed_unplaced_and_best_pair_selection_rejected(self):
        rows = self.attempts()
        mutations = [rows[:-1], rows[1:] + rows[:1]]
        for key, value in [("exit_code", 1), ("placement_before_warmup", False), ("affinity_applied", False), ("background_cpu_percent", float("nan"))]:
            changed = copy.deepcopy(rows); changed[0][key] = value; mutations.append(changed)
        for changed in mutations:
            with self.assertRaises(ValueError): runner.analyse(changed, self.policy, self.seal)

    def test_timing_rejects_allocation_metrics_or_profiled_binary(self):
        row = runner.schedule(self.policy)[0]
        for key, value in [("allocation_profile_enabled", True), ("gross_allocated_bytes_per_operation", 0.0), ("instrumentation_enabled", True), ("binary_sha256", "allocation-off")]:
            receipt = self.receipt(row); receipt[key] = value
            with self.assertRaises(ValueError): runner.validate(receipt, self.cells[row[3]], row[0], False, self.seal)
        receipt = self.receipt(row); receipt["allocation"] = {}
        with self.assertRaises(ValueError): runner.validate(receipt, self.cells[row[3]], row[0], False, self.seal)

    def test_workload_latency_denominator_and_precision_guard(self):
        row = runner.schedule(self.policy)[0]
        for key, value in [("latency_samples", 1), ("cpu_seconds", 0.01), ("latency_unit", "scheduled_operation"), ("cpu_nanoseconds_per_operation", float("inf")), ("goodput_operations_per_second", 10)]:
            receipt = self.receipt(row); receipt[key] = value
            with self.assertRaises(ValueError): runner.validate(receipt, self.cells[row[3]], row[0], False, self.seal)
        receipt = self.receipt(row); receipt["workload"]["seed"] = 74
        with self.assertRaises(ValueError): runner.validate(receipt, self.cells[row[3]], row[0], False, self.seal)

    def test_noise_invalidates_without_changing_floor(self):
        rows = self.attempts(); rows[1]["receipt"]["p99_us"] = 34
        with self.assertRaises(ValueError): runner.analyse(rows, self.policy, self.seal)

    def test_native_or_pipeline_batch_regression_is_red(self):
        for selected in ["embedded-get-4k-c1", "resp-get-4k-p50-c8"]:
            rows = self.attempts()
            for row in rows:
                if row["mode"] == "ab" and row["role"] == "on" and row["lane"] == "timing" and row["cell"] == selected:
                    row["receipt"]["p99_us"] = 32
            result = runner.analyse(rows, self.policy, self.seal)
            self.assertEqual(result["classification"], "local-b0-guard-red")
            self.assertFalse(result["promotable"])

    def test_trace_must_match_across_lanes_and_pairs(self):
        rows = self.attempts(); rows[-1]["receipt"]["workload_sha256"] = "other"
        with self.assertRaises(ValueError): runner.analyse(rows, self.policy, self.seal)

    def test_replay_hashes_every_raw_file_and_rejects_extra_selection(self):
        rows = self.attempts()
        seal = self.seal | {"contract_sha256": "contract", "policy": self.policy, "schedule": [list(row) for row in runner.schedule(self.policy)]}
        with tempfile.TemporaryDirectory() as directory:
            packet = pathlib.Path(directory)
            runner.write_new(packet / "seal.json", seal)
            for ordinal, row in enumerate(rows, 1):
                attempt = packet / f"attempt-{ordinal:04}"; attempt.mkdir()
                runner.write_new(attempt / "raw.json", row["receipt"])
                row["raw_sha256"] = runner.digest(attempt / "raw.json")
                runner.write_new(attempt / "attempt.json", row)
                (attempt / "ready").touch(); (attempt / "go").touch()
            runner.write_new(packet / "summary.json", runner.analyse(rows, self.policy, seal))
            runner.replay(packet, self.policy, "contract")
            extra = packet / "best-pairs.json"; extra.touch()
            with self.assertRaises(ValueError): runner.replay(packet, self.policy, "contract")
            extra.unlink()
            raw = packet / "attempt-0001/raw.json"; raw.write_bytes(raw.read_bytes() + b" ")
            with self.assertRaises(ValueError): runner.replay(packet, self.policy, "contract")

    def test_run_retains_background_invalidation_without_spawning_or_retrying(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory).resolve()
            policy = root / runner.POLICY; policy.parent.mkdir(parents=True)
            policy.write_bytes((ROOT / runner.POLICY).read_bytes())
            lock = root / "tools/get-owner-controls-074/Cargo.lock"; lock.parent.mkdir(parents=True)
            lock.write_bytes(b"synthetic lock")
            binaries = {}
            for name in self.seal["binaries"]:
                binary = root / name; binary.write_bytes(name.encode()); binaries[name] = binary
            output = root / "target/invalid"
            with mock.patch.object(runner, "git", side_effect=lambda root, *args: "1" * 40 if args == ("rev-parse", "HEAD") else ""), mock.patch.object(runner.sys, "platform", "win32"), mock.patch.object(runner.subprocess, "check_output", return_value="synthetic compiler"), mock.patch.object(runner, "background_cpu_percent", return_value=10.01), mock.patch.object(runner.subprocess, "Popen") as spawn:
                result = runner.run(root, binaries, output)
            self.assertEqual(result["classification"], "invalidated")
            self.assertEqual(result["attempts_retained"], 1)
            spawn.assert_not_called()
            attempt = json.loads((output / "attempt-0001/attempt.json").read_text())
            self.assertIn("background CPU", attempt["failure"])
            self.assertFalse(attempt["placement_before_warmup"])
            self.assertTrue((output / "seal.json").is_file())
            self.assertTrue((output / "summary.json").is_file())


if __name__ == "__main__": unittest.main()
