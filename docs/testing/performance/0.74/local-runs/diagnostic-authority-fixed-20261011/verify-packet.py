"""Read-only local evidence verifier; never launch/admission authority."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib

packet = Path(__file__).resolve().parent
repo = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
manifest = json.loads((packet / "manifest.json").read_bytes())


def git(*args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def totals(name):
    text = (packet / name).read_text(encoding="utf-8")
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text)
    assert rows, name
    return sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)


files = manifest["files"]
assert len({f["name"] for f in files}) == len(files)
assert {p.name for p in packet.glob("*.log")} == {f["name"] for f in files}
for f in files:
    assert Path(f["name"]).name == f["name"]
    raw = (packet / f["name"]).read_bytes()
    assert len(raw) == f["bytes"] and hashlib.sha256(raw).hexdigest() == f["sha256"], f["name"]

for prefix in ("baseline", "implementation", "reviewed"):
    assert git("rev-parse", f"{manifest[prefix + '_source']}^{{tree}}").decode().strip() == manifest[prefix + "_tree"]

assert totals("final-supervisor-linux.log") == (419, 1)
assert totals("supervisor-windows.log") == (195, 0)
assert totals("root-contracts-windows.log") == (139, 0)
names = manifest["new_unit_test_names"]
assert len(names) == 9
for i in (1, 2, 3):
    name = f"final-repeat-linux-{i}.log"
    assert totals(name) == (23, 0)
    text = (packet / name).read_text(encoding="utf-8")
    for test in names:
        assert f"::{test} ... ok" in text, (name, test)
assert "could not find `fixed`" in (packet / "missing-api-red.log").read_text(encoding="utf-8")
red = (packet / "named-root-red.log").read_text(encoding="utf-8")
assert "assertion failed: writer.inspect(&p.trust, &p.context).is_err()" in red
assert "1 failed" in red
assert totals("root-review-green-linux.log") == (1, 0)
metadata_red = (packet / "root-review-repeat-linux-2.log").read_text(encoding="utf-8")
assert "authority_fixture_original_path_file_and_descriptor_drift_latch_without_refresh" in metadata_red
assert "FAILED. 22 passed; 1 failed" in metadata_red
for i in range(1, 6):
    assert totals(f"observable-drift-linux-{i}.log") == (1, 0)

qualification_path = repo / "docs/testing/performance/0.74/qualification-manifest.toml"
assert hashlib.sha256(qualification_path.read_bytes()).hexdigest() == manifest["qualification_manifest_sha256"]
qualification = tomllib.loads(qualification_path.read_text(encoding="utf-8"))
mismatches = []
for item in qualification["contract_inputs"]:
    path = item["path"]
    committed = git("show", f"{manifest['baseline_source']}:{path}")
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == committed, path
    if hashlib.sha256(committed).hexdigest() != item["sha256"]:
        mismatches.append(path)
assert mismatches == ["docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"]
assert "contract input digest mismatch" in (packet / "qualification-dry-red.log").read_text(encoding="utf-8")
assert "error" in (packet / "ship-refusal-windows.log").read_text(encoding="utf-8").lower()
print(f"verified {len(files)} immutable logs; local safety only; inherited qualification mismatch retained")
