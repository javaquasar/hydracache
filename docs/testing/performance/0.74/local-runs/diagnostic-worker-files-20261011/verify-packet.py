"""Read-only local log consistency, not production enrollment or qualification."""
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-worker-files-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-fixed-reader-not-host-enrollment", "state")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
names = {
    "baseline", "api-red", "api-red-reviewed", "api-red-short", "first-green",
    "linux", "linux-reviewed", "linux-final", "linux-check", "linux-lint",
    "linux-final-check", "linux-final-lint", "windows", "windows-check", "windows-lint",
    "root", "root-reviewed", "format", "docs", "ship", "repeats", "isolation",
    "isolation-reviewed", "docs-final", "registry", "linux-root",
}
expected_names = {name + ".log" for name in names}
require(set(manifest["logs"]) == expected_names, "declared log inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual log inventory")
for name, expected in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == expected["bytes"], f"size: {name}")
    require(hashlib.sha256(data).hexdigest() == expected["sha256"], f"digest: {name}")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


for name, rows in {
    "baseline": ["15 passed; 0 failed", "13 passed; 0 failed"],
    "api-red": ["compiler unexpectedly panicked", "rustc --explain E0432", "exit status: 101"],
    "api-red-reviewed": ["compiler unexpectedly panicked", "rustc --explain E0432"],
    "api-red-short": ["error[E0432]", "could not find `local_files`"],
    "first-green": ["12 passed; 0 failed"],
    "linux": ["222 passed; 1 failed; 1 ignored", "account_positional_read_refuses_short_overrun_error_and_midread_drift ... FAILED"],
    "linux-final": ["223 passed; 0 failed; 1 ignored", "29 passed; 0 failed", "2 passed; 0 failed", "3 passed; 0 failed", "13 passed; 0 failed"],
    "windows": ["15 passed; 0 failed", "13 passed; 0 failed", "0 passed; 0 failed"],
    "root": ["93 passed; 0 failed", "13 passed; 0 failed", "19 passed; 4 failed", "unregistered cfg-gated target hydracache-long-run-supervisor-074/diagnostic_worker_files"],
    "root-reviewed": ["93 passed; 0 failed", "13 passed; 0 failed", "23 passed; 0 failed"],
    "format": ["scoped rustfmt 1.94.0 passed"],
    "docs-final": ["doc-check: OK", "performance-contract-check 0.74: OK", "HTML book written"],
    "registry": ["1 passed; 0 failed"],
    "linux-root": ["diagnostic_worker_files_preserve_fixed_names_original_policy_and_closed_start ... ok", "1 passed; 0 failed"],
    "isolation-reviewed": ["all actual qualification contract_inputs: unchanged", "product/cache/native code and legacy authority/routes: unchanged"],
    "ship": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    require(all(row in text(name) for row in rows), f"claim rows: {name}")
require("compiler unexpectedly panicked" not in text("api-red-short"), "short-format diagnostic is ordinary E0432")

tests = {
    "worker_files_fixture_roundtrip_keeps_policy_private_and_healthy",
    "worker_files_refused_or_revoked_policy_precedes_any_path_access",
    "worker_files_fixture_rejects_production_paths_and_root_assertions",
    "worker_files_resigned_strict_grammar_and_ambiguity_refuse",
    "worker_files_mapping_and_exact_supplementary_groups_refuse_inference",
    "worker_files_whole_documents_and_budgets_are_not_worker_only",
    "worker_files_symlink_hardlink_fifo_directory_and_socket_refuse",
    "worker_files_modes_ownership_and_symlink_ancestors_refuse",
    "worker_files_named_leaf_and_directory_replacement_refuse_after_restore",
    "worker_files_inplace_rewrite_growth_and_trust_failures_latch_original",
    "worker_files_seeded_valid_unrelated_changes_keep_original_digest_pin",
    "worker_files_independent_concurrent_readers_share_no_refusal",
    "worker_files_exact_byte_line_record_and_group_bounds_accept",
}
unit = "diagnostic_worker_policy::local_files::tests::account_positional_read_refuses_short_overrun_error_and_midread_drift"
require(len(manifest["new_integration_tests"]) == 13 and set(manifest["new_integration_tests"]) == tests, "new test inventory")
require(all(f"test {test} ... ok" in text("linux-final") for test in tests), "final cases")
require(f"test {unit} ... ok" in text("linux-final"), "final unit")
repeats = text("repeats")
require(repeats.count("13 passed; 0 failed") == 3 and repeats.count("1 passed; 0 failed") == 3, "three integration/unit repetitions")
require(repeats.count("worker-files seed=0x7522026; mutations=64") == 3, "three seeded repetitions")
require(all(repeats.count(f"test {test} ... ok") == 3 for test in tests), "repeated case inventory")
require(repeats.count(f"test {unit} ... ok") == 3, "repeated unit inventory")
for name in ["windows-check", "windows-lint", "linux-final-check", "linux-final-lint"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"package gate: {name}")
boundaries = {
    "real_issuer_enrollment", "positive_fixed_production_inspection", "trusted_host_namespace_proved",
    "atomic_cross_file_snapshot", "continuous_context_or_writer_revocation", "regular_file_io_deadline",
    "all_threads_proved", "durable_epoch_or_refusal_registry", "production_preparation",
    "authenticated_original_start", "new_helper_ipc_or_live_route", "host_mutation", "product_workload",
    "performance_measurement", "qualification", "full_workspace_verification",
}
require(set(manifest["proof_boundaries"]) == boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed boundaries")
require(manifest["missing_api_native_exit"] == 101 and manifest["require_ship_native_exit"] == 1, "refusal exits")
require(manifest["first_linux_native_exit"] == 101 and manifest["first_root_native_exit"] == 101, "retained failures")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen qualification bytes")
print("fixed worker files: exact local evidence and negatives; host enrollment and production remain closed")
