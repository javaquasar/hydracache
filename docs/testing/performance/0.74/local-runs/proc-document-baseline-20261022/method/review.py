"""Read-only baseline reconstruction from sealed history and first Windows controls."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/offline-reserved-reconciliation-074"))
import reconcile as prior

need, load, sha = prior.need, prior.load, prior.sha
BASE = prior.BASE
OUTPUT = BASE / "proc-document-baseline-20261022"
INPUTS = dict(prior.PACKETS, **{
    "reserved-reconciliation-20261022": "d3aa773d6254d5297d303cfc8ba7a397e06ae8a2d4978e39e29c496062b3fb72",
    "custody-preparation-cost-20261020": "3194244cb4e45c6f4f611b946afaa6a6a8b3aaf83179457c260d0a9e25f93864",
})
PROBE_PINS = {
    "tools/proc-document-baseline-074/Cargo.toml": "034ae9f79aea88740564505442004356cb0fee97b24088e58f8147d3faac5068",
    "tools/proc-document-baseline-074/Cargo.lock": "7e9b6f7aa46330f48dcdb70ddb0e40f0bedaa06df77f67a6efcdcc1894a9249e",
    "tools/proc-document-baseline-074/probe.rs": "08bef801e96c2930e553be87a4ff47da50ca7621fa7d24de42991ce6c486c838",
    "tools/proc-document-baseline-074/model.rs": "2ef53a304eb1c5c15ad04db6450b14f75741568b0ddc210cfb64c90332c5d771",
    "tools/proc-document-baseline-074/controls.rs": "d7ea4d90c48955f423ca32bbc9ef622e72b3b9f5ceceab9fbceeb8fcea43058e",
    "tools/proc-document-baseline-074/counter.rs": "2fb9539593499aaf1618f7be10f1452fbdbedc555fbf6f0a9d4f7955ab184d01",
    "crates/xtask/tests/proc_document_baseline_074.rs": "4dcbe683165f18f30c5d20d01d130234e3bdd9d709d560de1cf01077b21b9773",
}
KINDS = ("MAIN", "ERRORS", "SEEDED", "STAGES")
FIELDS = ("allocations", "zeroed", "reallocations", "requested_bytes", "deallocations", "deallocated_bytes")


def exact(actual, expected, label):
    # Distinguish bool/int and float/int as well as values and membership.
    need(json.dumps(actual, sort_keys=True) == json.dumps(expected, sort_keys=True), label)


def counts(allocations=0, zeroed=0, reallocations=0, requested_bytes=0, deallocations=0, deallocated_bytes=0):
    return dict(zip(FIELDS, (allocations, zeroed, reallocations, requested_bytes, deallocations, deallocated_bytes)))


def log_facts(text):
    rows = {}
    for line in text.splitlines():
        marker = "HC074-PROC-DOCUMENT-"
        if marker not in line:
            continue
        kind, payload = line.split(marker, 1)[1].split(" ", 1)
        need(kind in KINDS and kind not in rows, "Unknown/duplicate control row")
        rows[kind] = json.loads(payload)
    need(set(rows) == set(KINDS), "Missing control row")
    expected_main = dict(instances=256, reads_per_instance=4, lengths=[16,37,4096,128], callbacks=1024,
        counts=counts(256,256,0,16777472,256,16777472), model_only=True, timing_claim=False, server=False, product=False)
    exact(rows["MAIN"], expected_main, "Main corpus/counts changed")
    exact(rows["ERRORS"], dict(error_kind_cases=3,raw_error_cases=3,retry=False,model_only=True), "Error contract changed")
    exact(rows["SEEDED"], dict(seed="0x074d5a18",samples=256,callbacks=256,model_only=True), "Seeded corpus changed")
    stage = rows["STAGES"]
    exact(stage, dict(instances_per_arm=1,not_pooled=True,created=counts(1,1,0,65537),reused=counts(),
          dropped=counts(deallocations=1,deallocated_bytes=65537),complete=counts(1,1,0,65537,1,65537)), "Stage counts changed")
    for field in FIELDS:
        need(sum(stage[name][field] for name in ("created","reused","dropped")) == stage["complete"][field], "Stage conservation failed")
    passed = re.findall(r"(?m)^test (\S+) \.\.\. (?:HC074[^\n]*\n)?ok\s*$", text)
    expected_tests = ["counter::counter_nested_unwind_and_thread_isolation", "counter::counter_zeroed_reallocation_and_drop_forwarding"]
    expected_tests += ["model::controls::" + name for name in (
        "proc_document_constructor_and_four_reads_requests", "proc_document_errors_are_propagated_once_without_retry_or_view",
        "proc_document_independent_ownership_and_long_to_short_tail", "proc_document_seeded_visible_prefix_and_single_callback",
        "proc_document_separate_stages_conserve_one_instance")]
    exact(sorted(passed), sorted(expected_tests), "Exact seven passing tests required")
    need("test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;" in text
         and "test result: FAILED" not in text, "Baseline test selection failed")
    return rows


def receipt(directory, lane):
    directory = Path(directory)
    row = load(directory / f"{lane}-result.json")
    need(row["lane"] == lane and row["cohort"] == "first" and row["failure"] is None
         and row["platform"] == "windows-msvc" and row["model_only"] is True, "Wrong first Windows receipt")
    for field in ("runtime_candidate_implemented", "ignored_tests_run", "server", "product"):
        need(row[field] is False, "Receipt promotion forbidden")
    exact(row["original_owner_profiles"], 0, "Live profile forbidden")
    names = ["canaries","check","clippy"] if lane == "host" else ["compiler","controls","check","clippy"]
    exact([step["name"] for step in row["steps"]], names, "Receipt steps changed")
    for step in row["steps"]:
        exact(step["exit"], 0, "Nonzero/invalid exit")
        need(step["log"] == f"{lane}-{step['name']}.log", "Unsafe/wrong receipt log")
        need(sha(directory / step["log"]) == step["sha256"], "Check log drift")
    return row


def source_pins(root):
    for name, pin in PROBE_PINS.items():
        text = prior.canonical(root / name)
        need(hashlib.sha256(text.encode("utf-8")).hexdigest() == pin, f"Probe source drift: {name}")
    return PROBE_PINS


def analyze(root=ROOT, checks=None):
    root = Path(root)
    checks = Path(checks) if checks else root / OUTPUT / "first"
    packets = [prior.verify_packet(root / BASE / name, pin) for name, pin in INPUTS.items()]
    need(sum(packet["members"] for packet in packets) == 770, "Prior member total changed")
    old = prior.analyze(root)
    exact(old, load(root / BASE / "reserved-reconciliation-20261022/facts.json"), "Prior reconciliation differs")
    pins = source_pins(root)
    lanes = []
    for lane, version in (("194","1.94.0"),("188","1.88.0")):
        receipt(checks,lane)
        compiler = (checks / f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
        need(compiler.startswith("rustc " + version + " ") and "host: x86_64-pc-windows-msvc" in compiler, "Wrong compiler/platform")
        facts = log_facts((checks / f"{lane}-controls.log").read_text(encoding="utf-8-sig"))
        lanes.append(dict(lane=lane,compiler=version,platform="windows-msvc",controls=7,rows=facts))
    receipt(checks,"host")
    canaries = (checks / "host-canaries.log").read_text(encoding="utf-8-sig")
    passed = [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;",canaries)]
    need(len(passed) == 14 and sum(passed) == 40 and "test result: FAILED" not in canaries, "Wrong source canary cohort")
    exact(lanes[0]["rows"],lanes[1]["rows"],"Windows compiler results differ")
    return dict(schema="pure-proc-document-baseline-074-v1", baseline_runtime_source="5310113d3a104b97f09325b4ac7f89c7f85263c4",
        input_packets=packets,input_members_verified=770,probe_source_pins=pins,lanes=lanes,source_canaries=40,
        original_windows_seconds=old["original_windows_seconds"],
        first_refusals=[row["first_original_refusal"] for row in old["old_reserved_live_lanes"]],
        qualification_sha256=old["qualification_sha256"],
        interpretation=dict(model_not_actual_kernel_helper=True,windows_not_gnu_linux=True,
            old_sender_reuse_not_a_new_optimization=True,stage_controls_not_pooled=True,
            requested_zeroed_bytes_not_physical_zeroing=True,old_call_counts_not_extrapolated=True),
        allocation_baseline_runs=2,original_owner_profiles=0,runtime_candidate_implemented=False,
        full_path_speedup_proven=False,original_material_gate_passed=False,full_sigkill_matrix=False,
        clocks_changed=False,services_changed=False,server=False,product=False)


def require_output(path):
    need(Path(path).resolve() == (ROOT / OUTPUT / "facts.json").resolve(), "Separate baseline facts required")
    for node in (Path(path), *Path(path).parents):
        need(not node.is_symlink() and not getattr(node,"is_junction",lambda:False)(), "Linked output")


if __name__ == "__main__":
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output",type=Path,required=True)
    parser.add_argument("--verify",action="store_true")
    parser.add_argument("--packet-sha256")
    args=parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify,"Packet review must be read-only")
        prior.verify_packet(args.output.parent,args.packet_sha256)
    prior.prior.rounds.prior.publish(args.output,analyze(),args.verify)
    print("Pure proc baseline verified: two first Windows lanes, 13 prior seals/770 members, no runtime candidate or live profile")
