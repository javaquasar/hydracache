"""Read-only byte integrity checks; no key/host enrollment or performance claim."""
import hashlib
import json
import pathlib


def digest(data):
    return hashlib.sha256(data).hexdigest()


def main():
    packet = pathlib.Path(__file__).resolve().parent
    root = packet.parents[5]
    manifest = json.loads((packet / "manifest.json").read_bytes())
    assert manifest["schema_version"] == 1
    assert not any(manifest["claims"].values()), "no admission claim allowed"
    assert digest((root / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == manifest["qualification_sha256_raw"]
    for category in ("source_files", "frozen_files", "trust_evidence"):
        for entry in manifest[category]:
            relative = pathlib.PurePosixPath(entry["path"])
            assert not relative.is_absolute() and ".." not in relative.parts
            raw = (root / relative).read_bytes().replace(b"\r\n", b"\n")
            assert digest(raw) == entry["sha256_lf"], entry["path"]
    captures = manifest["captures"]
    assert len(captures) == len({entry["path"] for entry in captures})
    assert {entry["path"] for entry in captures} == {p.name for p in packet.glob("*.log")}
    for entry in captures:
        assert pathlib.PurePath(entry["path"]).name == entry["path"]
        raw = (packet / entry["path"]).read_bytes()
        assert len(raw) == entry["bytes"] and digest(raw) == entry["sha256"], entry["path"]
    assert digest(pathlib.Path(__file__).read_bytes().replace(b"\r\n", b"\n")) == manifest["verifier_sha256_lf"]
    policy = (root / "docs/testing/performance/0.74/diagnostic-builder-public-policy.json").read_bytes()
    assert digest(policy) == "dfe81cf770088029e03f04faa4da429ef163583c0ba1b7002b8ec5606e766e22"
    selection = json.loads(policy)
    assert selection["controller_key_hex"] == "c56f89595c45d0656225f1b8915babed046d278dc9ce1908c6bcb6e9c93e2fda"
    assert selection["builder_key_hex"] == "39215088b0102dffb987b937efa9f49a6c5c7704192fa1f16b4e19ccad12c3ba"
    assert selection["controller_key_hex"] != selection["builder_key_hex"]
    print(f"Verified {len(captures)} raw captures, reviewed public pins, source and frozen-input hashes; admission remains closed.")


if __name__ == "__main__":
    main()
