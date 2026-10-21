"""Offline attribution of the two sealed refused prefixes; never runs a fixture."""

import argparse
import hashlib
import json
from pathlib import Path


PACKET = "single-session-rounds-bound-20261021"
INVENTORY = "91a6e589235f2552ae9335680f47d4a20a418efeefcc3bc9fd2a05ba96f12afe"
SOURCE = "c1aa639990c4d2434a896ab7e90d93a0f8ce4b8a"
PHASES = ("prepare-consent", "accept-consent", "outputs", "enrollment-frame",
          "fixed-manager-message", "owned-root-preparation", "enrollment",
          "placement", "fixed-slice", "consuming")
STEPS = ("StorageBefore", "ReservedLive", "Progress", "Causal", "Prepared",
         "Consent", "StorageAfter", "FinalClocks")
KERNEL = ("PacketContext", "Sender", "Alive", "Projection", "Origin", "Boot",
          "Namespace", "Sample", "WorkerOrigin", "Document")
LIVE = ("KernelWorker", "WorkerPolicy", "WorkerSource", "ActiveAuthority",
        "KernelContext", "LocalContext")


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def number(value):
    require(type(value) is int and value >= 0, "Expected nonnegative integer")
    return value


def flags(row, true=(), false=()):
    for key in true:
        require(row.get(key) is True, f"Flag must be true: {key}")
    for key in false:
        require(row.get(key) is False, f"Flag must be false: {key}")


def rows(path, marker):
    found = []
    for line in Path(path).read_text(encoding="utf-8-sig").splitlines():
        if marker in line:
            found.append(json.loads(line.split(marker, 1)[1]))
    return found


def verify_inventory(packet):
    require(sha(packet / "inventory.json") == INVENTORY, "Prior inventory pin changed")
    expected = set()
    for row in load(packet / "inventory.json")["files"]:
        relative = Path(row["path"])
        require(not relative.is_absolute() and ".." not in relative.parts, "Unsafe member")
        path = packet / relative
        require(path.resolve().is_relative_to(packet.resolve()) and not path.is_symlink(), "Escaped member")
        require(row["path"] not in expected, "Duplicate member")
        expected.add(row["path"])
        require(path.stat().st_size == row["bytes"] and sha(path) == row["sha256"], "Prior member changed")
    actual = {str(p.relative_to(packet)).replace("\\", "/") for p in packet.rglob("*") if p.is_file()}
    require(actual == expected | {"inventory.json"}, "Prior membership changed")


def named_nodes(nodes, names):
    require(len(nodes) == len(names), "Node shape changed")
    result = []
    for name, row in zip(names, nodes):
        count, inclusive, outside = (number(row[k]) for k in ("calls", "inclusive_ns", "outside_children_ns"))
        require(outside <= inclusive, "Outside span exceeds inclusive")
        require(count > 0 or inclusive == outside == 0, "Time without calls")
        result.append(dict(kind=name, calls=count, inclusive_ns=inclusive,
                           outside_instrumented_children_ns=outside))
    return result


def step_facts(row, index):
    require(row["step"] == index, "Step order changed")
    flags(row, true=("overlapping_spans",), false=("guard_clock_added",))
    calls, completed, wall = (number(row[k]) for k in ("calls", "completed", "wall_ns"))
    require(0 <= calls - completed <= 1, "Invalid unfinished step count")
    kernel = named_nodes(row["kernel"], KERNEL)
    live = named_nodes(row["live"], LIVE)
    require(len(row["roots"]) == len(KERNEL), "Root shape changed")
    roots = [number(v) for v in row["roots"]]
    incoming = [0] * len(KERNEL)
    outgoing_ns = [0] * len(KERNEL)
    pairs = set()
    for edge in row["edges"]:
        parent, child = number(edge["parent"]), number(edge["child"])
        count, duration = number(edge["calls"]), number(edge["inclusive_ns"])
        require(parent < len(KERNEL) and child < len(KERNEL) and count > 0, "Invalid edge")
        require((parent, child) not in pairs, "Duplicate edge")
        pairs.add((parent, child))
        incoming[child] += count
        outgoing_ns[parent] += duration
    for i, node in enumerate(kernel):
        require(node["calls"] == roots[i] + incoming[i], "Kernel count not conserved")
        require(node["inclusive_ns"] == node["outside_instrumented_children_ns"] + outgoing_ns[i],
                "Kernel wall not conserved")
    outside = sum(node["outside_instrumented_children_ns"] for node in kernel)
    require(outside <= wall, "Kernel wall exceeds step")
    return dict(step=STEPS[index], calls=calls, completed=completed, wall_ns=wall,
                kernel_outside_children_total_ns=outside,
                wall_outside_instrumented_kernel_ns=wall-outside, kernel=kernel, live=live,
                authentication_calls=number(row["authentication_calls"]),
                authentication_ns=number(row["authentication_ns"]),
                signature_calls=number(row["signature_calls"]), signature_ns=number(row["signature_ns"]))


def phase_facts(row):
    flags(row, true=("instrumented", "overlapping_spans"), false=("includes_child_cpu",
          "guard_clock_added", "deadline_changed", "healthy_gate", "server", "product"))
    require(type(row["returned"]) is bool and len(row["round_steps"]) == 8, "Invalid phase")
    steps = [step_facts(step, i) for i, step in enumerate(row["round_steps"])]
    for before, after in zip(steps, steps[1:]):
        require(after["calls"] == before["completed"], "Round prefix skips or continues after refusal")
    require(steps[0]["calls"] - steps[-1]["completed"] in (0, 1), "Multiple unfinished rounds")
    require(sum(s["calls"] - s["completed"] for s in steps) <= 1, "Multiple failed steps")
    if row["returned"]:
        require(all(s["calls"] == s["completed"] for s in steps), "Returned phase has unfinished round")
    wall = number(row["wall_ns"])
    cpu = row["process_cpu_ns"]
    if cpu is not None:
        number(cpu)  # Parent process CPU may include other threads, not children.
    step_wall = sum(s["wall_ns"] for s in steps)
    require(step_wall <= wall, "Step wall exceeds phase")
    kernel, live = named_nodes(row["kernel"], KERNEL), named_nodes(row["live"], LIVE)
    require(sum(n["outside_instrumented_children_ns"] for n in kernel) <= wall, "Kernel wall exceeds phase")
    require(sum(n["outside_instrumented_children_ns"] for n in live) <= wall, "Live wall exceeds phase")
    for index, node in enumerate(kernel):
        require(sum(s["kernel"][index]["calls"] for s in steps) <= node["calls"], "Step calls exceed parent")
    result = dict(phase=row["phase"], returned=row["returned"], wall_ns=wall,
                  parent_process_cpu_ns=cpu, round_step_wall_ns=step_wall,
                  wall_outside_round_steps_ns=wall-step_wall, steps=steps,
                  kernel=kernel, live=live, authentication_calls=number(row["authentication_calls"]),
                  authentication_ns=number(row["authentication_ns"]),
                  signature_calls=number(row["signature_calls"]), signature_ns=number(row["signature_ns"]))
    if row["phase"] == "outputs":
        require(row["returned"] and all(s["calls"] == s["completed"] == 46 for s in steps)
                and result["authentication_calls"] == 782, "Original output rounds changed")
        require([n["calls"] for n in kernel] == [8464,19872,39744,39744,76544,153088,306176,9936,8464,232576],
                "Original output call signature changed")
        for i, node in enumerate(kernel):
            require(sum(s["kernel"][i]["calls"] for s in steps) == node["calls"], "Output calls not reconciled")
        for field in ("authentication_calls", "authentication_ns", "signature_calls", "signature_ns"):
            require(sum(s[field] for s in steps) == result[field], "Output auth/signature not reconciled")
    return result


def lane_facts(phase_rows, original, receipt, lane, metadata):
    require(lane in ("194", "188"), "Invalid lane")
    count = 6 if lane == "194" else 7
    require(len(phase_rows) == count and tuple(r["phase"] for r in phase_rows) == PHASES[:count],
            "Expected refused prefix changed")
    require(all(r["returned"] is True for r in phase_rows[:-1]) and phase_rows[-1]["returned"] is False,
            "Original partial phase erased")
    require(receipt["lane"] == lane and receipt["sample"] == 0 and receipt["exit"] == 101
            and receipt["containment_failure"] is None, "Original exit changed")
    flags(original, false=("returned", "deadline_changed", "guard_clock_added", "server", "product"))
    refusal = original["causal"]["first_refusal"]
    require(refusal["diagnostic_reason"] == "expired" and
            refusal["issued_boot"] == refusal["now_boot"] == metadata["boot"], "Original refusal kind/boot changed")
    require(number(refusal["now_ns"]) - number(refusal["issued_ns"]) == number(refusal["age_ns"])
            and refusal["age_ns"] >= 10_000_000_000, "Original expiration arithmetic changed")
    phases = [phase_facts(row) for row in phase_rows]
    failed_step = "Causal" if lane == "194" else "FinalClocks"
    require(refusal["site"] == ("window" if lane == "194" else "final_clocks"), "Refusal site changed")
    unfinished = [s for s in phases[-1]["steps"] if s["calls"] > s["completed"]]
    require(len(unfinished) == 1 and unfinished[0]["step"] == failed_step, "Refusal does not join unfinished step")
    output = phases[2]
    return dict(lane=lane, sample=0, exit=101, original_refusal=refusal, phases=phases,
                observed_phase_wall_total_ns=sum(r["wall_ns"] for r in phases),
                observed_parent_cpu_total_ns=(sum(r["parent_process_cpu_ns"] for r in phases)
                    if all(r["parent_process_cpu_ns"] is not None for r in phases) else None),
                dominant_phase=max(phases, key=lambda r:r["wall_ns"])["phase"],
                output_dominant_step=max(output["steps"], key=lambda s:s["wall_ns"])["step"],
                complete_profile_cohort=False, healthy_gate=False)


def analyze(packet):
    verify_inventory(packet)
    metadata, postflight, intent, result = (load(packet / name) for name in
                                          ("metadata.json", "postflight.json", "intent.json", "result.json"))
    require(metadata == postflight and metadata["source"] == SOURCE and not metadata["fixtures"], "Context changed")
    require(intent["original_windows_seconds"] == [300,60,10], "Original windows changed")
    flags(result, false=("schedule_completed", "native_gate_open", "server", "product", "original_send", "full_matrix"))
    require(result["source"] == SOURCE and result["failure"] is None and result["measured_profile_invocations"] == 2,
            "Original scheduling changed")
    lanes = []
    for lane in ("194", "188"):
        log = packet / f"{lane}-Profile-0.log"
        original = rows(log, "HC074-HELD-LIVE-ORIGINAL ")
        require(len(original) == 1, "Original refusal row missing/duplicated")
        receipt = load(packet / f"{lane}-Profile-0.exit.json")
        require(receipt["log_sha256"] == sha(log), "Profile log changed")
        lanes.append(lane_facts(rows(log, "HC074-HELD-PHASE "), original[0], receipt, lane, metadata))
        require(not any(packet.glob(f"{lane}-Profile-[12]*")), "Continuation after refusal")
    return dict(schema="offline-partial-preparation-attribution-074-v1", measured_source=SOURCE,
                input_packet=PACKET, input_inventory_sha256=INVENTORY, input_members_verified=55,
                analyzer_sha256=sha(__file__), lanes=lanes, interpretation=dict(
                    samples_per_lane=1, sample_kind="partial-refused-prefix",
                    scope="instrumented synthetic preparation only",
                    step_kernel_live_auth_overlap=True, kernel_outside_is_cpu=False,
                    includes_child_cpu=False, phase_sum_is_issued_age=False,
                    phase_excludes_report_aggregation_and_emission=True,
                    overhead_subtracted=False, compiler_comparison=False, timing_speedup_claim=False),
                new_profile_invocations=0, runtime_changed=False, observer_changed=False,
                complete_profile_cohort=False, native_gate_open=False, server=False, product=False)


def publish(path, facts, verify=False):
    if verify:
        require(load(path) == facts, "Derived facts changed")
    else:
        with Path(path).open("x", encoding="utf-8", newline="\n") as stream:
            json.dump(facts, stream, indent=2, sort_keys=True)
            stream.write("\n")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    facts = analyze(repo / "docs/testing/performance/0.74/local-runs" / PACKET)
    publish(args.output, facts, args.verify)
    print("offline partial attribution verified; complete cohort=false; native admission=false")


if __name__ == "__main__":
    main()
