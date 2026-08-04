#!/usr/bin/env python3

import json
from pathlib import Path
import tempfile
import unittest
import zipfile

import reference_campaign as campaign


SHA = "2" * 40
FINGERPRINT = "5" * 64


def base_state() -> dict:
    return {
        "campaign_id": "hc0671-rental-a1b2c3",
        "repository": campaign.EXPECTED_REPOSITORY,
        "branch": campaign.EXPECTED_BRANCH,
        "workflow": campaign.WORKFLOW,
        "expected_sha": SHA,
        "stages": {},
    }


class ReferenceCampaignTests(unittest.TestCase):
    def test_campaign_and_artifact_names_are_strict(self) -> None:
        self.assertTrue(campaign.CAMPAIGN_RE.fullmatch("hc0671-rental-a1b2c3"))
        self.assertFalse(campaign.CAMPAIGN_RE.fullmatch("HC0671 bad; rm"))
        self.assertEqual(
            campaign.safe_artifact_filename(123, "performance-0671/bootstrap receipt"),
            "123-performance-0671-bootstrap-receipt.zip",
        )
        self.assertEqual(campaign.expand_cpu_list("0,5-7"), {0, 5, 6, 7})
        with self.assertRaises(campaign.CampaignError):
            campaign.expand_cpu_list("7-5")

    def test_stage_plan_never_skips_an_unaccepted_predecessor(self) -> None:
        state = base_state()
        self.assertEqual([item["name"] for item in campaign.stage_specs(state)], ["qualification"])
        state["stages"]["qualification"] = {"status": "completed", "run_id": 101}
        self.assertEqual(
            [item["name"] for item in campaign.stage_specs(state)],
            ["qualification", "full-dress-1"],
        )
        state["stages"]["full-dress-1"] = {"status": "completed", "run_id": 102}
        self.assertEqual(campaign.stage_specs(state)[-1]["name"], "full-dress-2")
        state["stages"]["full-dress-2"] = {"status": "completed", "run_id": 103}
        first = campaign.stage_specs(state)[-1]
        self.assertEqual(first["name"], "bootstrap-1")
        self.assertEqual(first["admission_run"], "103")
        self.assertEqual(first["bootstrap_predecessor"], "")
        state["stages"]["bootstrap-1"] = {"status": "completed", "run_id": 104}
        second = campaign.stage_specs(state)[-1]
        self.assertEqual(second["name"], "bootstrap-2")
        self.assertEqual(second["bootstrap_predecessor"], "104")

    def test_dispatch_fields_bind_unique_campaign_step_and_chain(self) -> None:
        state = base_state()
        spec = {
            "name": "bootstrap-3",
            "mode": "bootstrap",
            "sample_index": "3",
            "admission_run": "200",
            "bootstrap_predecessor": "202",
        }
        fields = campaign.dispatch_fields(state, spec)
        joined = "\n".join(fields)
        self.assertIn("performance_0671_campaign=hc0671-rental-a1b2c3:bootstrap-3", joined)
        self.assertIn("full_dress_admission_run_id=200", joined)
        self.assertIn("bootstrap_predecessor_run_id=202", joined)

    def test_receipt_retention_is_immutable_and_sample_directory_is_exact(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            data = b'{"passed":true}\n'
            path = campaign.retain_receipt(root, "bootstrap-samples/sample-1.json", data)
            self.assertEqual(path.read_bytes(), data)
            self.assertEqual(campaign.retain_receipt(root, "bootstrap-samples/sample-1.json", data), path)
            with self.assertRaises(campaign.CampaignError):
                campaign.retain_receipt(root, "bootstrap-samples/sample-1.json", b"changed")
            with self.assertRaises(campaign.CampaignError):
                campaign.retain_receipt(root, "../escape.json", data)

    def test_zip_receipt_selection_rejects_duplicates(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "artifact.zip"
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr("a/bootstrap-sample.json", "{}")
                output.writestr("b/bootstrap-sample.json", "{}")
            with self.assertRaises(campaign.CampaignError):
                campaign.read_unique_member(archive, "bootstrap-sample.json")

    def test_common_receipt_rejects_fingerprint_drift(self) -> None:
        state = base_state()
        state["stages"]["runner_fingerprint"] = FINGERPRINT
        receipt = {
            "source_commit": SHA,
            "github_run_id": "101",
            "passed": True,
            "ship_evidence_eligible": False,
            "runner_fingerprint": "6" * 64,
        }
        with self.assertRaises(campaign.CampaignError):
            campaign.expect_common_receipt(receipt, state, 101)

    def test_burn_receipt_is_non_evidence_and_minimum_duration_is_enforced(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "burn.json"
            state = base_state()
            state["profile_sha256"] = "3" * 64
            receipt = {
                "schema_version": 1,
                "stage": "reference-host-irq-burn-in",
                "source_commit": SHA,
                "profile_sha256": "3" * 64,
                "measurement_cpus": "1-4",
                "duration_seconds": 900,
                "passed": True,
                "failure_step": None,
                "qualification_evidence": False,
                "bootstrap_evidence": False,
                "ship_evidence_eligible": False,
                "irq_baseline_sha256": "4" * 64,
                "interrupts_before_sha256": "5" * 64,
                "interrupts_after_sha256": "6" * 64,
            }
            path.write_text(json.dumps(receipt), encoding="utf-8")
            campaign.validate_burn_receipt(path, state)
            receipt["duration_seconds"] = 599
            path.write_text(json.dumps(receipt), encoding="utf-8")
            with self.assertRaises(campaign.CampaignError):
                campaign.validate_burn_receipt(path, state)


if __name__ == "__main__":
    unittest.main()
