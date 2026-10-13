"""Read-only exact-source modeled context replay proof; never a reboot test."""
import hashlib
import json
import re
import subprocess
import tomllib
from collections import Counter
from pathlib import Path

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
M = json.loads((PACKET / "manifest.json").read_text(encoding="utf-8"))
def lf(raw): return raw.replace(b"\r\n", b"\n")
def sha(raw): return hashlib.sha256(raw).hexdigest()
def git(commit, path):
    return lf(subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT))
def passed(log):
    return sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", log)))

assert M["schema_version"] == 1
assert M["baseline_source"] == "bd72707189db21fe5ac67111462c3c91b71f049e"
assert M["preregistration_source"] == "d6ee09ce37aad58f429ea075e0d80ca20579b566"
assert M["implementation_source"] == "4302202b12d90f0708c029929b6f7f51c6f38aea"
assert sha(lf((PACKET / "verify_packet.py").read_bytes())) == M["verifier_sha256"]
for field in ("baseline_source", "preregistration_source", "implementation_source"):
    subprocess.check_call(["git", "cat-file", "-e", M[field] + "^{commit}"], cwd=ROOT)
sources = {
    "tools/long-run-supervisor-074/src/diagnostic_context_replay_tests.rs",
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
design = "docs/testing/performance/0.74/diagnostic-context-replay-local-design.md"
assert changed == sources | {design}
assert set(subprocess.check_output(["git", "diff", "--name-only",
    M["baseline_source"], M["preregistration_source"]], cwd=ROOT, text=True).splitlines()) == {design}
assert git(M["preregistration_source"], design) == git(M["implementation_source"], design)
assert lf((ROOT / design).read_bytes()) == git(M["preregistration_source"], design)
assert {e["path"] for e in M["unchanged_sources"]} == set(["tools/long-run-supervisor-074/src/diagnostic_authority_fixed.rs","tools/long-run-supervisor-074/src/diagnostic_signed_authority.rs","tools/long-run-supervisor-074/src/diagnostic_authority_intent.rs","tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs","tools/long-run-supervisor-074/src/diagnostic_starting_intent_storage.rs","tools/long-run-supervisor-074/src/diagnostic_reservation_publication_storage.rs","tools/long-run-supervisor-074/src/diagnostic_initial_consumption_storage.rs","tools/long-run-supervisor-074/src/diagnostic_lease.rs","tools/long-run-supervisor-074/src/diagnostic_transition.rs","tools/long-run-supervisor-074/src/diagnostic_process.rs","tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs","tools/long-run-supervisor-074/src/diagnostic_authority_fixed_tests.rs","tools/long-run-supervisor-074/src/diagnostic_authority_checkpoint.rs"])
assert {e["path"] for e in M["frozen_files"]} == set(["docs/testing/performance/0.74/baseline-identities.toml","docs/testing/performance/0.74/scenario-matrix.toml","docs/testing/performance/0.74/statistics.toml","docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md","docs/testing/performance/0.74/long-run-controller-resilience-contract.toml","docs/testing/performance/0.74/composition-ledger.toml","docs/testing/performance/0.74/release-admission-contract.toml","docs/testing/performance/0.74/terminal-disposition-ledger.toml"])
for entry in M["unchanged_sources"] + M["frozen_files"]:
    raw = git(M["baseline_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((ROOT / entry["path"]).read_bytes()) == raw
assert sha((ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
logs = {}
unreceipted = {"performance-contract", "qualification-refusal", "ship-refusal"}
for entry in M["captures"]:
    assert entry["path"] == entry["id"] + ".log" and entry["id"] not in logs
    expected_exit = 101 if entry["id"] == "red" else 1 if entry["id"].startswith(("qualification-refusal", "ship-refusal")) else 0
    assert entry["expected_exit"] == expected_exit
    raw = (PACKET / entry["path"]).read_bytes()
    assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"]
    if entry["id"] in unreceipted:
        assert entry["observed_exit"] is None
    else:
        assert entry["observed_exit"] == entry["expected_exit"]
    logs[entry["id"]] = raw.decode("utf-8")
assert len(logs) == M["capture_count"]
assert set(logs) == set(["baseline","red","functional-first","linux-check","linux-msrv","linux-clippy","functional-final-1","functional-final-2","functional-final-3","process-final-1","process-final-2","process-final-3","process-regression","windows-regression","windows-check","windows-msrv","windows-clippy","root-contracts","format","doc-check","performance-contract","qualification-refusal","ship-refusal","performance-contract-final","qualification-refusal-final","ship-refusal-final","linux-regression","linux-doctests","linux-rustdoc","root-artifacts-final","doc-check-final","book-final","format-final"])
assert M["unreceipted_initial_admission_captures"] == sorted(unreceipted)
assert passed(logs["baseline"]) == 9
assert "E0433" in logs["red"] and "context_replay" in logs["red"]
profiles = ("same", "boot", "machine", "user-namespace", "mount-namespace", "digest", "epoch", "issuer")
stages = ("BeforeRefusal", "RecordCreated", "RecordSynced", "MarkerCreated", "DurableCommit", "AfterReturn")
for repeat in (1, 2, 3):
    log = logs[f"functional-final-{repeat}"]
    assert passed(log) == 3
    assert "context-replay seed=0x074c same=32 changed=224 component_changes=exactly-one" in log

def replay_matrix(log):
    assert "seed=0x074d stages=6" in log
    for stage in stages:
        marker = f"P1 case=quarantine:{stage} terminated=SIGKILL signal=9 fresh_processes=16 retained=unchanged original_lock=released helper=terminal"
        assert log.count(marker) == 1
    control = "P1 case=quarantine:AfterReturn terminated=control signal=0 fresh_processes=16 retained=unchanged original_lock=released helper=terminal"
    assert log.count(control) == 1
    observed = re.findall(r"P1 replay profile=(\S+) checkpoint=(\S+) inspection=([12]) authority=(\S+) reservation=refused retained=unchanged", log)
    assert len(observed) == 112
    expected = Counter()
    for stage in stages:
        for profile in profiles:
            label = ("fresh-refused-authority-invalid" if stage in stages[1:4]
                     else "fresh-refused-authority-binding" if profile != "same"
                     else "fresh-refused-authority-active" if stage == "BeforeRefusal"
                     else "fresh-refused-authority-refused")
            for inspection in ("1", "2"):
                expected[(profile, stage, inspection, label)] += 2 if stage == "AfterReturn" else 1
    assert Counter(observed) == expected
    labels = Counter(o[3] for o in observed)
    assert labels == {"fresh-refused-authority-binding": 56, "fresh-refused-authority-invalid": 48,
                      "fresh-refused-authority-active": 2, "fresh-refused-authority-refused": 6}
for repeat in (1, 2, 3):
    log = logs[f"process-final-{repeat}"]
    replay_matrix(log)
    assert passed(log) == 1
regression = logs["process-regression"]
replay_matrix(regression)
assert regression.count("terminated=SIGKILL signal=9") == 78
assert regression.count("terminated=control signal=0") == 8
assert sum(map(int, re.findall(r"fresh_processes=(\d+)", regression))) == 270
assert passed(regression) == 9
assert passed(logs["linux-regression"]) == 676
assert sum(map(int, re.findall(r"test result: ok\. \d+ passed; 0 failed; (\d+) ignored;", logs["linux-regression"]))) == 10
assert passed(logs["windows-regression"]) == 204
assert passed(logs["root-contracts"]) == 166
assert passed(logs["linux-doctests"]) == 52
assert passed(logs["root-artifacts-final"]) == 13
assert M["qualification_output_created"] is False
assert M["server_operation_performed"] is False and M["performance_measurements"] == []
for name in ("qualification-refusal-final", "ship-refusal-final"):
    entry = next(e for e in M["captures"] if e["id"] == name)
    assert entry["expected_exit"] == entry["observed_exit"] == 1
contract = tomllib.loads((ROOT / "docs/testing/performance/0.74/diagnostic-context-replay-local-contract.toml").read_text())
assert contract["baseline_source"] == M["baseline_source"]
assert contract["profiles"] == list(profiles)
assert (contract["property_seed"], contract["unchanged_contexts_per_repeat"], contract["changed_contexts_per_repeat"]) == ("0x074c", 32, 224)
assert (contract["process_seed"], contract["sigkill_cases_per_repeat"], contract["controls_per_repeat"], contract["profile_groups_per_repeat"]) == ("0x074d", 6, 1, 56)
assert contract["fresh_processes_per_repeat"] == 112 and contract["targeted_repetitions"] == 3
for name in ("test_only_inspection_profiles", "one_context_or_trust_component_changed",
             "unknown_profiles_refuse", "complete_wrong_context_returns_binding",
             "partial_tail_returns_invalid_before_binding", "all_retained_context_refuses_fresh_reservation",
             "inspection_read_only_no_ack_recovery", "bytes_and_metadata_unchanged_after_each_inspection",
             "production_code_and_storage_unchanged", "modeled_boot_and_namespace_changes"):
    assert contract[name] is True
for name in ("actual_reboot_performed", "actual_namespace_switch_performed", "production_issuer_enrolled",
             "authenticated_peer_or_clock", "physical_power_loss_proved", "whole_host_rollback_resistance_proved",
             "full_fixed_lifecycle_implemented", "completion_or_cleanup_authenticated", "worker_execution_allowed",
             "server_operation_performed", "qualification_allowed", "ship_allowed", "performance_claim_allowed"):
    assert contract[name] is False
registry = tomllib.loads((ROOT / "docs/testing/gated-test-registry.toml").read_text())
baseline = tomllib.loads(git(M["baseline_source"], "docs/testing/gated-test-registry.toml").decode())
assert registry["gate"][:-1] == baseline["gate"]
gate = registry["gate"][-1]
assert gate["id"] == M["gate_id"] and gate["timeout_seconds"] == 600
assert gate["tier"] == "manual" and gate["ship_mandatory"] is False
assert gate["command"]["platform"] == "linux" and "--exact" in gate["command"]["args"]
evidence = tomllib.loads((ROOT / "docs/testing/release-evidence/0.74.toml").read_text())
w11 = next(w for w in evidence["work_item"] if w["id"] == "W11")
assert M["gate_id"] in w11["gated_gate_ids"]
for name in ("README.md", "manifest.json", "verify_packet.py"):
    assert f"{PACKET.relative_to(ROOT).as_posix()}/{name}" in w11["required_artifacts"]
for source, function in M["required_test_names"]:
    assert any(t["source"] == source and t["function"] == function for t in w11["required_tests"])
print("PASS: exact-source modeled context replay after SIGKILL; readonly refusal; no reboot, execution or qualification")
