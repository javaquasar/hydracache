"""Read-only retained producer evidence; no key/token/network/host enrollment."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-retained-producer-20261014"
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
    assert sha((ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    for commit in manifest["frozen_lock_sources"]:
        for entry in manifest["frozen_lock_files"]:
            assert entry["path"] in ("Cargo.lock", "tools/get-owner-scheduled-controls-074/Cargo.lock")
            assert sha(committed(commit, entry["path"])) == entry["sha256"]
    entries = manifest["captures"]
    assert len(entries) == len({e["path"] for e in entries})
    assert {e["path"] for e in entries} == {p.name for p in PACKET.glob("*.log")}
    for entry in entries:
        assert Path(entry["path"]).name == entry["path"]
        raw = (PACKET / entry["path"]).read_bytes()
        assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"], entry["path"]
        assert entry["expected_outer_exit"] == entry["observed_outer_exit"]
    linux = (PACKET / "retained-producer-linux-final.log").read_text()
    assert linux.count("test result: ok. 12 passed; 0 failed") == 3
    assert linux.count("seed=0x07403b21 retained_body_cases=32 independent_preparations=4 actual_token_or_signer_custody=false") == 3
    for count in (6, 88):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    root = (PACKET / "retained-producer-root-contracts.log").read_text()
    for count in (6, 133, 13):
        assert f"test result: ok. {count} passed; 0 failed" in root
    assert manifest["admission_claims"] and all(v is False for v in manifest["admission_claims"].values())
    assert sha(Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    assert sha((Path(__file__).parent / "test_verify_packet.py").read_bytes().replace(b"\r\n", b"\n")) == manifest["packet_test_sha256_lf"]
    print(f"Verified retained producer source, {len(entries)} raw captures, original/frozen bindings and closed token/bootstrap/start admission.")


if __name__ == "__main__":
    main()
