"""Read-only original tracer-loss evidence; not production enrollment."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from collections import Counter

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "5882caae03acc5b4a5b336d3f83a6bb2982b281d"
BASELINE = "ad5c4c8681eabde410d4195cdadb9bad33afa323"
SOURCES = set([
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_channel.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_tests.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-root-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-mode-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-control-result-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-progress-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-fd-amendment-20261015.md"
])
UNCHANGED = set([
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-handoff-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-handoff-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-handoff-plan-20261015.md",
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
    "docs/testing/performance/0.74/diagnostic-signer-namespace-control-result-20261015.md",
    "tools/long-run-supervisor-074/src/diagnostic_kernel_ancillary.rs",
    "docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"
])
EXPECTED = {
    "signer-loss-baseline.log": 0,
    "signer-loss-check-development.log": 0,
    "signer-loss-development-2.log": 1,
    "signer-loss-development-3.log": 1,
    "signer-loss-development.log": 1,
    "signer-loss-directory-diagnostic.log": 1,
    "signer-loss-doc-check.log": 0,
    "signer-loss-fd-development.log": 0,
    "signer-loss-format-development.log": 1,
    "signer-loss-format-final.log": 0,
    "signer-loss-format.log": 0,
    "signer-loss-gate-registry.log": 0,
    "signer-loss-linux-before-fd-isolation.log": 0,
    "signer-loss-linux-final.log": 0,
    "signer-loss-linux-msrv.log": 0,
    "signer-loss-linux-pressure.log": 1,
    "signer-loss-local-contract.log": 0,
    "signer-loss-missing-observer.log": 1,
    "signer-loss-msrv-fd-development.log": 1,
    "signer-loss-msrv-pressure.log": 1,
    "signer-loss-normal-library.log": 0,
    "signer-loss-progress-development.log": 0,
    "signer-loss-report-diagnostic.log": 1,
    "signer-loss-require-ship.log": 1,
    "signer-loss-root-contracts-final.log": 0,
    "signer-loss-root-contracts-isolated.log": 0,
    "signer-loss-root-contracts.log": 0,
    "signer-loss-windows-check-final.log": 0,
    "signer-loss-windows-check.log": 0,
    "signer-loss-windows-lint-final.log": 0,
    "signer-loss-windows-lint.log": 0,
    "signer-loss-windows-msrv.log": 0,
    "signer-loss-windows-tests.log": 0,
    "signer-loss-xtask-check.log": 0,
    "signer-loss-xtask-lint-final.log": 0,
    "signer-loss-xtask-lint.log": 0
}
ADMISSION = set([
    "pre_options_constructor_window_proved",
    "continuous_loaded_runtime_closure_proved",
    "mapped_memory_attested",
    "independent_runtime_custody",
    "complete_sandbox",
    "multithreaded_signer_supported",
    "production_tracer_lifetime_enrolled",
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
AUXILIARY = {"signer-loss-packet-windows.log", "signer-loss-packet-linux.log"}
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
    assert m["packet_id"] == "diagnostic-signer-tracer-loss-20261015"
    assert m["source_commit"] == SOURCE and m["baseline_source"] == BASELINE
    assert m["preregistration"] == "8af96999b72816fb9ed9fa3fce2defafbea242cf"
    assert m["amendments"] == [
        "f1abc0f2206ea49c0930890f0dd1985fba20ea04",
        "b797aa4ddd7a604d6462f952a57a6ac653df8dcc",
        "a14433d0ed39ab37e7e91fe282f9b3b51726285a",
        "d535639b752e3b376c300639e7c4c072b88a6f61",
        "efcfc0933ce9a9f7a79c18c3118cd7046b38a76d"]
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
    parent = "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_fixture.rs"
    addition = b'#[path = "diagnostic_signer_tracer_loss_fixture.rs"]\nmod loss;\n'
    after = committed(SOURCE, parent).replace(b"\r\n", b"\n")
    assert after.count(addition) == 1
    assert after.replace(addition, b"") == committed(BASELINE, parent).replace(b"\r\n", b"\n")
    q = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(q) == m["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    assert q == committed(SOURCE, "docs/testing/performance/0.74/qualification-manifest.toml")
    captures = m["captures"]
    names = {c["path"] for c in captures}
    assert len(captures) == len(names)
    assert set(EXPECTED) <= names <= set(EXPECTED) | AUXILIARY
    assert names == {p.name for p in PACKET.glob("*.log")}
    for c in captures:
        assert re.fullmatch(r"signer-loss-[a-z0-9-]+\.log", c["path"])
        raw = (PACKET / c["path"]).read_bytes()
        assert sha(raw) == c["sha256"] and len(raw) == c["bytes"], c["path"]
        assert type(c["expected_outer_exit"]) is type(c["observed_outer_exit"]) is int
        assert c["expected_outer_exit"] == c["observed_outer_exit"] == EXPECTED.get(c["path"], 0)
    assert results("signer-loss-linux-final.log") == [11,11,11,13,13,11,7,10,9,11,8,18,12]
    assert results("signer-loss-baseline.log") == [13]
    for name, count in [("linux-msrv",11),("windows-tests",9),("root-contracts-isolated",145)]:
        assert results(f"signer-loss-{name}.log") == [count]
    linux = (PACKET / "signer-loss-linux-final.log").read_text()
    markers = [
        "tracer_loss=installed tracer=SIGKILL child=SIGKILL original_pidfd_reaped=true emergency_cleanup=false dummy_sends=0",
        "tracer_loss=ready tracer=SIGKILL child=SIGKILL original_pidfd_reaped=true emergency_cleanup=false dummy_sends=0",
        "tracer_loss=delivered tracer=SIGKILL child=SIGKILL original_pidfd_reaped=true emergency_cleanup=false dummy_sends=1",
        "tracer_loss=initial,rooted,first_exec original_child=SIGKILL no_authority_or_restart=true",
        "tracer_loss_without_EXITKILL=bounded_natural_exit old_exact_prediction=false live_orphan_reproduced=false direct_child_kill=false",
        "tracer_loss_healthy_control=ACK_close_exit0 original_owner_reaped=true replaced=false",
        "tracer_loss_report_faults=6 original_owner_refuses_and_reaps=true no_unvalidated_pidfd_signal=true",
        "tracer_loss_frame_seed=0x07404a12 bounded_mutations=256 future_fields_refused=true",
        "tracer_loss_seed=0x07404a12 fresh_cases=8 independent_monitors=4 parent_subreaper_unchanged=true",
        "tracer_loss_received_rights=owned_before_refusal pipe_EOF_proves_no_leaked_writer=true"]
    for marker in markers:
        assert linux.count(marker) == 3, marker
    tuples = re.findall(r"loss_child_terminal stage=(\d) mode=(kill|no-exitkill) code=(\d+) status=(\d+)", linux)
    kills = [(int(stage),int(code),int(status)) for stage,mode,code,status in tuples if mode == "kill"]
    controls = [(int(stage),int(code),int(status)) for stage,mode,code,status in tuples if mode == "no-exitkill"]
    assert len(kills) == 54 and {s for s,_,_ in kills} == set(range(6))
    assert all((code,status) == (2,9) for _,code,status in kills)
    assert len(controls) == 9 and Counter(stage for stage,_,_ in controls) == {3:3,4:3,5:3}
    assert all(code == 1 and status in {3:{73,74},4:{74},5:{0,76}}[stage]
               for stage,code,status in controls)
    assert m["fixture_helper_sha256"] == HELPER
    for prefix, digest in [("handoff",HELPER),
        ("namespace","bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081"),
        ("gnu","e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37")]:
        assert f"fixture_{prefix}_helper_sha256={digest}" in linux
    for name, marker in [
        ("missing-observer","E0425"), ("development","FAILED. 3 passed; 7 failed"),
        ("report-diagnostic","directories: owned runtime root refused"),
        ("development-2","FAILED. 3 passed; 7 failed"),
        ("directory-diagnostic","loss_container mode=755 uid=1000"),
        ("development-3","child death (1, 74), expected (1, 73)"),
        ("linux-pressure","report: original loss observer stalled"),
        ("msrv-pressure","FAILED. 8 passed; 2 failed"),
        ("msrv-fd-development","FAILED. 10 passed; 1 failed"),
        ("format-development","expected expression"),
        ("normal-library","normal-library_tracer_loss_fixture_symbols_absent"),
        ("gate-registry","169 discovered gates, all registered"),
        ("local-contract","local, non-promotable"),
        ("require-ship","ship admission is closed"),
        ("format-final","scoped Rust1.94 rustfmt check passed"),
    ]:
        assert marker in (PACKET / f"signer-loss-{name}.log").read_text(), name
    assert set(m["admission_claims"]) == ADMISSION
    assert all(v is False for v in m["admission_claims"].values())
    for name, key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n",b"\n")) == m[key]
    print(f"Verified original tracer-loss evidence, {len(captures)} captures; constructor window and production remain closed.")

def verify_committed(commit):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    m = json.loads((PACKET / "manifest.json").read_bytes())
    names = {"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(c["path"] for c in m["captures"])
    relative = PACKET.relative_to(ROOT)
    for name in sorted(names):
        assert (PACKET / name).read_bytes() == committed(commit,(relative / name).as_posix()), name
    print(f"Verified committed packet bytes at {commit}; no Git EOL normalization.")

if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])
