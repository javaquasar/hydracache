"""Separate recorded fresh kernel spans from authority; never invoke a profile."""
import argparse
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location("partial", Path(__file__).with_name("attribute_partial.py"))
partial = importlib.util.module_from_spec(spec)
spec.loader.exec_module(partial)


def separate(raw, index):
    step = partial.step_facts(raw, index)
    nodes = step["kernel"]
    boot, namespace, document = (nodes[i] for i in (5, 6, 9))
    # The actual Origin calls Boot twice and opens four fresh namespace handles.
    # Reject a changed topology even if aggregate count/time conservation holds.
    incoming_boot = [e for e in raw["edges"] if e["child"] == 5]
    incoming_namespace = [e for e in raw["edges"] if e["child"] == 6]
    for incoming, node in ((incoming_boot, boot), (incoming_namespace, namespace)):
        partial.require(not node["calls"] or
                        len(incoming) == 1 and incoming[0]["parent"] == 4,
                        "Fresh leaf parent changed")
    children = [e for e in raw["edges"] if e["parent"] == 5]
    partial.require(len(children) == (1 if boot["calls"] else 0), "Boot children changed")
    boot_document = children[0] if children else dict(child=9, calls=0, inclusive_ns=0)
    partial.require(boot_document["child"] == 9 and boot_document["calls"] == boot["calls"],
                    "Boot document count changed")
    partial.require(not any(e["parent"] in (6, 9) for e in raw["edges"]),
                    "Namespace or Document is no longer a leaf")
    partial.require(not raw["roots"][5] and not raw["roots"][6], "Fresh leaf root changed")
    partial.require(boot["inclusive_ns"] == boot["outside_instrumented_children_ns"] +
                    boot_document["inclusive_ns"], "Boot document wall changed")
    partial.require(boot_document["inclusive_ns"] <= document["inclusive_ns"],
                    "Boot document exceeds all Document spans")
    total = step["kernel_outside_children_total_ns"]
    selected = boot["inclusive_ns"] + namespace["inclusive_ns"]
    partial.require(selected <= total, "Fresh partition exceeds kernel")
    authority = step["live"][3]
    # Current active() has no nested live span. A changed live topology needs a
    # new method, not automatic conversion of inclusive time to exclusive CPU.
    partial.require(authority["inclusive_ns"] == authority["outside_instrumented_children_ns"],
                    "Authority live children changed")
    partial.require(authority["inclusive_ns"] <= step["wall_ns"], "Authority exceeds step")
    return dict(step=step["step"], calls=step["calls"], completed=step["completed"],
                wall_ns=step["wall_ns"], boot_calls=boot["calls"],
                boot_inclusive_ns=boot["inclusive_ns"],
                boot_outside_document_ns=boot["outside_instrumented_children_ns"],
                boot_document_calls=boot_document["calls"], boot_document_ns=boot_document["inclusive_ns"],
                namespace_calls=namespace["calls"], namespace_ns=namespace["inclusive_ns"],
                other_document_ns=document["inclusive_ns"]-boot_document["inclusive_ns"],
                other_kernel_ns=total-selected, kernel_partition_ns=total,
                outside_kernel_ns=step["wall_outside_instrumented_kernel_ns"],
                authority_calls=authority["calls"], authority_ns=authority["inclusive_ns"],
                authentication_calls=step["authentication_calls"], authentication_ns=step["authentication_ns"])


def analyze(packet):
    original = partial.analyze(packet)  # Includes inventory, refusal, prefix and conservation checks.
    lanes = []
    for lane in original["lanes"]:
        raw = partial.rows(packet / f"{lane['lane']}-Profile-0.log", "HC074-HELD-PHASE ")
        phases = []
        for phase, verified in zip(raw, lane["phases"]):
            steps = [separate(row, i) for i, row in enumerate(phase["round_steps"])]
            phases.append(dict(phase=phase["phase"], returned=verified["returned"],
                               phase_wall_ns=verified["wall_ns"],
                               parent_process_cpu_ns=verified["parent_process_cpu_ns"], steps=steps))
        lanes.append(dict(lane=lane["lane"], original_refusal=lane["original_refusal"], phases=phases))
    return dict(schema="fresh-kernel-authority-attribution-074-v1", measured_source=partial.SOURCE,
                input_inventory_sha256=partial.INVENTORY, analyzer_sha256=partial.sha(__file__),
                parent_analyzer_sha256=partial.sha(partial.__file__), lanes=lanes,
                interpretation=dict(samples_per_lane=1, cost_unit="instrumented wall nanoseconds",
                    kernel_partition_disjoint=True, authority_kernel_intersection="unrecorded",
                    authority_authentication_intersection="unrecorded",
                    authority_is_crypto=False, per_syscall_cost_known=False,
                    exclusive_cpu_known=False, overhead_subtracted=False, timing_speedup_claim=False,
                    complete_profile_cohort=False, original_windows_seconds=[300,60,10]),
                new_profile_invocations=0, runtime_changed=False, observer_changed=False,
                native_gate_open=False, server=False, product=False)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    facts = analyze(args.input)
    if args.verify:
        # Python structural equality equates False with 0; a release gate must
        # retain its actual JSON type, not merely its truth value.
        partial.require(json.dumps(partial.load(args.output), sort_keys=True) ==
                        json.dumps(facts, sort_keys=True), "Derived facts changed")
    else:
        partial.publish(args.output, facts)
    print("fresh Boot/Document/Namespace partition verified; authority overlap unrecorded; no new profiles")


if __name__ == "__main__":
    main()
