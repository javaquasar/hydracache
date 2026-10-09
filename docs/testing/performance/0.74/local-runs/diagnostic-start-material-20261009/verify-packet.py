"""Read-only capture integrity/count guard, not a live backend verifier."""
import hashlib
import json
from pathlib import Path
import re

root = Path(__file__).resolve().parent
packet = json.loads((root / "manifest.json").read_bytes())
listed = packet["retention"]["raw_files"]
assert len(listed) == 7
assert {p.name for p in root.glob("*.log")} == {p["name"] for p in listed}
for entry in listed:
    assert Path(entry["name"]).name == entry["name"]
    raw = (root / entry["name"]).read_bytes()
    assert len(raw) == entry["bytes"], entry["name"]
    assert hashlib.sha256(raw).hexdigest() == entry["sha256"], entry["name"]
    assert b"PRIVATE KEY" not in raw


def passed(name):
    return sum(map(int, re.findall(rb"test result: ok\. (\d+) passed", (root / name).read_bytes())))


assert passed("artifacts-green.log") == 28
assert passed("linux-focused.log") == 181
assert passed("linux-budget-green.log") == 182
assert passed("windows-artifacts.log") == 15
assert passed("root-contracts.log") == 114
assert b"cannot find type `PinnedStartMaterial`" in (root / "test-first-red.log").read_bytes()
negative = (root / "starting-budget-red.log").read_bytes()
assert b"left: 60" in negative and b"right: 17" in negative
repository = next(p for p in root.parents if (p / "Cargo.toml").is_file())
qualification = repository / "docs/testing/performance/0.74/qualification-manifest.toml"
assert hashlib.sha256(qualification.read_bytes()).hexdigest() == packet["proof_boundaries"]["qualification_manifest_sha256"]
assert not packet["proof_boundaries"]["execution_or_admission_allowed"]
print("PASS: seven captured files, byte digests, counts, retained refusals and unchanged qualification pin")
