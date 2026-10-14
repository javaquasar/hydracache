"""Read-only local signer model evidence, not trust enrollment or execution."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
ADMISSION = {
    "production_signer_issuer_selected", "actual_signer_build_authenticated",
    "original_executable_or_runtime_custody_authenticated", "bootstrap_authorized",
    "private_key_or_token_read", "trust_installed", "protected_job_dispatched",
    "original_start_authorized", "whole_writer_revocation", "later_cells_enabled",
    "server_allowed", "qualification_allowed", "ship_allowed", "performance_claim_allowed",
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-signer-build-20261014"
    assert manifest["baseline_source"] == "99beb76dbcb8d10c4d4c9d0b46ca9cbfa3b68d22"
    assert manifest["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    for group, commit in [("source_files", manifest["source_commit"]),
                          ("unchanged_files", manifest["baseline_source"])]:
        entries = manifest[group]
        assert entries and len(entries) == len({e["path"] for e in entries})
        for entry in entries:
            relative = Path(entry["path"])
            assert not relative.anchor and ".." not in relative.parts
            assert (ROOT / relative).resolve().is_relative_to(ROOT)
            raw = (ROOT / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
            assert raw == committed(commit, entry["path"]).replace(b"\r\n", b"\n"), entry["path"]
    qualification = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(qualification) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    for commit in manifest["frozen_lock_sources"]:
        for entry in manifest["frozen_lock_files"]:
            assert entry["path"] in ("Cargo.lock", "tools/get-owner-scheduled-controls-074/Cargo.lock")
            assert sha(committed(commit, entry["path"])) == entry["sha256"]
    captures = manifest["captures"]
    assert len(captures) == len({c["path"] for c in captures})
    assert {c["path"] for c in captures} == {p.name for p in PACKET.glob("*.log")}
    for capture in captures:
        assert Path(capture["path"]).name == capture["path"]
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["observed_outer_exit"]) is int
        assert type(capture["expected_outer_exit"]) is int
        assert capture["observed_outer_exit"] == capture["expected_outer_exit"]
    linux = (PACKET / "signer-build-linux-tests.log").read_text()
    assert linux.count("test result: ok. 9 passed; 0 failed") == 3
    assert linux.count("seed=0x07403b22 content_mutations=64 independent_models=4 actual_build_or_bootstrap=false") == 3
    for count in (12, 29, 8, 6, 91):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    assert "test result: ok. 18 passed; 0 failed" in (PACKET / "signer-build-oidc-regression.log").read_text()
    for count in (6, 134, 13):
        assert f"test result: ok. {count} passed; 0 failed" in (PACKET / "signer-build-root-contracts.log").read_text()
    assert "error[E0432]" in (PACKET / "signer-build-missing-api.log").read_text()
    assert "ship admission is closed" in (PACKET / "signer-build-require-ship.log").read_text()
    assert "Ran 25 tests" in (PACKET / "signer-python-regression.log").read_text()
    formatted = json.loads((PACKET / "signer-format-result.log").read_bytes())
    assert type(formatted["observed_exit"]) is int and formatted["observed_exit"] == 0
    assert formatted["source_commit"] == manifest["source_commit"]
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for file, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / file).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified signer model source, {len(captures)} captures and unchanged frozen inputs; bootstrap/start closed.")


if __name__ == "__main__":
    main()
