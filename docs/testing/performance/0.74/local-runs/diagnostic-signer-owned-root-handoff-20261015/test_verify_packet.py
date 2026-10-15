"""Mutations of unsigned consistency rules, not execution authenticity."""
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

    def test_closed_claims_and_numeric_substitutions_refuse(self):
        for key in self.data["closed_claims"]:
            for value in [True, 0, "false", None]:
                data = copy.deepcopy(self.data)
                data["closed_claims"][key] = value
                with self.assertRaises(ValueError): v.manifest(json.dumps(data).encode())

    def test_source_preregistration_and_bound_changes_refuse(self):
        for key in ["source_commit", "source_tree", "baseline", "preregistration", "amendment", "bounds", "msrv_lint_executed"]:
            data = copy.deepcopy(self.data)
            data.pop(key)
            with self.assertRaises(ValueError): v.manifest(json.dumps(data).encode())

    def test_capture_bytes_results_and_markers(self):
        for name, expected in self.data["captures"].items():
            raw = (v.PACKET / name).read_bytes()
            v.capture(name, raw, expected)
            with self.assertRaises(ValueError): v.capture(name, raw + b"x", expected)

    def test_msrv_refusal_cannot_be_current_success(self):
        raw = (v.PACKET / "msrv.log").read_bytes()
        with self.assertRaises(ValueError): v.capture("current-forged.log", raw, self.data["captures"]["msrv.log"])

    def test_lost_ack_cannot_be_promoted_even_after_rehash(self):
        raw = (v.PACKET / "current-1.log").read_bytes().replace(b"mode=no-ack attempted=1 sends=1 success_claim=false", b"mode=no-ack attempted=1 sends=1 ack_close_exit=true")
        expected = copy.deepcopy(self.data["captures"]["current-1.log"])
        expected.update(sha256=v.digest(raw), bytes=len(raw), required_markers=[])
        with self.assertRaises(ValueError): v.capture("current-1.log", raw, expected)

    def test_failed_send_cannot_be_relabelled_unattempted(self):
        raw = (v.PACKET / "current-1.log").read_bytes().replace(b"attempted=1 sends=0", b"attempted=0 sends=0")
        expected = copy.deepcopy(self.data["captures"]["current-1.log"])
        expected.update(sha256=v.digest(raw), bytes=len(raw), required_markers=[])
        with self.assertRaises(ValueError): v.capture("current-1.log", raw, expected)

    def test_incomplete_and_changed_counts_refuse(self):
        raw = (v.PACKET / "current-1.log").read_bytes()
        for key, value in [("capture_complete", False), ("results", [])]:
            expected = copy.deepcopy(self.data["captures"]["current-1.log"])
            expected[key] = value
            with self.assertRaises(ValueError): v.capture("current-1.log", raw, expected)

    def test_original_function_comparison_detects_raw_child_change(self):
        before = "\n".join("fn " + fn + "() { old(); }" for fn in ["raw_child", "wait", "enter", "first_exec", "advance"])
        v.old_owner_unchanged(before, before)
        with self.assertRaises(ValueError): v.old_owner_unchanged(before, before.replace("raw_child() { old();", "raw_child() { changed();"))

    def test_installer_recomposition_rejects_changed_core_or_exercise(self):
        prefix = "\n".join("fn " + fn + "() { old(); }" for fn in ["install", "entry", "accept", "decode", "memory"])
        before = prefix + "fn child() { setup(); exercise(); }"
        after = prefix + "fn install_child() { setup(); } fn child() { install_child(); exercise(); }"
        v.installer_unchanged(before, after)
        for bad in [after.replace("setup();", "changed();"), after.replace("exercise();", "changed();")]:
            with self.assertRaises(ValueError): v.installer_unchanged(before, bad)

if __name__ == "__main__":
    unittest.main()
