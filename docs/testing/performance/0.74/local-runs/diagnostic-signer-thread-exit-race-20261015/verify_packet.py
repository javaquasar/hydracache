"""Unsigned local race evidence, not execution attestation or admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "9b9ee382939923bb6a03ca5564d10a8065829b21"
TREE = "fce5bb6e1ff812d66e749ccbdbd5b7bf07ccff77"
BASELINE = "7eca2350"
PREREG = "7072f5e04bb9a4b789caf462e79ff0995c7490ab"
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml":
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
}
COUNTS = {
    "baseline-current.log": [14], "attributed-current.log": [14],
    "concurrent-before.log": [], "kernel-before.log": [],
    "isolated-negative-before.log": [1], "first-group-only.log": [8],
    "group-only-repeats.log": [16, 16], "group-only-detach-diagnostic.log": [],
    "pending-signal-exploration.log": [], "transition-model-before.log": [],
    "joint-negative-final.log": [1] * 5, "joint-parallel-final.log": [17] * 5,
    "linux-msrv-final.log": [17], "windows-final.log": [7, 1, 7, 1],
    "root-final.log": [158], "regressions.log": [11, 7, 11, 13, 13, 12, 5, 7, 1, 5],
    "lint-final.log": [], "windows-lint-final.log": [], "static-final.log": [],
}
EXITS = {name: 0 for name in COUNTS}
EXITS.update({name: 1 for name in ["concurrent-before.log", "kernel-before.log",
             "pending-signal-exploration.log", "transition-model-before.log"]})
EXITS.update({"group-only-repeats.log": 101, "group-only-detach-diagnostic.log": 101})
CLOSED = {"production_runtime_or_issuer_custody", "actual_private_key_or_token_read",
          "authenticated_original_worker_start", "all_writers_revoked",
          "completion_or_later_cells_enabled", "host_services_or_cgroups_changed",
          "server_entry_wired", "qualification_allowed", "ship_allowed",
          "performance_claim_allowed", "full_release_gate_passed",
          "all_linux_kernels_supported", "extra_pidfd_sigstop_sent",
          "exact_natural_exit_code_observed"}
ORACLES = ["tools/long-run-supervisor-074/src/" + name for name in [
    "lib.rs", "diagnostic_signer_owned_probe.rs", "diagnostic_signer_isolation.rs",
    "diagnostic_signer_isolation_linux.rs", "diagnostic_signer_handoff_helper.rs",
    "diagnostic_signer_rooted_exec_fixture.rs", "diagnostic_signer_creator_loss_fixture.rs"]]
PRIOR = ["docs/testing/performance/0.74/local-runs/diagnostic-signer-owned-loss-20261015/" + name
         for name in ["creator-parallel-failed.log", "creator-parallel-repeat-failed.log"]]


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
            env.update(GIT_DIR="/mnt/" + path[0].lower() + path[2:], GIT_WORK_TREE=str(ROOT))
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)


def verify():
    m = json.loads((PACKET / "manifest.json").read_bytes(), object_pairs_hook=unique)
    assert m["schema_version"] == "diagnostic-signer-thread-exit-race-074-v1"
    assert (m["source_commit"], m["source_tree"], m["preregistration"]) == (SOURCE, TREE, PREREG)
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert set(m["closed_claims"]) == CLOSED and all(v is False for v in m["closed_claims"].values())
    assert m["scoped_final_checks_passed"] is True
    assert m["frozen_inputs"] == FROZEN
    for path, digest in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == digest
    sources = set(git("diff", "--name-only", PREREG, SOURCE).decode().splitlines())
    assert set(m["implementation_files"]) == sources and len(sources) == 7
    for path, digest in m["implementation_files"].items():
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == digest and (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original
    assert set(m["unchanged_sources"]) == set(ORACLES)
    for path in ORACLES:
        original = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert original == git("show", BASELINE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(original) == m["unchanged_sources"][path]
        assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == original
    assert set(m["prior_negative_evidence"]) == set(PRIOR)
    for path in PRIOR:
        raw = (ROOT / path).read_bytes()
        assert sha(raw) == m["prior_negative_evidence"][path] == sha(git("show", BASELINE + ":" + path))
        assert b"test result: FAILED. 13 passed; 1 failed;" in raw
    assert set(m["files"]) == set(COUNTS) | {"README.md", "verify_packet.py", "test_verify_packet.py"}
    for name, row in m["files"].items():
        assert Path(name).name == name
        raw = (PACKET / name).read_bytes()
        assert row["bytes"] == len(raw) and row["sha256"] == sha(raw), name
        if name in EXITS:
            assert row["capture_complete"] is True and type(row["outer_exit"]) is int
            assert row["outer_exit"] == EXITS[name]
            counts = re.findall(rb"test result: ok\. (\d+) passed; 0 failed;", raw)
            assert [int(n) for n in counts] == COUNTS[name], name
    def log(name):
        return (PACKET / name).read_text()
    for name, count in [("joint-parallel-final.log", 5), ("linux-msrv-final.log", 1)]:
        raw = log(name)
        for marker in ["thread_rooted_kernel_delivery_to_group_stop=true exact_EINVAL=true classification_cases=25",
                       "thread_detach_transition_models=6 running_never_accepted=true terminal_refuses=true",
                       "thread_negative_independent_observers=4 cleanup_never_positive=true"]:
            assert raw.count(marker) == count, (name, marker)
        negative = "thread_terminal stage=1 mode=1 code=2 status=9 original_tgid_alive_at_child_terminal=true independent_reaped=true child_signaled=true dummy_sends=0 positive=false"
        assert raw.splitlines().count(negative) == count * 5
        for stage in range(6):
            assert f"thread_terminal stage={stage} mode=0 code=2 status=9" in raw
        assert "child_signaled=true dummy_sends=0 positive=true" not in raw
    assert "child_live=false tgid_live=true" in log("concurrent-before.log")
    assert "Seccomp:" in log("concurrent-before.log") and "zombie" in log("concurrent-before.log")
    assert "rooted loss cut remained a delivery-stop" in log("kernel-before.log")
    assert "test result: FAILED. 15 passed; 1 failed;" in log("group-only-repeats.log")
    assert "State:" in log("group-only-detach-diagnostic.log") and "R (running)" in log("group-only-detach-diagnostic.log")
    assert "kernel group stop alone has no pending restart stop" in log("pending-signal-exploration.log")
    assert "test result: FAILED. 0 passed; 1 failed;" in log("transition-model-before.log")
    for marker in ["174 discovered gates, all registered", "local, non-promotable", "expected_ship_refusal_exit=1"]:
        assert marker in log("static-final.log")
    print("Verified local tracer-thread race correction; production and release admission remain closed.")


if __name__ == "__main__":
    verify()
