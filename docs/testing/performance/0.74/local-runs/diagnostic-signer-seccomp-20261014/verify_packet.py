"""Read-only owned seccomp evidence; never signer/runtime admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "4f8d20eeb2f630b65669da78d5551ba3bded6d50"
BASELINE = "06481accc61678025de73affe8364c43ce9f6513"
SOURCES = {
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture_tests.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/testing/performance/0.74/diagnostic-signer-seccomp-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-seccomp-design.md",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-seccomp-plan-20261014.md",
}
UNCHANGED = {
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
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-first-exec-contract.toml",
}
RED_CAPTURES = {
    "signer-seccomp-missing-owner.log",
    "signer-seccomp-check-development.log",
    "signer-seccomp-abi-investigation.log",
    "signer-seccomp-abi-exit-observation.log",
    "signer-seccomp-hardened-development.log",
    "signer-seccomp-registry.log",
    "signer-seccomp-require-ship.log",
}
ADMISSION = {
    "first_exec_and_installation_composed",
    "multithreaded_production_signer_supported",
    "atomic_proof_against_external_memory_writer",
    "runtime_or_memory_attested",
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
    "performance_claim_allowed",
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-signer-seccomp-20261014"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    assert {entry["path"] for entry in manifest["source_files"]} == SOURCES
    assert {entry["path"] for entry in manifest["unchanged_files"]} == UNCHANGED
    assert manifest["preregistration"] == "96da191195c7a7bb1a8941c76035c71e4e28486d"
    assert manifest["development_amendment"] == "34584c4dc1bd136b62c366691af8cc6a160d1844"
    for group, commit in [("source_files", SOURCE), ("unchanged_files", BASELINE)]:
        entries = manifest[group]
        assert entries and len(entries) == len({entry["path"] for entry in entries})
        for entry in entries:
            relative = Path(entry["path"])
            assert not relative.anchor and ".." not in relative.parts
            assert (ROOT / relative).resolve().is_relative_to(ROOT)
            raw = (ROOT / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
            assert raw == committed(commit, entry["path"]).replace(b"\r\n", b"\n"), entry["path"]
    assert {"Cargo.lock", "tools/long-run-supervisor-074/Cargo.toml",
            "tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs"}.issubset(
        {entry["path"] for entry in manifest["unchanged_files"]})
    raw = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(raw) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    captures = manifest["captures"]
    assert len(captures) == len({capture["path"] for capture in captures})
    assert {capture["path"] for capture in captures} == {p.name for p in PACKET.glob("*.log")}
    for capture in captures:
        assert re.fullmatch(r"signer-seccomp-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        expected = 1 if capture["path"] in RED_CAPTURES else 0
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"] == expected
    assert RED_CAPTURES.issubset({capture["path"] for capture in captures})
    linux = (PACKET / "signer-seccomp-linux-final.log").read_text()
    for marker in ["test result: ok. 8 passed; 0 failed",
                   "kernel_install=exact_entry_memory_exit_zero original_fork_helper=true handoff_or_runtime_admission=false",
                   "actual_x32_number_bit_and_i386_int80=SIGSYS no_exec_or_handoff=true",
                   "policy_instructions=34 serialized_bytes=272 bounded_syscall_numbers=1024 single_thread_fixture_only=true",
                   "seed=0x07403b26 fresh_kernel_probes=16 independent_tracer_threads=4 multithreaded_signer_supported=false"]:
        assert linux.count(marker) == 3, marker
    for count in (11, 18, 12):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    assert "error[E0433]" in (PACKET / "signer-seccomp-missing-owner.log").read_text()
    assert "7 passed; 1 failed" in (PACKET / "signer-seccomp-check-development.log").read_text()
    assert "status=0x857f" in (PACKET / "signer-seccomp-abi-investigation.log").read_text()
    assert "1073741883" in (PACKET / "signer-seccomp-abi-exit-observation.log").read_text()
    assert "3 passed; 5 failed" in (PACKET / "signer-seccomp-hardened-development.log").read_text()
    assert "normal-library seccomp fixture symbols absent" in (PACKET / "signer-seccomp-normal-library.log").read_text()
    assert "test result: ok. 138 passed; 0 failed" in (PACKET / "signer-seccomp-root-contracts.log").read_text()
    assert "ship admission is closed" in (PACKET / "signer-seccomp-require-ship.log").read_text()
    assert "169 discovered gates, all registered" in (PACKET / "signer-seccomp-gate-registry.log").read_text()
    assert "test result: ok. 9 passed; 0 failed" in (PACKET / "signer-seccomp-windows-tests.log").read_text()
    formatted = json.loads((PACKET / "signer-seccomp-format-result.log").read_bytes())
    assert type(formatted["observed_exit"]) is int and formatted["observed_exit"] == 0
    assert formatted["source_commit"] == SOURCE
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified owned seccomp installation and denial evidence, {len(captures)} captures; runtime/handoff/start closed.")


if __name__ == "__main__":
    main()
