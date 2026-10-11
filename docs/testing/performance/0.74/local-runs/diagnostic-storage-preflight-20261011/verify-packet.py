"""Verify local storage evidence, not installed enrollment or launch permission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import unquote

packet = Path(__file__).resolve().parent
repo = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
m = json.loads((packet / "manifest.json").read_bytes())


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def git(*args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def text(name):
    return (packet / name).read_text(encoding="utf-8")


def totals(name):
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))
    assert rows, name
    return sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)


def raw_files(directory, entries):
    assert len({e["name"] for e in entries}) == len(entries)
    assert {p.name for p in directory.glob("*.log")} == {e["name"] for e in entries}
    for e in entries:
        assert Path(e["name"]).name == e["name"], e
        raw = (directory / e["name"]).read_bytes()
        assert len(raw) == e["bytes"] and digest(raw) == e["sha256"], e["name"]


assert m["state"] == "retained-storage-preflight-not-worker-pilot"
assert not m["host_operation_performed"] and not m["execution_allowed"]
raw_files(packet, m["files"])
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", m[prefix + "_source"] + "^{tree}").decode().strip() == m[prefix + "_tree"]
for path, expected in m["implementation_source_sha256"].items():
    committed = git("show", m["implementation_source"] + ":" + path)
    assert digest(committed) == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == committed, path
for name, counts in m["test_totals"].items():
    assert totals(name) == tuple(counts), name
for name in ("34-repeat-1.log", "34-repeat-2.log", "34-repeat-3.log"):
    assert totals(name) == (4, 0), name
    for test in m["repeated_unit_test_names"]:
        assert f"::{test} ... ok" in text(name), (name, test)
for name, marker in m["required_log_markers"].items():
    assert marker in text(name), (name, marker)
for name in m["green_compile_logs"]:
    assert "Finished `dev` profile" in text(name) and "error:" not in text(name), name
for name in ("13-shell-syntax.log", "19-format-check.log"):
    assert (packet / name).read_bytes() == b"", name

# Earlier packets remain immutable historical proof; their current-source equality
# verifier is intentionally not applicable after this ownership/provisioning amendment.
historical = repo / m["historical_packet"]
prior_raw = (historical / "manifest.json").read_bytes()
assert digest(prior_raw) == m["historical_manifest_sha256"]
prior = json.loads(prior_raw)
raw_files(historical, prior["files"])
assert git("rev-parse", prior["implementation_source"] + "^{tree}").decode().strip() == prior["implementation_tree"]
for path, expected in prior["implementation_source_sha256"].items():
    assert digest(git("show", prior["implementation_source"] + ":" + path)) == expected, path
q_path = repo / "docs/testing/performance/0.74/qualification-manifest.toml"
assert digest(q_path.read_bytes()) == m["qualification_manifest_sha256"]
q = tomllib.loads(q_path.read_text(encoding="utf-8"))
assert len(q["contract_inputs"]) == 8
mismatches = []
for entry in q["contract_inputs"]:
    old = git("show", m["baseline_source"] + ":" + entry["path"])
    assert (repo / entry["path"]).read_bytes().replace(b"\r\n", b"\n") == old, entry["path"]
    if digest(old) != entry["sha256"]:
        mismatches.append(entry["path"])
assert mismatches == ["docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"]
registry = tomllib.loads((repo / "docs/testing/release-evidence/0.74.toml").read_text(encoding="utf-8"))
w11 = next(w for w in registry["work_item"] if w["id"] == "W11")
assert {(t["source"], t["function"]) for t in m["new_tests"]} <= {
    (t["source"], t["function"]) for t in w11["required_tests"]}
assert packet.relative_to(repo).as_posix() + "/manifest.json" in w11["required_artifacts"]
c = tomllib.loads((repo / "docs/testing/performance/0.74/diagnostic-storage-preflight-contract.toml").read_text(encoding="utf-8"))
assert c["preflight_read_only"] and c["leaf_mode"] == 0o700
for key in ("intent_publication_or_authority_join_implemented", "worker_execution_allowed",
            "qualification_allowed", "ship_allowed", "server_operation_performed", "performance_claim_allowed"):
    assert c[key] is False, key
for path in (packet / "README.md", repo / "docs/testing/performance/0.74/README.md",
             repo / "docs/testing/performance/0.74/diagnostic-storage-preflight-design.md",
             repo / "docs/testing/performance/0.74/production-readiness-and-server-launch-20261011.md",
             repo / "docs/articles/007-optimization-starts-with-measurement.md"):
    for target in re.findall(r"\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
        if "://" in target or target.startswith("#"):
            continue
        assert (path.parent / unquote(target.split("#")[0])).exists(), (path, target)
print(f"verified {len(m['files'])} closed captures, repeated storage checks and historical source; frozen inputs unchanged, no launch admission")
