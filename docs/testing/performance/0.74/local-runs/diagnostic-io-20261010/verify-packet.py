"""Offline retained-byte/count/scope guard; never execution or qualification proof."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-io-local-evidence-074-v1"
    listed = {entry["name"] for entry in manifest["retention"]["files"]}
    assert listed == {path.name for path in packet.glob("*.log")}
    for entry in manifest["retention"]["files"]:
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]

    def rows(name):
        text = (packet / name).read_text(encoding="utf-8")
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text)]

    assert rows("baseline.log") == [(15, 0)]
    assert rows("focused.log") == [
        (97, 1), (29, 0), (22, 0), (20, 0), (2, 0),
        (9, 0), (7, 0), (8, 0), (6, 0), (10, 0), (10, 0), (10, 0)]
    assert rows("post-review.log") == [(25, 0), (1, 0)]
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("root.log") == [(80, 0), (13, 0), (23, 0)]
    assert "could not compile" in (packet / "test-first-red.log").read_text()
    assert "compiler unexpectedly panicked" in (packet / "test-first-red.log").read_text()
    assert "test result: FAILED. 6 passed; 1 failed" in (packet / "initial.log").read_text()
    assert "handshake timed out" in (packet / "initial.log").read_text()
    assert "test result: FAILED. 0 passed; 1 failed" in (packet / "contract-red.log").read_text()
    assert "ineffective_open_options" in (packet / "check-clippy-initial.log").read_text()
    assert "compiler unexpectedly panicked" in (packet / "check-clippy-initial.log").read_text()
    assert "Finished" in (packet / "clippy-green.log").read_text()
    gates = (packet / "gates.log").read_text()
    assert "doc-check: OK" in gates
    assert "performance-contract-check 0.74: OK (local, non-promotable)" in gates
    final_docs = (packet / "doc-final.log").read_text()
    assert "doc-check: OK" in final_docs
    assert "performance-contract-check 0.74: OK (local, non-promotable)" in final_docs
    for key in ["output_pathnames_verified", "original_start_authenticated",
                "same_open_file_description_proved", "continuous_exec_identity_proved",
                "effective_environment_proved", "writer_revocation_proved",
                "durable_refusal_journal", "host_service_changed", "unit_created",
                "observer_workload_run", "qualification_run", "numerical_performance_claim",
                "admission_allowed", "frozen_inputs_changed"]:
        assert manifest["proof_boundaries"][key] is False, key
    root = next(parent for parent in packet.parents if (parent / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
    assert manifest["checks"]["linux_focused_passed"] == 200
    assert manifest["checks"]["new_linux_tests_including_worker_entry"] == 11
    assert manifest["checks"]["windows_linux_io_tests_executed"] == 0
    print("PASS: retained bytes, local counts, expected refusals; point-in-time object scope only")


if __name__ == "__main__":
    check()
