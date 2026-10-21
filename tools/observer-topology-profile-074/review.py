"""Independent validation of synthetic observer counts and timing summaries."""
import argparse
import json
from pathlib import Path
from statistics import median
from topology import partial

ORDERS = (("inactive","kernel","rounds"),("rounds","kernel","inactive"),
          ("kernel","inactive","rounds"),("rounds","inactive","kernel"),
          ("kernel","rounds","inactive"),("inactive","rounds","kernel"))


def equal(a,b):
    # Unlike Python numeric equality, this refuses bool/int/float substitutions.
    return json.dumps(a,sort_keys=True,separators=(",",":")) == json.dumps(b,sort_keys=True,separators=(",",":"))


def graph(nodes, roots, edge_calls, edge_ns, expected):
    partial.require(len(nodes)==len(roots)==len(edge_calls)==len(edge_ns)==10,"Graph shape")
    partial.require(equal([n["calls"] for n in nodes],[n*46 for n in expected["calls"]]) and
                    equal(roots,[n*46 for n in expected["roots"]]) and
                    equal(edge_calls,[[n*46 for n in row] for row in expected["edges"]]),"Topology mismatch")
    for i,n in enumerate(nodes):
        for field in ("calls","inclusive_ns","outside_children_ns"):
            partial.number(n[field])
        partial.require(len(edge_ns[i])==10,"Edge shape")
        partial.require(n["inclusive_ns"]==n["outside_children_ns"]+sum(partial.number(t) for t in edge_ns[i]),"Time conservation")
        partial.require(n["calls"]==roots[i]+sum(row[i] for row in edge_calls),"Call conservation")


def validate(row, plan, source, case):
    partial.require(row["schema"]=="observer-topology-control-074-v1" and row["source"]==source and row["case"]==case,"Case identity")
    partial.require(equal(row["order"],list(ORDERS[case])) and row["seed"]==0x074c051e and row["rounds"]==46 and row["repeats"]==3,"Fixed schedule")
    partial.flags(row,true=("synthetic","canonical_order"),false=("original_temporal_order_known","admission_exercised","native_gate_open","guard_or_deadline_changed","server","product"))
    partial.require(type(row["live_guard_invocations"]) is int and row["live_guard_invocations"]==0,"Synthetic scope")
    partial.require(len(row["blocks"])==3,"Missing mode")
    total=dict(calls=[sum(g["calls"][i] for g in plan["steps"]) for i in range(10)],
               roots=[sum(g["roots"][i] for g in plan["steps"]) for i in range(10)],
               edges=[[sum(g["edges"][i][j] for g in plan["steps"]) for j in range(10)] for i in range(10)])
    checksum=None
    for name,block in zip(ORDERS[case],row["blocks"]):
        partial.require(block["mode"]==name and len(block["samples"])==3,"Mode/sample mismatch")
        for sample in block["samples"]:
            partial.require(equal(sample["calls"],[n*46 for n in total["calls"]]),"Walker count mismatch")
            checksum=sample["checksum"] if checksum is None else checksum
            partial.require(sample["checksum"]==checksum,"Body checksum mismatch")
            for stage in ("walk","aggregation","serialization"):
                partial.require(partial.number(sample[stage]["wall_ns"])>0,"Missing timing")
                partial.number(sample[stage]["parent_process_cpu_ns"])
            report=sample["report"]
            partial.flags(report,true=("synthetic",))
            encoded=json.dumps(report,sort_keys=True,separators=(",",":"),ensure_ascii=False).encode()
            partial.require(sample["report_bytes"]==len(encoded),"Serialization bytes differ")
            if name=="inactive":
                partial.require(report["kernel"]==[] and report["round_steps"]==[] and sample["observed_topology"] is None,"Inactive capture not inactive")
                continue
            proof=sample["observed_topology"]
            graph(report["kernel"],proof["roots"],proof["edge_calls"],proof["edge_inclusive_ns"],total)
            if name=="kernel":
                partial.require(report["round_steps"]==[],"Round capture leaked")
                continue
            partial.require(len(report["round_steps"])==8,"Eight round rows required")
            for index,part in enumerate(report["round_steps"]):
                partial.require(part["step"]==index and part["calls"]==part["completed"]==46,"Round completion mismatch")
                partial.flags(part,true=("overlapping_spans",),false=("guard_clock_added",))
                calls=[[0]*10 for _ in range(10)];times=[[0]*10 for _ in range(10)]
                seen=set()
                for edge in part["edges"]:
                    parent,child=edge["parent"],edge["child"]
                    partial.require(type(parent) is int and type(child) is int and 0<=parent<10 and 0<=child<10 and (parent,child) not in seen,"Invalid/duplicate edge")
                    seen.add((parent,child));calls[parent][child]=edge["calls"];times[parent][child]=edge["inclusive_ns"]
                graph(part["kernel"],part["roots"],calls,times,plan["steps"][index])
                partial.require(sum(n["outside_children_ns"] for n in part["kernel"])<=part["wall_ns"],"Step wall conservation")
                partial.require(all(n["calls"]==0 for n in part["live"]) and part["authentication_calls"]==part["signature_calls"]==part["worker_policy_calls"]==0,"Non-kernel observer became active")
    return checksum


def bounds(values):
    return dict(minimum=min(values),maximum=max(values))


def analyze(packet):
    result=partial.load(packet/"result.json")
    partial.require(result["complete"] is True and result["failure"] is None and len(result["cases"])==12,"Incomplete synthetic cohort")
    plan=partial.load(packet/"input.json")
    source=result["source"]
    before=partial.load(packet/"metadata.json");after=partial.load(packet/"postflight.json")
    partial.require(equal(before,after) and before["source"]==source,"Session context drift")
    summaries=[];checksum=None
    for lane in ("194","188"):
        cases=[]
        for case in range(6):
            path=packet/f"{lane}-{case}.log"
            receipt=partial.load(packet/f"{lane}-{case}.json")
            partial.require(receipt["source"]==source and receipt["lane"]==lane and receipt["case"]==case and receipt["exit"]==0 and receipt["log_sha256"]==partial.sha(path),"Receipt binding")
            row=partial.load(path)
            actual=validate(row,plan,source,case)
            checksum=actual if checksum is None else checksum
            partial.require(actual==checksum,"Cross-case/compiler checksum changed")
            cases.append(row)
        modes={}
        for name in ORDERS[0]:
            samples=[s for c in cases for b in c["blocks"] if b["mode"]==name for s in b["samples"]]
            modes[name]={stage:{metric:bounds([s[stage][metric] for s in samples]) for metric in ("wall_ns","parent_process_cpu_ns")} for stage in ("walk","aggregation","serialization")}
        deltas={}
        for label,first,second in (("kernel_minus_inactive","inactive","kernel"),("rounds_minus_kernel","kernel","rounds")):
            deltas[label]={}
            for metric in ("wall_ns","parent_process_cpu_ns"):
                values=[]
                for c in cases:
                    blocks={b["mode"]:b["samples"] for b in c["blocks"]}
                    values.append(median([s["walk"][metric] for s in blocks[second]])-median([s["walk"][metric] for s in blocks[first]]))
                deltas[label][metric]=dict(per_case_median_delta_ns=values,**bounds(values))
        summaries.append(dict(lane=lane,cases=6,samples_per_mode=18,modes=modes,deltas=deltas))
    return dict(schema="observer-topology-summary-074-v1",source=source,checksum=checksum,lanes=summaries,
                synthetic=True,original_order_known=False,live_guard_invocations=0,admission_exercised=False,
                full_observer_cost_measured=False,overhead_subtracted_from_live=False,
                native_gate_open=False,server=False,product=False)


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet",type=Path,required=True)
    parser.add_argument("--verify",action="store_true")
    args=parser.parse_args()
    partial.publish(args.packet/"summary.json",analyze(args.packet),args.verify)
    print("synthetic counts verified; no live overhead subtraction or admission")
