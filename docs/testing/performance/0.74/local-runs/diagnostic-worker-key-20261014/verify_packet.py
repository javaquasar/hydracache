"""Public-only integrity verification; never opens private key storage."""
import hashlib
import json
from pathlib import Path


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    packet = Path(__file__).resolve().parent
    root = packet.parents[5]
    manifest = json.loads((packet / "manifest.json").read_bytes())
    assert manifest["schema_version"] == 1
    for category in ("source_files", "frozen_files", "unchanged_authority_files"):
        for entry in manifest[category]:
            relative = Path(entry["path"])
            assert not relative.is_absolute() and ".." not in relative.parts
            assert sha((root / relative).read_bytes().replace(b"\r\n", b"\n")) == entry["sha256_lf"], entry["path"]
    assert sha((root / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    public_path = root / "docs/testing/performance/0.74/diagnostic-worker-issuer-public.json"
    public_raw = public_path.read_bytes()
    assert sha(public_raw) == manifest["public_selection_sha256_raw"]
    public = json.loads(public_raw)
    assert public_raw == (json.dumps(public, sort_keys=True, separators=(",", ":")) + "\n").encode()
    key = bytes.fromhex(public["public_key_hex"])
    assert len(key) == 32 and sha(key) == public["public_key_sha256"] == manifest["public_key_sha256"]
    assert public["schema_version"] == "diagnostic-worker-issuer-key-selection-074-v1"
    assert public["algorithm"] == "Ed25519" and public["purpose"] == "diagnostic-worker-policy-only"
    assert public["policy_epoch"] is None and public["worker_policy_body_sha256"] is None
    assert not public["worker_policy_signed"] and not public["host_enrolled"] and not public["admission_allowed"]
    old = json.loads((root / "docs/testing/performance/0.74/diagnostic-builder-public-policy.json").read_bytes())
    assert public["public_key_hex"] not in (old["builder_key_hex"], old["controller_key_hex"])
    captures = manifest["captures"]
    assert {entry["path"] for entry in captures} == {p.name for p in packet.glob("*.log")}
    assert len(captures) == len({entry["path"] for entry in captures})
    for entry in captures:
        assert Path(entry["path"]).name == entry["path"]
        raw = (packet / entry["path"]).read_bytes()
        assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"], entry["path"]
        assert b"-----BEGIN PRIVATE KEY-----" not in raw
        assert b"-----BEGIN OPENSSH PRIVATE KEY-----" not in raw
    for name in ("06-real-generation-public.log", "07-real-inspection-public.log", "10-documented-inspection.log", "12-explicit-exec-inspection.log"):
        first = (packet / name).read_text().splitlines()[0]
        assert json.loads(first) == public, name
    assert sha(Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    assert not any(manifest["admission_claims"].values())
    print(f"Verified {len(captures)} public-only captures, issuer selection and unchanged inputs; policy/epoch and admission remain unset.")


if __name__ == "__main__":
    main()
