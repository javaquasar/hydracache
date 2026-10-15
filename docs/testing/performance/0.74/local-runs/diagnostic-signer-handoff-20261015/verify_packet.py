"""Read-only source-bound synthetic handoff evidence, never production authority."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "a2b763533e997aef1c4de9ecfbb2cfa77f3e9f8d"
BASELINE = "d5d502d570c7ec062e911617f143b74992bc5742"
SOURCES = set([
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_tests.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-handoff-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-handoff-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-handoff-plan-20261015.md"
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
    "docs/testing/performance/0.74/diagnostic-signer-rooted-exec-design.md",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_fixture_tests.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-environ-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-fd-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-namespace-control-result-20261015.md"
])
EXPECTED = {
    "signer-handoff-baseline.log": 0,
    "signer-handoff-check-development.log": 0,
    "signer-handoff-development-2.log": 1,
    "signer-handoff-development-3.log": 0,
    "signer-handoff-development.log": 1,
    "signer-handoff-doc-check.log": 0,
    "signer-handoff-format.log": 0,
    "signer-handoff-gate-registry.log": 0,
    "signer-handoff-helper-lint.log": 0,
    "signer-handoff-linux-final.log": 1,
    "signer-handoff-linux-msrv.log": 0,
    "signer-handoff-linux-regression-completion.log": 0,
    "signer-handoff-linux-tail.log": 0,
    "signer-handoff-local-contract.log": 0,
    "signer-handoff-missing-owner.log": 1,
    "signer-handoff-normal-library-final.log": 0,
    "signer-handoff-normal-library.log": 1,
    "signer-handoff-require-ship.log": 1,
    "signer-handoff-root-contracts-development.log": 1,
    "signer-handoff-root-contracts-lock.log": 101,
    "signer-handoff-root-contracts.log": 0,
    "signer-handoff-stage-diagnostic.log": 1,
    "signer-handoff-windows-check.log": 0,
    "signer-handoff-windows-lint.log": 0,
    "signer-handoff-windows-msrv.log": 0,
    "signer-handoff-windows-tests.log": 0,
    "signer-handoff-xtask-check.log": 0,
    "signer-handoff-xtask-lint-final.log": 0,
    "signer-handoff-xtask-lint.log": 0
}
ADMISSION = set([
    "actual_tracer_loss_exitkill_proved",
    "continuous_loaded_runtime_closure_proved",
    "mapped_memory_attested",
    "independent_runtime_custody",
    "complete_sandbox",
    "multithreaded_signer_supported",
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
AUXILIARY = {"signer-handoff-packet-windows.log", "signer-handoff-packet-linux.log"}
HELPER = "b67ab287ac59bbe5c22649ea79ffa5c32e34f079f8a56cb294f88c52075908e4"

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def committed(commit, path):
    return subprocess.check_output(["git", "-C", str(ROOT), "show", f"{commit}:{path}"])

def results(name):
    return list(map(int, re.findall(r"test result: ok\. (\d+) passed; 0 failed",
                                   (PACKET / name).read_text())))

def main():
    m = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(m["schema_version"]) is int and m["schema_version"] == 1
    assert m["packet_id"] == "diagnostic-signer-handoff-20261015"
    assert m["source_commit"] == SOURCE and m["baseline_source"] == BASELINE
    assert m["preregistration"] == "0ae93c19fd5a0f15a9538663864cd402623715b8"
    assert m["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    assert (PACKET / ".gitattributes").read_bytes() == b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof\n"
    for group, commit, paths in [("source_files", SOURCE, SOURCES),
                                 ("unchanged_files", BASELINE, UNCHANGED)]:
        entries = m[group]
        assert len(entries) == len(paths) and {e["path"] for e in entries} == paths
        for entry in entries:
            path = entry["path"]
            raw = (ROOT / path).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], path
            assert raw == committed(commit, path).replace(b"\r\n", b"\n"), path
    q = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(q) == m["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    assert q == committed(SOURCE, "docs/testing/performance/0.74/qualification-manifest.toml")
    captures = m["captures"]
    names = {c["path"] for c in captures}
    assert len(captures) == len(names)
    assert set(EXPECTED) <= names <= set(EXPECTED) | AUXILIARY
    assert names == {p.name for p in PACKET.glob("*.log")}
    for c in captures:
        assert re.fullmatch(r"signer-handoff-[a-z0-9-]+\.log", c["path"])
        raw = (PACKET / c["path"]).read_bytes()
        assert sha(raw) == c["sha256"] and len(raw) == c["bytes"], c["path"]
        assert type(c["expected_outer_exit"]) is type(c["observed_outer_exit"]) is int
        assert c["expected_outer_exit"] == c["observed_outer_exit"] == EXPECTED.get(c["path"], 0)
    # Preserve the command-runner failure after the three complete focused repeats
    # and seven complete regression groups; do not represent its outer exit as green.
    assert results("signer-handoff-linux-final.log") == [13,13,13,13,11,7,10,9,11,8]
    assert "no test target named" in (PACKET / "signer-handoff-linux-final.log").read_text()
    # The first correction selected zero producer tests; it is not counted.
    assert results("signer-handoff-linux-tail.log") == [10,0]
    assert results("signer-handoff-linux-regression-completion.log") == [18,12]
    assert results("signer-handoff-baseline.log") == [13,11]
    for name, count in [("linux-msrv",13),("windows-tests",9),("root-contracts",144)]:
        assert results(f"signer-handoff-{name}.log") == [count]
    linux = (PACKET / "signer-handoff-linux-final.log").read_text()
    for marker in [
        "handoff_original_ready_dummy_ack_close_exit=exact packets=16 sends=1 real_credentials=false",
        "handoff_withheld=initial,rooted,exec,installed,ready no_send_before_original_read_entry=true",
        "handoff_bad_ready_cases=7 sends=0 replacement=false",
        "handoff_ACK_fault_cases=3 exactly_one_dummy_no_retry=true",
        "handoff_child_SIGKILL_drop=installed,ready,delivered tracer_loss_proved=false",
        "handoff_sendmsg_dup2_nonstack=refused_before_dispatch ancillary_FDs_received=0",
        "handoff_send_failure=latched attempted=true sends=0 retry=false",
        "handoff_seed=0x07404a11 fresh_cases=8 independent_owners=4 multithreaded_signer=false",
    ]:
        assert linux.count(marker) == 3, marker
    assert m["fixture_helper_sha256"] == HELPER
    assert linux.count("fixture_handoff_helper_sha256=" + HELPER) == 3
    assert "fixture_namespace_helper_sha256=bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081" in linux
    assert "fixture_gnu_helper_sha256=e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37" in linux
    for name, marker in [
        ("missing-owner", "E0599"), ("development", "E0382"),
        ("development-2", "FAILED. 2 passed; 9 failed"),
        ("stage-diagnostic", "handoff install: owned runtime root refused"),
        ("development-3", "test result: ok. 11 passed"),
        ("check-development", "test result: ok. 13 passed"),
        ("root-contracts-development", "FAILED. 143 passed; 1 failed"),
        ("root-contracts-lock", "Access is denied"),
        ("normal-library", "awk:"),
        ("normal-library-final", "normal-library_handoff_fixture_symbols_absent"),
        ("gate-registry", "169 discovered gates, all registered"),
        ("local-contract", "local, non-promotable"),
        ("require-ship", "ship admission is closed"),
        ("format", "scoped Rust1.94 rustfmt check passed"),
    ]:
        assert marker in (PACKET / f"signer-handoff-{name}.log").read_text(), name
    assert set(m["admission_claims"]) == ADMISSION
    assert all(v is False for v in m["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == m[key]
    print(f"Verified original synthetic handoff evidence, {len(captures)} captures; tracer loss and production remain closed.")

def verify_committed(commit):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    m = json.loads((PACKET / "manifest.json").read_bytes())
    names = {"manifest.json", "README.md", ".gitattributes", "verify_packet.py", "test_verify_packet.py"}
    names.update(c["path"] for c in m["captures"])
    relative = PACKET.relative_to(ROOT)
    for name in sorted(names):
        assert (PACKET / name).read_bytes() == committed(commit, (relative / name).as_posix()), name
    print(f"Verified committed packet bytes at {commit}; no Git EOL normalization.")

if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])
