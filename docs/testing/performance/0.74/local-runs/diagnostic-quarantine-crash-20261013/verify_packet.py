"""Read-only exact-source local proof of quarantine authority process loss."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
M = json.loads((PACKET / "manifest.json").read_text(encoding="utf-8"))
def lf(raw): return raw.replace(b"\r\n", b"\n")
def sha(raw): return hashlib.sha256(raw).hexdigest()
def git(commit, path): return lf(subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT))
assert M["schema_version"] == 1
assert M["baseline_source"] == "b6c76476be7b99117a267dfcd570e4dffa8c7e00"
assert sha(lf((PACKET / "verify_packet.py").read_bytes())) == M["verifier_sha256"]
assert sha(lf((PACKET / "inspect_production_symbols.py").read_bytes())) == M["production_probe_sha256_lf"]
for field in ("baseline_source", "preregistration_source", "implementation_source"):
    assert re.fullmatch(r"[0-9a-f]{40}", M[field])
    subprocess.check_call(["git", "cat-file", "-e", M[field] + "^{commit}"], cwd=ROOT)
sources = {
    "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_authority_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_authority_checkpoint.rs",
    "tools/long-run-supervisor-074/src/diagnostic_process_crash_tests.rs",
    "docs/testing/gated-test-registry.toml",
}
assert {e["path"] for e in M["source_files"]} == sources
for entry in M["source_files"]:
    raw = git(M["implementation_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((ROOT / entry["path"]).read_bytes()) == raw
changed = set(subprocess.check_output(["git", "diff", "--name-only",
    M["baseline_source"], M["implementation_source"]], cwd=ROOT, text=True).splitlines())
assert changed == sources | {"docs/testing/performance/0.74/diagnostic-quarantine-crash-local-design.md"}
engine_path = "tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs"
engine = git(M["implementation_source"], engine_path).decode()
declaration = '#[cfg(test)]\n#[path = "diagnostic_authority_checkpoint.rs"]\npub(crate) mod publication_checkpoint;\n\n'
call = '    #[cfg(test)]\n    publication_checkpoint::observe(stage);\n'
assert engine.count(declaration) == 1 and engine.count(call) == 1
assert engine.replace(declaration, "").replace(call, "").encode() == git(M["baseline_source"], engine_path)
for entry in M["unchanged_sources"] + M["frozen_files"]:
    raw = git(M["baseline_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((ROOT / entry["path"]).read_bytes()) == raw
assert sha((ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"

logs = {}
for entry in M["captures"]:
    assert entry["path"] == entry["id"] + ".log"
    assert entry["id"] not in logs
    raw = (PACKET / entry["path"]).read_bytes()
    assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"]
    if entry["id"] in ("process-first", "root-contracts"):
        assert entry["expected_exit"] == 0 and entry["observed_exit"] == 101
    else:
        assert entry["expected_exit"] == entry["observed_exit"]
    logs[entry["id"]] = raw.decode("utf-8")
assert len(logs) == M["capture_count"]
assert M["capture_count"] == 39
assert M["retained_development_failures"] == ["process-first", "root-contracts"]
assert "8 passed; 0 failed" in logs["baseline"]
assert "E0433" in logs["red"] and "publication_checkpoint" in logs["red"]
assert "E0716" in logs["process-first"]
assert "missing journal invariant FileExt::try_lock_exclusive" in logs["root-contracts"]
assert "3 passed; 0 failed" in logs["hook-functional"]

cases = ("BeforeRefusal", "RecordCreated", "RecordSynced", "MarkerCreated", "DurableCommit", "AfterReturn")
def quarantine_matrix(log):
    assert "seed=0x074d stages=6" in log
    for stage in cases:
        marker = f"P1 case=quarantine:{stage} terminated=SIGKILL signal=9 fresh_processes=2 retained=unchanged original_lock=released helper=terminal"
        assert log.count(marker) == 1
    assert log.count("P1 case=quarantine:AfterReturn terminated=control signal=0 fresh_processes=2 retained=unchanged original_lock=released helper=terminal") == 1
for repeat in (1, 2, 3):
    log = logs[f"process-final-{repeat}"]
    quarantine_matrix(log)
    assert "1 passed; 0 failed" in log
quarantine_matrix(logs["process-regression"])
assert logs["process-regression"].count("terminated=SIGKILL signal=9") == 72
assert logs["process-regression"].count("terminated=control signal=0") == 7
assert "8 passed; 0 failed" in logs["process-regression"]
def passed(log):
    return sum(int(x) for x in re.findall(r"test result: ok\. (\d+) passed;", log))
assert passed(logs["linux-regression"]) == 673
assert passed(logs["windows-final-regression"]) == 204
assert passed(logs["root-final-contracts"]) == 166
assert passed(logs["linux-doctests"]) == 52
assert passed(logs["hook-final-functional"]) == 3
assert passed(logs["hook-final-protocol"]) == 2
assert passed(logs["quarantine-final-functional"]) == 8
assert passed(logs["root-artifacts-final"]) == 13
for name in ("doc-check-final", "book-final", "format-final", "performance-contract"):
    assert name in logs
receipt = json.loads(logs["production-symbols"].strip())
assert receipt["cfg_test_observer_symbols"] == "absent" and receipt["production_activation"] is False
assert re.fullmatch(r"[0-9a-f]{64}", receipt["sha256"])
assert receipt["artifact"].endswith("/libhydracache_long_run_supervisor_074.rlib")
assert M["qualification_output_created"] is False
assert M["server_operation_performed"] is False
assert M["performance_measurements"] == []
for name in ("qualification-refusal", "ship-refusal"):
    entry = next(e for e in M["captures"] if e["id"] == name)
    assert entry["expected_exit"] == 1 and entry["observed_exit"] == 1
contract = tomllib.loads((ROOT / "docs/testing/performance/0.74/diagnostic-quarantine-crash-local-contract.toml").read_text())
assert contract["sigkill_cases_per_repeat"] == 6 and contract["controls_per_repeat"] == 1
assert contract["fresh_processes_per_repeat"] == 14 and contract["targeted_repetitions"] == 3
for name in ("new_durable_schema", "new_production_storage_boundary", "host_reboot_proved",
             "physical_power_loss_proved", "completion_or_cleanup_authenticated", "worker_execution_allowed",
             "server_operation_performed", "qualification_allowed", "ship_allowed", "performance_claim_allowed"):
    assert contract[name] is False
registry = tomllib.loads((ROOT / "docs/testing/gated-test-registry.toml").read_text())
baseline_registry = tomllib.loads(git(M["baseline_source"], "docs/testing/gated-test-registry.toml").decode())
assert registry["gate"][:-1] == baseline_registry["gate"]
gate_id = "tool.hydracache-long-run-supervisor-074.diagnostic-process-crash-quarantine-all-stages-and-post-return"
gate = registry["gate"][-1]
assert gate["id"] == gate_id and gate["tier"] == "manual" and gate["ship_mandatory"] is False
assert gate["timeout_seconds"] == 600 and gate["command"]["platform"] == "linux"
evidence = tomllib.loads((ROOT / "docs/testing/release-evidence/0.74.toml").read_text())
w11 = next(w for w in evidence["work_item"] if w["id"] == "W11")
assert gate_id in w11["gated_gate_ids"]
for name in ("README.md", "manifest.json", "verify_packet.py", "inspect_production_symbols.py"):
    assert f"{PACKET.relative_to(ROOT).as_posix()}/{name}" in w11["required_artifacts"]
for source, function in M["required_test_names"]:
    assert any(t["source"] == source and t["function"] == function for t in w11["required_tests"])
print("PASS: exact-source authority SIGKILL matrix; unchanged production protocol; closed execution and qualification")
