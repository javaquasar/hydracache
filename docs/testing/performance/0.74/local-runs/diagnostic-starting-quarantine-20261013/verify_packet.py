"""Read-only exact-source verification of the local Starting quarantine packet."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
M = json.loads((PACKET / "manifest.json").read_text(encoding="utf-8"))

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def lf(raw):
    return raw.replace(b"\r\n", b"\n")

def git_bytes(commit, path):
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)

assert M["schema_version"] == 1
assert sha(lf((PACKET / "verify_packet.py").read_bytes())) == M["verifier_sha256"]
assert M["baseline_source"] == "7f18ae34e2de2c4163bba305f443ced367c25cb6"
for field in ("baseline_source", "preregistration_source", "implementation_source"):
    assert re.fullmatch(r"[0-9a-f]{40}", M[field])
    subprocess.check_call(["git", "cat-file", "-e", M[field] + "^{commit}"], cwd=ROOT)
expected_sources = {
    "tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs",
    "tools/long-run-supervisor-074/src/diagnostic_starting_intent_tests.rs",
}
assert {entry["path"] for entry in M["source_files"]} == expected_sources
for entry in M["source_files"]:
    committed = lf(git_bytes(M["implementation_source"], entry["path"]))
    assert sha(committed) == entry["sha256_lf"]
    assert lf((ROOT / entry["path"]).read_bytes()) == committed
changed = set(subprocess.check_output(
    ["git", "diff", "--name-only", M["baseline_source"], M["implementation_source"]],
    cwd=ROOT, text=True).splitlines())
assert changed == expected_sources | {
    "docs/testing/performance/0.74/diagnostic-starting-quarantine-local-design.md"
}
for entry in M["frozen_files"]:
    original = lf(git_bytes(M["baseline_source"], entry["path"]))
    assert sha(original) == entry["sha256_lf"]
    assert lf((ROOT / entry["path"]).read_bytes()) == original
qualification = ROOT / "docs/testing/performance/0.74/qualification-manifest.toml"
assert sha(qualification.read_bytes()) == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"

captures = {}
for entry in M["captures"]:
    name = entry["path"]
    assert name == entry["id"] + ".log" and Path(name).name == name
    assert entry["id"] not in captures
    raw = (PACKET / name).read_bytes()
    assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"]
    assert entry["observed_exit"] == entry["expected_exit"] or entry["id"] == "functional-first"
    captures[entry["id"]] = raw.decode("utf-8")
assert len(captures) == M["capture_count"]
assert M["capture_count"] == 30
assert "16 passed; 0 failed" in captures["baseline"]
assert "method `quarantine`" in captures["red"] and "E0599" in captures["red"]
assert M["negative_captures"] == ["functional-first"]
assert "compiler unexpectedly panicked" in captures["functional-first"]
assert next(e for e in M["captures"] if e["id"] == "functional-first")["observed_exit"] == 101
assert "unused_mut" in captures["msrv-diagnostic"]
assert "7 passed; 0 failed" in captures["functional-repeat-1"]
names = M["functional_test_names"]
assert len(names) == 8 and len(set(names)) == 8
source = (ROOT / "tools/long-run-supervisor-074/src/diagnostic_starting_intent_tests.rs").read_text()
for index in (1, 2, 3):
    log = captures[f"functional-final-{index}"]
    assert "8 passed; 0 failed; 0 ignored" in log
    for name in names:
        assert f"fn {name}(" in source and re.search(re.escape(name) + r" \.\.\. ok", log)

def passed(log):
    return sum(int(value) for value in re.findall(r"test result: ok\. (\d+) passed;", log))
assert passed(captures["linux-regression"]) == 668
assert passed(captures["windows-regression"]) == 204
assert passed(captures["root-contracts"]) == 166
assert passed(captures["linux-doctests"]) == 52
assert passed(captures["root-artifacts-final"]) == 13
for name in ("doc-check-final", "book-final", "format-final", "toolchain", "performance-contract"):
    assert name in captures
for entry in M["captures"]:
    if entry["source_stage"] == "final":
        assert entry["observed_exit"] == entry["expected_exit"]
for name in ("qualification-refusal", "ship-refusal"):
    entry = next(e for e in M["captures"] if e["id"] == name)
    assert entry["expected_exit"] == 1 and entry["observed_exit"] == 1
assert M["qualification_output_created"] is False
assert M["server_operation_performed"] is False
assert M["performance_measurements"] == []
assert M["new_authority_sigkill_proof"] is False

contract = tomllib.loads((ROOT / "docs/testing/performance/0.74/diagnostic-starting-quarantine-local-contract.toml").read_text())
assert contract["functional_cases_per_repeat"] == 8 and contract["targeted_repetitions"] == 3
assert contract["logical_authority_faults_per_repeat"] == 4
for name in ("new_durable_schema", "new_storage_write_boundary", "new_sigkill_authority_append_proof",
             "completion_or_cleanup_authenticated", "worker_execution_allowed", "server_operation_performed",
             "qualification_allowed", "ship_allowed", "performance_claim_allowed"):
    assert contract[name] is False
registry = tomllib.loads((ROOT / "docs/testing/release-evidence/0.74.toml").read_text())
w11 = next(item for item in registry["work_item"] if item["id"] == "W11")
prefix = PACKET.relative_to(ROOT).as_posix()
for name in ("README.md", "manifest.json", "verify_packet.py"):
    assert f"{prefix}/{name}" in w11["required_artifacts"]
for name in names:
    assert any(test["function"] == name and test["source"] in expected_sources for test in w11["required_tests"])
print("PASS: exact-source quarantine packet; three eight-case repeats; closed execution and qualification")
