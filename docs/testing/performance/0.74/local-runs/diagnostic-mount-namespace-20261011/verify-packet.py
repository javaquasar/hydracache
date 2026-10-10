"""Read-only local mount consistency evidence; never enrollment or start."""
import hashlib
import json
import re
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
m = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(m["schema_version"] == "diagnostic-mount-namespace-local-evidence-074-v1", "schema")
require(m["state"] == "retained-local-leader-mount-consistency-not-production-enrollment", "state")
require(m["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
expected_names = set(json.loads("[\"api-red.log\",\"baseline.log\",\"docs-final.log\",\"docs.log\",\"first-green.log\",\"isolation.log\",\"linux-check.log\",\"linux-lint.log\",\"linux-root.log\",\"linux-scope.log\",\"registry.log\",\"repeat-1.log\",\"repeat-2.log\",\"repeat-3.log\",\"root.log\",\"ship.log\",\"windows-check.log\",\"windows-lint.log\",\"windows-scope.log\"]"))
require(set(m["logs"]) == expected_names, "declared inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual inventory")
for name, pin in m["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")
require(m["results"] == json.loads("{\"windows_baseline_passed\":43,\"windows_final_passed\":43,\"windows_linux_target_tests\":0,\"first_green_passed\":16,\"linux_final_passed\":356,\"linux_existing_ignored\":1,\"new_tests\":16,\"additional_linux_repetitions\":3,\"root_final_passed\":134,\"linux_new_root_guard_passed\":1}"), "result claims")
require(m["new_tests"] == json.loads("[{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_order_brackets_both_observations_with_generation\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_every_first_error_stops_with_original_type\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_prior_refusal_and_foreign_thread_never_observe\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_identity_drift_never_adopts_device_or_inode\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_seeded_failure_and_identity_are_sticky\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_guard_is_neither_send_nor_sync\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_owned_roundtrip_readonly_flags_and_healthy_drop\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_owned_exit_refuses_despite_retained_namespace\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_constructor_refuses_dead_original_generation\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_wrong_objects_and_restoration_latch\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_original_directory_substitution_cannot_restore\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_named_directory_pin_drift_cannot_refresh\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_type_is_kernel_checked_not_link_text\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_observer_tid_drift_precedes_dead_process_and_bad_fd\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_distinct_identity_check_is_not_initial_host_attestation\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_mount_namespace_tests.rs\",\"name\":\"mount_namespace_concurrent_readers_construct_on_own_threads\"}]"), "new test inventory")
require(set(m["proof_boundaries"]) == set(json.loads("[\"admission_allowed\",\"all_threads_proved\",\"arbitrary_path_or_namespace_selector\",\"atomic_or_continuous_proof\",\"authenticated_original_start\",\"file_opener_credentials_attested\",\"full_workspace_verification\",\"guard_send_or_sync\",\"host_mutation_allowed\",\"initial_host_namespace_proved\",\"kernel_foreign_namespace_mutation_tested\",\"mount_content_immutability_proved\",\"new_helper_ipc_or_live_route\",\"performance_claim_allowed\",\"product_workload_allowed\",\"production_preparation_allowed\",\"public_descriptor_identity_or_reset\",\"qualification_allowed\",\"real_issuer_enrolled\",\"signed_policy_composition\"]")), "boundary inventory")
require(all(value is False for value in m["proof_boundaries"].values()), "closed boundaries")
require(m["baseline_source"] == "4c93f79987f2f582e75bd2d5cdc3a435d5b3bf4b", "baseline")
require(m["preregistered_design_commit"] == "9600967f0a3d43b496efa7069b46e6acfada9c08", "preregistration")
require(m["implementation_commit"] == "6137743ae518e73d8bc9815f209665b734428653", "implementation")
require(m["implementation_tree"] == "64c1d03c2c2ac2de5f1f0e40a15a26b8875f036d", "implementation tree")
require(m["toolchain"] == "1.94.0", "toolchain")
require(m["missing_api_wsl_exit"] == 1 and m["require_ship_native_exit"] == 1, "refusal exits")
require(m["seed"] == "0x7582026" and m["modeled_iterations_per_seeded_run"] == 256, "seed")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(p), int(i)) for p, i in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline") == [(15, 0), (15, 0), (13, 0)], "baseline totals")
require(totals("windows-scope") == [(15, 0), (15, 0), (0, 0), (13, 0)], "Windows totals")
require(totals("first-green") == [(16, 0)], "first green")
require(totals("linux-scope") == [(282, 1), (29, 0), (9, 0), (2, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "Linux totals")
require(totals("root") == [(98, 0), (13, 0), (23, 0)], "root totals")
require(totals("linux-root") == [(1, 0)], "Linux root")
require(totals("registry") == [(1, 0)], "final W11 registry")
for trial in range(1, 4):
    repeat = f"repeat-{trial}"
    require(totals(repeat) == [(16, 0)], f"repeat {trial}")
    require("mount namespace mutation seed=0x7582026; mutations=256" in text(repeat), f"seed {trial}")
    for row in m["new_tests"]:
        label = f"test diagnostic_process::mount_namespace::tests::{row['name']} ... ok"
        require(label in text(repeat) and label in text("linux-scope"), f"test {row['name']}/{trial}")
for name, rows in {
    "api-red": ["error[E0432]", "error[E0282]", "could not find `mount_namespace` in `super`"],
    "docs": ["scoped rustfmt 1.94.0 passed", "doc-check: OK", "performance-contract-check 0.74: OK", "Checked 51 markdown files", "HTML book written"],
    "docs-final": ["doc-check: OK", "performance-contract-check 0.74: OK", "Checked 51 markdown files", "HTML book written"],
    "linux-root": ["diagnostic_mount_namespace_keeps_leader_consistency_and_production_closed ... ok"],
    "registry": ["w12_evidence_skeleton_is_exact_and_fail_closed ... ok"],
    "isolation": ["all actual qualification contract_inputs: unchanged", "product/cache/native and prior guards/live routes: unchanged"],
    "ship": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    require(all(row in text(name) for row in rows), f"claim rows {name}")
require("compiler unexpectedly panicked" not in text("api-red"), "ordinary missing API")
for name in ["windows-check", "windows-lint", "linux-check", "linux-lint"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"package gate {name}")
for name in expected_names - {"api-red.log", "ship.log"}:
    require("warning:" not in text(name[:-4]), f"warning-free gate {name}")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(m["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen bytes")
print("leader mount namespace: 19 exact local captures; signed composition, all-thread proof and production remain closed")
