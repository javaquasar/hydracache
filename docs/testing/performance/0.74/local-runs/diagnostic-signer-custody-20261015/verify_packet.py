"""Read-only public signer issuer preparation evidence, never a trust loader."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
from functools import lru_cache
PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "2b90c66593e6fabbec407f1aea743d4987915e38"
BASELINE = "0b6eff00cad0c8497987523a0016e0cc42ed3a76"
PREREG = "d408c294f26d08922d914fdeab69153bcd197c6a"
PUBLIC = "docs/testing/performance/0.74/diagnostic-signer-issuer-public.json"
RAW_KEY = "08f18b0078ad4d754ed615682ec0c9c076109965548d9c674ac606e3625f79ea"
FINGERPRINT = "0c22ba7d6c6d16b334e4b1d4a0ce161117800135a977aa30020906caa70af884"
QUALIFICATION = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
SOURCES = set([
  "scripts/perf/diagnostic_signer_key_074.py",
  "scripts/perf/test_diagnostic_signer_key_074.py",
  "tools/long-run-supervisor-074/examples/verify_signer_issuer_public.rs",
  "crates/xtask/tests/performance_contract_074.rs",
  "docs/GATES.md",
  "docs/COMPAT.md",
  "docs/testing/performance/0.74/diagnostic-signer-custody-design.md",
  "docs/testing/performance/0.74/diagnostic-signer-job-074.yml.disabled",
  "docs/testing/performance/0.74/diagnostic-signer-custody-plan-20261015.md"
])
UNCHANGED = set([
  "Cargo.toml",
  "Cargo.lock",
  "tools/long-run-supervisor-074/Cargo.toml",
  "tools/long-run-supervisor-074/src/lib.rs",
  "tools/long-run-supervisor-074/src/server.rs",
  "tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs",
  "tools/long-run-supervisor-074/src/diagnostic_signer_build.rs",
  "tools/long-run-supervisor-074/src/diagnostic_signer_bundle.rs",
  "tools/long-run-supervisor-074/src/diagnostic_controller_producer_linux.rs",
  "tools/long-run-supervisor-074/src/bin/diagnostic_controller.rs",
  "scripts/perf/diagnostic_worker_key_074.py",
  "scripts/perf/test_diagnostic_worker_key_074.py",
  "docs/testing/performance/0.74/diagnostic-worker-issuer-public.json",
  "docs/testing/performance/0.74/diagnostic-worker-issuer-key.md",
  "docs/testing/performance/0.74/qualification-manifest.toml",
  "docs/plans/releases.toml",
  "docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md",
  ".github/workflows/diagnostic-builder-074.yml",
  ".github/workflows/performance-long-run-qualification-074.yml",
  "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs",
  "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs",
  "tools/long-run-supervisor-074/src/diagnostic_signer_creator_task.rs",
  "tools/long-run-supervisor-074/src/diagnostic_signer_creator_loss_fixture.rs",
  "tools/long-run-supervisor-074/src/diagnostic_signer_thread_exit_fixture.rs",
  "docs/testing/performance/0.74/diagnostic-signer-bootstrap-decision.md"
])
EXPECTED = {
  "signer-custody-worker-windows-baseline.log": 0,
  "signer-custody-baseline.log": 0,
  "signer-custody-missing-helper.log": 1,
  "signer-custody-windows-tests.log": 0,
  "signer-custody-linux-tests.log": 0,
  "signer-custody-windows-rust.log": 0,
  "signer-custody-windows-tests-final.log": 0,
  "signer-custody-contracts.log": 0,
  "signer-custody-linux-final.log": 0,
  "signer-custody-windows-final.log": 0,
  "signer-custody-static.log": 0,
  "signer-custody-generation.json": 0,
  "signer-custody-inspection.json": 0,
  "signer-custody-public-proof-linux.log": 0,
  "signer-custody-public-proof-windows.log": 0,
  "signer-custody-require-ship.log": 1,
  "signer-custody-static-final.log": 0
}
ADMISSION = set([
  "public_pin_approved",
  "production_trust_enrolled",
  "signer_build_receipt_signed",
  "workflow_installed",
  "admission_allowed",
  "old_private_keys_accessed",
  "raw_private_material_output",
  "private_key_exported",
  "github_modified",
  "server_operated",
  "product_workload_run",
  "qualification_allowed",
  "ship_allowed",
  "authenticated_original_worker_start",
  "all_writers_revoked",
  "completion_enabled"
])
def sha(raw):
    return hashlib.sha256(raw).hexdigest()
def lf(raw):
    return raw.replace(b"\r\n", b"\n")
def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":"))+"\n").encode()
def unique(pairs):
    out = {}
    for key,value in pairs:
        assert key not in out, key
        out[key] = value
    return out
def manifest():
    return json.loads((PACKET/"manifest.json").read_bytes(), object_pairs_hook=unique)
@lru_cache(maxsize=256)
def committed(commit,path):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    return subprocess.check_output(["git","show",f"{commit}:{path}"],cwd=ROOT)
def text(name):
    return (PACKET/name).read_text(encoding="utf-8-sig")
def results(name):
    return [int(n) for n in re.findall(r"test result: ok\. (\d+) passed; 0 failed;",text(name))]
def main():
    m = manifest()
    assert m["schema_version"] == 1 and m["packet_id"] == "diagnostic-signer-custody-20261015"
    assert m["source_commit"] == SOURCE and m["baseline_source"] == BASELINE and m["preregistration"] == PREREG
    for key, paths, ref in [("source_files",SOURCES,SOURCE),("unchanged_files",UNCHANGED,BASELINE)]:
        rows = m[key]
        assert len(rows) == len(paths) and {r["path"] for r in rows} == paths
        for row in rows:
            path = row["path"]
            current = lf((ROOT/path).read_bytes())
            assert sha(current) == row["sha256_lf"] and current == lf(committed(ref,path)), path
            if key == "unchanged_files":
                assert current == lf(committed(SOURCE,path)),path
    qualification = "docs/testing/performance/0.74/qualification-manifest.toml"
    assert m["qualification_sha256_raw"] == sha((ROOT/qualification).read_bytes()) == QUALIFICATION
    assert sha(committed(BASELINE,qualification)) == QUALIFICATION
    assert (PACKET/".gitattributes").read_bytes() == b"* -text\n*.log whitespace=cr-at-eol,-blank-at-eof\n"
    captures = m["captures"]
    assert len(captures) == len(EXPECTED) and {r["path"] for r in captures} == set(EXPECTED)
    for row in captures:
        name = row["path"]
        assert Path(name).name == name
        raw = (PACKET/name).read_bytes()
        assert len(raw) == row["bytes"] and sha(raw) == row["sha256"],name
        assert row["observed_outer_exit"] == row["expected_outer_exit"] == EXPECTED[name]
    for name,expected in [("signer-custody-baseline.log",[9]),("signer-custody-windows-rust.log",[3,9]),
        ("signer-custody-linux-final.log",[3,9,8,3]),("signer-custody-windows-final.log",[3,3]),
        ("signer-custody-contracts.log",[149])]:
        assert results(name) == expected,name
    linux = text("signer-custody-linux-final.log")
    assert linux.count("Ran 23 tests") == 3 and linux.count("\nOK\n") == 3
    windows = text("signer-custody-windows-tests-final.log")
    assert "Ran 23 tests" in windows and "OK (skipped=14)" in windows
    assert "Ran 18 tests" in text("signer-custody-worker-windows-baseline.log")
    assert "ImportError" in text("signer-custody-missing-helper.log")
    assert "ship admission is closed" in text("signer-custody-require-ship.log")
    assert "static_and_scoped_format=passed" in text("signer-custody-static-final.log")
    assert "169 discovered gates, all registered" in text("signer-custody-static-final.log")
    assert "local, non-promotable" in text("signer-custody-static-final.log")
    public = (ROOT/PUBLIC).read_bytes()
    assert sha(public) == m["public_record_sha256_raw"]
    assert public == (PACKET/"signer-custody-generation.json").read_bytes() == (PACKET/"signer-custody-inspection.json").read_bytes()
    p = json.loads(public,object_pairs_hook=unique)
    assert public == canonical(p)
    assert p["public_key_hex"] == RAW_KEY
    assert p["public_key_sha256"] == sha(bytes.fromhex(RAW_KEY)) == FINGERPRINT
    assert p["schema_version"] == "diagnostic-signer-build-issuer-selection-074-v1"
    assert p["purpose"] == "diagnostic-signer-build-receipt-only" and p["algorithm"] == "Ed25519"
    assert p["storage_uid"] == 1000 and p["directory_mode"] == "0700" and p["private_key_mode"] == "0600"
    assert p["storage_format"] == "unencrypted-pkcs8-pem" and p["signer_build_statement_sha256"] is None
    assert p["selfcheck_message_hex"] == b"hydracache-diagnostic-signer-build-key-selfcheck-074-v1\0not-a-build-receipt\n".hex()
    assert re.fullmatch("[0-9a-f]{128}",p["selfcheck_signature_hex"])
    for key in ["public_pin_approved","production_trust_enrolled","signer_build_receipt_signed","workflow_installed","admission_allowed"]:
        assert p[key] is False,key
    for name in ["signer-custody-public-proof-linux.log","signer-custody-public-proof-windows.log"]:
        assert f"public_signer_selfcheck=verified raw_public_sha256={FINGERPRINT} pin_approved=false production_enrolled=false" in text(name)
    assert "600 1000 1000 1 regular file" in text("signer-custody-public-proof-linux.log")
    assert m["new_key_generated_locally"] is True and m["new_key_used_for_fixed_selfcheck_only"] is True
    assert set(m["admission_claims"]) == ADMISSION and all(v is False for v in m["admission_claims"].values())
    assert sha(lf((PACKET/"verify_packet.py").read_bytes())) == m["verifier_sha256_lf"]
    assert sha(lf((PACKET/"test_verify_packet.py").read_bytes())) == m["packet_test_sha256_lf"]
    print(f"Verified {len(captures)} local signer custody captures; fingerprint approval and production remain closed.")
def verify_committed(commit):
    assert re.fullmatch("[0-9a-f]{40}",commit)
    names = {"manifest.json","README.md",".gitattributes","verify_packet.py","test_verify_packet.py"}
    names.update(r["path"] for r in manifest()["captures"])
    for name in names:
        assert (PACKET/name).read_bytes() == committed(commit,(PACKET.relative_to(ROOT)/name).as_posix()),name
    assert (ROOT/PUBLIC).read_bytes() == committed(commit,PUBLIC)
    print(f"Verified exact committed custody packet and public record at {commit}.")
if __name__ == "__main__":
    main()
    if len(sys.argv) != 1:
        assert len(sys.argv) == 3 and sys.argv[1] == "--committed"
        verify_committed(sys.argv[2])
