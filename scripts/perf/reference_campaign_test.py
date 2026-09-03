#!/usr/bin/env python3

import json
from pathlib import Path
import subprocess
import sys
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
    def test_full_dress_admission_binds_exact_run_receipt_pairs_and_contracts(self) -> None:
        contracts = {
            "runner_provisioning_sha256": "1" * 64,
            "prebuild_contract_digest": "2" * 64,
            "scenario_contract_set_digest": "3" * 64,
        }
        receipt_identity = {
            "schema_version": 1,
            "release": "0.67.1",
            "profile": "reference-v1",
            "source_commit": SHA,
            "runner_fingerprint": FINGERPRINT,
        }
        first = {**receipt_identity, **contracts}
        second = {**receipt_identity, **contracts}
        expected_members = {"101": "4" * 64, "202": "5" * 64}
        admission = {
            "schema_version": 1,
            "release": "0.67.1",
            "profile": "reference-v1",
            "source_commit": SHA,
            "runner_fingerprint": FINGERPRINT,
            **contracts,
            "full_dress_runs": [
                {"github_run_id": run_id, "receipt_sha256": digest}
                for run_id, digest in expected_members.items()
            ],
            "passed": True,
            "bootstrap_admission_eligible": True,
            "bootstrap_eligible": False,
            "ship_evidence_eligible": False,
        }
        self.assertEqual(
            campaign.validate_full_dress_admission_chain(
                admission, first, second, expected_members
            ),
            contracts,
        )
        admission["full_dress_runs"][0]["receipt_sha256"] = "5" * 64
        with self.assertRaisesRegex(campaign.CampaignError, "mapping is wrong"):
            campaign.validate_full_dress_admission_chain(
                admission, first, second, expected_members
            )

    def test_frozen_receipt_seal_and_ship_aggregate_are_recomputed(self) -> None:
        receipt = {
            "schema_version": 1,
            "release": "0.67.1",
            "receipt_sha256": "",
        }
        receipt["receipt_sha256"] = campaign.sha256_bytes(
            json.dumps(
                receipt, ensure_ascii=False, separators=(",", ":")
            ).encode("utf-8")
        )
        self.assertTrue(campaign.sealed_json_receipt_is_valid(receipt))
        receipt["release"] = "0.67.2"
        self.assertFalse(campaign.sealed_json_receipt_is_valid(receipt))

        aggregate = {
            "schema_version": 1,
            "release": "0.67.1",
            "source_commit": SHA,
            "current_worktree_dirty": False,
            "receipts_supplied": True,
            "counts": {
                "planned": 0,
                "implemented": 0,
                "fast-green": 0,
                "gated-green": 0,
                "ship-ready": 8,
            },
            "reasons": [],
            "work_items": [
                {"id": f"W{index}", "stage": "ship-ready", "reasons": []}
                for index in range(8)
            ],
        }
        campaign.validate_ship_aggregate(aggregate, base_state())
        aggregate["work_items"][0]["stage"] = "gated-green"
        with self.assertRaises(campaign.CampaignError):
            campaign.validate_ship_aggregate(aggregate, base_state())

    def test_frozen_receipt_requires_every_digest_bound_archive_member(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            archive_path = Path(temporary) / "frozen.zip"
            activation = b'{"activation":true}\n'
            verdict = b'{"verdict":true}\n'
            reference_path = "target/test-evidence/0.67/local.json"
            reference = b'{"reference":true}\n'
            canaries = {
                f"target/release-evidence/canaries/0.67.1-W{index}.json": (
                    f'{{"canary":"W{index}"}}\n'.encode()
                )
                for index in range(8)
            }
            with zipfile.ZipFile(archive_path, "w") as archive:
                archive.writestr(campaign.FROZEN_ACTIVATION_PATH.removeprefix("target/"), activation)
                archive.writestr(campaign.FROZEN_BUDGET_VERDICT_PATH, verdict)
                archive.writestr(reference_path, reference)
                for relative, data in canaries.items():
                    archive.writestr(relative.removeprefix("target/"), data)

            receipt = {
                "schema_version": 1,
                "release": "0.67.1",
                "profile": "reference-v1",
                "source_commit": SHA,
                "github_run_id": "123",
                "runner_fingerprint": FINGERPRINT,
                "activation_sha256": campaign.sha256_bytes(activation),
                "budget_verdict_sha256": campaign.sha256_bytes(verdict),
                "reference_evidence_sha256": [
                    {
                        "id": reference_path,
                        "path": reference_path,
                        "sha256": campaign.sha256_bytes(reference),
                    }
                ],
                "canary_receipt_sha256": [
                    {
                        "id": f"W{index}",
                        "path": relative,
                        "sha256": campaign.sha256_bytes(canaries[relative]),
                    }
                    for index, relative in enumerate(canaries)
                ],
                "passed": True,
                "ship_evidence_eligible": True,
                "receipt_sha256": "6" * 64,
            }
            campaign.validate_frozen_receipt_artifacts(receipt, archive_path)
            receipt["reference_evidence_sha256"][0]["sha256"] = "7" * 64
            with self.assertRaisesRegex(campaign.CampaignError, "digest mismatch"):
                campaign.validate_frozen_receipt_artifacts(receipt, archive_path)

    def test_visible_command_timeout_is_bounded(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            with self.assertRaisesRegex(campaign.CampaignError, "timed out"):
                campaign.run_visible(
                    [sys.executable, "-c", "import time; time.sleep(5)"],
                    cwd=root,
                    log_path=root / "timeout.log",
                    timeout_seconds=0.05,
                )

    def test_sample_set_cargo_is_probed_as_the_runner_user(self) -> None:
        runner_cargo = "/home/github-runner/.cargo/bin/cargo"
        with (
            mock.patch.object(
                campaign.subprocess,
                "run",
                return_value=mock.Mock(returncode=0),
            ) as run,
            mock.patch.object(campaign, "repo_root", return_value=Path("/repo")),
            mock.patch.object(campaign.shutil, "which", return_value=runner_cargo),
        ):
            self.assertEqual(
                campaign.select_sample_set_cargo(),
                campaign.runner_command(runner_cargo),
            )

        run.assert_called_once_with(
            campaign.runner_command("test", "-x", runner_cargo),
            cwd=Path("/repo"),
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
            timeout=30,
        )

    def test_sample_set_cargo_rejects_inaccessible_runner_path_fallback(self) -> None:
        runner_cargo = "/home/github-runner/.cargo/bin/cargo"
        with (
            mock.patch.object(
                campaign.subprocess,
                "run",
                return_value=mock.Mock(returncode=1),
            ),
            mock.patch.object(campaign.shutil, "which", return_value=runner_cargo),
            self.assertRaisesRegex(campaign.CampaignError, "cargo is unavailable"),
        ):
            campaign.select_sample_set_cargo()

    def test_sample_set_validator_uses_unique_bounded_output(self) -> None:
        data = b'{"bootstrap_eligible":true}\n'
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo = root / "repo"
            campaign_dir = root / "hc0671-test-campaign"
            repo.mkdir()
            campaign_dir.mkdir()

            def run_validator(command: list[str], **kwargs: object) -> int:
                output = Path(command[command.index("--output") + 1])
                output.parent.mkdir(parents=True)
                output.write_bytes(data)
                self.assertEqual(
                    kwargs.get("timeout_seconds"),
                    campaign.SAMPLE_SET_VALIDATION_TIMEOUT_SECONDS,
                )
                return 0

            with (
                mock.patch.object(campaign, "repo_root", return_value=repo),
                mock.patch.object(
                    campaign, "select_sample_set_cargo", return_value=["cargo"]
                ),
                mock.patch.object(
                    campaign, "run_visible", side_effect=run_validator
                ) as run,
            ):
                retained = campaign.cargo_sample_set(campaign_dir)

            self.assertEqual(retained.read_bytes(), data)
            command = run.call_args.args[0]
            output = Path(command[command.index("--output") + 1])
            self.assertIn("controller-sample-sets", output.parts)
            self.assertNotEqual(
                output,
                repo / "target/test-evidence/0.67.1/bootstrap-sample-set.json",
            )

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
            ["gh", "auth", "status"],
            cwd=Path("/repo"),
            timeout_seconds=campaign.GITHUB_CONTROL_TIMEOUT_SECONDS,
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

    def test_reference_dispatch_is_isolated_from_scheduled_ci_concurrency(self) -> None:
        workflow = (campaign.repo_root() / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        concurrency = workflow.split("concurrency:", 1)[1].split("\npermissions:", 1)[0]
        self.assertIn("github.event_name", concurrency)
        self.assertIn("github.event_name == 'pull_request'", concurrency)
        self.assertIn("github.event_name == 'push'", concurrency)
        self.assertNotIn(
            "cancel-in-progress: ${{ !startsWith(github.ref, 'refs/tags/v') }}",
            concurrency,
        )

    def test_reference_job_timeout_and_runner_watchdog_contract(self) -> None:
        workflow = (campaign.repo_root() / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
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
            "release-0671-performance-qualification",
            "release-0671-performance-full-dress",
            "release-0671-performance-bootstrap",
            "release-0671-frozen-candidate",
        ):
            block = job_block(job)
            self.assertIn("timeout-minutes: 360", block, job)
            self.assertIn("group: release-067-performance-reference-v1", block, job)
            self.assertIn("cancel-in-progress: false", block, job)

        source = Path(campaign.__file__).read_text(encoding="utf-8")
        self.assertIn('"--on-active=370m"', source)

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

    def test_transient_artifact_download_failure_retries_atomic_download(self) -> None:
        state = base_state()
        expected = {"diagnostics": Path("diagnostics.zip")}
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(
                    campaign,
                    "download_artifacts",
                    side_effect=[campaign.ArtifactTransportError("TLS timeout"), expected],
                ) as download,
                mock.patch.object(campaign.time, "sleep") as sleep,
            ):
                self.assertEqual(
                    campaign.download_artifacts_with_retry(
                        campaign_dir, state, 123, "full-dress-1"
                    ),
                    expected,
                )
        self.assertEqual(download.call_count, 2)
        sleep.assert_called_once_with(15)

    def test_artifact_download_timeout_removes_partial_file(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary) / "artifact.zip"
            with mock.patch.object(
                campaign.subprocess,
                "run",
                side_effect=subprocess.TimeoutExpired(["gh", "api"], 23),
            ) as run:
                with self.assertRaisesRegex(
                    campaign.CampaignError, "timed out after 23 seconds"
                ):
                    campaign.download_binary(["gh", "api"], output, timeout_seconds=23)
        self.assertEqual(run.call_args.kwargs["timeout"], 23)
        self.assertFalse(output.exists())
        self.assertFalse(output.with_name(".artifact.zip.partial").exists())

    def test_persistent_artifact_download_failure_remains_resumable(self) -> None:
        state = base_state()
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(
                    campaign,
                    "download_artifacts",
                    side_effect=campaign.ArtifactTransportError("still unavailable"),
                ) as download,
                mock.patch.object(campaign.time, "sleep") as sleep,
                self.assertRaisesRegex(
                    campaign.ArtifactTransportError,
                    f"failed after {campaign.ARTIFACT_DOWNLOAD_ATTEMPTS} attempts",
                ),
            ):
                campaign.download_artifacts_with_retry(
                    campaign_dir, state, 123, "full-dress-1"
                )
        self.assertEqual(download.call_count, campaign.ARTIFACT_DOWNLOAD_ATTEMPTS)
        self.assertEqual(
            [call.args[0] for call in sleep.call_args_list],
            [15, 30, *([60] * (campaign.ARTIFACT_DOWNLOAD_ATTEMPTS - 3))],
        )

    def test_artifact_integrity_failure_is_not_retried(self) -> None:
        state = base_state()
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(
                    campaign,
                    "download_artifacts",
                    side_effect=campaign.ArtifactIntegrityError("bad ZIP"),
                ) as download,
                mock.patch.object(campaign.time, "sleep") as sleep,
                self.assertRaisesRegex(campaign.ArtifactIntegrityError, "bad ZIP"),
            ):
                campaign.download_artifacts_with_retry(
                    campaign_dir, state, 123, "full-dress-1"
                )
        download.assert_called_once()
        sleep.assert_not_called()

    def test_retained_artifact_manifest_is_run_bound_and_cannot_escape(self) -> None:
        state = base_state()
        run_id = 123
        step = "bootstrap-1"
        artifact_id = 456
        name = "performance-0671-bootstrap-example"
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            run_dir = campaign_dir / "runs" / f"{step}-{run_id}"
            originals = run_dir / "original-artifacts"
            originals.mkdir(parents=True)
            archive = originals / campaign.safe_artifact_filename(artifact_id, name)
            archive.write_bytes(b"retained-zip")
            relative = archive.relative_to(campaign_dir)
            manifest = {
                "schema_version": 1,
                "run_id": run_id,
                "step": step,
                "artifacts": [
                    {
                        "artifact_id": artifact_id,
                        "name": name,
                        "reported_size_bytes": archive.stat().st_size,
                        "archive_file": str(relative),
                        "archive_size_bytes": archive.stat().st_size,
                        "archive_sha256": campaign.sha256_file(archive),
                    }
                ],
            }
            manifest_path = run_dir / "artifact-manifest.json"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            self.assertEqual(
                campaign.download_artifacts(campaign_dir, state, run_id, step),
                {name: archive},
            )

            manifest["artifacts"][0]["archive_file"] = "../outside.zip"
            manifest_path.write_text(json.dumps(manifest), encoding="utf-8")
            with self.assertRaises(campaign.ArtifactIntegrityError):
                campaign.download_artifacts(campaign_dir, state, run_id, step)

    def test_artifact_lookup_requires_the_exact_name(self) -> None:
        expected = "performance-0671-bootstrap-sha-run"
        with self.assertRaises(campaign.CampaignError):
            campaign.artifact_named(
                {f"{expected}-unexpected-suffix": Path("artifact.zip")}, expected
            )

    def test_artifact_transport_canary_downloads_smallest_readable_zip(self) -> None:
        state = base_state()
        listing = {
            "artifacts": [
                {"id": 11, "name": "large", "size_in_bytes": 2000, "expired": False},
                {"id": 12, "name": "small", "size_in_bytes": 1000, "expired": False},
            ]
        }

        def write_zip(_args: list[str], output: Path, **_kwargs: object) -> None:
            with zipfile.ZipFile(output, "w") as archive:
                archive.writestr("canary.txt", "ok")
            output.chmod(0o444)

        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(campaign, "gh_json", return_value=listing),
                mock.patch.object(campaign, "download_binary", side_effect=write_zip) as download,
            ):
                campaign.check_artifact_transport_canary(campaign_dir, state, "qualification")
            receipt_path = campaign_dir / "artifact-transport-canary-qualification.json"
            receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
            self.assertEqual(receipt["artifact_id"], 12)
            self.assertRegex(receipt["archive_sha256"], r"^[0-9a-f]{64}$")
            self.assertFalse((campaign_dir / ".artifact-transport-canary-qualification.zip").exists())
        self.assertIn("actions/artifacts/12/zip", download.call_args.args[0][-1])

    def test_artifact_transport_canary_blocks_before_frozen_host_check(self) -> None:
        state = base_state()
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(campaign, "ensure_checkout"),
                mock.patch.object(campaign, "github_main_sha", return_value=SHA),
                mock.patch.object(campaign, "ensure_github_runner_contract"),
                mock.patch.object(campaign, "ensure_runner_offline"),
                mock.patch.object(
                    campaign,
                    "check_artifact_transport_canary",
                    side_effect=campaign.ArtifactTransportError("blob unavailable"),
                ),
                mock.patch.object(campaign, "run_host_action") as host_action,
                self.assertRaisesRegex(campaign.ArtifactTransportError, "blob unavailable"),
            ):
                campaign.check_pre_dispatch(campaign_dir, state, "qualification")
        host_action.assert_not_called()

    def test_successful_run_with_transport_failure_resumes_same_run(self) -> None:
        state = base_state()
        state["phase"] = "running"
        state["stages"]["qualification"] = {"status": "running", "run_id": 123}
        run = {
            "databaseId": 123,
            "displayTitle": campaign.expected_title(state, "qualification"),
            "headSha": SHA,
            "status": "completed",
            "conclusion": "success",
            "url": "https://example.invalid/run/123",
        }
        artifacts = {"diagnostic": Path("diagnostic.zip")}
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            with (
                mock.patch.object(campaign, "view_run", return_value=run),
                mock.patch.object(campaign, "assert_no_foreign_reference_runs"),
                mock.patch.object(campaign, "ensure_runner_offline"),
                mock.patch.object(campaign, "disarm_runner_watchdog"),
                mock.patch.object(
                    campaign,
                    "download_artifacts_with_retry",
                    side_effect=[campaign.ArtifactTransportError("blob unavailable"), artifacts],
                ) as download,
                mock.patch.object(campaign, "run_host_action"),
                mock.patch.object(campaign, "run_visible", return_value=0),
                mock.patch.object(
                    campaign, "validate_stage_artifacts", return_value={"receipt": "receipt.json"}
                ),
                mock.patch.object(campaign, "check_pre_dispatch") as pre_dispatch,
            ):
                with self.assertRaisesRegex(campaign.CampaignError, "awaiting artifact retrieval"):
                    campaign.execute_stage(
                        campaign_dir, state, {"name": "qualification"}
                    )
                self.assertEqual(state["phase"], "awaiting-artifacts")
                self.assertEqual(
                    state["stages"]["qualification"]["status"], "awaiting-artifacts"
                )
                self.assertEqual(state["stages"]["qualification"]["run_id"], 123)

                state["phase"] = "running"
                campaign.execute_stage(campaign_dir, state, {"name": "qualification"})

            self.assertEqual(state["stages"]["qualification"]["status"], "completed")
            self.assertEqual(state["stages"]["qualification"]["run_id"], 123)
            self.assertNotIn(
                "artifact_transport_error", state["stages"]["qualification"]
            )
        self.assertEqual(download.call_count, 2)
        pre_dispatch.assert_not_called()

    def test_command_run_accepts_awaiting_artifacts_phase(self) -> None:
        state = base_state()
        state["phase"] = "awaiting-artifacts"
        state["stages"]["bootstrap-5"] = {"status": "completed"}
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            args = mock.Mock(campaign_dir=str(campaign_dir))
            with (
                mock.patch.object(
                    campaign, "ensure_external_campaign_dir", return_value=campaign_dir
                ),
                mock.patch.object(campaign, "load_state", return_value=state),
                mock.patch.object(campaign, "require_tools"),
                mock.patch.object(campaign, "require_github_dispatch_readiness"),
                mock.patch.object(campaign, "ensure_checkout"),
                mock.patch.object(campaign, "validate_host_admission_state"),
                mock.patch.object(campaign, "pin_controller_to_housekeeping"),
                mock.patch.object(campaign, "stage_specs", return_value=[]),
                mock.patch.object(campaign, "ensure_runner_offline"),
                mock.patch.object(campaign, "run_host_action"),
                mock.patch.object(
                    campaign, "cargo_sample_set", return_value=campaign_dir / "samples.json"
                ),
                mock.patch.object(campaign, "sha256_file", return_value="a" * 64),
                mock.patch.object(
                    campaign,
                    "prepare_reference_inputs",
                    return_value=campaign_dir / "reference-inputs.json",
                ),
                mock.patch.object(campaign, "save_state"),
                mock.patch.object(campaign, "append_event"),
                mock.patch.object(campaign, "write_summary"),
            ):
                campaign.command_run(args)
        self.assertEqual(state["phase"], "complete")

    def test_command_frozen_resumes_awaiting_artifacts_without_redispatch(self) -> None:
        state = base_state()
        state["phase"] = "awaiting-artifacts"
        state["stages"]["frozen-candidate"] = {
            "status": "awaiting-artifacts",
            "run_id": 123,
        }
        with tempfile.TemporaryDirectory() as temporary:
            campaign_dir = Path(temporary)
            args = mock.Mock(campaign_dir=str(campaign_dir))
            with (
                mock.patch.object(
                    campaign, "ensure_external_campaign_dir", return_value=campaign_dir
                ),
                mock.patch.object(campaign, "load_state", return_value=state),
                mock.patch.object(campaign, "require_tools"),
                mock.patch.object(campaign, "run_capture"),
                mock.patch.object(campaign, "ensure_checkout"),
                mock.patch.object(campaign, "validate_host_admission_state"),
                mock.patch.object(campaign, "pin_controller_to_housekeeping"),
                mock.patch.object(campaign, "execute_stage") as execute,
                mock.patch.object(campaign, "ensure_runner_offline"),
                mock.patch.object(campaign, "run_host_action"),
                mock.patch.object(campaign, "save_state"),
                mock.patch.object(campaign, "append_event"),
                mock.patch.object(campaign, "write_summary"),
            ):
                campaign.command_frozen(args)

        self.assertEqual(state["phase"], "complete")
        execute.assert_called_once()
        self.assertEqual(execute.call_args.args[2]["name"], "frozen-candidate")

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
                "source_commit": SHA,
                "runner_fingerprint": FINGERPRINT,
                "evidence_files": [],
            }
            receipt_data = (json.dumps(receipt) + "\n").encode()
            archive = root / "diagnostic.zip"
            raw_relative = "target/test-evidence/0.67/w7-raw/report.raw.json"
            raw = b'{"raw":true}\n'
            prebuild_relative = "target/test-evidence/0.67/prebuild-manifest.json"
            prebuild = b'{"prebuild":true}\n'
            marker_relative = (
                "target/test-evidence/0.67/w7-raw/macro-publication-receipt.json"
            )
            marker = {
                "schema_version": 1,
                "source_commit": SHA,
                "runner_profile": "reference-v1",
                "runner_fingerprint": FINGERPRINT,
                "prebuild_manifest_sha256": campaign.sha256_bytes(prebuild),
                "artifacts": [
                    {
                        "canonical_path": relative,
                        "envelope_sha256": campaign.sha256_bytes(evidence),
                        "raw_sidecar_path": raw_relative,
                        "raw_sha256": campaign.sha256_bytes(raw),
                    }
                ],
            }
            marker_data = json.dumps(marker).encode()
            receipt["evidence_files"] = [
                {"path": relative, "sha256": campaign.sha256_bytes(evidence)},
                {"path": raw_relative, "sha256": campaign.sha256_bytes(raw)},
                {
                    "path": prebuild_relative,
                    "sha256": campaign.sha256_bytes(prebuild),
                },
                {
                    "path": marker_relative,
                    "sha256": campaign.sha256_bytes(marker_data),
                },
            ]
            receipt_data = (json.dumps(receipt) + "\n").encode()
            with zipfile.ZipFile(archive, "w") as output:
                output.writestr(relative.removeprefix("target/"), evidence)
                output.writestr(raw_relative.removeprefix("target/"), raw)
                output.writestr(prebuild_relative.removeprefix("target/"), prebuild)
                output.writestr(
                    marker_relative.removeprefix("target/"), marker_data
                )
            materialized = campaign.materialize_bootstrap_input(
                root, 1, receipt_data, archive
            )
            self.assertEqual((materialized / relative).read_bytes(), evidence)
            self.assertEqual((materialized / raw_relative).read_bytes(), raw)
            self.assertEqual((materialized / prebuild_relative).read_bytes(), prebuild)
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
