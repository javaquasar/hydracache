"""Read-only local namespace-policy evidence, not handoff or production authority."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "6eb187642f0b7d1bd51fc98f49a5eeefcdaef4eb"
BASELINE = "7a38c07cd38fc105067b186235cd49753561dd9a"
SOURCES = set([
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-environ-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-fd-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-control-result-20261015.md"
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
    "docs/testing/performance/0.74/diagnostic-signer-runtime-contract.toml",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-root-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-root-design.md",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_maps.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_runtime_fixture.rs",
    "docs/testing/performance/0.74/diagnostic-signer-rooted-exec-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-rooted-exec-design.md"
])
REQUIRED_CAPTURES = set([
    "signer-namespace-baseline.log",
    "signer-namespace-missing-owner.log",
    "signer-namespace-development.log",
    "signer-namespace-proc-access.log",
    "signer-namespace-retained-development.log",
    "signer-namespace-directory-access.log",
    "signer-namespace-directory-development.log",
    "signer-namespace-control-development.log",
    "signer-namespace-check-development.log",
    "signer-namespace-linux-final.log",
    "signer-namespace-linux-msrv.log",
    "signer-namespace-helper-lint.log",
    "signer-namespace-normal-library.log",
    "signer-namespace-root-contracts.log",
    "signer-namespace-windows-tests.log",
    "signer-namespace-windows-check.log",
    "signer-namespace-windows-lint.log",
    "signer-namespace-windows-msrv.log",
    "signer-namespace-xtask-check.log",
    "signer-namespace-xtask-lint.log",
    "signer-namespace-doc-check.log",
    "signer-namespace-gate-registry.log",
    "signer-namespace-local-contract.log",
    "signer-namespace-require-ship.log",
    "signer-namespace-format.log"
])
RED_CAPTURES = set(["signer-namespace-missing-owner.log","signer-namespace-development.log","signer-namespace-retained-development.log","signer-namespace-directory-development.log","signer-namespace-control-development.log","signer-namespace-check-development.log","signer-namespace-require-ship.log"])
AUXILIARY = {"signer-namespace-packet-windows.log", "signer-namespace-packet-linux.log"}
ADMISSION = set([
    "nested_capability_regain_counterexample_reproduced",
    "continuous_loaded_runtime_closure_proved",
    "mapped_memory_attested",
    "independent_runtime_custody",
    "complete_sandbox",
    "multithreaded_signer_supported",
    "dummy_handoff_implemented",
    "actual_tracer_loss_exitkill_proved",
    "production_issuer_root_or_job_enrolled",
    "actual_private_key_or_token_read",
    "bootstrap_signing_or_worker_start_authority",
    "whole_writer_revocation",
    "completion_or_later_cells_enabled",
    "host_namespace_mount_account_service_or_cgroup_changed",
    "server_entry_wired",
    "qualification_allowed",
    "ship_allowed",
    "performance_claim_allowed"
])


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    assert (PACKET / ".gitattributes").read_bytes() == b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof\n"
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-signer-namespace-20261015"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["preregistration"] == "7cdf6c8461533e0a644c02cb1253ee15d5635ed6"
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
    parent = "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs"
    split = b"""        Self::materialize(images, uid, gid)
    }
    fn materialize(images: &[Vec<u8>; 4], uid: u32, gid: u32) -> io::Result<Self> {
        if uid == 0 {
            return Err(invalid());
        }
"""
    before = committed(BASELINE, parent).replace(b"\r\n", b"\n")
    after = committed(SOURCE, parent).replace(b"\r\n", b"\n")
    assert after.count(split) == 1 and after.replace(split, b"") == before
    q = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(q) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    captures = manifest["captures"]
    names = {c["path"] for c in captures}
    assert len(captures) == len(names)
    assert names == {p.name for p in PACKET.glob("*.log")}
    assert REQUIRED_CAPTURES <= names <= REQUIRED_CAPTURES | AUXILIARY
    for capture in captures:
        assert re.fullmatch(r"signer-namespace-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        expected = 1 if capture["path"] in RED_CAPTURES else 0
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"] == expected
    linux = (PACKET / "signer-namespace-linux-final.log").read_text()
    assert list(map(int, re.findall(r"test result: ok\. (\d+) passed; 0 failed", linux))) == [13,13,13,11,7,10,9,11,8,18,12]
    for marker in ["post_install_native_namespace_root_mount_denials=15 original_context_unchanged=true handoff=false",
                   "post_ND0_original_environ_and_fd_inventory=retained fresh_open=EACCES live_context_checked=true",
                   "old_policy_nested_userns=kernel_EPERM regain_not_reproduced=true control_refused=true host_namespace_changed=false",
                   "namespace_candidate_x32_i386=SIGSYS same_child_original_exec_install=true",
                   "namespace_original_SIGKILL_drop=initial,rooted,first_exec,installed tracer_loss_proved=false",
                   "namespace_probe_seed=0x07403b2b fresh_probes=8 independent_owners=4 multithreaded_signer=false"]:
        assert linux.count(marker) == 3, marker
    digest = "bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081"
    old_digest = "e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37"
    assert manifest["fixture_helper_sha256"] == digest
    assert manifest["old_fixture_helper_sha256"] == old_digest
    assert linux.count("fixture_namespace_helper_sha256=" + digest) == 3
    assert "fixture_gnu_helper_sha256=" + old_digest in linux
    baseline = (PACKET / "signer-namespace-baseline.log").read_text()
    for count in (11,8):
        assert f"test result: ok. {count} passed; 0 failed" in baseline
    for name, count in [("linux-msrv",13),("windows-tests",9),("root-contracts",143)]:
        assert f"test result: ok. {count} passed; 0 failed" in (PACKET / f"signer-namespace-{name}.log").read_text()
    for name, marker in [
        ("normal-library","normal-library namespace fixture symbols absent"),
        ("gate-registry","169 discovered gates, all registered"),
        ("local-contract","local, non-promotable"),
        ("require-ship","ship admission is closed"),
        ("missing-owner","compiler unexpectedly panicked"),
        ("development","FAILED. 4 passed; 5 failed"),
        ("retained-development","FAILED. 5 passed; 6 failed"),
        ("directory-development","FAILED. 12 passed; 1 failed"),
        ("control-development","ControlUnshare exit refused: Exit"),
        ("check-development","manual_is_multiple_of"),
        ("proc-access","post_ND0_text_environ=false"),
        ("directory-access","post_ND0_directory_fd=false"),
    ]:
        assert marker in (PACKET / f"signer-namespace-{name}.log").read_text(), name
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified namespace policy evidence, {len(captures)} captures; regain not reproduced, handoff/custody/start closed.")


def verify_committed(commit):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    names = {"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(c["path"] for c in manifest["captures"])
    relative = PACKET.relative_to(ROOT)
    for name in sorted(names):
        assert (PACKET / name).read_bytes() == committed(commit,(relative / name).as_posix()), name
    print(f"Verified committed packet bytes at {commit}; no Git EOL normalization.")


if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])
