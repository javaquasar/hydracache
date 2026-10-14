"""Read-only selected-root original-exec evidence, not production admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "48cf97adcc34d8428efafeb60cb743d0f3096036"
BASELINE = "2072b30ef4084cb910712450dc87b0529b3f80fa"
SOURCES = set([
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_maps.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-rooted-exec-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-rooted-exec-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-rooted-exec-plan-20261015.md"
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
    "docs/testing/performance/0.74/diagnostic-signer-root-design.md"
])
REQUIRED_CAPTURES = set([
    "rooted-exec-committed-packet-development.log",
    "rooted-exec-baseline.log",
    "rooted-exec-development.log",
    "rooted-exec-doc-check.log",
    "rooted-exec-focused-development.log",
    "rooted-exec-format.log",
    "rooted-exec-gate-registry.log",
    "rooted-exec-linux-check-lint.log",
    "rooted-exec-linux-corrected.log",
    "rooted-exec-linux-final.log",
    "rooted-exec-linux-msrv.log",
    "rooted-exec-local-contract.log",
    "rooted-exec-missing-owner.log",
    "rooted-exec-normal-library.log",
    "rooted-exec-positive-development.log",
    "rooted-exec-require-ship.log",
    "rooted-exec-regressions.log",
    "rooted-exec-root-contracts.log",
    "rooted-exec-windows-check.log",
    "rooted-exec-windows-lint.log",
    "rooted-exec-windows-msrv.log",
    "rooted-exec-windows-tests.log",
    "rooted-exec-xtask-check.log",
    "rooted-exec-xtask-lint.log"
])
RED_CAPTURES = {"rooted-exec-missing-owner.log", "rooted-exec-development.log",
                "rooted-exec-linux-check-lint.log", "rooted-exec-require-ship.log",
                "rooted-exec-committed-packet-development.log"}
AUXILIARY = {"rooted-exec-packet-windows.log", "rooted-exec-packet-linux.log"}
ADMISSION = set([
    "continuous_loaded_runtime_closure_proved",
    "mapped_memory_or_map_files_attested",
    "post_install_root_namespace_mutation_policy_installed",
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
    "mount_or_host_namespace_or_sysctl_changed",
    "host_service_or_cgroup_mutated",
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
    assert manifest["packet_id"] == "diagnostic-signer-rooted-exec-20261015"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["preregistration"] == "ae42954bdca491b1bf6a39600c7821e2ef2cf849"
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
    hook = b'#[path = "diagnostic_signer_rooted_exec_fixture.rs"]\nmod rooted_exec;\n'
    before = committed(BASELINE, parent).replace(b"\r\n", b"\n")
    after = committed(SOURCE, parent).replace(b"\r\n", b"\n")
    assert after.count(hook) == 1 and after.replace(hook, b"") == before
    q = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(q) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    captures = manifest["captures"]
    names = {c["path"] for c in captures}
    assert len(captures) == len(names)
    assert names == {p.name for p in PACKET.glob("*.log")}
    assert REQUIRED_CAPTURES <= names <= REQUIRED_CAPTURES | AUXILIARY
    for capture in captures:
        assert re.fullmatch(r"rooted-exec-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        expected = 1 if capture["path"] in RED_CAPTURES else 0
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"] == expected
    linux = (PACKET / "rooted-exec-linux-corrected.log").read_text()
    for marker in ["test result: ok. 11 passed; 0 failed",
                   "rooted_original_exec_and_exact_install=same_child selected_VMA_file_ids=4 memory_attested=false handoff=false",
                   "owned_rooted_exec_SIGKILL=initial,rooted,first_exec,installed original_reaped=true tracer_loss_proved=false",
                   "rooted_exec_seed=0x07403b2a fresh_execs=8 independent_owners=4 parent_root_changed=false actual_multithread_signer=false"]:
        assert linux.count(marker) == 3, marker
    digest = "e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37"
    assert manifest["fixture_helper_sha256"] == digest
    assert linux.count("fixture_gnu_helper_sha256=" + digest) == 3
    regression = (PACKET / "rooted-exec-regressions.log").read_text()
    for count in (7, 10, 9, 11, 8, 18, 12):
        assert f"test result: ok. {count} passed; 0 failed" in regression
    baseline = (PACKET / "rooted-exec-baseline.log").read_text()
    for count in (7, 9):
        assert f"test result: ok. {count} passed; 0 failed" in baseline
    for name, count in [("linux-msrv", 11), ("windows-tests", 9), ("root-contracts", 142)]:
        assert f"test result: ok. {count} passed; 0 failed" in (PACKET / f"rooted-exec-{name}.log").read_text()
    assert "normal-library rooted-exec fixture symbols absent" in (PACKET / "rooted-exec-normal-library.log").read_text()
    assert "169 discovered gates, all registered" in (PACKET / "rooted-exec-gate-registry.log").read_text()
    assert "local, non-promotable" in (PACKET / "rooted-exec-local-contract.log").read_text()
    assert "ship admission is closed" in (PACKET / "rooted-exec-require-ship.log").read_text()
    assert "error[E0433]" in (PACKET / "rooted-exec-missing-owner.log").read_text()
    assert "error[E0599]" in (PACKET / "rooted-exec-development.log").read_text()
    assert "collapsible_if" in (PACKET / "rooted-exec-linux-check-lint.log").read_text()
    negative = (PACKET / "rooted-exec-committed-packet-development.log").read_text()
    assert "committed_raw_capture_mismatches=['rooted-exec-doc-check.log'" in negative
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified selected-root original-exec evidence, {len(captures)} captures; custody/handoff/start closed.")


def verify_committed(commit):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    names = {"manifest.json", "README.md", ".gitattributes", "verify_packet.py", "test_verify_packet.py"}
    names.update(c["path"] for c in manifest["captures"])
    relative = PACKET.relative_to(ROOT)
    for name in sorted(names):
        assert (PACKET / name).read_bytes() == committed(commit, (relative / name).as_posix()), name
    print(f"Verified committed packet bytes at {commit}; no Git EOL normalization.")


if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])

