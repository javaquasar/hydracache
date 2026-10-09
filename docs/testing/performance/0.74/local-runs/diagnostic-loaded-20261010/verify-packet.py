"""Offline byte/count/scope guard, not a manager or launch verifier."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-loaded-local-evidence-074-v1"
    listed = {entry["name"] for entry in manifest["retention"]["files"]}
    assert listed == {path.name for path in packet.glob("*.log")}
    for entry in manifest["retention"]["files"]:
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]
    def totals(name):
        text = (packet / name).read_text(encoding="utf-8")
        rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text)
        assert rows, name
        return sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)
    assert totals("focused.log") == (189, 1)
    assert totals("windows.log") == (28, 0)
    assert totals("root.log") == (115, 0)
    assert "test result: FAILED. 0 passed; 6 failed" in (packet / "initial.log").read_text()
    for name in ["test-first-red.log", "latch-red.log"]:
        assert "could not compile" in (packet / name).read_text()
    assert "test result: FAILED" in (packet / "contract-red.log").read_text()
    for surface in ["embedded", "direct", "resp2", "resp3"]:
        observed = json.loads((packet / f"live-{surface}.log").read_text())
        manager = observed["manager"]
        assert observed["schema_version"] == 1 and observed["settings"] is None
        assert manager["unit"] is None
        assert manager["manager_uid"] == 0 and manager["manager_pid"] == 1
        assert manager["manager_owner"] == ":1.2"
        assert manager["scope"]["surface"] == surface
        assert manager["scope"]["lease_id"] == "a" * 64
        assert manager["scope"]["boot_id"] == "a520ed90-3312-46d8-9fe9-2cfb0f59a08c"
    assert "helper cleanup=true" in (packet / "live-wrong-boot.log").read_text()
    for key in ["complete_loaded_policy_proved", "append_destinations_verified",
                "original_process_or_executable_verified", "host_service_changed",
                "unit_created", "observer_workload_run", "qualification_run",
                "numerical_performance_claim", "admission_allowed"]:
        assert manifest["proof_boundaries"][key] is False, key
    root = next(parent for parent in packet.parents if (parent / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["proof_boundaries"]["qualification_manifest_sha256"]
    print("PASS: retained bytes, local test counts, expected refusals and absent-unit scope; no execution authority")


if __name__ == "__main__":
    check()
