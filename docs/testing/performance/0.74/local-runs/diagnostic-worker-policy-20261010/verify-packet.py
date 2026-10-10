"""Read-only integrity and scope checks, not host enrollment or qualification."""
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-worker-policy-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-policy-bytes-not-host-enrollment", "state")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
names = {
    "baseline", "api-red", "first-green", "windows", "windows-check", "windows-lint",
    "linux", "linux-check", "linux-lint", "repeats", "root", "reviewed-check", "reviewed-lint",
    "linux-reviewed-check", "linux-reviewed-lint", "format", "doc", "contract", "links",
    "book", "ship", "docs-final", "registry", "isolation",
}
expected_names = {name + ".log" for name in names}
require(set(manifest["logs"]) == expected_names, "declared log inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual log inventory")
for name, expected in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == expected["bytes"], f"size: {name}")
    require(hashlib.sha256(data).hexdigest() == expected["sha256"], f"digest: {name}")

for name, rows in {
    "baseline.log": ["15 passed; 0 failed"],
    "api-red.log": ["error[E0432]", "could not find `diagnostic_worker_policy`"],
    "first-green.log": ["12 passed; 0 failed"],
    "windows.log": ["15 passed; 0 failed", "13 passed; 0 failed"],
    "linux.log": ["222 passed; 0 failed; 1 ignored", "29 passed; 0 failed", "2 passed; 0 failed", "3 passed; 0 failed", "13 passed; 0 failed"],
    "root.log": ["92 passed; 0 failed", "13 passed; 0 failed", "23 passed; 0 failed"],
    "registry.log": ["1 passed; 0 failed"],
    "format.log": ["scoped rustfmt 1.94.0 passed"],
    "docs-final.log": ["doc-check: OK", "performance-contract-check 0.74: OK", "HTML book written"],
    "isolation.log": ["legacy NSS/numeric/namespace/receipt/production routes: unchanged"],
    "ship.log": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    text = (packet / name).read_text(encoding="utf-8-sig")
    require(all(row in text for row in rows), f"claim rows: {name}")

tests = {
    "worker_policy_roundtrip_is_idempotent_not_host_enrollment",
    "worker_policy_unsigned_field_changes_never_adopt_new_values",
    "worker_policy_resigned_invalid_identity_and_context_refuse",
    "worker_policy_canonical_strict_json_and_budgets_refuse",
    "worker_policy_external_trust_rejects_weak_keys_and_invalid_pins",
    "worker_policy_signatures_and_foreign_domains_refuse",
    "worker_policy_asserted_host_boot_and_both_namespaces_match_exactly",
    "worker_policy_group_rules_preserve_exact_lists_and_union_bound",
    "worker_policy_revocation_is_first_error_and_cannot_restore",
    "worker_policy_context_and_envelope_drift_latch_without_refresh",
    "worker_policy_oversized_revalidation_and_key_pin_changes_latch",
    "worker_policy_seeded_valid_resigned_mutations_keep_original_pin",
    "worker_policy_concurrent_guards_share_no_refusal_or_authority",
}
require(len(manifest["new_tests"]) == 13 and set(manifest["new_tests"]) == tests, "new test inventory")
for name in ["windows.log", "linux.log"]:
    text = (packet / name).read_text(encoding="utf-8-sig")
    require(all(f"test {test} ... ok" in text for test in tests), f"case rows: {name}")
    require("worker-policy seed=0x7512026; mutations=256" in text, f"seed: {name}")
repeats = (packet / "repeats.log").read_text(encoding="utf-8-sig")
require(repeats.count("13 passed; 0 failed") == 3, "three full repetitions")
require(repeats.count("worker-policy seed=0x7512026; mutations=256") == 3, "three seed repetitions")
require(all(repeats.count(f"test {test} ... ok") == 3 for test in tests), "repeated case inventory")
boundaries = {
    "real_issuer_key_enrollment", "fixed_local_account_files_observed", "trusted_host_namespace_proved",
    "all_threads_proved", "durable_pin_epoch_or_refusal_registry", "production_preparation",
    "authenticated_original_start", "new_helper_ipc_or_live_route", "host_mutation",
    "product_workload", "performance_measurement", "qualification", "full_workspace_verification",
}
require(set(manifest["proof_boundaries"]) == boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed boundaries")
require(manifest["missing_api_native_exit"] == 101, "missing API refusal")
require(manifest["require_ship_native_exit"] == 1, "ship refusal")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen qualification bytes")
print("signed worker policy: exact local evidence; host enrollment and production remain closed")
