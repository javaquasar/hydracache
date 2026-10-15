"""Read-only unsigned copy evidence checks, not prebuilt or execution admission."""
import hashlib,json,re,subprocess,sys
from functools import lru_cache
from pathlib import Path,PurePosixPath
if not __debug__:
    raise SystemExit("Evidence checks require Python without -O")
PACKET=Path(__file__).resolve().parent
ROOT=PACKET.parents[5]
SOURCE="6148aa9b11d5768c74bd31e4175c3332d3715ada"
TREE="3d7348be70e7c7f4171ee29ff4e6ea36998bbce4"
BASELINE="a7819987b3a7f5b4efea7925abeec1b29b9995a3"
SOURCES=set(["tools/long-run-supervisor-074/src/lib.rs","tools/long-run-supervisor-074/src/diagnostic_prebuilt_copy.rs","tools/long-run-supervisor-074/src/diagnostic_prebuilt_copy_tests.rs","tools/long-run-supervisor-074/tests/diagnostic_prebuilt_copy.rs","tools/long-run-supervisor-074/examples/prepare_unsigned_prebuilt.rs","crates/xtask/tests/performance_contract_074.rs","docs/GATES.md","docs/COMPAT.md","docs/testing/gated-test-registry.toml","docs/testing/performance/0.74/diagnostic-prebuilt-copy-contract.toml","docs/testing/performance/0.74/diagnostic-prebuilt-copy-design.md","docs/testing/performance/0.74/diagnostic-prebuilt-copy-plan-20261015.md"])
UNCHANGED=set(["Cargo.toml","Cargo.lock","tools/long-run-supervisor-074/Cargo.toml",".github/workflows/ci.yml","tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs","tools/long-run-supervisor-074/src/diagnostic_signer_issuer.rs","tools/long-run-supervisor-074/src/diagnostic_signer_bundle.rs","tools/long-run-supervisor-074/src/diagnostic_signer_executable.rs","tools/long-run-supervisor-074/src/diagnostic_signer_build.rs","tools/long-run-supervisor-074/src/diagnostic_signer_runtime_elf.rs","tools/long-run-supervisor-074/src/diagnostic_controller_producer_linux.rs","tools/long-run-supervisor-074/src/bin/diagnostic_controller.rs","tools/long-run-supervisor-074/src/server.rs","scripts/perf/diagnostic_signer_key_074.py","docs/testing/performance/0.74/diagnostic-signer-issuer-public.json","docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json","docs/testing/performance/0.74/diagnostic-signer-job-074.yml.disabled","docs/testing/performance/0.74/qualification-manifest.toml","docs/plans/releases.toml","docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"])
EXITS={"prebuilt-copy-baseline.log":[0,0],"prebuilt-copy-missing-module.log":[101,101],"prebuilt-copy-metadata-only.log":[0,101],"prebuilt-copy-contracts-unregistered.log":[0,1],"prebuilt-copy-contracts-format.log":[0,1],"prebuilt-copy-linux-final.log":[0,0],"prebuilt-copy-windows-final.log":[0,0],"prebuilt-copy-contracts.log":[0,0],"prebuilt-copy-preflight.log":[0,0],"prebuilt-copy-actual.log":[0,0],"prebuilt-copy-postflight.log":[0,0],"prebuilt-copy-require-ship.log":[1,1]}
FLAGS=set(["authenticated_original_prebuilt_custody","protected_ancestry_or_account_runtime_job_enrolled","foreign_writer_revoked_or_continuous_immutability","power_loss_or_storage_durability_proved","actual_prepared_binary_executed_or_signed","private_key_or_token_read","github_or_server_operated","authenticated_original_start","whole_writer_revocation","completion_or_later_cells_enabled","qualification_allowed","ship_allowed","performance_claim_allowed"])
BINARY="ca7bc17ad21d9495c48945430289e6626540cffb511ffc206fda42a1b86659ba"
TOOL="aa6e84b79e8fde4aafb6d2d6dd10647fbbf0126c2c1ff14e6c78b1add31d7847"
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
        assert [row["expected_exit"],row["observed_exit"]]==EXITS[row["path"]]
    assert results("prebuilt-copy-baseline.log")==[10]
    assert "E0432" in text("prebuilt-copy-missing-module.log")
    assert results("prebuilt-copy-metadata-only.log")==[16,16]
    assert "test result: FAILED. 15 passed; 1 failed;" in text("prebuilt-copy-metadata-only.log")
    assert "prebuilt_copy_source_changes_during_copy_refuse_without_source_repair" in text("prebuilt-copy-metadata-only.log")
    assert "unregistered cfg-gated target" in text("prebuilt-copy-contracts-unregistered.log")
    assert "Diff in " in text("prebuilt-copy-contracts-format.log")
    assert results("prebuilt-copy-linux-final.log")==[16,16,16,3,6,18,3,16,3]
    assert results("prebuilt-copy-windows-final.log")==[6]
    assert results("prebuilt-copy-contracts.log")==[151]
    linux=text("prebuilt-copy-linux-final.log")
    assert linux.count("actual_owned_child_SIGKILL=7 retained_attempts=7 retries_refused=7")==4
    assert linux.count("retained_foreign_writer_survives_0555=true inventory_is_not_revocation=true")==4
    assert linux.count("source_content_rehash_refuses=true")==4
    assert "170 discovered gates, all registered" in text("prebuilt-copy-contracts.log")
    assert "doc-check: OK" in text("prebuilt-copy-contracts.log")
    assert "local, non-promotable" in text("prebuilt-copy-contracts.log")
    assert "ship admission is closed" in text("prebuilt-copy-require-ship.log")
    for name,marker in [("prebuilt-copy-preflight.log","preparer_source_clean_before=true"),("prebuilt-copy-postflight.log","preparer_source_clean_after=true")]:
        value=text(name)
        assert SOURCE in value and TREE in value and marker in value
        assert "99C8CF65FD709E390727D1441117B85E84FA9FE120CAC9A94E35C5D44FC2FCC7" in value
    actual=text("prebuilt-copy-actual.log")
    def stat(label):
        pattern=rf"^{label} dev=(\d+) ino=(\d+) bytes=(\d+) mode=(\d+) uid=(\d+) gid=(\d+) links=(\d+) mtime=(\d+) ctime=(\d+)$"
        rows=re.findall(pattern,actual,re.M)
        assert len(rows)==1,label
        return tuple(map(int,rows[0]))
    before,after,copy=stat("source_before"),stat("source_after"),stat("copy")
    assert before==after and before[2:7]==(1922976,755,1000,1000,2)
    assert copy[:2]!=before[:2] and copy[2:7]==(1922976,555,1000,1000,1)
    assert "copy_directory mode=555 uid=1000 gid=1000 links=2" in actual
    assert actual.count(BINARY)==4 and actual.count(TOOL)==2
    assert "copy_inventory_only=true authenticated_custody=false executed=false signed=false admission=false" in actual
    b=m["unsigned_copy"]
    assert b["sha256"]==BINARY and b["bytes"]==1922976 and b["mode"]=="0555" and b["links"]==1
    assert tuple(b["source_identity"])==before[:2] and tuple(b["copy_identity"])==copy[:2]
    assert b["original_binary_source_commit"]=="16e9420313ef842e19183c7f6f189eb2e10207cf"
    assert b["preparer_tool_sha256"]==TOOL and b["inventory_only"] is True
    assert b["executed"] is False and b["signed"] is False and b["admitted_bundle_input"] is False
    assert b["path"] in actual and "copy_parent="+str(PurePosixPath(b["path"]).parent.parent) in actual
    assert set(m["admission_claims"])==FLAGS and all(v is False for v in m["admission_claims"].values())
    assert (PACKET/".gitattributes").read_bytes()==b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof,-blank-at-eol\n"
    for name,key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha(lf((PACKET/name).read_bytes()))==m[key]
    print("Verified unsigned distinct-inode copy, seven-cut SIGKILL and retained negative observations; admission closed.")
def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    names={"README.md","manifest.json",".gitattributes","verify_packet.py","test_verify_packet.py"} | set(EXITS)
    for name in names:
        assert (PACKET/name).read_bytes()==committed(commit,(PACKET.relative_to(ROOT)/name).as_posix()),name
    print(f"Verified exact committed unsigned-copy packet bytes at {commit}.")
if __name__=="__main__":
    main()
    if len(sys.argv)!=1:
        assert len(sys.argv)==3 and sys.argv[1]=="--committed"
        verify_committed(sys.argv[2])
