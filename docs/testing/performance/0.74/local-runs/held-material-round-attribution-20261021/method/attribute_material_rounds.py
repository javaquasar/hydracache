"""Offline arithmetic on sealed first refusals. No profiler or runtime entry."""
import argparse
from pathlib import Path, PurePosixPath
import attribute_partial as prior
import review_material_live as review

PACKET = "held-material-live-20261021"
INVENTORY = "0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109"
SOURCE = "1d6d179bd7150ef564560821a897794e20b24b36"
PARTITION = ("kernel_inside_steps_ns", "other_inside_steps_ns",
             "kernel_outside_steps_ns", "unassigned_outside_steps_ns")


def verify_inventory(packet):
    packet = Path(packet)
    prior.require(prior.sha(packet / "inventory.json") == INVENTORY, "Input inventory pin drift")
    seal = prior.load(packet / "inventory.json")
    prior.require(seal["source"] == SOURCE and len(seal["files"]) == 65, "Input source/membership count drift")
    expected = set()
    for row in seal["files"]:
        name = row["path"]
        relative = PurePosixPath(name)
        prior.require(name == relative.as_posix() and not relative.is_absolute()
                      and ".." not in relative.parts and "\\" not in name and ":" not in name
                      and name not in expected and name != "inventory.json", "Unsafe/duplicate input member")
        expected.add(name)
        path = packet.joinpath(*relative.parts)
        prior.require(all(not p.is_symlink() for p in (path, *path.parents))
                      and path.resolve().is_relative_to(packet.resolve()), "Input member escapes packet")
        prior.require(type(row["bytes"]) is int and path.stat().st_size == row["bytes"]
                      and prior.sha(path) == row["sha256"], "Input member drift")
    actual = {p.relative_to(packet).as_posix() for p in packet.rglob("*") if p.is_file()}
    prior.require(actual == expected | {"inventory.json"}, "Unexpected/missing input member")


def difference(total, subset, label):
    prior.number(total)
    prior.number(subset)
    prior.require(total >= subset, f"Negative {label}; do not clamp or subtract overhead")
    return total - subset


def phase_facts(row):
    # Existing immutable reader checks step order, partial-round prefix, every
    # root/edge/node count and duration, and the old complete output signature.
    old = prior.phase_facts(row)
    steps = old["steps"]
    kernel = []
    for i, node in enumerate(old["kernel"]):
        inside = {field: sum(s["kernel"][i][field] for s in steps)
                  for field in ("calls", "inclusive_ns", "outside_instrumented_children_ns")}
        outside = {field: difference(node[field], inside[field], f"kernel {node['kind']}/{field}")
                   for field in inside}
        kernel.append(dict(kind=node["kind"], phase=node, inside_steps=inside, outside_steps=outside))
    step_wall = sum(s["wall_ns"] for s in steps)
    kernel_inside = sum(n["inside_steps"]["outside_instrumented_children_ns"] for n in kernel)
    kernel_outside = sum(n["outside_steps"]["outside_instrumented_children_ns"] for n in kernel)
    outside_wall = difference(old["wall_ns"], step_wall, "outside-step wall")
    partition = dict(kernel_inside_steps_ns=kernel_inside,
        other_inside_steps_ns=difference(step_wall, kernel_inside, "other inside-step wall"),
        kernel_outside_steps_ns=kernel_outside,
        unassigned_outside_steps_ns=difference(outside_wall, kernel_outside, "unassigned outside-step wall"))
    prior.require(sum(partition.values()) == old["wall_ns"], "Wall partition does not conserve phase")
    auth = {}
    for field in ("authentication_calls", "authentication_ns", "signature_calls", "signature_ns"):
        auth[field] = dict(phase=prior.number(row[field]), inside_steps=sum(s[field] for s in steps))
        auth[field]["outside_steps"] = difference(auth[field]["phase"], auth[field]["inside_steps"], field)
    started, completed = steps[0]["calls"], steps[-1]["completed"]
    unfinished = [dict(step=s["step"], calls=s["calls"], completed=s["completed"])
                  for s in steps if s["calls"] != s["completed"]]
    compact_steps = [dict(step=s["step"], calls=s["calls"], completed=s["completed"],
        wall_ns=s["wall_ns"], kernel_ns=s["kernel_outside_children_total_ns"],
        other_ns=s["wall_outside_instrumented_kernel_ns"], authentication_calls=s["authentication_calls"],
        authentication_ns=s["authentication_ns"], signature_ns=s["signature_ns"]) for s in steps]
    return dict(phase=row["phase"], returned=row["returned"], wall_ns=old["wall_ns"],
        parent_process_cpu_ns=old["parent_process_cpu_ns"], rounds_started=started,
        rounds_completed=completed, unfinished_steps=unfinished, steps=compact_steps,
        partition=partition, kernel=kernel, authentication_overlapping_view=auth,
        dominant_step=max(compact_steps, key=lambda s:s["wall_ns"])["step"] if started else None,
        kernel_rank_by_outside_children_wall=[n["kind"] for n in sorted(kernel,
            key=lambda n:n["phase"]["outside_instrumented_children_ns"], reverse=True)])


def lane_facts(packet, lane, checked):
    rows = prior.rows(packet / f"{lane}.log", "HC074-HELD-PHASE ")
    prior.require(len(rows) == 11 and checked["harness_passed"] is False
                  and checked["cleanup_assertions_passed"] is False, "First refused prefix erased")
    prior.require(checked["stages"] == list(review.STAGES[:3]), "Original material stage prefix changed")
    refusal = checked["original"]["causal"]["first_refusal"]
    prior.require(refusal["diagnostic_reason"] == "expired" and refusal["site"] == "window"
                  and prior.number(refusal["age_ns"]) >= 10_000_000_000, "Original expiry changed")
    phases = [phase_facts(r) for r in rows]
    final = phases[-1]
    prior.require(final["returned"] is False and final["unfinished_steps"] == [dict(
        step="Causal",calls=final["rounds_started"],completed=final["rounds_completed"])], "Refusal/unfinished step mismatch")
    totals = {key: sum(p["partition"][key] for p in phases) for key in PARTITION}
    observed_wall = sum(p["wall_ns"] for p in phases)
    prior.require(sum(totals.values()) == observed_wall, "Observed phase totals do not conserve")
    return dict(lane=lane, first_original_refusal=refusal, original_stages=checked["stages"],
        phases=phases, observed_phase_wall_ns=observed_wall, observed_phase_partition=totals,
        total_rounds_started=sum(p["rounds_started"] for p in phases),
        total_rounds_completed=sum(p["rounds_completed"] for p in phases),
        failed_phase="projection-materialization", cleanup_assertions_passed=False)


def analyze(packet):
    packet = Path(packet)
    verify_inventory(packet)
    checked = review.analyze(packet, SOURCE)
    prior.require(checked == prior.load(packet / "summary.json"), "Sealed summary reconstruction differs")
    prior.flags(checked, false=("original_material_gate_passed", "full_sigkill_matrix", "original_delivery_proven",
        "all_production_writers_proven", "cpu_speedup_claim", "server", "product", "original_send", "native_gate_open"))
    lanes = [lane_facts(packet, lane, row) for lane, row in zip(("194","188"), checked["profiles"])]
    return dict(schema="offline-original-material-round-attribution-074-v1", measured_source=SOURCE,
        input_packet=PACKET, input_inventory_sha256=INVENTORY, input_members_verified=65,
        analyzer_sha256=prior.sha(__file__), reader_sha256=prior.sha(review.__file__),
        arithmetic_reader_sha256=prior.sha(prior.__file__), lanes=lanes,
        interpretation=dict(samples_per_lane=1, sample_kind="first-refused-prefix", wall_is_not_exclusive_cpu=True,
            step_kernel_live_auth_views_overlap=True, kernel_outside_is_outside_instrumented_children_only=True,
            outside_step_residual_is_effect_cost=False, observer_overhead_subtracted=False,
            stage_durations_reconstructed=False, per_round_latency_distribution_available=False,
            phase_sum_is_admission_age=False, compiler_comparison=False, timing_speedup_claim=False,
            requires_separate_candidate_before_new_live_entry=True),
        new_profile_invocations=0, runtime_changed=False, observer_changed=False,
        original_material_gate_passed=False, full_sigkill_matrix=False, native_gate_open=False,
        original_send=False, server=False, product=False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    packet = args.input or root / "docs/testing/performance/0.74/local-runs" / PACKET
    facts = analyze(packet)
    prior.require(not args.output.resolve().is_relative_to(packet.resolve()), "Never write into sealed input")
    prior.publish(args.output, facts, args.verify)
    print("Offline round costs conserved; first refusals unchanged; zero new profile invocations")
