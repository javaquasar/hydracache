"""Read-only owned fresh exec and filter installation evidence; never signer/runtime admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "bdaa20a5f50645ddb609b7404bb6d94889dac215"
BASELINE = "3b5df398037234d533059c53a5b5b7c4dc650525"
SOURCES = {
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_helper.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/testing/performance/0.74/diagnostic-signer-joined-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-joined-design.md",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-joined-plan-20261014.md",
    "docs/testing/performance/0.74/diagnostic-signer-joined-writer-amendment-20261015.md",
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
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-first-exec-contract.toml",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-seccomp-contract.toml",
}
RED_CAPTURES = {
    "signer-joined-missing-owner.log",
    "signer-joined-development.log",
    "signer-joined-check-development.log",
    "signer-joined-integration-development.log",
    "signer-joined-concurrent-exec-red.log",
    "signer-joined-writer-regression-red.log",
    "signer-joined-require-ship.log",
}
ADMISSION = {
    "post_exec_addresses_preselected_before_fork",
    "independent_runtime_custody",
    "mapped_memory_attested",
    "atomic_proof_against_external_memory_writer",
    "multithreaded_production_signer_supported",
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
    assert manifest["packet_id"] == "diagnostic-signer-joined-20261015"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    assert {entry["path"] for entry in manifest["source_files"]} == SOURCES
    assert {entry["path"] for entry in manifest["unchanged_files"]} == UNCHANGED
    assert manifest["preregistration"] == "1e436922413fb7c6daa7eacea9f1721488fe7d82"
    assert manifest["development_amendment"] == "ff4fe07e2bf78588389f8b1eb78a649d53ec9eb5"
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
        assert re.fullmatch(r"signer-joined-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        expected = 1 if capture["path"] in RED_CAPTURES else 0
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"] == expected
    assert RED_CAPTURES.issubset({capture["path"] for capture in captures})
    assert set(["signer-joined-baseline.log","signer-joined-check-development.log","signer-joined-compiler-path-development.log","signer-joined-concurrent-exec-red.log","signer-joined-development.log","signer-joined-doc-check.log","signer-joined-gate-registry.log","signer-joined-helper-lint.log","signer-joined-integration-development.log","signer-joined-linux-final.log","signer-joined-linux-msrv.log","signer-joined-local-contract.log","signer-joined-missing-owner.log","signer-joined-normal-library.log","signer-joined-require-ship.log","signer-joined-root-contracts.log","signer-joined-root-development.log","signer-joined-windows-check.log","signer-joined-windows-lint.log","signer-joined-windows-msrv.log","signer-joined-windows-tests.log","signer-joined-writer-fix-development.log","signer-joined-writer-regression-red.log","signer-joined-xtask-check.log","signer-joined-xtask-lint.log","signer-joined-format-result.log","signer-joined-helper-lint-result.log"]) <= {capture["path"] for capture in captures}
    for path in ["tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture.rs",
                 "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture.rs"]:
        current = committed(SOURCE, path).replace(b"\r\n", b"\n").replace(b"pub(super) ", b"")
        assert current == committed(BASELINE, path).replace(b"\r\n", b"\n"), path
    linux = (PACKET / "signer-joined-linux-final.log").read_text()
    for marker in ["test result: ok. 9 passed; 0 failed",
                   "fresh_exec_and_exact_install=same_owned_child credential_handoff=false runtime_custody=false",
                   "pre_install_exec=entry_refused original_child_reaped=true new_image_not_dispatched=true",
                   "owned_child_SIGKILL_boundaries=first_exec,installed no_handoff=true tracer_loss_proved=false",
                   "inherited_writer_counterexample=ETXTBSY whole_writer_revocation=false",
                   "image_preparation_writer=isolated_copy_child stopped_parent_snapshot_does_not_block_exec=true",
                   "seed=0x07403b27 fresh_exec_install_probes=8 independent_tracer_threads=4 production_multithreading=false"]:
        assert linux.count(marker) == 3, marker
    helper_sha = "e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37"
    assert manifest["fixture_helper_sha256"] == helper_sha
    assert linux.count("fixture_gnu_helper_sha256=" + helper_sha) == 3
    for count in (8, 11, 18, 12):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    assert "error[E0433]" in (PACKET / "signer-joined-missing-owner.log").read_text()
    assert "error[E0277]" in (PACKET / "signer-joined-development.log").read_text()
    assert "0 passed; 8 failed" in (PACKET / "signer-joined-integration-development.log").read_text()
    red = (PACKET / "signer-joined-concurrent-exec-red.log").read_text()
    assert "7 passed; 1 failed" in red and "0x4800" in red
    red = (PACKET / "signer-joined-writer-regression-red.log").read_text()
    assert "0 passed; 1 failed" in red and "0x9a00" in red
    assert "normal-library joined fixture symbols absent" in (PACKET / "signer-joined-normal-library.log").read_text()
    assert "test result: ok. 139 passed; 0 failed" in (PACKET / "signer-joined-root-contracts.log").read_text()
    assert "ship admission is closed" in (PACKET / "signer-joined-require-ship.log").read_text()
    assert "169 discovered gates, all registered" in (PACKET / "signer-joined-gate-registry.log").read_text()
    assert "test result: ok. 9 passed; 0 failed" in (PACKET / "signer-joined-windows-tests.log").read_text()
    linted = json.loads((PACKET / "signer-joined-helper-lint-result.log").read_bytes())
    assert linted["source_commit"] == SOURCE
    assert type(linted["observed_exit"]) is int and linted["observed_exit"] == 0
    assert (PACKET / "signer-joined-helper-lint.log").read_bytes() == b""
    formatted = json.loads((PACKET / "signer-joined-format-result.log").read_bytes())
    assert type(formatted["observed_exit"]) is int and formatted["observed_exit"] == 0
    assert formatted["source_commit"] == SOURCE
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified owned fresh exec and exact installation evidence, {len(captures)} captures; runtime/handoff/start closed.")


if __name__ == "__main__":
    main()
