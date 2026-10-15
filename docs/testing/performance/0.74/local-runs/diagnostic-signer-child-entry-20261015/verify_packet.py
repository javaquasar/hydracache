"""Offline local consistency evidence, never production trust or admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "41cb929af58ed8e81d6083710832aa3513ab9bfe"
TREE = "9242b7358b381aba89bf8628e97a37686c9d8df5"
PREREG = "7fd7a45608f9175be81c8ddb35b060d0a87080d4"
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml":
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
}
COUNTS = {
    "baseline-windows.log": [
        7
    ],
    "baseline-linux.log": [
        13
    ],
    "missing-api.log": [],
    "kernel-initial.log": [
        5,
        7
    ],
    "unit-initial.log": [
        3
    ],
    "windows-final.log": [
        7
    ],
    "windows-msrv.log": [
        7
    ],
    "clippy-ice-partial.log": [],
    "clippy-diagnostic-failed.log": [],
    "registry-failed.log": [],
    "regressions.log": [
        13,
        11,
        13
    ],
    "kernel-failed.log": [],
    "msrv-failed.log": [
        3
    ],
    "stage-diagnostic.log": [
        5
    ],
    "diagnostic-repeats-failed.log": [
        5
    ],
    "fd-diagnosis-failed.log": [
        5
    ],
    "linux-repeats.log": [
        5,
        7,
        5,
        7,
        5,
        7,
        5,
        7,
        5,
        7,
        5,
        7,
        5,
        7,
        5,
        7
    ],
    "linux-msrv.log": [
        3,
        5,
        7
    ],
    "root-final.log": [
        156
    ],
    "linux-final.log": [
        3
    ],
    "static-final.log": []
}
EXITS = {
    "baseline-windows.log": 0,
    "baseline-linux.log": 0,
    "missing-api.log": 101,
    "kernel-initial.log": 0,
    "unit-initial.log": 0,
    "windows-final.log": 0,
    "windows-msrv.log": 0,
    "clippy-ice-partial.log": 1,
    "clippy-diagnostic-failed.log": 1,
    "registry-failed.log": 1,
    "regressions.log": 0,
    "kernel-failed.log": 101,
    "msrv-failed.log": 101,
    "stage-diagnostic.log": 0,
    "diagnostic-repeats-failed.log": 101,
    "fd-diagnosis-failed.log": 101,
    "linux-repeats.log": 0,
    "linux-msrv.log": 0,
    "root-final.log": 0,
    "linux-final.log": 0,
    "static-final.log": 0
}
CLOSED = set(["original_start_authenticated","selected_root_authenticated","complete_sandbox","independent_runtime_custody","host_job_enrolled","all_writers_revoked","real_private_key_or_token_read","server_or_github_mutated","completion_or_later_cells_enabled","qualification_allowed","ship_allowed","performance_claim_allowed"])
ORACLES = [
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture.rs",
]
BASELINE = "a4a3c1b78055bbf6c6a2495c00950a4a3a7b021d"


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
    assert m["schema_version"] == "diagnostic-signer-child-entry-packet-074-v1"
    assert (m["source_commit"], m["source_tree"], m["preregistration"]) == (SOURCE, TREE, PREREG)
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert set(m["closed_claims"]) == CLOSED and all(v is False for v in m["closed_claims"].values())
    assert m["frozen_inputs"] == FROZEN
    for path, digest in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == digest
    expected_sources = set(git("diff", "--name-only", PREREG, SOURCE).decode().splitlines())
    assert set(m["implementation_files"]) == expected_sources and len(expected_sources) == 15
    for path, digest in m["implementation_files"].items():
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == digest and (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original, path
    assert set(m["independent_oracles"]) == set(ORACLES)
    for path in ORACLES:
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert original == git("show", BASELINE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == m["independent_oracles"][path]
        assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original
    expected_files = set(EXITS) | {"README.md", "verify_packet.py", "test_verify_packet.py"}
    assert set(m["files"]) == expected_files
    for name, row in m["files"].items():
        assert Path(name).name == name
        raw = (PACKET / name).read_bytes()
        assert row["bytes"] == len(raw) and row["sha256"] == sha(raw), name
        if name in EXITS:
            assert row["capture_complete"] is (name != "clippy-ice-partial.log")
            assert type(row["outer_exit"]) is int and row["outer_exit"] == EXITS[name]
    for name, expected in COUNTS.items():
        counts = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", (PACKET / name).read_text())
        assert [int(n) for n in counts] == expected, name
    assert "error[E0432]" in (PACKET / "missing-api.log").read_text()
    final = (PACKET / "static-final.log").read_text()
    assert "173 discovered gates, all registered" in final
    assert "local, non-promotable" in final and "expected_ship_refusal_exit=1" in final
    repeats = (PACKET / "linux-repeats.log").read_text()
    for marker in ["x32=SIGSYS core=false", "i386=SIGSYS core=false",
                   "fixed_native_probes=20 exact_EPERM=true", "parent_restrictions_and_cwd_unchanged=true"]:
        assert repeats.count(marker) == 8, marker
    assert "names={0, 1, 2, 3, 7} own=3" in (PACKET / "fd-diagnosis-failed.log").read_text()
    assert "manual implementation" in (PACKET / "clippy-diagnostic-failed.log").read_text()
    assert "compiler unexpectedly panicked" in (PACKET / "clippy-ice-partial.log").read_text()
    helper = "bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081"
    for name in ["baseline-linux.log", "regressions.log"]:
        assert "fixture_namespace_helper_sha256=" + helper in (PACKET / name).read_text()
    print("Verified fixed child restriction evidence; original launch/signing/start remain closed.")


if __name__ == "__main__":
    verify()
