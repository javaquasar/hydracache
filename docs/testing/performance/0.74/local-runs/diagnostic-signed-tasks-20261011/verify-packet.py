"""Read-only local signed-task composition evidence, never host enrollment."""
import hashlib
import json
import re
import subprocess
from pathlib import Path


def require(ok, message):
    if not ok:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = packet.parents[5]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-signed-tasks-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-original-signed-tasks-not-host-enrollment", "state")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
require(manifest["baseline_source"] == "4c3f7d463d8886bcd607b45e4dcd5e81d8bb7d20", "baseline")
require(manifest["preregistration_source"] == "8e404ceb893c1b7fa9e3ff035f8ee5e068947414", "preregistration")
require(manifest["implementation_source"] == "f624df7f9c9da31f18f0ee2841506253df382bc6", "implementation")
require(manifest["toolchain"] == "1.94.0" and manifest["msrv"] == "1.88.0", "toolchain")
require(manifest["original_signed_composition"] is True, "local composition")
require(manifest["limits"] == {"tasks": 128, "directory_calls": 16, "directory_bytes": 65536}, "bounds")
require(manifest["modeled_seed"] == "0x7632026" and manifest["modeled_schedules"] == 256, "model")
expected_boundaries = set(["controlled_fixed_host_positive_enrollment","initial_host_namespace_proved","atomic_or_continuous_proof","mount_content_immutability_proved","file_opener_credentials_attested","durable_authority_or_refusal","real_issuer_enrolled","production_preparation_or_start","authenticated_original_start","autonomous_watchdog_or_writer_revocation","new_helper_ipc_or_live_route","host_mutation_allowed","product_workload_allowed","performance_claim_allowed","qualification_or_admission_allowed","full_workspace_run_for_this_slice","real_foreign_namespace_mutation","real_nonleader_credential_mutation","guard_send_or_sync","public_replacement_or_refresh","old_public_behavior_changed"])
require(set(manifest["proof_boundaries"]) == expected_boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed boundaries")
require(len(manifest["logs"]) == 41, "capture count")
require(set(manifest["logs"]) == {p.name for p in packet.glob("*.log")}, "capture inventory")
for name, pin in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")


def text(name):
    return (packet / name).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(p), int(i)) for p, i in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


expected_results = {"linux_scoped_passed": 394, "linux_existing_ignored": 1,
                    "new_unit_tests": 11, "new_api_tests": 1, "serial_repetitions": 3,
                    "windows_package_passed": 189, "windows_new_linux_cases": 0,
                    "root_contract_passed": 101, "root_evidence_passed": 13,
                    "governance_passed": 23}
require(manifest["results"] == expected_results, "result claims")
require(totals("baseline.log") == [(13, 0), (11, 0)], "unchanged baseline")
require("error[E0433]" in text("api-red.log"), "retained wrong API path")
require("error[E0432]" in text("api-absent-red.log") and "task_binding" in text("api-absent-red.log"), "test-first API")
require("needless_range_loop" in text("linux-clippy.log"), "retained lint negative")
require("no test target named" in text("root-tests.log"), "retained wrong root target")
for name in ["book.log", "book-reviewed.log"]:
    require("ERROR failed to read" in text(name), f"retained book path {name}")
require(totals("linux-reviewed-tests.log") == [(318, 1), (29, 0), (9, 0), (2, 0), (2, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "scoped Linux")
require(sum(p for p, _ in totals("windows-reviewed-tests.log")) == 189, "portable Windows")
require(totals("root-reviewed-tests.log") == [(101, 0), (13, 0), (23, 0)], "root gates")
require(totals("packet-registry.log") == [(1, 0)], "post-packet registry")
require(len(manifest["new_unit_test_names"]) == 11 and len(set(manifest["new_unit_test_names"])) == 11, "new test names")
for trial in range(1, 4):
    name = f"reviewed-repeat-{trial}.log"
    require(totals(name) == [(11, 0)], f"serial repeat {trial}")
    require("signed tasks seed=0x7632026; schedules=256" in text(name), "seed")
    for test in manifest["new_unit_test_names"]:
        require(f"::tests::{test} ..." in text(name), f"test {trial}/{test}")
require("signed_tasks_api_preserves_fixed_fixture_original_mount_types ... ok" in text("linux-reviewed-tests.log"), "typed API")
for name in ["linux-reviewed-check.log", "linux-reviewed-clippy.log", "linux-msrv.log",
             "windows-reviewed-check.log", "windows-reviewed-clippy.log", "windows-msrv.log",
             "root-clippy.log", "linux-rustdoc.log"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"check {name}")
require("doc-check: OK" in text("docs.log"), "docs")
require("performance-contract-check 0.74: OK" in text("contract.log"), "local contract")
require("HTML book written" in text("book-final.log"), "book")
require("HTML book written" in text("book-postpacket.log"), "post-packet book")
require("doc-check: OK" in text("docs-final.log") and "performance-contract-check 0.74: OK" in text("docs-final.log"), "final docs/contract")
require("local Markdown file targets resolved=175" in text("links-final.log"), "local file links")
require(text("format.log") == "", "format")
require("ship admission is closed while candidate identity and release qualification are incomplete" in text("ship-refusal.log"), "closed ship")
require("(exit code: 1)" in text("ship-refusal.log"), "native ship refusal")
require("frozen inputs and prior product/runtime routes: unchanged" in text("isolation.log"), "isolation")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen bytes")
for field in ["baseline", "implementation"]:
    source = manifest[field + "_source"]
    tree = subprocess.check_output(["git", "rev-parse", source + "^{tree}"], cwd=repository, text=True).strip()
    require(tree == manifest[field + "_tree"], f"{field} tree")
require(subprocess.run(["git", "merge-base", "--is-ancestor", manifest["preregistration_source"], manifest["implementation_source"]], cwd=repository, check=False).returncode == 0, "preregistered ancestry")
print(f"signed tasks: {len(manifest['logs'])} exact local captures; controlled-host enrollment and production remain closed")
