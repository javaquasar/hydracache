"""Read-only owned nonroot child root evidence; no signer exec, runtime custody or authority."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "28d98596c8e6126a9df3ab1009a875e980d07eb7"
BASELINE = "aa54bfd9b64f1ba87bd090f6b17c55e4da8186a8"
SOURCES = set([
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_fixture.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-root-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-root-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-root-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-root-reuse-amendment-20261015.md"
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
    "docs/testing/performance/0.74/diagnostic-signer-joined-contract.toml",
    "tools/long-run-supervisor-074/src/diagnostic_signer_joined_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_elf.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-runtime-contract.toml"
])
RED_CAPTURES = set([
    "signer-root-missing-owner.log",
    "signer-root-check-development.log",
    "signer-root-regression-command-development.log",
    "signer-root-require-ship.log"
])
REQUIRED_CAPTURES = set([
    "signer-root-baseline.log",
    "signer-root-feasibility.log",
    "signer-root-missing-owner.log",
    "signer-root-development.log",
    "signer-root-check-development.log",
    "signer-root-regression-command-development.log",
    "signer-root-reuse-development.log",
    "signer-root-linux-final.log",
    "signer-root-linux-msrv.log",
    "signer-root-normal-library.log",
    "signer-root-windows-tests.log",
    "signer-root-windows-check.log",
    "signer-root-windows-lint.log",
    "signer-root-windows-msrv.log",
    "signer-root-root-contracts.log",
    "signer-root-xtask-check.log",
    "signer-root-xtask-lint.log",
    "signer-root-doc-check.log",
    "signer-root-gate-registry.log",
    "signer-root-local-contract.log",
    "signer-root-require-ship.log",
    "signer-root-format-result.log"
])
ADMISSION = set([
    "mount_or_host_namespace_or_sysctl_changed",
    "signer_exec_in_selected_root",
    "selected_loader_root_enforced",
    "loaded_runtime_closure_proved",
    "post_launch_namespace_escape_policy_installed",
    "independent_runtime_custody",
    "mapped_memory_attested",
    "complete_sandbox",
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
AUXILIARY = {"signer-root-packet-windows.log", "signer-root-packet-linux.log"}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-signer-root-20261015"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["preregistration"] == "d7bcbe61e9a20f3440c4d44422bed3721ee60bd3"
    assert manifest["reuse_amendment"] == "96466ed3b81bcce47119981116d60e1a53cdfd81"
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
    q = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(q) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    captures = manifest["captures"]
    names = {c["path"] for c in captures}
    assert len(captures) == len(names)
    assert names == {p.name for p in PACKET.glob("*.log")}
    assert REQUIRED_CAPTURES <= names <= REQUIRED_CAPTURES | AUXILIARY
    for capture in captures:
        assert re.fullmatch(r"signer-root-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        expected = 1 if capture["path"] in RED_CAPTURES else 0
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"] == expected
    linux = (PACKET / "signer-root-linux-final.log").read_text()
    for marker in ["test result: ok. 7 passed; 0 failed",
                   "owned_nonroot_child_root=selected_inode namespace=child_only caps=zero host_paths=absent signer_exec=false custody=false",
                   "owned_root_SIGKILL=initial,rooted original_child_reaped=true signer_exec=false tracer_loss_proved=false",
                   "seed=0x07403b29 rooted_path_probes=8 independent_owners=4 parent_root_changed=false signer_exec=false"]:
        assert linux.count(marker) == 3, marker
    digest = "e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37"
    assert manifest["fixture_helper_sha256"] == digest
    assert linux.count("fixture_gnu_helper_sha256=" + digest) == 3
    for count in (10, 9, 11, 8, 18, 12):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    assert "test result: ok. 10 passed; 0 failed" in (PACKET / "signer-root-baseline.log").read_text()
    assert "uid=1000(artur)" in (PACKET / "signer-root-feasibility.log").read_text()
    assert "test result: ok. 7 passed; 0 failed" in (PACKET / "signer-root-linux-msrv.log").read_text()
    assert "test result: ok. 9 passed; 0 failed" in (PACKET / "signer-root-windows-tests.log").read_text()
    assert "test result: ok. 141 passed; 0 failed" in (PACKET / "signer-root-root-contracts.log").read_text()
    assert "normal-library child-root fixture symbols absent" in (PACKET / "signer-root-normal-library.log").read_text()
    assert "169 discovered gates, all registered" in (PACKET / "signer-root-gate-registry.log").read_text()
    assert "ship admission is closed" in (PACKET / "signer-root-require-ship.log").read_text()
    missing = (PACKET / "signer-root-missing-owner.log").read_text()
    assert "error[E0433]" in missing and "compiler unexpectedly panicked" in missing
    assert "duplicate_mod" in (PACKET / "signer-root-check-development.log").read_text()
    interrupted = (PACKET / "signer-root-regression-command-development.log").read_text()
    assert "running 781 tests" in interrupted
    assert "test result: ok. 781 passed" not in interrupted
    formatted = json.loads((PACKET / "signer-root-format-result.log").read_bytes())
    assert formatted["source_commit"] == SOURCE
    assert type(formatted["observed_exit"]) is int and formatted["observed_exit"] == 0
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified owned nonroot child root evidence, {len(captures)} captures; signer-exec/custody/handoff/start closed.")


if __name__ == "__main__":
    main()
