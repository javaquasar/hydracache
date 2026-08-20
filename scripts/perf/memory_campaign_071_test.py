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

    def test_selected_rows_have_row_not_cell_caps(self) -> None:
        plan = campaign.build_plan(
            self.root,
            ["M0-cold", "M1-shape"],
            ["B0-release", "B1-instrumented"],
            1,
            False,
        )
        self.assertEqual(plan["job_count"], 34)
        self.assertEqual(plan["admitted_host_cap_seconds"], 3_600 + 28_800)

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


if __name__ == "__main__":
    unittest.main()
