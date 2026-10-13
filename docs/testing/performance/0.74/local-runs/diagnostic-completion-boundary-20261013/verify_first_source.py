"""Read-only local terminal-process evidence; never completion admission."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path
P = Path(__file__).resolve().parent
R = P.parents[5]
M = json.loads((P / "manifest.json").read_text())
def lf(raw): return raw.replace(b"\r\n", b"\n")
def sha(raw): return hashlib.sha256(raw).hexdigest()
def git(commit, path): return lf(subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=R))
def passed(log): return sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", log)))
assert M["baseline_source"] == "34b3cb37a2774347cf3865af031ad3bd0e2d04c3"
assert M["preregistration_source"] == "d2f3b04120f433f0cf287ae979a35ff27424ff5e"
assert sha(lf((P / "verify_packet.py").read_bytes())) == M["verifier_sha256"]
for field in ("baseline_source", "preregistration_source", "implementation_source"):
    assert re.fullmatch(r"[0-9a-f]{40}", M[field])
    subprocess.check_call(["git", "cat-file", "-e", M[field] + "^{commit}"], cwd=R)
sources = set(["tools/long-run-supervisor-074/src/diagnostic_process.rs","tools/long-run-supervisor-074/src/diagnostic_process_terminal.rs","tools/long-run-supervisor-074/src/diagnostic_process_terminal_tests.rs","docs/testing/gated-test-registry.toml"])
assert {s["path"] for s in M["source_files"]} == sources
for e in M["source_files"]:
    raw = git(M["implementation_source"], e["path"])
    assert sha(raw) == e["sha256_lf"] and lf((R / e["path"]).read_bytes()) == raw
design = "docs/testing/performance/0.74/diagnostic-completion-boundary-local-design.md"
changed = set(subprocess.check_output(["git", "diff", "--name-only", M["baseline_source"], M["implementation_source"]], cwd=R, text=True).splitlines())
assert changed == sources | {design}
assert git(M["preregistration_source"], design) == git(M["implementation_source"], design)
assert lf((R / design).read_bytes()) == git(M["preregistration_source"], design)
process = "tools/long-run-supervisor-074/src/diagnostic_process.rs"
declaration = '#[path = "diagnostic_process_terminal.rs"]\nmod terminal;\npub use terminal::{TerminalProcessObservation, TerminalState};\n\n'
raw = git(M["implementation_source"], process).decode()
assert raw.count(declaration) == 1
assert raw.replace(declaration, "").encode() == git(M["baseline_source"], process)
assert {e["path"] for e in M["unchanged_sources"]} == set(["tools/long-run-supervisor-074/src/diagnostic_authority_fixed.rs","tools/long-run-supervisor-074/src/diagnostic_signed_authority.rs","tools/long-run-supervisor-074/src/diagnostic_authority_intent.rs","tools/long-run-supervisor-074/src/diagnostic_starting_intent.rs","tools/long-run-supervisor-074/src/diagnostic_starting_intent_storage.rs","tools/long-run-supervisor-074/src/diagnostic_reservation_publication_storage.rs","tools/long-run-supervisor-074/src/diagnostic_initial_consumption_storage.rs","tools/long-run-supervisor-074/src/diagnostic_lease.rs","tools/long-run-supervisor-074/src/diagnostic_transition.rs","tools/long-run-supervisor-074/src/diagnostic_authority_fixture.rs","tools/long-run-supervisor-074/src/diagnostic_authority_fixed_tests.rs","tools/long-run-supervisor-074/src/diagnostic_authority_checkpoint.rs","tools/long-run-supervisor-074/src/diagnostic_tree.rs","tools/long-run-supervisor-074/src/diagnostic_live_execution.rs","tools/long-run-supervisor-074/src/lib.rs"])
assert {e["path"] for e in M["frozen_files"]} == set(["docs/testing/performance/0.74/baseline-identities.toml","docs/testing/performance/0.74/scenario-matrix.toml","docs/testing/performance/0.74/statistics.toml","docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md","docs/testing/performance/0.74/long-run-controller-resilience-contract.toml","docs/testing/performance/0.74/composition-ledger.toml","docs/testing/performance/0.74/release-admission-contract.toml","docs/testing/performance/0.74/terminal-disposition-ledger.toml"])
for e in M["unchanged_sources"] + M["frozen_files"]:
    raw = git(M["baseline_source"], e["path"])
    assert sha(raw) == e["sha256_lf"] and lf((R / e["path"]).read_bytes()) == raw
assert sha((R / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
logs = {}
for e in M["captures"]:
    assert e["path"] == e["id"] + ".log" and e["id"] not in logs
    expected = 101 if e["id"] == "red" else 1 if e["id"] in ("qualification-refusal", "ship-refusal") else 0
    assert e["expected_exit"] == expected
    raw = (P / e["path"]).read_bytes()
    assert len(raw) == e["bytes"] and sha(raw) == e["sha256"]
    if e["id"] == "functional-first":
        assert e["expected_exit"] == 0 and e["observed_exit"] == 101
    else:
        assert e["observed_exit"] == e["expected_exit"]
    logs[e["id"]] = raw.decode("utf-8")
assert len(logs) == M["capture_count"]
assert set(logs) == set(["baseline","red","functional-first","functional-corrected","process-first","functional-final-1-pre-review","functional-final-2-pre-review","functional-final-3-pre-review","process-final-1-pre-review","process-final-2-pre-review","process-final-3-pre-review","starting-regression-pre-review","windows-regression","windows-check","windows-msrv","windows-clippy","root-contracts","kernel","linux-process-regression-pre-review","linux-integrations-pre-review","linux-check-pre-review","linux-msrv-pre-review","linux-clippy-pre-review","linux-doctests-pre-review","linux-rustdoc-pre-review","functional-final-1","functional-final-2","functional-final-3","process-final-1","process-final-2","process-final-3","starting-regression","linux-process-regression","linux-integrations","linux-check","linux-msrv","linux-clippy","linux-doctests","linux-rustdoc","windows-reviewed-check","windows-reviewed-msrv","windows-reviewed-clippy","root-reviewed-contracts","root-artifacts-final","doc-check-final","book-final","format-final","performance-contract","qualification-refusal","ship-refusal"])
assert logs["kernel"].strip() == "6.6.87.2-microsoft-standard-WSL2"
assert passed(logs["baseline"]) == 15
assert "E0405" in logs["red"] and "E0425" in logs["red"] and "E0433" in logs["red"]
assert "4 passed; 2 failed; 1 ignored" in logs["functional-first"]
assert "left: Exited" in logs["functional-first"] and "right: Reaped" in logs["functional-first"]
assert M["retained_development_failures"] == ["functional-first"]
for i in (1, 2, 3):
    fast = logs[f"functional-final-{i}"]
    assert passed(fast) == 7
    assert "terminal-mask exhaustive=65536 accepted=3 unknown_or_inconsistent=refused" in fast
    process = logs[f"process-final-{i}"]
    assert passed(process) == 1
    assert process.count("P1 terminal seed=0x074f leader=SIGKILL signal=9 original_pidfd=terminal descendant=live post_exit_write=observed descendant_after_control_eof=terminal completion_authority=absent") == 1
reg = logs["starting-regression"]
assert passed(reg) == 1 and reg.count("terminated=SIGKILL signal=9") == 9
assert reg.count("terminated=control signal=0") == 1
assert sum(map(int, re.findall(r"fresh_processes=(\d+)", reg))) == 20
assert passed(logs["linux-process-regression"]) == 103
assert passed(logs["linux-integrations"]) == 52
assert passed(logs["windows-regression"]) == 204
assert passed(logs["root-reviewed-contracts"]) == 166
assert passed(logs["linux-doctests"]) == 55
assert passed(logs["root-artifacts-final"]) == 13
assert M["performance_measurements"] == [] and M["server_operation_performed"] is False
assert M["qualification_output_created"] is False
for name in ("qualification-refusal", "ship-refusal"):
    entry = next(e for e in M["captures"] if e["id"] == name)
    assert entry["expected_exit"] == entry["observed_exit"] == 1
contract = tomllib.loads((R / "docs/testing/performance/0.74/diagnostic-completion-boundary-local-contract.toml").read_text())
assert contract["poll_mask_cases"] == 65536 and contract["accepted_terminal_masks"] == 3
assert contract["manual_repetitions"] == 3 and contract["manual_gate_timeout_seconds"] == 180
assert contract["fixture_kernel_parent_verified_before_signal_ownership"] is True
for name in ("new_persistent_write_boundary", "authenticated_original_start_implemented",
             "authenticated_completion_implemented", "cells_two_through_four_implemented",
             "trusted_peer_time_enrolled", "production_issuer_enrolled", "writer_revocation_implemented",
             "cleanup_authority_exposed", "actual_reboot_performed", "server_operation_performed",
             "qualification_allowed", "ship_allowed", "performance_claim_allowed"):
    assert contract[name] is False
registry = tomllib.loads((R / "docs/testing/gated-test-registry.toml").read_text())
baseline = tomllib.loads(git(M["baseline_source"], "docs/testing/gated-test-registry.toml").decode())
assert registry["gate"][:-1] == baseline["gate"]
gate = registry["gate"][-1]
assert gate["id"] == M["gate_id"] and gate["timeout_seconds"] == 180
assert gate["tier"] == "manual" and gate["ship_mandatory"] is False
assert "--exact" in gate["command"]["args"]
w11 = next(w for w in tomllib.loads((R / "docs/testing/release-evidence/0.74.toml").read_text())["work_item"] if w["id"] == "W11")
assert M["gate_id"] in w11["gated_gate_ids"]
for name in ("README.md", "manifest.json", "verify_packet.py"):
    assert f"{P.relative_to(R).as_posix()}/{name}" in w11["required_artifacts"]
for function in M["required_test_names"]:
    assert any(t["source"] == "tools/long-run-supervisor-074/src/diagnostic_process_terminal_tests.rs" and t["function"] == function for t in w11["required_tests"])
print("PASS: original pidfd terminal observation; post-exit descendant write; no completion or cleanup admission")
