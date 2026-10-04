import copy
import hashlib
import importlib.util
import json
import pathlib
import tempfile
import tomllib
import unittest


SCRIPT = pathlib.Path(__file__).with_name("performance_long_run_074.py")
SCHEMA = (
    SCRIPT.parents[2]
    / "docs"
    / "testing"
    / "performance"
    / "0.74"
    / "schemas"
    / "campaign-start-manifest.schema.json"
)
CONTRACT = SCHEMA.parent.parent / "long-run-controller-resilience-contract.toml"
SPEC = importlib.util.spec_from_file_location("performance_long_run_074", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def inputs() -> dict:
    return {
        "schema_version": 1,
        "repository_id": 123,
        "authorization_identity": "protected-performance-074",
        "contract_sha256": "a" * 64,
        "tooling_sha": "b" * 40,
        "i74_source_sha": "c" * 40,
        "c74_source_sha": "d" * 40,
        "i74_tree_sha": "1" * 40,
        "c74_tree_sha": "2" * 40,
        "i74_cargo_lock_sha256": "3" * 64,
        "c74_cargo_lock_sha256": "4" * 64,
        "i74_dirty": False,
        "c74_dirty": False,
        "scenario_sha256": "e" * 64,
        "workload_sha256": "5" * 64,
        "offered_load_sha256": "6" * 64,
        "estimator_sha256": "7" * 64,
        "thresholds_sha256": "8" * 64,
        "host_receipt_sha256": "f" * 64,
        "lease_id": "123e4567-e89b-42d3-a456-426614174000",
        "random_nonce_hex": "1" * 64,
        "machine_id": "machine-a",
        "boot_id": "boot-a",
        "mount_identity": "dev=1;opts=rw",
        "isolated_cpuset": "2-7",
        "housekeeping_cpuset": "0-1",
        "seed": 740074,
        "checkpoint_cadence_seconds": 30,
        "progress_warning_gap_seconds": 90,
        "progress_rejection_gap_seconds": 180,
        "diagnostic_grace_seconds": 30,
        "product_lease_deadline_unix_seconds": 2_000_000_000,
        "maximum_campaign_bytes": 21_474_836_480,
        "maximum_campaign_files": 20_000,
        "installed_binaries": [
            {"role": "i74", "path": "/opt/hydracache-performance/0.74/i74/hydracache", "sha256": "9" * 64, "size": 1, "inode": 2, "device": 3, "uid": 1001, "gid": 1001, "mode": 0o555},
            {"role": "c74", "path": "/opt/hydracache-performance/0.74/c74/hydracache", "sha256": "a" * 64, "size": 1, "inode": 4, "device": 3, "uid": 1001, "gid": 1001, "mode": 0o555},
        ],
        "argv_templates": {
            "i74": ["/opt/hydracache-performance/0.74/i74/hydracache", "--role", "i74"],
            "c74": ["/opt/hydracache-performance/0.74/c74/hydracache", "--role", "c74"],
        },
        "command_environment_sha256": "b" * 64,
        "role_order": ["i74", "c74"],
        "phase_durations_seconds": {"warmup": 60, "measured": 300, "drain": 30, "durable_companion": 30, "post_work_idle": 60, "reconciliation": 30},
        "output_limits": {"stdout_bytes": 1048576, "stderr_bytes": 1048576, "diagnostic_bytes": 1048576, "final_artifact_bytes": 1073741824, "files": 2000},
        "expected_output_schema_sha256s": {"checkpoint": "c" * 64, "measurement": "d" * 64, "reconciliation": "e" * 64, "packet_manifest": "f" * 64},
        "required_final_guards": ["semantic", "native-non-regression", "retention"],
        "secret_identifiers": ["github-environment-key-v1"],
    }


class PerformanceLongRun074Tests(unittest.TestCase):
    def test_campaign_identity_is_deterministic_and_nonce_is_not_retained(self) -> None:
        value = inputs()
        manifest = MODULE.build_manifest(value)
        self.assertEqual(manifest["campaign_id"], MODULE.campaign_id(value))
        self.assertNotIn("random_nonce_hex", manifest)
        self.assertEqual(
            manifest["nonce_sha256"],
            MODULE.digest_bytes(bytes.fromhex(value["random_nonce_hex"])),
        )
        changed = copy.deepcopy(value)
        changed["c74_source_sha"] = "1" * 40
        self.assertNotEqual(MODULE.campaign_id(value), MODULE.campaign_id(changed))

    def test_unknown_float_secret_and_relaxed_deadline_fields_fail_closed(self) -> None:
        value = inputs()
        value["token_value"] = "forbidden"
        value["seed"] = 1.5
        value["progress_rejection_gap_seconds"] = 181
        value["installed_binaries"][0]["path"] = "/tmp/untrusted"
        problems = MODULE.validate_inputs(value)
        self.assertTrue(any("unknown" in problem for problem in problems))
        self.assertTrue(any("floating-point" in problem for problem in problems))
        self.assertTrue(any("secret-bearing" in problem for problem in problems))
        self.assertTrue(any("must remain 180" in problem for problem in problems))
        self.assertTrue(any("fixed 0.74 root" in problem for problem in problems))

    def test_output_is_create_new_and_digest_matches_canonical_manifest(self) -> None:
        manifest = MODULE.build_manifest(inputs())
        with tempfile.TemporaryDirectory() as temporary:
            output = pathlib.Path(temporary) / "campaign"
            path, digest = MODULE.write_manifest(output, manifest)
            encoded = path.read_bytes()
            self.assertEqual(encoded, MODULE.canonical_json(manifest) + b"\n")
            self.assertEqual(digest, MODULE.digest_bytes(encoded[:-1]))
            self.assertEqual(
                (output / "campaign-start.sha256").read_text(encoding="ascii"),
                digest + "\n",
            )
            with self.assertRaises(FileExistsError):
                MODULE.write_manifest(output, manifest)

    def test_json_round_trip_does_not_add_execution_fields(self) -> None:
        manifest = MODULE.build_manifest(inputs())
        round_trip = json.loads(MODULE.canonical_json(manifest))
        self.assertNotIn("command", round_trip)
        self.assertNotIn("environment", round_trip)
        self.assertEqual(round_trip["state"], "PREPARED")

    def test_nested_schema_argv_dirty_and_output_limits_fail_closed(self) -> None:
        value = inputs()
        value["output_limits"]["unknown"] = 1
        value["argv_templates"]["c74"][0] = "/tmp/unbound"
        value["i74_dirty"] = True
        value["output_limits"]["final_artifact_bytes"] = value["maximum_campaign_bytes"] + 1
        problems = MODULE.validate_inputs(value)
        self.assertTrue(any("output_limits fields differ" in problem for problem in problems))
        self.assertTrue(any("argv_templates.c74" in problem for problem in problems))
        self.assertTrue(any("dirty=false" in problem for problem in problems))
        self.assertTrue(any("exceeds the campaign byte limit" in problem for problem in problems))

    def test_checked_in_schema_has_the_exact_built_manifest_field_set(self) -> None:
        schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
        manifest_fields = set(MODULE.build_manifest(inputs()))
        self.assertEqual(set(schema["required"]), manifest_fields)
        self.assertEqual(set(schema["properties"]), manifest_fields)
        self.assertFalse(schema["additionalProperties"])
        contract = tomllib.loads(CONTRACT.read_text(encoding="utf-8"))
        self.assertEqual(
            contract["local_implementation"]["start_manifest_schema_sha256"],
            hashlib.sha256(SCHEMA.read_bytes()).hexdigest(),
        )

    def test_malformed_nested_types_are_reported_without_validator_exceptions(self) -> None:
        value = inputs()
        value["installed_binaries"] = 42
        value["required_final_guards"] = [{}]
        problems = MODULE.validate_inputs(value)
        self.assertTrue(any("installed_binaries" in problem for problem in problems))
        self.assertTrue(any("required_final_guards" in problem for problem in problems))


if __name__ == "__main__":
    unittest.main()
