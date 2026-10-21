"""Offline sealed-log attribution controls. No compiled fixture or WSL entry."""
import ast
import copy
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from unittest.mock import patch
import attribute_material_rounds as subject

ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "docs/testing/performance/0.74/local-runs" / subject.PACKET


class ArithmeticTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.rows = {lane:subject.prior.rows(PACKET / f"{lane}.log", "HC074-HELD-PHASE ")
                    for lane in ("194", "188")}

    def phase(self, mutate=lambda r:None, lane="194", index=-1):
        row = copy.deepcopy(self.rows[lane][index])
        mutate(row)
        return subject.phase_facts(row)

    def test_both_first_partial_rounds_and_disjoint_wall_partition(self):
        for lane, started, completed, wall in (("194",56,55,4418926816),("188",59,58,4653421864)):
            fact = self.phase(lane=lane)
            self.assertEqual((fact["rounds_started"],fact["rounds_completed"]),(started,completed))
            self.assertEqual(fact["wall_ns"],wall)
            self.assertEqual(sum(fact["partition"].values()),wall)
            self.assertEqual(fact["dominant_step"],"ReservedLive")
            self.assertEqual(fact["unfinished_steps"],[dict(step="Causal",calls=started,completed=completed)])
            self.assertFalse(fact["returned"])
            for row in fact["steps"]: self.assertEqual(row["kernel_ns"]+row["other_ns"],row["wall_ns"])

    def test_all_eleven_phases_reconcile_and_no_round_phase_has_no_dominant_step(self):
        for lane in self.rows:
            for index in range(11):
                fact=self.phase(lane=lane,index=index)
                self.assertEqual(sum(fact["partition"].values()),fact["wall_ns"])
            self.assertIsNone(self.phase(lane=lane,index=0)["dominant_step"])

    def test_inclusive_and_auth_views_are_not_added_to_wall_partition(self):
        fact=self.phase()
        self.assertGreater(sum(r["phase"]["inclusive_ns"] for r in fact["kernel"]),fact["wall_ns"])
        self.assertEqual(set(fact["partition"]),set(subject.PARTITION))
        for row in fact["kernel"]:
            for field in row["inside_steps"]:
                self.assertEqual(row["inside_steps"][field]+row["outside_steps"][field],row["phase"][field])
        for row in fact["authentication_overlapping_view"].values():
            self.assertEqual(row["inside_steps"]+row["outside_steps"],row["phase"])

    def test_kernel_phase_subset_underflow_never_clamped(self):
        for field in ("calls","inclusive_ns","outside_children_ns"):
            def mutate(row):
                row["kernel"][5][field]=sum(s["kernel"][5][field] for s in row["round_steps"])-1
            with self.subTest(field=field), self.assertRaises(ValueError): self.phase(mutate)

    def test_nonround_residual_never_assumed_to_be_effects_or_clamped(self):
        def mutate(row):
            row["wall_ns"]=sum(s["wall_ns"] for s in row["round_steps"])
            # Reach the residual guard rather than an earlier overlapping-live
            # wall guard. This is a synthetic falsification, not trace repair.
            for node in row["live"]: node["outside_children_ns"]=0
        with self.assertRaisesRegex(ValueError,"Negative unassigned"):
            self.phase(mutate)

    def test_auth_subset_and_invalid_numeric_types_refuse(self):
        with self.assertRaisesRegex(ValueError,"Negative authentication_calls"):
            self.phase(lambda row:row.update(authentication_calls=sum(s["authentication_calls"] for s in row["round_steps"])-1))
        for value in (-1,True,1.0):
            with self.subTest(value=value),self.assertRaises(ValueError):
                self.phase(lambda row:row.update(wall_ns=value))

    def test_root_edge_node_and_round_completion_mutations_refuse(self):
        for mutation in (
            lambda row:row["round_steps"][1]["roots"].__setitem__(0,1),
            lambda row:row["round_steps"][1]["edges"][0].update(calls=1),
            lambda row:row["round_steps"][1]["edges"][0].update(inclusive_ns=1),
            lambda row:row["round_steps"][1]["kernel"][0].update(calls=1),
            lambda row:row["round_steps"][3].update(completed=56),
            lambda row:row["round_steps"].reverse()):
            with self.subTest(mutation=mutation),self.assertRaises(ValueError): self.phase(mutation)

    def test_original_output_signature_is_not_renormalized(self):
        with self.assertRaises(ValueError):
            self.phase(lambda row:row.update(authentication_calls=781),index=2)

    def test_scope_cannot_be_changed(self):
        for flag in ("deadline_changed","guard_clock_added","server","product","healthy_gate","includes_child_cpu"):
            with self.subTest(flag=flag),self.assertRaises(ValueError):
                self.phase(lambda row:row.update({flag:True}))

    def test_optional_cpu_stays_optional_and_is_not_wall_time(self):
        self.assertIsNone(self.phase(lambda row:row.update(process_cpu_ns=None))["parent_process_cpu_ns"])
        fact=self.phase()
        self.assertGreater(fact["parent_process_cpu_ns"],fact["wall_ns"])


class PacketTests(unittest.TestCase):
    def test_pinned_whole_packet_keeps_both_first_refusals_and_no_readiness_claim(self):
        fact=subject.analyze(PACKET)
        self.assertEqual([r["first_original_refusal"]["age_ns"] for r in fact["lanes"]],[10042390247,10010835739])
        self.assertEqual(fact["new_profile_invocations"],0)
        self.assertEqual(fact["input_members_verified"],65)
        for flag in ("runtime_changed","observer_changed","original_material_gate_passed","full_sigkill_matrix","native_gate_open","original_send","server","product"):
            self.assertIs(fact[flag],False)
        self.assertFalse(fact["interpretation"]["stage_durations_reconstructed"])

    def test_member_and_inventory_drift_and_extra_member_refuse(self):
        with tempfile.TemporaryDirectory() as temporary:
            packet=Path(temporary)/"packet";shutil.copytree(PACKET,packet)
            subject.verify_inventory(packet)
            extra=packet/"foreign.txt";extra.write_text("offline mutation")
            with self.assertRaisesRegex(ValueError,"Unexpected/missing"):subject.verify_inventory(packet)
            extra.unlink()
            raw=(packet/"194.log").read_bytes();(packet/"194.log").write_bytes(raw+b"drift")
            with self.assertRaisesRegex(ValueError,"member drift"):subject.verify_inventory(packet)
            (packet/"194.log").write_bytes(raw)
            with patch.object(subject,"INVENTORY","0"*64),self.assertRaisesRegex(ValueError,"pin drift"):
                subject.verify_inventory(packet)

    def test_unsafe_duplicate_inventory_paths_refuse_even_with_controlled_new_pin(self):
        with tempfile.TemporaryDirectory() as temporary:
            packet=Path(temporary)/"packet";shutil.copytree(PACKET,packet)
            original=subject.prior.load(packet/"inventory.json")
            for name in ("../escape","/absolute","C:/absolute","checks\\escape","./194.log",original["files"][1]["path"]):
                data=copy.deepcopy(original);data["files"][0]["path"]=name
                (packet/"inventory.json").write_text(json.dumps(data))
                with self.subTest(name=name),patch.object(subject,"INVENTORY",subject.prior.sha(packet/"inventory.json")),self.assertRaises(ValueError):
                    subject.verify_inventory(packet)

    def test_publish_exclusive_and_verify_never_replace_derived_evidence(self):
        with tempfile.TemporaryDirectory() as temporary:
            output=Path(temporary)/"facts.json"
            subject.prior.publish(output,{"first":101})
            with self.assertRaises(FileExistsError):subject.prior.publish(output,{"first":0})
            with self.assertRaises(ValueError):subject.prior.publish(output,{"first":0},verify=True)
            self.assertEqual(subject.prior.load(output),{"first":101})

    def test_analyzer_has_no_process_network_clock_service_or_runtime_entry(self):
        tree=ast.parse(Path(subject.__file__).read_text())
        imports={n.names[0].name for n in ast.walk(tree) if isinstance(n,ast.Import)}
        self.assertEqual(imports,{"argparse","attribute_partial","review_material_live"})
        forbidden={"capture","command","child_env","run_session","verify_preparation","Popen","run","system","sleep","clock_gettime","exec","eval"}
        for node in ast.walk(tree):
            if isinstance(node,ast.Call):
                name=node.func.attr if isinstance(node.func,ast.Attribute) else node.func.id if isinstance(node.func,ast.Name) else ""
                self.assertNotIn(name,forbidden)


if __name__=="__main__":unittest.main(verbosity=2)
