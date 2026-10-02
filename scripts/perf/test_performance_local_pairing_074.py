import importlib.util
import pathlib
import unittest


SCRIPT = pathlib.Path(__file__).with_name("performance_local_pairing_074.py")
SPEC = importlib.util.spec_from_file_location("performance_local_pairing_074", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
PAIRING = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PAIRING)


class LocalPairing074Tests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = {
            "pairs": 5,
            "maximum_background_cpu_percent": 20.0,
            "maximum_failed_attempts": 0,
            "noise": {
                "percentile": 0.95,
                "minimum_goodput_effect": 0.02,
                "minimum_cpu_per_operation_effect": 0.03,
                "minimum_p99_effect": 0.03,
                "noise_multiplier": 2.0,
                "classification": "inconclusive-below-aa-derived-mde",
            },
            "receipt": {
                "required_identity_fields": [
                    "release",
                    "surface",
                    "operation",
                    "operations",
                    "warmup_operations",
                    "concurrency",
                    "payload_bytes",
                    "key_space",
                    "seed",
                    "workload_sha256",
                ],
                "optional_identity_fields": ["pipeline", "batch_size", "transport"],
                "required_metrics": [
                    "goodput_operations_per_second",
                    "cpu_nanoseconds_per_operation",
                    "latency.p99_us",
                ],
            },
        }

    def receipt(self, goodput: float, cpu: float, p99: float) -> dict:
        return {
            "release": "0.74",
            "promotable": False,
            "surface": "client-surface",
            "operation": "get",
            "operations": 100_000,
            "warmup_operations": 10_000,
            "concurrency": 8,
            "payload_bytes": 256,
            "key_space": 4096,
            "seed": 740074,
            "workload_sha256": "sha256:workload",
            "goodput_operations_per_second": goodput,
            "cpu_nanoseconds_per_operation": cpu,
            "latency": {"p99_us": p99},
        }

    def attempts(self) -> list[dict]:
        attempts = []
        for pair_index, order in enumerate(PAIRING.abba_order(5, 740074)):
            for role in order:
                direction = 1 if role == "candidate" else 0
                attempts.append(
                    {
                        "role": role,
                        "binary_sha256": "same",
                        "background_cpu_percent": 3.0,
                        "affinity_applied": True,
                        "priority_applied": True,
                        "result": "success",
                        "receipt": self.receipt(
                            1000.0 + direction * (pair_index + 1),
                            100.0 + direction * 0.2,
                            50.0 + direction * 0.1,
                        ),
                    }
                )
        return attempts

    def test_abba_order_is_counterbalanced_and_seeded(self) -> None:
        self.assertEqual(
            PAIRING.abba_order(5, 2),
            [
                ("baseline", "candidate"),
                ("candidate", "baseline"),
                ("baseline", "candidate"),
                ("candidate", "baseline"),
                ("baseline", "candidate"),
            ],
        )
        self.assertEqual(PAIRING.abba_order(2, 3)[0], ("candidate", "baseline"))

    def test_stable_same_binary_series_derives_frozen_floor_mde(self) -> None:
        summary = PAIRING.analyse_attempts(self.attempts(), self.contract, "aa", 740074)
        self.assertFalse(summary["promotable"])
        self.assertGreaterEqual(
            summary["minimum_detectable_effect"]["goodput_operations_per_second"], 0.02
        )
        self.assertGreaterEqual(
            summary["minimum_detectable_effect"]["latency.p99_us"], 0.03
        )

    def test_workload_drift_is_rejected(self) -> None:
        attempts = self.attempts()
        attempts[-1]["receipt"]["seed"] = 9
        with self.assertRaisesRegex(ValueError, "identity drifted"):
            PAIRING.analyse_attempts(attempts, self.contract, "aa", 740074)

    def test_missing_placement_or_busy_host_is_rejected(self) -> None:
        attempts = self.attempts()
        attempts[0]["affinity_applied"] = False
        with self.assertRaisesRegex(ValueError, "CPU affinity"):
            PAIRING.analyse_attempts(attempts, self.contract, "aa", 740074)
        attempts = self.attempts()
        attempts[0]["background_cpu_percent"] = 21.0
        with self.assertRaisesRegex(ValueError, "background CPU"):
            PAIRING.analyse_attempts(attempts, self.contract, "aa", 740074)


if __name__ == "__main__":
    unittest.main()
