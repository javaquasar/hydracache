"""Read sealed logs and pinned source text only; no host/profile entry point."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/local-profile-session-074"))
import attribute_material_rounds as rounds

RUNTIME_SOURCE = "e59190ffa431f36d9fc37998e60b368bcc2a1623"  # Not a measured candidate.
PACKETS = {
    "held-material-live-20261021": "0c385fbfe209fad1903aa2c0e620817c458f413eec8bd36fa32be5c01fbb4109",
    "held-material-round-attribution-20261021": "84c2d0a10e1d2d66b4dc76e1c041b9920d90d85621461bc3e83102569a6e1a2f",
    "pidfd-info-path-20261021": "dc7af6cd78dfc1b69c7443078a8c6e2edc4504e294f74d30df93e085f8471daa",
    "projection-groups-20261021": "ae7db79c3e579b7ad9f1939d2571aab27452b62f81c89b46b98340381f4b91c6",
}
SOURCE_PINS = {
    "diagnostic_validation_frame.rs": "6ae6d77f724c03e6449342f22152e685216af477172f39c6239d329ae0b78e6e",
    "diagnostic_preconsent_frame.rs": "5d91ada6967e522f161fe7ad16ff0a18b018bab77c7d8652993544e9fd566333",
    "diagnostic_collector_reserved.rs": "81f16769a8d95d62a78b5fc792ae234ca9979bb32b4847f69011e2eca1dfe2cf",
    "diagnostic_controller_request_attempt.rs": "664bbb4d73f02649b3984ef4e0d1ea64a292a18f9a404e357340069216d28ac7",
    "diagnostic_worker_kernel_binding.rs": "153a71dc617694e2b275fecfa1dc09faafb85a57920794a3e7005858bfb732c0",
    "diagnostic_worker_trust_input.rs": "e04f3d30a5d98b9cea96954ab4a8c5f7804a6e00b2fb0306fcd8fa531431c4eb",
    "diagnostic_controller_trust_input.rs": "db9e97c1ff11fa3cf95f9c54c6dbf065bfa4b0f07d558eab23c7aff76c7314b8",
    "diagnostic_worker_context.rs": "233c6315cbb4a73ad62eef4cadbf414bc99d91ef1ac40f71c730bd21db8e43ac",
    "diagnostic_kernel_admission.rs": "5f77f86c3c29a42156d63ffa913583f8a8c6419562f6c2bea5c34e75937260fa",
    "diagnostic_kernel_origin.rs": "4e18523c71b6b5bdf3a74d75e58df096f25c6f773067d013a07db2a8cfde5214",
    "diagnostic_kernel_collector_inputs.rs": "97b0fa0be4a59f245a7857a1106c1275030660922436fd8d9a65b1e4ce2c5395",
    "diagnostic_authority_fixture.rs": "3b3c75f41dbd8a8db1f73d61af3b0c701a666c3ebeed326ae49012ed0c01fb6c",
    "diagnostic_authority_fixed.rs": "fd1e10a038ae046888c8ff3c76303e36e5ed49a46aef2025391f8aa009d554a1",
    "diagnostic_boot_document.rs": "8e1a063dc637e4b92f8699848f4cf232b839663051d75e9c421f691815d96472",
    "diagnostic_boot_comparison.rs": "d6f4b02fd424cd80b8f4462b7f87e6149c84bbfe21a4fc83b685c5a6926b2040",
    "diagnostic_pidfd_info_path.rs": "33abc65043558b81639898ddeab46bee8cc32e0308644d09f8847b4891a90b94",
    "diagnostic_projection_groups.rs": "d6d46a710c664d9eb1a8a9a9ef09a515522aa5d47b4a4d7752d1e02825ff620a",
}
QUALIFICATION = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
BASE = Path("docs/testing/performance/0.74/local-runs")
OUTPUT = BASE / "reserved-leaf-audit-20261021"


def need(ok, why):
    if not ok:
        raise ValueError(why)


def sha(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def load(path):
    return json.loads(Path(path).read_text(encoding="utf-8-sig"))


def verify_packet(packet, pin):
    """Verify old raw seals directly, not a relaxed old source-review recipe."""
    packet = Path(packet)
    need(sha(packet / "inventory.json") == pin, "Input seal drift")
    seal = load(packet / "inventory.json")
    expected = set()
    for row in seal["files"]:
        name = row["path"]
        part = PurePosixPath(name)
        need(name == part.as_posix() and not part.is_absolute() and ".." not in part.parts
             and "\\" not in name and ":" not in name and name not in expected
             and name != "inventory.json", "Unsafe/duplicate member")
        expected.add(name)
        path = packet.joinpath(*part.parts)
        need(path.resolve().is_relative_to(packet.resolve()), "Member escapes packet")
        need(type(row["bytes"]) is int and path.stat().st_size == row["bytes"]
             and sha(path) == row["sha256"], "Member drift")
    all_paths = list(packet.rglob("*"))
    need(not packet.is_symlink() and not any(p.is_symlink() or
         (getattr(p, "is_junction", lambda: False)()) for p in all_paths), "Linked input")
    actual = {p.relative_to(packet).as_posix() for p in all_paths if p.is_file()}
    need(actual == expected | {"inventory.json"}, "Input membership drift")
    return dict(packet=packet.name, inventory_sha256=pin, members=len(expected))


def source_text(root):
    result = {}
    for name, pin in SOURCE_PINS.items():
        text = (root / "tools/long-run-supervisor-074/src" / name).read_text(encoding="utf-8").replace("\r", "")
        need(hashlib.sha256(text.encode("utf-8")).hexdigest() == pin, f"Source drift: {name}")
        result[name] = text
    return result


def witness(sources, name, fragment):
    text = sources[name]
    need(text.count(fragment) == 1, f"Ambiguous/missing source witness: {name}/{fragment}")
    return dict(path=f"tools/long-run-supervisor-074/src/{name}",
                line=text[:text.index(fragment)].count("\n") + 1, fragment=fragment)


def leaves(sources):
    # Review order is scope/risk, not a claim about remaining elapsed-time rank.
    specs = [
        ("context-document-scratch", "next-bounded-candidate", "diagnostic_worker_context.rs", "let mut buffer = vec![0; length + 1];"),
        ("trust-component-cstring", "separate-small-candidate", "diagnostic_worker_trust_input.rs", "let name = CString::new(name).map_err(|_| TrustInputError::Invalid)?;"),
        ("collector-peer-growth", "separate-small-candidate", "diagnostic_kernel_collector_inputs.rs", "let mut gids = supplementary.to_vec();"),
        ("proc-document-scratch", "attribution-and-lifetime-review", "diagnostic_kernel_origin.rs", "bytes: vec![0; DOC_BYTES + 1],"),
        ("sample-boot-ownership", "representation-change-deferred", "diagnostic_kernel_origin.rs", "boot: self.boot.clone(),"),
        ("authority-fresh-record-buffer", "read-semantics-review", "diagnostic_authority_fixture.rs", "let mut raw = Vec::with_capacity(before.len() as usize);"),
        ("authority-inventory-ownership", "inventory-semantics-review", "diagnostic_authority_fixture.rs", "let mut seen = BTreeSet::new();"),
        ("trust-snapshot-buffer", "bounded-loop-semantics-review", "diagnostic_worker_trust_input.rs", "let mut bytes = vec![0; limit + 1];"),
        ("context-temporary-dupfd", "descriptor-lifetime-review", "diagnostic_worker_context.rs", "let d = PinnedDirectory::new(current.try_clone()?, true)?;"),
        ("canonical-request-reconstruction", "not-a-single-allocation-leaf", "diagnostic_controller_request_attempt.rs", "fn capture("),
        ("fresh-kernel-walk", "preserve-all-observations", "diagnostic_kernel_origin.rs", "let sys = fixed_child(root, c\"sys\", true)?;"),
        ("pidfd-pathname", "allocation-candidate-already-retained", "diagnostic_kernel_origin.rs", ".open(pidfd_info_path::InfoPath::new(pidfd.as_raw_fd()).as_path())?;"),
        ("groups-collection", "allocation-candidate-already-retained", "diagnostic_kernel_admission.rs", ".collect::<Result<projection_groups::Values, _>>()?"),
    ]
    return [dict(id=identity, disposition=state, exclusive_leaf_cost_measured=False,
                 witness=witness(sources, name, fragment)) for identity, state, name, fragment in specs]


def reserved_step(row):
    # The unchanged arithmetic reader checks every step, kernel edge and prefix.
    phase = rounds.prior.phase_facts(row)
    need(phase["phase"] == "projection-materialization" and not row["returned"], "Wrong refused phase")
    step = phase["steps"][1]
    need(step["step"] == "ReservedLive" and step["calls"] == step["completed"], "ReservedLive prefix changed")
    kernel = step["kernel"]
    total = sum(n["outside_instrumented_children_ns"] for n in kernel)
    need(total == step["kernel_outside_children_total_ns"], "Kernel partition changed")
    return dict(calls=step["calls"], completed=step["completed"], wall_ns=step["wall_ns"],
        kernel_outside_children_total_ns=total,
        other_wall_ns=step["wall_outside_instrumented_kernel_ns"],
        kernel=kernel, live_overlapping_view=step["live"],
        kernel_rank_by_outside_children_wall=[n["kind"] for n in sorted(kernel,
            key=lambda n:n["outside_instrumented_children_ns"], reverse=True)])


def analyze(root=ROOT):
    root = Path(root)
    packets = [verify_packet(root / BASE / name, pin) for name, pin in PACKETS.items()]
    sources = source_text(root)
    need(sha(root / "docs/testing/performance/0.74/qualification-manifest.toml") == QUALIFICATION, "Qualification drift")
    old = rounds.analyze(root / BASE / rounds.PACKET)
    need(old == load(root / BASE / "held-material-round-attribution-20261021/facts.json"), "Prior facts reconstruction differs")
    lanes = []
    for checked in old["lanes"]:
        rows = rounds.prior.rows(root / BASE / rounds.PACKET / f"{checked['lane']}.log", "HC074-HELD-PHASE ")
        step = reserved_step(rows[-1])
        prior_step = next(s for s in checked["phases"][-1]["steps"] if s["step"] == "ReservedLive")
        need((step["wall_ns"], step["calls"], step["kernel_outside_children_total_ns"], step["other_wall_ns"])
             == (prior_step["wall_ns"], prior_step["calls"], prior_step["kernel_ns"], prior_step["other_ns"]), "Earlier step totals changed")
        lanes.append(dict(lane=checked["lane"], first_original_refusal=checked["first_original_refusal"],
                          reserved_live=step))
    return dict(schema="offline-reserved-leaf-audit-074-v1", audited_runtime_source=RUNTIME_SOURCE,
        measured_source=rounds.SOURCE, input_packets=packets, sources=SOURCE_PINS,
        qualification_sha256=QUALIFICATION, lanes=lanes, leaves=leaves(sources),
        source_only_context_shape=dict(document_lengths=[33,37], callback_buffer_lengths=[34,38],
            document_reads_per_complete_successful_probe_observe=4,
            shape_is_not_measured_allocation_count=True),
        interpretation=dict(leaf_review_order_is_not_elapsed_rank=True,
            kernel_live_auth_views_overlap=True, outside_children_wall_is_exclusive_cpu=False,
            per_leaf_allocation_or_syscall_time_available=False, post_candidate_timing_available=False,
            old_trace_counts_not_extrapolated_to_candidate_savings=True),
        original_windows_seconds=[300,60,10], new_profile_invocations=0,
        runtime_changed=False, observer_changed=False, clocks_changed=False, services_changed=False,
        original_material_gate_passed=False, full_sigkill_matrix=False, server=False, product=False)


def require_output(path):
    need(Path(path).resolve() == (ROOT / OUTPUT / "facts.json").resolve(),
         "Output must use facts.json in the separate audit packet")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    require_output(args.output)
    facts = analyze()
    rounds.prior.publish(args.output, facts, args.verify)
    print("Offline ReservedLive leaf audit verified; no new profile, runtime or host changes")
