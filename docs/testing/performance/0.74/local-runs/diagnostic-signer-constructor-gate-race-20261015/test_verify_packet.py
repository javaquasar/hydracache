"""Local capture consistency mutations, not kernel or execution attestation."""
import copy
import json
import unittest
import verify_packet as v


class PacketTests(unittest.TestCase):
    def setUp(self):
        self.m = copy.deepcopy(v.load_manifest())
        self.logs = {p.name: p.read_bytes() for p in v.PACKET.glob("*.log")}

    def refuses(self):
        with self.assertRaises(AssertionError):
            v.verify_evidence(self.m, self.logs)

    def test_positive(self):
        v.verify_evidence(self.m, self.logs)

    def test_source_pin(self):
        self.m["source_commit"] = "0" * 40
        self.refuses()

    def test_tree_pin(self):
        self.m["source_tree"] = "0" * 40
        self.refuses()

    def test_instrumentation_pin(self):
        self.m["instrumentation_commit"] = "0" * 40
        self.refuses()

    def test_each_closed_claim(self):
        for key in v.CLOSED:
            with self.subTest(key=key):
                self.m["closed_claims"][key] = True
                self.refuses()
                self.m["closed_claims"][key] = False

    def test_timeout_not_widened(self):
        self.m["bounds"]["logical_stall_millis"] = 6000
        self.refuses()

    def test_report_descriptor_limit(self):
        self.m["bounds"]["report_descriptors"] = 6
        self.refuses()

    def test_setup_packet_count(self):
        self.m["bounds"]["max_received_packets"] = 11
        self.refuses()

    def test_red_regression_cannot_disappear(self):
        self.logs.pop("ordering-before.log")
        self.m["captures"].pop("ordering-before.log")
        self.refuses()

    def test_prior_negative_cannot_change(self):
        key = next(iter(v.PRIOR))
        self.m["prior_negative_evidence"][key] = "0" * 64
        self.refuses()

    def test_raw_final_capture(self):
        self.logs["broad-final.log"] += b"changed\n"
        self.refuses()

    def test_exit_code_boolean_alias(self):
        self.m["captures"]["broad-final.log"]["exit_code"] = False
        self.refuses()

    def test_unknown_capture(self):
        self.logs["unknown.log"] = b"invented"
        self.refuses()

    def test_scoped_success_type(self):
        self.m["scoped_final_checks_passed"] = 1
        self.refuses()

    def test_duplicate_json_key(self):
        with self.assertRaises(AssertionError):
            json.loads('{"source_commit":1,"source_commit":2}', object_pairs_hook=v.unique)


if __name__ == "__main__":
    unittest.main()
