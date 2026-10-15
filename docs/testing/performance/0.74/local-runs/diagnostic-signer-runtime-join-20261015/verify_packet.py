"""Offline local consistency evidence, never production trust or admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "7e6d75fb4a67dbf4167d74d97eeee3c3efcff368"
TREE = "1bcad596cdcd977b6c0ae85e9ce8c2305d1f1f60"
PREREG = "751d7803b8cb99119d7f10d2b635d04e0328ae10"
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml":
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
}
COUNTS = {
    "baseline-windows.log": [9, 6], "baseline-linux.log": [12],
    "windows-final.log": [154, 9, 6, 6], "linux-final.log": [8, 8, 13, 2, 9, 6, 6, 1, 5],
    "linux-msrv.log": [8], "windows-msrv.log": [6],
    "linux-regression.log": [10, 10, 6], "root-repeat.log": [154],
    "registry-development-failure.log": [1],
}
EXITS = {name: 0 for name in COUNTS}
EXITS.update({"registry-development-failure.log": 1, "missing-api.log": 101, "static-final.log": 0})
CLOSED = {"independent_runtime_custody", "actual_production_receipt_checked",
          "real_private_key_or_token_read", "server_or_github_mutated",
          "all_writers_revoked", "original_start_authenticated", "qualification_allowed", "ship_allowed"}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def unique(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, key
        result[key] = value
    return result


def git(*args):
    env = os.environ.copy()
    marker = ROOT / ".git"
    if os.name != "nt" and marker.is_file():
        path = marker.read_text().strip().removeprefix("gitdir: ")
        if re.match(r"^[A-Za-z]:/", path):
            env["GIT_DIR"] = "/mnt/" + path[0].lower() + path[2:]
            env["GIT_WORK_TREE"] = str(ROOT)
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)


def verify():
    m = json.loads((PACKET / "manifest.json").read_bytes(), object_pairs_hook=unique)
    assert m["schema_version"] == "diagnostic-signer-runtime-join-packet-074-v1"
    assert (m["source_commit"], m["source_tree"], m["preregistration"]) == (SOURCE, TREE, PREREG)
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert set(m["closed_claims"]) == CLOSED and all(v is False for v in m["closed_claims"].values())
    assert m["frozen_inputs"] == FROZEN
    for path, digest in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == digest
    expected_sources = set(git("diff", "--name-only", PREREG, SOURCE).decode().splitlines())
    assert set(m["implementation_files"]) == expected_sources and len(expected_sources) == 16
    for path, digest in m["implementation_files"].items():
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == digest and (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original, path
    expected_files = set(EXITS) | {"README.md", "verify_packet.py", "test_verify_packet.py"}
    assert set(m["files"]) == expected_files
    for name, row in m["files"].items():
        assert Path(name).name == name
        raw = (PACKET / name).read_bytes()
        assert row["bytes"] == len(raw) and row["sha256"] == sha(raw), name
        if name in EXITS:
            assert type(row["outer_exit"]) is int and row["outer_exit"] == EXITS[name]
    for name, expected in COUNTS.items():
        counts = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", (PACKET / name).read_text())
        assert [int(n) for n in counts] == expected, name
    assert "error[E0432]" in (PACKET / "missing-api.log").read_text()
    assert "unregistered cfg-gated target" in (PACKET / "registry-development-failure.log").read_text()
    final = (PACKET / "static-final.log").read_text()
    assert "172 discovered gates, all registered" in final
    assert "local, non-promotable" in final and "expected_ship_refusal_exit=1" in final
    print("Verified original GNU runtime join packet; independent custody and execution remain closed.")


if __name__ == "__main__":
    verify()
