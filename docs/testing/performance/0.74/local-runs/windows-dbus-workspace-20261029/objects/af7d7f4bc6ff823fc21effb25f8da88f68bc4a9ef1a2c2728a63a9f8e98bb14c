"""Reconstruct local operation probes; never subtract observer clocks or retry."""
import argparse
import hashlib
import json
from pathlib import Path
from statistics import median

CASES = ("actual","traced","open","filesystem","read","decode","close","empty")
JOBS = ("boot","phases","leaf","names","owners","full")


def require(ok, reason):
    if not ok:
        raise ValueError(reason)


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def number(value):
    require(type(value) is int and 0 <= value < 2**128, "Expected nonnegative integer")
    return value


def scope(row):
    for key in ("server","product","native_gate_open","overhead_subtracted","allocator_active"):
        if key in row:
            require(row[key] is False, f"Scope changed: {key}")


def rows(path, marker):
    return [json.loads(line.split(marker,1)[1]) for line in Path(path).read_text().splitlines() if marker in line]


def stats(values):
    require(bool(values) and all(type(v) in (int,float) for v in values), "Empty/nonnumeric observations")
    return dict(count=len(values), median=median(values), minimum=min(values), maximum=max(values))


def boot_facts(data):
    require(len(data)==128,"Boot observations incomplete")
    grouped={case:[] for case in CASES}
    for index,row in enumerate(data):
        scope(row)
        sample,slot=divmod(index,8)
        case_index=(sample+slot)%8 if sample<8 else (sample-8+8-slot)%8
        require(number(row["sample"])==sample and number(row["slot"])==slot and row["case"]==CASES[case_index]
                and row["iterations"]==1024,"Boot schedule changed")
        calls=0 if case_index<2 else 4096 if case_index in (2,3,6) else 1024
        require(number(row["selected_calls"])==calls,"Boot window count changed")
        outer,selected=number(row["outer_cpu_ns"]),number(row["selected_cpu_ns"])
        require(outer>0 and selected<=outer and (calls or selected==0),"Invalid Boot CPU")
        grouped[row["case"]].append((outer/1024,selected/calls if calls else None))
    return {key:dict(outer_ns_per_boot=stats([r[0] for r in values]),
                     selected_ns_per_window=stats([r[1] for r in values]) if values[0][1] is not None else None)
            for key,values in grouped.items()}


def timed_facts(data, kind):
    if kind=="phases":
        cases=("full","directory","inventory","records","markers","bind","empty")
        groups=[(depth,case) for depth in (1,7) for case in cases]
        expected=196
    else:
        cases=("actual","traced","held-metadata","leaf-open","leaf-before","parent-metadata","buffer","read","after","empty")
        groups=[(depth,leaf,case) for depth in (1,7) for leaf in ("record","marker") for case in cases]
        expected=800
    require(len(data)==expected,"Authority CPU observations incomplete")
    grouped={group:[] for group in groups}
    seen=set()
    for row in data:
        scope(row)
        depth=number(row["generations"])
        group=(depth,row["phase"]) if kind=="phases" else (depth,row["kind"],row["case"])
        require(group in grouped,"Unexpected authority case")
        sample=number(row["sample"]);position=number(row["position"])
        n=len(cases);count=14 if kind=="phases" else 20
        require(sample<count and position<n and row["iterations"]==(2048 if kind=="phases" else 1024),"Authority schedule changed")
        index=(sample+position)%n if sample<n else (sample-n+n-position)%n
        require(group[-1]==cases[index] and (group,sample) not in seen,"Duplicate/reordered authority sample")
        seen.add((group,sample))
        outer=number(row["cpu_ns"] if kind=="phases" else row["outer_ns"])
        selected=None
        if kind!="phases":
            calls=number(row["calls"])
            expected_calls=0 if group[-1] in ("actual","traced") else depth*1024
            require(calls==expected_calls and row["selected_clock_reads"]==2*calls,"Authority window count changed")
            selected=number(row["selected_ns"])
            require(selected<=outer and (calls or selected==0),"Invalid authority selected CPU")
            selected=selected/calls if calls else None
        require(outer>0,"Missing authority CPU")
        grouped[group].append((outer/row["iterations"],selected))
    return [{"case":list(group),"outer_ns_per_phase":stats([r[0] for r in values]),
             "selected_ns_per_leaf":stats([r[1] for r in values]) if values[0][1] is not None else None}
            for group,values in grouped.items()]


def counts(row):
    require(set(row)=={"allocations","reallocations","requested_bytes"},"Count shape changed")
    return {k:number(v) for k,v in row.items()}


def stage(packet, label):
    metadata=load(packet/f"{label}-metadata.json")
    require(metadata==load(packet/f"{label}-postflight.json") and not metadata["dirty"] and metadata["uid"]>0,"Session drift")
    result=load(packet/f"{label}-result.json")
    scope(result)
    require(result["complete"] is True and result["failure"] is None and result["source"]==metadata["source"],"Session incomplete")
    require([(r["lane"],r["job"]) for r in result["results"]]==[(lane,job) for lane in ("194","188") for job in JOBS],"Session schedule changed")
    lanes={}
    for lane in ("194","188"):
        for job in JOBS:
            receipt=load(packet/f"{label}-{lane}-{job}.json")
            require(receipt==next(r for r in result["results"] if r["lane"]==lane and r["job"]==job),"Receipt changed")
            log=packet/f"{label}-{lane}-{job}.log"
            require(receipt["exit"]==0 and receipt["containment_failure"] is None and receipt["log_sha256"]==sha(log)
                    and "1 passed; 0 failed; 0 ignored" in log.read_text(),"Probe incomplete/changed")
        path=lambda job:packet/f"{label}-{lane}-{job}.log"
        boot=boot_facts(rows(path("boot"),"HC074-BOOT-OPS "))
        phases=timed_facts(rows(path("phases"),"HC074-AUTHORITY-CPU "),"phases")
        leaf=timed_facts(rows(path("leaf"),"HC074-LEAF-CPU "),"leaf")
        alloc=rows(path("names"),"HC074-OPEN-NAME-ALLOC ")
        paired=rows(path("names"),"HC074-OPEN-NAME-CPU ")
        full=rows(path("full"),"HC074-VALIDATION-LEAF-ALLOC ")
        io=rows(path("full"),"HC074-VALIDATION-LEAF-IO ")
        owners=rows(path("owners"),"HC074-AUTHORITY-OWNERS ")
        require(len(alloc)==18 and len(paired)==120 and len(full)==len(io)==24 and len(owners)==54,"Request observations incomplete")
        for observations in (alloc,full,io,owners):
            for row in observations:
                scope(row)
                require(row["iterations"]==1024 and number(row["sample"])<3,"Request schedule changed")
        directory=[]
        for index,row in enumerate(alloc):
            case,sample=divmod(index,3)
            require(row["case"]==case and row["sample"]==sample and row["order"]==([0,1] if sample%2==0 else [1,0]),"Helper order changed")
            counts(row["reference"]);counts(row["actual"])
            if case==5:directory.append(row["actual"])
        helper_cpu={case:[] for case in range(6)}
        for index,row in enumerate(paired):
            case,sample=divmod(index,20)
            scope(row)
            require(row["case"]==case and row["sample"]==sample and row["iterations"]==1024 and
                    row["order"]==([0,1] if sample%2==0 else [1,0]),"Helper CPU schedule changed")
            reference,actual=number(row["reference_ns"]),number(row["actual_ns"])
            require(type(row["difference_ns"]) is int and row["difference_ns"]==actual-reference,"Helper CPU difference changed")
            helper_cpu[case].append((actual-reference)/1024)
        normalized=[]
        for row in full:
            count=counts(row["counts"])
            require(all(v%1024==0 for v in count.values()),"Fractional request count")
            normalized.append(dict(case=row["case"],sample=row["sample"],counts={k:v//1024 for k,v in count.items()}))
        reads=[]
        for row in io:
            require(row["empty"]["syscr"]==1 and row["empty"]["syscw"]==row["observed"]["syscw"]==row["observed"]["wchar"]==0,"Read/write control changed")
            for value in row["kernel_calls"]:number(value)
            reads.append(dict(case=row["case"],sample=row["sample"],
                read_requests=number(row["observed"]["syscr"])-number(row["empty"]["syscr"]),kernel_calls=row["kernel_calls"]))
        lanes[lane]=dict(boot=boot,authority_phases=phases,authority_leaf=leaf,directory_helper_allocations=directory,
            full_counts=normalized,full_io=reads,helper_allocations=alloc,owners=owners,
            helper_cpu_differences_ns_per_call={str(case):stats(values) for case,values in helper_cpu.items()},
            helper_fixtures=rows(path("names"),"HC074-OPEN-NAME-FIXTURE "),
            cold_warm=rows(path("names"),"HC074-OPEN-NAME-LEAF "),
            owners_fixtures=rows(path("owners"),"HC074-AUTHORITY-OWNERS-FIXTURE "))
    return dict(source=metadata["source"],boot=metadata["boot"],lanes=lanes)


def compare(baseline, candidate):
    require(baseline["helper_fixtures"]==candidate["helper_fixtures"],"Helper workload drift")
    require(baseline["cold_warm"]==candidate["cold_warm"],"Cold/warm file control drift")
    def identities(lane):
        return [{k:v for k,v in row.items() if k!="leaf_fd"} for row in lane["owners_fixtures"]]
    require(identities(baseline)==identities(candidate),"Authority payload/epoch identity drift")
    require(baseline["full_io"]==candidate["full_io"],"Fresh read/kernel-call drift")
    full=[]
    for before,after in zip(baseline["full_counts"],candidate["full_counts"]):
        require((before["case"],before["sample"])==(after["case"],after["sample"]),"Full request order drift")
        expected=dict(before["counts"])
        if before["case"] in ("authority-1","authority-7"):
            expected["allocations"]-=2;expected["requested_bytes"]-=8
        require(after["counts"]==expected,"Full request target missed")
        full.append(dict(case=before["case"],sample=before["sample"],before=before["counts"],after=after["counts"]))
    for before,after in zip(baseline["helper_allocations"],candidate["helper_allocations"]):
        expected=json.loads(json.dumps(before))
        if before["case"]==5:expected["actual"]=dict(allocations=0,reallocations=0,requested_bytes=0)
        require(expected==after,"Unrelated helper request drift")
    require(baseline["directory_helper_allocations"]==[dict(allocations=1024,reallocations=0,requested_bytes=4096)]*3,
            "Directory baseline not demonstrated")
    for before,after in zip(baseline["owners"],candidate["owners"]):
        require((before["generations"],before["phase"],before["sample"])==(after["generations"],after["phase"],after["sample"]),"Owner schedule drift")
        expected=dict(before["counts"])
        multiplier=2 if before["phase"]=="full" else 1 if before["phase"]=="directory" else 0
        expected["allocations"]-=multiplier*1024;expected["requested_bytes"]-=multiplier*4096
        require(after["counts"]==expected and before["sites"]==after["sites"],"Owner request/site drift")
        require(before["observed"]["syscr"]-before["empty"]["syscr"]==after["observed"]["syscr"]-after["empty"]["syscr"],"Owner read drift")
    return dict(allocation_reduction_verified=True,full_counts=full,
        directory_before=baseline["directory_helper_allocations"],directory_after=candidate["directory_helper_allocations"],
        cpu_speedup_claim=False,cpu_nonregression_proven=False)


def analyze(packet, baseline_only=False):
    stages={label:stage(packet,label) for label in (("Baseline",) if baseline_only else ("Baseline","Candidate"))}
    comparison=None if baseline_only else {lane:compare(stages["Baseline"]["lanes"][lane],stages["Candidate"]["lanes"][lane]) for lane in ("194","188")}
    return dict(schema="boot-authority-operations-074-v1",stages=stages,comparison=comparison,overhead_subtracted=False,
        cpu_speedup_claim=False,exclusive_cross_observer_cpu_known=False,native_gate_open=False,server=False,product=False)


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet",type=Path,required=True)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--baseline-only",action="store_true")
    parser.add_argument("--verify",action="store_true")
    args=parser.parse_args()
    facts=analyze(args.packet,args.baseline_only)
    if args.verify:
        require(json.dumps(facts,sort_keys=True)==json.dumps(load(args.output),sort_keys=True),"Derived facts changed")
    else:
        with args.output.open("x",encoding="utf-8",newline="\n") as stream:
            json.dump(facts,stream,indent=2,sort_keys=True);stream.write("\n")
    print("operation probes reconstructed; observer overhead not subtracted; native admission=false")


if __name__=="__main__":
    main()
