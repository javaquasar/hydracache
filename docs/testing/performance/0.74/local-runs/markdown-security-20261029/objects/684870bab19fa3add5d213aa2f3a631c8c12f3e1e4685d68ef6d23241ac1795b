"""Read-only review of one pure owned-inline prototype; never runs a probe."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/sample-boot-baseline-074"))
import review_boot as previous

need, load, sha, exact, counts = previous.need, previous.load, previous.sha, previous.exact, previous.counts
canonical, publish, BASE = previous.canonical, previous.publish, previous.BASE
OUTPUT = BASE / "sample-boot-inline-20261022"
INPUTS = dict(previous.INPUTS, **{
    "sample-boot-baseline-20261022": "fd07cd7911ab50d51e5a471de9ae5b427ec4076636396fe12861c46433ac6b10",
})
PROBE_PINS = {
    "tools/sample-boot-inline-074/Cargo.toml": "72b6477ee71d46e803aed4fd4eba3c964256cab730a2cbb194b619349d1804ae",
    "tools/sample-boot-inline-074/Cargo.lock": "6f8b4178b840231e5bac06f3fd7abaa165363ed45bda5bd8c11064b0670c2e72",
    "tools/sample-boot-inline-074/probe.rs": "af693fcb36ad9117ab8ba06eb280ce98516ca9a277e2c893496c23df7126a24b",
    "tools/sample-boot-inline-074/model.rs": "ee1ca7f5e3899519ae85c1cf89950dac7b2ad63af8268b2c6e70c94974ed2cfc",
    "tools/sample-boot-inline-074/cost.rs": "e6a7bb4a00a8c13e7a0ad74d841ea3919c0bbcccfe601896f14f316f366a7aca",
    "tools/sample-boot-inline-074/prototype.rs": "14adf00af849063ca29247fea1c5d46039df604bbb3e277b0cd9a0f04d4b5e20",
    "tools/sample-boot-inline-074/compare.rs": "5bada85f733cd97bf41ec6f95f8d2ee8c2cb8b614ab99a7eca6027206c9212b4",
    "tools/sample-boot-inline-074/check.ps1": "4e4fb9d8b54bdcdb08da96370b3acbed1c6a57e24abcf4847d1f11acdeed8452",
    "crates/xtask/tests/sample_boot_inline_074.rs": "3c28559917ccd0250de8f9cab47b0b7117a8bcf9181de4d1299354e3935ef7e3",
    "docs/testing/performance/0.74/sample-boot-inline-plan-20261022.md": "ea5d5a8e1d046f6dc564b3440fdfc6050736b937c37707a4ab5f0ea199605da6",
}
REFERENCE_PINS = {name: pin for name, pin in PROBE_PINS.items() if name.endswith(("Cargo.toml", "Cargo.lock", "cost.rs"))}
REFERENCE_PINS.update({
    "tools/sample-boot-inline-074/probe.rs": "138629805a63831938e215b6416b353e72284cc0db46d42b077dd5bc3b6baf6d",
    "tools/sample-boot-inline-074/model.rs": "f2ed3e2841ca65e774b25d38db1d925ff793fe53464da1fbe14e0a41c3a018cf",
    "tools/sample-boot-inline-074/check.ps1": "e238d08c07bf59a36e9d005808a9d249eeed230e293040c23af77c32760981bc",
    "crates/xtask/tests/sample_boot_inline_074.rs": "11dae48e672be75536dd1d45fd74090865fd97301fd4f5872c78868a2f72ff34",
})
OLD_TESTS = ["counter::counter_nested_unwind_and_thread_isolation", "counter::counter_zeroed_reallocation_and_drop_forwarding"]
OLD_TESTS += ["model::controls::" + name for name in (
    "sample_boot_all_fault_masks_keep_first_error_and_no_clone", "sample_boot_decoder_preserves_original_bounded_shape",
    "sample_boot_main_owned_sample_requests", "sample_boot_seeded_owned_points_getters_and_point_clones",
    "sample_boot_stage_requests_are_separate_from_main")]
NEW_TESTS = ["model::comparisons::" + name for name in (
    "inline_all_first_fault_prefixes", "inline_layout_clone_and_getter_cost_controls",
    "inline_ownership_generic_boundaries_seed_and_debug", "inline_paired_getter_intervals", "inline_paired_sample_requests")]


def source_pins(root, pins=PROBE_PINS):
    for name, pin in pins.items():
        need(hashlib.sha256(canonical(Path(root) / name).encode()).hexdigest() == pin, f"Source drift: {name}")
    return pins


def receipt(directory, lane, phase, cohort="first", failed=False):
    directory = Path(directory)
    row = load(directory / f"{lane}-result.json")
    exact({k: row[k] for k in row if k not in ("steps", "failure")}, dict(
        phase=phase, cohort=cohort, lane=lane, platform="windows-msvc", model_only=True, prototype_only=True,
        runtime_candidate_implemented=False, original_owner_profiles=0, ignored_tests_run=False, server=False, product=False),
        "Wrong isolated receipt identity")
    names = ["canaries"] if failed else (["canaries", "check", "clippy"] if lane == "host" else ["compiler", "controls", "check", "clippy"])
    exact([step["name"] for step in row["steps"]], names, "Wrong receipt steps")
    exact(row["failure"], "Check refused: candidate/host/canaries (101)" if failed else None, "Failure must not be hidden")
    for step in row["steps"]:
        exact(step["exit"], 101 if failed else 0, "Wrong check exit")
        need(step["log"] == f"{lane}-{step['name']}.log", "Wrong receipt path")
        need(sha(directory / step["log"]) == step["sha256"], "Changed raw log")
    return row


def parse(text, sizes, tests, filtered):
    rows = {kind: [] for kind in sizes}
    old_rows, cleaned = [], []
    for line in text.splitlines():
        if "HC074-BOOT-INLINE-" in line:
            prefix, payload = line.split("HC074-BOOT-INLINE-", 1)
            kind, payload = payload.split(" ", 1)
            need(kind in rows, "Unknown inline row")
            rows[kind].append(json.loads(payload))
            cleaned.append(prefix.rstrip())
        elif "HC074-SAMPLE-BOOT-" in line:
            prefix, payload = line.split("HC074-SAMPLE-BOOT-", 1)
            old_rows.append("HC074-SAMPLE-BOOT-" + payload)
            cleaned.append(prefix.rstrip())
        else:
            cleaned.append(line)
    for kind, size in sizes.items():
        exact(len(rows[kind]), size, "Missing/duplicate inline rows")
    passed = re.findall(r"(?m)^test (\S+) \.\.\.\s+ok\s*$", "\n".join(cleaned))
    exact(sorted(passed), sorted(tests), "Exact passing selection required")
    need(f"test result: ok. {len(tests)} passed; 0 failed; 0 ignored; 0 measured; {filtered} filtered out;" in text
         and "test result: FAILED" not in text, "Wrong test outcome")
    if not old_rows:
        need(tests == ["model::cost::reference_cost_preflight"], "Frozen baseline missing")
        return rows, None
    # The whole passing selection above authenticates the old seven outcomes;
    # the unchanged old reader independently checks every old structured row.
    baseline = previous.log_facts("\n".join(old_rows + ["test " + name + " ... ok" for name in OLD_TESTS] + [
        "test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;"]))
    return rows, baseline


def intervals(rows, paired):
    for sample, row in enumerate(rows):
        times = ("reference_wall_ns", "candidate_wall_ns") if paired else ("reference_wall_ns",)
        for field in times:
            need(type(row[field]) is int and row[field] > 0, "Positive integer timer required")
        expected = dict(sample=sample, iterations=262144, checksum=9437184, allocator_capture_active=False, model_only=True)
        if paired:
            expected.update(warmup=32768, order="reference-first" if sample % 2 == 0 else "candidate-first")
        expected.update({field: row[field] for field in times})
        exact(row, expected, "Getter corpus/order changed")
    return {field: sorted(row[field] for row in rows)[3] for field in times}


def reference_facts(text):
    rows, baseline = parse(text, {"REFERENCE": 1, "PREFLIGHT": 7}, ["model::cost::reference_cost_preflight"], 7)
    need(baseline is None, "Reference-only preflight required")
    exact(rows["REFERENCE"][0], dict(point_size=40, point_align=8, clone_calls=256,
          clone_counts=counts(256, 0, 0, 9216, 256, 9216), getter_counts=counts(), iterations=262144,
          warmup=32768, checksum=9437184, model_only=True), "Reference layout/requests changed")
    return dict(rows=rows, medians_ns=intervals(rows["PREFLIGHT"], False))


def candidate_facts(text):
    rows, baseline = parse(text, {"SAMPLES": 9, "FAULTS": 1, "OWNERSHIP": 1, "LAYOUT": 1, "GETTERS": 7}, OLD_TESTS + NEW_TESTS, 1)
    for row, (name, length, capacity) in zip(rows["SAMPLES"], previous.CASES):
        old_n = 256 if length else 0
        new_n = 256 if length > 36 else 0
        exact(row, dict(case=name, length=length, input_capacity=capacity, heap_capacity=length if length > 36 else 0,
              samples_per_arm=256, reference=counts(old_n, 0, 0, length * old_n, old_n, length * old_n),
              candidate=counts(new_n, 0, 0, length * new_n, new_n, length * new_n), model_only=True), "Paired corpus/requests changed")
    exact(rows["FAULTS"][0], dict(masks=16, refused=15, same_first_error_and_events=True,
          refused_allocations=0, model_only=True), "First-error prefixes changed")
    exact(rows["OWNERSHIP"][0], dict(corpus=9, generic=246, seeded=256, seed="0x074b0a19",
          debug_and_pretty_debug_equal=True, owned_and_independent=True, model_only=True), "Ownership contract changed")
    exact(rows["LAYOUT"][0], dict(reference_size=40, reference_align=8, candidate_size=56, candidate_align=8,
          clone_calls=256, reference_clones=counts(256, 0, 0, 9216, 256, 9216), candidate_clones=counts(),
          reference_getters=counts(), candidate_getters=counts(), iterations=262144, checksum=9437184,
          physical_copy_claim=False, model_only=True), "Layout/Clone/getter cost changed")
    totals = {arm: {field: sum(row[arm][field] for row in rows["SAMPLES"])
              for field in previous.previous.FIELDS} for arm in ("reference", "candidate")}
    exact(totals["reference"], baseline["main_totals"], "Paired reference differs from frozen corpus")
    return dict(rows=rows, old_baseline=baseline, totals=totals, medians_ns=intervals(rows["GETTERS"], True))


def host_canaries(directory):
    text = (Path(directory) / "host-canaries.log").read_text(encoding="utf-8-sig")
    passed = [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed; 0 ignored;", text)]
    need(len(passed) == 16 and sum(passed) == 46 and "test result: FAILED" not in text, "Source canary cohort changed")


def analyze(root=ROOT, checks=None):
    root = Path(root)
    checks = Path(checks) if checks else root / OUTPUT
    packets = [previous.previous.prior.verify_packet(root / BASE / name, pin) for name, pin in INPUTS.items()]
    exact(sum(row["members"] for row in packets), 856, "Old member total changed")
    need(canonical(root / "tools/sample-boot-baseline-074/review_boot.py") ==
         canonical(root / BASE / "sample-boot-baseline-20261022/method/review_boot.py"), "Old reader drift")
    old = previous.analyze(root)
    exact(old, load(root / BASE / "sample-boot-baseline-20261022/facts.json"), "Old baseline drift")
    source_pins(root)
    # Before packet publication, reference source is verified by its immutable
    # Git commit/source canary; archived snapshot is mandatory in sealed review.
    if (checks / "reference-source").is_dir():
        source_pins(checks / "reference-source", REFERENCE_PINS)
    lanes = []
    for lane, version in (("194", "1.94.0"), ("188", "1.88.0")):
        phases = {}
        for phase, reader in (("reference", reference_facts), ("candidate", candidate_facts)):
            directory = checks / phase
            receipt(directory, lane, phase)
            compiler = (directory / f"{lane}-compiler.log").read_text(encoding="utf-8-sig")
            need(compiler.startswith("rustc " + version + " ") and "host: x86_64-pc-windows-msvc" in compiler, "Wrong compiler")
            phases[phase] = reader((directory / f"{lane}-controls.log").read_text(encoding="utf-8-sig"))
        exact(phases["candidate"]["old_baseline"], old["lanes"][0]["facts"], "Frozen reference rows changed")
        lanes.append(dict(lane=lane, compiler=version, **phases))
    receipt(checks / "reference", "host", "reference")
    host_canaries(checks / "reference")
    receipt(checks / "candidate", "host", "candidate", failed=True)
    failure = (checks / "candidate/host-canaries.log").read_text(encoding="utf-8-sig")
    need("test result: FAILED. 2 passed; 1 failed;" in failure and
         "inline_boot_reference_restores_the_complete_frozen_receiver_and_model ... FAILED" in failure, "First canary refusal missing")
    failed_source = canonical(checks / "candidate/first-failed-canary.rs")
    fixed_source = canonical(root / "crates/xtask/tests/sample_boot_inline_074.rs")
    before = '    assert_eq!(\n        format!("{restored}\\n"),\n        old("tools/sample-boot-baseline-074/probe.rs")\n    );'
    after = '    assert_eq!(restored, old("tools/sample-boot-baseline-074/probe.rs"));'
    need(failed_source.count(before) == 1 and failed_source.replace(before, after) == fixed_source,
         "Only exact newline guard repair permitted")
    receipt(checks / "candidate-canary-repair", "host", "candidate", cohort="canary-repair")
    host_canaries(checks / "candidate-canary-repair")
    rejected = all(lane["candidate"]["medians_ns"]["candidate_wall_ns"] >
                   lane["candidate"]["medians_ns"]["reference_wall_ns"] for lane in lanes)
    return dict(schema="pure-sample-boot-inline-074-v1", input_packets=packets, input_members_verified=856,
        probe_source_pins=PROBE_PINS, reference_source_pins=REFERENCE_PINS, source_canaries=46, lanes=lanes,
        decision="reject-for-adoption" if rejected else "non-promotable-pending-evidence",
        decision_rule="getter median greater on both compilers rejects adoption",
        first_canary_refusal_preserved=True, canary_repair="one extra newline in source restoration assertion",
        measurement_retries=0, prototype_representation_only=True, runtime_candidate_implemented=False,
        original_windows_seconds=old["original_windows_seconds"], first_refusals=old["first_refusals"],
        qualification_sha256=old["qualification_sha256"], original_owner_profiles=0,
        full_path_speedup_proven=False, original_material_gate_passed=False, full_sigkill_matrix=False,
        clocks_changed=False, services_changed=False, server=False, product=False)


def require_output(path):
    need(Path(path).resolve() == (ROOT / OUTPUT / "facts.json").resolve(), "Separate inline facts required")
    for node in (Path(path), *Path(path).parents):
        need(not node.is_symlink() and not getattr(node, "is_junction", lambda: False)(), "Linked output")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--packet-sha256")
    args = parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify, "Sealed review is read-only")
        previous.previous.prior.verify_packet(args.output.parent, args.packet_sha256)
        need((args.output.parent / "reference-source").is_dir(), "Reference source missing")
    publish(args.output, analyze(), args.verify)
    print("Owned inline prototype reviewed; first paired measurements only; no runtime/live/server promotion")
