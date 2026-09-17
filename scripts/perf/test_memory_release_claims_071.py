#!/usr/bin/env python3
"""Regression tests for evidence-derived, no-win 0.71 claims."""

from __future__ import annotations

import copy
import importlib.util
import json
from pathlib import Path
import sys
import tomllib
import unittest


ROOT = Path(__file__).resolve().parents[2]
MODULE_PATH = Path(__file__).with_name("memory_release_claims_071.py")
SPEC = importlib.util.spec_from_file_location("memory_release_claims_071", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
CLAIMS = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = CLAIMS
SPEC.loader.exec_module(CLAIMS)


class ClaimGenerationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.policy = tomllib.loads((ROOT / "docs/testing/memory/0.71/release-policy.toml").read_text(encoding="utf-8"))
        cls.d3 = json.loads((ROOT / "docs/testing/memory/0.71/d3-implementation-acceptance.json").read_text(encoding="utf-8"))
        cls.manifest = {
            "release": "0.71",
            "source_sha": "a" * 40,
            "workflow_sha": "a" * 40,
            "campaigns": [
                {"id": f"sample-{stage}", "stage": stage, "github_run_id": i,
                 "github_artifact_id": i + 10, "archive": f"sample-{stage}.tar.gz"}
                for i, stage in enumerate(CLAIMS.EXPECTED_ROWS, start=1)
            ],
        }
        cls.acceptance = {
            "release": "0.71",
            "measured_source_sha": "a" * 40,
            "workflow_sha": "a" * 40,
            "evidence_branch": "evidence/0.71/ax42/d4",
            "evidence_commit": "b" * 40,
            "campaigns": [
                {key: item[key] for key in ("stage", "id", "github_run_id", "github_artifact_id")}
                for item in cls.manifest["campaigns"]
            ],
        }
        cls.receipts = {
            item["id"]: {
                "campaign_id": item["id"],
                "case_ids": [CLAIMS.EXPECTED_ROWS[item["stage"]][0]],
                "source_sha": "a" * 40,
                "workflow_sha": "a" * 40,
                "result": "success",
                "ship_evidence_eligible": True,
                "job_count": CLAIMS.EXPECTED_ROWS[item["stage"]][1],
                "completed_jobs": CLAIMS.EXPECTED_ROWS[item["stage"]][1],
            }
            for item in cls.manifest["campaigns"]
        }

    def test_complete_no_win_chain_emits_no_numeric_claim(self) -> None:
        claims = CLAIMS.derive_claims(self.policy, self.d3, self.manifest, self.receipts, self.acceptance)
        self.assertEqual(claims["numeric_memory_improvement_claims"], [])
        self.assertEqual(len(claims["campaign_ids"]), 4)
        self.assertEqual(len(claims["optional_dispositions"]), 4)

    def test_missing_or_failed_long_cell_is_rejected(self) -> None:
        receipts = copy.deepcopy(self.receipts)
        receipts["sample-M10"]["completed_jobs"] = 1
        with self.assertRaisesRegex(ValueError, "unaccepted campaign"):
            CLAIMS.derive_claims(self.policy, self.d3, self.manifest, receipts, self.acceptance)

    def test_unreviewed_numerical_proposal_is_rejected(self) -> None:
        d3 = copy.deepcopy(self.d3)
        d3["proposals"][0]["numerical_claim_authorized"] = True
        with self.assertRaisesRegex(ValueError, "numerical proposal"):
            CLAIMS.derive_claims(self.policy, d3, self.manifest, self.receipts, self.acceptance)

    def test_pending_optional_disposition_is_rejected(self) -> None:
        policy = copy.deepcopy(self.policy)
        policy["optional_work"][0]["disposition"] = "pending-evidence"
        with self.assertRaisesRegex(ValueError, "unreviewed disposition"):
            CLAIMS.derive_claims(policy, self.d3, self.manifest, self.receipts, self.acceptance)

    def test_acceptance_manifest_mismatch_is_rejected(self) -> None:
        acceptance = copy.deepcopy(self.acceptance)
        acceptance["campaigns"][0]["github_artifact_id"] += 1
        with self.assertRaisesRegex(ValueError, "D4 acceptance"):
            CLAIMS.derive_claims(self.policy, self.d3, self.manifest, self.receipts, acceptance)


if __name__ == "__main__":
    unittest.main()
