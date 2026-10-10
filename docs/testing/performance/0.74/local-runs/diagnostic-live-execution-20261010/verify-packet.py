"""Byte/count retention QA only, never production execution or admission proof."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    m = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert m["schema_version"] == "diagnostic-live-execution-local-evidence-074-v1"
    entries = m["retention"]["files"]
    names = {entry["name"] for entry in entries}
    assert len(names) == len(entries) == m["retention"]["log_count"]
    assert names == {p.name for p in packet.glob("*.log")}
    for entry in entries:
        assert Path(entry["name"]).name == entry["name"]
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]

    def rows(name):
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;",
            (packet / name).read_text(encoding="utf-8"))]

    assert rows("baseline.log") == [(122, 1), (29, 0)]
    assert rows("first-green.log") == [(127, 1), (29, 0)]
    for name in ["focused.log", "reviewed-focused.log"]:
        assert rows(name) == [(129, 1), (29, 0)]
    assert rows("repeats.log") == [(7, 0)] * 3
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("root.log") == [(84, 0), (13, 0), (23, 0)]
    assert rows("post-evidence.log") == [(1, 0)]
    assert "16 previous errors" in (packet / "red.log").read_text()
    assert "20 previous errors" in (packet / "binding-red.log").read_text()
    for name in ["check.log", "lint.log", "windows-check.log", "windows-lint.log"]:
        text = (packet / name).read_text()
        assert "Finished" in text and "error:" not in text, name
    assert "doc-check: OK" in (packet / "doc.log").read_text()
    assert "OK (local, non-promotable)" in (packet / "contract.log").read_text()
    assert "doc-check: OK" in (packet / "doc-final.log").read_text()
    assert "OK (local, non-promotable)" in (packet / "doc-final.log").read_text()
    assert "Checked 51 markdown files and 20 external includes" in (packet / "links.log").read_text()
    assert "HTML book written" in (packet / "book.log").read_text()
    assert "ship admission is closed while candidate identity and release qualification are incomplete" in (packet / "ship.log").read_text()
    assert m["checks"]["ship_expected_red_native_exit"] == 1
    assert m["checks"]["new_linux_tests"] == 7
    assert m["retention"]["log_count"] == 23
    assert m["checks"]["post_evidence_registry"] is True
    assert m["checks"]["final_doc_contract"] is True
    assert m["checks"]["linux_focused_passed"] == 158
    assert m["checks"]["windows_linux_execution_tests_executed"] == 0
    assert m["checks"]["full_workspace_verify_rerun"] is False
    assert not any(m["proof_boundaries"].values())
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == m["qualification_manifest_sha256"]
    print("PASS: exact retained bytes/counts; fixture/synthetic refusals only; production and ship admission closed")


if __name__ == "__main__":
    check()
