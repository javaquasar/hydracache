"""Read-only owned first-exec evidence; never signer/runtime admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "6ea57052920f7db4528a6c4f76c31560a1d2ab2e"
BASELINE = "09c9d439934203e1b8e4383c1fc378f8df1f6f2c"
SOURCES = {
    "tools/long-run-supervisor-074/src/lib.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture.rs",
    "tools/long-run-supervisor-074/src/diagnostic_signer_first_exec_fixture_tests.rs",
    "crates/xtask/tests/performance_contract_074.rs",
    "docs/testing/performance/0.74/diagnostic-signer-first-exec-contract.toml",
    "docs/testing/performance/0.74/diagnostic-signer-first-exec-design.md",
    "docs/GATES.md", "docs/COMPAT.md",
}
ADMISSION = {
    "seccomp_installed", "pre_syscall_exec_denial_proved",
    "actual_tracer_loss_exitkill_proved", "dummy_handoff_implemented",
    "runtime_or_memory_attested", "production_issuer_root_or_job_enrolled",
    "actual_private_key_or_token_read", "bootstrap_signing_or_worker_start_authority",
    "whole_writer_revocation", "completion_or_later_cells_enabled",
    "host_service_or_cgroup_mutated", "protected_job_dispatched",
    "server_entry_wired", "qualification_allowed", "ship_allowed", "performance_claim_allowed",
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def committed(commit, path):
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    return subprocess.check_output(["git", "show", commit + ":" + path], cwd=ROOT)


def main():
    manifest = json.loads((PACKET / "manifest.json").read_bytes())
    assert type(manifest["schema_version"]) is int and manifest["schema_version"] == 1
    assert manifest["packet_id"] == "diagnostic-signer-first-exec-20261014"
    assert manifest["source_commit"] == SOURCE and manifest["baseline_source"] == BASELINE
    assert manifest["b73_frozen_product"] == "16d2e98b6cc9e22d9ccf95eb26fe28bbbcf80f2b"
    assert {entry["path"] for entry in manifest["source_files"]} == SOURCES
    for group, commit in [("source_files", SOURCE), ("unchanged_files", BASELINE)]:
        entries = manifest[group]
        assert entries and len(entries) == len({entry["path"] for entry in entries})
        for entry in entries:
            relative = Path(entry["path"])
            assert not relative.anchor and ".." not in relative.parts
            assert (ROOT / relative).resolve().is_relative_to(ROOT)
            raw = (ROOT / relative).read_bytes().replace(b"\r\n", b"\n")
            assert sha(raw) == entry["sha256_lf"], entry["path"]
            assert raw == committed(commit, entry["path"]).replace(b"\r\n", b"\n"), entry["path"]
    assert {"Cargo.lock", "tools/long-run-supervisor-074/Cargo.toml",
            "tools/long-run-supervisor-074/src/diagnostic_reviewed_trust.rs"}.issubset(
        {entry["path"] for entry in manifest["unchanged_files"]})
    raw = (ROOT / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()
    assert sha(raw) == manifest["qualification_sha256_raw"] == "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    captures = manifest["captures"]
    assert len(captures) == len({capture["path"] for capture in captures})
    assert {capture["path"] for capture in captures} == {p.name for p in PACKET.glob("*.log")}
    for capture in captures:
        assert re.fullmatch(r"signer-first-exec-[a-z-]+\.log", capture["path"])
        raw = (PACKET / capture["path"]).read_bytes()
        assert sha(raw) == capture["sha256"] and len(raw) == capture["bytes"], capture["path"]
        assert type(capture["expected_outer_exit"]) is type(capture["observed_outer_exit"]) is int
        assert capture["expected_outer_exit"] == capture["observed_outer_exit"]
    linux = (PACKET / "signer-first-exec-linux-final.log").read_text()
    for marker in ["test result: ok. 11 passed; 0 failed",
                   "same_inode_second_exec=kernel_event_refused new_user_code_ran=false seccomp_installed=false",
                   "inherited_foreign_writer=closed_before_initial_stop CLOEXEC_alone=insufficient",
                   "seed=0x07403b25 fresh_original_launches=16 independent_tracer_threads=4 signer_or_runtime_authority=false"]:
        assert linux.count(marker) == 3, marker
    for count in (18, 10, 12):
        assert f"test result: ok. {count} passed; 0 failed" in linux
    assert "error[E0433]" in (PACKET / "signer-first-exec-missing-owner.log").read_text()
    red = (PACKET / "signer-first-exec-inherited-writer-red.log").read_text()
    assert "owned first exec stop refused: 0x9a00" in red and "0 passed; 1 failed" in red
    assert "normal-library fixture symbols absent" in (PACKET / "signer-first-exec-normal-library.log").read_text()
    assert "test result: ok. 137 passed; 0 failed" in (PACKET / "signer-first-exec-root-contracts.log").read_text()
    assert "ship admission is closed" in (PACKET / "signer-first-exec-require-ship.log").read_text()
    formatted = json.loads((PACKET / "signer-first-exec-format-result.log").read_bytes())
    assert type(formatted["observed_exit"]) is int and formatted["observed_exit"] == 0
    assert formatted["source_commit"] == SOURCE
    assert set(manifest["admission_claims"]) == ADMISSION
    assert all(value is False for value in manifest["admission_claims"].values())
    for name, key in [("verify_packet.py", "verifier_sha256_lf"),
                      ("test_verify_packet.py", "packet_test_sha256_lf")]:
        assert sha((PACKET / name).read_bytes().replace(b"\r\n", b"\n")) == manifest[key]
    print(f"Verified owned first exec and inherited-writer evidence, {len(captures)} captures; runtime/handoff/start closed.")


if __name__ == "__main__":
    main()
