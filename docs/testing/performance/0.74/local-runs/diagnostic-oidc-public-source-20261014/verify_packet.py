"""Verify retained public data and exact local source only; never refetch or enroll trust."""
import base64
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def _verify(packet, root):
    manifest = json.loads((packet / "manifest.json").read_bytes())
    assert manifest["schema_version"] == 1
    for commit in [manifest["source_commit"], manifest["preregister_commit"], *manifest["frozen_lock_sources"]]:
        assert re.fullmatch("[0-9a-f]{40}", commit)
    for category in ("source_files", "frozen_files", "unchanged_files"):
        for entry in manifest[category]:
            relative = Path(entry["path"])
            assert not relative.is_absolute() and not relative.anchor and ".." not in relative.parts
            assert (root / relative).resolve().is_relative_to(root)
            raw = (root / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
            committed = subprocess.check_output(["git", "show", manifest["source_commit"] + ":" + entry["path"]], cwd=root)
            assert committed.replace(b"\r\n", b"\n") == raw, entry["path"]
    assert sha((root / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    for commit in manifest["frozen_lock_sources"]:
        for entry in manifest["frozen_lock_files"]:
            assert entry["path"] in ("Cargo.lock", "tools/get-owner-scheduled-controls-074/Cargo.lock")
            assert sha(subprocess.check_output(["git", "show", commit + ":" + entry["path"]], cwd=root)) == entry["sha256"]
    for group, glob in [("captures", "*.log"), ("public_artifacts", "acquisition-*.json")]:
        entries = manifest[group]
        assert {e["path"] for e in entries} == {p.name for p in packet.glob(glob)}
        assert len(entries) == len({e["path"] for e in entries})
        for entry in entries:
            assert Path(entry["path"]).name == entry["path"]
            raw = (packet / entry["path"]).read_bytes()
            assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"], entry["path"]
    sys.path.insert(0, str(root))
    from scripts.perf import diagnostic_oidc_source_074 as inspector
    raw = (packet / "acquisition-report.json").read_bytes()
    report = json.loads(raw, object_pairs_hook=inspector.unique)
    assert 0 < len(raw) <= 196608
    assert report["schema_version"] == "diagnostic-oidc-public-acquisition-074-v1"
    assert set(report) == {"schema_version", "platform_ca_roster_sha256", "received_before_unix_ns",
                           "received_after_unix_ns", "transport", "discovery_base64", "jwks_base64", "inspection",
                           "authenticated_transport_observed", "key_selected", "trust_config_created",
                           "protected_job_token_verified", "admission_allowed"}
    assert json.dumps(report, sort_keys=True, separators=(",", ":")).encode() + b"\n" == raw
    bodies = [base64.b64decode(report[k], validate=True) for k in ("discovery_base64", "jwks_base64")]
    assert json.dumps(inspector.inspect_sources(*bodies), sort_keys=True) == json.dumps(report["inspection"], sort_keys=True)
    assert all(0 < len(body) <= 65536 for body in bodies)
    start, end = report["received_before_unix_ns"], report["received_after_unix_ns"]
    assert type(start) is int and type(end) is int and 0 < start <= end < 1 << 64
    assert end - start <= 15 * 1_000_000_000
    assert report["authenticated_transport_observed"] is True
    for key in ("key_selected", "trust_config_created", "protected_job_token_verified", "admission_allowed"):
        assert report[key] is False
    assert [item["url"] for item in report["transport"]] == [inspector.ISSUER + "/.well-known/openid-configuration", inspector.JWKS_URI]
    assert all(type(item["status"]) is int and item["status"] == 200 and item["tls_version"] == "TLSv1.3" for item in report["transport"])
    assert all(item["peer_certificate_sha256"] == manifest["observed_peer_certificate_sha256"] for item in report["transport"])
    assert len(report["inspection"]["keys"]) == manifest["observed_key_count"] == 4
    before = json.loads((packet / "clean-runtime-preflight.log").read_bytes())
    after = json.loads((packet / "clean-runtime-postflight.log").read_bytes())
    assert before["ca_roster_sha256"] == after["ca_roster_sha256"] == report["platform_ca_roster_sha256"] == manifest["observed_ca_roster_sha256"]
    assert before["ca_count"] == after["ca_count"] == 69
    assert before["check_hostname"] is True and before["common_name_fallback"] is False
    assert before["keylog_filename"] is None and before["tls_minimum"] == "TLSv1_2"
    assert before["verify_flags"] == 32800
    assert after["synthetic_ca_in_platform_roots"] is False
    for key in ("python_version", "openssl_version"):
        assert before[key] == after[key]
    retained = {e["path"]: e["sha256"] for e in after["runtime_files"]}
    assert all(retained[e["path"]] == e["sha256"] for e in before["runtime_files"])
    for path, digest in manifest["before_binary_hashes"].items():
        assert retained[path] == digest
    status = json.loads((packet / "acquisition-status.json").read_bytes())
    assert status["exit_code"] == 0 and status["stderr"] == "" and status["outer_timed_out"] is False
    assert status["normal_acquire_public_invocations"] == 1 and status["stdout_bytes"] == len(raw)
    assert status["source_commit"] == manifest["source_commit"] and status["preregister_commit"] == manifest["preregister_commit"]
    for key in ("parent_environment_modified", "tls_overrides_passed", "tokens_passed", "key_selected", "token_requested", "trust_installed", "job_dispatched", "host_modified"):
        assert status[key] is False
    assert sha(Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    assert manifest["admission_claims"] and all(v is False for v in manifest["admission_claims"].values())
    print(f"Verified one retained fixed HTTPS acquisition, {len(manifest['captures'])} raw captures, four unselected public keys and frozen/source bindings; no refetch or enrollment.")


def main():
    packet = Path(__file__).resolve().parent
    _verify(packet, packet.parents[5])


if __name__ == "__main__":
    main()
