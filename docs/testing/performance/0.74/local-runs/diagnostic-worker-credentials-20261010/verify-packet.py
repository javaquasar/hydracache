"""Offline local safety evidence verification; no execution or admission authority."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-worker-credentials-local-evidence-074-v1"
    assert manifest["state"] == "retained-local-safety-evidence"
    assert manifest["retention"]["log_count"] == 23
    entries = manifest["retention"]["files"]
    assert len(entries) == len({e["name"] for e in entries}) == 23
    assert {e["name"] for e in entries} == {p.name for p in packet.glob("*.log")}
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

    assert rows("baseline.log") == [(143, 1), (29, 0)]
    assert rows("first-green.log") == [(13, 0)]
    assert rows("initial-reviewed-linux.log") == [(158, 1), (29, 0)]
    assert rows("initial-repeats.log") == [(15, 0)] * 3
    assert rows("final-linux.log") == [(159, 1), (29, 0)]
    assert rows("final-repeats.log") == [(16, 0)] * 3
    assert text("final-repeats.log").count("credential mutation seed=0x74c2026") == 3
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("root.log") == [(86, 0), (13, 0), (23, 0)]
    assert rows("post-registry.log") == [(1, 0)]
    assert "no test target named `diagnostic_artifacts_linux`" in text("baseline-wrong-target.log")
    assert not rows("baseline-wrong-target.log")
    assert "the compiler unexpectedly panicked" in text("api-renderer-red.log")
    assert "28 previous errors" in text("api-renderer-red.log")
    assert "unresolved imports" in text("api-check-red.log")
    assert "1 previous error" in text("api-check-red.log")
    assert "panicked" not in text("api-check-red.log")
    for name in ["linux-check.log", "linux-lint.log", "windows-check.log", "windows-lint.log"]:
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
    checks = manifest["checks"]
    assert checks["baseline_linux_passed"] == 172
    assert checks["final_linux_passed"] == 188
    assert checks["new_linux_tests"] == 16
    assert checks["windows_linux_credential_tests_executed"] == 0
    assert checks["ship_expected_red_native_exit"] == 1
    assert checks["full_workspace_verify_rerun"] is False
    assert checks["post_evidence_registry"] is True
    assert checks["documentation_final_passed"] is True
    assert not any(manifest["proof_boundaries"].values())
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
    print("PASS: 23 exact captures, counts and frozen digest; numeric observation only; admission closed")


if __name__ == "__main__":
    check()
