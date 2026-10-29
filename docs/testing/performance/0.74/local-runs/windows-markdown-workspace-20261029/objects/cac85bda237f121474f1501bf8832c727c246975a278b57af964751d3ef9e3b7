"""Read-only count export. No claim to reconstruct original temporal ordering."""
import argparse
import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("partial", ROOT / "tools/local-profile-session-074/attribute_partial.py")
partial = importlib.util.module_from_spec(spec)
spec.loader.exec_module(partial)


def export():
    packet = ROOT / "docs/testing/performance/0.74/local-runs" / partial.PACKET
    partial.verify_inventory(packet)
    original = []
    for lane in ("194", "188"):
        phases = partial.rows(packet / f"{lane}-Profile-0.log", "HC074-HELD-PHASE ")
        output = partial.phase_facts(phases[2])
        raw = phases[2]
        steps = []
        for index, row in enumerate(raw["round_steps"]):
            values = list(row["roots"]) + [n["calls"] for n in row["kernel"]] + [e["calls"] for e in row["edges"]]
            partial.require(all(type(n) is int and n % 46 == 0 for n in values), "Per-round divisibility changed")
            edges = [[0]*10 for _ in range(10)]
            for edge in row["edges"]:
                edges[edge["parent"]][edge["child"]] = edge["calls"]//46
            steps.append(dict(calls=[n["calls"]//46 for n in row["kernel"]],
                              roots=[n//46 for n in row["roots"]], edges=edges))
        partial.require(sum(n["calls"] for n in output["kernel"]) == 894608, "Original topology total differs")
        original.append(steps)
    partial.require(original[0] == original[1], "Compiler output topologies differ")
    return dict(schema="observer-kernel-topology-074-v1", seed=0x074c051e, rounds=46,
                input_source=partial.SOURCE, input_inventory_sha256=partial.INVENTORY, steps=original[0])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--verify", action="store_true")
    args = parser.parse_args()
    partial.publish(args.output, export(), args.verify)
    print("exact per-step nodes/roots/edges; canonical ordering only; live guards=0")
