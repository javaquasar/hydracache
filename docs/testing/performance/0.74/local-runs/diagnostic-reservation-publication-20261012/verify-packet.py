"""Verify fixed initial reservation publication evidence; never authorize reservation or worker start."""
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
assert m["state"] == "retained-original-reservation-publication-local"
assert m["fixed_initial_reserved_publication_implemented"] is True
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
assert totals("37-corrected-linux-regression.log") == [583, 1]
assert totals("42-corrected-windows-regression.log") == [196, 0]
assert totals("43-corrected-root-regression.log") == [159, 0]
assert totals("38-corrected-linux-doctests.log") == [34, 0]
assert "reads_a_real_loaded_service_over_dbus ... ignored" in capture("37-corrected-linux-regression.log")
for name in ("34-corrected-repeat-1.log", "35-corrected-repeat-2.log", "36-corrected-repeat-3.log"):
    assert totals(name) == [15, 0]
    for test in m["repeated_unit_test_names"]:
        assert "::" + test + " ... " in capture(name), (name, test)
    assert "seed=0x74b" in capture(name)
assert len(set(m["repeated_unit_test_names"])) == 15
for name, marker in m["required_log_markers"].items():
    assert marker in capture(name), (name, marker)
for name in m["green_compile_logs"]:
    assert "Finished `dev` profile" in capture(name) and "error:" not in capture(name), name
for name in ("22-format.log", "49-corrected-format.log", "56-final-format.log"):
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
assert len(m["new_tests"]) == 17
assert {(t["source"], t["function"]) for t in m["new_tests"]} <= {
    (t["source"], t["function"]) for t in w11["required_tests"]}
for t in m["new_tests"]:
    assert "fn " + t["function"] + "(" in (repo / t["source"]).read_text(encoding="utf-8"), t
assert packet.relative_to(repo).as_posix() + "/manifest.json" in w11["required_artifacts"]
for path in ("tools/long-run-supervisor-074/src/diagnostic_reservation_publication.rs",
             "tools/long-run-supervisor-074/src/diagnostic_reservation_publication_tests.rs",
             "docs/testing/performance/0.74/diagnostic-reservation-publication-local-contract.toml"):
    assert path in w11["required_sources"], path
contract_path = "docs/testing/performance/0.74/diagnostic-reservation-publication-local-contract.toml"
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
old_encoder = git("show", m["baseline_source"] + ":tools/long-run-supervisor-074/src/diagnostic_lease.rs")
new_encoder = git("show", m["implementation_source"] + ":tools/long-run-supervisor-074/src/diagnostic_lease.rs")
assert new_encoder.replace(b"pub(crate) fn encode(", b"fn encode(") == old_encoder
assert m["logical_interruption_stages"] == ["BeforeCreate", "StateCreated", "PrefixWritten", "BytesWritten", "FileSynced", "DirectorySynced", "DocumentBound", "Acknowledging"]
assert "result_large_err" in capture("13-linux-clippy.log")
assert "unused variable" in capture("13-linux-clippy.log")
print("fixed initial reservation publication packet verified; activation, enrollment and start remain closed")
