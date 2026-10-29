import copy
import json
from pathlib import Path
import sys
import unittest

sys.path.insert(0,str(Path(__file__).resolve().parent))
import review
import topology


def fake(plan, case=0):
    calls=[sum(g["calls"][i] for g in plan["steps"])*46 for i in range(10)]
    roots=[sum(g["roots"][i] for g in plan["steps"])*46 for i in range(10)]
    edges=[[sum(g["edges"][i][j] for g in plan["steps"])*46 for j in range(10)] for i in range(10)]
    nodes=lambda c:[dict(calls=n,inclusive_ns=0,outside_children_ns=0) for n in c]
    blocks=[]
    for mode in review.ORDERS[case]:
        report=dict(kernel=[] if mode=="inactive" else nodes(calls),round_steps=[],synthetic=True)
        if mode=="rounds":
            for step,g in enumerate(plan["steps"]):
                part=dict(step=step,calls=46,completed=46,wall_ns=1,overlapping_spans=True,
                          guard_clock_added=False,kernel=nodes([n*46 for n in g["calls"]]),
                          roots=[n*46 for n in g["roots"]],edges=[],live=[dict(calls=0)]*6,
                          authentication_calls=0,signature_calls=0,worker_policy_calls=0)
                for i in range(10):
                    for j in range(10):
                        if g["edges"][i][j]:
                            part["edges"].append(dict(parent=i,child=j,calls=g["edges"][i][j]*46,inclusive_ns=0))
                report["round_steps"].append(part)
        sample=dict(walk=dict(wall_ns=1,parent_process_cpu_ns=1),aggregation=dict(wall_ns=1,parent_process_cpu_ns=1),
                    serialization=dict(wall_ns=1,parent_process_cpu_ns=1),calls=calls,checksum=74,report=report,
                    observed_topology=None if mode=="inactive" else dict(roots=roots,edge_calls=edges,edge_inclusive_ns=[[0]*10 for _ in range(10)]),
                    report_bytes=len(json.dumps(report,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()))
        blocks.append(dict(mode=mode,samples=[copy.deepcopy(sample) for _ in range(3)]))
    return dict(schema="observer-topology-control-074-v1",source="a"*40,case=case,order=list(review.ORDERS[case]),
                seed=0x074c051e,rounds=46,repeats=3,blocks=blocks,synthetic=True,canonical_order=True,
                original_temporal_order_known=False,live_guard_invocations=0,admission_exercised=False,
                native_gate_open=False,guard_or_deadline_changed=False,server=False,product=False)


class ReviewTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.plan=topology.export()

    def row(self,mutate=lambda r:None):
        row=fake(self.plan)
        mutate(row)
        return review.validate(row,self.plan,"a"*40,0)

    def test_all_orders_and_exact_graph(self):
        self.assertEqual(sum(sum(g["calls"]) for g in self.plan["steps"])*46,894608)
        for case in range(6):
            self.assertEqual(review.validate(fake(self.plan,case),self.plan,"a"*40,case),74)

    def test_walker_counts_and_checksum_mutations_refused(self):
        for change in (lambda r:r["blocks"][0]["samples"][0]["calls"].__setitem__(0,1),
                       lambda r:r["blocks"][0]["samples"][1].update(checksum=75)):
            with self.assertRaises(ValueError):self.row(change)

    def test_capture_roots_edges_and_time_mutations_refused(self):
        for change in (lambda r:r["blocks"][1]["samples"][0]["observed_topology"]["roots"].__setitem__(0,1),
                       lambda r:r["blocks"][1]["samples"][0]["observed_topology"]["edge_calls"][0].__setitem__(1,1),
                       lambda r:r["blocks"][1]["samples"][0]["observed_topology"]["edge_inclusive_ns"][0].__setitem__(1,1)):
            with self.assertRaises(ValueError):self.row(change)

    def test_missing_round_or_mode_refused(self):
        for change in (lambda r:r["blocks"][2]["samples"][0]["report"]["round_steps"].pop(),
                       lambda r:r["blocks"].pop(),lambda r:r["blocks"][1]["samples"].pop()):
            with self.assertRaises(ValueError):self.row(change)

    def test_scope_and_schedule_mutations_refused(self):
        for change in (lambda r:r.update(native_gate_open=True),lambda r:r.update(native_gate_open=0),
                       lambda r:r.update(original_temporal_order_known=True),
                       lambda r:r.update(order=list(reversed(r["order"]))),lambda r:r.update(rounds=47)):
            with self.assertRaises(ValueError):self.row(change)

    def test_negative_cpu_and_serialized_bytes_refused(self):
        for change in (lambda r:r["blocks"][0]["samples"][0]["walk"].update(parent_process_cpu_ns=-1),
                       lambda r:r["blocks"][0]["samples"][0].update(report_bytes=1)):
            with self.assertRaises(ValueError):self.row(change)

    def test_boolean_integer_equality_is_not_coerced(self):
        self.assertFalse(review.equal(False,0))
        self.assertFalse(review.equal(46,46.0))


if __name__=="__main__":unittest.main(verbosity=2)
