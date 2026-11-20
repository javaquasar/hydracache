import copy
from dataclasses import replace
from hashlib import sha256
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.perf import product_build_receipt_074 as build
from scripts.perf import w11_product_build_receipt_074 as w11


HEAD = "a" * 40
RUN, ATTEMPT = "123", "1"
ENV = {
    "GITHUB_REPOSITORY": build.REPOSITORY,
    "GITHUB_REPOSITORY_ID": build.REPOSITORY_ID,
    "GITHUB_REF": "refs/heads/main",
    "GITHUB_WORKFLOW_REF": build.WORKFLOW,
    "GITHUB_SHA": HEAD,
    "GITHUB_RUN_ID": RUN,
    "GITHUB_RUN_ATTEMPT": ATTEMPT,
}
HEADER = b"\x7fELF\x02\x01\x01" + bytes(9) + b"\x03\x00\x3e\x00" + bytes(44)
BOOT = "12aecb67-49df-4c97-b69a-deb009f40f8c"


def digest(raw):
    return sha256(raw).hexdigest()


def subject(role, directory, binding):
    identity = {"I74": ("1" * 40, "2" * 40), "C74": ("3" * 40, "4" * 40)}[role]
    result = build.receipt_for_source(role, directory, identity, "0" * 64, binding)
    result["schema"] = w11.RECEIPT_SCHEMA
    return result


def cases(accepted, refused):
    return [{"command": command, "entries": size, "outcome": outcome,
             "request_sha256": digest(f"request {command} {size}".encode()),
             "response_sha256": digest(f"response {command} {outcome} {size}".encode()),
             "oracle_sha256": digest(f"oracle {command} {outcome} {size}".encode())}
            for command in ("MGET", "MSET")
            for outcome, counts in (("accepted", accepted), ("refused", refused))
            for size in counts]


def run(selector, binary_sha, pid, rows):
    observed = {"boot_id": BOOT, "pid": pid, "start_ticks": pid * 5, "elf_sha256": binary_sha}
    return {"before": observed, "after": observed.copy(),
            "effective_config": w11.config(selector), "cases": rows}


def evidence(role, folder, binding):
    image = folder / (role + "-server")
    image.write_bytes(HEADER + b"controlled ELF header for " + role.encode())
    identity = subject(role, folder, binding)
    binary = identity["binary"]["sha256"]
    return {"schema": w11.EVIDENCE_SCHEMA, "build": identity,
            "default": run(None, binary, 100, cases((128,), (129,))),
            w11.SELECTOR: run(w11.SELECTOR, binary, 101, cases((128, 129, 256), (257,)))}


def approval(folder, body):
    raw = w11.canonical(body) + b"\n"
    (folder / w11.EVIDENCE).write_bytes(raw)
    fields = body["build"]
    return w11.ApprovedRole(fields["role"], fields["source_commit"], fields["source_tree"],
                            fields["cargo_lock_sha256"], digest(raw))


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        private_root = Path.cwd().resolve()
        temporary = tempfile.TemporaryDirectory(prefix="receipt-fixture-", dir=private_root)
        self.directory = Path(temporary.name).resolve()
        self.assertEqual(self.directory.parent, private_root)
        self.addCleanup(temporary.cleanup)
        self.binding = build.approved_workflow_run(HEAD, RUN, ATTEMPT)

    def attest(self, expected):
        verified = [{"verificationResult": {"statement": {"predicate": expected}}}]
        return patch.object(build.subprocess, "check_output", return_value=json.dumps(verified).encode())

    def test_write_and_validate_both_roles_and_specific_predicate(self):
        for role in ("I74", "C74"):
            with self.subTest(role=role), patch.dict(os.environ, ENV):
                directory = self.directory / role
                directory.mkdir()
                observed = evidence(role, directory, self.binding)
                pins = approval(directory, observed)
                with patch.object(build, "verify_source_identity") as identity:
                    w11.write_receipt(pins, self.directory, directory)
                    identity.assert_called_once_with(self.directory, (pins.source_commit, pins.source_tree),
                                                     pins.cargo_lock_sha256)
                w11.verify_receipt(pins, directory)
                path = directory / w11.RECEIPT
                expected = build.load(path)
                self.assertEqual(expected["w11_profile"], {
                    "selector": "w11-256", "config_sha256": w11.PROFILE_SHA256,
                    "boundary_evidence_sha256": pins.boundary_evidence_sha256,
                    "accepted": [128, 129, 256], "refused": [257],
                })
                self.assertEqual({key: expected[key] for key in observed["build"]}, observed["build"])
                with self.assertRaisesRegex(ValueError, "unexpected files"):
                    w11.write_receipt(pins, self.directory, directory)

                (directory / w11.BUNDLE).write_bytes(b"controlled bundle fixture")
                with self.attest(expected) as gh:
                    self.assertEqual(w11.verify_signed_receipt(pins, directory, HEAD, RUN, ATTEMPT),
                                     digest(path.read_bytes()))
                    gh.assert_called_once()
                    args = gh.call_args.args[0]
                    self.assertEqual(args[3], str(directory / (role + "-server")))
                    for flag, value in {
                        "--predicate-type": w11.PREDICATE, "--repo": build.REPOSITORY,
                        "--signer-workflow": build.WORKFLOW.rsplit("@", 1)[0],
                        "--source-ref": "refs/heads/main", "--source-digest": HEAD,
                        "--signer-digest": HEAD,
                    }.items():
                        self.assertEqual(args[args.index(flag) + 1], value)
                    self.assertIn("--deny-self-hosted-runners", args)

    def test_evidence_must_match_the_independent_pin(self):
        observed = evidence("I74", self.directory, self.binding)
        pins = approval(self.directory, observed)
        leaf = self.directory / w11.EVIDENCE
        leaf.write_bytes(leaf.read_bytes() + b"\n")
        with patch.dict(os.environ, ENV), patch.object(build, "verify_source_identity"):
            with self.assertRaisesRegex(ValueError, "admission pin"):
                w11.write_receipt(pins, self.directory, self.directory)
        self.assertFalse((self.directory / w11.RECEIPT).exists())

    def test_same_byte_archive_cannot_attest_a_changed_elf_or_run(self):
        observed = evidence("I74", self.directory, self.binding)
        pins = approval(self.directory, observed)
        foreign_run = {**self.binding, "run_attempt": "2"}
        with self.assertRaisesRegex(ValueError, "different build"):
            w11.expected_receipt(pins, self.directory, foreign_run)
        image = self.directory / "I74-server"
        image.write_bytes(image.read_bytes() + b"changed")
        with self.assertRaisesRegex(ValueError, "different build"):
            w11.expected_receipt(pins, self.directory, self.binding)

    def test_workflow_binding_has_the_start_readers_exact_domain(self):
        pins = approval(self.directory, evidence("I74", self.directory, self.binding))
        for change in (
            {"role": "C74"}, {"run_id": "0001"}, {"run_id": "18446744073709551616"},
            {"run_attempt": "0"}, {"run_attempt": True}, {"workflow_commit": "A" * 40},
        ):
            with self.subTest(change=change), self.assertRaisesRegex(ValueError, "workflow"):
                w11.expected_receipt(pins, self.directory, {**self.binding, **change})

    def test_even_repinned_invalid_boundaries_are_refused(self):
        observed = evidence("C74", self.directory, self.binding)
        for name, field, value in [
            ("receipt-version", ("schema",), "legacy"),
            ("other-role", ("build", "role"), "I74"),
            ("swapped-source", ("build", "source_commit"), "5" * 40),
            ("other-tree", ("build", "source_tree"), "6" * 40),
            ("other-lock", ("build", "cargo_lock_sha256"), "7" * 64),
            ("wrong-build-run", ("build", "run_id"), "124"),
            ("wrong-run-attempt", ("build", "run_attempt"), "2"),
            ("missing-default-limit", ("default", "effective_config", "shared_batch_entries"), 256),
            ("changed-byte-limit", (w11.SELECTOR, "effective_config", "resp_frame_bytes"), 2097152),
            ("profile-missing", (w11.SELECTOR, "effective_config", "selector"), None),
            ("wrong-image", (w11.SELECTOR, "before", "elf_sha256"), "0" * 64),
            ("generation-change", (w11.SELECTOR, "after", "start_ticks"), 506),
            ("invalid-boot", (w11.SELECTOR, "before", "boot_id"), "bad"),
            ("missing-samples", (w11.SELECTOR, "cases"), observed[w11.SELECTOR]["cases"][:6]),
            ("duplicate-sample", ("default", "cases", 1), observed["default"]["cases"][0]),
            ("wrong-257", (w11.SELECTOR, "cases", 3, "outcome"), "accepted"),
            ("wrong-command", (w11.SELECTOR, "cases", 3, "command"), "SET"),
            ("lost-semantic-proof", (w11.SELECTOR, "cases", 3, "oracle_sha256"), ""),
        ]:
            with self.subTest(name=name):
                bad = copy.deepcopy(observed)
                node = bad
                for component in field[:-1]:
                    node = node[component]
                node[field[-1]] = value
                raw = w11.canonical(bad) + b"\n"
                (self.directory / w11.EVIDENCE).write_bytes(raw)
                pins = w11.ApprovedRole("C74", "3" * 40, "4" * 40, "0" * 64, digest(raw))
                with self.assertRaises(ValueError):
                    w11.expected_receipt(pins, self.directory, self.binding)

    def test_legacy_role_sources_are_not_successor_pins(self):
        good = approval(self.directory, evidence("I74", self.directory, self.binding))
        for commit, tree in build.ROLES.values():
            with self.subTest(commit=commit):
                with self.assertRaisesRegex(ValueError, "legacy"):
                    replace(good, source_commit=commit)
                with self.assertRaisesRegex(ValueError, "legacy"):
                    replace(good, source_tree=tree)

    def test_signed_predicate_mismatch_and_verifier_failure(self):
        observed = evidence("I74", self.directory, self.binding)
        pins = approval(self.directory, observed)
        final = w11.expected_receipt(pins, self.directory, self.binding)
        (self.directory / w11.RECEIPT).write_bytes(w11.canonical(final))
        (self.directory / w11.BUNDLE).write_bytes(b"controlled bundle fixture")
        old = build.receipt("I74", self.directory, self.binding)
        wrong_boundary = copy.deepcopy(final)
        wrong_boundary["w11_profile"]["accepted"] = [128]
        wrong_image = copy.deepcopy(final)
        wrong_image["binary"]["bytes"] = float(final["binary"]["bytes"])
        for predicate in (old, wrong_boundary, wrong_image):
            with self.subTest(schema=predicate["schema"]), self.attest(predicate):
                with self.assertRaisesRegex(ValueError, "signed predicate differs"):
                    w11.verify_signed_receipt(pins, self.directory, HEAD, RUN, ATTEMPT)
        error = subprocess.CalledProcessError(1, ["gh", "attestation", "verify"])
        with patch.object(build.subprocess, "check_output", side_effect=error):
            with self.assertRaises(subprocess.CalledProcessError):
                w11.verify_signed_receipt(pins, self.directory, HEAD, RUN, ATTEMPT)

    def test_unexpected_artifact_entry_and_duplicate_json_are_rejected(self):
        observed = evidence("I74", self.directory, self.binding)
        pins = approval(self.directory, observed)
        (self.directory / "extra").write_bytes(b"unexpected")
        with self.assertRaisesRegex(ValueError, "unexpected files"):
            w11.verify_receipt(pins, self.directory)
        leaf = self.directory / w11.EVIDENCE
        raw = b'{"schema":"other",' + leaf.read_bytes()[1:]
        leaf.write_bytes(raw)
        with self.assertRaisesRegex(ValueError, "duplicate JSON property"):
            w11.expected_receipt(replace(pins, boundary_evidence_sha256=digest(raw)),
                                 self.directory, self.binding)

    def test_current_receipt_and_valid_signing_policy_keep_their_behavior(self):
        image = self.directory / "I74-server"
        image.write_bytes(HEADER)
        expected = build.receipt("I74", self.directory, self.binding)
        self.assertEqual(expected, {
            "schema": "hydracache-product-build-074-v1", "repository_id": "1217101761",
            "role": "I74", "source_commit": "321d94e906dcef95817e67cad46883fb29791ce5",
            "source_tree": "b17f67a66ffed712b1930c6ff9298ee3c57f4bc8",
            "cargo_lock_sha256": "8ae1eb43047d31339a3ce81c768d83db367737281b4ceba723f32c943cb0a3d7",
            "package": "hydracache-server", "profile": "release", "rust_toolchain": "1.94.0",
            "target": "x86_64-unknown-linux-gnu", **self.binding,
            "binary": {"name": "I74-server", "sha256": digest(HEADER), "bytes": len(HEADER)},
        })
        (self.directory / w11.RECEIPT).write_bytes(w11.canonical(expected))
        with patch.dict(os.environ, ENV):
            build.verify_artifact("I74", self.directory)
        with self.attest(expected) as gh:
            build.verify_signed_artifact("I74", self.directory, HEAD, RUN, ATTEMPT)
            args = gh.call_args.args[0]
            self.assertEqual(args[args.index("--predicate-type") + 1],
                             f"https://github.com/{build.REPOSITORY}/attestations/product-build-074/v1")
        branch = {"name": "main", "protected": True}
        policies = {"total_count": 1, "branch_policies": [{"name": "main", "type": "branch"}]}
        environment = {
            "name": build.ENVIRONMENT, "can_admins_bypass": False,
            "deployment_branch_policy": {"protected_branches": False, "custom_branch_policies": True},
            "protection_rules": [{"type": "required_reviewers", "prevent_self_review": True,
                                  "reviewers": [{"type": "User", "reviewer": {"id": build.SIGNING_REVIEWER_ID}}]}],
        }
        build.protection(branch, environment, policies)
        with self.assertRaises(ValueError):
            build.protection(branch, {**environment, "can_admins_bypass": True}, policies)
