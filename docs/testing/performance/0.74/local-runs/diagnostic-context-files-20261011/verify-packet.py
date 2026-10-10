"""Read-only local safety evidence; never worker enrollment or qualification."""
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
require(manifest["schema_version"] == "diagnostic-context-files-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-context-account-opening-not-kernel-worker-enrollment", "state")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
expected_results = json.loads("{\"windows_baseline_passed\":43,\"windows_final_passed\":43,\"windows_new_linux_target_tests\":0,\"first_green_integration_passed\":9,\"linux_final_passed\":308,\"linux_existing_ignored\":1,\"new_integration_tests\":9,\"new_unit_tests\":4,\"additional_linux_repetitions\":3,\"root_final_passed\":131,\"linux_new_root_guard_passed\":1}")
require(manifest["results"] == expected_results, "result claims")
expected_names = set(json.loads("[\"api-red.log\",\"baseline.log\",\"book.log\",\"contract.log\",\"docs.log\",\"first-green.log\",\"format.log\",\"isolation.log\",\"links.log\",\"linux-check.log\",\"linux-lint.log\",\"linux-root.log\",\"linux-tests.log\",\"repeat-integration-.log\",\"repeat-integration-1.log\",\"repeat-integration-2.log\",\"repeat-integration-3.log\",\"repeat-unit-.log\",\"repeat-unit-1.log\",\"repeat-unit-2.log\",\"repeat-unit-3.log\",\"root-tests.log\",\"ship.log\",\"windows-check.log\",\"windows-lint.log\",\"windows-tests.log\",\"docs-final.log\",\"registry.log\"]"))
require(set(manifest["logs"]) == expected_names, "declared log inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual log inventory")
for name, pin in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(passed), int(ignored)) for passed, ignored in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline") == [(15, 0), (15, 0), (13, 0)], "baseline portable totals")
require(totals("windows-tests") == [(15, 0), (15, 0), (0, 0), (0, 0), (0, 0), (13, 0)], "Windows totals")
require(totals("linux-tests") == [(234, 1), (29, 0), (9, 0), (2, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "Linux totals")
require(totals("root-tests") == [(95, 0), (13, 0), (23, 0)], "root totals")
require(totals("first-green") == [(9, 0)], "first green")
for name, rows in {
    "api-red": ["error[E0432]", "could not find `account_files`"],
    "linux-root": ["diagnostic_context_files_open_inside_original_context_and_keep_start_closed ... ok", "1 passed; 0 failed"],
    "format": ["scoped rustfmt 1.94.0 passed"],
    "docs-final": ["doc-check: OK", "performance-contract-check 0.74: OK", "Checked 51 markdown files", "HTML book written"],
    "registry": ["w12_evidence_skeleton_is_exact_and_fail_closed ... ok", "1 passed; 0 failed"],
    "isolation": ["all actual qualification contract_inputs: unchanged", "product/cache/native code and legacy authority/routes: unchanged"],
    "ship": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    require(all(row in text(name) for row in rows), f"claim rows {name}")
require("compiler unexpectedly panicked" not in text("api-red"), "ordinary missing API")
for name in ["windows-check", "windows-lint", "linux-check", "linux-lint"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"package gate {name}")
expected_tests = json.loads("[{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_owned_roundtrip_and_successful_drop_preserve_policy\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_refusal_and_revocation_precede_any_account_path_access\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_actual_context_failure_precedes_missing_file_path\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_constructor_open_mapping_and_document_failures_latch_original\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_changed_names_contents_and_restoration_cannot_refresh\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_policy_failure_after_open_latches_through_drop\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_fixture_cannot_claim_fixed_root_owner_or_symlink_origin\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_seeded_resigned_foreign_context_cannot_enroll_owned_files\"},{\"source\":\"tools/long-run-supervisor-074/tests/diagnostic_context_files.rs\",\"name\":\"context_files_independent_concurrent_guards_do_not_share_refusal\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_context_files.rs\",\"name\":\"context_files_opening_and_later_orders_are_exact\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_context_files.rs\",\"name\":\"context_files_every_failure_stops_and_preserves_original_error\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_context_files.rs\",\"name\":\"context_files_prior_refusal_never_opens_or_reads\"},{\"source\":\"tools/long-run-supervisor-074/src/diagnostic_context_files.rs\",\"name\":\"context_files_both_origin_guards_cannot_be_sent_or_shared\"}]")
require(manifest["new_tests"] == expected_tests, "new test inventory")
integration = [row["name"] for row in expected_tests if "/tests/" in row["source"]]
units = [row["name"] for row in expected_tests if "/src/" in row["source"]]
for trial in range(1, 4):
    integration_log = f"repeat-integration-{trial}"
    unit_log = f"repeat-unit-{trial}"
    require(totals(integration_log) == [(9, 0)], f"integration repeat {trial}")
    require(totals(unit_log) == [(4, 0)], f"unit repeat {trial}")
    require("context files mutation seed=0x7552026; mutations=64" in text(integration_log), f"seed repeat {trial}")
    for name in integration:
        label = f"test {name} ... ok"
        require(label in text("linux-tests") and label in text(integration_log), f"integration {name}/{trial}")
    for name in units:
        label = f"test diagnostic_worker_policy::local_context::account_files::tests::{name} ... ok"
        require(label in text("linux-tests") and label in text(unit_log), f"unit {name}/{trial}")
# The earlier suffix-less captures each retain only one iteration, not three.
require(totals("repeat-integration-") == [(9, 0)], "initial integration capture limitation")
require(totals("repeat-unit-") == [(4, 0)], "initial unit capture limitation")
require("retained only" in manifest["capture_limitation"] or "retain only" in manifest["capture_limitation"], "capture limitation declared")
require(manifest["seed"] == "0x7552026" and manifest["valid_mutations_per_seeded_run"] == 64, "seed pin")
expected_boundaries = set(json.loads("[\"real_issuer_enrollment\",\"positive_fixed_production_account_inspection\",\"kernel_credentials_joined\",\"physical_host_attestation\",\"initial_host_namespace_proved\",\"atomic_cross_file_snapshot\",\"file_opener_credentials_attested\",\"continuous_context_or_writer_revocation\",\"regular_file_io_deadline\",\"all_threads_proved\",\"durable_epoch_or_refusal_registry\",\"production_preparation\",\"authenticated_original_start\",\"new_helper_ipc_or_live_route\",\"host_mutation\",\"product_workload\",\"performance_measurement\",\"qualification\",\"full_workspace_verification\"]"))
require(set(manifest["proof_boundaries"]) == expected_boundaries, "boundary inventory")
require(all(value is False for value in manifest["proof_boundaries"].values()), "closed boundaries")
require(manifest["missing_api_wsl_exit"] == 1 and manifest["require_ship_native_exit"] == 1, "refusal exits")
require(manifest["baseline_source"] == "81a3a5574121ea807218c23c5694c723dd6e71d8", "baseline")
require(manifest["preregistered_design_commit"] == "99abe16df6df3f4bcb8213169914ee69aeb3cd60", "preregistration")
require(manifest["implementation_commit"] == "fb8ddc733d0815c403a47948ee04a1fe2be9b820", "implementation")
require(manifest["implementation_tree"] == "77576c165e4112a7e0ea69a74acb13a147569964", "implementation tree")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen qualification bytes")
print("context/account opening: exact local evidence; kernel credentials, production preparation and start remain closed")
