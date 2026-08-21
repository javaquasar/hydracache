#!/usr/bin/env python3

from __future__ import annotations

import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock


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
        self.assertEqual(counts["M6-connections"], 5)
        self.assertEqual(counts["M8-60m"], 4)
        self.assertEqual(plan["job_count"], 40)
        m5 = [job["dimensions"] for job in plan["jobs"] if job["case_id"] == "M5-tags"]
        self.assertIn(
            {"distribution": "one-hot", "tags_per_entry": 1, "tag_pool": 1},
            m5,
        )
        self.assertIn(
            {"distribution": "high-fanout", "tags_per_entry": 16, "tag_pool": 16},
            m5,
        )
        m6 = [job["dimensions"] for job in plan["jobs"] if job["case_id"] == "M6-connections"]
        self.assertTrue(all(cell["tls"] is True for cell in m6))

    def test_selected_rows_have_row_not_cell_caps(self) -> None:
        with mock.patch.object(
            campaign,
            "resolve_commit",
            side_effect=lambda _root, value, label: campaign.require_full_sha(value, label),
        ):
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

    def test_candidate_plan_pins_explicit_source_separately_from_workflow(self) -> None:
        workflow_sha = campaign.git(self.root, "rev-parse", "HEAD")
        source_sha = workflow_sha
        plan = campaign.build_plan(
            self.root,
            ["M0-cold"],
            ["B1-instrumented", "C-candidate"],
            1,
            True,
            workflow_sha,
            source_sha,
            "candidate",
        )
        self.assertEqual(plan["workflow_sha"], workflow_sha)
        self.assertEqual(plan["source_sha"], source_sha)
        self.assertEqual(plan["source_shas"]["C-candidate"], source_sha)
        self.assertEqual(plan["campaign_role"], "candidate")

    def test_baseline_rejects_non_b1_source(self) -> None:
        workflow_sha = campaign.git(self.root, "rev-parse", "HEAD")
        with self.assertRaises(campaign.CampaignError):
            campaign.build_plan(
                self.root,
                ["M0-cold"],
                ["B1-instrumented"],
                1,
                False,
                workflow_sha,
                workflow_sha,
                "baseline",
            )

    def test_evidence_roles_reject_wrong_cohort_sets(self) -> None:
        workflow_sha = campaign.git(self.root, "rev-parse", "HEAD")
        identities = campaign.tomllib.loads(
            (self.root / campaign.IDENTITIES_RELATIVE).read_text(encoding="utf-8")
        )
        b1_sha = str(identities["b1_instrumented"]["source_sha"])
        with mock.patch.object(
            campaign,
            "resolve_commit",
            side_effect=lambda _root, value, label: campaign.require_full_sha(value, label),
        ):
            with self.assertRaises(campaign.CampaignError):
                campaign.build_plan(
                    self.root,
                    ["M0-cold"],
                    ["B1-instrumented"],
                    1,
                    False,
                    workflow_sha,
                    b1_sha,
                    "candidate",
                )
            with self.assertRaises(campaign.CampaignError):
                campaign.build_plan(
                    self.root,
                    ["M0-cold"],
                    ["B1-instrumented", "C-candidate"],
                    1,
                    False,
                    workflow_sha,
                    b1_sha,
                    "baseline",
                )

    def test_source_sha_must_be_full_lowercase_commit(self) -> None:
        for value in ("main", "ABCDEF" * 6 + "ABCD", "0" * 39, "g" * 40):
            with self.subTest(value=value):
                with self.assertRaises(campaign.CampaignError):
                    campaign.require_full_sha(value, "source_sha")

    def test_protected_workflow_keeps_trusted_harness_source_boundary(self) -> None:
        workflow = (self.root / ".github/workflows/memory-reference-071.yml").read_text(
            encoding="utf-8"
        )
        for marker in (
            "source_sha:",
            "campaign_role:",
            'working-directory: trusted-harness',
            "path: trusted-harness",
            "path: candidate-source",
            "persist-credentials: false",
            'if [[ "$GITHUB_REF" != "refs/heads/main" ]]',
            "verify-identity",
            '--workflow-sha "$HYDRACACHE_MEMORY_WORKFLOW_SHA"',
            '--source-sha "$HYDRACACHE_MEMORY_SOURCE_SHA"',
        ):
            with self.subTest(marker=marker):
                self.assertIn(marker, workflow)

    def test_campaign_identity_rejects_moved_source_harness_role_and_case(self) -> None:
        workflow_sha = campaign.git(self.root, "rev-parse", "HEAD")
        source_sha = workflow_sha
        plan = campaign.build_plan(
            self.root,
            ["M0-cold"],
            ["B1-instrumented", "C-candidate"],
            1,
            True,
            workflow_sha,
            source_sha,
            "candidate",
        )
        plan["campaign_id"] = "candidate-1"
        with tempfile.TemporaryDirectory() as directory:
            campaign_dir = Path(directory)
            identity_path = campaign_dir / "campaign-identity.json"
            campaign.atomic_json(identity_path, campaign.campaign_identity(plan), create=True)
            plan["identity"] = {
                "path": identity_path.name,
                "sha256": campaign.sha256(identity_path),
            }
            campaign.atomic_json(campaign_dir / "state.json", plan, create=True)
            accepted = campaign.verify_campaign_identity(
                campaign_dir, workflow_sha, source_sha, "candidate", "M0-cold"
            )
            self.assertEqual(accepted["source_sha"], source_sha)
            tampered = dict(plan)
            tampered["controller_sha"] = "0" * 40
            with self.assertRaises(campaign.CampaignError):
                campaign.retained_campaign_identity(campaign_dir, tampered)
            for requested_workflow, requested_source, role, case_id in (
                ("0" * 40, source_sha, "candidate", "M0-cold"),
                (workflow_sha, "0" * 40, "candidate", "M0-cold"),
                (workflow_sha, source_sha, "baseline", "M0-cold"),
                (workflow_sha, source_sha, "candidate", "M1-shape"),
            ):
                with self.assertRaises(campaign.CampaignError):
                    campaign.verify_campaign_identity(
                        campaign_dir,
                        requested_workflow,
                        requested_source,
                        role,
                        case_id,
                    )

    def test_campaign_identity_receipt_drift_fails_closed(self) -> None:
        workflow_sha = campaign.git(self.root, "rev-parse", "HEAD")
        plan = campaign.build_plan(
            self.root,
            ["M0-cold"],
            ["B1-instrumented", "C-candidate"],
            1,
            True,
            workflow_sha,
            workflow_sha,
            "candidate",
        )
        plan["campaign_id"] = "candidate-2"
        with tempfile.TemporaryDirectory() as directory:
            campaign_dir = Path(directory)
            identity_path = campaign_dir / "campaign-identity.json"
            campaign.atomic_json(identity_path, campaign.campaign_identity(plan), create=True)
            plan["identity"] = {
                "path": identity_path.name,
                "sha256": campaign.sha256(identity_path),
            }
            campaign.atomic_json(campaign_dir / "state.json", plan, create=True)
            identity_path.write_text("{}\n", encoding="utf-8")
            with self.assertRaises(campaign.CampaignError):
                campaign.retained_campaign_identity(campaign_dir, plan)

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
        self.assertIsNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M6-connections", "dimensions": {"tls": True}}
            )
        )
        self.assertIsNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M7-persistence", "dimensions": {"persistence": "supported"}}
            )
        )
        self.assertIsNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M10-24h", "dimensions": {}}
            )
        )
        self.assertIsNone(
            campaign.unsupported_evidence_reason(
                {"case_id": "M9-6h", "dimensions": {}}
            )
        )
        self.assertIsNone(
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
                "workflow_sha": "1" * 40,
                "source_sha": "2" * 40,
                "campaign_role": "candidate",
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
