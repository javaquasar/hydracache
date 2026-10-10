"""Offline asserted binding evidence; no enrollment or execution authority."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-worker-binding-local-evidence-074-v1"
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

    assert rows("baseline.log") == [(171, 1), (29, 0), (2, 0), (3, 0)]
    assert rows("first-green.log") == [(12, 0)]
    for name in ["final-linux.log", "reviewed-linux.log"]:
        assert rows(name) == [(184, 1), (29, 0), (2, 0), (3, 0)], name
        assert "account operator exit=Some(9) stdout= stderr=worker account observation refused: Exit; helper cleanup=true" in text(name)
    for name in ["repeats.log", "reviewed-repeats.log"]:
        assert rows(name) == [(13, 0), (3, 0)] * 3, name
        assert text(name).count("binding mutation seed=0x74b2026") == 3
        assert text(name).count("account operator exit=Some(9) stdout= stderr=worker account observation refused: Exit; helper cleanup=true") == 3
    assert rows("windows.log") == [(13, 0), (15, 0), (0, 0), (0, 0)]
    assert rows("root.log") == [(88, 0), (13, 0), (23, 0)]
    assert rows("post-registry.log") == [(1, 0)]
    assert "unresolved imports" in text("api-red.log")
    assert "1 previous error" in text("api-red.log")
    assert "panicked" not in text("api-red.log")
    assert text("linux-lint.log").count("error: call to `std::mem::drop`") == 2
    assert "2 previous errors" in text("linux-lint.log")
    for name in ["linux-check.log", "windows-check.log", "windows-lint.log",
                 "reviewed-check.log", "reviewed-lint.log"]:
        assert "Finished" in text(name) and "error:" not in text(name), name
    for name in ["format.log", "reviewed-format.log"]:
        assert "scoped-format-OK" in text(name)
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
    for key, value in {
        "baseline_linux_passed": 205, "first_green_new_unit_tests": 12,
        "final_linux_passed": 218, "new_linux_binding_tests": 13,
        "preexisting_linux_ignored": 1, "reviewed_repetitions": 3,
        "windows_portable_passed": 28, "windows_linux_binding_tests_executed": 0,
        "root_passed_before_packet_registration": 124,
        "ship_expected_red_native_exit": 1,
        "post_evidence_registry": True, "documentation_final_passed": True,
        "full_workspace_verify_rerun": False,
    }.items():
        assert checks[key] == value, key
    assert manifest["positive_binding_test_boundary"] == "real-owned-original-kernel-credentials-with-synthetic-fixed-account-snapshot"
    assert not any(manifest["proof_boundaries"].values())
    for key in ["baseline_source", "preregistration_commit", "clarification_commit",
                "implementation_commit", "implementation_tree"]:
        assert re.fullmatch(r"[a-f0-9]{40}", manifest[key]), key
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    expected = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == expected
    assert manifest["qualification_manifest_sha256"] == expected
    print("PASS: 24 exact captures, reviewed binding checks and frozen digest; trusted enrollment and admission closed")


if __name__ == "__main__":
    check()
