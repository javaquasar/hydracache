"""Offline reconstruction; never launches either test image or a live profile."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import re
import statistics
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/authority-read-baseline-074"))
import review_read as previous

core=previous.core
need,exact,load,sha=previous.need,previous.exact,previous.load,previous.sha
canonical,publish,BASE=previous.canonical,previous.publish,previous.BASE
counts=previous.counts
OUTPUT=BASE/"authority-scratch-reuse-20261022"
INPUTS=dict(previous.INPUTS,**{"authority-read-baseline-20261022":"ed0d1199cbd8e105b3b1143173d67911719cf140d797f7c1dd192b20bf76ccc1"})
PINS={
    "tools/authority-scratch-reuse-074/cost.rs": "9fd7cb318eef66157a8443d9f12dbfd9b4ccb82bcf958c6b94dcb525bb554b2c",
    "tools/authority-scratch-reuse-074/check.ps1": "9ad576f85debb0e672614b74fda803c7986686b9ba35298c12ed9f8983681f58",
    "crates/xtask/tests/authority_scratch_reuse_074.rs": "1500f6ae469047ffef3905bce1174cb4a29627a6ef1587558f3cb78b51a06798",
    "tools/authority-read-baseline-074/controls.rs": "f19eb5f9e559dc6a35d278e1218f5f11a0a047018c5073db9ef0d5b2782eec6a",
    "docs/testing/performance/0.74/authority-scratch-reuse-plan-20261022.md": "090a0ba8961f366bbdfac7f47c033c9a83b4eb17f68c18b8c24d59729bf8dec3",
    "tools/proc-document-baseline-074/counter.rs": "2fb9539593499aaf1618f7be10f1452fbdbedc555fbf6f0a9d4f7955ab184d01",
    "tools/authority-scratch-reuse-074/candidate.rs": "858f79d15f14bec48ba5c00fb3f7e959931aa1b9f739a71585b09ec438d47b8f",
    "tools/authority-scratch-reuse-074/comparisons.rs": "02ed9700955627f0bd503a2d2ce69c58fff9bbac1749decf462d32d93a414b6e",
    "tools/authority-scratch-reuse-074/model.rs": "3c50938ffe4791fcff6ca2048bf66319efd9ba28a0965d2ff89b4be8c28fca4e",
    "tools/authority-scratch-reuse-074/Cargo.toml": "ca3637b72d3917d8250117b0a85249c7853d57a0055f9cd22f535dcb224f99ab",
    "tools/authority-scratch-reuse-074/probe.rs": "7dff185665e80fadc5bb5f59518df51f18c7a917812c1434876832716c3d7543",
    "tools/authority-scratch-reuse-074/Cargo.lock": "09285888ad45ab3b81ce526d767f90a15f6de1852e1d5e93947b20202a054b6c"
}
REFERENCE_PINS={
    "tools/authority-scratch-reuse-074/cost.rs": "9fd7cb318eef66157a8443d9f12dbfd9b4ccb82bcf958c6b94dcb525bb554b2c",
    "tools/authority-scratch-reuse-074/check.ps1": "f5101d85c7abb0f4ae3a175df28e538e4649450b406288e9bb51a0988a3f1b1a",
    "docs/testing/performance/0.74/authority-scratch-reuse-plan-20261022.md": "090a0ba8961f366bbdfac7f47c033c9a83b4eb17f68c18b8c24d59729bf8dec3",
    "tools/authority-scratch-reuse-074/model.rs": "96813110da02e2b7ba440130d5c36febc1b796830ab426b6343138fd9b1a3e3d",
    "tools/authority-scratch-reuse-074/Cargo.toml": "ca3637b72d3917d8250117b0a85249c7853d57a0055f9cd22f535dcb224f99ab",
    "tools/authority-scratch-reuse-074/probe.rs": "7dff185665e80fadc5bb5f59518df51f18c7a917812c1434876832716c3d7543",
    "tools/authority-scratch-reuse-074/Cargo.lock": "09285888ad45ab3b81ce526d767f90a15f6de1852e1d5e93947b20202a054b6c"
}
COMPARISONS=("scratch_cold_main","scratch_all_fault_masks","scratch_io_schedule_matrix",
             "scratch_metadata_and_seeded_drift","scratch_owner_lifetime_and_layout","scratch_paired_cost")


def source_pins(root):
    for path,pin in PINS.items():
        exact(hashlib.sha256(canonical(Path(root)/path).encode()).hexdigest(),pin,"Scratch source drift: "+path)
    return PINS


def rows(text,phase):
    sizes={"PREFLIGHT":14,"PREFLIGHT-COUNTS":2} if phase=="reference" else dict(MAIN=9,FAULT=64,IO=1512,DRIFT=1,LIFETIME=1,COST=14)
    found={k:[] for k in sizes};clean=[]
    for line in text.splitlines():
        if "HC074-SCRATCH-" not in line:
            clean.append(line);continue
        prefix,payload=line.split("HC074-SCRATCH-",1);kind,payload=payload.split(" ",1)
        need(kind in found,"Unknown scratch row");found[kind].append(json.loads(payload));clean.append(prefix.rstrip())
    for kind,size in sizes.items():exact(len(found[kind]),size,"Missing/duplicate scratch row: "+kind)
    clean="\n".join(clean)
    if phase=="reference":
        expected=["model::cost::reference_only_preflight"];summary="1 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out;"
    else:
        expected=["counter::counter_nested_unwind_and_thread_isolation","counter::counter_zeroed_reallocation_and_drop_forwarding"]
        expected += ["model::controls::"+n for n in ("authority_main_nine_fixed_cases","authority_separate_constructor_read_return_and_drop_stages",
            "authority_all_first_error_masks_and_fake_file_retirement","authority_metadata_predicates_and_all_stamp_fields",
            "authority_bounds_eof_partial_error_and_standard_interrupted","authority_seeded_owned_returns_and_fresh_content_drift",
            "authority_synthetic_metadata_and_drop_receivers_allocate_nothing")]
        expected += ["model::comparisons::"+n for n in COMPARISONS]
        summary="15 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;"
    # Strip embedded legacy rows before matching test completion lines.
    clean=re.sub(r"HC074-AUTH-READ-[^\n]*","",clean)
    passed=re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$",clean)
    exact(sorted(passed),sorted(expected),"Exact scratch test selection required")
    need("test result: ok. "+summary in text and "test result: FAILED" not in text,"Scratch selection failed")
    return found


def positive_ns(value):
    need(type(value) is int and value>0,"Positive integer duration required")


def preflight_facts(text):
    found=rows(text,"reference")
    for row,(length,sample) in zip(found["PREFLIGHT"],itertools.product((65,2048),range(7))):
        positive_ns(row["ns"])
        exact(row,dict(length=length,budget=length,sample=sample,warm=64,calls=512,ns=row["ns"],checksum=length*512),"Reference timing corpus drift")
    for row,length in zip(found["PREFLIGHT-COUNTS"],(65,2048)):
        exact(row,dict(length=length,calls=512,checksum=length*512,counts=counts(512,0,0,length*512,512,length*512)),"Full-method reference counts drift")
    return found


def candidate_facts(text):
    found=rows(text,"candidate")
    # Reconstruct every frozen old control with the unchanged old reviewer.
    legacy=[]
    old_names=[]
    for line in text.splitlines():
        if "HC074-AUTH-READ-" in line:legacy.append("HC074-AUTH-READ-"+line.split("HC074-AUTH-READ-",1)[1])
        if line.startswith("test counter::") or line.startswith("test model::controls::"):
            old_names.append(line.split(" ...",1)[0]+" ... ok")
    legacy += old_names+["test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"]
    frozen=previous.log_facts("\n".join(legacy))
    for row,case in zip(found["MAIN"],previous.CASES):
        name,length,budget,chunk,interrupted,reads,request=case
        exact(row,dict(case=name,length=length,samples=256,cold=True,
            reference=counts(256 if length else 0,0,0,length*256,256 if length else 0,length*256),
            candidate=counts(1 if length else 0,0,0,length),cache_capacity=length,
            cache_drop=counts(deallocations=1 if length else 0,deallocated_bytes=length),
            trace=dict(events=["held"]+previous.HEALTHY,reads=reads,bytes=length,eof=1,interrupted=interrupted,
                read_errors=0,max_request=request,fresh_live=0,held_live=True,first_error="none")),"Cold scratch corpus/counts/drop/trace drift")
    for mask,row in enumerate(found["FAULT"]):
        old=frozen["rows"]["FAULT"][32+mask]
        exact(row,dict(mask=mask,outcome=old["outcome"],reference=old["counts"],candidate=counts(
            deallocations=1 if mask else 0,deallocated_bytes=65 if mask else 0),cached=None if mask else 65,trace=old["trace"]),"Warm scratch first-error/eviction drift")
    corpus=itertools.product((0,1,9,10,11,65,66),(0,9,10,11,65,67),(1,7,128),(0,1,2,3,4,12),(0,1))
    for row,case in zip(found["IO"],corpus):
        declared,body,chunk,error,interrupted=case
        exact({k:row[k] for k in ("declared","body","chunk","error","interrupted")},dict(zip(("declared","body","chunk","error","interrupted"),case)),"IO schedule reordered/changed")
        exact(sorted(row),sorted(("declared","body","chunk","error","interrupted","outcome","cached","trace")),"IO fields changed")
        t=row["trace"]
        exact(sorted(t),sorted(("events","reads","bytes","eof","interrupted","read_errors","max_request","fresh_live","held_live","first_error")),"IO trace fields changed")
        for k in ("reads","bytes","eof","interrupted","read_errors","max_request"):
            need(type(t[k]) is int and t[k]>=0,"Invalid IO trace scalar")
        need(t["bytes"]<=min(body,66) and t["max_request"]<=66,"IO bound changed")
        exact(t["fresh_live"],0,"Fresh file leaked");exact(t["held_live"],True,"Owner lost")
        need(row["outcome"] in ("ok","io","invalid","budget"),"Unknown outcome")
        need(row["cached"] is None if row["outcome"]!="ok" else type(row["cached"]) is int and row["cached"]==10,"IO cache eviction drift")
        if declared==66:
            exact(row["outcome"],"budget","Over-budget must refuse before read")
            exact(t,dict(events=["held","open","before","regular","parent","drop-fresh"],reads=0,bytes=0,eof=0,interrupted=0,read_errors=0,max_request=0,fresh_live=0,held_live=True,first_error="none"),"Over-budget prefix drift")
        else:
            need(t["events"] in (["held","open","before","regular","parent","read","drop-fresh"], ["held"]+previous.HEALTHY),"IO observation order drift")
            if row["outcome"]=="io":
                exact(t["first_error"],"read","IO first error drift");exact(t["read_errors"],1,"IO error missing")
                exact(t["reads"],error,"Call-index error moved")
            else:
                exact(t["first_error"],"none","Invented first error");exact(t["read_errors"],0,"Invented IO error")
                exact(t["events"],["held"]+previous.HEALTHY,"Missing after check")
                expected="ok" if declared==body==10 else "invalid"
                exact(row["outcome"],expected,"Length/stamp/content refusal changed")
    exact(found["DRIFT"],[dict(metadata=28,content=256,seed="0x074a17b3",all_refused=True)],"Drift controls changed")
    exact(found["LIFETIME"],[dict(constructor=counts(),cache_drop=counts(deallocations=1,deallocated_bytes=65),size=32,align=8,owner_alive=True,owner_bytes_unchanged=True)],"Lifetime/layout drift")
    medians=[]
    for row,(length,pair) in zip(found["COST"],itertools.product((65,2048),range(7))):
        positive_ns(row["reference_ns"]);positive_ns(row["candidate_ns"])
        exact(row,dict(length=length,budget=length,pair=pair,candidate_first=bool(pair%2),warm=64,calls=512,checksum=length*512,
            reference_ns=row["reference_ns"],candidate_ns=row["candidate_ns"]),"Paired cost workload/order changed")
    for length in (65,2048):
        group=[r for r in found["COST"] if r["length"]==length]
        medians.append(dict(length=length,reference_ns=statistics.median(r["reference_ns"] for r in group),candidate_ns=statistics.median(r["candidate_ns"] for r in group)))
    return dict(rows=found,legacy=frozen,medians=medians,
        main_reference=counts(2048,0,0,1222656,2048,1222656),main_candidate=counts(8,0,0,4776),
        cache_drop_separate=counts(deallocations=8,deallocated_bytes=4776),main_calls_per_arm=2304)


def analyze(root=ROOT):
    root=Path(root);packet=root/OUTPUT
    packets=[core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(p["members"] for p in packets),1000,"Previous raw history drift")
    exact(canonical(root/"tools/authority-read-baseline-074/review_read.py"),canonical(root/BASE/"authority-read-baseline-20261022/method/tools/authority-read-baseline-074/review_read.py"),"Old reviewer drift")
    old=previous.analyze(root);exact(old,load(root/BASE/"authority-read-baseline-20261022/facts.json"),"Frozen baseline drift")
    source_pins(root)
    for path,pin in REFERENCE_PINS.items():
        exact(hashlib.sha256(canonical(packet/"reference-source"/path).encode()).hexdigest(),pin,"Reference source snapshot drift")
    lanes=[]
    for lane,version in (("194","1.94.0"),("188","1.88.0")):
        facts={}
        for phase,reader in (("reference",preflight_facts),("candidate",candidate_facts)):
            checks=packet/phase/"first";receipt=core.receipt(checks,lane)
            exact(receipt["phase"],phase,"Mixed measurement phases")
            compiler=(checks/f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
            need(compiler.startswith("rustc "+version+" ") and "host: x86_64-pc-windows-msvc" in compiler,"Wrong compiler/platform")
            facts[phase]=reader((checks/f"{lane}-controls.log").read_text(encoding="utf-8-sig"))
        lanes.append(dict(lane=lane,compiler=version,**facts))
    core.receipt(packet/"candidate/first","host")
    text=(packet/"candidate/first/host-canaries.log").read_text(encoding="utf-8-sig")
    cohort=[int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;",text)]
    need(len(cohort)==18 and sum(cohort)==52 and "test result: FAILED" not in text,"Source canary cohort changed")
    first=load(packet/"tooling-first/host-result.json")
    exact([(s["name"],s["exit"]) for s in first["steps"]],[("canaries",101)],"First source refusal lost")
    for step in first["steps"]:exact(sha(packet/"tooling-first"/step["log"]),step["sha256"],"First refusal log drift")
    rejected=any(all(l["candidate"]["medians"][i]["candidate_ns"]>l["candidate"]["medians"][i]["reference_ns"] for l in lanes) for i in (0,1))
    return dict(schema="owner-bound-authority-scratch-074-v1",input_packets=packets,input_members_verified=1000,
        source_pins=PINS,reference_source_pins=REFERENCE_PINS,source_canaries=52,lanes=lanes,
        decision="reject-for-adoption" if rejected else "retain-unpromoted-prototype",allocation_hypothesis_confirmed_in_model=True,
        timing_is_preliminary=True,concurrent_host_activity_not_isolated=True,physical_copy_bytes_measured=False,rss_measured=False,
        retained_capacity_is_additional=True,scratch_wrapper_bytes=32,
        original_windows_seconds=old["original_windows_seconds"],first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],
        runtime_candidate_implemented=False,original_owner_profiles=0,original_material_gate_passed=False,
        full_sigkill_matrix=False,reconnect_proven=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate scratch packet required")
    for node in (Path(path),*Path(path).parents):
        need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify,"Sealed review is read-only");core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Scratch evidence verified: 18 prior seals/1000 members; storage prototype only, no runtime promotion")
