"""Offline local consistency evidence, never production trust or admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "e3c91ad2d42a5af0d6ce089fb93c925622fa272a"
TREE = "b7d832f62805986721e1c4d002b1f3d8b49f481f"
PREREG = "0f6a3145a5898c39d6a8797d93725d57a94e6268"
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml":
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
}
COUNTS = {
    "baseline-first-exec.log": [
        11
    ],
    "baseline-child.log": [
        5,
        7
    ],
    "missing-binary.log": [],
    "initial.log": [
        7,
        1,
        4
    ],
    "failed-linux.log": [
        8,
        1,
        4,
        8,
        1
    ],
    "failed-msrv.log": [
        8,
        1
    ],
    "lint-failed.log": [],
    "linux-final.log": [
        9,
        1,
        5,
        9,
        1,
        5,
        9,
        1,
        5,
        9,
        1,
        5,
        9,
        1,
        5
    ],
    "linux-msrv.log": [
        9,
        1,
        5
    ],
    "windows-final.log": [
        7,
        1
    ],
    "windows-msrv.log": [
        7,
        1
    ],
    "root-final.log": [
        157
    ],
    "regressions.log": [
        11,
        13,
        13,
        7,
        8
    ],
    "child-final.log": [
        5,
        7
    ],
    "static-final.log": []
}
EXITS = {
    "baseline-first-exec.log": 0,
    "baseline-child.log": 0,
    "missing-binary.log": 1,
    "initial.log": 0,
    "failed-linux.log": 101,
    "failed-msrv.log": 101,
    "lint-failed.log": 101,
    "linux-final.log": 0,
    "linux-msrv.log": 0,
    "windows-final.log": 0,
    "windows-msrv.log": 0,
    "root-final.log": 0,
    "regressions.log": 0,
    "child-final.log": 0,
    "static-final.log": 0
}
CLOSED = set(["normal_parent_loss_all_boundaries_proved","selected_signer_root_or_runtime_authenticated","independent_job_or_runtime_custody","mapped_instruction_integrity_or_complete_sandbox","real_private_key_or_token_read","authenticated_worker_original_start","all_writers_revoked","completion_or_later_cells_enabled","signing_allowed","server_or_github_mutated","qualification_allowed","ship_allowed","performance_claim_allowed"])
ORACLES = [
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation_linux.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs"
]
BASELINE = "f22ebc7c11dbb1a4104ca2b0dda207ceb66488f1"


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
    assert m["schema_version"] == "diagnostic-signer-owned-probe-packet-074-v1"
    assert (m["source_commit"], m["source_tree"], m["preregistration"]) == (SOURCE, TREE, PREREG)
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert set(m["closed_claims"]) == CLOSED and all(v is False for v in m["closed_claims"].values())
    assert m["frozen_inputs"] == FROZEN
    for path, digest in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == digest
    expected_sources = set(git("diff", "--name-only", PREREG, SOURCE).decode().splitlines())
    assert set(m["implementation_files"]) == expected_sources and len(expected_sources) == 12
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
            assert row["capture_complete"] is True
            assert type(row["outer_exit"]) is int and row["outer_exit"] == EXITS[name]
    for name, expected in COUNTS.items():
        counts = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", (PACKET / name).read_text())
        assert [int(n) for n in counts] == expected, name
    missing = (PACKET / "missing-binary.log").read_text()
    assert "CARGO_BIN_EXE_diagnostic_signer_owned_probe" in missing
    assert "not defined at compile time" in missing
    final = (PACKET / "static-final.log").read_text()
    assert "174 discovered gates, all registered" in final
    assert "local, non-promotable" in final and "expected_ship_refusal_exit=1" in final
    repeats = (PACKET / "linux-final.log").read_text()
    for marker in ["normal_original_exec_modes=native,x32,i386",
                   "held_writer_exec=ETXTBSY retry=false",
                   "parent_context_unchanged=true seed=0x0740c07a",
                   "original_child_SIGKILL_and_drop=initial,first_exec",
                   "wait_status_cases=327680 seed=0x0740c07a mutations=256"]:
        assert repeats.count(marker) == 5, marker
    for name in ["failed-linux.log", "failed-msrv.log"]:
        failure = (PACKET / name).read_text()
        assert "Text file busy" in failure and "test result: FAILED." in failure
    assert "err().expect()" in (PACKET / "lint-failed.log").read_text()
    assert "fixture_namespace_helper_sha256=bf78cb987e98962676c63cf62f3efae0ecf0cd7258f126a231634bc41d075081" in (PACKET / "regressions.log").read_text()
    print("Verified ordinary original-self-exec packet; signer/root/runtime/start admission stays closed.")


if __name__ == "__main__":
    verify()
