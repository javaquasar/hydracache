"""Read-only local authority fixture evidence; no issuer enrollment or start."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path


def require(ok, message):
    if not ok:
        raise SystemExit(message)


packet = Path(__file__).resolve().parent
repository = packet.parents[5]
manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
require(manifest["schema_version"] == "diagnostic-authority-journal-local-evidence-074-v1", "schema")
require(manifest["evidence_class"] == "working-tree-local-safety-not-release-qualification", "class")
require(manifest["state"] == "retained-core-and-temporary-fixture-not-production-authority", "state")
require(manifest["baseline_source"] == "27e5f0bd7c7596a8e54d9720cdeebae06ea1d268", "baseline")
require(manifest["preregistration_source"] == "08fbf0d1f1a7c1c8d1a359f23f547e3a7e653f46", "preregistration")
require(manifest["implementation_source"] == "5e826dd7ed5998564c0c4581534df9fe027febca", "implementation")
require(manifest["reviewed_source"] == "e777f5a6e5b16c724d63d6c139761d5d05e44e08", "reviewed source")
require(manifest["toolchain"] == "1.94.0" and manifest["msrv"] == "1.88.0", "toolchain")
require(manifest["limits"] == {"generations": 128, "record_bytes": 2048,
                             "marker_bytes": 65, "named_entries": 256}, "bounds")
require(manifest["modeled_seed"] == "0x7642026" and manifest["modeled_mutations"] == 256, "model")
expected_boundaries = {
    "whole_host_snapshot_rollback_detection", "root_or_owner_tampering_detection",
    "historical_policy_signature_attestation", "physical_power_loss_proved",
    "regular_file_io_deadline_proved", "root_owned_production_backend",
    "real_issuer_enrolled", "signed_worker_durable_composition",
    "automatic_durable_refusal_after_inspection_error", "production_preparation_or_start",
    "watchdog_or_writer_revocation", "host_mutation_allowed", "new_helper_ipc_or_live_route",
    "product_workload_allowed", "performance_claim_allowed", "qualification_or_admission_allowed",
    "full_workspace_run_for_this_slice", "qualification_input_digests_match"}
require(set(manifest["proof_boundaries"]) == expected_boundaries, "boundary inventory")
require(all(v is False for v in manifest["proof_boundaries"].values()), "closed boundaries")
require(len(manifest["logs"]) == len(set(manifest["logs"])), "capture inventory")
require({p.name for p in packet.glob("*.log")} == set(manifest["logs"]), "capture completeness")
for name, binding in manifest["logs"].items():
    require(Path(name).name == name and name.endswith(".log"), "local log path")
    raw = (packet / name).read_bytes()
    require(len(raw) == binding["bytes"], f"size {name}")
    require(hashlib.sha256(raw).hexdigest() == binding["sha256"], f"hash {name}")


def text(name):
    return (packet / name).read_text(encoding="utf-8-sig")


def totals(name):
    return [(int(p), int(i)) for p, i in re.findall(
        r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]


require(totals("baseline.log") == [(13, 0)], "unchanged policy baseline")
require("error[E0432]" in text("api-red.log") and "authority_registry" in text("api-red.log"), "test-first API")
require("dead-code" in text("windows-clippy.log"), "retained Windows lint negative")
require("kind: WouldBlock" in text("linux-reviewed-tests.log") and
        "test result: FAILED. 330 passed; 1 failed; 1 ignored" in text("linux-reviewed-tests.log"),
        "retained parallel reopen failure")
require("kind: WouldBlock" in text("inherited-description-red.log"), "deterministic lifetime red")
require(totals("inherited-description-green.log") == [(1, 0)], "lifetime fix")
require(totals("linux-final-tests.log") == [
    (332, 1), (29, 0), (9, 0), (2, 0), (2, 0), (3, 0), (5, 0), (13, 0), (14, 0)], "scoped Linux")
require(sum(p for p, _ in totals("windows-reviewed-tests.log")) == 195, "portable Windows")
require(totals("root-final-tests.log") == [(102, 0), (13, 0), (23, 0)], "root gates")
require(totals("packet-registry.log") == [(1, 0)], "post-packet registry")
require(manifest["results"] == {
    "linux_scoped_passed": 409, "linux_existing_ignored": 1,
    "windows_package_passed": 195, "new_unit_tests": 14, "new_portable_core_tests": 5,
    "new_linux_fixture_tests": 9, "new_api_tests": 1, "serial_repetitions": 3,
    "root_contract_passed": 102, "root_evidence_passed": 13, "governance_passed": 23}, "results")
require(len(manifest["new_unit_test_names"]) == 14 and
        len(set(manifest["new_unit_test_names"])) == 14, "test names")
for trial in range(1, 4):
    name = f"final-repeat-{trial}.log"
    require(totals(name) == [(14, 0)], f"serial repeat {trial}")
    require("authority journal seed=0x7642026; mutations=256" in text(name), "seed")
    for test in manifest["new_unit_test_names"]:
        require(f"::tests::{test} ..." in text(name), f"test {trial}/{test}")
for name in ["linux-reviewed-check.log", "linux-final-clippy.log", "linux-final-msrv.log", "linux-rustdoc.log",
             "windows-reviewed-check.log", "windows-reviewed-clippy.log", "windows-msrv.log",
             "root-reviewed-clippy.log"]:
    require("Finished `dev` profile" in text(name) and "error:" not in text(name), f"check {name}")
require(text("format-reviewed.log") == "", "format")
require("doc-check: OK" in text("docs-final.log"), "docs")
require("performance-contract-check 0.74: OK" in text("contract-final.log"), "local contract")
require("HTML book written" in text("book-final.log"), "book")
require("HTML book written" in text("book-postpacket.log"), "post-packet book")
require("doc-check: OK" in text("docs-postpacket.log"), "post-packet docs")
require("performance-contract-check 0.74: OK" in text("contract-postpacket.log"), "post-packet contract")
require("local Markdown file targets resolved=" in text("links-final.log"), "file links")
require("ship admission is closed while candidate identity and release qualification are incomplete"
        in text("ship-refusal.log"), "closed ship")
require("(exit code: 1)" in text("ship-refusal.log"), "native ship refusal")
require("frozen inputs and prior product/runtime routes: unchanged" in text("isolation-reviewed.log"), "isolation")
require("contract input digest mismatch" in text("qualification-input-red.log"), "inherited qualification refusal")
pin = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
qualification = repository / "docs/testing/performance/0.74/qualification-manifest.toml"
require(manifest["qualification_manifest_sha256"] == pin, "qualification pin")
require(hashlib.sha256(qualification.read_bytes()).hexdigest() == pin, "frozen manifest bytes")
inputs = tomllib.loads(qualification.read_text(encoding="utf-8"))["contract_inputs"]
require(len(inputs) == 8, "frozen input inventory")
mismatches = []
for binding in inputs:
    committed = subprocess.check_output(["git", "show", manifest["baseline_source"] + ":" + binding["path"]],
                                        cwd=repository)
    current = (repository / binding["path"]).read_bytes()
    require(current.replace(b"\r\n", b"\n") == committed.replace(b"\r\n", b"\n"),
            f"baseline input unchanged {binding['path']}")
    actual = hashlib.sha256(committed).hexdigest()
    if actual != binding["sha256"]:
        mismatches.append({"path": binding["path"], "declared_sha256": binding["sha256"],
                           "baseline_committed_sha256": actual})
require(mismatches == manifest["inherited_qualification_mismatches"], "inherited mismatch identity")
require(mismatches == [{"path": "docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md",
                       "declared_sha256": "6b443b0c766de5dcd131bdf7dcf46e6e8d256063f1fab1685034ae86427aa44d",
                       "baseline_committed_sha256": "cff351817e9d83c3ec72b587e4ad7580bb8b882c8fab154f88c31b63aebb9828"}],
        "known baseline qualification blocker")
for field in ["baseline", "implementation", "reviewed"]:
    tree = subprocess.check_output(["git", "rev-parse", manifest[field + "_source"] + "^{tree}"],
                                   cwd=repository, text=True).strip()
    require(tree == manifest[field + "_tree"], f"{field} tree")
require(subprocess.run(["git", "merge-base", "--is-ancestor", manifest["preregistration_source"],
                        manifest["implementation_source"]], cwd=repository, check=False).returncode == 0,
        "preregistered ancestry")
print(f"authority journal: {len(manifest['logs'])} exact local captures; production authority remains closed")
