"""Read-only local consistency evidence, never enrollment or qualification."""
import hashlib
import json
import re
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-policy-credentials-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-original-policy-credential-consistency-not-production-enrollment", "state")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
require(manifest["results"] == json.loads("{\"windows_baseline_passed\":43,\"windows_final_passed\":43,\"windows_linux_target_tests\":0,\"first_green_binding_passed\":14,\"linux_final_passed\":324,\"linux_existing_ignored\":1,\"new_binding_tests\":15,\"new_component_tests\":1,\"additional_linux_repetitions\":3,\"root_final_passed\":132,\"linux_new_root_guard_passed\":1}"), "result claims")
expected_names = set(json.loads("[\"api-red.log\",\"baseline.log\",\"docs.log\",\"first-green.log\",\"isolation.log\",\"linux-check-final.log\",\"linux-check.log\",\"linux-lint.log\",\"linux-root.log\",\"linux-tests-final.log\",\"linux-tests.log\",\"repeat-1.log\",\"repeat-2.log\",\"repeat-3.log\",\"repeat-components-1.log\",\"repeat-components-2.log\",\"repeat-components-3.log\",\"root-tests.log\",\"ship.log\",\"windows-check.log\",\"windows-lint-final.log\",\"windows-lint.log\",\"windows-tests.log\",\"docs-final.log\",\"registry.log\"]"))
require(set(manifest["logs"]) == expected_names, "declared inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual inventory")
for name, pin in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(passed), int(ignored)) for passed, ignored in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline") == [(15, 0), (15, 0), (13, 0)], "baseline")
require(totals("windows-tests") == [(15, 0), (15, 0), (0, 0), (13, 0)], "Windows")
require(totals("first-green") == [(14, 0)], "first green")
require(totals("linux-tests-final") == [(250, 1), (29, 0), (9, 0), (2, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "Linux")
require(totals("linux-tests") == totals("linux-tests-final"), "initial Linux suite")
require(totals("root-tests") == [(96, 0), (13, 0), (23, 0)], "root")
for name, rows in {
    "api-red": ["error[E0432]", "could not find `kernel_binding`"],
    "linux-root": ["diagnostic_policy_credentials_bind_exact_original_mapping_and_keep_start_closed ... ok", "1 passed; 0 failed"],
    "docs": ["scoped rustfmt 1.94.0 passed", "doc-check: OK", "performance-contract-check 0.74: OK", "HTML book written"],
    "docs-final": ["doc-check: OK", "performance-contract-check 0.74: OK", "Checked 51 markdown files", "HTML book written"],
    "registry": ["w12_evidence_skeleton_is_exact_and_fail_closed ... ok", "1 passed; 0 failed"],
    "isolation": ["all actual qualification contract_inputs: unchanged", "product/cache/native code and legacy authority/routes: unchanged"],
    "ship": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    require(all(row in text(name) for row in rows), f"claim rows {name}")
require("compiler unexpectedly panicked" not in text("api-red"), "ordinary missing API")
for name in ["windows-check", "windows-lint", "windows-lint-final", "linux-check", "linux-check-final", "linux-lint"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"package gate {name}")
require("unused_mut" in text("linux-tests") and "unused_mut" in text("linux-check"), "initial warning retained")
for name in ["linux-tests-final", "linux-check-final", "linux-lint", "windows-lint-final"]:
    require("warning:" not in text(name), f"final warning-free {name}")
expected_tests = json.loads("[{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_order_preserves_both_original_brackets\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_prior_refusal_and_mismatch_never_observe\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_every_error_stops_with_original_type\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_exact_mapping_never_forms_primary_union\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_seeded_mapping_mutations_cannot_refresh\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_binding_types_are_neither_send_nor_sync\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_fixed_prior_refusal_never_reads_production_accounts\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_owned_roundtrip_and_drop_keep_inputs_healthy\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_public_mismatch_precedes_broken_account_io\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_public_prior_refusal_latches_other_input\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_constructor_file_failure_latches_after_drop\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_file_restoration_cannot_clear_binding_refusal\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_revocation_precedes_account_and_process_errors\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_original_exit_refuses_constructor_and_later_read\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_policy_credentials_tests.rs\",\"name\":\"policy_credentials_independent_concurrent_bindings_share_no_refusal\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_namespace_credentials.rs\",\"name\":\"namespace_policy_matching_requires_original_projection_and_both_guards\"}]")
require(manifest["new_tests"] == expected_tests, "new tests")
binding = [row["name"] for row in expected_tests if row["source"].endswith("diagnostic_policy_credentials_tests.rs")]
component = [row["name"] for row in expected_tests if row["source"].endswith("diagnostic_namespace_credentials.rs")]
require(len(binding) == 15 and len(component) == 1, "test counts")
for trial in range(1, 4):
    name = f"repeat-{trial}"
    require(totals(name) == [(15, 0)], f"binding repeat {trial}")
    require("policy credentials mutation seed=0x7562026; mutations=256" in text(name), f"seed {trial}")
    for test in binding:
        label = f"test diagnostic_worker_policy::local_context::account_files::kernel_binding::tests::{test} ... ok"
        require(label in text(name) and label in text("linux-tests-final"), f"binding {test}/{trial}")
    name = f"repeat-components-{trial}"
    require(totals(name) == [(1, 0)], f"component repeat {trial}")
    for test in component:
        label = f"test diagnostic_process::credentials::namespace_checked::tests::{test} ... ok"
        require(label in text(name) and label in text("linux-tests-final"), f"component {test}/{trial}")
require(manifest["seed"] == "0x7562026" and manifest["modeled_mapping_mutations_per_seeded_run"] == 256, "seed pin")
require(set(manifest["proof_boundaries"]) == set(json.loads("[\"real_issuer_enrollment\",\"positive_fixed_production_inspection\",\"signed_context_at_status_open_attested\",\"worker_mount_namespace_proved\",\"initial_host_namespace_proved\",\"file_opener_credentials_attested\",\"atomic_cross_file_snapshot\",\"continuous_context_or_writer_revocation\",\"regular_file_io_deadline\",\"all_threads_proved\",\"durable_epoch_or_refusal_registry\",\"production_preparation\",\"authenticated_original_start\",\"new_helper_ipc_or_live_route\",\"host_mutation\",\"product_workload\",\"performance_measurement\",\"qualification\",\"full_workspace_verification\"]")), "boundaries")
require(all(value is False for value in manifest["proof_boundaries"].values()), "closed boundaries")
require(manifest["missing_api_wsl_exit"] == 1 and manifest["require_ship_native_exit"] == 1, "refusal exits")
require(manifest["baseline_source"] == "4fa462b2cd339f6e4bab4e7371bcdaa8125dea98", "baseline source")
require(manifest["preregistered_design_commit"] == "c83f320ea5b14a9929b83f686bcf2d0db608cd5c", "preregistration")
require(manifest["implementation_commit"] == "72322696b422f3621763fb2af1c9535c0f1d9a7c", "implementation")
require(manifest["implementation_tree"] == "b42f5798bfde51d1b26f3db25be5e7e335b633bf", "implementation tree")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen qualification bytes")
print("signed policy/credentials: exact local evidence; signed status-opening context, production and start remain closed")
