"""Read-only source/capture verification; no trust selection, network or credential IO."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-oidc-roster-20261014"
    for group, commit in [("source_files", manifest["source_commit"]),
                          ("unchanged_files", manifest["baseline_source"])]:
        entries = manifest[group]
        assert entries and len(entries) == len({e["path"] for e in entries})
        for entry in entries:
            path = Path(entry["path"])
            assert not path.anchor and ".." not in path.parts
            assert (ROOT / path).resolve().is_relative_to(ROOT)
            raw = (ROOT / path).read_bytes().replace(b"\r\n", b"\n")
            assert digest(raw) == entry["sha256_lf"], entry["path"]
            assert raw == committed(commit, entry["path"]).replace(b"\r\n", b"\n"), entry["path"]
    assert digest((ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    for commit in manifest["frozen_lock_sources"]:
        for entry in manifest["frozen_lock_files"]:
            assert entry["path"] in ("Cargo.lock", "tools/get-owner-scheduled-controls-074/Cargo.lock")
            assert digest(committed(commit, entry["path"])) == entry["sha256"]
    captures = manifest["captures"]
    assert len(captures) == len({e["path"] for e in captures})
    assert {e["path"] for e in captures} == {p.name for p in PACKET.glob("*.log")}
    for entry in captures:
        assert Path(entry["path"]).name == entry["path"]
        raw = (PACKET / entry["path"]).read_bytes()
        assert len(raw) == entry["bytes"] and digest(raw) == entry["sha256"], entry["path"]
        if entry["expected_outer_exit"] != entry["observed_outer_exit"]:
            assert entry["path"] in ("oidc-roster-baseline.log", "oidc-roster-root-contracts.log", "oidc-roster-linux-regression-lint.log")
            assert entry["negative_result"] and entry["observed_outer_exit"] != 0
    repeats = (PACKET / "oidc-roster-focused-repeats.log").read_text()
    assert repeats.count("test result: ok. 18 passed; 0 failed") == 3
    assert repeats.count("seed=0x07402a11 membership_mutations=64 independent_bindings=4 actual_issuer_enrolled=false") == 3
    regression = (PACKET / "oidc-roster-linux-regression-lint.log").read_text()
    for count in (49, 5, 85):
        assert f"test result: ok. {count} passed; 0 failed" in regression
    root_tests = (PACKET / "oidc-roster-root-contracts-corrected.log").read_text()
    for count in (6, 132, 13):
        assert f"test result: ok. {count} passed; 0 failed" in root_tests
    assert manifest["admission_claims"] and all(v is False for v in manifest["admission_claims"].values())
    assert digest(Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    assert digest((Path(__file__).parent / "test_verify_packet.py").read_bytes().replace(b"\r\n", b"\n")) == manifest["packet_test_sha256_lf"]
    print(f"Verified bounded OIDC v2 source, {len(captures)} raw captures, v1/frozen bindings and closed admission; no refresh or enrollment.")


if __name__ == "__main__":
    main()
