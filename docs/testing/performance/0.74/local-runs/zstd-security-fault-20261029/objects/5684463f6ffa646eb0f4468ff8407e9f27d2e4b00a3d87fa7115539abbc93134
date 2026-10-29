import copy
import importlib.util
import pathlib
import unittest


SCRIPT = pathlib.Path(__file__).with_name("monitor-long-run-campaign-074.py")
SPEC = importlib.util.spec_from_file_location("monitor_long_run_campaign_074", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MONITOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MONITOR)


def process(pid: int) -> dict:
    return {
        "boot_id": "boot-a",
        "pid": pid,
        "start_ticks": pid * 100,
        "process_group": 10,
        "cgroup_path": "/hc/a",
        "cgroup_inode": 50,
        "unit_name": "hc-a.service",
    }


def state() -> dict:
    return {
        "revision": 7,
        "campaign_state": "I74_RUNNING",
        "identity": {
            "campaign_id": "a" * 64,
            "manifest_sha256": "b" * 64,
            "contract_sha256": "c" * 64,
            "scenario_sha256": "d" * 64,
            "tooling_sha256": "e" * 64,
            "source_bundle_sha256": "f" * 64,
            "binary_bundle_sha256": "1" * 64,
            "workload_bundle_sha256": "2" * 64,
            "machine_id": "machine-a",
            "boot_id": "boot-a",
            "host_receipt_sha256": "3" * 64,
            "mount_identity": "mount-a",
            "isolated_cpuset": "2-5",
            "housekeeping_cpuset": "0-1",
            "command_environment_sha256": "4" * 64,
            "lease_id": "00000000-0000-4000-8000-000000000074",
            "lease_deadline_unix_seconds": 2_000,
        },
        "harness": process(100),
        "daemon": process(101),
        "checkpoint": {
            "sequence": 8,
            "record_sha256": "c" * 64,
            "useful_progress_unix_seconds": 1_000,
        },
        "controller_lease": None,
        "recorded_failure": False,
        "duplicate_executor": False,
        "durable_history_corrupt": False,
    }


def inspect(value: dict, now: int = 1_010) -> dict:
    return MONITOR.inspect_state(
        value,
        expected_campaign_id="a" * 64,
        expected_manifest_sha256="b" * 64,
        now_unix_seconds=now,
        progress_rejection_gap_seconds=180,
    )


class MonitorLongRunCampaign074Tests(unittest.TestCase):
    def test_live_process_without_controller_is_controller_loss_not_measurement_loss(self) -> None:
        receipt = inspect(state())
        self.assertEqual(receipt["classification"], "controller-loss")
        self.assertEqual(receipt["mutations"], [])
        self.assertEqual(receipt["harness_start_ticks"], 10_000)

    def test_stale_progress_is_progress_loss_even_when_pids_exist(self) -> None:
        receipt = inspect(state(), now=1_181)
        self.assertEqual(receipt["classification"], "progress-loss")
        self.assertEqual(receipt["useful_progress_age_seconds"], 181)

    def test_corruption_failure_and_duplicate_executor_fail_closed(self) -> None:
        for field, expected in [
            ("durable_history_corrupt", "evidence-corruption"),
            ("recorded_failure", "measurement-loss"),
            ("duplicate_executor", "measurement-loss"),
        ]:
            changed = state()
            changed[field] = True
            self.assertEqual(inspect(changed)["classification"], expected)

    def test_identity_or_schema_drift_is_invalid_state(self) -> None:
        changed = copy.deepcopy(state())
        changed["identity"]["campaign_id"] = "d" * 64
        changed["unexpected"] = True
        receipt = inspect(changed)
        self.assertEqual(receipt["classification"], "invalid-state")
        self.assertTrue(receipt["problems"])

    def test_nested_schema_drift_and_future_progress_are_invalid(self) -> None:
        changed = copy.deepcopy(state())
        changed["harness"]["unexpected"] = True
        changed["checkpoint"]["useful_progress_unix_seconds"] = 1_011
        receipt = inspect(changed)
        self.assertEqual(receipt["classification"], "invalid-state")
        self.assertIn("harness fields differ from the frozen schema", receipt["problems"])
        self.assertIn("useful progress time is in the future", receipt["problems"])

    def test_live_controller_lease_is_healthy_and_terminal_is_terminal(self) -> None:
        live = state()
        live["controller_lease"] = {
            "holder_request_id": "00000000-0000-4000-8000-000000000075",
            "authorization_sha256": "5" * 64,
            "expires_unix_seconds": 1_100,
        }
        self.assertEqual(inspect(live)["classification"], "healthy")
        terminal = state()
        terminal["campaign_state"] = "I74_SEALED"
        self.assertEqual(inspect(terminal)["classification"], "terminal")

    def test_completed_measurement_loss_accepts_cleared_execution_identity(self) -> None:
        failed = state()
        failed["campaign_state"] = "FAILED_INCOMPLETE"
        failed["recorded_failure"] = True
        failed["harness"] = None
        failed["daemon"] = None
        failed["checkpoint"] = None
        receipt = inspect(failed)
        self.assertEqual(receipt["classification"], "measurement-loss")
        self.assertEqual(receipt["problems"], [])
        self.assertIsNone(receipt["useful_progress_age_seconds"])
        self.assertIsNone(receipt["harness_pid"])
        self.assertIsNone(receipt["daemon_pid"])


if __name__ == "__main__":
    unittest.main()
