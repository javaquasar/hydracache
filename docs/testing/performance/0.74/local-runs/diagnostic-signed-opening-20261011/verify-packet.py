"""Read-only retained local evidence; never authorizes host, enrollment or start."""
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
require(m["schema_version"] == "diagnostic-signed-opening-local-evidence-074-v1", "schema")
require(m["state"] == "retained-local-signed-status-opening-not-production-enrollment", "state")
require(m["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
expected_names = set(json.loads("[\"api-red.log\",\"baseline.log\",\"component-1.log\",\"component-2.log\",\"component-3.log\",\"contract-local.log\",\"docs-final.log\",\"docs-pre.log\",\"docs-sealed.log\",\"isolation.log\",\"linux-check.log\",\"linux-first-green.log\",\"linux-lint.log\",\"linux-root.log\",\"linux-scope.log\",\"performance-contract.log\",\"registry.log\",\"repeat-1.log\",\"repeat-2.log\",\"repeat-3.log\",\"root-evidence.log\",\"ship.log\",\"windows-check.log\",\"windows-lint.log\",\"windows-scope.log\"]"))
require(set(m["logs"]) == expected_names, "declared inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual inventory")
for name, pin in m["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")
require(m["results"] == json.loads("{\"windows_baseline_passed\":43,\"windows_final_passed\":43,\"linux_final_passed\":340,\"linux_existing_ignored\":1,\"new_opening_tests\":15,\"new_component_tests\":1,\"additional_linux_repetitions\":3,\"root_final_passed\":133,\"linux_new_root_guard_passed\":1}"), "result claims")
require(m["new_tests"] == json.loads("[{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_constructor_and_later_orders_are_exact\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_every_failure_preserves_first_typed_error_and_stops\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_prior_refusal_never_opens_or_reads\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_seeded_failure_positions_remain_sticky\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_origin_types_are_neither_send_nor_sync\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_owned_roundtrip_and_drop_preserve_original_inputs\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_refused_account_precedes_dead_process_and_missing_files\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_revocation_precedes_process_and_file_errors\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_dead_original_process_latches_policy_at_construction\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_unhardened_process_refuses_only_at_projection_read\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_valid_signed_mapping_mismatch_is_not_caller_numbers\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_post_open_account_failure_precedes_first_projection_stage\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_restoration_latches_owned_credentials_and_policy_after_drop\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_original_exit_refuses_without_replacement_or_signal\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_signed_opening_tests.rs\",\"name\":\"signed_opening_independent_concurrent_readers_share_no_refusal\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_namespace_credentials.rs\",\"name\":\"signed_status_private_opener_defers_projection_until_revalidation\"}]"), "new test names")
require(set(m["proof_boundaries"]) == set(json.loads("[\"admission_allowed\",\"all_threads_proved\",\"atomic_cross_file_snapshot\",\"authenticated_original_start\",\"continuous_context_or_writer_revocation\",\"durable_epoch_or_refusal_registry\",\"file_opener_credentials_attested\",\"full_workspace_verification\",\"guard_send_or_sync\",\"host_mutation_allowed\",\"initial_host_namespace_proved\",\"new_helper_ipc_or_live_route\",\"old_public_constructor_or_parser_behavior_changed\",\"performance_claim_allowed\",\"positive_fixed_production_inspection\",\"preopened_credentials_accepted\",\"product_workload_allowed\",\"production_preparation_allowed\",\"qualification_allowed\",\"raw_pid_path_numeric_policy_accepted\",\"real_issuer_enrolled\",\"regular_file_io_deadline\",\"worker_mount_namespace_proved\"]")), "boundary inventory")
require(all(v is False for v in m["proof_boundaries"].values()), "closed boundaries")
require(m["baseline_source"] == "70064e71078c587f79d371fa611c78a2e53b9ff8", "baseline")
require(m["preregistered_design_commit"] == "3ad236d3eac257fc182b699b882962f931995492", "preregistration")
require(m["implementation_commit"] == "69eb668cd135443c1b6ecef8c95756b6c367b83f", "implementation")
require(m["implementation_tree"] == "63e50a904e1b64c52b382754bbd10d2319cf30d6", "implementation tree")
require(m["toolchain"] == "1.94.0", "toolchain")
require(m["missing_api_wsl_exit"] == 1 and m["require_ship_native_exit"] == 1, "refusal exits")
require(m["seed"] == "0x7572026" and m["modeled_failure_positions_per_seeded_run"] == 256, "seed")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(p), int(i)) for p, i in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline") == [(15, 0), (15, 0), (13, 0)], "baseline totals")
require(totals("windows-scope") == [(15, 0), (15, 0), (0, 0), (13, 0)], "Windows totals")
require(totals("linux-scope") == [(266, 1), (29, 0), (9, 0), (2, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "Linux totals")
require(totals("linux-first-green") == [(15, 0)], "first green")
require(totals("performance-contract") == [(97, 0)], "contracts")
require(totals("root-evidence") == [(13, 0), (23, 0)], "evidence/governance")
require(totals("linux-root") == [(1, 0)], "Linux root")
require(totals("registry") == [(1, 0)], "final W11 registry")
for trial in range(1, 4):
    repeat = f"repeat-{trial}"
    require(totals(repeat) == [(15, 0)], f"opening repetition {trial}")
    require(totals(f"component-{trial}") == [(1, 0)], f"component repetition {trial}")
    require("signed opening mutation seed=0x7572026; mutations=256" in text(repeat), f"seed {trial}")
    for test in m["new_tests"]:
        if test["source"].endswith("diagnostic_signed_opening_tests.rs"):
            prefix = "diagnostic_worker_policy::local_context::account_files::kernel_binding::opening::tests::"
            capture = repeat
        else:
            prefix = "diagnostic_process::credentials::namespace_checked::tests::"
            capture = f"component-{trial}"
        label = f"test {prefix}{test['name']} ... ok"
        require(label in text(capture) and label in text("linux-scope"), f"test {test['name']}/{trial}")
for name, rows in {
    "api-red": ["error[E0432]", "could not find `opening` in `super`"],
    "docs-final": ["scoped rustfmt 1.94.0 passed", "doc-check: OK", "performance-contract-check 0.74: OK", "Checked 51 markdown files", "HTML book written"],
    "docs-sealed": ["doc-check: OK", "performance-contract-check 0.74: OK", "Checked 51 markdown files", "HTML book written"],
    "contract-local": ["performance-contract-check 0.74: OK"],
    "registry": ["w12_evidence_skeleton_is_exact_and_fail_closed ... ok"],
    "linux-root": ["diagnostic_signed_opening_keeps_projection_after_original_context_and_start_closed ... ok"],
    "isolation": ["all actual qualification contract_inputs: unchanged", "product/cache/native code and legacy authority/routes: unchanged", "old public namespace constructor body: byte-normalized unchanged"],
    "ship": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    require(all(row in text(name) for row in rows), f"claim rows {name}")
require("compiler unexpectedly panicked" not in text("api-red"), "ordinary API refusal")
for name in ["windows-check", "windows-lint", "linux-check", "linux-lint"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"package gate {name}")
for name in expected_names - {"api-red.log", "ship.log"}:
    require("warning:" not in text(name[:-4]), f"warning-free final capture {name}")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(m["qualification_manifest_sha256"] == pin, "qualification manifest pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen bytes")
print("signed status opening: exact local safety evidence; production, durable authority and start remain closed")
