"""Pure/offline controls; all mutations use synthetic data or owned temp files."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import audit as subject


class AuditTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = subject.analyze()
        cls.rows = {lane: subject.rounds.prior.rows(subject.ROOT / subject.BASE /
            subject.rounds.PACKET / f"{lane}.log", "HC074-HELD-PHASE ") for lane in ("194", "188")}

    def test_all_four_original_seals_and_sources_are_checked(self):
        self.assertEqual([p["members"] for p in self.facts["input_packets"]], [65,18,51,52])
        self.assertEqual(len(self.facts["sources"]), 17)
        self.assertEqual(len(self.facts["leaves"]), 13)

    def test_exact_reserved_step_totals_not_the_all_step_totals(self):
        for lane, calls, wall, kernel, rest in [
            ("194",56,2808784775,1745205141,1063579634),
            ("188",59,2941286486,1828186002,1113100484),
        ]:
            row = next(r for r in self.facts["lanes"] if r["lane"] == lane)["reserved_live"]
            self.assertEqual((row["calls"],row["completed"],row["wall_ns"],
                row["kernel_outside_children_total_ns"],row["other_wall_ns"]), (calls,calls,wall,kernel,rest))
            self.assertEqual(sum(n["outside_instrumented_children_ns"] for n in row["kernel"]),kernel)
            self.assertEqual(kernel+rest,wall)

    def test_first_refusals_and_closed_admission_remain(self):
        self.assertEqual([r["first_original_refusal"]["age_ns"] for r in self.facts["lanes"]], [10042390247,10010835739])
        self.assertEqual(self.facts["original_windows_seconds"],[300,60,10])
        self.assertEqual(self.facts["new_profile_invocations"],0)
        for key in ("runtime_changed","observer_changed","clocks_changed","services_changed",
                    "original_material_gate_passed","full_sigkill_matrix","server","product"):
            self.assertIs(self.facts[key],False)

    def test_scope_flags_prevent_a_leaf_time_or_savings_claim(self):
        for row in self.facts["leaves"]:
            self.assertIs(row["exclusive_leaf_cost_measured"],False)
        flags = self.facts["interpretation"]
        self.assertTrue(flags["kernel_live_auth_views_overlap"])
        self.assertTrue(flags["leaf_review_order_is_not_elapsed_rank"])
        self.assertFalse(flags["outside_children_wall_is_exclusive_cpu"])
        self.assertFalse(flags["post_candidate_timing_available"])
        self.assertTrue(flags["old_trace_counts_not_extrapolated_to_candidate_savings"])

    def test_missing_duplicate_source_witnesses_refuse(self):
        for text in ("", "one one"):
            with self.subTest(text=text), self.assertRaisesRegex(ValueError,"source witness"):
                subject.witness({"fixture.rs":text},"fixture.rs","one")
        self.assertEqual(subject.witness({"fixture.rs":"x\none\ny"},"fixture.rs","one")["line"],2)

    def test_source_drift_is_not_normalized_except_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            base = root / "tools/long-run-supervisor-074/src"
            base.mkdir(parents=True)
            file = base / "fixture.rs"
            pin = hashlib.sha256(b"fresh();\n").hexdigest()
            with patch.dict(subject.SOURCE_PINS,{"fixture.rs":pin},clear=True):
                file.write_bytes(b"fresh();\r\n")
                self.assertEqual(subject.source_text(root),{"fixture.rs":"fresh();\n"})
                for bad in (b"cached();\n",b" fresh();\n",b"fresh();\nfresh();\n"):
                    file.write_bytes(bad)
                    with self.subTest(bad=bad),self.assertRaisesRegex(ValueError,"Source drift"):
                        subject.source_text(root)

    def synthetic_packet(self, root, rows=None):
        file = root / "one.txt"
        file.write_bytes(b"first refusal\n")
        if rows is None:
            rows = [dict(path="one.txt",bytes=file.stat().st_size,sha256=subject.sha(file))]
        seal = root / "inventory.json"
        seal.write_text(json.dumps(dict(files=rows)),encoding="utf-8")
        return subject.sha(seal)

    def test_raw_seal_and_member_changes_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pin = self.synthetic_packet(root)
            self.assertEqual(subject.verify_packet(root,pin)["members"],1)
            with self.assertRaisesRegex(ValueError,"seal drift"):
                subject.verify_packet(root,"0"*64)
            (root / "one.txt").write_bytes(b"new result!!!\n")
            with self.assertRaisesRegex(ValueError,"Member drift"):
                subject.verify_packet(root,pin)

    def test_unlisted_evidence_refuses(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            pin = self.synthetic_packet(root)
            (root / "extra.txt").write_bytes(b"not observed")
            with self.assertRaisesRegex(ValueError,"membership drift"):
                subject.verify_packet(root,pin)

    def test_unsafe_duplicate_and_bool_size_members_refuse(self):
        for name in ("../one.txt","/one.txt","C:/one.txt","one\\txt","./one.txt","inventory.json"):
            with tempfile.TemporaryDirectory() as directory:
                root=Path(directory)
                pin=self.synthetic_packet(root,[dict(path=name,bytes=14,sha256="0"*64)])
                with self.subTest(name=name), self.assertRaisesRegex(ValueError,"Unsafe/duplicate"):
                    subject.verify_packet(root,pin)
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            pin=self.synthetic_packet(root)
            row=subject.load(root / "inventory.json")["files"][0]
            pin=self.synthetic_packet(root,[row,row])
            with self.assertRaisesRegex(ValueError,"Unsafe/duplicate"):
                subject.verify_packet(root,pin)
            pin=self.synthetic_packet(root,[dict(row,bytes=True)])
            with self.assertRaisesRegex(ValueError,"Member drift"):
                subject.verify_packet(root,pin)

    def test_wrong_phase_and_broken_prefix_are_not_repaired(self):
        for change in (lambda r:r.update(phase="not-material"),
                       lambda r:r["round_steps"][1].update(completed=0),
                       lambda r:r["round_steps"][1].update(wall_ns=True)):
            row=copy.deepcopy(self.rows["194"][-1])
            change(row)
            with self.assertRaises(ValueError):
                subject.reserved_step(row)

    def test_audit_cannot_publish_into_old_packet_or_runtime(self):
        subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        for path in (subject.ROOT / subject.BASE / subject.rounds.PACKET / "facts.json",
                     subject.ROOT / "tools/long-run-supervisor-074/src/overwrite.rs",
                     subject.ROOT / subject.OUTPUT / "inventory.json"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                subject.require_output(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
