"""Offline local safety evidence checks; no execution or admission authority."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-worker-account-local-evidence-074-v1"
    assert manifest["state"] == "retained-local-safety-evidence"
    entries = manifest["retention"]["files"]
    assert manifest["retention"]["log_count"] == len(entries) == 24
    assert len({entry["name"] for entry in entries}) == 24
    assert {entry["name"] for entry in entries} == {p.name for p in packet.glob("*.log")}
    for entry in entries:
        assert Path(entry["name"]).name == entry["name"]
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]

    def text(name):
        return (packet / name).read_text(encoding="utf-8")

    def rows(name):
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]

    assert rows("baseline.log") == [(159, 1), (29, 0), (2, 0)]
    assert rows("first-green.log") == [(11, 0)]
    assert rows("integration-first.log") == [(3, 0)]
    assert rows("final-linux.log") == [(171, 1), (29, 0), (2, 0), (3, 0)]
    assert rows("reviewed-linux.log") == rows("final-linux.log")
    for name in ["repeats.log", "reviewed-repeats.log"]:
        assert rows(name) == [(12, 0), (3, 0)] * 3, name
        assert text(name).count("account mutation seed=0x74a2026") == 3
        assert text(name).count("account operator exit=Some(9) stdout= stderr=worker account observation refused: Exit; helper cleanup=true") == 3
    for name in ["integration-first.log", "final-linux.log", "reviewed-linux.log"]:
        assert "account operator exit=Some(9) stdout= stderr=worker account observation refused: Exit; helper cleanup=true" in text(name)
    assert rows("windows.log") == [(13, 0), (15, 0), (0, 0), (0, 0)]
    assert rows("root.log") == [(87, 0), (13, 0), (23, 0)]
    assert rows("post-registry.log") == [(1, 0)]
    assert "unresolved imports" in text("api-red.log")
    assert "1 previous error" in text("api-red.log")
    assert "panicked" not in text("api-red.log")
    for name in ["linux-check.log", "linux-lint.log", "windows-check.log", "windows-lint.log",
                 "reviewed-check.log", "reviewed-lint.log"]:
        assert "Finished" in text(name) and "error:" not in text(name), name
    assert "scoped-format-OK" in text("format.log")
    assert "doc-check: OK" in text("doc.log")
    assert "OK (local, non-promotable)" in text("contract.log")
    assert "Checked 51 markdown files and 20 external includes" in text("links.log")
    assert "HTML book written" in text("book.log")
    for expected in ["doc-check: OK", "OK (local, non-promotable)",
                     "Checked 51 markdown files and 20 external includes", "HTML book written"]:
        assert expected in text("docs-final.log"), expected
    assert "ship admission is closed while candidate identity and release qualification are incomplete" in text("ship.log")
    assert "observed-native-exit=1" in text("ship.log")
    checks = manifest["checks"]
    assert checks["baseline_linux_passed"] == 190
    assert checks["final_linux_passed"] == 205
    assert checks["new_linux_unit_tests"] == 12
    assert checks["new_linux_integration_tests"] == 3
    assert checks["preexisting_linux_ignored"] == 1
    assert checks["final_repetitions"] == 3
    assert checks["windows_linux_account_tests_executed"] == 0
    assert checks["root_passed_before_packet_registration"] == 123
    assert checks["ship_expected_red_native_exit"] == 1
    assert checks["full_workspace_verify_rerun"] is False
    assert checks["post_evidence_registry"] is True
    assert checks["documentation_final_passed"] is True
    assert manifest["real_account_observation"]["kind"] == "Exit"
    assert manifest["real_account_observation"]["helper_cleanup_confirmed"] is True
    assert manifest["real_account_observation"]["positive_real_nss_enrollment_proved"] is False
    assert not any(manifest["proof_boundaries"].values())
    assert len(manifest["reviewed_commit"]) == len(manifest["reviewed_tree"]) == 40
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
    print("PASS: 24 exact captures, test counts, refused real observation and frozen digest; admission closed")


if __name__ == "__main__":
    check()
