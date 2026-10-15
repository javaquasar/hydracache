"""Read-only current-context ownership inventory evidence, never admission."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from functools import lru_cache
if not __debug__:
    raise SystemExit("Evidence checks require Python without -O")
PACKET=Path(__file__).resolve().parent
ROOT=PACKET.parents[5]
SOURCE="e535898005aef243316d8a9e5a81ce23acfa3d3f"
TREE="f118936fc665cd96a71883bec47ae598e2e593d7"
BASELINE="f09330e3cc9c211edf6a94f814fb38c3b37e5cf6"
SOURCES=set(["tools/long-run-supervisor-074/src/lib.rs","tools/long-run-supervisor-074/src/diagnostic_root_owned_input.rs","tools/long-run-supervisor-074/src/diagnostic_root_owned_input_tests.rs","tools/long-run-supervisor-074/tests/diagnostic_root_owned_input.rs","tools/long-run-supervisor-074/examples/inspect_root_owned_input.rs","crates/xtask/tests/performance_contract_074.rs","docs/GATES.md","docs/COMPAT.md","docs/testing/gated-test-registry.toml","docs/testing/performance/0.74/diagnostic-root-owned-input-contract.toml","docs/testing/performance/0.74/diagnostic-root-owned-input-design.md","docs/testing/performance/0.74/diagnostic-root-owned-input-plan-20261015.md"])
UNCHANGED=set(["Cargo.toml","Cargo.lock","tools/long-run-supervisor-074/Cargo.toml",".github/workflows/ci.yml","tools/long-run-supervisor-074/src/diagnostic_prebuilt_copy.rs","tools/long-run-supervisor-074/src/diagnostic_prebuilt_copy_tests.rs","tools/long-run-supervisor-074/tests/diagnostic_prebuilt_copy.rs","tools/long-run-supervisor-074/examples/prepare_unsigned_prebuilt.rs","tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs","tools/long-run-supervisor-074/src/diagnostic_signer_issuer.rs","tools/long-run-supervisor-074/src/diagnostic_signer_bundle.rs","tools/long-run-supervisor-074/src/diagnostic_signer_build.rs","tools/long-run-supervisor-074/src/diagnostic_signer_runtime_elf.rs","tools/long-run-supervisor-074/src/diagnostic_controller_producer_linux.rs","tools/long-run-supervisor-074/src/bin/diagnostic_controller.rs","tools/long-run-supervisor-074/src/server.rs","scripts/perf/diagnostic_signer_key_074.py","docs/testing/performance/0.74/diagnostic-signer-issuer-public.json","docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json","docs/testing/performance/0.74/diagnostic-signer-job-074.yml.disabled","docs/testing/performance/0.74/qualification-manifest.toml","docs/plans/releases.toml","docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"])
EXITS={"root-input-baseline.log":0,"root-input-missing-module.log":101,"root-input-linux-final.log":0,"root-input-windows-contracts-final.log":0,"root-input-preflight.log":0,"root-input-actual-runtime.log":0,"root-input-user-copy-refusal.log":1,"root-input-postflight.log":0,"root-input-require-ship.log":1}
FLAGS=set(["initial_host_or_filesystem_authenticated","independent_runtime_root_or_job_policy_enrolled","loaded_runtime_closure_or_memory_attested","root_ancestor_namespace_or_mount_mutated","foreign_writer_revoked_or_continuous_immutability","actual_signing_or_private_key_token_access","authenticated_original_start","whole_writer_revocation","completion_or_later_cells_enabled","server_route_or_workflow_enabled","qualification_allowed","ship_allowed","performance_claim_allowed"])
LIBRARIES=[{"path":"/usr/lib/x86_64-linux-gnu/libc.so.6","sha256":"3a15d66867d83762c7f2f1e37359cb8f6c5743edb369c65285cb0b1c4f7498bf","bytes":2129424,"mode":"0755"},{"path":"/usr/lib/x86_64-linux-gnu/libgcc_s.so.1","sha256":"d93224d2b0dab4247598be683adca02f5cf00586f99c187579cd7e92058fb7cb","bytes":183024,"mode":"0644"},{"path":"/usr/lib/x86_64-linux-gnu/ld-linux-x86-64.so.2","sha256":"c20a2dc8917c755f02b94049356320fe1f62ac7d9f8994731f807d9df39302da","bytes":236616,"mode":"0755"}]
TOOL="e10e5a14e2cfb6408f408cf249aa3b87f2c41c58524bc289cd841443ca2d8e8b"
QUALIFICATION="11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
def sha(raw): return hashlib.sha256(raw).hexdigest()
def lf(raw): return raw.replace(b"\r\n",b"\n")
def unique(pairs):
    out={}
    for key,value in pairs:
        assert key not in out,key
        out[key]=value
    return out
def manifest(): return json.loads((PACKET/"manifest.json").read_bytes(),object_pairs_hook=unique)
def text(name): return (PACKET/name).read_text(encoding="utf-8-sig")
def results(name): return [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed;",text(name))]
@lru_cache(maxsize=256)
def committed(commit,path):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    return subprocess.check_output(["git","show",f"{commit}:{path}"],cwd=ROOT)
def main():
    m=manifest()
    assert set(m)=={"schema_version","source_commit","source_tree","baseline_source","source_files","unchanged_files","captures","runtime_inputs","reader_tool_sha256","user_copy_refused","admission_claims","verifier_sha256_lf","packet_test_sha256_lf"}
    assert m["schema_version"]==1 and m["source_commit"]==SOURCE and m["source_tree"]==TREE and m["baseline_source"]==BASELINE
    for key,paths,ref in [("source_files",SOURCES,SOURCE),("unchanged_files",UNCHANGED,BASELINE)]:
        rows=m[key]
        assert len(rows)==len(paths) and {r["path"] for r in rows}==paths
        for row in rows:
            raw=lf((ROOT/row["path"]).read_bytes())
            assert sha(raw)==row["sha256_lf"] and raw==lf(committed(ref,row["path"])),row["path"]
            if key=="unchanged_files": assert raw==lf(committed(SOURCE,row["path"])),row["path"]
    assert sha((ROOT/"docs/testing/performance/0.74/qualification-manifest.toml").read_bytes())==QUALIFICATION
    rows=m["captures"]
    assert len(rows)==len(EXITS) and {r["path"] for r in rows}==set(EXITS)
    for row in rows:
        raw=(PACKET/row["path"]).read_bytes()
        assert len(raw)==row["bytes"] and sha(raw)==row["sha256"],row["path"]
        assert row["expected_exit"]==row["observed_exit"]==EXITS[row["path"]]
    assert results("root-input-baseline.log")==[3]
    assert "E0432" in text("root-input-missing-module.log")
    assert results("root-input-linux-final.log")==[12,12,12,3,2,6,16,18,6,12,2]
    assert results("root-input-windows-contracts-final.log")==[6,152]
    linux=text("root-input-linux-final.log")
    assert linux.count("seed=0x0740c075 metadata_mutations=64 independent_observers=4 production_custody=false")==4
    assert linux.count("actual_root_owned_input=observed original_reader_namespaces=matched initial_host_or_job_enrolled=false input_executed=false")==4
    assert "actual_owned_child_SIGKILL=7 retained_attempts=7 retries_refused=7" in linux
    windows=text("root-input-windows-contracts-final.log")
    assert "171 discovered gates, all registered" in windows and "doc-check: OK" in windows and "local, non-promotable" in windows
    for name,marker in [("root-input-preflight.log","reader_source_clean_before=true"),("root-input-postflight.log","reader_source_clean_after=true")]:
        value=text(name)
        assert SOURCE in value and TREE in value and marker in value
        assert "99C8CF65FD709E390727D1441117B85E84FA9FE120CAC9A94E35C5D44FC2FCC7" in value
    actual=text("root-input-actual-runtime.log")
    def stats(label):
        rows=re.findall(rf"^{label} path=(\S+) dev=(\d+) ino=(\d+) size=(\d+) mode=(\d+) uid=(\d+) gid=(\d+) links=(\d+) mtime=(\d+) ctime=(\d+)$",actual,re.M)
        assert len(rows)==3 and len({r[0] for r in rows})==3
        return {row[0]:tuple(map(int,row[1:])) for row in rows}
    before,after=stats("runtime_before"),stats("runtime_after")
    assert before==after and set(before)=={r["path"] for r in LIBRARIES}
    rows=m["runtime_inputs"]
    assert len(rows)==3 and {r["path"] for r in rows}==set(before)
    for selected in LIBRARIES:
        path=selected["path"]; row=next(r for r in rows if r["path"]==path)
        for key in ["path","sha256","bytes","mode"]: assert row[key]==selected[key],key
        observed=before[path]
        assert observed[2:7]==(row["bytes"],int(row["mode"]),0,0,1)
        assert (row["device"],row["inode"])==observed[:2]
        assert row["inventory_only"] is True and row["runtime_selected"] is False
        assert actual.count(row["sha256"])==3
        assert f'root_owned_input_sha256={row["sha256"]} bytes={row["bytes"]} original_reader_context=matched inventory_only=true authenticated_custody=false executed=false signed=false admission=false' in actual
    assert actual.count(TOOL)==2 and m["reader_tool_sha256"]==TOOL
    assert m["user_copy_refused"] is True and "original root-owned input inventory refused" in text("root-input-user-copy-refusal.log")
    post=text("root-input-postflight.log")
    assert "prepared_copy mode=555 uid=1000 gid=1000 links=1 size=1922976" in post
    assert "ca7bc17ad21d9495c48945430289e6626540cffb511ffc206fda42a1b86659ba" in post
    assert "ship admission is closed" in text("root-input-require-ship.log")
    assert set(m["admission_claims"])==FLAGS and all(v is False for v in m["admission_claims"].values())
    assert (PACKET/".gitattributes").read_bytes()==b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof,-blank-at-eol\n"
    for name,key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha(lf((PACKET/name).read_bytes()))==m[key]
    print("Verified original-context numeric-root inventory and user-copy refusal; host/runtime/job admission closed.")
def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    names={"README.md","manifest.json",".gitattributes","verify_packet.py","test_verify_packet.py"}|set(EXITS)
    for name in names:
        assert (PACKET/name).read_bytes()==committed(commit,(PACKET.relative_to(ROOT)/name).as_posix()),name
    print(f"Verified exact committed root-input packet bytes at {commit}.")
if __name__=="__main__":
    main()
    if len(sys.argv)!=1:
        assert len(sys.argv)==3 and sys.argv[1]=="--committed"
        verify_committed(sys.argv[2])
