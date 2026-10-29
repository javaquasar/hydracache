"""Offline mutation controls; only owned temporary files are changed."""
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import reconcile as subject


class ReconciliationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.facts = subject.analyze()

    def test_all_eleven_seals_and_491_members(self):
        self.assertEqual([r["members"] for r in self.facts["input_packets"]], [65,18,51,52,23,40,54,45,53,41,49])
        self.assertEqual(len(self.facts["sources"]), 19)
        self.assertEqual(len(self.facts["reader_methods"]), 6)

    def test_five_retained_leaves_and_eight_distinct_unresolved_owners(self):
        leaves = self.facts["leaves"]
        self.assertEqual(sum(r["disposition"] == "allocation-only-retained" for r in leaves), 5)
        self.assertEqual(len(leaves), 13)
        self.assertTrue(all(r["exclusive_leaf_time_measured"] is False for r in leaves))
        self.assertEqual(len(self.facts["allocation_cohorts"]), 5)
        self.assertFalse(any(key in self.facts for key in ("total_allocations_saved", "total_bytes_saved", "speedup")))

    def test_exact_separate_corpora_not_a_combined_workload(self):
        expected = [(2304,115200,0),(1792,237568,93184),(2304,64768,26368),
                    (5376,1274880,1221632),(7936,1798144,627712)]
        for cohort, (count,before,after) in zip(self.facts["allocation_cohorts"], expected):
            for lane in cohort["lanes"]:
                self.assertEqual(lane["constructions"], count)
                self.assertEqual(lane["counters"]["requested_bytes"], dict(reference=before,candidate=after))
        # The earlier Groups summary did not include allocation counts; do not invent zero.
        self.assertNotIn("allocations", self.facts["allocation_cohorts"][1]["lanes"][0]["counters"])

    def test_first_refusals_and_original_nested_wall_preserved(self):
        lanes = self.facts["old_reserved_live_lanes"]
        self.assertEqual([r["first_original_refusal"]["age_ns"] for r in lanes], [10042390247,10010835739])
        self.assertEqual([r["reserved_live"]["wall_ns"] for r in lanes], [2808784775,2941286486])
        self.assertEqual(self.facts["original_windows_seconds"], [300,60,10])
        self.assertNotEqual(self.facts["measured_source"], self.facts["audited_runtime_source"])
        self.assertTrue(all(self.facts["interpretation"].values()))
        for field in ("runtime_changed","observer_changed","clocks_changed","services_changed",
                      "original_material_gate_passed","cleanup_eof_fence_proven","original_delivery_proven",
                      "all_production_writers_proven","full_sigkill_matrix","lost_ack_matrix",
                      "reconnect_gate","full_path_speedup_proven","server","product"):
            self.assertIs(self.facts[field], False)
        self.assertEqual((self.facts["new_profile_invocations"],self.facts["new_measurements"]), (0,0))

    def test_cohort_source_count_lane_windows_refusals_promotions_refuse(self):
        spec = subject.SPECS[-1]
        original = subject.load(subject.ROOT / subject.BASE / spec[1] / "summary.json")
        changes = [lambda r:r.update(candidate_source="other"), lambda r:r.update(baseline_source="other"),
                   lambda r:r.update(original_windows_seconds=[300,60,11]),
                   lambda r:r.update(first_refusal_ages_ns=[0,0]),
                   lambda r:r.update(timing_improvement_proven=True), lambda r:r.update(server=True),
                   lambda r:r.update(product=True), lambda r:r["lanes"].reverse(),
                   lambda r:r["lanes"][0].update(main_constructions=1),
                   lambda r:r["lanes"][0].update(reference_allocations=True),
                   lambda r:r["lanes"][0].update(candidate_reallocations=-1),
                   lambda r:r["lanes"][0].update(candidate_requested_bytes=99999999),
                   lambda r:r["lanes"][0].update(candidate_requested_bytes=1)]
        for change in changes:
            row = copy.deepcopy(original)
            change(row)
            with self.subTest(change=changes.index(change)), self.assertRaises(ValueError):
                subject.cohort(row,spec)

    def test_missing_duplicate_current_witnesses_refuse(self):
        old = subject.load(subject.ROOT / subject.BASE / subject.AUDIT / "facts.json")
        _, texts = subject.sources(subject.ROOT,old)
        name, fragment = subject.REPLACED["collector-peer-growth"]
        for replacement in ("",fragment + "\n" + fragment):
            broken = dict(texts, **{name:replacement})
            with self.assertRaisesRegex(ValueError,"source witness"):
                subject.leaf_rows(old,broken)

    def test_source_hash_drift_refuses_except_line_endings(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            base=root / "tools/long-run-supervisor-074/src"
            base.mkdir(parents=True)
            file=base / "one.rs"
            pin=hashlib.sha256(b"fresh();\n").hexdigest()
            with patch.dict(subject.UPDATES,{},clear=True):
                file.write_bytes(b"fresh();\r\n")
                self.assertEqual(subject.sources(root,dict(sources={"one.rs":pin}))[1]["one.rs"],"fresh();\n")
                file.write_bytes(b"cached();\n")
                with self.assertRaisesRegex(ValueError,"Source drift"):
                    subject.sources(root,dict(sources={"one.rs":pin}))

    def packet(self, root, names=("one.txt",), size=5):
        (root / "one.txt").write_bytes(b"first")
        rows=[dict(path=name,bytes=size,sha256=subject.sha(root / "one.txt")) for name in names]
        (root / "inventory.json").write_text(json.dumps(dict(files=rows)),encoding="utf-8")
        return subject.sha(root / "inventory.json")

    def test_raw_inventory_member_extra_and_bool_size_refuse(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            pin=self.packet(root)
            self.assertEqual(subject.verify_packet(root,pin)["members"],1)
            with self.assertRaises(ValueError): subject.verify_packet(root,"0"*64)
            (root / "one.txt").write_bytes(b"later")
            with self.assertRaises(ValueError): subject.verify_packet(root,pin)
            pin=self.packet(root)
            (root / "extra").write_bytes(b"unlisted")
            with self.assertRaises(ValueError): subject.verify_packet(root,pin)
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory)
            pin=self.packet(root,size=True)
            with self.assertRaises(ValueError): subject.verify_packet(root,pin)

    def test_unsafe_duplicate_member_paths_refuse(self):
        for names in (("../one.txt",),("/one.txt",),("C:/one.txt",),("one\\txt",),
                      ("./one.txt",),("inventory.json",),("one.txt","one.txt")):
            with tempfile.TemporaryDirectory() as directory:
                root=Path(directory)
                pin=self.packet(root,names)
                with self.subTest(names=names),self.assertRaises(ValueError): subject.verify_packet(root,pin)

    def test_linked_input_refuses_before_read(self):
        with tempfile.TemporaryDirectory() as directory:
            with patch.object(Path,"is_symlink",return_value=True):
                with self.assertRaisesRegex(ValueError,"Linked input"):
                    subject.verify_packet(Path(directory),"0"*64)

    def test_old_reader_method_drift_refuses(self):
        real=subject.canonical
        with patch.object(subject,"canonical",side_effect=lambda p: "changed" if Path(p).parts[-3:] == ("tools","offline-reserved-leaf-audit-074","audit.py") else real(p)):
            with self.assertRaisesRegex(ValueError,"Reader method drift"):
                subject.methods(subject.ROOT)

    def test_no_output_into_old_packet_or_runtime(self):
        subject.require_output(subject.ROOT / subject.OUTPUT / "facts.json")
        for path in (subject.ROOT / subject.BASE / subject.AUDIT / "facts.json",
                     subject.ROOT / "tools/long-run-supervisor-074/src/one.rs",
                     subject.ROOT / subject.OUTPUT / "inventory.json"):
            with self.subTest(path=path),self.assertRaises(ValueError): subject.require_output(path)


if __name__ == "__main__":
    unittest.main(verbosity=2)
