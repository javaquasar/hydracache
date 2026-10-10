"""Read-only local safety verification, never enrollment or qualification."""
import hashlib
import json
from pathlib import Path


def require(condition, message):
    if not condition:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = Path(__file__).resolve().parents[6]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-worker-context-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-observed-context-not-host-enrollment", "state")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
require(manifest["results"] == {
    "windows_baseline_passed": 43, "windows_final_passed": 43,
    "windows_new_linux_target_tests": 0, "first_green_integration_passed": 5,
    "linux_final_passed": 295, "linux_existing_ignored": 1,
    "new_integration_tests": 5, "new_unit_tests": 7,
    "additional_linux_repetitions": 3, "root_final_passed": 130,
    "linux_new_root_guard_passed": 1,
}, "result claims")
names = {
    "baseline", "api-red", "first-green", "linux", "linux-check", "linux-lint", "linux-root",
    "windows", "root", "windows-check", "windows-lint", "format", "isolation", "repeats",
    "docs", "ship", "docs-final", "registry",
}
expected = {name + ".log" for name in names}
require(set(manifest["logs"]) == expected, "declared inventory")
require({p.name for p in packet.glob("*.log")} == expected, "actual inventory")
for name, pin in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


for name, rows in {
    "baseline": ["15 passed; 0 failed", "13 passed; 0 failed"],
    "windows": ["15 passed; 0 failed", "13 passed; 0 failed", "0 passed; 0 failed"],
    "api-red": ["error[E0432]", "could not find `local_context`"],
    "first-green": ["5 passed; 0 failed"],
    "linux": ["230 passed; 0 failed; 1 ignored", "29 passed; 0 failed", "2 passed; 0 failed", "3 passed; 0 failed", "5 passed; 0 failed", "13 passed; 0 failed"],
    "root": ["94 passed; 0 failed", "13 passed; 0 failed", "23 passed; 0 failed"],
    "linux-root": ["diagnostic_worker_context_preserves_external_pins_typed_threads_and_closed_start ... ok", "1 passed; 0 failed"],
    "format": ["scoped rustfmt 1.94.0 passed"],
    "docs-final": ["doc-check: OK", "performance-contract-check 0.74: OK", "HTML book written"],
    "registry": ["1 passed; 0 failed"],
    "isolation": ["all actual qualification contract_inputs: unchanged", "product/cache/native code and legacy authority/routes: unchanged"],
    "ship": ["ship admission is closed while candidate identity and release qualification are incomplete", "require-ship native exit=1"],
}.items():
    require(all(row in text(name) for row in rows), f"claim rows {name}")
require("compiler unexpectedly panicked" not in text("api-red"), "ordinary E0432")
for name in ["windows-check", "windows-lint", "linux-check", "linux-lint"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"package gate {name}")
integration = [row["name"] for row in manifest["new_tests"] if "/tests/" in row["source"]]
units = [row["name"] for row in manifest["new_tests"] if "/src/" in row["source"]]
require(len(integration) == len(set(integration)) == 5, "integration inventory")
require(len(units) == len(set(units)) == 7, "unit inventory")
for name in integration:
    require(f"test {name} ... ok" in text("linux"), f"final integration {name}")
    require(text("repeats").count(f"test {name} ... ok") == 3, f"repeated integration {name}")
for name in units:
    label = f"test diagnostic_worker_policy::local_context::tests::{name} ... ok"
    require(label in text("linux"), f"final unit {name}")
    require(text("repeats").count(label) == 3, f"repeated unit {name}")
require(text("repeats").count("5 passed; 0 failed") == 3, "integration repetitions")
require(text("repeats").count("7 passed; 0 failed") == 3, "unit repetitions")
require(text("repeats").count("worker context mutation seed=0x7542026") == 3, "seed repetitions")
require(manifest["seed"] == "0x7542026" and manifest["valid_mutations_per_seeded_run"] == 64, "seed pin")
boundaries = {
    "real_issuer_enrollment", "physical_host_attestation", "initial_host_namespace_proved",
    "atomic_cross_file_snapshot", "continuous_context_or_writer_revocation", "regular_file_io_deadline",
    "all_threads_proved", "durable_epoch_or_refusal_registry", "context_account_credential_composition",
    "production_preparation", "authenticated_original_start", "new_helper_ipc_or_live_route",
    "host_mutation", "product_workload", "performance_measurement", "qualification", "full_workspace_verification",
}
require(set(manifest["proof_boundaries"]) == boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed boundaries")
require(manifest["missing_api_wsl_exit"] == 1 and manifest["require_ship_native_exit"] == 1, "refusal exits")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen qualification bytes")
require(manifest["implementation_commit"] == "f861e460137d6a35ba2dc519310d4796f79aefdd", "implementation")
require(manifest["implementation_tree"] == "c7479506f3fffe70ffc3de0a92196e68652d2ae7", "implementation tree")
print("observed worker context: exact local evidence; host enrollment, preparation and start remain closed")
