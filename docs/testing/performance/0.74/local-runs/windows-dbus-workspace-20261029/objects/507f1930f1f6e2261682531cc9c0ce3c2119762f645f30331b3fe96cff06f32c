"""Read-only reconstruction of original material gate; no fixture invocation."""
import argparse
import copy
from pathlib import Path
import attribute_partial as prior
import run_material_live as method
import runner as base

STAGES = (
    "projection-enter", "projection-return",
    "material-return descriptors=20 original_bounds=true",
    "revalidate-return original_bounds=true",
    "drop-return original_send=false completion=false",
)
FOOTER = ("HC074-HELD-LIVE case=healthy seed=0x074c051e actual_fixed=true actual_consuming=true "
          "original_bounds=true eof=true fence_retained=true custodians_terminal=true synthetic_images=true "
          "original_send=false completion=false server=false product=false")


def profile_facts(log, receipt, metadata):
    require = prior.require
    require(type(receipt["exit"]) is int and receipt["exit"] in (0,101)
            and receipt["containment_failure"] is None, "Not an original Rust result")
    require(receipt["log_sha256"] == base.sha(log) and receipt["bytes"] == log.stat().st_size, "Raw log drift")
    rows = prior.rows(log,"HC074-HELD-PHASE ")
    expected = prior.PHASES+("projection-materialization",)
    require(tuple(r["phase"] for r in rows) == expected[:len(rows)] and len(rows)<=len(expected), "Phase prefix drift")
    originals = prior.rows(log,"HC074-HELD-LIVE-ORIGINAL ")
    require(len(originals)==1, "Missing/duplicate original result")
    original=originals[0]
    prior.flags(original,false=("guard_clock_added","deadline_changed","server","product"))
    passed = receipt["exit"]==0
    require(original["case"]=="healthy" and original["returned"] is passed, "First original outcome drift")
    require(all(r["returned"] is True for r in rows[:-1]), "Continued after refusal")
    text = log.read_text(encoding="utf-8-sig")
    # libtest can prefix the first emitted line with its test name.
    stages=[line.split("HC074-HELD-MATERIAL stage=",1)[1] for line in text.splitlines() if "HC074-HELD-MATERIAL stage=" in line]
    require(tuple(stages)==STAGES[:len(stages)] and len(stages)<=len(STAGES), "Stage prefix drift")
    footers=[line for line in text.splitlines() if line.startswith("HC074-HELD-LIVE ")]
    require(footers==([FOOTER] if passed else []), "Premature/foreign/missing cleanup footer")
    if passed:
        require(len(rows)==11 and all(r["returned"] is True for r in rows) and tuple(stages)==STAGES, "Incomplete material success")
        require(original["causal"] and original["causal"]["first_refusal"] is None, "Erased original refusal")
        require(original["kernel"] is None, "Erased original kernel refusal")
    if stages:
        require(len(rows)==11, "Stage entered outside final phase")
    phases=[]
    for row in rows:
        projected=copy.deepcopy(row)
        if row["phase"]=="outputs" and not row["returned"]: projected["phase"]="unfinished-outputs"
        fact=prior.phase_facts(projected);fact["phase"]=row["phase"];phases.append(fact)
    causal=original["causal"]
    if causal:
        origin=causal["first"]["issued_ns"]
        for position in ("first","last","first_refusal"):
            row=causal.get(position)
            if row is None: continue
            require(row["issued_boot"]==row["now_boot"]==metadata["boot"] and row["issued_ns"]==origin
                    and type(row["age_ns"]) is int and row["now_ns"]-row["issued_ns"]==row["age_ns"], "Causal origin/boot/arithmetic drift")
            if passed: require(row["diagnostic_reason"]=="within-window" and 0<=row["age_ns"]<10_000_000_000, "Causal window drift")
        if passed:
            for site in original["age"]["sites"]:
                require(site["first_refusal"] is None,"Hidden live refusal")
                if site["calls"]:
                    for position in ("first","last"):
                        row=site[position]
                        require(row["reason"]=="within-window" and row["issued_ns"]==origin
                            and type(row["age_ns"]) is int and row["now_ns"]-row["issued_ns"]==row["age_ns"]
                            and 0<=row["age_ns"]<10_000_000_000,"Live window drift")
    return dict(lane=receipt["lane"],harness_passed=passed,stages=stages,original=original,phases=phases,
                child_wall_seconds=receipt["wall_seconds"],cleanup_assertions_passed=passed)


def analyze(packet,source):
    require=prior.require
    metadata,post,intent,result=(base.read_json(packet/n) for n in ("metadata.json","postflight.json","intent.json","result.json"))
    require(metadata==post and all(r["source"]==source for r in (metadata,intent,result)),"Source/context drift")
    require(result["failure"] is None,"Incomplete context: retain raw evidence")
    require(metadata["qualification_sha256"]==base.QUALIFICATION and metadata["uid"]==0 and not metadata["fixtures"]
            and not metadata["tracked_dirty"],"Scope/qualification drift")
    require(intent["schedule"]==list(base.LANES) and type(intent["calls_per_lane"]) is int and intent["calls_per_lane"]==1
            and intent["original_windows_seconds"]==[300,60,10] and intent["seed"]=="0x074c051e"
            and intent["child_timeout_seconds"]==1200 and intent["kill_after_seconds"]==5
            and intent["child_containment_seconds"]==base.CHILD_SECONDS and intent["session_seconds"]==base.SESSION_SECONDS
            and intent["max_log_bytes"]==base.MAX_LOG_BYTES,"Schedule/bounds drift")
    for row in (intent,result): prior.flags(row,false=("server","product","original_send","native_gate_open"))
    prior.flags(result,false=("full_matrix",))
    require(metadata["material_method"]=={n:base.sha(Path(__file__).parent/n) for n in method.METHOD},"Method drift")
    profiles=[];receipts=[]
    for lane in base.LANES:
        require(base.read_json(packet/f"{lane}-before.json")==metadata==base.read_json(packet/f"{lane}-after.json"),"Lane context drift")
        receipt=base.read_json(packet/f"{lane}.exit.json")
        require(receipt["lane"]==lane,"Lane order drift")
        child=base.read_json(packet/f"{lane}.intent.json")
        require(child["lane"]==lane and child["case"]=="healthy" and child["material_mode"]=="original-once-v1","Child mode drift")
        prior.flags(child,false=("server","product","original_send","native_gate_open"))
        receipts.append(receipt);profiles.append(profile_facts(packet/f"{lane}.log",receipt,metadata))
    require(result["profiles"]==receipts and result["schedule_completed"] is all(p["harness_passed"] for p in profiles),"Receipt/completion drift")
    return dict(schema="original-held-material-review-074-v1",source=source,metadata=metadata,profiles=profiles,
        original_material_gate_passed=all(p["harness_passed"] for p in profiles),full_sigkill_matrix=False,
        original_delivery_proven=False,all_production_writers_proven=False,cpu_speedup_claim=False,**base.SCOPE)


if __name__=="__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source",required=True)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true")
    args=parser.parse_args()
    packet=Path(__file__).resolve().parents[2]/"docs/testing/performance/0.74/local-runs"/method.NAME
    prior.publish(args.output,analyze(packet,args.source),args.verify)
    print("First original material outcomes reconstructed; no server admission")
