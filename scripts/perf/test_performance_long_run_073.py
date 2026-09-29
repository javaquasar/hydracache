import importlib.util
import json
import pathlib
import tempfile
import unittest
from argparse import Namespace
from unittest import mock


SCRIPT = pathlib.Path(__file__).with_name("performance_long_run_073.py")
SPEC = importlib.util.spec_from_file_location("performance_long_run_073", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC.loader is not None
SPEC.loader.exec_module(MODULE)


def resource(value: int = 1) -> dict:
    return {
        "available": True,
        "cpu_seconds": float(value),
        "rss_bytes": value,
        "peak_rss_bytes": value,
        "anonymous_pss_bytes": value,
        "file_pss_bytes": value,
        "minor_faults": value,
        "major_faults": 0,
        "threads": 2,
        "file_descriptors": 3,
    }


class PerformanceLongRun073Tests(unittest.TestCase):
    def test_final_candidate_and_historical_reanalysis_identities_are_separate(self) -> None:
        self.assertEqual(
            MODULE.C73_SHA, "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
        )
        harness = (
            SCRIPT.parents[2] / "tools" / "performance-integrated-073" / "src" / "main.rs"
        ).read_text(encoding="utf-8")
        self.assertIn(
            'const C73_SHA: &str = "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b";',
            harness,
        )
        self.assertNotIn(
            'const C73_SHA: &str = "7e3070894aa51af96cdcb3e350eff923a309e1fa";',
            harness,
        )
        reanalyzer = SCRIPT.with_name("reanalyze_performance_long_run_073.py").read_text(
            encoding="utf-8"
        )
        self.assertIn(
            'ORIGINAL_C73_SHA = "7e3070894aa51af96cdcb3e350eff923a309e1fa"',
            reanalyzer,
        )

    def test_verify_inputs_returns_identity_and_rejects_overlay_drift(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = pathlib.Path(temporary)
            files = {}
            for name in ["i73_harness", "c73_harness", "i73_server", "c73_server", "scenario"]:
                files[name] = root / name
                files[name].write_text(name, encoding="utf-8")
            for name in ["i73_overlay", "c73_overlay"]:
                files[name] = root / name
                files[name].mkdir()
                (files[name] / "tool.txt").write_text("same", encoding="utf-8")
            identity = MODULE.verify_inputs(Namespace(**files))
            self.assertEqual(identity["paths"]["scenario"], files["scenario"].resolve())
            self.assertEqual(len(identity["overlay_sha256"]), 64)

            (files["c73_overlay"] / "tool.txt").write_text("drift", encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "not byte-identical"):
                MODULE.verify_inputs(Namespace(**files))

    def test_theil_sen_and_moving_block_bound_are_deterministic(self) -> None:
        points = [(float(index * 60), float(index * 10)) for index in range(36)]
        self.assertAlmostEqual(MODULE.theil_sen(points), 1.0 / 6.0)
        left = MODULE.moving_block_upper_bound(points, block_samples=12, iterations=25, seed=7)
        right = MODULE.moving_block_upper_bound(points, block_samples=12, iterations=25, seed=7)
        self.assertEqual(left, right)
        self.assertEqual(left["samples"], 36)
        self.assertEqual(left["bootstrap_method"], "moving-block-adjacent-slope-v1")
        self.assertAlmostEqual(left["upper_95_bytes_per_second"], 1.0 / 6.0)
        self.assertGreaterEqual(
            left["upper_95_bytes_per_second"], left["theil_sen_bytes_per_second"]
        )

    def test_checkpoint_validator_rejects_missing_reconciled_final(self) -> None:
        rows = [
            {
                "schema_version": 1,
                "sequence": 0,
                "kind": "pre-work",
                "elapsed_seconds": 0.0,
                "owner_reconciled": False,
                "resources": resource(),
            },
            {
                "schema_version": 1,
                "sequence": 1,
                "kind": "periodic-work",
                "elapsed_seconds": 60.0,
                "owner_reconciled": False,
                "resources": resource(),
            },
            {
                "schema_version": 1,
                "sequence": 2,
                "kind": "final-work",
                "elapsed_seconds": 61.0,
                "owner_reconciled": False,
                "resources": resource(),
            },
        ]
        with tempfile.TemporaryDirectory() as temporary:
            path = pathlib.Path(temporary) / "checkpoints.jsonl"
            path.write_text("".join(json.dumps(row) + "\n" for row in rows), encoding="utf-8")
            with self.assertRaisesRegex(ValueError, "checkpoint series|reconciled final"):
                MODULE.validate_checkpoints(path, minimum_periodic=1, resources_required=True)

    def test_host_commands_bind_exact_phase_volume_and_checkpoint_shape(self) -> None:
        command, operations, interval = MODULE.command_for_role(
            pathlib.Path("harness"),
            pathlib.Path("server"),
            "I73",
            pathlib.Path("receipt.json"),
            pathlib.Path("checkpoints.jsonl"),
            "5-6",
            "7",
            phase="qualification",
            host_mode=True,
        )
        self.assertEqual(operations, 259_200_000)
        self.assertEqual(interval, 60)
        self.assertEqual(command[:3], ["taskset", "--cpu-list", "7"])
        self.assertIn("integrated-long-run-073-v1", command)
        self.assertIn("259200000", command)
        self.assertIn("300", command)
        self.assertEqual(command[-1], "false")

    def test_source_has_one_serial_role_order_and_no_retry_loop(self) -> None:
        source = SCRIPT.read_text(encoding="utf-8")
        self.assertIn('for role in ["I73", "C73"]:', source)
        self.assertIn('"automatic_retry_allowed": False', source)
        self.assertNotIn("for retry in", source)

    def test_canary_does_not_accept_harness_startup_failure_as_expected_red(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = pathlib.Path(temporary) / "canary"
            options = Namespace(output=output, daemon_cpu_set="5-6", loadgen_cpu_set="7")
            inputs = {
                "paths": {
                    "c73_harness": pathlib.Path("harness"),
                    "c73_server": pathlib.Path("server"),
                }
            }
            failed_attempt = {
                "exit_code": 1,
                "timed_out": False,
                "stdout_sha256": "0" * 64,
                "stderr_sha256": "1" * 64,
            }
            with mock.patch.object(MODULE, "run_role", return_value=failed_attempt):
                self.assertEqual(MODULE.run_canary(options, inputs), 1)
            result = json.loads((output / "canary.json").read_text(encoding="utf-8"))
            self.assertEqual(result["result"], "failed")
            self.assertFalse(result["marker_observed"])
            self.assertEqual(result["failure"], "canary process failed with exit code 1")


if __name__ == "__main__":
    unittest.main()
