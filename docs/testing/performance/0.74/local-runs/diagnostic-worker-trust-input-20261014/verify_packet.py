"""Read-only source/evidence integrity; no host, key or epoch enrollment."""
import hashlib
import json
from pathlib import Path
import subprocess


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    packet = Path(__file__).resolve().parent
    root = packet.parents[5]
    manifest = json.loads((packet / "manifest.json").read_bytes())
    assert manifest["schema_version"] == 1
    for category in ("source_files", "frozen_files", "unchanged_files"):
        for entry in manifest[category]:
            relative = Path(entry["path"])
            assert not relative.is_absolute() and ".." not in relative.parts
            raw = (root / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
    assert sha((root / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    captures = manifest["captures"]
    assert len(captures) == len({entry["path"] for entry in captures})
    assert {entry["path"] for entry in captures} == {p.name for p in packet.glob("*.log")}
    for entry in captures:
        assert Path(entry["path"]).name == entry["path"]
        raw = (packet / entry["path"]).read_bytes()
        assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"], entry["path"]
    relative = "tools/long-run-supervisor-074/src/diagnostic_worker_policy.rs"
    previous = subprocess.check_output(["git", "show", manifest["baseline_source"] + ":" + relative], cwd=root).replace(b"\r\n", b"\n")
    current = (root / relative).read_bytes().replace(b"\r\n", b"\n")
    declaration = b'#[cfg(target_os = "linux")]\n#[path = "diagnostic_worker_trust_input.rs"]\npub mod trusted_input;\n\n'
    assert current.count(declaration) == 1 and current.replace(declaration, b"", 1) == previous
    assert sha(Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    assert not any(manifest["admission_claims"].values())
    print(f"Verified {len(captures)} raw captures, source/frozen hashes and unchanged old policy bodies; admission remains closed.")


if __name__ == "__main__":
    main()
