"""Read-only byte-binding safety verification; never launch or admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import unquote

packet = Path(__file__).resolve().parent
repo = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
manifest = json.loads((packet / "manifest.json").read_bytes())
assert manifest["state"] == "retained-local-signed-projection-not-production-intent"


def git(*args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def totals(name):
    raw = (packet / name).read_text(encoding="utf-8")
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", raw)
    assert rows, name
    return sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)


files = manifest["files"]
assert len({f["name"] for f in files}) == len(files)
assert {p.name for p in packet.glob("*.log")} == {f["name"] for f in files}
for entry in files:
    assert Path(entry["name"]).name == entry["name"]
    raw = (packet / entry["name"]).read_bytes()
    assert len(raw) == entry["bytes"], entry["name"]
    assert hashlib.sha256(raw).hexdigest() == entry["sha256"], entry["name"]
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", manifest[prefix + "_source"] + "^{tree}").decode().strip() == manifest[prefix + "_tree"]
for path, expected in manifest["implementation_source_sha256"].items():
    committed = git("show", manifest["implementation_source"] + ":" + path)
    assert hashlib.sha256(committed).hexdigest() == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == committed, path
for name, expected in [
    ("03-baseline-serial.log", (433, 1)), ("04-first-tests.log", (12, 0)),
    ("06-scoped-linux.log", (451, 1)), ("11-intent-final.log", (16, 0)),
    ("12-unit-final.log", (2, 0)), ("13-doctests.log", (5, 0)),
    ("16-windows-tests.log", (195, 0)), ("30-governance-corrected.log", (23, 0)),
    ("34-final-root-tests.log", (142, 0)),
]:
    assert totals(name) == expected, name
assert len(manifest["new_integration_test_names"]) == 16
assert len(manifest["new_unit_test_names"]) == 2
for name in ("19-repeat-1.log", "20-repeat-2.log", "21-repeat-3.log"):
    assert totals(name) == (16, 0)
    raw = (packet / name).read_text(encoding="utf-8")
    for test in manifest["new_integration_test_names"]:
        assert f"test {test} ... ok" in raw, (name, test)
for name, marker in [
    ("01-baseline.log", "ExecutableFileBusy"), ("02-missing-api.log", "could not find `intent_projection`"),
    ("18-root-tests.log", "unregistered cfg-gated target"),
    ("29-governance-registered.log", "unknown field `mode`"),
    ("26-local-contract.log", "OK (local, non-promotable)"),
    ("27-ship-red.log", "ship admission is closed"),
    ("28-qualification-red.log", "contract input digest mismatch"),
    ("31-doc-check.log", "doc-check: OK"), ("32-book.log", "Book building has started"),
    ("33-prior-packet.log", "AssertionError"), ("35-final-doc-check.log", "doc-check: OK"),
    ("36-final-local-contract.log", "OK (local, non-promotable)"),
    ("37-prior-packet-corrected.log", "verified 37 closed captures"),
]:
    assert marker in (packet / name).read_text(encoding="utf-8"), name
for name in ("07-windows-check.log", "09-windows-clippy.log", "10-linux-clippy.log",
             "14-linux-check.log", "15-linux-msrv.log", "17-windows-msrv.log", "25-xtask-clippy.log"):
    raw = (packet / name).read_text(encoding="utf-8")
    assert "Finished `dev` profile" in raw and "error:" not in raw, name
assert "Finished `dev` profile" in (packet / "22-linux-rustdoc.log").read_text(encoding="utf-8")
assert (packet / "23-format-check.log").read_bytes() == b""
qualification_path = repo / "docs/testing/performance/0.74/qualification-manifest.toml"
assert hashlib.sha256(qualification_path.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
qualification = tomllib.loads(qualification_path.read_text(encoding="utf-8"))
mismatches = []
for entry in qualification["contract_inputs"]:
    committed = git("show", manifest["baseline_source"] + ":" + entry["path"])
    assert (repo / entry["path"]).read_bytes().replace(b"\r\n", b"\n") == committed, entry["path"]
    if hashlib.sha256(committed).hexdigest() != entry["sha256"]:
        mismatches.append(entry["path"])
assert mismatches == ["docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"]
registry = tomllib.loads((repo / "docs/testing/release-evidence/0.74.toml").read_text(encoding="utf-8"))
w11 = next(item for item in registry["work_item"] if item["id"] == "W11")
names = manifest["new_integration_test_names"] + manifest["new_unit_test_names"]
assert set(names) <= {item["function"] for item in w11["required_tests"]}
assert str(packet.relative_to(repo)).replace("\\", "/") + "/manifest.json" in w11["required_artifacts"]
gated = tomllib.loads((repo / "docs/testing/gated-test-registry.toml").read_text(encoding="utf-8"))
gate = next(g for g in gated["gate"] if g["id"].endswith(".diagnostic-intent-projection"))
assert gate["command"]["platform"] == "linux"
assert gate["ci"]["step"] == "Release 0.74 Linux supervisor contracts"
links = 0
for relative in [
    "docs/articles/007-optimization-starts-with-measurement.md",
    "docs/testing/performance/0.74/README.md",
    "docs/testing/performance/0.74/production-readiness-and-server-launch-20261011.md",
    "docs/testing/performance/0.74/diagnostic-intent-projection-local-design.md",
    str(packet.relative_to(repo) / "README.md"),
]:
    source = repo / relative
    for target in re.findall(r"\]\(([^)]+)\)", source.read_text(encoding="utf-8")):
        target = unquote(target.split("#", 1)[0])
        if not target or "://" in target or target.startswith("#"):
            continue
        assert (source.parent / target).is_file(), (relative, target)
        links += 1
print(f"verified {len(files)} closed captures, source binding, 18 new tests, frozen inputs and {links} local links; not production/admission")
