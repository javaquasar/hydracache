"""Read-only original-creator evidence; not production custody or workload admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from collections import Counter
from functools import lru_cache

PACKET=Path(__file__).resolve().parent
ROOT=PACKET.parents[5]
SOURCE="658d66fbeb6760c05d557b2d410898b4f25feb32"
BASELINE="23fdfa78cab391e18768ae2ccd9105679712c621"
PREREG="e80d47d03cf5b3282f89e653651dd925499dd018"
SOURCES=set([
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_creator_task.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_creator_loss_fixture.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/GATES.md",
    "docs/COMPAT.md",
    "docs/testing/performance/0.74/diagnostic-signer-creator-loss-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-creator-loss-design.md",
    "docs/testing/performance/0.74/diagnostic-signer-creator-loss-plan-20261015.md"
])
UNCHANGED=set([
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
    "docs/testing/performance/0.74/diagnostic-signer-tracer-loss-fd-amendment-20261015.md"
])
EXPECTED={
    "creator-loss-baseline.log": 0,
    "creator-loss-missing-guard.log": 1,
    "creator-loss-development.log": 0,
    "creator-loss-development-2.log": 0,
    "creator-loss-linux-check-lint.log": 0,
    "creator-loss-linux-msrv.log": 0,
    "creator-loss-source-contracts.log": 0,
    "creator-loss-xtask-check.log": 0,
    "creator-loss-xtask-lint.log": 0,
    "creator-loss-windows-tests.log": 0,
    "creator-loss-windows-check.log": 0,
    "creator-loss-windows-lint.log": 0,
    "creator-loss-windows-msrv.log": 0,
    "creator-loss-doc-check.log": 0,
    "creator-loss-gate-registry.log": 0,
    "creator-loss-local-contract.log": 0,
    "creator-loss-require-ship.log": 1,
    "creator-loss-format.log": 0,
    "creator-loss-normal-library.log": 0,
    "creator-loss-linux-final.log": 0
}
ADMISSION=set([
    "forced_tid_reuse_reproduced",
    "arbitrary_multithreaded_signer_supported",
    "all_thread_credential_or_policy_attestation",
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
QUALIFICATION="11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"

def sha(data):
    return hashlib.sha256(data).hexdigest()

def lf(data):
    return data.replace(b"\r\n",b"\n")

@lru_cache(maxsize=512)
def committed(commit,path):
    # Only immutable full-SHA objects are cached; worktree/packet bytes are always reread.
    assert re.fullmatch("[0-9a-f]{40}",commit)
    return subprocess.check_output(["git","show",f"{commit}:{path}"],cwd=ROOT)

def unique(pairs):
    result={}
    for key,value in pairs:
        assert key not in result, key
        result[key]=value
    return result

def manifest():
    return json.loads((PACKET/"manifest.json").read_bytes(),object_pairs_hook=unique)

def text(name):
    return (PACKET/name).read_text()

def results(name):
    return [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed;",text(name))]

def main():
    m=manifest()
    assert m["schema_version"]==1 and m["packet_id"]=="diagnostic-signer-creator-loss-20261015"
    assert m["source_commit"]==SOURCE and re.fullmatch("[0-9a-f]{40}",SOURCE)
    assert m["baseline_source"]==BASELINE and m["preregistration"]==PREREG
    assert m["b73_frozen_product"]=="16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    for key,expected,commit in [("source_files",SOURCES,SOURCE),("unchanged_files",UNCHANGED,BASELINE)]:
        rows=m[key]
        assert len(rows)==len(expected) and {r["path"] for r in rows}==expected
        for row in rows:
            path=row["path"]
            current=lf((ROOT/path).read_bytes())
            assert sha(current)==row["sha256_lf"],path
            assert current==lf(committed(commit,path)),path
            if key=="unchanged_files":
                assert current==lf(committed(SOURCE,path)),path
    qualification="docs/testing/performance/0.74/qualification-manifest.toml"
    assert m["qualification_sha256_raw"]==QUALIFICATION
    assert sha((ROOT/qualification).read_bytes())==QUALIFICATION
    assert sha(committed(BASELINE,qualification))==QUALIFICATION
    assert (PACKET/".gitattributes").read_bytes()==b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof\n"
    observer="tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_fixture.rs"
    declaration=b'#[path = "diagnostic_signer_creator_loss_fixture.rs"]\nmod creator_loss;\n'
    assert lf((ROOT/observer).read_bytes()).replace(declaration,b"",1)==lf(committed(BASELINE,observer))
    old="tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs"
    assert lf((ROOT/old).read_bytes()).replace(b"            Fault::NoCreatorCheck => return false,\n",b"",1)==lf(committed(BASELINE,old))
    captures=m["captures"]
    names=[r["path"] for r in captures]
    optional={"creator-loss-packet-windows.log","creator-loss-packet-linux.log"}
    assert len(names)==len(set(names)) and set(EXPECTED)<=set(names)<=set(EXPECTED)|optional
    for row in captures:
        assert Path(row["path"]).name==row["path"]
        raw=(PACKET/row["path"]).read_bytes()
        assert len(raw)==row["bytes"] and sha(raw)==row["sha256"],row["path"]
        assert row["expected_outer_exit"]==row["observed_outer_exit"]==EXPECTED.get(row["path"],0)
    assert results("creator-loss-baseline.log")==[7]
    assert results("creator-loss-linux-final.log")==[8,8,8,7,11,13,13,11,7,10,9,11,8,18,12]
    assert results("creator-loss-linux-msrv.log")==[8]
    assert results("creator-loss-source-contracts.log")==[147]
    assert results("creator-loss-windows-tests.log")==[9]
    final=text("creator-loss-linux-final.log")
    pattern=r"creator_terminal cut=(\d) control=(true|false) healthy=false code=(\d+) status=(\d+) original_tgid_alive_at_child_terminal=true independent_reaped=true child_signaled=false dummy_sends=0"
    records=re.findall(pattern,final)
    assert len(records)==54
    positive=[(int(cut),int(code),int(status)) for cut,control,code,status in records if control=="false"]
    assert Counter(cut for cut,_,_ in positive)=={0:12,1:9,2:9,3:9,4:12}
    assert all((code,status)==((1,81) if cut==0 else (2,9)) for cut,code,status in positive)
    controls=Counter((int(cut),int(code),int(status)) for cut,control,code,status in records if control=="true")
    assert controls=={(0,1,86):3}
    healthy=re.findall(r"creator_terminal cut=(\d) control=false healthy=true original_ACK_close_exit0=true keeper_joined=true",final)
    assert Counter(healthy)=={str(cut):3 for cut in range(5)}
    for marker in [
        "creator_mutations seed=0x0740c072 cases=256 exact_status_frame_terminal=true control_never_positive=true",
        "creator_task_lifetime original_dead_object_not_refreshed=true duplicates_closed=true nonproc_opath_refused=true reader_fds_reconciled=true",
        "creator_concurrency seed=0x0740c072 fresh=8 independent=4 ordinary_subreaper_unchanged=true"]:
        assert final.count(marker)==3,marker
    for prefix,digest in [
        ("handoff","b67ab287ac59bbe5c22649ea79ffa5c32e34f079f8a56cb294f88c52075908e4"),
        ("namespace","bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081"),
        ("gnu","e725127e5a399c0a18099e79157b11527bf41f0abd702bf09aaf3474a75ebf37")]:
        assert f"fixture_{prefix}_helper_sha256={digest}" in final
    for name,marker in [
        ("missing-guard","E0425"),("normal-library","normal-library_creator_fixture_symbols_absent"),
        ("gate-registry","169 discovered gates, all registered"),
        ("format","Scoped Rust1.94 rustfmt check passed"),
        ("doc-check","doc-check: OK"),("local-contract","local, non-promotable"),
        ("require-ship","ship admission is closed")]:
        assert marker in text(f"creator-loss-{name}.log"),name
    assert set(m["admission_claims"])==ADMISSION and all(v is False for v in m["admission_claims"].values())
    for name,key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha(lf((PACKET/name).read_bytes()))==m[key]
    print(f"Verified original creating-thread loss: {len(captures)} captures; production/forced-TID-reuse remain closed.")

def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    m=manifest()
    names={"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(row["path"] for row in m["captures"])
    relative=PACKET.relative_to(ROOT)
    for name in sorted(names):
        assert (PACKET/name).read_bytes()==committed(commit,(relative/name).as_posix()),name
    print(f"Verified exact committed creator-loss packet bytes at {commit}.")

if __name__=="__main__":
    main()
    if len(sys.argv)!=1:
        assert len(sys.argv)==3 and sys.argv[1]=="--committed"
        verify_committed(sys.argv[2])
