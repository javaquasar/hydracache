"""Read-only reconstruction of first Windows boot ownership controls; no probe entry."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT=Path(__file__).resolve().parents[2]
sys.path.insert(0,str(ROOT / "tools/proc-document-baseline-074"))
import review as previous

need,load,sha,exact,counts=previous.need,previous.load,previous.sha,previous.exact,previous.counts
canonical=previous.prior.canonical
publish=previous.prior.prior.rounds.prior.publish
BASE=previous.BASE
OUTPUT=BASE / "sample-boot-baseline-20261022"
INPUTS=dict(previous.INPUTS,**{
    "proc-document-baseline-20261022":"dbaa727197cf3c08b5206b40050bef4424f8a1e1793191ca661ba5540a81c306",
})
PROBE_PINS={
    "tools/sample-boot-baseline-074/Cargo.toml":"b34a9234cd793fb5bdd1ba34bbb21ad6a47b6ce1e63d8d68d370941763271879",
    "tools/sample-boot-baseline-074/Cargo.lock":"f19eee452c1bbce3d963ba2eaf6e4174ca945fa6cf86fc8f8b5c5a8b90426529",
    "tools/sample-boot-baseline-074/probe.rs":"138629805a63831938e215b6416b353e72284cc0db46d42b077dd5bc3b6baf6d",
    "tools/sample-boot-baseline-074/model.rs":"bceccf83a930d220eeb5a392e06e46c2c12f4a07a0ae32c2ac697040cd714019",
    "tools/sample-boot-baseline-074/controls.rs":"b50230b6e8a1423511fc09b8a3a1294b16d9f21ef0c9667af8a8835af8594d0a",
    "crates/xtask/tests/sample_boot_baseline_074.rs":"d3c5eef01ad1bfb64cac8faed867601102fc2a0cf4dbefb9e83d45db15570378",
    "tools/proc-document-baseline-074/counter.rs":"2fb9539593499aaf1618f7be10f1452fbdbedc555fbf6f0a9d4f7955ab184d01",
}
CASES=(("empty",0,0),("one",1,1),("below",35,35),("fixed",36,36),
       ("fixed-spare",36,256),("above",37,37),("utf8-generic",36,36),("long",64,64),("larger",1024,1024))
KINDS=("MAIN","STAGES","FAULTS","SEEDED","DECODER")


def source_pins(root):
    for name,pin in PROBE_PINS.items():
        need(hashlib.sha256(canonical(root / name).encode("utf-8")).hexdigest()==pin,f"Probe source drift: {name}")
    return PROBE_PINS


def log_facts(text):
    rows={kind:[] for kind in KINDS}
    cleaned=[]
    for line in text.splitlines():
        marker="HC074-SAMPLE-BOOT-"
        if marker not in line:
            cleaned.append(line)
            continue
        prefix,payload=line.split(marker,1)
        kind,payload=payload.split(" ",1)
        need(kind in KINDS,"Unknown boot control row")
        rows[kind].append(json.loads(payload))
        cleaned.append(prefix.rstrip())
    for kind,size in (("MAIN",9),("STAGES",9),("FAULTS",1),("SEEDED",1),("DECODER",1)):
        need(len(rows[kind])==size,"Missing/duplicate boot control row")
    for index,(name,length,capacity) in enumerate(CASES):
        allocations=256 if length else 0
        exact(rows["MAIN"][index],dict(case=name,length=length,input_capacity=capacity,output_capacity=length,samples=256,
            counts=counts(allocations,0,0,length*256,allocations,length*256),synthetic_frame=True,
            timing_claim=False,server=False,product=False),"Main boot corpus changed")
        owned=counts(1,0,0,length) if length else counts()
        dropped=counts(deallocations=1,deallocated_bytes=length) if length else counts()
        exact(rows["STAGES"][index],dict(case=name,length=length,samples_per_arm=1,not_pooled=True,frame=counts(),
            cloned=owned,dropped=dropped,whole_returning=owned,whole_drop=dropped),"Separate boot stage changed")
    exact(rows["FAULTS"][0],dict(masks=16,refused=15,refused_allocations=0,first_error_prefix=True,model_only=True),"Fault prefix changed")
    exact(rows["SEEDED"][0],dict(seed="0x074b0a19",samples=256,owned_after_origin_drop=True,
          independent_point_clone=True,model_only=True),"Seeded ownership changed")
    exact(rows["DECODER"][0],dict(valid=1,invalid_counts=5,invalid_bytes=7,model_only=True),"Decoder shape changed")
    passed=re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$","\n".join(cleaned))
    expected=["counter::counter_nested_unwind_and_thread_isolation","counter::counter_zeroed_reallocation_and_drop_forwarding"]
    expected += ["model::controls::"+name for name in (
        "sample_boot_all_fault_masks_keep_first_error_and_no_clone","sample_boot_decoder_preserves_original_bounded_shape",
        "sample_boot_main_owned_sample_requests","sample_boot_seeded_owned_points_getters_and_point_clones",
        "sample_boot_stage_requests_are_separate_from_main")]
    exact(sorted(passed),sorted(expected),"Exact seven passing tests required")
    need("test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;" in text
         and "test result: FAILED" not in text,"Boot selection failed")
    totals={field:sum(row["counts"][field] for row in rows["MAIN"]) for field in previous.FIELDS}
    exact(totals,counts(2048,0,0,324864,2048,324864),"Same-corpus totals differ")
    fixed=[row for row in rows["MAIN"] if row["case"] in ("fixed","fixed-spare")]
    fixed_totals={field:sum(row["counts"][field] for row in fixed) for field in previous.FIELDS}
    exact(fixed_totals,counts(512,0,0,18432,512,18432),"Fixed-shape subset differs")
    return dict(rows=rows,main_samples=2304,main_totals=totals,
                fixed_shape_samples=512,fixed_shape_subset=fixed_totals,stage_samples_per_arm_separate=9)


def analyze(root=ROOT,checks=None):
    root=Path(root)
    checks=Path(checks) if checks else root / OUTPUT / "first"
    packets=[previous.prior.verify_packet(root / BASE / name,pin) for name,pin in INPUTS.items()]
    need(sum(row["members"] for row in packets)==813,"Prior member total changed")
    need(canonical(root / "tools/proc-document-baseline-074/review.py") ==
         canonical(root / BASE / "proc-document-baseline-20261022/method/review.py"),"Previous reviewer drift")
    old=previous.analyze(root)
    exact(old,load(root / BASE / "proc-document-baseline-20261022/facts.json"),"Prior baseline differs")
    pins=source_pins(root)
    lanes=[]
    for lane,version in (("194","1.94.0"),("188","1.88.0")):
        previous.receipt(checks,lane)
        compiler=(checks / f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
        need(compiler.startswith("rustc "+version+" ") and "host: x86_64-pc-windows-msvc" in compiler,"Wrong compiler/platform")
        lanes.append(dict(lane=lane,compiler=version,platform="windows-msvc",controls=7,
            facts=log_facts((checks / f"{lane}-controls.log").read_text(encoding="utf-8-sig"))))
    previous.receipt(checks,"host")
    canaries=(checks / "host-canaries.log").read_text(encoding="utf-8-sig")
    passed=[int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;",canaries)]
    need(len(passed)==15 and sum(passed)==43 and "test result: FAILED" not in canaries,"Wrong source canary cohort")
    exact(lanes[0]["facts"],lanes[1]["facts"],"Windows compiler results differ")
    return dict(schema="pure-sample-boot-baseline-074-v1",baseline_runtime_source="79dcf64bee22d801dc983ef62b4c2fefa44a0988",
        input_packets=packets,input_members_verified=813,probe_source_pins=pins,lanes=lanes,source_canaries=43,
        original_windows_seconds=old["original_windows_seconds"],first_refusals=old["first_refusals"],
        qualification_sha256=old["qualification_sha256"],
        interpretation=dict(model_not_actual_kernel_frame=True,windows_not_gnu_linux=True,
            generic_cases_are_not_admitted_boots=True,synthetic_weights_are_not_production_frequencies=True,
            stage_and_subset_not_added_to_main=True,allocation_counts_not_full_path_timing=True,
            first_error_masks_are_not_real_kernel_faults=True,old_call_counts_not_extrapolated=True),
        allocation_baseline_runs=2,original_owner_profiles=0,runtime_candidate_implemented=False,
        representation_changed=False,full_path_speedup_proven=False,original_material_gate_passed=False,
        full_sigkill_matrix=False,clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve()==(ROOT / OUTPUT / "facts.json").resolve(),"Separate boot baseline facts required")
    for node in (Path(path),*Path(path).parents):
        need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(),"Linked output")


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true")
    parser.add_argument("--packet-sha256")
    args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify,"Packet review must be read-only")
        previous.prior.verify_packet(args.output.parent,args.packet_sha256)
    publish(args.output,analyze(),args.verify)
    print("Pure boot baseline verified: two first Windows lanes, 14 prior seals/813 members; no representation or live change")
