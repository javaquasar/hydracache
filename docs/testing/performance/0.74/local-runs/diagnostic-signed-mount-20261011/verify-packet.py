"""Read-only local signed mount evidence; never production enrollment."""
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
expected_logs = set(["adapter-repeat.log","api-red.log","baseline.log","dependency-policy-windows.log","dependency-policy.log","docs-final.log","docs-windows.log","docs.log","first-green.log","hardened-linux-clippy.log","hardened-linux-tests.log","hardened-repeat-1.log","hardened-repeat-2.log","hardened-repeat-3.log","hardened-root-guards.log","hardened-root.log","isolation.log","linux-check.log","linux-clippy-final.log","linux-clippy.log","linux-root.log","linux-tests-final.log","linux-tests.log","msrv-windows.log","registry-final.log","registry.log","repeat-1.log","repeat-2.log","repeat-3.log","reviewed-linux-clippy.log","reviewed-linux-tests.log","reviewed-repeat-1.log","reviewed-repeat-2.log","reviewed-repeat-3.log","root-tests-final.log","root-tests.log","serial-repeat-1.log","serial-repeat-2.log","serial-repeat-3.log","ship.log","windows-check.log","windows-clippy.log","windows-tests.log","workspace-verify.log"])
require(m["schema_version"] == "diagnostic-signed-mount-local-evidence-074-v1", "schema")
require(m["state"] == "retained-local-original-signed-leader-mount-not-production-enrollment", "state")
require(m["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
require(set(m["logs"]) == expected_logs, "declared inventory")
require({p.name for p in packet.glob("*.log")} == expected_logs, "actual inventory")
for name, pin in m["logs"].items():
    data = (packet / name).read_bytes()
    require(len(data) == pin["bytes"], f"size {name}")
    require(hashlib.sha256(data).hexdigest() == pin["sha256"], f"hash {name}")
require(m["baseline_source"] == "b07a655bb42237b3eb80e774b953017e39850dd3", "baseline")
require(m["preregistered_design_commit"] == "2f9da9e07fcce93309488b7d12ea1c80e483cac9", "preregistration")
require(m["implementation_commit"] == "7cd8b43061638a66b10a3df431a642cd95f19cb3", "implementation")
require(m["implementation_tree"] == "d6e7904fe644d8f1a784ba44650d8cd474ba3a33", "tree")
require(m["initial_implementation_commit"] == "1a7d946ed542908ca04c1aa3d4c5572973d90c4d", "initial source")
require(m["toolchain"] == "1.94.0" and m["portable_msrv_check_toolchain"] == "1.88.0", "toolchains")
require(m["missing_api_wsl_exit"] == 1 and m["require_ship_native_exit"] == 1, "negative exits")
require(m["seed"] == "0x7592026" and m["modeled_iterations_per_seeded_run"] == 256, "seed")
expected_boundaries = set(["replacement_process_or_mount_argument","public_process_descriptor_pin_or_reset","guard_send_or_sync","old_public_api_behavior_changed","all_threads_proved","initial_host_namespace_proved","mount_content_immutability_proved","atomic_or_continuous_proof","file_opener_credentials_attested","durable_issuer_epoch_or_refusal","real_issuer_enrolled","positive_fixed_production_inspection","production_preparation_allowed","authenticated_original_start","new_helper_ipc_or_live_route","host_mutation_allowed","product_workload_allowed","performance_claim_allowed","qualification_allowed","admission_allowed","full_workspace_verification"])
require(set(m["proof_boundaries"]) == expected_boundaries, "boundary inventory")
require(all(v is False for v in m["proof_boundaries"].values()), "closed boundaries")
expected_tests = ["signed_mount_api_requires_original_signed_reader","signed_mount_context_restoration_refuses_mount_credentials_and_policy","signed_mount_orders_first_errors_and_seeded_refusal_are_exact","signed_mount_fixed_fixture_types_are_not_send_or_sync","signed_mount_owned_roundtrip_and_healthy_drop_keep_original_usable","signed_mount_prior_refusal_never_opens_and_latches_policy","signed_mount_post_open_signed_error_precedes_invalid_mount_and_is_sticky","signed_mount_open_failure_and_original_exit_never_adopt_replacement","signed_mount_substitution_restoration_and_component_refusal_latch_all","signed_mount_revocation_precedes_dead_process_and_bad_mount","signed_mount_original_exit_latches_owned_mount_after_drop","signed_mount_policy_pin_drift_is_not_adopted","signed_mount_independent_concurrent_readers_share_no_latch"]
require([r["name"] for r in m["new_tests"][:-1]] == expected_tests, "composition test inventory")
require(m["new_tests"][-1]["name"] == "namespace_credentials_original_mount_adapter_refuses_either_component_and_exit", "adapter test")


def text(name):
    return (packet / (name + ".log")).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(p), int(i)) for p, i in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline") == [(15, 0), (15, 0), (13, 0)], "baseline totals")
require(totals("windows-tests") == [(15, 0), (15, 0), (0, 0), (13, 0)], "portable totals")
require(totals("first-green") == [(12, 0)], "first green")
require(totals("hardened-linux-tests") == [(296, 1), (29, 0), (9, 0), (2, 0), (3, 0), (5, 0), (13, 0), (13, 0)], "final Linux totals")
require(totals("root-tests-final") == [(99, 0), (13, 0), (23, 0)], "root totals before registration")
require(totals("hardened-root-guards") == [(28, 0)], "hardened diagnostic root guards")
require(totals("registry-final") == [(1, 0)], "final registry")
require(totals("adapter-repeat") == [(1, 0)], "adapter repeat")
for trial in range(1, 4):
    name = f"hardened-repeat-{trial}"
    require(totals(name) == [(13, 0)], f"repeat {trial}")
    require("signed mount mutation seed=0x7592026; mutations=256" in text(name), f"seed {trial}")
    # stderr can split a harness line; totals and unique function labels are checked.
    for row in m["new_tests"][:-1]:
        require(f"::tests::{row['name']} ..." in text(name), f"test {trial}/{row['name']}")
        require(f"::tests::{row['name']} ..." in text("hardened-linux-tests"), f"full test {row['name']}")
for name in ["linux-check", "windows-check", "windows-clippy", "hardened-linux-clippy", "msrv-windows"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"compile/lint {name}")
require("97 passed; 2 failed" in text("root-tests"), "retained root failures")
require("needless_range_loop" in text("linux-clippy"), "retained strict lint")
require("error[E0432]" in text("api-red") and "could not find `mount_binding`" in text("api-red"), "test-first API")
require("no such command: `deny`" in text("dependency-policy"), "missing Linux tool")
require("advisories ok, bans ok, licenses ok, sources ok" in text("dependency-policy-windows"), "native dependency policy")
require("warning[duplicate]" in text("dependency-policy-windows"), "warnings not hidden")
require("not a git repository" in text("workspace-verify"), "Linux worktree limitation")
require("git rev-parse HEAD failed" in text("registry"), "first registry environment failure")
require("ship admission is closed while candidate identity and release qualification are incomplete" in text("ship"), "ship boundary")
require("require-ship native exit=1" in text("ship"), "ship exit")
for row in ["doc-check: OK", "performance-contract-check 0.74: OK", "scoped rustfmt 1.94.0 passed", "Checked 51 markdown files", "HTML book written"]:
    require(row in text("docs-final"), f"docs {row}")
for row in ["all actual qualification contract_inputs: unchanged", "product/cache/native and prior live routes: unchanged", "old signed-opening body: unchanged", "old namespace credential constructors and observations: unchanged"]:
    require(row in text("isolation"), f"isolation {row}")
expected_results = {"windows_baseline_passed":43,"windows_final_passed":43,"windows_linux_target_tests":0,"first_green_passed":12,"linux_final_passed":370,"linux_existing_ignored":1,"new_composition_tests":13,"new_adapter_tests":1,"root_before_registration_passed":135,"hardened_root_guards_passed":28,"final_registry_passed":1,"hardened_serial_repetitions":3}
require(m["results"] == expected_results, "result claims")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
require(m["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256((repository / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()).hexdigest() == pin, "frozen bytes")
print("signed leader mount: 44 exact local captures; all-thread, durable authority and production remain closed")
