"""Read-only source and raw-evidence integrity; never opens private keys or a host."""
import hashlib
import json
from pathlib import Path
import subprocess


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    packet = Path(__file__).resolve().parent
    root = packet.parents[5]
    manifest = json.loads((packet / "manifest.json").read_bytes())
    assert manifest["schema_version"] == 1
    for category in ("source_files", "frozen_files", "unchanged_files"):
        for entry in manifest[category]:
            relative = Path(entry["path"])
            assert not relative.anchor and not relative.is_absolute() and ".." not in relative.parts
            assert (root / relative).resolve().is_relative_to(root)
            raw = (root / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
            if category == "unchanged_files":
                old = subprocess.check_output(["git", "show", manifest["baseline_source"] + ":" + entry["path"]], cwd=root)
                assert old.replace(b"\r\n", b"\n") == raw, entry["path"]
    assert sha((root / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    captures = manifest["captures"]
    assert manifest["capture_count"] == len(captures) == len({entry["path"] for entry in captures})
    assert {entry["path"] for entry in captures} == {p.name for p in packet.glob("*.log")}
    for entry in captures:
        assert Path(entry["path"]).name == entry["path"]
        raw = (packet / entry["path"]).read_bytes()
        assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"], entry["path"]
    assert sha(Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    assert manifest["admission_claims"] and all(value is False for value in manifest["admission_claims"].values())
    print(f"Verified {len(captures)} raw captures and source/frozen/unchanged inputs; enrollment and execution remain closed.")


if __name__ == "__main__":
    main()
