import pathlib
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/perf/long-run-supervisor-074/provision-host-074.sh"
WORKFLOW = ROOT / ".github/workflows/performance-long-run-host-provision-074.yml"
CAPABILITY_WORKFLOW = ROOT / ".github/workflows/performance-long-run-host-capability-074.yml"
SUDOERS = ROOT / "scripts/perf/long-run-supervisor-074/hydracache-performance-074.sudoers"


class ProvisionHostContractTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.source = SCRIPT.read_text(encoding="utf-8")
        cls.workflow = WORKFLOW.read_text(encoding="utf-8")
        cls.capability_workflow = CAPABILITY_WORKFLOW.read_text(encoding="utf-8")
        cls.sudoers = SUDOERS.read_text(encoding="utf-8")

    def test_private_signing_key_never_enters_the_host_installer(self) -> None:
        self.assertNotIn("SIGNING_KEY", self.source)
        self.assertNotIn("signing-key", self.source)
        self.assertIn("verification-key.hex", self.source)
        self.assertIn("verification_key_sha256", self.source)

    def test_installer_is_fail_closed_and_refuses_active_drift(self) -> None:
        for required in (
            "set -euo pipefail",
            "(( EUID == 0 ))",
            "sha256sum --strict --check bundle.sha256",
            "verify-provisioning-manifest",
            "bundle installer differs from the root-owned entrypoint",
            "--emit-receipt",
            "--systemd-smoke",
            "--controller-loss-smoke-start",
            "--controller-loss-smoke-resume",
            "--campaign-lifecycle-smoke-start",
            "--campaign-lifecycle-smoke-resume",
            "campaign-lifecycle-fixture",
            "refusing to replace a differing active installation",
            "validate-production-config",
            "systemctl is-active --quiet",
        ):
            self.assertIn(required, self.source)

    def test_installer_binds_identity_hardening_and_receipt(self) -> None:
        for required in (
            "expected_repository_id",
            "allowed_actor_ids",
            "allowed_client_uids",
            "required_client_gid",
            "NoNewPrivileges",
            "ProtectSystem",
            "ProtectControlGroups",
            "RuntimeDirectoryMode",
            "hydracache-w11-host-provisioning-v1",
            "runner_process_group_refresh_may_be_required",
            "fixture_binary_sha256",
        ):
            self.assertIn(required, self.source)

    def test_workflow_keeps_private_key_off_the_self_hosted_runner(self) -> None:
        prepare, provision = self.workflow.split("\n  provision:\n", maxsplit=1)
        self.assertIn("SIGNING_KEY_HEX", prepare)
        self.assertIn("derive-verification-key", prepare)
        self.assertIn("sign-provisioning-manifest", prepare)
        self.assertNotIn("SIGNING_KEY_HEX", provision)
        self.assertNotIn("HYDRACACHE_074_AUTH_SIGNING_KEY_HEX", provision)
        self.assertIn("sudo -n", provision)
        self.assertIn("/usr/local/sbin/hydracache-provision-host-074", provision)
        self.assertNotIn("sudo -n true", provision)
        self.assertIn("--emit-receipt", provision)

    def test_workflow_is_manual_and_does_not_run_qualification(self) -> None:
        self.assertIn("workflow_dispatch:", self.workflow)
        self.assertNotIn("\n  push:", self.workflow)
        self.assertNotIn("\n  schedule:", self.workflow)
        self.assertNotIn("performance-integrated-074", self.workflow)
        self.assertNotIn("duration-hours", self.workflow)
        self.assertNotIn(
            "name: long-run-host-provisioning-074-${{ inputs.source_sha }}-${{ github.run_id }}-${{ github.run_attempt }}",
            self.workflow,
        )
        self.assertIn("overwrite: true", self.workflow)

    def test_registered_capability_workflow_requires_explicit_provision_mode(self) -> None:
        self.assertIn("default: probe", self.capability_workflow)
        self.assertIn("inputs.mode == 'provision'", self.capability_workflow)
        self.assertIn("inputs.mode == 'systemd-smoke'", self.capability_workflow)
        self.assertIn("inputs.mode == 'controller-loss-smoke'", self.capability_workflow)
        self.assertIn("controller-attach-abort-rehearsal", self.capability_workflow)
        self.assertIn("needs: controller-recovery-attach", self.capability_workflow)
        self.assertIn(
            "expected_state_revision: ${{ inputs.abort_expected_state_revision }}",
            self.capability_workflow,
        )
        self.assertIn("controller-start-seal-rehearsal", self.capability_workflow)
        self.assertIn("controller-attach-seal-rehearsal", self.capability_workflow)
        self.assertIn("needs: controller-rehearsal-terminal-attach", self.capability_workflow)
        self.assertIn("name: Attach the successful terminal evidence", self.capability_workflow)
        self.assertIn("operation: seal", self.capability_workflow)
        self.assertIn('expected_state_revision: "3"', self.capability_workflow)
        self.assertIn("request_id: ${{ inputs.seal_request_id }}", self.capability_workflow)
        self.assertIn("needs: controller-recovery-attach", self.capability_workflow)
        self.assertIn(
            "expected_state_revision: ${{ inputs.seal_expected_state_revision }}",
            self.capability_workflow,
        )
        self.assertIn(
            "hydracache-performance-074-i74-${INPUT_CAMPAIGN_ID}.service",
            self.capability_workflow,
        )
        self.assertIn("inputs.mode == 'campaign-lifecycle-smoke'", self.capability_workflow)
        self.assertIn("product_candidate_started", self.capability_workflow)
        self.assertIn("original_controller_exited", self.capability_workflow)
        self.assertIn("fixture_identity_unchanged", self.capability_workflow)
        self.assertIn("start_response_replayed", self.capability_workflow)
        self.assertIn("replay_spawn_calls", self.capability_workflow)
        self.assertIn("active_campaign_released", self.capability_workflow)
        self.assertIn("github.event_name == 'workflow_dispatch'", self.capability_workflow)
        self.assertIn("source_sha: ${{ github.sha }}", self.capability_workflow)
        self.assertIn("secrets: inherit", self.capability_workflow)

    def test_sudo_boundary_is_only_the_root_owned_installer(self) -> None:
        self.assertIn("NOPASSWD: /usr/local/sbin/hydracache-provision-host-074", self.sudoers)
        self.assertNotIn("NOPASSWD: ALL", self.sudoers)
        self.assertNotIn("/bin/sh", self.sudoers)


if __name__ == "__main__":
    unittest.main()
