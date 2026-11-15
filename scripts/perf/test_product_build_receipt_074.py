import copy
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

from scripts.perf import product_build_receipt_074 as build


class ProtectionTests(unittest.TestCase):
    def setUp(self):
        self.branch = {"name": "main", "protected": True}
        self.environment = {
            "name": build.ENVIRONMENT,
            "deployment_branch_policy": {"protected_branches": False, "custom_branch_policies": True},
            "protection_rules": [{"type": "required_reviewers", "prevent_self_review": True, "reviewers": [{"id": 1}]}],
            "can_admins_bypass": False,
        }
        self.policies = {"total_count": 1, "branch_policies": [{"name": "main", "type": "branch"}]}

    def test_environment_and_exact_branch_must_be_protected(self):
        build.protection(self.branch, self.environment, self.policies)
        with self.assertRaises(ValueError):
            build.protection({**self.branch, "protected": False}, self.environment, self.policies)
        with self.assertRaises(ValueError):
            build.protection(self.branch, {**self.environment, "can_admins_bypass": True}, self.policies)
        missing_bypass_status = {**self.environment}
        del missing_bypass_status["can_admins_bypass"]
        with self.assertRaises(ValueError):
            build.protection(self.branch, missing_bypass_status, self.policies)
        permissive = {"total_count": 1, "branch_policies": [{"name": "*", "type": "branch"}]}
        with self.assertRaises(ValueError):
            build.protection(self.branch, self.environment, permissive)
        self_approval = copy.deepcopy(self.environment)
        self_approval["protection_rules"][0]["prevent_self_review"] = False
        with self.assertRaises(ValueError):
            build.protection(self.branch, self_approval, self.policies)


class ArtifactTests(unittest.TestCase):
    def test_role_image_and_workflow_attempt_are_bound(self):
        identity = {"GITHUB_REPOSITORY": build.REPOSITORY, "GITHUB_REPOSITORY_ID": build.REPOSITORY_ID,
                    "GITHUB_REF": "refs/heads/main", "GITHUB_WORKFLOW_REF": build.WORKFLOW,
                    "GITHUB_SHA": "a" * 40, "GITHUB_RUN_ID": "1", "GITHUB_RUN_ATTEMPT": "1"}
        with tempfile.TemporaryDirectory() as tmp, patch.dict("os.environ", identity):
            folder = Path(tmp)
            image = folder / "I74-server"
            header = b"\x7fELF\x02\x01\x01" + bytes(9) + b"\x03\x00\x3e\x00" + bytes(44)
            image.write_bytes(header)
            receipt_path = folder / "build-receipt.json"
            receipt = build.receipt("I74", folder)
            receipt_path.write_text(json.dumps(receipt))
            build.verify_artifact("I74", folder)
            verified = [{"verificationResult": {"statement": {"predicate": receipt}}}]
            with patch.object(build.subprocess, "check_output", return_value=json.dumps(verified)) as gh:
                build.verify_signed_artifact("I74", folder, "a" * 40, "1", "1")
                argv = gh.call_args.args[0]
                self.assertIn("--deny-self-hosted-runners", argv)
                self.assertEqual(argv[argv.index("--signer-digest") + 1], "a" * 40)
            wrong = [{"verificationResult": {"statement": {"predicate": {**receipt, "role": "C74"}}}}]
            with patch.object(build.subprocess, "check_output", return_value=json.dumps(wrong)):
                with self.assertRaises(ValueError):
                    build.verify_signed_artifact("I74", folder, "a" * 40, "1", "1")
            with self.assertRaises(ValueError):
                build.verify_artifact("C74", folder)
            with patch.dict("os.environ", {"GITHUB_RUN_ATTEMPT": "2"}), self.assertRaises(ValueError):
                build.verify_artifact("I74", folder)
            image.write_bytes(header + b"modified")
            with self.assertRaises(ValueError):
                build.verify_artifact("I74", folder)
