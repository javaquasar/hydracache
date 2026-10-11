"""Verify original campaign fence evidence; never authorize enrollment or start."""
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


def capture(name):
    return (packet / name).read_text(encoding="utf-8")


def raw_files(directory, entries):
    assert {p.name for p in directory.glob("*.log")} == {e["name"] for e in entries}
    assert len(entries) == len({e["name"] for e in entries})
    for e in entries:
        assert Path(e["name"]).name == e["name"]
        raw = (directory / e["name"]).read_bytes()
        assert len(raw) == e["bytes"] and digest(raw) == e["sha256"], e["name"]


def totals(name):
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", capture(name))
    assert rows, name
    return [sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)]


assert m["state"] == "retained-original-campaign-fence-short-observation"
for key in m["closed_permissions"]:
    assert m["closed_permissions"][key] is False, key
assert m["same_existing_campaign_flock_tested"] is True
assert m["legacy_claim_after_guard_drop_demonstrated"] is True
raw_files(packet, m["files"])
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", m[prefix + "_source"] + "^{tree}").decode().strip() == m[prefix + "_tree"]
assert git("merge-base", m["preregistration_source"], m["implementation_source"]).decode().strip() == m["preregistration_source"]
for path, expected in m["implementation_source_sha256"].items():
    raw = git("show", m["implementation_source"] + ":" + path)
    assert digest(raw) == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == raw, path
for path in m["unchanged_legacy_sources"]:
    assert git("show", m["baseline_source"] + ":" + path) == git("show", m["implementation_source"] + ":" + path), path
for name, counts in m["test_totals"].items():
    assert totals(name) == counts, name
for name in ("11-repeat-1.log", "12-repeat-2.log", "13-repeat-3.log"):
    assert totals(name) == [10, 0]
    for test in m["repeated_unit_test_names"]:
        assert "::" + test + " ... " in capture(name), (name, test)
    assert "seed=0x746" in capture(name)
for name, marker in m["required_log_markers"].items():
    assert marker in capture(name), (name, marker)
for name in m["green_compile_logs"]:
    assert "Finished `dev` profile" in capture(name) and "error:" not in capture(name), name
for name in ("23-format.log", "33-final-format.log"):
    assert (packet / name).read_bytes() == b""
for e in m["files"]:
    assert e["observed_exit"] == e["expected_exit"], e["name"]

# Historical evidence binds its recorded source, not today's extended API.
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
gated = tomllib.loads((repo / "docs/testing/gated-test-registry.toml").read_text(encoding="utf-8"))
gate = next(g for g in gated["gate"] if g["id"] == "cfg.hydracache-long-run-supervisor-074.diagnostic-original-fence")
assert gate["command"]["args"] == ["test", "-p", "hydracache-long-run-supervisor-074", "--test", "diagnostic_original_fence", "--locked"]
contract = tomllib.loads((repo / "docs/testing/performance/0.74/diagnostic-original-fence-local-contract.toml").read_text(encoding="utf-8"))
assert contract["baseline_source"] == m["baseline_source"]
for key in ("existing_campaign_lock_name_only", "lock_creation_or_independent_lock_forbidden",
            "original_reserved_reader_exclusively_borrowed", "initial_busy_does_not_revoke_healthy_storage"):
    assert contract[key] is True
for key in ("continuous_or_durable_campaign_exclusion_proved", "trusted_state_publication_implemented",
            "installer_or_campaign_admission_changed", "worker_execution_allowed", "ship_allowed"):
    assert contract[key] is False
for path in m["documentation_link_sources"]:
    for target in re.findall(r"\]\(([^)]+)\)", (repo / path).read_text(encoding="utf-8")):
        if target.startswith(("http:", "https:", "#", "/")):
            continue
        assert ((repo / path).parent / unquote(target.split("#")[0])).exists(), (path, target)
print("original campaign fence packet verified; durable admission, enrollment and start remain closed")
