"""Retained local bytes/counts only, never production execution or admission."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-named-output-local-evidence-074-v1"
    files = manifest["retention"]["files"]
    listed = {entry["name"] for entry in files}
    assert len(listed) == len(files)
    assert listed == {path.name for path in packet.glob("*.log")}
    for entry in files:
        assert Path(entry["name"]).name == entry["name"]
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]

    def rows(name):
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;",
            (packet / name).read_text(encoding="utf-8"))]

    assert rows("baseline.log") == [(25, 0)]
    assert rows("first-green.log") == [(7, 0)]
    assert rows("binding.log") == [(3, 0)]
    assert rows("review.log") == [(7, 0)]
    assert rows("repeats.log") == [(7, 0), (3, 0)] * 3
    assert rows("focused.log") == [
        (107, 1), (29, 0), (22, 0), (20, 0), (2, 0),
        (9, 0), (7, 0), (8, 0), (6, 0)]
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("root-green.log") == [(81, 0), (13, 0), (23, 0)]
    assert rows("post-evidence.log") == [(1, 0)]
    assert manifest["checks"]["post_evidence_registry_passed"] == 1
    assert "could not compile" in (packet / "test-first.log").read_text()
    assert "compiler unexpectedly panicked" in (packet / "test-first.log").read_text()
    assert "test result: FAILED. 80 passed; 1 failed" in (packet / "root.log").read_text()
    assert "O_RDONLY | libc::O_NOFOLLOW" in (packet / "guard-red.log").read_text()
    for name in ["check.log", "clippy.log", "final-clippy.log",
                 "windows-check.log", "windows-clippy.log"]:
        text = (packet / name).read_text()
        assert "Finished" in text and "error:" not in text, name
    for name in ["gates.log", "doc-final.log"]:
        text = (packet / name).read_text()
        assert "doc-check: OK" in text
        assert "performance-contract-check 0.74: OK (local, non-promotable)" in text
    ship = (packet / "ship-final.log").read_text()
    assert "ship admission is closed while candidate identity and release qualification are incomplete" in ship
    assert "found 1 problem(s)" in ship
    assert manifest["checks"]["ship_expected_red_exit"] == 1
    for key in ["production_ancestry_proved", "production_output_prepared",
                "original_start_authenticated", "atomic_namespace_snapshot_proved",
                "same_open_file_description_proved", "continuous_exec_identity_proved",
                "writer_revocation_proved", "durable_refusal_journal",
                "host_service_changed", "unit_created", "observer_workload_run",
                "qualification_run", "numerical_performance_claim", "admission_allowed",
                "frozen_inputs_changed", "existing_wire_or_persisted_schema_changed"]:
        assert manifest["proof_boundaries"][key] is False, key
    root = next(parent for parent in packet.parents if (parent / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
    assert manifest["checks"]["linux_focused_passed"] == 210
    assert manifest["checks"]["new_linux_tests"] == 10
    assert manifest["checks"]["windows_linux_named_output_tests_executed"] == 0
    print("PASS: retained bytes, local counts, refused substitutions; fixture-only scope")


if __name__ == "__main__":
    check()
