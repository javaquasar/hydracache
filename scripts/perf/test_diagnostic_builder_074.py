"""Deterministic local policy/coordinator tests; no compilation, keys or workloads."""
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from scripts.perf import diagnostic_builder_074 as builder


class DiagnosticBuilderTests(unittest.TestCase):
    def test_build_environment_removes_secrets_and_overrides(self):
        env = {"PATH": "fixed", "HOME": "fixed", "LANG": "C",
               "BUILDER_SIGNING_KEY_HEX": "not-real", "GITHUB_TOKEN": "not-real",
               "ACTIONS_RUNTIME_TOKEN": "not-real", "RUSTFLAGS": "override",
               "CARGO_TARGET_DIR": "override", "RUSTC_WRAPPER": "override",
               "CARGO_PROFILE_RELEASE_OPT_LEVEL": "0", "LD_PRELOAD": "override"}
        self.assertEqual(builder.build_environment(env), {"PATH": "fixed", "HOME": "fixed", "LANG": "C"})

    def test_policy_requires_external_pin_and_separate_public_keys(self):
        env = {"BUILDER_PUBLIC_KEY_HEX": "a" * 64,
               "CONTROLLER_PUBLIC_KEY_HEX": "b" * 64,
               "BUILDER_POLICY_SHA256": "0" * 64}
        raw = builder.canonical({"schema_version": "diagnostic-builder-policy-074-v1",
                                 "repository_id": 1217101761, "builder_id": "hydracache-linux-observer-074-v1",
                                 "builder_key_hex": "a" * 64, "controller_key_hex": "b" * 64})
        with self.assertRaises(ValueError):
            builder.policy(env)
        env["BUILDER_POLICY_SHA256"] = hashlib.sha256(raw).hexdigest()
        self.assertEqual(builder.policy(env), raw)
        for key in ("a" * 64, "B" * 64, "b" * 63, "g" * 64, ""):
            with self.assertRaises(ValueError):
                builder.policy({**env, "CONTROLLER_PUBLIC_KEY_HEX": key})
        with self.assertRaises(KeyError):
            builder.policy({})

    def test_source_observation_refuses_dirty_or_other_source(self):
        with patch.object(builder.subprocess, "check_output", side_effect=[
                builder.SOURCE.encode(), builder.TREE.encode(), b""]):
            self.assertEqual(builder.source_observation(Path("unused"), {}), (builder.SOURCE, builder.TREE))
        for values in ([b"wrong", builder.TREE.encode(), b""],
                       [builder.SOURCE.encode(), b"wrong", b""],
                       [builder.SOURCE.encode(), builder.TREE.encode(), b" M source"]):
            with patch.object(builder.subprocess, "check_output", side_effect=values), self.assertRaises(ValueError):
                builder.source_observation(Path("unused"), {})

    def test_fixed_inputs_require_actual_frozen_hashes(self):
        root = Path(__file__).resolve().parents[2]
        real = builder.checked_inputs(root, root / "docs/testing/performance/0.74/rental-pilot-draft")
        self.assertEqual(set(real), set(builder.HASHES))
        with tempfile.TemporaryDirectory() as temp:
            with self.assertRaises(ValueError):
                builder.checked_inputs(Path(temp), Path(temp))

    def test_workflow_separates_build_and_signing_without_host_routes(self):
        root = Path(__file__).resolve().parents[2]
        workflow = (root / ".github/workflows/diagnostic-builder-074.yml").read_text()
        build, sign = workflow.split("  sign:", 1)
        self.assertNotIn("secrets.", build)
        self.assertIn("needs: build", sign)
        self.assertIn("environment: performance-diagnostic-builder-074", sign)
        self.assertIn("unset BUILDER_SIGNING_KEY_HEX", sign)
        self.assertIn("--bin diagnostic_builder", sign)
        self.assertIn("${{ github.run_id }}-${{ github.run_attempt }}", sign)
        for forbidden in ("HYDRACACHE_074_AUTH_SIGNING_KEY_HEX", "self-hosted", "sudo", "--run ", "workflow_call:", "  push:"):
            self.assertNotIn(forbidden, workflow)
        self.assertEqual(workflow.count("overwrite: false"), 2)
        self.assertEqual(workflow.count("check-protection"), 2)

    def test_signing_environment_requires_human_review_and_exact_branch(self):
        environment = {"name": "performance-diagnostic-builder-074",
                       "can_admins_bypass": False,
                       "deployment_branch_policy": {"protected_branches": False, "custom_branch_policies": True},
                       "protection_rules": [{"type": "required_reviewers", "reviewers": [{"type": "User", "reviewer": {"id": 1}}]}]}
        branches = {"total_count": 1, "branch_policies": [{"name": "feat/0.74-resp-native-throughput", "type": "branch"}]}
        builder.check_protection(environment, branches)
        for changed in ({}, {**environment, "name": "performance-reference-074"},
                        {**environment, "can_admins_bypass": True},
                        {k: v for k, v in environment.items() if k != "can_admins_bypass"},
                        {**environment, "can_admins_bypass": None},
                        {**environment, "can_admins_bypass": 0},
                        {**environment, "protection_rules": []}, {**environment, "deployment_branch_policy": None}):
            with self.assertRaises(ValueError):
                builder.check_protection(changed, branches)
        for changed in ({}, {**branches, "total_count": 2},
                        {"total_count": 1, "branch_policies": [{"name": "*", "type": "branch"}]},
                        {"total_count": 1, "branch_policies": [{"name": "feat/0.74-resp-native-throughput", "type": "tag"}]}):
            with self.assertRaises(ValueError):
                builder.check_protection(environment, changed)

    def test_coordinator_fixture_retains_observations_and_never_runs_workload(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            source = root / "source"
            source.mkdir()
            target = source / "tools/get-owner-scheduled-controls-074/target/x86_64-unknown-linux-gnu/release/timing-controls-074"
            def compile_fixture(command, **kwargs):
                self.assertEqual(command, builder.COMMAND)
                self.assertEqual(kwargs["cwd"], source)
                self.assertNotIn("BUILDER_SIGNING_KEY_HEX", kwargs["env"])
                target.parent.mkdir(parents=True)
                target.write_bytes(b"synthetic-not-runnable")
                kwargs["stdout"].write(b"synthetic cargo log\n")
                kwargs["stderr"].write(b"separate stderr\n")
            calls = []
            def query(command, **kwargs):
                calls.append(command)
                if command[0] == "rustc":
                    return b"rustc 1.94.0 (4a4ef493e 2026-03-02)\n"
                if command[0] == "cargo":
                    return b"cargo 1.94.0 (85eff7c80 2026-01-15)\n"
                self.assertEqual(command[1], "--validate")
                return b'{"valid":true,"fixture_started":false,"admission_allowed":false}\n'
            output = root / "bundle"
            with patch.object(builder, "source_observation", return_value=(builder.SOURCE, builder.TREE)), \
                 patch.object(builder, "checked_inputs", return_value={"direct.json": b"fixed"}), \
                 patch.object(builder.subprocess, "run", side_effect=compile_fixture), \
                 patch.object(builder.subprocess, "check_output", side_effect=query):
                builder.build(source, root, output)
            observed = json.loads((output / "observation.json").read_bytes())
            self.assertEqual(observed["source_commit_before"], builder.SOURCE)
            self.assertEqual(observed["source_tree_after"], builder.TREE)
            self.assertEqual(observed["build_command"], builder.COMMAND)
            self.assertEqual((output / "compiler-stderr.log").read_bytes(), b"separate stderr\n")
            self.assertEqual(len([c for c in calls if c[1] == "--validate"]), 4)
            with patch.object(builder, "source_observation", return_value=(builder.SOURCE, builder.TREE)), \
                 patch.object(builder, "checked_inputs", return_value={}), self.assertRaises(ValueError):
                builder.build(source, root, root / "other")
            self.assertFalse((root / "other").exists())


if __name__ == "__main__":
    unittest.main()
