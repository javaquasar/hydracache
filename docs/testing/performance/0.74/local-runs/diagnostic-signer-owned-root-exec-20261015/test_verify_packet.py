"""Test the unsigned packet validator without changing retained observations."""
import copy
import json
from pathlib import Path
import unittest
import verify_packet as verify


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.manifest = copy.deepcopy(verify.EXPECTED)
        self.logs = {p.name: p.read_bytes() for p in Path(__file__).parent.glob("*.log")}

    def test_original_packet(self):
        verify.verify_evidence(self.manifest, self.logs)

    def rejected(self):
        with self.assertRaises(AssertionError):
            verify.verify_evidence(self.manifest, self.logs)

    def test_unknown_schema_and_source(self):
        for key in ["schema_version", "source_commit", "source_tree", "baseline", "preregistration"]:
            with self.subTest(key=key):
                old = self.manifest[key]
                self.manifest[key] = "unknown"
                self.rejected()
                self.manifest[key] = old

    def test_closed_claims_cannot_be_promoted_or_dropped(self):
        for key in self.manifest["closed_claims"]:
            with self.subTest(key=key):
                self.manifest["closed_claims"][key] = True
                self.rejected()
                self.manifest["closed_claims"][key] = False
        self.manifest["closed_claims"].pop("all_writers_revoked")
        self.rejected()

    def test_boolean_and_numeric_types_not_interchangeable(self):
        for value in [0, 0.0, None, "false"]:
            self.manifest["closed_claims"]["ship_allowed"] = value
            self.rejected()

    def test_bounds_cannot_be_widened(self):
        for key in self.manifest["bounds"]:
            self.manifest["bounds"][key] += 1
            self.rejected()
            self.manifest["bounds"][key] -= 1

    def test_source_and_unchanged_oracle_pins(self):
        for group in ["implementation_files", "unchanged_sources", "frozen_inputs"]:
            key = next(iter(self.manifest[group]))
            old = self.manifest[group][key]
            self.manifest[group][key] = "0" * 64
            self.rejected()
            self.manifest[group][key] = old

    def test_capture_mutation_and_truncation(self):
        for key in self.logs:
            with self.subTest(key=key):
                old = self.logs[key]
                self.logs[key] = old[:-1]
                self.rejected()
                self.logs[key] = old + b"forged success\n"
                self.rejected()
                self.logs[key] = old

    def test_missing_or_extra_capture(self):
        self.logs["invented.log"] = b"test result: ok. 1 passed; 0 failed;\n"
        self.rejected()
        del self.logs["invented.log"]
        self.logs.pop("overlapping-broad-timeouts.log")
        self.rejected()

    def test_timeout_cannot_be_relabelled_success(self):
        self.manifest["captures"]["overlapping-broad-timeouts.log"]["exit_code"] = 0
        self.rejected()

    def test_msrv_refusal_cannot_be_labelled_normal_runtime_support(self):
        self.manifest["captures"]["msrv-current.log"]["results"] = [["ok", 25, 0]]
        self.rejected()

    def test_unknown_field_and_duplicate_json_key(self):
        self.manifest["signer_admitted"] = True
        self.rejected()
        with self.assertRaises(AssertionError):
            json.loads('{"a":1,"a":2}', object_pairs_hook=verify.unique)


if __name__ == "__main__":
    unittest.main()
