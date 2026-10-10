"""Offline byte/count retention QA; not production authority or performance proof."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    m = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert m["schema_version"] == "diagnostic-output-preparation-local-evidence-074-v1"
    assert m["state"] == "retained-local-safety-evidence"
    entries = m["retention"]["files"]
    assert len(entries) == len({e["name"] for e in entries}) == 27
    assert {e["name"] for e in entries} == {p.name for p in packet.glob("*.log")}
    assert m["retention"]["log_count"] == 27
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

    assert rows("baseline.log") == [(129, 1), (29, 0)]
    assert rows("first-green.log") == [(10, 0)]
    for name in ["reviewed.log", "final.log"]:
        assert rows(name) == [(143, 1), (29, 0)]
    for name in ["repeats.log", "final-repeats.log"]:
        assert rows(name) == [(14, 0)] * 3
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("root.log") == [(85, 0), (13, 0), (23, 0)]
    assert rows("post-evidence.log") == [(1, 0)]
    assert "panicked at" in text("compiler-renderer-red.log")
    assert "27 previous errors" in text("api-red.log")
    assert "panicked at" not in text("api-red.log")
    assert "141 passed; 1 failed; 1 ignored" in text("parallel-initial-failure.log")
    assert "Host(Busy)" in text("parallel-initial-failure.log")
    assert "0 passed; 1 failed" in text("lock-red.log")
    assert "missing field `unlocked`" in text("placement-error.log")
    assert "all variants have the same prefix" in text("lint-red.log")
    for name in ["check.log", "final-check.log", "final-lint.log",
                 "windows-check.log", "windows-lint.log"]:
        assert "Finished" in text(name) and "error:" not in text(name), name
    assert "scoped-format-OK" in text("format.log")
    assert "doc-check: OK" in text("doc.log")
    assert "OK (local, non-promotable)" in text("contract.log")
    assert "doc-check: OK" in text("doc-final.log")
    assert "OK (local, non-promotable)" in text("doc-final.log")
    assert "Checked 51 markdown files and 20 external includes" in text("links.log")
    assert "HTML book written" in text("book.log")
    assert "ship admission is closed while candidate identity and release qualification are incomplete" in text("ship.log")
    assert m["checks"]["ship_expected_red_native_exit"] == 1
    assert m["checks"]["new_linux_tests"] == 14
    assert m["checks"]["final_linux_passed"] == 172
    assert m["checks"]["windows_linux_preparation_tests_executed"] == 0
    assert m["checks"]["full_workspace_verify_rerun"] is False
    assert not any(m["proof_boundaries"].values())
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == m["qualification_manifest_sha256"]
    print("PASS: 27 exact captures, counts and frozen digest; fixture safety only; admission closed")


if __name__ == "__main__":
    check()
