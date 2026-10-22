"""Offline reader of first owned journal model results; no image execution."""
import argparse
import hashlib
import itertools
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT/"tools/offline-authority-scratch-owner-audit-074"))
import audit_owners as previous
need,exact,load,sha=previous.need,previous.exact,previous.load,previous.sha
canonical,publish,BASE=previous.canonical,previous.publish,previous.BASE
core=previous.previous.core
counts=core.counts
OUTPUT=BASE/"authority-owned-journal-20261022"
INPUTS=dict(previous.INPUTS,**{"authority-scratch-owner-audit-20261022":"ea7ec7a0e1cb7d15c1032b519efdbbccd5ed28fb412f9137d4d615ec5b0db1d6"})
PINS={
    "tools/authority-owned-journal-074/Cargo.toml": "d189e96a50723e54075b5adbc6d19793cc00dd2c9d9d22335095d0ed11a805da",
    "tools/authority-owned-journal-074/owned_controls.rs": "16722b6f1de13725ea699474fa5f889c58c8c32b1f49bebfcf48e8c662c7827b",
    "crates/xtask/tests/authority_owned_journal_074.rs": "6f289745d058a479dd15d1007826c22949b6f0b664a196181864636f8cf83c46",
    "tools/authority-owned-journal-074/lifecycle.rs": "376aa0af4b834c3373a351e4c802abdaf20deec6a93099bae63436914db560a2",
    "tools/authority-owned-journal-074/journal.rs": "a7f5c9120025f92824a7ceda85b4c890c4cb5d72fadd13907b12df500752bbe0",
    "tools/authority-owned-journal-074/controls.rs": "a31f39f93fbe77d9be36782b3b610d05766d560a1d3e15f900fb4c841b5a5fb7",
    "tools/authority-owned-journal-074/owned.rs": "da6dedf137873f6257fd2b407a657a0aa14c3f813f1ba27c07c7417e7feb6d78",
    "tools/authority-owned-journal-074/check.ps1": "df81e3019aef9c9b95faf6280c200be755199e7dc44851d21523b633c0bcc506",
    "docs/testing/performance/0.74/authority-owned-journal-plan-20261022.md": "606c19c4c2a0c43ddccea954fd34692906bf06a5f6cc5cf95cfd2029d63cc147",
    "tools/authority-owned-journal-074/probe.rs": "484a21376c6666dbbca18ccd79262939566212a592d7700dbe865f30a5c66dbd",
    "tools/authority-owned-journal-074/model.rs": "662c09f8b1076920b0ce8f5079b1f2143e8136ab24803d2e395ca05a230d8f9b",
    "tools/authority-owned-journal-074/Cargo.lock": "fd26e22d5acb1ab2f38f01dafa0a0f09253fe0a9f4ff384e4ebffad31f89f010",
    "tools/proc-document-baseline-074/counter.rs": "2fb9539593499aaf1618f7be10f1452fbdbedc555fbf6f0a9d4f7955ab184d01"
}
REFERENCE_PINS={
    "docs/testing/performance/0.74/authority-owned-journal-plan-20261022.md": "606c19c4c2a0c43ddccea954fd34692906bf06a5f6cc5cf95cfd2029d63cc147",
    "tools/authority-owned-journal-074/check.ps1": "56d225e0e63397d3d972c2a9e9eb3ee528d26acedfd94908c8681d5381304a7e",
    "tools/authority-owned-journal-074/controls.rs": "a31f39f93fbe77d9be36782b3b610d05766d560a1d3e15f900fb4c841b5a5fb7",
    "tools/authority-owned-journal-074/model.rs": "614809972c25fed803a4c19935472d6981f47ef40cfc042a85fd8f817b84265d",
    "tools/authority-owned-journal-074/Cargo.lock": "fd26e22d5acb1ab2f38f01dafa0a0f09253fe0a9f4ff384e4ebffad31f89f010",
    "tools/authority-owned-journal-074/probe.rs": "484a21376c6666dbbca18ccd79262939566212a592d7700dbe865f30a5c66dbd",
    "tools/authority-owned-journal-074/journal.rs": "637bb8dcdb8c7abbe6729c1e5c21117bc4f56b99027e87198ec1437fe232a957",
    "tools/authority-owned-journal-074/Cargo.toml": "d189e96a50723e54075b5adbc6d19793cc00dd2c9d9d22335095d0ed11a805da"
}
GENERATIONS=(1,2,16,64,128)
SITES=("dir-before","inv-before","dir-after","inv-after","bind","transition","publication","external")
TESTS=("owned_main","owned_fault_matrix","owned_metadata_and_io","owned_lifecycle","owned_layout_drop_and_thread_affinity")


def source_pins(root):
    for name,pin in PINS.items():exact(hashlib.sha256(canonical(Path(root)/name).encode()).hexdigest(),pin,"Owned source drift: "+name)
    return PINS


def rows(text,phase):
    sizes=dict(REFERENCE=5) if phase=="reference" else dict(MAIN=5,FAULT=1152,GATE=32,DRIFT=1,LIFECYCLE=1,LAYOUT=1)
    found={k:[] for k in sizes};clean=[]
    for line in text.splitlines():
        if "HC074-OWNED-" not in line:clean.append(line);continue
        prefix,payload=line.split("HC074-OWNED-",1);kind,payload=payload.split(" ",1)
        need(kind in found,"Unknown owned row");found[kind].append(json.loads(payload));clean.append(prefix.rstrip())
    for kind,size in sizes.items():exact(len(found[kind]),size,"Missing/duplicate owned row: "+kind)
    expected=["model::controls::reference_only"] if phase=="reference" else ["model::owned_controls::"+t for t in TESTS]+[
        "counter::counter_nested_unwind_and_thread_isolation","counter::counter_zeroed_reallocation_and_drop_forwarding"]
    passed=re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$","\n".join(clean))
    exact(sorted(passed),sorted(expected),"Exact owned test selection required")
    summary="1 passed; 0 failed; 0 ignored; 0 measured; 2 filtered out;" if phase=="reference" else "7 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out;"
    need("test result: ok. "+summary in text and "test result: FAILED" not in text,"Owned test cohort failed")
    return found


def trace(value,events,reads,body,held,first):
    need(type(value["hash"]) is str and re.fullmatch("[0-9a-f]{16}",value["hash"]) is not None,"Diagnostic checksum required")
    exact(value,dict(hash=value["hash"],events=events,reads=reads,bytes=body,held=held,fresh=0,first_error=first),"Owned trace totals/prefix/lifetime changed")


def reference_facts(text):
    found=rows(text,"reference")["REFERENCE"]
    for row,g in zip(found,GENERATIONS):
        trace(row["trace"],4+16*g,4*g,2113*g,2*g,"none")
        exact(row,dict(generations=g,rounds=16,counts=counts(32*g,0,0,33808*g,32*g,33808*g),trace=row["trace"]),"Reference workload/request counts changed")
    return found


def candidate_facts(text,reference):
    found=rows(text,"candidate")
    for row,old,g in zip(found["MAIN"],reference,GENERATIONS):
        exact(row,dict(generations=g,rounds=16,reference=old["counts"],candidate=counts(2*g,0,0,2113*g),
            eviction=counts(deallocations=2*g,deallocated_bytes=2113*g),trace=old["trace"]),"Owned cold main/terminal trace/eviction changed")
    cases=[]
    for g in (1,2,16,128):
        for index in sorted({0,g//2,g-1}):
            cases.extend((g,index*2+role,mask) for role,mask in itertools.product(range(2),range(64)))
    for row,(g,slot,mask) in zip(found["FAULT"],cases):
        first,events=previous.previous.previous.fault_trace(mask,True)
        length=2048 if slot%2==0 else 65
        prefix=(slot//2)*2113+(2048 if slot%2 else 0)
        late=first in ("none","read","after")
        n=2*g if mask==0 else slot+int(late)
        requested=2113*g if mask==0 else prefix+(length if late else 0)
        target_bytes=length if first in ("none","after") else 0
        reads=4*g if mask==0 else slot*2+(2 if first=="after" else 1 if first=="read" else 0)
        body=2113*g if mask==0 else prefix+target_bytes
        trace(row["trace"],4+16*g if mask==0 else 2+slot*8+len(events),reads,body,2*g,first)
        exact(row,dict(generations=g,slot=slot,mask=mask,outcome="ok" if mask==0 else "io",
            reference=counts(n,0,0,requested,n,requested),candidate=counts(deallocations=2*g if mask else 0,
                deallocated_bytes=2113*g if mask else 0),trace=row["trace"]),"Owned fault precedence/cache eviction/counts changed")
    for row,(g,site) in zip(found["GATE"],itertools.product((1,2,16,128),SITES)):
        full=site not in ("dir-before","inv-before")
        event_count=1 if site=="dir-before" else 2 if site=="inv-before" else (3 if site=="dir-after" else 4 if site=="inv-after" else 5)+16*g
        trace(row["trace"],event_count,4*g if full else 0,2113*g if full else 0,2*g,"none")
        exact(row,dict(generations=g,site=site,outcome="invalid",candidate=counts(deallocations=2*g,deallocated_bytes=2113*g),trace=row["trace"]),"Journal gate/whole-cache cleanup changed")
    exact(found["DRIFT"],[dict(cases=582,metadata=56,io=14,content=512,seed="0x074a17b3",traces_equal=True)],"Drift coverage changed")
    exact(found["LIFECYCLE"],[dict(append_to_generations=128,growths=6,originals_preserved=True,reopen_new_tokens=True,
        reopen_cold=True,operator_state_distinct=True,append_failure_sticky=True)],"Logical lifecycle/relocation guards changed")
    exact(found["LAYOUT"],[dict(leaf=152,owned_leaf=176,entry=304,owned_entry=352,journal=40,owned_journal=40,owned_align=8,
        eviction=counts(deallocations=4,deallocated_bytes=4226),full_drop=counts(deallocations=13,deallocated_bytes=9884),
        send=False,sync=False,synthetic_not_linux_layout=True)],"Layout/independent original lifetime/drop changed")
    totals={arm:{f:sum(row[arm][f] for row in found["MAIN"]) for f in core.FIELDS} for arm in ("reference","candidate","eviction")}
    return dict(rows=found,main_totals=totals,main_leaf_calls_per_arm=6752,
        first_fault_masks=1152,non_leaf_gate_failures=32,extra_retained_capacity_at_128=270464,
        synthetic_owned_layout_delta_per_leaf=24,synthetic_layout_delta_at_128=6144)


def receipt(directory,lane,phase):
    if not (lane=="194" and phase=="reference"):row=core.receipt(directory,lane)
    else:
        row=load(Path(directory)/"194-result.json")
        exact(row["lane"],"194","Wrong first reference lane")
        exact([s["name"] for s in row["steps"]],["compiler","controls","check","clippy"],"First reference steps changed")
        exact([s["exit"] for s in row["steps"]],[0,0,0,101],"First 1.94 lint refusal lost")
        exact(row["failure"],"Baseline check refused: 194/clippy (101)","First lint refusal identity changed")
        for s in row["steps"]:
            exact(s["log"],"194-"+s["name"]+".log","Wrong first reference log identity")
            exact(sha(Path(directory)/s["log"]),s["sha256"],"First reference log changed")
        lint=(Path(directory)/"194-clippy.log").read_text(encoding="utf-8-sig")
        need("manual implementation of `.is_multiple_of()`" in lint,"Wrong retained lint refusal")
        need(row["cohort"]=="first" and row["platform"]=="windows-msvc" and row["model_only"] is True,"Wrong first reference receipt")
    exact(row["phase"],phase,"Mixed reference/candidate phases")
    for key in ("runtime_candidate_implemented","ignored_tests_run","server","product"):exact(row[key],False,"Promotion forbidden")
    exact(row["original_owner_profiles"],0,"Live profiles forbidden")
    return row


def analyze(root=ROOT):
    root=Path(root);packet=root/OUTPUT
    packets=[core.prior.verify_packet(root/BASE/name,pin) for name,pin in INPUTS.items()]
    exact(sum(p["members"] for p in packets),1111,"Prior history drift")
    exact(canonical(root/"tools/offline-authority-scratch-owner-audit-074/audit_owners.py"),canonical(root/BASE/"authority-scratch-owner-audit-20261022/method/tools/offline-authority-scratch-owner-audit-074/audit_owners.py"),"Frozen owner auditor drift")
    old=previous.analyze(root);exact(old,load(root/BASE/"authority-scratch-owner-audit-20261022/facts.json"),"Prior ownership boundary changed")
    source_pins(root)
    for name,pin in REFERENCE_PINS.items():exact(hashlib.sha256(canonical(packet/"reference-source"/name).encode()).hexdigest(),pin,"Frozen reference source drift")
    lanes=[]
    for lane,version in (("194","1.94.0"),("188","1.88.0")):
        for phase in ("reference","candidate"):
            checks=packet/phase/"first";receipt(checks,lane,phase)
            compiler=(checks/f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
            need(compiler.startswith("rustc "+version+" ") and "host: x86_64-pc-windows-msvc" in compiler,"Wrong compiler/platform")
        reference=reference_facts((packet/f"reference/first/{lane}-controls.log").read_text(encoding="utf-8-sig"))
        candidate=candidate_facts((packet/f"candidate/first/{lane}-controls.log").read_text(encoding="utf-8-sig"),reference)
        lanes.append(dict(lane=lane,compiler=version,reference=reference,candidate=candidate))
    exact(lanes[0]["candidate"],lanes[1]["candidate"],"Compiler model results disagree")
    core.receipt(packet/"candidate/first","host")
    text=(packet/"candidate/first/host-canaries.log").read_text(encoding="utf-8-sig")
    cohort=re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;",text)
    need(len(cohort)==19 and sum(map(int,cohort))==55 and "test result: FAILED" not in text,"Source guard cohort changed")
    return dict(schema="owned-authority-journal-model-074-v1",input_packets=packets,input_members_verified=1111,
        source_pins=PINS,reference_source_pins=REFERENCE_PINS,source_canaries=55,lanes=lanes,
        decision="retain-unpromoted-owned-model; audit actual engine integration before runtime mutation",
        new_timing_claim=False,physical_copy_measured=False,rss_measured=False,real_linux_layout_measured=False,
        first_reference_194_lint_refusal_retained=True,measurement_retries=0,synthetic_gates_not_filesystem_publication=True,
        original_windows_seconds=old["original_windows_seconds"],first_refusals=old["first_refusals"],qualification_sha256=old["qualification_sha256"],
        runtime_candidate_implemented=False,original_owner_profiles=0,original_material_gate_passed=False,
        full_sigkill_matrix=False,reconnect_proven=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT/OUTPUT/"facts.json").resolve(),"Separate owned model packet required")
    for p in (Path(path),*Path(path).parents):need(not p.is_symlink() and not getattr(p,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__);parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true");parser.add_argument("--packet-sha256");args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:need(args.verify,"Sealed review is read-only");core.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Owned model verified: 20 prior seals/1111 members; no runtime integration or live admission")
