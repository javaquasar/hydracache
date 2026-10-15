"""Read-only local original tracer thread loss evidence, never execution authority."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from collections import Counter
from functools import lru_cache

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "7f239a949d05ca7b808ead22c233385bdb34ffe6"
BASELINE = "cd0e66ba0bd881740808ce8434a9fc5ca95dc5c7"
PREREG = "f3b523fd9f50e73d8c34ce2db2cbf891d931b779"
PROGRESS = "1044860a2dfb77180f083efef5d384fc2681ab6f"
SOURCES = set([
    "tools/long-run-supervisor-074/src/diagnostic_signer_creator_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_thread_exit_fixture.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-thread-exit-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-thread-exit-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-thread-exit-progress.md"
])
UNCHANGED = set([
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_channel.rs",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-plan-20261015.md",
    "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-progress-20261015.md",
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
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-fd-amendment-20261015.md",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_creator_task.rs",
    "docs/testing/performance/0.74/diagnostic-signer-creator-loss-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-creator-loss-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-creator-loss-plan-20261015.md"
])
EXPECTED = {
    "thread-exit-baseline.log": 0,
    "thread-exit-missing.log": 1,
    "thread-exit-development.log": 0,
    "thread-exit-linux-final.log": 1,
    "thread-exit-linux-msrv.log": 0,
    "thread-exit-source-contracts.log": 0,
    "thread-exit-windows.log": 0,
    "thread-exit-static.log": 0,
    "thread-exit-require-ship.log": 1,
    "thread-exit-linux-final-2.log": 0,
    "thread-exit-source-contracts-final.log": 0,
    "thread-exit-linux-msrv-final.log": 0,
    "thread-exit-normal-library.log": 0,
    "thread-exit-static-final.log": 0,
    "thread-exit-windows-final-check.log": 0,
    "thread-exit-linux-check-lint.log": 0
}
ADMISSION = set([
    "arbitrary_threads_supported",
    "forced_tid_reuse_reproduced",
    "production_runtime_or_issuer_custody",
    "actual_private_key_or_token_read",
    "authenticated_original_worker_start",
    "all_writers_revoked",
    "completion_or_later_cells_enabled",
    "host_services_or_cgroups_changed",
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

@lru_cache(maxsize=512)
def committed(commit, path):
    assert re.fullmatch("[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=ROOT)

def unique(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, key
        result[key] = value
    return result

def manifest():
    return json.loads((PACKET/"manifest.json").read_bytes(), object_pairs_hook=unique)

def text(name):
    return (PACKET/name).read_text(encoding="utf-8-sig")

def results(name):
    return [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed;", text(name))]

def main():
    m = manifest()
    assert m["schema_version"] == 1 and m["packet_id"] == "diagnostic-signer-thread-exit-20261015"
    assert m["source_commit"] == SOURCE and m["baseline_source"] == BASELINE
    assert m["preregistration"] == PREREG and m["progress_amendment"] == PROGRESS
    assert m["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    for key, expected, commit in [("source_files", SOURCES, SOURCE), ("unchanged_files", UNCHANGED, BASELINE)]:
        rows = m[key]
        assert len(rows) == len(expected) and {r["path"] for r in rows} == expected
        for row in rows:
            path = row["path"]
            current = lf((ROOT/path).read_bytes())
            assert sha(current) == row["sha256_lf"], path
            assert current == lf(committed(commit, path)), path
            if key == "unchanged_files":
                assert current == lf(committed(SOURCE, path)), path
    old = "tools/long-run-supervisor-074/src/diagnostic_signer_creator_loss_fixture.rs"
    declaration = b'#[path = "diagnostic_signer_thread_exit_fixture.rs"]\nmod thread_exit;\n'
    assert lf((ROOT/old).read_bytes()).replace(declaration,b"",1) == lf(committed(BASELINE,old))
    qualification = "docs/testing/performance/0.74/qualification-manifest.toml"
    assert m["qualification_sha256_raw"] == QUALIFICATION
    assert sha((ROOT/qualification).read_bytes()) == sha(committed(BASELINE,qualification)) == QUALIFICATION
    assert (PACKET/".gitattributes").read_bytes() == b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof\n"
    captures = m["captures"]
    names = [r["path"] for r in captures]
    assert len(names) == len(set(names)) and set(names) == set(EXPECTED)
    for row in captures:
        assert Path(row["path"]).name == row["path"]
        raw = (PACKET/row["path"]).read_bytes()
        assert sha(raw) == row["sha256"] and len(raw) == row["bytes"], row["path"]
        assert row["observed_outer_exit"] == row["expected_outer_exit"] == EXPECTED[row["path"]]
    assert results("thread-exit-baseline.log") == [8]
    assert results("thread-exit-linux-final-2.log") == [6,6,6,8,7,11,13,13,11,7,10,9,11,8,18,12]
    assert results("thread-exit-linux-msrv-final.log") == [6]
    assert results("thread-exit-source-contracts-final.log") == [148]
    assert results("thread-exit-windows.log") == [9]
    final = text("thread-exit-linux-final-2.log")
    pattern = r"thread_terminal stage=(\d) mode=(\d) code=(\d+) status=(\d+) original_tgid_alive_at_child_terminal=true independent_reaped=true child_signaled=(true|false) dummy_sends=(\d) positive=(true|false)"
    records = re.findall(pattern, final)
    assert len(records) == 57
    positives = [(int(s), int(c), int(v), killed, int(sent)) for s,mode,c,v,killed,sent,p in records if p == "true" and mode == "0"]
    assert Counter(s for s,_,_,_,_ in positives) == {0:3,1:15,2:12,3:6,4:12,5:6}
    assert all((c,v,killed,sent) == (2,9,"false",int(s==5)) for s,c,v,killed,sent in positives)
    negatives = [(s,mode,c,v,killed,sent,p) for s,mode,c,v,killed,sent,p in records if p == "false"]
    assert negatives == [("1","1","2","9","true","0","false")]*3
    healthy = re.findall(r"thread_terminal stage=(\d) mode=2 original_ACK_close_exit0=true keeper_joined=true", final)
    assert Counter(healthy) == {str(s):3 for s in range(6)}
    assert len(re.findall(r"thread_terminal ",final)) == 75
    for marker in [
        "thread_negative stage=1 detached=true original_child_nonterminal=true original_tgid_nonterminal=true cleanup_is_not_positive=true",
        "thread_mutations exact_frame_terminal=true control_never_positive=true",
        "thread_concurrency seed=0x0740d073 fresh=8 independent=4 ordinary_subreaper_unchanged=true"]:
        assert final.count(marker) == 3, marker
    for name,marker in [
        ("thread-exit-missing.log","E0425"),
        ("thread-exit-linux-final.log","test result: FAILED. 5 passed; 1 failed;"),
        ("thread-exit-linux-final.log","original loss observer stalled"),
        ("thread-exit-static.log","169 discovered gates, all registered"),
        ("thread-exit-static.log","local, non-promotable"),
        ("thread-exit-static.log","doc-check: OK"),
        ("thread-exit-static.log","thread_static docs_registry_local_contract_scoped_format=passed"),
        ("thread-exit-static-final.log","thread_static docs_registry_local_contract_scoped_format=passed"),
        ("thread-exit-normal-library.log","normal-library_thread_fixture_symbols_absent"),
        ("thread-exit-require-ship.log","ship admission is closed")]:
        assert marker in text(name), (name,marker)
    for digest in [
        "b67ab287ac59bbe5c22649ea79ffa5c32e34f079f8a56cb294f88c52075908e4",
        "bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081",
        "e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37"]:
        assert digest in final
    assert set(m["admission_claims"]) == ADMISSION and all(v is False for v in m["admission_claims"].values())
    for name,key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha(lf((PACKET/name).read_bytes())) == m[key]
    print(f"Verified original tracing-thread loss: {len(captures)} captures; production and server admission closed.")

def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}", commit)
    names = {"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(r["path"] for r in manifest()["captures"])
    for name in sorted(names):
        assert (PACKET/name).read_bytes() == committed(commit,(PACKET.relative_to(ROOT)/name).as_posix()),name
    print(f"Verified exact committed thread-exit packet bytes at {commit}.")

if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])
