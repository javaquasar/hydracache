"""Copied-record mutations; no refetch, signing, installation or host action."""
import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location("packet", Path(__file__).with_name("verify_packet.py"))
packet = importlib.util.module_from_spec(spec)
spec.loader.exec_module(packet)


class PacketChecks(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.original = [packet.record(name) for name in (
            "host-inventory.json", "supervisor-inventory.json", "preflight-before.json",
            "preflight-after.json", "github-inventory.json", "oidc-acquisition.json", "proposal.json")]

    def test_retained_records(self):
        packet.check_records(*self.original)

    def test_every_closed_proposal_flag_cannot_promote_even_after_rehash(self):
        for name in ("operator_selection_approved", "host_admission", "trust_config_installed",
                     "policy_signed", "workflow_enabled", "server_execution_allowed", "qualification_allowed", "ship_allowed"):
            for value in (True, 0):
                records = copy.deepcopy(self.original)
                records[6][name] = value
                with self.subTest(name=name, value=value), self.assertRaises(ValueError):
                    packet.check_records(*records)

    def mutation(self, change):
        records = copy.deepcopy(self.original)
        change(records)
        with self.assertRaises(ValueError):
            packet.check_records(*records)

    def test_ssh_namespace_cannot_become_receiver_pin(self):
        self.mutation(lambda r: r[6]["host_proposal"].update(actual_receiver_namespace_selected=True,
                      actual_receiver_mount_namespace=r[0]["context_after"]["namespaces"]["thread-self"]["mnt"]))

    def test_new_epoch_proposal_is_not_observed_authority(self):
        self.mutation(lambda r: r[6]["worker_proposal"].update(observed_current_epoch=1))
        self.mutation(lambda r: r[6]["worker_proposal"].update(epoch_initialization_approved=True))
        self.mutation(lambda r: r[6]["worker_proposal"].update(proposed_initial_epoch=True))

    def test_worker_file_digest_substitution(self):
        self.mutation(lambda r: r[6]["worker_proposal"].update(passwd_sha256="a" * 64))

    def test_campaign_or_restart_drift(self):
        self.mutation(lambda r: r[3]["lifecycle_after"]["markers"][0].update(status="present"))
        self.mutation(lambda r: r[3]["lifecycle_after"]["supervisor"].update(stdout="MainPID=8839\nNRestarts=1\n"))

    def test_github_mutating_method_refuses(self):
        self.mutation(lambda r: r[4]["first"][0].update(method="PUT"))

    def test_missing_environment_cannot_become_enrolled(self):
        self.mutation(lambda r: next(e for e in r[4]["first"] if e["id"] == "controller_environment").update(exit_code=0))

    def test_invented_signer_receipt_refuses(self):
        self.mutation(lambda r: r[6]["job_proposal"].update(signer_build_receipt_sha256="a" * 64))

    def test_roster_substitution_refuses(self):
        self.mutation(lambda r: r[6]["oidc_proposal"]["proposed_members"][0].update(public_key_projection_sha256="b" * 64))

    def test_snapshot_extension_refuses(self):
        original_expiry = self.original[6]["oidc_proposal"]["proposed_snapshot_expires_at"]
        self.mutation(lambda r: r[6]["oidc_proposal"].update(proposed_snapshot_expires_at=original_expiry + 1))

    def test_duplicate_json_refuses(self):
        with self.assertRaises(ValueError):
            packet.decode('{"approved":false,"approved":true}')


if __name__ == "__main__":
    unittest.main()
