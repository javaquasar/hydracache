"""Retention QA only; no production ancestry or execution proof."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    m = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert m["schema_version"] == "diagnostic-production-output-local-evidence-074-v1"
    files = m["retention"]["files"]
    names = {entry["name"] for entry in files}
    assert len(names) == len(files)
    assert names == {p.name for p in packet.glob("*.log")}
    for entry in files:
        assert Path(entry["name"]).name == entry["name"]
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]

    def rows(name):
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;",
            (packet / name).read_text(encoding="utf-8"))]

    assert rows("baseline.log") == [(7, 0)]
    assert rows("first-green.log") == [(10, 0)]
    assert rows("initial-focused.log") == [(121, 1), (29, 0)]
    assert rows("reviewed-focused.log") == [(121, 1), (29, 0)]
    assert rows("repeats.log") == [(11, 0)] * 3
    assert rows("traversal-focused.log") == [(122, 1), (29, 0)]
    assert rows("traversal-repeats.log") == [(12, 0)] * 3
    assert rows("traversal-root-guard.log") == [(1, 0)]
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("traversal-windows.log") == [(13, 0), (15, 0)]
    assert rows("root.log") == [(83, 0), (13, 0), (23, 0)]
    assert rows("post-evidence.log") == [(1, 0)]
    assert rows("traversal-post-evidence.log") == [(1, 0)]
    assert "20 previous errors" in (packet / "test-first.log").read_text()
    assert "test result: FAILED. 0 passed; 1 failed" in (packet / "traversal-red.log").read_text()
    for name in ["check.log", "lint.log", "reviewed-check.log", "reviewed-lint.log",
                 "windows-check.log", "windows-lint.log", "traversal-check.log", "traversal-lint.log"]:
        text = (packet / name).read_text()
        assert "Finished" in text and "error:" not in text, name
    text = (packet / "doc-final.log").read_text()
    assert "doc-check: OK" in text
    assert "performance-contract-check 0.74: OK (local, non-promotable)" in text
    text = (packet / "traversal-doc-final.log").read_text()
    assert "doc-check: OK" in text
    assert "performance-contract-check 0.74: OK (local, non-promotable)" in text
    assert "Checked 51 markdown files and 20 external includes" in (packet / "links-final.log").read_text()
    assert "HTML book written" in (packet / "book-final.log").read_text()
    assert "ship admission is closed while candidate identity and release qualification are incomplete" in (packet / "ship.log").read_text()
    assert m["checks"]["ship_expected_red_exit"] == 1
    assert m["checks"]["initial_linux_focused_passed"] == 150
    assert m["checks"]["linux_focused_passed"] == 151
    assert m["checks"]["windows_linux_output_tests_executed"] == 0
    assert m["checks"]["final_traversal_registry_passed"] == 1
    assert m["checks"]["final_doc_check"] is True
    assert m["checks"]["new_linux_tests"] == 5
    assert m["checks"]["final_ship_expected_red_exit"] == 1
    assert len(files) == m["retention"]["log_count"] == 43
    assert "ship admission is closed" in (packet / "traversal-ship.log").read_text()
    assert not any(m["proof_boundaries"].values())
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == m["qualification_manifest_sha256"]
    print("PASS: retained bytes/counts; synthetic policy and temporary readers only; admission closed")


if __name__ == "__main__":
    check()
