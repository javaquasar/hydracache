"""Local consistency mutations; these are not process fault or trust proofs."""
import copy
import json
import unittest
import verify_packet as v


class PacketTests(unittest.TestCase):
    def setUp(self):
        self.manifest = copy.deepcopy(v.load_manifest())
        self.logs = {p.name: p.read_bytes() for p in v.PACKET.glob("*.log")}

    def refuses(self):
        with self.assertRaises(AssertionError):
            v.verify_evidence(self.manifest, self.logs)

    def test_positive(self):
        v.verify_evidence(self.manifest, self.logs)

    def test_source_pin(self):
        self.manifest["source_commit"] = "0" * 40
        self.refuses()

    def test_tree_pin(self):
        self.manifest["source_tree"] = "0" * 40
        self.refuses()

    def test_preregistration_pin(self):
        self.manifest["preregistration"] = "0" * 40
        self.refuses()

    def test_every_closed_claim(self):
        for key in v.CLOSED:
            with self.subTest(key=key):
                self.manifest["closed_claims"][key] = True
                self.refuses()
                self.manifest["closed_claims"][key] = False

    def test_unknown_closed_claim(self):
        self.manifest["closed_claims"]["invented_admission"] = False
        self.refuses()

    def test_scoped_result_type(self):
        self.manifest["scoped_final_checks_passed"] = 1
        self.refuses()

    def test_frozen_input(self):
        self.manifest["frozen_inputs"]["Cargo.lock"] = "0" * 64
        self.refuses()

    def test_implementation_set(self):
        self.manifest["implementation_files"].pop(next(iter(v.IMPLEMENTATION)))
        self.refuses()

    def test_oracle_set(self):
        self.manifest["unchanged_sources"].pop(next(iter(v.ORACLES)))
        self.refuses()

    def test_raw_capture_changed(self):
        self.logs["current-final.log"] += b"changed\n"
        self.refuses()

    def test_baseline_failure_cannot_disappear(self):
        self.logs.pop("baseline-old-root-failed.log")
        self.manifest["captures"].pop("baseline-old-root-failed.log")
        self.refuses()

    def test_msrv_failure_cannot_be_reported_as_success(self):
        self.manifest["captures"]["msrv-extra-libm-failed.log"]["exit_code"] = 0
        self.refuses()

    def test_exit_code_boolean_alias(self):
        self.manifest["captures"]["current-final.log"]["exit_code"] = False
        self.refuses()

    def test_duplicate_json_key(self):
        with self.assertRaises(AssertionError):
            json.loads('{"schema_version":1,"schema_version":2}', object_pairs_hook=v.unique)


if __name__ == "__main__":
    unittest.main()
