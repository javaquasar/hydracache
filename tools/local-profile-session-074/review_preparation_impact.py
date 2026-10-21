"""Offline full preparation review, preserving success or first refused prefix."""
import argparse
import copy
import json
from pathlib import Path
import attribute_partial as prior
import runner
from run_preparation_impact import NAME, METHOD


def profile_facts(log, receipt, metadata):
    require = prior.require
    require(receipt["containment_failure"] is None and type(receipt["exit"]) is int
            and receipt["exit"] in (0, 101), "Not an original harness outcome")
    require(receipt["log_sha256"] == prior.sha(log) and receipt["bytes"] == log.stat().st_size,
            "Raw profile drift")
    rows = prior.rows(log, "HC074-HELD-PHASE ")
    originals = prior.rows(log, "HC074-HELD-LIVE-ORIGINAL ")
    require(0 < len(rows) <= len(prior.PHASES)
            and tuple(r["phase"] for r in rows) == prior.PHASES[:len(rows)], "Phase order changed")
    require(len(originals) == 1, "Original observation missing/duplicated")
    original = originals[0]
    prior.flags(original, false=("guard_clock_added", "deadline_changed", "server", "product"))
    require(original["case"] == "healthy" and type(original["returned"]) is bool,
            "Original case/result changed")
    success = receipt["exit"] == 0
    require(original["returned"] is success and all(r["returned"] is True for r in rows[:-1])
            and rows[-1]["returned"] is success, "Receipt does not join first original outcome")
    require(not success or len(rows) == 10, "Successful preparation missing phases")
    phases = []
    for row in rows:
        # Reuse the immutable arithmetic validator. Its historical complete-output
        # signature applies only if this output phase actually returned.
        projected = copy.deepcopy(row)
        if row["phase"] == "outputs" and not row["returned"]:
            projected["phase"] = "unfinished-outputs"
        fact = prior.phase_facts(projected)
        fact["phase"] = row["phase"]
        phases.append(fact)
    refusal = original["causal"]["first_refusal"] if original["causal"] else None
    if success:
        require(refusal is None, "Success erased an original refusal")
        require("HC074-HELD-LIVE case=healthy " in log.read_text()
                and "original_send=false completion=false server=false product=false" in log.read_text(),
                "Preparation-only success footer missing")
    elif refusal is not None:
        require(refusal["issued_boot"] == refusal["now_boot"] == metadata["boot"], "Refusal boot drift")
        if refusal["diagnostic_reason"] == "expired":
            require(prior.number(refusal["now_ns"]) - prior.number(refusal["issued_ns"])
                    == prior.number(refusal["age_ns"]) >= 10_000_000_000, "Expiration arithmetic changed")
        # Keep other original causes as refusals, never assume every failure is Time.
    return dict(lane=receipt["lane"], sample=receipt["sample"], exit=receipt["exit"],
                complete_preparation=success, original=original, phases=phases,
                observed_phase_wall_total_ns=sum(p["wall_ns"] for p in phases),
                observed_parent_cpu_total_ns=(sum(p["parent_process_cpu_ns"] for p in phases)
                    if all(p["parent_process_cpu_ns"] is not None for p in phases) else None))


def analyze(packet, source):
    require = prior.require
    metadata, post, intent, result = (prior.load(packet / n) for n in
                                    ("metadata.json", "postflight.json", "intent.json", "result.json"))
    require(metadata == post and metadata["source"] == source and result["source"] == source
            and intent["source"] == source, "Source/context drift")
    require(metadata["uid"] == 0 and not metadata["fixtures"] and not metadata["tracked_dirty"],
            "Dirty source/fixture context")
    require(metadata["qualification_sha256"] == runner.QUALIFICATION, "Qualification drift")
    require(intent["original_windows_seconds"] == [300,60,10]
            and intent["schedule"] == [[l,n] for l in runner.LANES for n in range(3)]
            and intent["child_timeout_seconds"] == 1200 and intent["kill_after_seconds"] == 5
            and intent["child_containment_seconds"] == runner.CHILD_SECONDS
            and intent["session_seconds"] == runner.SESSION_SECONDS
            and intent["max_log_bytes"] == runner.MAX_LOG_BYTES, "Original schedule/bounds drift")
    for row in (intent,result):
        prior.flags(row, false=("server","product","original_send","native_gate_open"))
    prior.flags(result, false=("full_matrix",))
    require(result["failure"] is None, "Containment/metadata failure; retain raw incomplete evidence")
    require(type(result["schedule_completed"]) is bool, "Invalid completion type")
    require(metadata["impact_method"] == {n: prior.sha(Path(__file__).parent/n) for n in METHOD},
            "Impact method drift")
    profiles = []
    expected = []
    for lane in runner.LANES:
        for sample in range(3):
            prefix = packet / f"{lane}-Profile-{sample}"
            receipt = prior.load(prefix.with_suffix(".exit.json"))
            require(receipt["lane"] == lane and type(receipt["sample"]) is int
                    and receipt["sample"] == sample, "Profile order changed")
            expected.append(receipt)
            require(prior.load(packet / f"{lane}-{sample}-before.json") == metadata
                    == prior.load(packet / f"{lane}-{sample}-after.json"), "Profile metadata drift")
            profile = profile_facts(prefix.with_suffix(".log"), receipt, metadata)
            profiles.append(profile)
            if receipt["exit"] != 0:
                require(not any(packet.glob(f"{lane}-Profile-[{sample+1}-3]*")) if sample < 2 else True,
                        "Continuation after first lane refusal")
                break
    require(result["profiles"] == expected and result["measured_profile_invocations"] == len(expected),
            "Recorded scheduling drift")
    completed = len(profiles) == 6 and all(p["complete_preparation"] for p in profiles)
    require(result["schedule_completed"] is completed, "False schedule completion")
    old = prior.analyze(packet.parent / prior.PACKET)
    return dict(schema="full-local-preparation-impact-074-v1", source=source,
                profiles=profiles, complete_profile_cohort=completed, metadata=metadata,
                previous_refused_prefixes=old["lanes"],
                interpretation=dict(phase_sum_is_issued_age=False, phase_excludes_report_emission=True,
                    overlapping_spans=True, includes_child_cpu=False, overhead_subtracted=False,
                    different_wsl_lifetimes=metadata["boot"] != prior.load(packet.parent/prior.PACKET/"metadata.json")["boot"],
                    causal_timing_comparison=False, cpu_nonregression_proven=False),
                native_gate_open=False, server=False, product=False)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    packet = Path(__file__).resolve().parents[2]/"docs/testing/performance/0.74/local-runs"/NAME
    prior.publish(args.output, analyze(packet, args.source), args.verify)
    print("Original preparation outcome and conserved prefixes verified; native admission=false")
