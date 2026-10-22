"""Read-only reconstruction of first pure bounded-read allocation baselines."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT / "tools/offline-authority-read-audit-074"))
import audit_read as prior

core=prior.previous.previous.previous
need,load,sha,exact=prior.need,prior.load,prior.sha,prior.exact
canonical,publish,BASE=prior.canonical,prior.publish,prior.BASE
counts=core.counts
OUTPUT=BASE / "authority-read-baseline-20261022"
INPUTS=dict(prior.INPUTS,**{"authority-read-audit-20261022":"1c5176e4e53eb56d21e66c061a9b580529b4930d32682e4397db4b5d3e0f6591"})
PINS={
    "tools/authority-read-baseline-074/Cargo.toml":"be9b54c722c201d1524409705a099168260b138e9d09818ae842371c61f4f7e9",
    "tools/authority-read-baseline-074/Cargo.lock":"e70fb12caf492ba10ba9201133336dee056fda18f0a5ac3bbee1d2cc28c29cde",
    "tools/authority-read-baseline-074/probe.rs":"7dff185665e80fadc5bb5f59518df51f18c7a917812c1434876832716c3d7543",
    "tools/authority-read-baseline-074/model.rs":"977c65e6e8a51ecb270842354d33b0a45e923250f8bbb0ea5125c7b3b367a655",
    "tools/authority-read-baseline-074/controls.rs":"f19eb5f9e559dc6a35d278e1218f5f11a0a047018c5073db9ef0d5b2782eec6a",
    "tools/authority-read-baseline-074/check.ps1":"abf2ab23be0e0d53e45b6c634fafb0ae1275489b64baee031175532d85d1219e",
    "crates/xtask/tests/authority_read_baseline_074.rs":"060a59d39bb545cbcc99bd1ea5de3f35b329ceb0c7723b468b0ba5dc2b73d258",
    "tools/proc-document-baseline-074/counter.rs":"2fb9539593499aaf1618f7be10f1452fbdbedc555fbf6f0a9d4f7955ab184d01",
    "docs/testing/performance/0.74/authority-read-baseline-plan-20261022.md":"aa242d9318b27e05b1d20ad48600e6e64dbb8e68c105df8a488b004a419097db",
}
CASES=(("empty",0,65,65,0,1,32),("one",1,65,65,0,2,32),("marker",65,65,65,0,2,65),
       ("marker-fragmented",65,65,1,0,66,65),("record-short",256,2048,2048,0,2,256),
       ("record-max",2048,2048,2048,0,2,2048),("record-fragmented",2048,2048,7,0,294,2048),
       ("record-interrupted",256,2048,11,1,26,256),("binary",37,2048,3,0,14,37))
HEALTHY=["open","before","regular","parent","read","after","drop-fresh"]


def source_pins(root):
    for name,pin in PINS.items():
        need(hashlib.sha256(canonical(Path(root)/name).encode()).hexdigest()==pin,f"Pure baseline source drift: {name}")
    return PINS


def valid_counts(row):
    exact(sorted(row),sorted(core.FIELDS),"Wrong counter fields")
    need(all(type(row[field]) is int and row[field]>=0 for field in core.FIELDS),"Nonnegative integer request counts required")
    return row


def add(a,b,n=1):
    return {field:(a[field]+b[field])*n for field in core.FIELDS}


def fault_trace(mask,revalidation):
    events=[]
    if revalidation:
        events.append("held")
        if mask&32:return "held",events
    for bit,event in ((1,"open"),(2,"before"),(4,"parent"),(8,"read"),(16,"after")):
        if event=="parent":events.append("regular")
        events.append(event)
        if mask&bit:
            if event!="open":events.append("drop-fresh")
            return event,events
    events.append("drop-fresh")
    return "none",events


def log_facts(text):
    sizes=dict(MAIN=9,STAGES=9,FAULT=96,METADATA=1,BOUNDS=1,OWNERSHIP=1,RECEIVERS=1)
    rows={kind:[] for kind in sizes};cleaned=[]
    for line in text.splitlines():
        if "HC074-AUTH-READ-" not in line:cleaned.append(line);continue
        prefix,payload=line.split("HC074-AUTH-READ-",1);kind,payload=payload.split(" ",1)
        need(kind in rows,"Unknown bounded-read row");rows[kind].append(json.loads(payload));cleaned.append(prefix.rstrip())
    for kind,size in sizes.items():exact(len(rows[kind]),size,"Missing/duplicate bounded-read row")
    tests=["counter::counter_nested_unwind_and_thread_isolation","counter::counter_zeroed_reallocation_and_drop_forwarding"]
    tests += ["model::controls::"+name for name in (
        "authority_main_nine_fixed_cases","authority_separate_constructor_read_return_and_drop_stages",
        "authority_all_first_error_masks_and_fake_file_retirement","authority_metadata_predicates_and_all_stamp_fields",
        "authority_bounds_eof_partial_error_and_standard_interrupted","authority_seeded_owned_returns_and_fresh_content_drift",
        "authority_synthetic_metadata_and_drop_receivers_allocate_nothing")]
    passed=re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$","\n".join(cleaned))
    exact(sorted(passed),sorted(tests),"Exact nine passing tests required")
    need("test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;" in text and
         "test result: FAILED" not in text,"Baseline selection failed")
    for main,stage,case in zip(rows["MAIN"],rows["STAGES"],CASES):
        name,length,budget,chunk,interrupt,reads,max_request=case
        capacity=main["output_capacity"]
        need(type(capacity) is int and capacity>=length,"Invalid output capacity")
        valid_counts(main["counts"])
        exact(main,dict(case=name,length=length,budget=budget,chunk=chunk,interrupt_at=interrupt,seed="0x074a17b3",
            samples=256,output_capacity=capacity,counts=main["counts"],last_trace=dict(events=HEALTHY,reads=reads,
            bytes=length,eof=1,interrupted=interrupt,read_errors=0,max_request=max_request,fresh_live=0,
            held_live=False,first_error="none"),model_only=True,timing_claim=False),"Main corpus/trace drift")
        for field in ("constructor","read","drop","whole_return","whole_drop"):valid_counts(stage[field])
        exact(stage,dict(case=name,samples_per_arm=1,not_pooled=True,output_capacity=capacity,
            constructor=counts(1 if length else 0,0,0,length),read=stage["read"],
            drop=counts(deallocations=1 if capacity else 0,deallocated_bytes=capacity),
            whole_return=stage["whole_return"],whole_drop=stage["whole_drop"],model_only=True),"Separate stage shape drift")
        exact(add(stage["constructor"],stage["read"]),stage["whole_return"],"Stage returning conservation failed")
        exact(stage["drop"],stage["whole_drop"],"Stage drop conservation failed")
        exact(main["counts"],add(stage["whole_return"],stage["whole_drop"],256),"Main/stage weighting drift")
    for index,row in enumerate(rows["FAULT"]):
        revalidation=index>=32;mask=index-32 if revalidation else index
        first,events=fault_trace(mask,revalidation)
        late=first in ("none","read","after")
        # These are the observed first reference results, not candidate thresholds.
        expected_counts=counts(1,0,0,65,1,65) if late else counts()
        observed=65 if first in ("none","after") else 0
        exact(row,dict(phase="revalidate" if revalidation else "read",mask=mask,
            outcome="ok" if first=="none" else "io",counts=expected_counts,trace=dict(events=events,
            reads=11 if observed else (1 if first=="read" else 0),bytes=observed,eof=1 if observed else 0,
            interrupted=0,read_errors=1 if first=="read" else 0,max_request=65 if late else 0,
            fresh_live=0,held_live=revalidation,first_error=first),not_pooled=True,model_only=True),"First-error prefix/lifetime/counters drift")
    exact(rows["METADATA"][0],dict(held_fields=11,after_fields=11,regular_predicates=5,parent_device=1,model_only=True),"Metadata controls drift")
    exact(rows["BOUNDS"][0],dict(cases=9,over_bound_bytes=66,partial_error_bytes=7,standard_interrupted=1,application_retry=False,model_only=True),"EOF/error bounds drift")
    exact(rows["OWNERSHIP"][0],dict(seed="0x074a17b3",samples=256,owned_after_token_drop_and_receiver_reset=True,
          independent_vec_clone=True,fresh_content_drift_refused=True,model_only=True),"Owned return drift")
    exact(rows["RECEIVERS"][0],dict(metadata=counts(),token_drop=counts(),model_only=True),"Synthetic receivers must not own requests")
    totals={field:sum(row["counts"][field] for row in rows["MAIN"]) for field in core.FIELDS}
    read_stages={field:sum(row["read"][field] for row in rows["STAGES"]) for field in core.FIELDS}
    return dict(rows=rows,main_samples=2304,main_totals=totals,read_stage_totals_separate=read_stages,
        stage_samples_per_arm=9,read_masks=32,revalidate_masks=64,early_allocation_free_read_refusals=28,
        early_allocation_free_revalidate_refusals=60,late_refusals_per_phase=3)


def analyze(root=ROOT,checks=None):
    root=Path(root);checks=Path(checks) if checks else root/OUTPUT/"first"
    packets=[core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(row["members"] for row in packets),955,"Old member total drift")
    need(canonical(root/"tools/offline-authority-read-audit-074/audit_read.py")==canonical(root/BASE/
         "authority-read-audit-20261022/method/tools/offline-authority-read-audit-074/audit_read.py"),"Frozen auditor drift")
    old=prior.analyze(root);exact(old,load(root/BASE/"authority-read-audit-20261022/facts.json"),"Prior ownership boundary drift")
    source_pins(root);lanes=[]
    for lane,version in (("194","1.94.0"),("188","1.88.0")):
        core.receipt(checks,lane)
        compiler=(checks/f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
        need(compiler.startswith("rustc "+version+" ") and "host: x86_64-pc-windows-msvc" in compiler,"Wrong compiler/platform")
        lanes.append(dict(lane=lane,compiler=version,facts=log_facts((checks/f"{lane}-controls.log").read_text(encoding="utf-8-sig"))))
    core.receipt(checks,"host")
    canaries=(checks/"host-canaries.log").read_text(encoding="utf-8-sig")
    passed=[int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;",canaries)]
    need(len(passed)==17 and sum(passed)==49 and "test result: FAILED" not in canaries,"Wrong source canary cohort")
    return dict(schema="pure-authority-read-baseline-074-v1",input_packets=packets,input_members_verified=955,
        source_pins=PINS,source_canaries=49,lanes=lanes,original_windows_seconds=old["original_windows_seconds"],
        first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],
        previous_boot_decision=old["previous_boot_decision"],interpretation=dict(
            generic_read_not_linux_file=True,tokens_not_os_fds=True,synthetic_bytes_not_admitted_records=True,
            stages_and_faults_not_pooled=True,no_wall_or_headroom_extrapolation=True,caller_latching_source_preserved=True),
        runtime_candidate_implemented=False,original_owner_profiles=0,original_material_gate_passed=False,
        full_sigkill_matrix=False,reconnect_proven=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate pure read baseline facts required")
    for node in (Path(path),*Path(path).parents):
        need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify,"Sealed review is read-only");core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Pure bounded-read baseline verified: two first Windows lanes, 17 prior seals/955 members; no runtime candidate")
