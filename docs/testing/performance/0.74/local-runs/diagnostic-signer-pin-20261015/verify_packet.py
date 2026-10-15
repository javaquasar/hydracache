"""Read-only selected issuer and unsigned build inventory; not admission."""
import hashlib,json,re,subprocess,sys
from pathlib import Path
from functools import lru_cache
PACKET=Path(__file__).resolve().parent
ROOT=PACKET.parents[5]
SOURCE="16e9420313ef842e19183c7f6f189eb2e10207cf"
BASELINE="08dc602d5ea6cbcc17ef3865e0aeb2e14bf7d1b4"
TREE="9fc430ed1c09934eaa4ebdb61dc282233e63a91b"
SOURCES=set([".gitattributes","tools/long-run-supervisor-074/src/lib.rs","tools/long-run-supervisor-074/src/diagnostic_signer_issuer.rs","tools/long-run-supervisor-074/tests/diagnostic_signer_issuer.rs","crates/xtask/tests/performance_contract_074.rs","docs/GATES.md","docs/COMPAT.md","docs/testing/performance/0.74/diagnostic-signer-issuer-key.md","docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json","docs/testing/performance/0.74/diagnostic-signer-pin-design.md","docs/testing/performance/0.74/diagnostic-signer-pin-plan-20261015.md"])
UNCHANGED=set(["Cargo.toml","Cargo.lock",".cargo/config.toml","tools/long-run-supervisor-074/Cargo.toml","tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs","tools/long-run-supervisor-074/src/diagnostic_signer_build.rs","tools/long-run-supervisor-074/src/diagnostic_signer_bundle.rs","tools/long-run-supervisor-074/src/diagnostic_controller_producer_linux.rs","tools/long-run-supervisor-074/src/bin/diagnostic_controller.rs","tools/long-run-supervisor-074/src/server.rs","scripts/perf/diagnostic_signer_key_074.py","scripts/perf/test_diagnostic_signer_key_074.py","docs/testing/performance/0.74/diagnostic-signer-issuer-public.json","docs/testing/performance/0.74/diagnostic-signer-job-074.yml.disabled","docs/testing/performance/0.74/qualification-manifest.toml","docs/plans/releases.toml","docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md","tools/long-run-supervisor-074/src/diagnostic_signer_runtime_elf.rs","tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs"])
EXPECTED={"signer-pin-baseline.log":0,"signer-pin-missing-module.log":1,"signer-pin-development.log":0,"signer-pin-windows-final.log":0,"signer-pin-linux-final.log":0,"signer-pin-contracts.log":0,"signer-pin-prebuild.log":0,"signer-pin-unsigned-build.log":0,"signer-pin-elf.log":0,"signer-pin-postbuild.log":0,"signer-pin-require-ship.log":1}
FLAGS=set(["artifact_selected","runtime_custody_enrolled","job_enrolled","actual_build_receipt_signed","private_key_accessed","token_requested","github_modified","server_operated","authenticated_original_start","all_writers_revoked","completion_enabled","qualification_allowed","ship_allowed","performance_claim_allowed"])
BINARY="ca7bc17ad21d9495c48945430289e6626540cffb511ffc206fda42a1b86659ba"
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
@lru_cache(maxsize=256)
def committed(commit,path):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    return subprocess.check_output(["git","show",f"{commit}:{path}"],cwd=ROOT)
def text(name): return (PACKET/name).read_text(encoding="utf-8-sig")
def results(name): return [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed;",text(name))]
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
    captures=m["captures"]
    assert len(captures)==len(EXPECTED) and {r["path"] for r in captures}==set(EXPECTED)
    for row in captures:
        assert Path(row["path"]).name==row["path"]
        raw=(PACKET/row["path"]).read_bytes()
        assert len(raw)==row["bytes"] and sha(raw)==row["sha256"],row["path"]
        assert row["expected_outer_exit"]==row["observed_outer_exit"]==EXPECTED[row["path"]]
    assert results("signer-pin-baseline.log")==[9,8]
    assert results("signer-pin-development.log")==[6]
    assert results("signer-pin-windows-final.log")==[6,1,6]
    assert results("signer-pin-linux-final.log")==[6,6,6,1,8,9,6]
    assert results("signer-pin-contracts.log")==[150]
    assert "Ran 24 tests" in text("signer-pin-linux-final.log") and "\nOK\n" in text("signer-pin-linux-final.log")
    assert "E0432" in text("signer-pin-missing-module.log")
    assert "169 discovered gates, all registered" in text("signer-pin-contracts.log")
    assert "local, non-promotable" in text("signer-pin-contracts.log")
    assert "doc-check: OK" in text("signer-pin-contracts.log")
    assert "ship admission is closed" in text("signer-pin-require-ship.log")
    for name,marker in [("signer-pin-prebuild.log","source_clean_before=true"),("signer-pin-postbuild.log","source_clean_after=true")]:
        value=text(name)
        assert SOURCE in value and TREE in value and marker in value
        assert "99C8CF65FD709E390727D1441117B85E84FA9FE120CAC9A94E35C5D44FC2FCC7" in value
    assert "Finished `release` profile [optimized]" in text("signer-pin-unsigned-build.log")
    elf=text("signer-pin-elf.log")
    assert elf.count(BINARY)==2
    assert "binary_bytes=1922976 mode=755 uid=1000 gid=1000 links=2" in elf
    assert "ELF64" in elf and "Advanced Micro Devices X86-64" in elf and "Position-Independent Executable" in elf
    assert re.findall(r"Shared library: \[([^\]]+)\]",elf)==["libgcc_s.so.1","libc.so.6","ld-linux-x86-64.so.2"]
    assert "[Requesting program interpreter: /lib64/ld-linux-x86-64.so.2]" in elf
    assert "GLIBC_2.34" in elf and "(RPATH)" not in elf and "(RUNPATH)" not in elf
    b=m["unsigned_artifact"]
    assert b["sha256"]==BINARY and b["bytes"]==1922976 and b["mode"]=="0755" and b["links"]==2
    assert b["executed"] is False and b["signed"] is False and b["admitted_bundle_input"] is False
    selection=json.loads((ROOT/"docs/testing/performance/0.74/diagnostic-signer-issuer-selection.json").read_bytes())
    assert selection["operator_public_key_selected"] is True
    assert selection["public_key_hex"]=="08f18b0078ad4d754ed615682ec0c9c076109965548d9c674ac606e3625f79ea"
    assert selection["raw_public_sha256"]==sha(bytes.fromhex(selection["public_key_hex"]))
    for key in ["artifact_selected","runtime_custody_enrolled","job_enrolled","production_signing_enabled","server_admission"]:
        assert selection[key] is False,key
    assert m["operator_public_key_selected"] is True
    assert set(m["admission_claims"])==FLAGS and all(v is False for v in m["admission_claims"].values())
    assert (PACKET/".gitattributes").read_bytes()==b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof,-blank-at-eol\n"
    for name,key in [("verify_packet.py","verifier_sha256_lf"),("test_verify_packet.py","packet_test_sha256_lf")]:
        assert sha(lf((PACKET/name).read_bytes()))==m[key]
    print("Verified selected issuer, 11 captures and unsigned non-admitted prebuilt inventory.")
def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    names={"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(r["path"] for r in manifest()["captures"])
    for name in names:
        assert (PACKET/name).read_bytes()==committed(commit,(PACKET.relative_to(ROOT)/name).as_posix()),name
    print(f"Verified exact committed selected-issuer packet bytes at {commit}.")
if __name__=="__main__":
    main()
    if len(sys.argv)!=1:
        assert len(sys.argv)==3 and sys.argv[1]=="--committed"
        verify_committed(sys.argv[2])
