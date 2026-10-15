"""Read-only constructor-loss evidence; not production custody or admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from collections import Counter

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "815fc5d7527153387b3b4c095c035cda03df523e"
BASELINE = "e4eaea4cd4af5e09082cf8cb8b759ff40ae09d7c"
PREREG = "6a02eeef9e09b319f2711953b80a9afb694be0b1"
AMENDMENT = "32237eb368e230f72fca636747cbc06984bf85ed"
SOURCES = set([
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_channel.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-progress-20261015.md"
])
UNCHANGED = set([
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
    "docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md",
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_tests.rs",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-root-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-mode-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-control-result-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-progress-amendment-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-fd-amendment-20261015.md"
])
EXPECTED = {
    "constructor-loss-baseline.log": 1,
    "constructor-loss-baseline-check.log": 0,
    "constructor-loss-missing-guard.log": 1,
    "constructor-loss-development.log": 1,
    "constructor-loss-development-2.log": 0,
    "constructor-loss-development-3.log": 0,
    "constructor-loss-shell-selection.log": 1,
    "constructor-loss-selection-development.log": 0,
    "constructor-loss-msrv-pressure.log": 1,
    "constructor-loss-source-contracts.log": 0,
    "constructor-loss-source-contracts-final.log": 0,
    "constructor-loss-windows-check.log": 0,
    "constructor-loss-windows-tests.log": 0,
    "constructor-loss-windows-portable.log": 0,
    "constructor-loss-windows-lint.log": 0,
    "constructor-loss-windows-msrv.log": 0,
    "constructor-loss-xtask-check.log": 0,
    "constructor-loss-xtask-lint.log": 0,
    "constructor-loss-xtask-lint-final.log": 0,
    "constructor-loss-doc-check.log": 0,
    "constructor-loss-doc-final.log": 0,
    "constructor-loss-local-contract.log": 0,
    "constructor-loss-require-ship.log": 1,
    "constructor-loss-linux-check-lint.log": 0,
    "constructor-loss-linux-msrv-final.log": 0,
    "constructor-loss-gate-registry.log": 1,
    "constructor-loss-gate-registry-final.log": 0,
    "constructor-loss-format-final.log": 0,
    "constructor-loss-normal-library.log": 0,
    "constructor-loss-windows-check-final.log": 0,
    "constructor-loss-windows-lint-final.log": 0,
    "constructor-loss-final-verified.log": 0
}
ADMISSION = set([
    "pre_arm_creating_thread_only_loss_proved",
    "arbitrary_multithreaded_signer_supported",
    "production_tracer_lifetime_enrolled",
    "independent_runtime_custody",
    "continuous_loaded_runtime_closure_proved",
    "mapped_memory_attested",
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
QUALIFICATION = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"

def sha(data):
    return hashlib.sha256(data).hexdigest()

def lf(data):
    return data.replace(b"\r\n", b"\n")

def committed(commit, path):
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)

def text(name):
    return (PACKET / name).read_text()

def results(name):
    return [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed;", text(name))]

def main():
    m = json.loads((PACKET / "manifest.json").read_bytes())
    assert m["schema_version"] == 1
    assert m["packet_id"] == "diagnostic-signer-constructor-loss-20261015"
    assert m["source_commit"] == SOURCE and re.fullmatch("[0-9a-f]{40}", SOURCE)
    assert m["baseline_source"] == BASELINE and m["preregistration"] == PREREG
    assert m["progress_amendment"] == AMENDMENT
    assert m["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    for key, expected, commit in [("source_files", SOURCES, SOURCE),
                                   ("unchanged_files", UNCHANGED, BASELINE)]:
        rows = m[key]
        assert len(rows) == len(expected) and {r["path"] for r in rows} == expected
        for row in rows:
            path = row["path"]
            current = lf((ROOT / path).read_bytes())
            assert sha(current) == row["sha256_lf"], path
            assert current == lf(committed(commit, path)), path
            if key == "unchanged_files":
                assert current == lf(committed(SOURCE, path)), path
    qualification = "docs/testing/performance/0.74/qualification-manifest.toml"
    assert m["qualification_sha256_raw"] == QUALIFICATION
    assert sha((ROOT / qualification).read_bytes()) == QUALIFICATION
    assert sha(committed(BASELINE, qualification)) == QUALIFICATION
    assert (PACKET / ".gitattributes").read_bytes() == b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof\n"
    # Reuse the old bounded parser and observer without silently changing their bodies.
    channel = "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_channel.rs"
    assert lf((ROOT/channel).read_bytes()).replace(b"pub(super) fn packet(",b"fn packet(",1) == lf(committed(BASELINE,channel))
    observer = "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_fixture.rs"
    declaration = b'#[path = "diagnostic_signer_constructor_loss_fixture.rs"]\nmod early;\n'
    assert lf((ROOT/observer).read_bytes()).replace(declaration,b"",1) == lf(committed(BASELINE,observer))
    captures = m["captures"]
    names = [c["path"] for c in captures]
    auxiliary = {"constructor-loss-packet-windows.log", "constructor-loss-packet-linux.log"}
    assert len(names) == len(set(names)) and set(EXPECTED) <= set(names)
    assert set(names) <= set(EXPECTED) | auxiliary
    for c in captures:
        assert Path(c["path"]).name == c["path"]
        raw = (PACKET / c["path"]).read_bytes()
        assert len(raw) == c["bytes"] and sha(raw) == c["sha256"], c["path"]
        assert c["expected_outer_exit"] == c["observed_outer_exit"] == EXPECTED.get(c["path"],0)
    assert results("constructor-loss-baseline.log") == [11]
    assert results("constructor-loss-final-verified.log") == [7,7,7,11,13,13,11,7,10,9,11,8,18,12]
    assert results("constructor-loss-linux-msrv-final.log") == [7]
    assert results("constructor-loss-windows-portable.log") == [9]
    assert results("constructor-loss-source-contracts-final.log") == [146]
    final = text("constructor-loss-final-verified.log")
    deaths = re.findall(r"constructor_terminal cut=(\d) fault=(\d) healthy=false code=(\d+) status=(\d+) original_pidfd_reaped=true child_signaled=false dummy_sends=0",final)
    assert len(deaths) == 57
    expected_cuts = {0:12,1:9,2:12,3:9,4:9}
    positive = [(int(cut),int(code),int(status)) for cut,fault,code,status in deaths if fault == "0"]
    assert Counter(cut for cut,_,_ in positive) == expected_cuts
    assert all((code,status) == ((1,81) if cut == 0 else (2,9)) for cut,code,status in positive)
    controls = Counter((int(cut),int(fault),int(code),int(status)) for cut,fault,code,status in deaths if fault != "0")
    assert controls == {(1,1,1,83):3,(0,2,1,84):3}
    healthy = re.findall(r"constructor_terminal cut=(\d) fault=0 healthy=true original_ACK_close_exit0=true healthy_progress=7",final)
    assert Counter(healthy) == {str(cut):3 for cut in range(5)}
    for marker in [
        "constructor_frame seed=0x0740c071 mutations=256 exact_terminal_oracle=true controls_never_positive=true",
        "constructor_concurrency seed=0x0740c071 fresh=8 independent=4 ordinary_subreaper_unchanged=true"]:
        assert final.count(marker) == 3, marker
    for prefix,digest in [
        ("handoff","b67ab287ac59bbe5c22649ea79ffa5c32e34f079f8a56cb294f88c52075908e4"),
        ("namespace","bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081"),
        ("gnu","e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37")]:
        assert f"fixture_{prefix}_helper_sha256={digest}" in final
    for name,marker in [
        ("missing-guard","E0425"), ("development","E0133"),
        ("msrv-pressure","FAILED. 6 passed; 1 failed"),
        ("shell-selection","7 passed"),
        ("selection-development","0 passed"),
        ("windows-tests","0 passed"),
        ("gate-registry","unsupported xtask command"),
        ("gate-registry-final","169 discovered gates, all registered"),
        ("normal-library","normal-library_constructor_fixture_symbols_absent"),
        ("format-final","Scoped Rust1.94 rustfmt check passed"),
        ("local-contract","local, non-promotable"),
        ("require-ship","ship admission is closed")]:
        assert marker in text(f"constructor-loss-{name}.log"), name
    assert set(m["admission_claims"]) == ADMISSION and all(v is False for v in m["admission_claims"].values())
    for name,key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha(lf((PACKET/name).read_bytes())) == m[key]
    print(f"Verified constructor-loss packet: {len(captures)} captures, five whole-parent cuts; thread-only/production remain closed.")

def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    m = json.loads((PACKET/"manifest.json").read_bytes())
    names = {"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(c["path"] for c in m["captures"])
    relative = PACKET.relative_to(ROOT)
    for name in sorted(names):
        assert (PACKET/name).read_bytes() == committed(commit,(relative/name).as_posix()), name
    print(f"Verified exact committed packet bytes at {commit}.")

if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])
