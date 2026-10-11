"""Read-only original-task local evidence, never production enrollment."""
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
require(manifest["schema_version"] == "diagnostic-task-set-local-evidence-074-v1", "schema")
require(manifest["state"] == "retained-local-original-tasks-not-signed-enrollment", "state")
require(manifest["baseline_source"] == "5efc0cfbe3e1defbefffdfa3dd0ddb012db04a36", "baseline")
require(manifest["preregistration_source"] == "a2c91dc0de343262e7accddb4928db1fbf205989", "preregistration")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
require(set(manifest["logs"]) == {p.name for p in packet.glob("*.log")}, "capture inventory")
for name, pin in manifest["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")
expected_boundaries = set(["original_signed_composition","initial_host_namespace_proved","atomic_or_continuous_proof","mount_content_immutability_proved","file_opener_credentials_attested","durable_authority_or_refusal","real_issuer_enrolled","positive_fixed_host_enrollment","production_preparation_or_start","authenticated_original_start","autonomous_watchdog_or_writer_revocation","new_helper_ipc_or_live_route","host_mutation_allowed","product_workload_allowed","performance_claim_allowed","qualification_or_admission_allowed","full_workspace_run_for_this_slice","real_foreign_namespace_mutation","real_nonleader_credential_mutation","guard_send_or_sync","public_replacement_or_refresh","old_public_behavior_changed"])
require(set(manifest["proof_boundaries"]) == expected_boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed proof boundaries")
require(len(manifest["new_unit_test_names"]) == 11 and len(set(manifest["new_unit_test_names"])) == 11, "test inventory")
require(manifest["toolchain"] == "1.94.0" and manifest["msrv"] == "1.88.0", "toolchain")
require(manifest["limits"] == {"tasks": 128, "directory_calls": 16, "directory_bytes": 65536}, "bounds")
require(manifest["results"] == {"linux_scoped_passed": 382, "linux_existing_ignored": 1, "new_unit_tests": 11, "new_api_tests": 1, "serial_repetitions": 3, "windows_package_passed": 189, "windows_new_linux_cases": 0, "root_contract_passed": 100, "root_evidence_passed": 13, "final_governance_passed": 23}, "result claims")


def text(name):
    return (packet / name).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(p), int(i)) for p, i in re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline-linux.log") == [(296, 1)], "baseline")
require("error[E0432]" in text("task-api-red.log"), "test-first API")
require("9 passed; 1 failed" in text("task-set-first.log"), "retained fixture negative")
require("manual_is_multiple_of" in text("linux-clippy.log"), "retained lint negative")
require("305 passed; 1 failed; 1 ignored" in text("linux-tests-final.log"), "retained parallel membership negative")
require(totals("linux-reviewed-tests.log") == [(307, 1), (29, 0), (9, 0), (2, 0), (1, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "Linux scoped results")
require(sum(p for p, _ in totals("windows-tests.log")) == 189, "portable results")
require(totals("root-reviewed-tests.log") == [(100, 0), (13, 0)], "root green results before registration")
require("19 passed; 4 failed" in text("root-reviewed-tests.log") and "unregistered cfg-gated target" in text("root-reviewed-tests.log"), "retained governance negative")
require(totals("governance-final.log") == [(23, 0)], "final governance")
require(totals("task-api-green.log") == [(1, 0)], "API")
for trial in range(1, 4):
    name = f"repeat-{trial}.log"
    require(totals(name) == [(11, 0)], f"repeat {trial}")
    require("task-set modeled seed=0x7612026; schedules=256" in text(name), "schedule seed")
    require("task directory modeled seed=0x7622026; mutations=512" in text(name), "directory seed")
    for test in manifest["new_unit_test_names"]:
        require(f"::tests::{test} ..." in text(name), f"test {trial}/{test}")
for name in ["linux-check.log", "linux-clippy-final.log", "linux-msrv.log", "windows-check.log", "windows-clippy.log", "windows-msrv.log", "root-clippy.log", "linux-rustdoc.log"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"check {name}")
require("doc-check: OK" in text("docs-final.log"), "docs")
require("performance-contract-check 0.74: OK" in text("docs-final.log"), "contract")
require("ship admission is closed while candidate identity and release qualification are incomplete" in text("ship-refusal.log"), "ship boundary")
require("require-ship exit=1" in text("ship-refusal.log"), "ship refusal exit")
require("frozen inputs and prior public/runtime routes: unchanged" in text("isolation.log"), "isolation")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen bytes")
implementation = manifest["implementation_source"]
require(implementation == "8c8e55aa68115f7633b9ecfe12367384b37474d3", "implementation source")
tree = subprocess.check_output(["git", "rev-parse", implementation + "^{tree}"], cwd=repository, text=True).strip()
require(tree == manifest["implementation_tree"], "implementation tree")
require(subprocess.run(["git", "merge-base", "--is-ancestor", manifest["preregistration_source"], implementation], cwd=repository, check=False).returncode == 0, "preregistered ancestry")
print(f"task set: {len(manifest['logs'])} exact local captures; signed enrollment and production remain closed")
