"""Falsifying controls on retained real synthetic output, without rerunning it."""
import copy
import json
from pathlib import Path
import sys

packet=Path(__file__).resolve().parent
root=packet.parents[5]
sys.path.insert(0,str(root/"tools/observer-topology-profile-074"))
import review

plan=review.partial.load(packet/"input.json")
row=review.partial.load(packet/"194-0.log")
source=review.partial.load(packet/"result.json")["source"]
review.validate(row,plan,source,0)
mutations={
    "walker-count":lambda r:r["blocks"][0]["samples"][0]["calls"].__setitem__(0,1),
    "root-count":lambda r:r["blocks"][1]["samples"][0]["observed_topology"]["roots"].__setitem__(0,1),
    "edge-time":lambda r:r["blocks"][1]["samples"][0]["observed_topology"]["edge_inclusive_ns"][0].__setitem__(1,1),
    "negative-cpu":lambda r:r["blocks"][0]["samples"][0]["walk"].update(parent_process_cpu_ns=-1),
    "invented-order":lambda r:r.update(original_temporal_order_known=True),
    "numeric-admission":lambda r:r.update(native_gate_open=0),
    "repeats":lambda r:r.update(repeats=2),
    "serialized-bytes":lambda r:r["blocks"][0]["samples"][0].update(report_bytes=1),
    "mode-order":lambda r:r.update(order=list(reversed(r["order"]))),
}
results=[]
for name,mutate in mutations.items():
    changed=copy.deepcopy(row);mutate(changed);rejected=False
    try:review.validate(changed,plan,source,0)
    except ValueError:rejected=True
    review.partial.require(rejected,f"Accepted mutation {name}")
    results.append(dict(mutation=name,refused=True))
with (packet/"audit.json").open("x",encoding="utf-8",newline="\n") as stream:
    json.dump(dict(mutations=results,server=False,product=False,native_gate_open=False),stream,indent=2)
    stream.write("\n")
