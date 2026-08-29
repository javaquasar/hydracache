#!/usr/bin/env python3

import json
from pathlib import Path
import tempfile
import unittest
from unittest import mock
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
    def test_privileged_commands_are_non_interactive_after_sudo_lease(self) -> None:
        with mock.patch.object(campaign.os, "geteuid", return_value=1000, create=True):
            self.assertEqual(
                campaign.sudo_command("systemctl", "stop", "example.service"),
                ["sudo", "-n", "systemctl", "stop", "example.service"],
            )

    def test_sudo_lease_authenticates_before_starting_keeper(self) -> None:
        keeper = mock.Mock()
        with (
            mock.patch.object(campaign.os, "geteuid", return_value=1000, create=True),
            mock.patch.object(
                campaign.subprocess,
                "run",
                return_value=mock.Mock(returncode=0),
            ) as run,
            mock.patch.object(campaign.threading, "Thread", return_value=keeper),
        ):
            with campaign.sudo_lease():
                pass
        run.assert_called_once_with(["sudo", "-v"], check=True)
        keeper.start.assert_called_once_with()
        keeper.join.assert_called_once_with(timeout=2)

    def test_runner_provisioning_receipt_is_bound_to_frozen_source(self) -> None:
        state = base_state()
        receipt = {
            "schema_version": 4,
            "release": "0.67.1",
            "stage": "runner-provisioned",
            "source_commit": SHA,
            "runner_name": campaign.EXPECTED_RUNNER_NAME,
            "runner_online": False,
            "ship_evidence_eligible": False,
        }
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "runner-provisioned.json"
            path.write_text(json.dumps(receipt) + "\n", encoding="utf-8")
            campaign.validate_runner_provisioning_receipt(path, state)
            receipt["source_commit"] = "3" * 40
            path.write_text(json.dumps(receipt) + "\n", encoding="utf-8")
            with self.assertRaises(campaign.CampaignError):
                campaign.validate_runner_provisioning_receipt(path, state)

    def test_github_runner_contract_requires_exact_custom_label_and_idle_runner(self) -> None:
        state = base_state()
        listing = {
            "runners": [
                {
                    "name": campaign.EXPECTED_RUNNER_NAME,
                    "busy": False,
                    "labels": [
                        {"name": "self-hosted"},
                        {"name": campaign.EXPECTED_RUNNER_LABEL},
                    ],
                }
            ]
        }
        with mock.patch.object(campaign, "gh_json", return_value=listing):
            campaign.ensure_github_runner_contract(state)
        listing["runners"][0]["labels"] = [{"name": "hydracache-perf-quarantined"}]
        with (
            mock.patch.object(campaign, "gh_json", return_value=listing),
            self.assertRaises(campaign.CampaignError),
        ):
            campaign.ensure_github_runner_contract(state)

    def test_freeze_manifests_exclude_transient_systemd_units_symmetrically(self) -> None:
        root = Path(__file__).resolve().parents[2]
        expected_filter = "awk '$2 != \"transient\"'"
        for relative in (
            "scripts/perf/reference-host-tuning.sh",
            "scripts/perf/check-reference-host-freeze.sh",
        ):
            self.assertIn(expected_filter, (root / relative).read_text(encoding="utf-8"))

    def test_github_readiness_requires_cli_and_authenticated_session(self) -> None:
        with (
            mock.patch.object(campaign, "require_tools") as require_tools,
            mock.patch.object(campaign, "run_capture", return_value="") as run_capture,
            mock.patch.object(campaign, "repo_root", return_value=Path("/repo")),
        ):
            campaign.require_github_dispatch_readiness()
        require_tools.assert_called_once_with(["gh"])
        run_capture.assert_called_once_with(
            ["gh", "auth", "status"], cwd=Path("/repo")
        )

    def test_canonical_admission_owner_is_digest_and_campaign_bound(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            canonical = Path(temporary)
            receipt = {
                "campaign_id": "hc0671-rental-a1b2c3",
                "source_commit": SHA,
            }
            receipt_path = canonical / "reference-campaign-admission.json"
            bundle_path = canonical / "reference-campaign-host-admission.tar.gz"
            receipt_path.write_text(json.dumps(receipt) + "\n", encoding="utf-8")
            bundle_path.write_bytes(b"bundle")
            state = base_state()
            state["stages"]["host_admission"] = {
                "host_admission_receipt_sha256": campaign.sha256_file(receipt_path),
                "host_admission_bundle_sha256": campaign.sha256_file(bundle_path),
            }
            self.assertTrue(campaign.canonical_admission_matches(canonical, state))
            receipt["campaign_id"] = "hc0671-rental-other"
            receipt_path.write_text(json.dumps(receipt) + "\n", encoding="utf-8")
            self.assertFalse(campaign.canonical_admission_matches(canonical, state))

    def test_prepare_preflight_rejects_stale_canonical_admission(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            canonical = Path(temporary) / "reference-campaign-v1"
            campaign.require_canonical_host_admission_absent(canonical)
            canonical.mkdir()
            with self.assertRaisesRegex(
                campaign.CampaignError,
                "previous campaign to close and retire",
            ):
                campaign.require_canonical_host_admission_absent(canonical)

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
        frozen = "\n".join(
            campaign.dispatch_fields(
                state,
                {"name": "frozen-candidate", "mode": "frozen-candidate"},
            )
        )
        self.assertIn("performance_0671_mode=frozen-candidate", frozen)
        self.assertIn(
            "performance_0671_campaign=hc0671-rental-a1b2c3:frozen-candidate",
            frozen,
        )

    def test_performance_dispatch_skips_unrelated_ci_jobs(self) -> None:
        root = Path(__file__).resolve().parents[2]
        workflow = (root / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        guard = (
            "github.event_name != 'workflow_dispatch' "
            "|| inputs.performance_0671_mode == ''"
        )
        workflow_lines = workflow.splitlines()

        def job_block(job: str) -> str:
            start = workflow_lines.index(f"  {job}:")
            end = next(
                (
                    index
                    for index in range(start + 1, len(workflow_lines))
                    if workflow_lines[index].startswith("  ")
                    and not workflow_lines[index].startswith("    ")
                    and workflow_lines[index].endswith(":")
                ),
                len(workflow_lines),
            )
            return "\n".join(workflow_lines[start:end])

        for job in (
            "ci-topology",
            "memory-contracts-071",
            "memory-regression-fast",
            "docs",
            "rust",
            "migration-conformance-fast-evidence-069",
            "migration-conformance-069",
            "migration-conformance-postgres-069",
            "migration-conformance-admission-069",
            "hc2-linux-required",
            "hc2-docker-interop",
            "msrv",
        ):
            self.assertIn(guard, job_block(job), job)

        for job in (
            "release-0671-performance-qualification",
            "release-0671-performance-full-dress",
            "release-0671-performance-bootstrap",
            "release-0671-frozen-candidate",
        ):
            self.assertNotIn(guard, job_block(job), job)

    def test_runner_is_online_before_dispatch_discovery(self) -> None:
        source = Path(campaign.__file__).read_text(encoding="utf-8")
        execute_stage = source[
            source.index("def execute_stage(") : source.index("def validate_host_admission_state(")
        ]
        dispatch_branch = execute_stage[execute_stage.index('stage["status"] = "dispatching"') :]
        self.assertLess(
            dispatch_branch.index("runner_online(campaign_dir)"),
            dispatch_branch.index('"workflow",'),
        )
        self.assertLess(
            dispatch_branch.index('"workflow",'),
            dispatch_branch.index("run = discover_run(state, step)"),
        )
        recovery_branch = execute_stage.split(
            'if stage.get("status") == "dispatching":', 1
        )[1].split("\n        else:\n            if matches:", 1)[0]
        self.assertIn("runner_online(campaign_dir)", recovery_branch)
        self.assertIn("run = discover_run(state, step)", recovery_branch)

    def test_transient_run_watch_failure_keeps_runner_online_until_authoritative_completion(self) -> None:
        state = base_state()
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(campaign, "disarm_runner_watchdog") as disarm,
                mock.patch.object(campaign, "arm_runner_watchdog") as arm,
                mock.patch.object(campaign, "runner_online") as online,
                mock.patch.object(campaign, "ensure_runner_offline") as offline,
                mock.patch.object(campaign, "run_visible", side_effect=[1, 0]) as watch,
                mock.patch.object(
                    campaign,
                    "view_run",
                    side_effect=[
                        {"status": "in_progress", "conclusion": ""},
                        {"status": "completed", "conclusion": "success"},
                    ],
                ) as view,
                mock.patch.object(campaign.time, "sleep") as sleep,
            ):
                self.assertEqual(campaign.wait_for_run(campaign_dir, state, 123, "full-dress-1"), 0)
        self.assertEqual(watch.call_count, 2)
        self.assertEqual(view.call_count, 2)
        sleep.assert_called_once_with(15)
        online.assert_called_once_with(campaign_dir)
        offline.assert_called_once_with(campaign_dir)
        self.assertEqual(arm.call_count, 1)
        self.assertEqual(disarm.call_count, 2)

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

    def test_bootstrap_materialization_is_digest_bound_and_resumable(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            evidence = b'{"stable":true}\n'
            relative = "target/test-evidence/0.67/report.json"
            receipt = {
                "sample_index": 1,
                "evidence_files": [
                    {"path": relative, "sha256": campaign.sha256_bytes(evidence)}
                ],
            }
            receipt_data = (json.dumps(receipt) + "\n").encode()
            archive = root / "diagnostic.zip"
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr(relative, evidence)
            materialized = campaign.materialize_bootstrap_input(
                root, 1, receipt_data, archive
            )
            self.assertEqual((materialized / relative).read_bytes(), evidence)
            self.assertEqual(
                campaign.materialize_bootstrap_input(root, 1, receipt_data, archive),
                materialized,
            )
            evidence_path = materialized / relative
            evidence_path.chmod(0o600)
            evidence_path.write_bytes(b"drift")
            with self.assertRaises(campaign.CampaignError):
                campaign.materialize_bootstrap_input(root, 1, receipt_data, archive)

    def test_bootstrap_materialization_rejects_unsafe_or_ambiguous_members(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive = Path(temporary) / "artifact.zip"
            evidence = b"value"
            relative = "target/test-evidence/0.67/report.json"
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr(relative, evidence)
                output.writestr(f"duplicate/{relative}", evidence)
            with self.assertRaises(campaign.CampaignError):
                campaign.read_evidence_member(
                    archive, relative, campaign.sha256_bytes(evidence)
                )
            with self.assertRaises(campaign.CampaignError):
                campaign.read_evidence_member(
                    archive, "../escape", campaign.sha256_bytes(evidence)
                )

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
                "schema_version": 2,
                "stage": "reference-host-irq-burn-in",
                "source_commit": SHA,
                "profile_sha256": "3" * 64,
                "measurement_cpus": "1-4",
                "storage_io_cpus": "0,5-7",
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
