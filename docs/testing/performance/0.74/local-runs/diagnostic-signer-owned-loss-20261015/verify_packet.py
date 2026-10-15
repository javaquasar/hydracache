"""Unsigned local consistency, including unresolved negative regression evidence."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "ce573dcc3aadd7b02e0f23527311116ceb10a7a7"
TREE = "0f4282d97995c597b33ded983adcf5356a8690ad"
BASELINE = "ce5fd37dc13e38e306eeec94a249140422d900da"
PREREG = "32098fef"
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml":
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
}
ORACLES = [
    "tools/long-run-supervisor-074/src/" + name for name in [
        "lib.rs", "diagnostic_signer_owned_probe.rs", "bin/diagnostic_signer_owned_probe.rs",
        "diagnostic_signer_isolation.rs", "diagnostic_signer_isolation_linux.rs",
        "diagnostic_signer_rooted_exec_fixture.rs", "diagnostic_signer_constructor_loss_fixture.rs",
        "diagnostic_signer_creator_loss_fixture.rs", "diagnostic_signer_thread_exit_fixture.rs",
    ]
]
COUNTS = {
    "baseline.log": [9], "missing-matrix.log": [0],
    "failed-self-stop.log": [], "failed-self-stop-diagnostic.log": [],
    "initial-ack-gate.log": [12], "linux-final.log": [12, 1, 5] * 5,
    "linux-msrv.log": [12, 1, 5], "windows-final.log": [1, 1],
    "root-final.log": [158], "lint-final.log": [],
    "regressions.log": [11, 7, 0, 5, 7], "creator-parallel-failed.log": [],
    "creator-serial.log": [14], "creator-parallel-repeat-failed.log": [],
    "static-final.log": [],
}
FAILED = {"failed-self-stop.log", "failed-self-stop-diagnostic.log",
          "creator-parallel-failed.log", "creator-parallel-repeat-failed.log"}
CLOSED = {
    "all_regressions_passed", "full_normal_entry_parent_loss_proved",
    "pre_arm_or_running_child_loss_proved", "after_exec_kill_mechanism_attributed",
    "selected_root_or_runtime_authenticated", "real_credentials_or_signing",
    "worker_original_start_or_all_writers_revoked", "completion_or_later_cells_enabled",
    "server_or_github_mutated", "qualification_allowed", "ship_allowed",
    "performance_claim_allowed",
}


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


def case_lines():
    return [f"owner_loss first_exec={first} healthy={healthy} original_pidfd_dead=true "
            "monitor_ECHILD=true monitor_child_signaled=false unrelated_alive=true admission=false"
            for healthy in ["false", "true"] for first in ["false", "true"]]


def verify():
    m = json.loads((PACKET / "manifest.json").read_bytes(), object_pairs_hook=unique)
    assert m["schema_version"] == "diagnostic-signer-owned-loss-packet-074-v1"
    assert (m["source_commit"], m["source_tree"], m["baseline"]) == (SOURCE, TREE, BASELINE)
    assert m["preregistration"] == git("rev-parse", PREREG).decode().strip()
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert set(m["closed_claims"]) == CLOSED
    assert all(value is False for value in m["closed_claims"].values())
    assert m["frozen_inputs"] == FROZEN
    for path, digest in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == digest
    sources = set(git("diff", "--name-only", PREREG, SOURCE).decode().splitlines())
    assert set(m["implementation_files"]) == sources and len(sources) == 8
    for path, digest in m["implementation_files"].items():
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == digest and (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original
    assert set(m["unchanged_sources"]) == set(ORACLES)
    for path in ORACLES:
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert original == git("show", BASELINE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == m["unchanged_sources"][path]
        assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original
    files = set(COUNTS) | {"README.md", "verify_packet.py", "test_verify_packet.py"}
    assert set(m["files"]) == files
    for name, row in m["files"].items():
        assert Path(name).name == name
        raw = (PACKET / name).read_bytes()
        assert row["bytes"] == len(raw) and row["sha256"] == sha(raw), name
        if name in COUNTS:
            assert row["capture_complete"] is True
            assert type(row["outer_exit"]) is int
            assert row["outer_exit"] == int(name in FAILED)
    for name, counts in COUNTS.items():
        raw = (PACKET / name).read_text()
        found = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", raw)
        assert [int(n) for n in found] == counts, name
    for name, repeats in [("linux-final.log", 5), ("linux-msrv.log", 1), ("initial-ack-gate.log", 1)]:
        raw = (PACKET / name).read_text()
        for line in case_lines():
            assert raw.splitlines().count(line) == repeats, (name, line)
        assert raw.count("owner_loss_matrix=4 actual_owner_SIGKILL=2 healthy_drop_controls=2") == repeats
        assert raw.count("owner_loss_report_fixed_bit_mutations=512 seeded_identity_shapes=128") == repeats
    for name in ["creator-parallel-failed.log", "creator-parallel-repeat-failed.log"]:
        raw = (PACKET / name).read_text()
        assert "signer_thread_exit_no_exitkill_is_not_positive ... FAILED" in raw
        assert "test result: FAILED. 13 passed; 1 failed;" in raw
        assert "owned runtime root refused" in raw
    assert "State:\tZ (zombie)" in (PACKET / "failed-self-stop-diagnostic.log").read_text()
    assert "assertion failed: !dead(&orphan.fd)" in (PACKET / "failed-self-stop.log").read_text()
    final = (PACKET / "static-final.log").read_text()
    for marker in ["174 discovered gates, all registered", "local, non-promotable", "expected_ship_refusal_exit=1"]:
        assert marker in final
    print("Verified private owner loss packet; parallel legacy regression remains red; admission closed.")


if __name__ == "__main__":
    verify()
