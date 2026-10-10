"""Offline retention QA only, never live identity or release-admission proof."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    manifest = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["schema_version"] == "diagnostic-live-identity-local-evidence-074-v1"
    files = manifest["retention"]["files"]
    names = {entry["name"] for entry in files}
    assert len(names) == len(files)
    assert names == {path.name for path in packet.glob("*.log")}
    for entry in files:
        assert Path(entry["name"]).name == entry["name"]
        data = (packet / entry["name"]).read_bytes()
        assert len(data) == entry["bytes"], entry["name"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"], entry["name"]

    def rows(name):
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;",
            (packet / name).read_text(encoding="utf-8"))]

    assert rows("baseline.log") == [(7, 0), (0, 0)]
    assert rows("first-green.log") == [(9, 0)]
    assert rows("review.log") == [(10, 0)]
    assert rows("repeats.log") == [(10, 0)] * 3
    assert rows("final-repeats.log") == [(10, 0)] * 3
    expected = [(117, 1), (29, 0), (22, 0), (20, 0), (2, 0),
                (9, 0), (7, 0), (8, 0), (6, 0)]
    assert rows("focused.log") == expected
    assert rows("final-focused.log") == expected
    assert rows("windows.log") == [(13, 0), (15, 0)]
    assert rows("root-green.log") == [(82, 0), (13, 0), (23, 0)]
    assert rows("post-evidence.log") == [(1, 0)]
    assert manifest["checks"]["post_evidence_registry_passed"] == 1
    assert "could not compile" in (packet / "test-first.log").read_text()
    red = (packet / "root.log").read_text()
    assert "test result: FAILED. 81 passed; 1 failed" in red
    assert "snapshot.pin_for(state)" in red
    for name in ["check.log", "clippy.log", "final-check.log", "final-clippy.log",
                 "windows-check.log", "windows-clippy.log", "xtask-check.log"]:
        text = (packet / name).read_text()
        assert "Finished" in text and "error:" not in text, name
    assert "doc-check: OK" in (packet / "doc.log").read_text()
    assert "performance-contract-check 0.74: OK (local, non-promotable)" in (
        packet / "contract.log").read_text()
    final_doc = (packet / "doc-final.log").read_text()
    assert "doc-check: OK" in final_doc
    assert "performance-contract-check 0.74: OK (local, non-promotable)" in final_doc
    assert "Checked 51 markdown files and 20 external includes" in (
        packet / "links-final.log").read_text()
    assert "HTML book written" in (packet / "mdbook-final.log").read_text()
    assert "ship admission is closed while candidate identity and release qualification are incomplete" in (
        packet / "ship.log").read_text()
    assert manifest["checks"]["ship_expected_red_exit"] == 1
    assert manifest["checks"]["linux_focused_passed"] == 220
    assert manifest["checks"]["new_linux_tests"] == 10
    assert manifest["checks"]["windows_linux_identity_tests_executed"] == 0
    for value in manifest["proof_boundaries"].values():
        assert value is False
    root = next(parent for parent in packet.parents if (parent / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
    print("PASS: retained local bytes and counts; synthetic joins, actual foreign-scope refusal; admission closed")


if __name__ == "__main__":
    check()
