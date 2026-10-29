"""Actual retained trace controls, not synthetic timing observations."""
import copy
import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("fresh", Path(__file__).with_name("attribute_fresh.py"))
subject = importlib.util.module_from_spec(spec)
spec.loader.exec_module(subject)
ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "docs/testing/performance/0.74/local-runs" / subject.partial.PACKET


class FreshTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.raw = subject.partial.rows(PACKET / "194-Profile-0.log", "HC074-HELD-PHASE ")[2]["round_steps"][1]

    def test_actual_partition_reconciles(self):
        facts = subject.analyze(PACKET)
        for lane in facts["lanes"]:
            for phase in lane["phases"]:
                for step in phase["steps"]:
                    self.assertEqual(step["boot_inclusive_ns"], step["boot_outside_document_ns"]+step["boot_document_ns"])
                    self.assertEqual(step["kernel_partition_ns"], step["boot_inclusive_ns"]+step["namespace_ns"]+step["other_kernel_ns"])
                    self.assertEqual(step["wall_ns"], step["kernel_partition_ns"]+step["outside_kernel_ns"])
            live = lane["phases"][2]["steps"][1]
            self.assertEqual((live["boot_calls"],live["namespace_calls"],live["authority_calls"]),
                             (129904,259808,14812))

    def test_refusals_and_scope_preserved(self):
        facts = subject.analyze(PACKET)
        self.assertEqual([r["original_refusal"]["age_ns"] for r in facts["lanes"]], [10175057064,10006497560])
        self.assertEqual(facts["new_profile_invocations"],0)
        for key in ("runtime_changed","observer_changed","native_gate_open","server","product"):
            self.assertIs(facts[key],False)
        self.assertEqual(facts["interpretation"]["authority_kernel_intersection"],"unrecorded")
        self.assertIs(facts["interpretation"]["exclusive_cpu_known"],False)

    def test_changed_leaf_parent_even_with_conserved_counts_refused(self):
        raw = copy.deepcopy(self.raw)
        edge = next(e for e in raw["edges"] if e["child"] == 6)
        duration = edge["inclusive_ns"]
        raw["kernel"][4]["outside_children_ns"] += duration
        # Keep inclusive node totals; reparent Namespace below Boot instead of
        # Origin. The old reader conserves counts/wall but does not know this
        # invented call topology is incompatible with the actual fresh path.
        raw["kernel"][5]["outside_children_ns"] -= duration
        edge["parent"] = 5
        subject.partial.step_facts(raw,1)
        with self.assertRaisesRegex(ValueError,"parent changed"):
            subject.separate(raw,1)

    def test_new_authority_child_is_not_silently_excluded(self):
        raw = copy.deepcopy(self.raw)
        raw["live"][3]["outside_children_ns"] -= 1
        with self.assertRaisesRegex(ValueError,"Authority live children"):
            subject.separate(raw,1)

    def test_impossible_authority_wall_refused(self):
        raw = copy.deepcopy(self.raw)
        raw["live"][3].update(inclusive_ns=raw["wall_ns"]+1,outside_children_ns=raw["wall_ns"]+1)
        with self.assertRaisesRegex(ValueError,"Authority exceeds"):
            subject.separate(raw,1)

    def test_numeric_and_edge_corruption_refused(self):
        for value in (-1,True,1.5):
            raw = copy.deepcopy(self.raw)
            raw["kernel"][5]["calls"] = value
            with self.assertRaises(ValueError):
                subject.separate(raw,1)
        raw = copy.deepcopy(self.raw)
        raw["edges"].append(copy.deepcopy(raw["edges"][0]))
        with self.assertRaises(ValueError):
            subject.separate(raw,1)

    def test_empty_step_has_no_invented_cost(self):
        raw = copy.deepcopy(self.raw)
        raw.update(wall_ns=0,calls=0,completed=0,edges=[],roots=[0]*10,authentication_calls=0,
                   authentication_ns=0,signature_calls=0,signature_ns=0)
        for row in raw["kernel"]+raw["live"]:
            row.update(calls=0,inclusive_ns=0,outside_children_ns=0)
        facts = subject.separate(raw,1)
        self.assertTrue(all(v==0 for k,v in facts.items() if k!="step"))

    def test_zero_call_with_time_refused(self):
        raw = copy.deepcopy(self.raw)
        raw["kernel"][6]["calls"]=0
        with self.assertRaisesRegex(ValueError,"Time without calls"):
            subject.separate(raw,1)

    def test_exclusive_publish_and_cli_verification(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"facts.json"
            args=["attribute_fresh.py","--input",str(PACKET),"--output",str(path)]
            with patch("sys.argv",args):
                subject.main()
            with self.assertRaises(FileExistsError), patch("sys.argv",args):
                subject.main()
            with patch("sys.argv",args+["--verify"]):
                subject.main()
            changed=subject.partial.load(path)
            changed["native_gate_open"]=True
            with self.assertRaisesRegex(ValueError,"Derived facts changed"):
                subject.partial.publish(path,changed,True)

    def test_verifier_refuses_numeric_boolean_gate_substitution(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"facts.json"
            facts=subject.analyze(PACKET)
            facts["native_gate_open"]=0
            subject.partial.publish(path,facts)
            with self.assertRaisesRegex(ValueError,"Derived facts changed"), patch("sys.argv",
                    ["attribute_fresh.py","--input",str(PACKET),"--output",str(path),"--verify"]):
                subject.main()


if __name__ == "__main__":
    unittest.main(verbosity=2)
