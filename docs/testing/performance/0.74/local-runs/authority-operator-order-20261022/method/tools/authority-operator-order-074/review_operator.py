"""Retain first operator correctness observations AND refuse their broad selection."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/authority-owned-journal-074"))
import review_owned as previous
need,exact,load,sha=previous.need,previous.exact,previous.load,previous.sha
canonical,publish,BASE=previous.canonical,previous.publish,previous.BASE
core=previous.core
OUTPUT=BASE/"authority-operator-order-20261022"
INPUTS=dict(previous.INPUTS,**{"authority-owned-journal-20261022":"ccadbe4fe15fe336049851019da4cef913401c09f55d0c875c9af5b257ac60f7"})
PINS={
    "tools/authority-operator-order-074/selection.ps1": "9d0cece286fdb0c89f457fdc2bdad40d561896c82cc8b7f44252278f4e16924a",
    "tools/authority-operator-order-074/argv-check.ps1": "f27aa85750ab0db28783a661cb06f11e6021b68fdb1d5d9d62ad26e80472e512",
    "tools/authority-operator-order-074/operator.rs": "bba5fbb3ac7bb5914e4a8e5eebc6f2406470c19cfa28137ba251c57331eca693",
    "tools/authority-operator-order-074/model.rs": "765c4cc914b1aadf01aa18a5be7db54b6b573ba7b0af28cfd6d0289927cc3ca0",
    "tools/authority-operator-order-074/candidate_controls.rs": "1f51e3cb4265f0e234743adc2449e0d32c503f5656cf09178099f5933350416e",
    "tools/authority-operator-order-074/check.ps1": "909dbd150305667ebbb793178f9935cc4da74a10c1419eeea6c839da25ce6d1a",
    "tools/authority-operator-order-074/Cargo.toml": "e6b82056407c5888b4f12b06f97ea40d7f08f5efec199c0eab7284c2bbbc835d",
    "tools/authority-operator-order-074/argv.rs": "7d7a9594f3e4b833b19bda96d44b770588c0802bb57715d0244d4785839fad6f",
    "tools/authority-operator-order-074/journal.rs": "42210559ea8138735bffee456be92ffc2c9f67f43f188fc30f76da6e6a0f768f",
    "tools/authority-operator-order-074/operator_controls.rs": "146f74ef8e971af3cfd9d0f560ba098d7f4776460c12061998bbecb47371ae00",
    "tools/authority-operator-order-074/probe.rs": "0e62e56b5063e354911eb79263ddd2464d76b7d7244f065e6d6209bbfc3f67a3",
    "tools/authority-operator-order-074/Cargo.lock": "4134aac57c236b6bb2d2a05c6534bfbb4cd45bbd42205b740307acc183865f11",
    "docs/testing/performance/0.74/authority-operator-order-plan-20261022.md": "1ba513fb4f48a2359ccd63f77ed6580684ab4ee47692f5cfd2cf0518507a08a9",
    "crates/xtask/tests/authority_operator_order_074.rs": "96a035e001afad881c19b24e246871553ce934675f6115c544e1aedb2438249a"
}
REFERENCE_PINS={
    "tools/authority-operator-order-074/probe.rs": "0e62e56b5063e354911eb79263ddd2464d76b7d7244f065e6d6209bbfc3f67a3",
    "tools/authority-operator-order-074/check.ps1": "46faf69b31e92251a1f26413ff2ec3c9ae08e375902cb01571139683670af743",
    "tools/authority-operator-order-074/Cargo.lock": "4134aac57c236b6bb2d2a05c6534bfbb4cd45bbd42205b740307acc183865f11",
    "tools/authority-operator-order-074/journal.rs": "0a5b37221bb60bca27f4bd3a26fa9ce4d050a31bff0e2c59bcd334e70c782a51",
    "tools/authority-operator-order-074/operator_controls.rs": "146f74ef8e971af3cfd9d0f560ba098d7f4776460c12061998bbecb47371ae00",
    "docs/testing/performance/0.74/authority-operator-order-plan-20261022.md": "1ba513fb4f48a2359ccd63f77ed6580684ab4ee47692f5cfd2cf0518507a08a9",
    "tools/authority-operator-order-074/Cargo.toml": "e6b82056407c5888b4f12b06f97ea40d7f08f5efec199c0eab7284c2bbbc835d",
    "tools/authority-operator-order-074/model.rs": "c1fbd2dbec2e5b5f2e1d8b74032aebee2ccef0539c81a9a35ef4b6906a393c5b"
}
FIRST_PINS={
    "tools/authority-operator-order-074/operator.rs": "bba5fbb3ac7bb5914e4a8e5eebc6f2406470c19cfa28137ba251c57331eca693",
    "tools/authority-operator-order-074/candidate_controls.rs": "1f51e3cb4265f0e234743adc2449e0d32c503f5656cf09178099f5933350416e",
    "tools/authority-operator-order-074/Cargo.lock": "4134aac57c236b6bb2d2a05c6534bfbb4cd45bbd42205b740307acc183865f11",
    "tools/authority-operator-order-074/model.rs": "765c4cc914b1aadf01aa18a5be7db54b6b573ba7b0af28cfd6d0289927cc3ca0",
    "crates/xtask/tests/authority_operator_order_074.rs": "96a035e001afad881c19b24e246871553ce934675f6115c544e1aedb2438249a",
    "tools/authority-operator-order-074/operator_controls.rs": "146f74ef8e971af3cfd9d0f560ba098d7f4776460c12061998bbecb47371ae00",
    "tools/authority-operator-order-074/Cargo.toml": "e6b82056407c5888b4f12b06f97ea40d7f08f5efec199c0eab7284c2bbbc835d",
    "tools/authority-operator-order-074/journal.rs": "42210559ea8138735bffee456be92ffc2c9f67f43f188fc30f76da6e6a0f768f",
    "tools/authority-operator-order-074/check.ps1": "6d000035036c81e332eab01faf87bfd1e5441da2bddece24e3556ce8f9ec8df6",
    "tools/authority-operator-order-074/probe.rs": "0e62e56b5063e354911eb79263ddd2464d76b7d7244f065e6d6209bbfc3f67a3"
}
OPS=("refuse","advance-less","advance-equal","advance-greater")
CANDIDATES=("candidate_state_priority","candidate_initial_fault_priority","candidate_publication_and_external_latch")


def source_pins(root):
    for name,pin in PINS.items():exact(hashlib.sha256(canonical(Path(root)/name).encode()).hexdigest(),pin,"Operator source drift: "+name)
    return PINS


def oracle(g,failed,refused,op):
    if failed:return "refused",0
    valid=(not refused) if op=="refuse" else refused and op=="advance-greater"
    return ("transition",1) if not valid else ("budget",1) if g==128 else ("ok",3)


def parse(text,phase):
    sizes=dict(LEGACY=16) if phase=="reference" else dict(LEGACY=16,STATE=80,FAULT=3072,GATE=96,CUT=24,EXTERNAL=24)
    found={k:[] for k in sizes};clean=[];excluded={}
    for line in text.splitlines():
        if "HC074-OWNED-" in line:
            kind=line.split("HC074-OWNED-",1)[1].split(" ",1)[0];excluded[kind]=excluded.get(kind,0)+1
            clean.append(line.split("HC074-OWNED-",1)[0].rstrip());continue
        if "HC074-OP-" not in line:clean.append(line);continue
        prefix,payload=line.split("HC074-OP-",1);kind,payload=payload.split(" ",1)
        need(kind in found,"Unknown operator row");found[kind].append(json.loads(payload));clean.append(prefix.rstrip())
    for kind,size in sizes.items():exact(len(found[kind]),size,"Missing/duplicate operator rows: "+kind)
    expected=["model::operator_controls::legacy_counterexamples"]
    if phase!="reference":
        expected += ["model::candidate_controls::"+name for name in CANDIDATES]
        expected += ["counter::counter_nested_unwind_and_thread_isolation","counter::counter_zeroed_reallocation_and_drop_forwarding","model::controls::reference_only"]
        expected += ["model::owned_controls::"+name for name in previous.TESTS]
        exact(excluded,dict(REFERENCE=5,MAIN=5,FAULT=1152,GATE=32,DRIFT=1,LIFECYCLE=1,LAYOUT=1),"Unexpected old-control execution record lost")
    else:exact(excluded,{},"Reference scope changed")
    passed=re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$","\n".join(clean))
    exact(sorted(passed),sorted(expected),"Exact actual first test membership required, not relabelled targeted")
    summary="1 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out;" if phase=="reference" else "12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"
    need("test result: ok. "+summary in text and "test result: FAILED" not in text,"First operator correctness run failed")
    return found,excluded


def legacy_facts(text):
    found,_=parse(text,"reference");rows=found["LEGACY"]
    for row,(failed,refused,op) in zip(rows,itertools.product((False,True),(False,True),OPS)):
        valid=(not refused) if op=="refuse" else refused and op=="advance-greater"
        raw="invalid" if not valid else "refused" if failed else "ok"
        normalized="transition" if raw=="invalid" else raw
        expected,walks=oracle(1,failed,refused,op)
        observed_walks=2 if valid and not failed else 0
        exact(row,dict(failed=failed,data_refused=refused,operation=op,raw=raw,normalized=normalized,expected=expected,
            walks=observed_walks,expected_walks=walks,outcome_mismatch=normalized!=expected,walk_mismatch=observed_walks!=walks),"Legacy counterexample changed")
    exact(sum(r["outcome_mismatch"] for r in rows),6,"Six original priority counterexamples required")
    exact(sum(r["walk_mismatch"] for r in rows),8,"Eight original schedule counterexamples required")
    return rows


def hashed(row):
    need(type(row["trace_hash"]) is str and re.fullmatch("[0-9a-f]{16}",row["trace_hash"]) is not None,"Diagnostic trace checksum required")
    return row["trace_hash"]


def broad_facts(text):
    found,excluded=parse(text,"broad")
    for row,(g,failed,refused,op) in zip(found["STATE"],itertools.product((1,2,16,64,128),(False,True),(False,True),OPS)):
        code,walks=oracle(g,failed,refused,op);ok=code=="ok"
        events=0 if failed else 48*g+40 if ok else 16*g+4
        exact(row,dict(generations=g,failed=failed,data_refused=refused,operation=op,outcome=code,walks=walks,events=events,
            entries_after=g+int(ok),refused_after=(op=="refuse") if ok else refused,epoch_after=8 if ok and op!="refuse" else 7,
            cache_after=(g+1)*2113 if ok and op!="refuse" else 0,trace_hash=hashed(row)),"Candidate priority/state/walk schedule changed")
    cases=((g,refused,op,g//2*2+role,mask) for g,refused,op,role,mask in itertools.product((1,16,128),(False,True),OPS,range(2),range(64)))
    for row,(g,refused,op,slot,mask) in zip(found["FAULT"],cases):
        code,walks=oracle(g,False,refused,op) if mask==0 else ("io",1)
        if mask:
            _,prefix=previous.previous.previous.previous.fault_trace(mask,True)
            events=2+slot*8+len(prefix)
        else:events=48*g+40 if code=="ok" else 16*g+4
        exact(row,dict(generations=g,data_refused=refused,operation=op,slot=slot,mask=mask,outcome=code,walks=walks,events=events,
            cache_after=(g+1)*2113 if code=="ok" and op!="refuse" else 0,trace_hash=hashed(row)),"Initial IO error lost precedence or cache survived")
    sites=("dir-before","inv-before","dir-after","inv-after")
    for row,(g,refused,op,site) in zip(found["GATE"],itertools.product((1,16,128),(False,True),OPS,sites)):
        events=1 if site=="dir-before" else 2 if site=="inv-before" else 16*g+(3 if site=="dir-after" else 4)
        exact(row,dict(generations=g,data_refused=refused,operation=op,site=site,outcome="invalid",walks=1,events=events,cache_after=0),"Initial bracket/error/cleanup changed")
    for row,(g,op,cut) in zip(found["CUT"],itertools.product((1,16,64),("refuse","advance-greater"),range(4))):
        appended=cut>=2
        exact(row,dict(generations=g,operation=op,cut=cut,outcome="io" if cut==3 else "invalid",walks=3 if cut==3 else 2,
            events=16*g+6 if cut==0 else 32*g+9 if cut==1 else 32*g+21 if cut==2 else 32*g+23,
            entries_after=g+int(appended),refused_after=(op=="refuse") if appended else op!="refuse",
            epoch_after=8 if appended and op!="refuse" else 7,cache_after=0,trace_hash=hashed(row)),"Logical publication cut/state/cleanup changed")
    for row,(g,refused,op) in zip(found["EXTERNAL"],itertools.product((1,16,128),(False,True),OPS)):
        exact(row,dict(generations=g,data_refused=refused,operation=op,outcome="refused",walks=0,events=0,cache_after=0),"Direct external latch admitted IO")
    # Deliberately omit every unexpected old allocation/control value and the
    # repeated legacy rows from derived candidate facts. Raw logs stay complete.
    return dict(rows={k:v for k,v in found.items() if k!="LEGACY"},excluded_old_rows=excluded,
        excluded_repeated_legacy_rows=len(found["LEGACY"]),targeted_isolation_gate_passed=False,
        scoped_model_outcomes_match=True)


def argv_facts(directory):
    directory=Path(directory);row=load(directory/"result.json")
    exact(row["failure"],None,"Native argv repair refused")
    exact([s["name"] for s in row["steps"]],["compile","broken-candidate","fixed-reference","fixed-candidate"],"Wrong argv cohort")
    for s in row["steps"]:
        exact(s["exit"],0,"Argv control failed");exact(s["log"],s["name"]+".log","Wrong argv log")
        exact(sha(directory/s["log"]),s["sha256"],"Native argv log changed")
    for key in ("model_image_executed","old_controls_executed","server","product"):exact(row[key],False,"Argv repair executed an image")
    exact(row["native_argv_only"],True,"Argv-only receipt required")
    observed={name:[json.loads(line) for line in (directory/(name+".log")).read_text(encoding="utf-8-sig").splitlines()] for name in ("broken-candidate","fixed-reference","fixed-candidate")}
    base=["--nocapture","--test-threads=1"]
    exact(observed["broken-candidate"],base+list("model::candidate_controls::candidate_"),"String-splat defect not retained")
    exact(observed["fixed-reference"],base+["model::operator_controls::legacy_counterexamples","--exact"],"Reference argv repair drift")
    exact(observed["fixed-candidate"],base+["model::candidate_controls::candidate_"],"Candidate filter must be one native argument")
    return observed


def analyze(root=ROOT):
    root=Path(root);packet=root/OUTPUT
    packets=[core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(p["members"] for p in packets),1183,"Prior sealed history changed")
    exact(canonical(root/"tools/authority-owned-journal-074/review_owned.py"),canonical(root/BASE/"authority-owned-journal-20261022/method/tools/authority-owned-journal-074/review_owned.py"),"Frozen reader drift")
    old=previous.analyze(root);exact(old,load(root/BASE/"authority-owned-journal-20261022/facts.json"),"Prior first measurements changed")
    source_pins(root)
    for family,pins in (("reference-source",REFERENCE_PINS),("broad-first-source",FIRST_PINS)):
        for name,pin in pins.items():exact(hashlib.sha256(canonical(packet/family/name).encode()).hexdigest(),pin,"First source drift")
    lanes=[]
    for lane,version in (("194","1.94.0"),("188","1.88.0")):
        for family,phase in (("reference","reference"),("broad-first","candidate")):
            directory=packet/family/"first";receipt=core.receipt(directory,lane);exact(receipt["phase"],phase,"First phase changed")
            compiler=(directory/f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
            need(compiler.startswith("rustc "+version+" ") and "host: x86_64-pc-windows-msvc" in compiler,"Wrong compiler/platform")
        lanes.append(dict(lane=lane,legacy=legacy_facts((packet/f"reference/first/{lane}-controls.log").read_text(encoding="utf-8-sig")),
            first_broad=broad_facts((packet/f"broad-first/first/{lane}-controls.log").read_text(encoding="utf-8-sig"))))
    exact(lanes[0]["legacy"],lanes[1]["legacy"],"Legacy counterexamples disagree")
    exact(lanes[0]["first_broad"],lanes[1]["first_broad"],"Scoped candidate observations disagree")
    core.receipt(packet/"broad-first/first","host")
    text=(packet/"broad-first/first/host-canaries.log").read_text(encoding="utf-8-sig")
    cohorts=re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;",text)
    need(len(cohorts)==20 and sum(map(int,cohorts))==58 and "test result: FAILED" not in text,"Source guard cohort changed")
    return dict(schema="authority-operator-order-074-v1",input_packets=packets,input_members_verified=1183,
        source_pins=PINS,reference_source_pins=REFERENCE_PINS,first_source_pins=FIRST_PINS,source_guards=58,lanes=lanes,
        native_argv=argv_facts(packet/"argv"),decision="refuse-target-isolation; retain-scoped-model-and-argv-repair",
        targeted_isolation_gate_passed=False,unexpected_old_controls_executed=True,unexpected_allocation_rows_excluded=True,
        model_rerun_after_argv_repair=False,performance_claim=False,runtime_candidate_implemented=False,original_owner_profiles=0,
        original_windows_seconds=old["original_windows_seconds"],first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],
        original_material_gate_passed=False,full_sigkill_matrix=False,reconnect_proven=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate operator evidence required")
    for p in (Path(path),*Path(path).parents):need(not p.is_symlink() and not getattr(p,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:need(args.verify,"Sealed review is read-only");core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Operator evidence verified; target-isolation gate remains REFUSED; no new performance claim")
