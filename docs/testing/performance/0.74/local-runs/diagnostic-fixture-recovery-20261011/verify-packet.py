"""Read-only local recovery safety verifier, never launch/admission authority."""
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
assert manifest["state"] == "retained-local-fixture-recovery-not-production-intent"


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
for name, expected in [
    ("01-baseline-linux.log", (403, 1)),
    ("05-recovery-tests-corrected-fixture.log", (17, 0)),
    ("08-linux-scoped-tests.log", (494, 1)),
    ("09-windows-tests.log", (195, 0)),
    ("15-linux-doctests.log", (3, 0)),
    ("25-root-contract-corrected.log", (105, 0)),
    ("26-root-registries.log", (36, 0)),
    ("43-final-root-tests-windows.log", (141, 0)),
]:
    assert totals(name) == expected, name
names = manifest["new_unit_test_names"]
assert len(names) == 14
for filename in ("17-repeat-1.log", "18-repeat-2.log", "19-repeat-3.log"):
    assert totals(filename) == (16, 0)
    raw = (packet / filename).read_text(encoding="utf-8")
    for name in names:
        assert f"::{name} ... ok" in raw, (filename, name)
for name, marker in [
    ("02-missing-api-red.log", "could not find `recovery`"),
    ("03-fixture-tests-attempt.log", "FAILED. 6 passed; 2 failed"),
    ("04-recovery-tests-attempt.log", "FAILED. 13 passed; 3 failed"),
    ("12-root-contract.log", "missing invariant file.read_at"),
    ("24-qualification-dry-red.log", "contract input digest mismatch"),
    ("28-local-performance-contract.log", "OK (local, non-promotable)"),
    ("29-ship-contract-red.log", "ship admission is closed"),
    ("30-ship-admission-red.log", "rejected non-green evidence"),
    ("31-final-root-tests-linux.log", "git rev-parse HEAD failed"),
    ("34-linux-git-preflight-red.log", "fatal: not a git repository"),
    ("35-linux-git-scoped-preflight.log", "fatal: not a git repository"),
    ("44-book-windows.log", "Book building has started"),
    ("45-prior-packet-windows.log", "verified 37 closed captures"),
]:
    assert marker in (packet / name).read_text(encoding="utf-8"), name
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
assert set(names) <= {item["function"] for item in w11["required_tests"]}
assert str(packet.relative_to(repo)).replace("\\", "/") + "/manifest.json" in w11["required_artifacts"]
links = 0
for relative in [
    "docs/articles/007-optimization-starts-with-measurement.md",
    "docs/testing/performance/0.74/README.md",
    "docs/testing/performance/0.74/production-readiness-and-server-launch-20261011.md",
    "docs/testing/performance/0.74/diagnostic-signed-authority-local-design.md",
    "docs/testing/performance/0.74/diagnostic-fixture-recovery-local-design.md",
    str(packet.relative_to(repo) / "README.md"),
]:
    source = repo / relative
    for target in re.findall(r"!?\[[^\]]*\]\(([^\s)]+)\)", source.read_text(encoding="utf-8")):
        if target.startswith(("http:", "https:", "mailto:", "#", "codex:", "plugin:")):
            continue
        target = unquote(target.split("#", 1)[0].split("?", 1)[0])
        assert (source.parent / target).is_file(), (relative, target)
        links += 1
assert links > 0
print(f"verified {links} local Markdown file targets")
print(f"verified {len(files)} closed captures; fixture recovery safety only; frozen qualification refusal unchanged")
