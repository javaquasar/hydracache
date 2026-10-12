"""Verify actual local process interruption evidence; never authorize reservation or worker start."""
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


assert m["schema_version"] == 1
assert m["state"] == "retained-original-process-crash-local"
assert m["actual_process_interruption_checked"] is True
assert all(value is False for value in m["closed_permissions"].values())
assert m["test_environment"]["root_for_local_tests"] is False
raw_files(packet, m["files"])
for e in m["files"]:
    assert e["observed_exit"] == e["expected_exit"], e["name"]
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", m[prefix + "_source"] + "^{tree}").decode().strip() == m[prefix + "_tree"]
for older, newer in (("baseline_source", "preregistration_source"),
                     ("preregistration_source", "implementation_source")):
    assert git("merge-base", m[older], m[newer]).decode().strip() == m[older]
for path, expected in m["implementation_source_sha256"].items():
    raw = git("show", m["implementation_source"] + ":" + path)
    assert digest(raw) == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == raw, path
for path in m["unchanged_sources"]:
    assert git("show", m["baseline_source"] + ":" + path) == git("show", m["implementation_source"] + ":" + path), path

for name, counts in m["test_totals"].items():
    assert totals(name) == counts, name
assert totals("16-linux-regression.log") == [599, 4]
assert totals("18-windows-regression.log") == [196, 0]
assert totals("05-root-regression.log") == [161, 0]
assert totals("17-linux-doctests.log") == [38, 0]
assert "reads_a_real_loaded_service_over_dbus ... ignored" in capture("16-linux-regression.log")
for name in ("13-repeat-1.log", "14-repeat-2.log", "15-repeat-3.log"):
    assert totals(name) == [3, 0]
    for test in m["repeated_unit_test_names"]:
        assert "::" + test + " ... " in capture(name), (name, test)
    assert "P0 seed=0x074d" in capture(name)
assert len(set(m["repeated_unit_test_names"])) == 3
for name, marker in m["required_log_markers"].items():
    assert marker in capture(name), (name, marker)
for name in m["green_compile_logs"]:
    assert "Finished `dev` profile" in capture(name) and "error:" not in capture(name), name
for name in ("24-format.log", "29-final-format.log"):
    assert (packet / name).read_bytes() == b""

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
assert len(m["new_tests"]) == 7
assert {(t["source"], t["function"]) for t in m["new_tests"]} <= {
    (t["source"], t["function"]) for t in w11["required_tests"]}
for t in m["new_tests"]:
    assert "fn " + t["function"] + "(" in (repo / t["source"]).read_text(encoding="utf-8"), t
assert packet.relative_to(repo).as_posix() + "/manifest.json" in w11["required_artifacts"]
for path in ("tools/long-run-supervisor-074/src/diagnostic_process_crash_tests.rs",
             "docs/testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md",
             "docs/testing/performance/0.74/diagnostic-process-crash-local-contract.toml"):
    assert path in w11["required_sources"], path
contract_path = "docs/testing/performance/0.74/diagnostic-process-crash-local-contract.toml"
contract = tomllib.loads((repo / contract_path).read_text(encoding="utf-8"))
assert contract["baseline_source"] == m["baseline_source"]
assert {key: value for key, value in contract.items() if isinstance(value, bool)} == m["contract_boolean_pins"]
assert m["closed_permissions"] == {key: value for key, value in m["contract_boolean_pins"].items() if value is False}
assert git("show", m["preregistration_source"] + ":" + contract_path) == git("show", m["implementation_source"] + ":" + contract_path)
for path in m["documentation_link_sources"]:
    for target in re.findall(r"\]\(([^)]+)\)", (repo / path).read_text(encoding="utf-8")):
        if target.startswith(("http:", "https:", "#", "/")):
            continue
        assert ((repo / path).parent / unquote(target.split("#")[0])).exists(), (path, target)


fence_path = "tools/long-run-supervisor-074/src/diagnostic_empty_reservation_fence.rs"
old_fence = git("show", m["baseline_source"] + ":" + fence_path)
new_fence = git("show", m["implementation_source"] + ":" + fence_path)
assert new_fence.replace(b"\n#[cfg(test)]\npub(crate) use tests::{inspect_empty_fence_at_for_test, with_owned_empty_fence_at_for_test};\n", b"") == old_fence
for name in ("13-repeat-1.log", "14-repeat-2.log", "15-repeat-3.log"):
    rows = re.findall(r"P0 case=([^ ]+) terminated=(SIGKILL|control) signal=(\d+) fresh_processes=2 retained=unchanged original_lock=released helper=terminal", capture(name))
    assert len(rows) == 26
    crash = {label for label, kind, signal in rows if kind == "SIGKILL" and signal == "9"}
    controls = {label for label, kind, signal in rows if kind == "control" and signal == "0"}
    assert len(crash) == 24 and len(controls) == 2
    assert crash == set(m["required_crash_cases"])
    assert controls == {"reserved:AfterReturn", "consumption:AfterReturn"}
    assert "P0 seed=0x074d stages=15" in capture(name)
    assert "P0 seed=0x074d stages=9" in capture(name)
assert "Timeout" in capture("03-first-process-crash.log")
assert "0 passed; 3 failed" in capture("03-first-process-crash.log")
assert "nonminimal_bool" in capture("07-linux-clippy.log")
assert not (packet / "08-linux-msrv.log").exists()
gates = tomllib.loads((repo / "docs/testing/gated-test-registry.toml").read_text(encoding="utf-8"))
for gate_id, test_name in zip(m["manual_gate_ids"], m["repeated_unit_test_names"]):
    g = next(g for g in gates["gate"] if g["id"] == gate_id)
    assert g["command"]["platform"] == "linux"
    assert g["ship_mandatory"] is False and g["tier"] == "manual"
    assert "--ignored" in g["command"]["args"] and "--exact" in g["command"]["args"]
    assert m["test_name_prefix"] + test_name in g["command"]["args"]
assert set(m["manual_gate_ids"]) <= set(w11["gated_gate_ids"])
plan_raw = git("show", m["preregistration_source"] + ":" + m["intermediate_plan_path"])
assert digest(plan_raw) == m["preregistered_plan_sha256"]
assert (repo / m["intermediate_plan_path"]).read_bytes().replace(b"\r\n", b"\n") == plan_raw + m["plan_status_suffix"].encode()
print("local process-crash packet verified; P0 local proof only, P1-P5 and product/server admission remain open")
