import copy
import importlib.util
import json
import pathlib
import tempfile
import unittest


SCRIPT = pathlib.Path(__file__).with_name("performance_long_run_074.py")
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
        "scenario_sha256": "e" * 64,
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
        problems = MODULE.validate_inputs(value)
        self.assertTrue(any("unknown" in problem for problem in problems))
        self.assertTrue(any("floating-point" in problem for problem in problems))
        self.assertTrue(any("secret-bearing" in problem for problem in problems))
        self.assertTrue(any("must remain 180" in problem for problem in problems))

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


if __name__ == "__main__":
    unittest.main()
