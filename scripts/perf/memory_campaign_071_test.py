#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("memory_campaign_071.py")
SPEC = importlib.util.spec_from_file_location("memory_campaign_071", SCRIPT)
assert SPEC and SPEC.loader
campaign = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = campaign
SPEC.loader.exec_module(campaign)


class MemoryCampaign071Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.root = campaign.repository_root()

    def test_matrix_expansion_is_finite_and_deliberate(self) -> None:
        plan = campaign.build_plan(
            self.root,
            [],
            ["B1-instrumented"],
            1,
            True,
        )
        counts: dict[str, int] = {}
        for job in plan["jobs"]:
            counts[job["case_id"]] = counts.get(job["case_id"], 0) + 1
        self.assertEqual(counts["M1-shape"], 16)
        self.assertEqual(counts["M5-tags"], 6)
        self.assertEqual(counts["M6-connections"], 9)
        self.assertEqual(counts["M8-60m"], 4)
        self.assertEqual(plan["job_count"], 44)
        m5 = [job["dimensions"] for job in plan["jobs"] if job["case_id"] == "M5-tags"]
        self.assertIn(
            {"distribution": "one-hot", "tags_per_entry": 1, "tag_pool": 1},
            m5,
        )
        self.assertIn(
            {"distribution": "high-fanout", "tags_per_entry": 16, "tag_pool": 16},
            m5,
        )

    def test_selected_rows_have_row_not_cell_caps(self) -> None:
        plan = campaign.build_plan(
            self.root,
            ["M0-cold"],
            ["B0-release", "B1-instrumented"],
            1,
            False,
        )
        self.assertEqual(plan["job_count"], 2)
        self.assertEqual(plan["admitted_host_cap_seconds"], 3_600)
        self.assertEqual(
            plan["source_shas"]["B1-instrumented"],
            "795f9493bcbb7a56aa229c59e4a717f60c654cdb",
        )

    def test_b0_cannot_be_pooled_into_instrumented_rows(self) -> None:
        with self.assertRaises(campaign.CampaignError):
            campaign.build_plan(
                self.root,
                ["M1-shape"],
                ["B0-release"],
                1,
                False,
            )

    def test_unknown_row_is_rejected(self) -> None:
        with self.assertRaises(campaign.CampaignError):
            campaign.build_plan(self.root, ["M99"], ["B1-instrumented"], 1, True)

    def test_atomic_create_refuses_overwrite(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "state.json"
            campaign.atomic_json(path, {"first": True}, create=True)
            with self.assertRaises(campaign.CampaignError):
                campaign.atomic_json(path, {"second": True}, create=True)
            self.assertEqual(campaign.json.loads(path.read_text()), {"first": True})

    def test_lock_rejects_second_owner(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / ".lock"
            with campaign.CampaignLock(path):
                with self.assertRaises(campaign.CampaignError):
                    with campaign.CampaignLock(path):
                        pass
            self.assertFalse(path.exists())

    def test_evidence_build_root_must_be_external(self) -> None:
        with self.assertRaises(campaign.CampaignError):
            campaign.ensure_external_build_root(self.root, self.root / "target" / "builds")

    def test_unsupported_evidence_cells_fail_closed(self) -> None:
        self.assertIsNotNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M6-connections", "dimensions": {"tls": False}}
            )
        )
        self.assertIsNotNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M10-24h", "dimensions": {}}
            )
        )
        self.assertIsNotNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M8-60m", "dimensions": {"sequence": "reset"}}
            )
        )
        self.assertIsNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M3-ttl", "dimensions": {"cycles": 60}}
            )
        )
        self.assertIsNone(
            campaign.unsupported_evidence_reason(
                {
                    "case_id": "M5-tags",
                    "dimensions": {"distribution": "high-fanout"},
                }
            )
        )

    def test_admission_rejects_cross_host_overhead(self) -> None:
        state = {
            "source_shas": {
                "B1-instrumented": "795f9493bcbb7a56aa229c59e4a717f60c654cdb"
            }
        }
        receipts = {
            "host-preflight": {
                "schema_version": 1,
                "release": "0.71",
                "profile_id": "memory-reference-071-v1",
                "protected_environment": "memory-reference-071",
                "result": "success",
                "ship_evidence_eligible": True,
                "host_fingerprint": "host-a",
            },
            "reference-activation": {
                "schema_version": 1,
                "release": "0.67.1",
                "profile": "reference-v1",
                "passed": True,
                "ship_evidence_eligible": True,
            },
            "historical-input-receipt": {
                "schema_version": 1,
                "release": "0.71",
                "commit": "dbc2f82f7f303528b3cca7842818730c82232b9c",
                "checkout_clean": True,
                "files": [{"path": "raw", "bytes": 1}],
                "mirror": {"manifest_sha256": "same", "restored_manifest_sha256": "same"},
            },
            "instrumentation-overhead": {
                "schema_version": 1,
                "release": "0.71",
                "source_sha": state["source_shas"]["B1-instrumented"],
                "host_fingerprint": "host-b",
                "passed": True,
                "ship_evidence_eligible": True,
            },
        }
        with self.assertRaises(campaign.CampaignError):
            campaign.validate_admission_receipts(receipts, state)

    def test_completed_job_is_published_once_and_drift_fails(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            campaign_dir = root / "campaign"
            job_dir = campaign_dir / "jobs" / "job-1"
            job_dir.mkdir(parents=True)
            (job_dir / "report.json").write_text("{}", encoding="utf-8")
            state = {
                "mode": "evidence",
                "campaign_id": "campaign-1",
                "mirror_root": str(root / "mirror"),
            }
            job = {"job_id": "job-1"}
            campaign.publish_job(campaign_dir, state, job)
            campaign.publish_job(campaign_dir, state, job)
            archive = Path(state["published_jobs"]["job-1"]["archive"])
            archive.write_bytes(b"drift")
            with self.assertRaises(campaign.CampaignError):
                campaign.publish_job(campaign_dir, state, job)

    def test_live_host_drift_blocks_resume(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            campaign_dir = Path(directory)
            (campaign_dir / "admission").mkdir()
            campaign.atomic_json(campaign_dir / "state.json", {"admission": {"manifest": "x"}})
            campaign.atomic_json(
                campaign_dir / "admission" / "host-preflight.json",
                {"host_fingerprint": "host-a", "profile_id": "memory-reference-071-v1"},
            )
            observed = campaign_dir / "observed.json"
            campaign.atomic_json(
                observed,
                {
                    "result": "success",
                    "ship_evidence_eligible": True,
                    "host_fingerprint": "host-b",
                    "profile_id": "memory-reference-071-v1",
                },
            )
            with self.assertRaises(campaign.CampaignError):
                campaign.verify_live_host(campaign_dir, observed)


if __name__ == "__main__":
    unittest.main()
