"""Verify local namespace/credential capture integrity without authorizing a start."""
import hashlib
import json
from pathlib import Path
import re


def check():
    packet = Path(__file__).resolve().parent
    m = json.loads((packet / "manifest.json").read_text(encoding="utf-8"))
    assert m["schema_version"] == "diagnostic-namespace-credentials-local-evidence-074-v1"
    assert m["state"] == "retained-local-safety-evidence"
    entries = m["retention"]["files"]
    assert m["retention"]["log_count"] == len(entries) == 20
    assert len({e["name"] for e in entries}) == 20
    assert {e["name"] for e in entries} == {p.name for p in packet.glob("*.log")}
    for e in entries:
        assert Path(e["name"]).name == e["name"]
        data = (packet / e["name"]).read_bytes()
        assert len(data) == e["bytes"], e["name"]
        assert hashlib.sha256(data).hexdigest() == e["sha256"], e["name"]

    def text(name):
        return (packet / name).read_text(encoding="utf-8")

    def rows(name):
        return [(int(p), int(i)) for p, i in re.findall(
            r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))]

    assert rows("baseline.log") == [(196, 1), (29, 0), (2, 0), (3, 0)]
    assert rows("first-green.log") == [(10, 0)]
    assert rows("final-linux.log") == [(208, 1), (29, 0), (2, 0), (3, 0)]
    assert rows("repeats.log") == [(12, 0)] * 3
    assert text("repeats.log").count("namespace credential mutation seed=0x74f2026") == 3
    for name in m["new_test_functions"]:
        assert text("repeats.log").count("::" + name + " ... ok") == 3, name
        assert "::" + name + " ... ok" in text("final-linux.log"), name
    assert len(set(m["new_test_functions"])) == 12
    assert rows("windows.log") == [(13, 0), (15, 0), (0, 0), (0, 0)]
    assert rows("root.log") == [(90, 0), (13, 0), (23, 0)]
    assert rows("post-registry.log") == [(1, 0)]
    for name in ["api-red.log", "api-red-confirmed.log"]:
        assert "unresolved imports" in text(name)
        assert "1 previous error" in text(name)
        assert "panicked" not in text(name)
    assert "observed-native-exit=\n" in text("api-red.log")
    assert "observed-native-exit=101" in text("api-red-confirmed.log")
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
    assert "observed-native-exit=1" in text("ship.log")
    for name in ["baseline.log", "final-linux.log"]:
        assert "account operator exit=Some(9) stdout= stderr=worker account observation refused: Exit; helper cleanup=true" in text(name)
    for key, value in {
        "baseline_linux_passed": 230, "first_green_new_tests": 10,
        "final_linux_passed": 242, "new_namespace_credential_tests": 12,
        "preexisting_linux_ignored": 1, "repetitions": 3,
        "windows_portable_passed": 28, "windows_linux_namespace_tests_executed": 0,
        "root_passed_before_packet_registration": 126,
        "api_red_confirmed_cargo_native_exit": 101,
        "ship_expected_red_native_exit": 1,
        "post_evidence_registry": True, "documentation_final_passed": True,
        "full_workspace_verify_rerun": False,
    }.items():
        assert m["checks"][key] == value, key
    assert m["real_kernel_test_boundary"] == "owned-nnp-cat-original-process-test-pin-and-current-reading-thread"
    assert not any(m["proof_boundaries"].values())
    assert all(m["implemented_mechanisms"].values())
    for key in ["baseline_source", "preregistration_commit", "implementation_commit", "implementation_tree"]:
        assert re.fullmatch(r"[a-f0-9]{40}", m[key]), key
    root = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
    frozen = root / "docs/testing/performance/0.74/qualification-manifest.toml"
    expected = "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
    assert hashlib.sha256(frozen.read_bytes()).hexdigest() == m["qualification_manifest_sha256"] == expected
    print("PASS: 20 exact captures, namespace credential checks and frozen digest; production admission closed")


if __name__ == "__main__":
    check()
