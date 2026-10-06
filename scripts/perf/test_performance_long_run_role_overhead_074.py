import importlib.util
from pathlib import Path
import sys
import unittest


MODULE_PATH = Path(__file__).with_name("performance_long_run_role_overhead_074.py")
SPEC = importlib.util.spec_from_file_location(
    "performance_long_run_role_overhead_074", MODULE_PATH
)
assert SPEC and SPEC.loader
overhead = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = overhead
SPEC.loader.exec_module(overhead)


I74_SOURCE = "1" * 40
C74_SOURCE = "2" * 40
HOST = "3" * 64


def budgets() -> dict[str, object]:
    return {
        "pairs": 5,
        "maximum_supervisor_cpu_percent": 0.5,
        "maximum_supervisor_rss_bytes": 64 * 1024 * 1024,
        "maximum_checkpoint_io_bytes_per_second": 1024 * 1024,
        "asymmetry_percent": 1.0,
    }


def identity(role: str) -> dict[str, object]:
    return {
        "source_commit": I74_SOURCE if role == "i74" else C74_SOURCE,
        "binary_sha256": ("4" if role == "i74" else "5") * 64,
        "workload_sha256": "6" * 64,
        "payload_sha256": "7" * 64,
        "host_receipt_sha256": HOST,
        "seed": 740074,
        "operations": 1_000_000,
        "warmup_operations": 10_000,
        "cpuset": "1-4",
    }


def attempt(role: str, variant: str, pair_index: int, position: int) -> dict[str, object]:
    elapsed_overhead = 0.005 if role == "i74" else 0.007
    cpu_overhead = 0.004 if role == "i74" else 0.006
    instrumented = variant == "instrumented"
    return {
        "schema_version": 1,
        "role": role,
        "variant": variant,
        "pair_index": pair_index,
        "position": position,
        "identity": identity(role),
        "metrics": {
            "elapsed_ns": int(1_000_000_000 * (1 + elapsed_overhead))
            if instrumented
            else 1_000_000_000,
            "role_cpu_ns": int(500_000_000 * (1 + cpu_overhead))
            if instrumented
            else 500_000_000,
            "role_rss_peak_bytes": 8 * 1024 * 1024,
            "role_io_bytes": 4_096 if instrumented else 0,
            "supervisor_cpu_ns": 1_000_000 if instrumented else 100_000,
            "supervisor_rss_peak_bytes": 4 * 1024 * 1024,
            "supervisor_io_bytes": 2_048 if instrumented else 0,
            "checkpoint_write_bytes": 4_096 if instrumented else 0,
            "completed_operations": 1_000_000,
        },
        "guards": {
            "affinity_applied": True,
            "priority_applied": True,
            "host_identity_stable": True,
            "role_identity_stable": True,
            "supervisor_identity_stable": True,
            "campaign_claim_valid": True,
            "unexpected_errors_absent": True,
        },
    }


def valid_attempt_set() -> dict[str, object]:
    attempts = []
    for role in overhead.ROLES:
        for pair_index in range(1, 6):
            for position, variant in enumerate(
                overhead._variant_order(pair_index, 740074), 1
            ):
                attempts.append(attempt(role, variant, pair_index, position))
    return {
        "schema_version": 1,
        "release": "0.74",
        "evidence_class": overhead.EVIDENCE_CLASS,
        "promotable": False,
        "seed": 740074,
        "order": overhead.ORDER,
        "attempts": attempts,
    }


def analyse(value: dict[str, object]) -> dict[str, object]:
    return overhead.analyse(
        value,
        "8" * 64,
        budgets(),
        {"i74": I74_SOURCE, "c74": C74_SOURCE},
        HOST,
    )


class RoleOverheadTests(unittest.TestCase):
    def test_checked_in_contracts_freeze_five_pairs_and_existing_budgets(self) -> None:
        repository = MODULE_PATH.parents[2]
        result = overhead._load_budgets(
            repository
            / "docs/testing/performance/0.74/long-run-controller-resilience-contract.toml",
            repository / "docs/testing/performance/0.74/statistics.toml",
        )
        self.assertEqual(result, budgets())

    def test_complete_counterbalanced_attempt_set_passes_rehearsal_only(self) -> None:
        result = analyse(valid_attempt_set())
        self.assertEqual(result["pairs_per_role"], 5)
        self.assertTrue(result["decision"]["resource_budgets_passed"])
        self.assertTrue(result["decision"]["non_product_rehearsal_passed"])
        self.assertFalse(result["decision"]["role_overhead_qualification_complete"])
        self.assertFalse(result["decision"]["release_admission_allowed"])
        self.assertAlmostEqual(
            result["asymmetry"]["elapsed_overhead_percentage_points"], 0.2
        )
        self.assertAlmostEqual(
            result["asymmetry"]["role_cpu_overhead_percentage_points"], 0.2
        )
        self.assertEqual(
            result["roles"]["i74"]["pairs"][0]["order"],
            ["control", "instrumented"],
        )
        self.assertEqual(
            result["roles"]["i74"]["pairs"][1]["order"],
            ["instrumented", "control"],
        )

    def test_order_identity_and_guard_drift_fail_closed(self) -> None:
        wrong_order = valid_attempt_set()
        wrong_order["attempts"][0], wrong_order["attempts"][1] = (
            wrong_order["attempts"][1],
            wrong_order["attempts"][0],
        )
        with self.assertRaisesRegex(ValueError, "order or identity"):
            analyse(wrong_order)

        identity_drift = valid_attempt_set()
        identity_drift["attempts"][3]["identity"]["payload_sha256"] = "9" * 64
        with self.assertRaisesRegex(ValueError, "workload identity drifted"):
            analyse(identity_drift)

        guard_drift = valid_attempt_set()
        guard_drift["attempts"][4]["guards"]["affinity_applied"] = False
        with self.assertRaisesRegex(ValueError, "every frozen guard"):
            analyse(guard_drift)

    def test_resource_budget_failure_is_retained_as_a_failed_decision(self) -> None:
        value = valid_attempt_set()
        value["attempts"][1]["metrics"]["supervisor_cpu_ns"] = 50_000_000
        result = analyse(value)
        self.assertFalse(result["roles"]["i74"]["pairs"][0]["resource_budgets_passed"])
        self.assertFalse(result["decision"]["resource_budgets_passed"])
        self.assertFalse(result["decision"]["non_product_rehearsal_passed"])

    def test_role_asymmetry_is_not_pooled_away(self) -> None:
        value = valid_attempt_set()
        for item in value["attempts"]:
            if item["role"] == "c74" and item["variant"] == "instrumented":
                item["metrics"]["elapsed_ns"] = 1_030_000_000
                item["metrics"]["role_cpu_ns"] = 520_000_000
        result = analyse(value)
        self.assertFalse(result["asymmetry"]["elapsed_passed"])
        self.assertFalse(result["asymmetry"]["role_cpu_passed"])
        self.assertFalse(result["decision"]["non_product_rehearsal_passed"])

    def test_control_checkpoint_writes_and_cross_role_workload_drift_are_rejected(self) -> None:
        control_write = valid_attempt_set()
        control_write["attempts"][0]["metrics"]["checkpoint_write_bytes"] = 1
        with self.assertRaisesRegex(ValueError, "control attempt wrote"):
            analyse(control_write)

        missing_instrumented_write = valid_attempt_set()
        missing_instrumented_write["attempts"][1]["metrics"][
            "checkpoint_write_bytes"
        ] = 0
        with self.assertRaisesRegex(ValueError, "did not write checkpoint"):
            analyse(missing_instrumented_write)

        cross_role = valid_attempt_set()
        for item in cross_role["attempts"]:
            if item["role"] == "c74":
                item["identity"]["operations"] = 2_000_000
                item["metrics"]["completed_operations"] = 2_000_000
        with self.assertRaisesRegex(ValueError, "not equivalent"):
            analyse(cross_role)

    def test_root_and_nested_shapes_are_strict(self) -> None:
        extra_root = valid_attempt_set()
        extra_root["claim"] = True
        with self.assertRaisesRegex(ValueError, "fields are not exact"):
            analyse(extra_root)

        extra_metric = valid_attempt_set()
        extra_metric["attempts"][0]["metrics"]["unknown"] = 1
        with self.assertRaisesRegex(ValueError, "fields are not exact"):
            analyse(extra_metric)


if __name__ == "__main__":
    unittest.main()
