"""Read-only integrity/claim checks for the local worker authority audit."""
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-host-policy-audit-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-audit-not-authority", "state")
require(manifest["evidence_class"] == "working-tree-local-regression-not-release-qualification", "class")
expected_names = {
    "baseline.log", "green.log", "check.log", "lint.log", "linux.log",
    "linux-check.log", "linux-lint.log", "root.log", "doc.log", "contract.log",
    "links.log", "book.log", "ship.log", "isolation.log", "registry.log", "docs-final.log", "post-retained.log",
}
require(set(manifest["logs"]) == expected_names, "declared capture inventory")
require({p.name for p in packet.glob("*.log")} == expected_names, "actual capture inventory")
for name, expected in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == expected["bytes"], f"size: {name}")
    require(hashlib.sha256(data).hexdigest() == expected["sha256"], f"digest: {name}")

for name, rows in {
    "baseline.log": ["7 passed; 0 failed"],
    "green.log": ["9 passed; 0 failed"],
    "linux.log": ["10 passed; 0 failed"],
    "root.log": ["91 passed; 0 failed", "13 passed; 0 failed", "23 passed; 0 failed"],
    "registry.log": ["1 passed; 0 failed"],
    "post-retained.log": ["1 passed; 0 failed", "doc-check: OK", "performance-contract-check 0.74: OK"],
    "ship.log": ["ship admission is closed while candidate identity and release qualification are incomplete", "exit code: 1"],
    "docs-final.log": ["doc-check: OK", "performance-contract-check 0.74: OK", "HTML book written"],
    "isolation.log": ["production prefix: unchanged", "scoped formatting passed"],
}.items():
    text = (packet / name).read_text(encoding="utf-8-sig")
    require(all(row in text for row in rows), f"capture claims: {name}")
require(manifest["new_tests"] == [
    "host_observation_v1_rejects_worker_authority_extensions",
    "provisioning_v1_rejects_worker_authority_extensions",
], "exact new test inventory")
for name in ["green.log", "linux.log"]:
    text = (packet / name).read_text(encoding="utf-8-sig")
    for test in manifest["new_tests"]:
        require(f"test host_receipt::tests::{test} ... ok" in text, f"new case: {test}")

expected_boundaries = {
    "trusted_worker_policy_implemented", "trusted_nss_provider_attested",
    "trusted_host_namespace_proved", "real_worker_enrollment", "production_preparation",
    "authenticated_diagnostic_start", "host_operations", "product_workload",
    "qualification", "performance_measurement", "full_workspace_verification",
}
require(set(manifest["proof_boundaries"]) == expected_boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed boundaries")
require(manifest["require_ship_native_exit"] == 1, "expected ship refusal")
require(manifest["post_retention_registry_and_contract_checked"] is True, "post-retention gate")
require(manifest["authority_field_mutations_per_test"] == 8, "mutation count")
frozen = repository / "docs/testing/performance/0.74/qualification-manifest.toml"
require(manifest["qualification_manifest_sha256"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc", "frozen pin")
require(hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"], "frozen qualification")
print("worker authority audit: exact local captures; production authority remains closed")
