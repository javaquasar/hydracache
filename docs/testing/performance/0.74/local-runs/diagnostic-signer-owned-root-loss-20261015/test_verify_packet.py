"""Mutation checks for consistency rules, not authenticity tests."""
import copy
import json
import unittest
import verify_packet as v

class PacketTests(unittest.TestCase):
    def setUp(self):
        self.raw = (v.PACKET / "manifest.json").read_bytes()
        self.data = v.manifest(self.raw)

    def test_original_manifest(self):
        self.assertEqual(v.manifest(self.raw)["source_commit"], v.SOURCE)

    def test_every_closed_claim_promotion_and_type_change_refuses(self):
        for key in self.data["closed_claims"]:
            for value in [True, 0, "false", None]:
                data = copy.deepcopy(self.data)
                data["closed_claims"][key] = value
                with self.assertRaises(ValueError): v.manifest(json.dumps(data).encode())

    def test_identity_bound_and_missing_field_refuse(self):
        for key in ["source_commit", "source_tree", "baseline", "preregistration", "bounds", "msrv_lint_executed"]:
            data = copy.deepcopy(self.data)
            data.pop(key)
            with self.assertRaises(ValueError): v.manifest(json.dumps(data).encode())

    def test_captures_match_bytes_counts_and_markers(self):
        for name, expected in self.data["captures"].items():
            raw = (v.PACKET / name).read_bytes()
            v.capture(name, raw, expected)
            with self.assertRaises(ValueError): v.capture(name, raw + b"x", expected)

    def test_current_cannot_be_relabelled_refusal(self):
        raw = (v.PACKET / "msrv.log").read_bytes()
        expected = copy.deepcopy(self.data["captures"]["msrv.log"])
        with self.assertRaises(ValueError): v.capture("current-forged.log", raw, expected)

    def test_negative_cannot_be_promoted_after_rehash(self):
        raw = (v.PACKET / "current-1.log").read_bytes().replace(b"kind=negative automatic_death=false", b"kind=loss automatic_death=true")
        expected = copy.deepcopy(self.data["captures"]["current-1.log"])
        expected.update(sha256=v.digest(raw), bytes=len(raw), required_markers=[])
        with self.assertRaises(ValueError): v.capture("current-1.log", raw, expected)

    def test_incomplete_or_changed_results_refuse(self):
        raw = (v.PACKET / "current-1.log").read_bytes()
        for field, value in [("capture_complete", False), ("results", [])]:
            expected = copy.deepcopy(self.data["captures"]["current-1.log"])
            expected[field] = value
            with self.assertRaises(ValueError): v.capture("current-1.log", raw, expected)

    def test_ordinary_owner_comparison_rejects_worker_change(self):
        before = "raw_child()\n"
        additions = '#[path = "diagnostic_signer_owned_root_loss.rs"]\nmod loss;\npub(super) fn creator_loss(mode: &str) -> io::Result<()> {\n    loss::run(mode)\n}\n'
        v.old_owner_unchanged(before, additions + before)
        with self.assertRaises(ValueError): v.old_owner_unchanged(before, additions + "different_raw_child()\n")

if __name__ == "__main__":
    unittest.main()
