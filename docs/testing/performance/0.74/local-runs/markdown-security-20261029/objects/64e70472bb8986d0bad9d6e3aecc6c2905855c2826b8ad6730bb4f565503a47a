from pathlib import Path
import tomllib
import unittest

import launch_preflight
from observation_coordinator import Coordinator


class ProtocolTests(unittest.TestCase):
    def protocol(self):
        return tomllib.loads((Path(__file__).resolve().parents[2] /
            "docs/testing/performance/0.74/contract-observation-protocol-20261025.toml").read_text())

    def test_original_command_and_windows_not_renewed(self):
        p = self.protocol()
        self.assertEqual(p["command"], launch_preflight.ARGV)
        self.assertEqual(p["launch_receipt_sha256"], launch_preflight.RECEIPT_SHA256)
        self.assertEqual((p["case_count"], p["case_threads"], p["wrapper_seconds"]), (321, 4, 780))
        self.assertEqual(p["original_live_windows_seconds"], [300, 60, 10])
        self.assertEqual(p["discovery_seconds"], Coordinator.DISCOVERY_SECONDS)
        self.assertEqual(p["discovery_snapshots"], Coordinator.DISCOVERY_SNAPSHOTS)
        self.assertEqual(p["outer_observer_seconds"], Coordinator.OUTER_SECONDS)

    def test_no_implicit_dispatch_or_claim_authority(self):
        p = self.protocol()
        self.assertEqual(p["status"], "prepared-not-authorized")
        for field in ("dispatch_enabled", "retry_allowed", "allow_result_splicing", "allow_full_package_claim",
                      "allow_product_speedup_claim", "kill_owner_on_observer_failure", "durable_custody_proven"):
            self.assertIs(p[field], False)
        self.assertEqual(p["maximum_dispatches"], 1)
        self.assertTrue(p["missing_requirements"])


if __name__ == "__main__":
    unittest.main()
