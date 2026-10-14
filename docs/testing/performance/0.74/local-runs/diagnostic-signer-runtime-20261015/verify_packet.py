"""Read-only bounded original ELF input evidence; no loaded-runtime or launch admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "acffe2c544d2f501993b90d8ff9cc6ae1dafad71"
BASELINE = "75c2d2d20db386b05190e67d0cee2b016ee106c9"
SOURCES = set([
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_elf.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_fixture_tests.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-runtime-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-runtime-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-runtime-plan-20261015.md"
])
UNCHANGED = set([
    "Cargo.toml",
    "Cargo.lock",
    "tools/long-run-supervisor-074/Cargo.toml",
    "tools/long-run-supervisor-074/src/diagnostic_process.rs",
    "tools/long-run-supervisor-074/src/diagnostic_process_io.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_build.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_bundle.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_executable.rs",
    "tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs",
    "tools/long-run-supervisor-074/src/diagnostic_controller_producer_linux.rs",
    "tools/long-run-supervisor-074/src/bin/diagnostic_controller.rs",
    "tools/long-run-supervisor-074/src/server.rs",
    "docs/testing/performance/0.74/qualification-manifest.toml",
    "docs/plans/releases.toml",
    "docs/testing/performance/0.74/diagnostic-worker-issuer-public.json",
    "docs/testing/performance/0.74/diagnostic-builder-public-policy.json",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-first-exec-contract.toml",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-seccomp-contract.toml",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_helper.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-joined-contract.toml"
])
RED_CAPTURES = set([
    "signer-runtime-missing-owner.log",
    "signer-runtime-development.log",
    "signer-runtime-regression-command-development.log",
    "signer-runtime-require-ship.log"
])
REQUIRED_CAPTURES = set([
    "signer-runtime-packet-windows-initial.log",
    "signer-runtime-baseline.log",
    "signer-runtime-missing-owner.log",
    "signer-runtime-development.log",
    "signer-runtime-check-development.log",
    "signer-runtime-bounds-development.log",
    "signer-runtime-regression-command-development.log",
    "signer-runtime-linux-final.log",
    "signer-runtime-linux-msrv.log",
    "signer-runtime-normal-library.log",
    "signer-runtime-windows-tests.log",
    "signer-runtime-windows-check.log",
    "signer-runtime-windows-lint.log",
    "signer-runtime-windows-msrv.log",
    "signer-runtime-root-contracts.log",
    "signer-runtime-xtask-check.log",
    "signer-runtime-xtask-lint.log",
    "signer-runtime-doc-check.log",
    "signer-runtime-gate-registry.log",
    "signer-runtime-local-contract.log",
    "signer-runtime-require-ship.log",
    "signer-runtime-format-result.log"
])
ADMISSION = set([
    "complete_elf_relocation_symbol_version_validation",
    "selected_loader_root_enforced",
    "loaded_runtime_closure_proved",
    "independent_runtime_custody",
    "mapped_memory_attested",
    "dummy_handoff_implemented",
    "actual_tracer_loss_exitkill_proved",
    "production_issuer_root_or_job_enrolled",
    "actual_private_key_or_token_read",
    "bootstrap_signing_or_worker_start_authority",
    "whole_writer_revocation",
    "completion_or_later_cells_enabled",
    "host_service_or_cgroup_mutated",
    "protected_job_dispatched",
    "server_entry_wired",
    "qualification_allowed",
    "ship_allowed",
    "performance_claim_allowed"
])
AUXILIARY = {"signer-runtime-packet-windows.log", "signer-runtime-packet-linux.log"}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-signer-runtime-20261015"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["preregistration"] == "47ff9e3fd152cf1230ed20777e6fb3477a73ee68"
    assert manifest["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    for group, commit, paths in [("source_files", SOURCE, SOURCES),
                                 ("unchanged_files", BASELINE, UNCHANGED)]:
        entries = manifest[group]
        assert len(entries) == len(paths) and {e["path"] for e in entries} == paths
        for entry in entries:
            relative = Path(entry["path"])
            assert not relative.anchor and ".." not in relative.parts
            assert (ROOT / relative).resolve().is_relative_to(ROOT)
            raw = (ROOT / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
            assert raw == committed(commit, entry["path"]).replace(b"\r\n", b"\n"), entry["path"]
    shared = "tools/long-run-supervisor-074/src/diagnostic_signer_joined_fixture.rs"
    raw = committed(SOURCE, shared).replace(b"pub(super) fn compiled_helper()", b"fn compiled_helper()")
    assert raw == committed(BASELINE, shared)
    q = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(q) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    captures = manifest["captures"]
    names = {c["path"] for c in captures}
    assert len(captures) == len(names)
    assert names == {p.name for p in PACKET.glob("*.log")}
    assert REQUIRED_CAPTURES <= names <= REQUIRED_CAPTURES | AUXILIARY
    for capture in captures:
        assert re.fullmatch(r"signer-runtime-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        expected = 1 if capture["path"] in RED_CAPTURES else 0
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"] == expected
    linux = (PACKET / "signer-runtime-linux-final.log").read_text()
    for marker in ["test result: ok. 10 passed; 0 failed",
                   "ELF_string_table=virtual_address_mapped_to_file synthetic_image_not_executed=true",
                   "original_runtime_files=4 declared_elf_dependencies=checked launch_root_enforced=false custody_or_handoff=false",
                   "seed=0x07403b28 parser_mutations=128 independent_runtime_fixtures=4 runtime_or_launch_authority=false"]:
        assert linux.count(marker) == 3, marker
    hashes = {
        "signer": "e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37",
        "ld-linux-x86-64.so.2": "c20a2dc8917c755f02b94049356320fe1f62ac7d9f8994731f807d9df39302da",
        "libc.so.6": "3a15d66867d83762c7f2f1e37359cb8f6c5743edb369c65285cb0b1c4f7498bf",
        "libgcc_s.so.1": "d93224d2b0dab4247598be683adca02f5cf00586f99c187579cd7e92058fb7cb",
    }
    assert manifest["fixture_inputs_sha256"] == hashes
    for name, digest in hashes.items():
        assert linux.count("runtime_input=" + name + " sha256=" + digest) == 3
    for count in (9, 11, 8, 18, 12):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    assert "test result: ok. 10 passed; 0 failed" in (PACKET / "signer-runtime-linux-msrv.log").read_text()
    assert "test result: ok. 9 passed; 0 failed" in (PACKET / "signer-runtime-windows-tests.log").read_text()
    assert "test result: ok. 140 passed; 0 failed" in (PACKET / "signer-runtime-root-contracts.log").read_text()
    assert "normal-library runtime fixture symbols absent" in (PACKET / "signer-runtime-normal-library.log").read_text()
    assert "169 discovered gates, all registered" in (PACKET / "signer-runtime-gate-registry.log").read_text()
    assert "ship admission is closed" in (PACKET / "signer-runtime-require-ship.log").read_text()
    assert "error[E0433]" in (PACKET / "signer-runtime-missing-owner.log").read_text()
    ice = (PACKET / "signer-runtime-development.log").read_text()
    assert "slice index starts at 5 but ends at 4" in ice and "check_mod_deathness" in ice
    assert "no test target named" in (PACKET / "signer-runtime-regression-command-development.log").read_text()
    formatted = json.loads((PACKET / "signer-runtime-format-result.log").read_bytes())
    assert formatted["source_commit"] == SOURCE
    assert type(formatted["observed_exit"]) is int and formatted["observed_exit"] == 0
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified bounded original runtime input evidence, {len(captures)} captures; loaded-root/custody/handoff/start closed.")


if __name__ == "__main__":
    main()
