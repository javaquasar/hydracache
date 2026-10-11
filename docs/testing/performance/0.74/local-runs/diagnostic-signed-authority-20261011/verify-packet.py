"""Read-only local safety verifier; no launch or admission authority."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

packet = Path(__file__).resolve().parent
repo = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
manifest = json.loads((packet / "manifest.json").read_bytes())
assert manifest["state"] == "retained-local-runtime-refusal-bridge-not-enrollment"


def git(*args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def totals(name):
    raw = (packet / name).read_text(encoding="utf-8")
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", raw)
    assert rows, name
    return sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)


files = manifest["files"]
assert len({f["name"] for f in files}) == len(files)
assert {p.name for p in packet.glob("*.log")} == {f["name"] for f in files if f["name"].endswith(".log")}
for entry in files:
    assert Path(entry["name"]).name == entry["name"]
    raw = (packet / entry["name"]).read_bytes()
    assert len(raw) == entry["bytes"]
    assert hashlib.sha256(raw).hexdigest() == entry["sha256"], entry["name"]
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", manifest[prefix + "_source"] + "^{tree}").decode().strip() == manifest[prefix + "_tree"]
assert totals("baseline-tasks.log") == (0, 0)
assert totals("baseline-tasks-corrected.log") == (11, 0)
assert totals("baseline-journal.log") == (23, 0)
assert totals("linux-scoped.log") == (437, 1)
assert totals("final-linux-scoped.log") == (437, 1)
assert totals("windows-tests.log") == (195, 0)
assert totals("root-performance-windows.log") == (104, 0)
assert totals("root-registry-windows.log") == (36, 0)
assert totals("doctests-linux.log") == (2, 0)
assert "signed_authority_api_keeps_original_fixed_and_fixture_types_distinct ... ok" in (packet / "final-linux-scoped.log").read_text(encoding="utf-8")
assert "OK (local, non-promotable)" in (packet / "final-performance-contract-windows.log").read_text(encoding="utf-8")
assert "unsupported performance-contract-check option --tier" in (packet / "performance-contract-windows.log").read_text(encoding="utf-8")
assert "unsafe repository path" in (packet / "ship-refusal-windows.log").read_text(encoding="utf-8")
names = manifest["new_unit_test_names"]
assert len(names) == 17
for i in (1, 2, 3):
    name = f"serial-linux-{i}.log"
    assert totals(name) == (17, 0)
    text = (packet / name).read_text(encoding="utf-8")
    for test in names:
        assert f"::{test} ... ok" in text, (name, test)
for name, expected in [
    ("missing-api-red.log", "could not find `authority_binding`"),
    ("bridge-first.log", "FnOnce"),
    ("bridge-expanded.log", "FAILED. 13 passed; 1 failed"),
    ("lint-linux.log", "needless_borrow"),
    ("qualification-dry-red.log", "contract input digest mismatch"),
    ("ship-admission-red-windows.log", "ship admission is closed"),
    ("ship-contract-red-windows.log", "rejected non-green evidence"),
]:
    assert expected in (packet / name).read_text(encoding="utf-8"), name
qualification_path = repo / "docs/testing/performance/0.74/qualification-manifest.toml"
assert hashlib.sha256(qualification_path.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
qualification = tomllib.loads(qualification_path.read_text(encoding="utf-8"))
mismatches = []
for item in qualification["contract_inputs"]:
    committed = git("show", manifest["baseline_source"] + ":" + item["path"])
    assert (repo / item["path"]).read_bytes().replace(b"\r\n", b"\n") == committed, item["path"]
    if hashlib.sha256(committed).hexdigest() != item["sha256"]:
        mismatches.append(item["path"])
assert mismatches == ["docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"]
print(f"verified {len(files)} closed captures; local safety only; inherited qualification refusal retained")
