"""Read-only review of a separate candidate-only operator correctness cohort."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/authority-operator-order-074"))
import review_operator as previous
need,exact,load,sha=previous.need,previous.exact,previous.load,previous.sha
canonical,publish,BASE=previous.canonical,previous.publish,previous.BASE
core=previous.core
OUTPUT=BASE/"authority-operator-targeted-20261022"
SOURCE="90f36fb7d794617ec3db3c4a0d0715fca7e380c8"
INPUTS=dict(previous.INPUTS,**{"authority-operator-order-20261022":"e596bab55acdaa07a77410b41a7fe6e4633dedd76a167e06db6de4db0d8148f4"})
TESTS=sorted("model::candidate_controls::"+name for name in previous.CANDIDATES)
FILTER=["model::candidate_controls::candidate_"]
LIST_ARGS=["--list"]+FILTER
CONTROL_ARGS=["--nocapture","--test-threads=1"]+FILTER
STEPS=["compiler","argv-compile","argv-list","argv-controls","listing","controls","check","clippy"]
PINS={
    "tools/authority-operator-targeted-074/check.ps1":"cf09ec2be481ebdf37f8e614919d2914715a8a1c8bb77099500b50b275bf79dc",
    "docs/testing/performance/0.74/authority-operator-targeted-plan-20261022.md":"2929361757bd5162d722fd75f6e65a674865e5170d86e27a7a2bd920bf67598b",
}


def source_pins(root):
    for name,pin in PINS.items():exact(hashlib.sha256(canonical(Path(root)/name).encode()).hexdigest(),pin,"Targeted source drift: "+name)
    previous.source_pins(root)
    for name in ("review_operator.py","test_review_operator.py"):
        path="tools/authority-operator-order-074/"+name
        exact(canonical(Path(root)/path),canonical(Path(root)/previous.OUTPUT/"method"/path),"Frozen operator reader changed")
    return PINS


def listing(text):
    exact(re.findall(r"(?m)^(\S+): test$",text),TESTS,"Exactly three listed candidate tests required")
    need("3 tests, 0 benchmarks" in text,"Listing cardinality required")
    need("HC074-" not in text and "test result:" not in text,"Listing executed test bodies")
    return TESTS


def controls(text,expected):
    rows={k:[] for k in ("STATE","FAULT","GATE","CUT","EXTERNAL")};clean=[]
    need("HC074-OWNED-" not in text and "HC074-OP-LEGACY" not in text,"Old controls must not execute")
    for line in text.splitlines():
        if "HC074-OP-" not in line:clean.append(line);continue
        prefix,payload=line.split("HC074-OP-",1);kind,payload=payload.split(" ",1)
        need(kind in rows,"Unknown candidate row");rows[kind].append(json.loads(payload));clean.append(prefix.rstrip())
    passed=re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$","\n".join(clean))
    exact(sorted(passed),TESTS,"Exact completed candidate membership required")
    need("test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out;" in text and "test result: FAILED" not in text,"Isolated summary required")
    exact(rows,expected,"Candidate rows differ from reconstructed frozen semantic oracle")
    exact({k:len(v) for k,v in rows.items()},dict(STATE=80,FAULT=3072,GATE=96,CUT=24,EXTERNAL=24),"Wrong case cardinality")
    return rows


def receipt(directory,lane):
    directory=Path(directory);row=load(directory/"result.json")
    exact(row["schema"],"authority-operator-targeted-first-v1","Wrong receipt schema")
    exact(row["lane"],lane,"Wrong compiler lane");exact(row["source"],SOURCE,"Wrong first source")
    exact(row["failure"],None,"First targeted run refused")
    for name,value in (("selection",FILTER),("listing_args",LIST_ARGS),("control_args",CONTROL_ARGS),("expected_tests",TESTS)):
        exact(row[name],value,"Receipt argument/membership drift: "+name)
    exact([s["name"] for s in row["steps"]],STEPS,"Preflight must precede controls")
    for step in row["steps"]:
        exact(step["exit"],0,"First targeted step failed")
        exact(step["log"],step["name"]+".log","Wrong receipt log")
        exact(sha(directory/step["log"]),step["sha256"],"First log hash drift")
    for key in ("model_tests_started","model_only"):exact(row[key],True,"Missing model-only execution")
    for key in ("old_controls_executed","runtime_candidate_implemented","server","product"):exact(row[key],False,"Forbidden execution scope")
    exact(row["original_owner_profiles"],0,"No live profiles allowed")
    for name,expected in (("argv-list",LIST_ARGS),("argv-controls",CONTROL_ARGS)):
        observed=[json.loads(line) for line in (directory/(name+".log")).read_text(encoding="utf-8-sig").splitlines()]
        exact(observed,expected,"Wrong native argv vector")
    listing((directory/"listing.log").read_text(encoding="utf-8-sig"))
    compiler=(directory/"compiler.log").read_text(encoding="utf-8-sig")
    version="1.94.0" if lane=="194" else "1.88.0"
    need(compiler.startswith("rustc "+version+" ") and "host: x86_64-pc-windows-msvc" in compiler,"Wrong compiler/platform")
    return row


def analyze(root=ROOT):
    root=Path(root);packet=root/OUTPUT
    packets=[core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(p["members"] for p in packets),1282,"Prior sealed history changed")
    old=previous.analyze(root)
    exact(old,load(root/previous.OUTPUT/"facts.json"),"Prior REFUSED evidence changed")
    exact(old["targeted_isolation_gate_passed"],False,"Do not promote the broad run")
    source_pins(root)
    for name in (*PINS,"tools/authority-operator-targeted-074/review_targeted.py","tools/authority-operator-targeted-074/test_review_targeted.py","tools/authority-operator-targeted-074/final-check.ps1"):
        exact(canonical(root/name),canonical(packet/"method"/name),"Method snapshot drift: "+name)
    lanes=[]
    for index,lane in enumerate(("194","188")):
        directory=packet/"first"/lane;receipt(directory,lane)
        # The sealed prior reader independently reconstructs the semantic oracle.
        # Only its candidate correctness rows are compared; no old allocation
        # values or broad selection flags are copied into the clean result.
        rows=controls((directory/"controls.log").read_text(encoding="utf-8-sig"),old["lanes"][index]["first_broad"]["rows"])
        lanes.append(dict(lane=lane,tests=TESTS,passed=3,filtered=9,rows=rows))
    exact(lanes[0]["rows"],lanes[1]["rows"],"Compiler correctness disagreement")
    return dict(schema="authority-operator-targeted-074-v1",input_packets=packets,input_members_verified=1282,
        source=SOURCE,source_pins=PINS,lanes=lanes,targeted_isolation_gate_passed=True,prior_targeted_isolation_gate_passed=False,
        old_controls_executed=False,model_only=True,performance_claim=False,runtime_candidate_implemented=False,original_owner_profiles=0,
        original_windows_seconds=old["original_windows_seconds"],first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],
        original_material_gate_passed=False,full_sigkill_matrix=False,reconnect_proven=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate targeted evidence required")
    for node in (Path(path),*Path(path).parents):need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:need(args.verify,"Sealed review is read-only");core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Separate targeted correctness verified; prior broad scope remains REFUSED; no runtime admission")
