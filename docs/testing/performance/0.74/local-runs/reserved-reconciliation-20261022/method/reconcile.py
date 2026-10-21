"""Reconcile sealed allocation cohorts with old refused traces; never run a profile."""
import argparse
import hashlib
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/offline-reserved-leaf-audit-074"))
import audit as prior

need, load, sha = prior.need, prior.load, prior.sha
BASE = prior.BASE
OUTPUT = BASE / "reserved-reconciliation-20261022"
RUNTIME_SOURCE = "1e5f31d9d443bf6c9b1d901781986501425bf9e9"
AUDIT = "reserved-leaf-audit-20261021"
PACKETS = dict(prior.PACKETS, **{
    AUDIT: "d66631eec23dc969f372d73c2bb83519058214383023564ec269ec542f854c4e",
    "context-document-baseline-20261021": "8db3e4df5579ec7a471886505191825609b5c839732670bece6702ae101afcb6",
    "context-document-stack-20261021": "7a65d50c19622754bda58fd389ef03d3f07254465d96f1e16b586cdec94f71f0",
    "trust-component-baseline-20261021": "7eacd69041a722fc3d6153ef8be43eff87bf945a8fb6ccdd456acadff1bfcc9d",
    "trust-component-stack-20261021": "e588fd8ae6b06a6e808446328b819240c4bb0e35955309e843ddb1078668fd85",
    "collector-peer-baseline-20261021": "cc19972e68a264b7b84ac48400f43e1b981d5a0e762ad7a9aee1a67548bf2452",
    "collector-peer-capacity-20261022": "846b5e7d6d07a47bd03c64be0a207b24b197b39ceddb25e4fc3b06cd049b05ae",
})
UPDATES = {
    "diagnostic_worker_context.rs": "9cb0d6820c8629d74bb09315f028fe7c448e1ae83e6ec4c2cbb08107a6685857",
    "diagnostic_worker_trust_input.rs": "891fa2ec885cf3193f575afe4e4b2f4f70a885868d9868b1843a49753d6f8807",
    "diagnostic_kernel_collector_inputs.rs": "ad87848f65a890f981a39fd5c0e5330d64770870d40d2320cd871fd0810a7e40",
    "diagnostic_context_document_buffer.rs": "888deb5f5b254ef2f82f07719e30e14dd2d1eca851175591420467a20f07725d",
    "diagnostic_trust_component_name.rs": "586a156d7e272c02d8d14e340b025457bc218c2b00380d9eee80aaeb11b57f68",
}
SPECS = (
    # Each metric pair is local to this corpus; missing counters stay absent.
    ("pidfd-pathname", "pidfd-info-path-20261021", "0cb604afdc14adb544ba613fd6d46189401254b4",
     "1c7e00f327e45be8ec543fcd9b8da21faf481874", None, 2304,
     (("allocations", "reference_allocations", "candidate_allocations"),
      ("requested_bytes", "reference_requested_bytes", "candidate_requested_bytes"))),
    ("groups-collection", "projection-groups-20261021", "355ab0126b25a8e4a693ee3b70b5a1f944004977",
     "55c1983602fb14e1870ea3817cfb31e77c2775d8", "invocations", 1792,
     (("reallocations", "reference_reallocations", "candidate_reallocations"),
      ("requested_bytes", "reference_requested_bytes", "candidate_requested_bytes"))),
    ("context-document-scratch", "context-document-stack-20261021", "71e784faff8791cce02b1d98d30adff5278cda37",
     "71638d9db2817c1a1d24edddb467e2cbf7a4ec9e", "constructions", 2304,
     (("allocations", "reference_allocations", "candidate_allocations"),
      ("requested_bytes", "reference_requested_bytes", "candidate_requested_bytes"))),
    ("trust-component-cstring", "trust-component-stack-20261021", "a7123c37985bd129b10eab969ab260a1fc551745",
     "a72457aade616e67d2fc5693f6105c8063d6e9e1", "valid_constructions", 5376,
     (("allocations", "reference_valid_allocations", "candidate_valid_allocations"),
      ("requested_bytes", "reference_valid_bytes", "candidate_valid_bytes"))),
    ("collector-peer-growth", "collector-peer-capacity-20261022", "6e3cea6313b492d50a58eb2986449045a805978f",
     "da44628a85779977d5d1cb2f32e324239165eeff", "main_constructions", 7936,
     (("allocations", "reference_allocations", "candidate_allocations"),
      ("reallocations", "reference_reallocations", "candidate_reallocations"),
      ("requested_bytes", "reference_requested_bytes", "candidate_requested_bytes"))),
)
REPLACED = {
    "context-document-scratch": ("diagnostic_worker_context.rs", "let mut buffer = document_buffer::Scratch::new(length);"),
    "trust-component-cstring": ("diagnostic_worker_trust_input.rs", "let name = component_name::Name::new(name).map_err(|_| TrustInputError::Invalid)?;"),
    "collector-peer-growth": ("diagnostic_kernel_collector_inputs.rs", "let mut gids = Vec::with_capacity(supplementary.len() + 1);"),
}


def canonical(path):
    return Path(path).read_text(encoding="utf-8").replace("\r", "")


def verify_packet(packet, pin):
    # Refuse linked packet roots/ancestors before reading their inventory.
    for path in (Path(packet), *Path(packet).parents):
        need(not path.is_symlink() and not getattr(path, "is_junction", lambda: False)(), "Linked input")
    return prior.verify_packet(packet, pin)


def methods(root):
    pairs = [("offline-reserved-leaf-audit-074/audit.py", AUDIT, "audit.py"),
             ("local-profile-session-074/attribute_material_rounds.py", "held-material-round-attribution-20261021", "attribute_material_rounds.py")]
    pairs += [("local-profile-session-074/" + name, "held-material-live-20261021", name)
              for name in ("attribute_partial.py", "review_material_live.py", "run_material_live.py", "runner.py")]
    for name, packet, member in pairs:
        need(canonical(root / "tools" / name) == canonical(root / BASE / packet / "method" / member),
             f"Reader method drift: {name}")
    return [name for name, _, _ in pairs]


def sources(root, old):
    pins = dict(old["sources"], **UPDATES)
    texts = {}
    for name, pin in pins.items():
        text = canonical(root / "tools/long-run-supervisor-074/src" / name)
        need(hashlib.sha256(text.encode("utf-8")).hexdigest() == pin, f"Source drift: {name}")
        texts[name] = text
    return pins, texts


def nonnegative(value):
    need(type(value) is int and value >= 0, "Counter must be a nonnegative integer")
    return value


def cohort(summary, spec):
    identity, packet, candidate, baseline, key, count, metrics = spec
    need(summary["candidate_source"] == candidate and summary["baseline_source"] == baseline,
         "Cohort source identity drift")
    need(summary["original_windows_seconds"] == [300, 60, 10]
         and summary["first_refusal_ages_ns"] == [10042390247, 10010835739], "Original refusal/window drift")
    for field in ("timing_improvement_proven", "server", "product"):
        need(summary[field] is False, f"Promotion forbidden: {field}")
    need([r["lane"] for r in summary["lanes"]] == ["194", "188"], "Cohort lane identity drift")
    lanes = []
    for lane in summary["lanes"]:
        row = lane.get("totals", lane)
        operations = nonnegative(row["cases"]) * nonnegative(row["iterations_per_case"]) if key is None else nonnegative(row[key])
        need(operations == count, "Cohort construction count drift")
        pairs = {}
        for unit, before, after in metrics:
            reference, candidate_value = nonnegative(row[before]), nonnegative(row[after])
            need(candidate_value <= reference, "Retained corpus regressed")
            pairs[unit] = dict(reference=reference, candidate=candidate_value)
        lanes.append(dict(lane=lane["lane"], constructions=operations, counters=pairs))
    need(lanes[0]["counters"] == lanes[1]["counters"], "Compiler counters differ")
    return dict(id=identity, packet=packet, baseline_source=baseline, candidate_source=candidate,
                scope="separate-pure-allocation-corpus", lanes=lanes, full_path_timing_proven=False)


def leaf_rows(old, texts):
    retained = {spec[0] for spec in SPECS}
    result = []
    for leaf in old["leaves"]:
        identity = leaf["id"]
        original = leaf["witness"]
        name, fragment = REPLACED.get(identity, (Path(original["path"]).name, original["fragment"]))
        result.append(dict(id=identity, prior_disposition=leaf["disposition"],
            disposition="allocation-only-retained" if identity in retained else leaf["disposition"],
            witness=prior.witness(texts, name, fragment), exclusive_leaf_time_measured=False))
    need(len(result) == 13 and len({r["id"] for r in result}) == 13, "Leaf membership drift")
    need({r["id"] for r in result if r["disposition"] == "allocation-only-retained"} == retained, "Candidate join drift")
    return result


def analyze(root=ROOT):
    root = Path(root)
    packets = [verify_packet(root / BASE / name, pin) for name, pin in PACKETS.items()]
    need(sum(p["members"] for p in packets) == 491, "Input member total drift")
    checked_methods = methods(root)
    old = load(root / BASE / AUDIT / "facts.json")
    need(old["audited_runtime_source"] == prior.RUNTIME_SOURCE and old["sources"] == prior.SOURCE_PINS,
         "Old source identity drift")
    pins, texts = sources(root, old)
    need(sha(root / "docs/testing/performance/0.74/qualification-manifest.toml") == prior.QUALIFICATION, "Qualification drift")
    rounds = prior.rounds.analyze(root / BASE / prior.rounds.PACKET)
    need(rounds == load(root / BASE / "held-material-round-attribution-20261021/facts.json"), "Old round facts differ")
    for lane, checked in zip(old["lanes"], rounds["lanes"]):
        rows = prior.rounds.prior.rows(root / BASE / prior.rounds.PACKET / f"{lane['lane']}.log", "HC074-HELD-PHASE ")
        need(lane["lane"] == checked["lane"] and lane["first_original_refusal"] == checked["first_original_refusal"]
             and lane["reserved_live"] == prior.reserved_step(rows[-1]), "Old ReservedLive facts differ")
    return dict(schema="offline-reserved-reconciliation-074-v1", audited_runtime_source=RUNTIME_SOURCE,
        measured_source=prior.rounds.SOURCE, input_packets=packets, input_members_verified=491,
        reader_methods=checked_methods, sources=pins, qualification_sha256=prior.QUALIFICATION,
        old_reserved_live_lanes=old["lanes"], leaves=leaf_rows(old, texts),
        allocation_cohorts=[cohort(load(root / BASE / spec[1] / "summary.json"), spec) for spec in SPECS],
        interpretation=dict(corpora_are_not_additive=True, old_call_counts_not_extrapolated=True,
            old_wall_is_not_current_timing=True, kernel_live_auth_views_overlap=True,
            requested_bytes_are_not_rss_or_retention=True, leaf_review_order_is_not_elapsed_rank=True),
        original_windows_seconds=[300, 60, 10], new_measurements=0, new_profile_invocations=0,
        runtime_changed=False, observer_changed=False, clocks_changed=False, services_changed=False,
        original_material_gate_passed=False, cleanup_eof_fence_proven=False,
        original_delivery_proven=False, all_production_writers_proven=False,
        full_sigkill_matrix=False, lost_ack_matrix=False, reconnect_gate=False,
        full_path_speedup_proven=False, server=False, product=False)


def require_output(path):
    need(Path(path).resolve() == (ROOT / OUTPUT / "facts.json").resolve(), "Separate reconciliation facts required")
    for node in (Path(path), *Path(path).parents):
        need(not node.is_symlink() and not getattr(node, "is_junction", lambda: False)(), "Linked output")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    parser.add_argument("--packet-sha256", help="Verify the final reconciliation seal as well; read-only")
    args = parser.parse_args()
    require_output(args.output)
    if args.packet_sha256:
        need(args.verify, "Packet review must be read-only")
        verify_packet(args.output.parent, args.packet_sha256)
    prior.rounds.prior.publish(args.output, analyze(), args.verify)
    print("Offline reconciliation verified: 11 input seals, 491 members, 5 separate cohorts, 8 unresolved owners; no gate promotion")
