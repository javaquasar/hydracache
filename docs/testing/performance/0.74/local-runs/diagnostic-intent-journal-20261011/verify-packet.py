"""Read-only local journal evidence verification; no launch or admission."""
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


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def git(*args):
    return subprocess.check_output(["git", "-C", str(repo), *args])


def text(name):
    return (packet / name).read_text(encoding="utf-8")


def totals(name):
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", text(name))
    assert rows, name
    return sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)


def verify_raw(directory, entries):
    assert len({f["name"] for f in entries}) == len(entries)
    assert {p.name for p in directory.glob("*.log")} == {f["name"] for f in entries}
    for entry in entries:
        assert Path(entry["name"]).name == entry["name"], entry
        raw = (directory / entry["name"]).read_bytes()
        assert len(raw) == entry["bytes"], entry["name"]
        assert digest(raw) == entry["sha256"], entry["name"]


assert manifest["schema_version"] == "diagnostic-intent-journal-local-evidence-074-v1"
assert manifest["state"] == "retained-local-fixture-consumption-not-execution"
assert manifest["evidence_class"] == "working-tree-local-safety-not-exact-candidate-release-receipt"
assert len(manifest["files"]) == 49
verify_raw(packet, manifest["files"])
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", manifest[prefix + "_source"] + "^{tree}").decode().strip() == manifest[prefix + "_tree"]
for path, expected in manifest["implementation_source_sha256"].items():
    committed = git("show", manifest["implementation_source"] + ":" + path)
    assert digest(committed) == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == committed, path

for name, expected in [
    ("01-baseline.log", (451, 1)), ("07-created-record-regression-red.log", (1, 0)),
    ("11-journal-hardened.log", (18, 0)), ("17-linux-scoped.log", (470, 1)),
    ("18-doctests.log", (6, 0)), ("20-windows-tests.log", (195, 0)),
    ("23-root-tests.log", (143, 0)), ("24-repeat-1.log", (18, 0)),
    ("25-repeat-2.log", (18, 0)), ("26-repeat-3.log", (18, 0)),
    ("33-final-linux-scoped.log", (471, 1)),
    ("38-final-root-corrected.log", (126, 0)), ("45-final-governance.log", (23, 0)),
]:
    assert totals(name) == expected, name
names = manifest["new_unit_test_names"]
assert len(names) == len(set(names)) == 19
for name in ("30-repeat-final-1.log", "31-repeat-final-2.log", "32-repeat-final-3.log"):
    assert totals(name) == (19, 0), name
    raw = text(name)
    for test in names:
        assert f"::{test} ... ok" in raw, (name, test)
assert "test signed_intent_journal_api_is_fixture_data_not_execution ... ok" in text("33-final-linux-scoped.log")
assert "test diagnostic_intent_journal_preserves_consumption_without_execution ... ok" in text("38-final-root-corrected.log")
for name, marker in [
    ("02-missing-api.log", "could not find `intent_journal`"),
    ("04-first-journal.log", "the compiler unexpectedly panicked"),
    ("06-journal-expanded.log", "error[E0507]"),
    ("08-journal-first-runtime.log", "5 passed; 8 failed"),
    ("09-created-record-regression-corrected-fixture.log", "new named inode with identical bytes"),
    ("39-local-contract.log", "OK (local, non-promotable)"),
    ("40-qualification-red.log", "contract input digest mismatch"),
    ("41-prior-working-source-red.log", "AssertionError: tools/long-run-supervisor-074/src/diagnostic_intent_projection.rs"),
    ("42-ship-red.log", "ship admission is closed"),
    ("43-doc-check.log", "doc-check: OK"), ("44-book.log", "Book building has started"),
    ("37-final-root-tests.log", "no test target named `cfg_gated_tests`"),
    ("46-final-doc-check.log", "doc-check: OK"), ("47-final-book.log", "Book building has started"),
    ("48-final-local-contract.log", "OK (local, non-promotable)"),
]:
    assert marker in text(name), (name, marker)
assert "0 passed; 1 failed" in text("09-created-record-regression-corrected-fixture.log")
for name in (
    "12-linux-clippy.log", "13-linux-check.log", "14-windows-clippy.log",
    "15-windows-check.log", "19-linux-msrv.log", "21-windows-msrv.log",
    "27-linux-clippy-final.log", "29-xtask-clippy.log", "34-final-linux-clippy.log",
    "35-final-linux-msrv.log", "36-linux-rustdoc.log",
):
    raw = text(name)
    assert "Finished `dev` profile" in raw and "error:" not in raw, name
for name in ("03-format.log", "05-format-expanded.log", "10-format-hardened.log",
             "16-format-final.log", "22-format-review.log", "28-format-last.log", "49-final-format.log"):
    assert (packet / name).read_bytes() == b"", name

# Old evidence stays historical: verify its Git source, not its equality to new code.
prior_path = repo / manifest["historical_packet"]
prior_raw = (prior_path / "manifest.json").read_bytes()
assert digest(prior_raw) == manifest["historical_manifest_sha256"]
prior = json.loads(prior_raw)
verify_raw(prior_path, prior["files"])
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", prior[prefix + "_source"] + "^{tree}").decode().strip() == prior[prefix + "_tree"]
for path, expected in prior["implementation_source_sha256"].items():
    assert digest(git("show", prior["implementation_source"] + ":" + path)) == expected, path

qualification_path = repo / "docs/testing/performance/0.74/qualification-manifest.toml"
qualification_raw = qualification_path.read_bytes()
assert digest(qualification_raw) == manifest["qualification_manifest_sha256"]
qualification = tomllib.loads(qualification_raw.decode())
assert len(qualification["contract_inputs"]) == 8
mismatches = []
for entry in qualification["contract_inputs"]:
    committed = git("show", manifest["baseline_source"] + ":" + entry["path"])
    assert (repo / entry["path"]).read_bytes().replace(b"\r\n", b"\n") == committed, entry["path"]
    if digest(committed) != entry["sha256"]:
        mismatches.append(entry["path"])
assert mismatches == ["docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"]
registry = tomllib.loads((repo / "docs/testing/release-evidence/0.74.toml").read_text(encoding="utf-8"))
w11 = next(item for item in registry["work_item"] if item["id"] == "W11")
assert set(names + ["signed_intent_journal_api_is_fixture_data_not_execution",
                    "diagnostic_intent_journal_preserves_consumption_without_execution"]) <= {t["function"] for t in w11["required_tests"]}
assert packet.relative_to(repo).as_posix() + "/manifest.json" in w11["required_artifacts"]
contract = tomllib.loads((repo / "docs/testing/performance/0.74/diagnostic-intent-journal-local-contract.toml").read_text(encoding="utf-8"))
assert contract["max_cells"] == 4 and contract["max_record_bytes"] == 16384
for key in ("production_root_backend", "actual_once_only_execution_proved",
            "host_mutation_allowed", "qualification_or_admission_allowed", "performance_claim_allowed"):
    assert contract[key] is False, key
for path in (
    packet / "README.md", repo / "docs/testing/performance/0.74/diagnostic-intent-journal-local-design.md",
    repo / "docs/testing/performance/0.74/README.md",
    repo / "docs/testing/performance/0.74/production-readiness-and-server-launch-20261011.md",
    repo / "docs/articles/007-optimization-starts-with-measurement.md",
):
    for target in re.findall(r"\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
        if "://" in target or target.startswith("#"):
            continue
        assert (path.parent / unquote(target.split("#")[0])).exists(), (path, target)
print(f"verified {len(manifest['files'])} closed captures, 19 repeated journal tests, historical packet and unchanged frozen inputs; not execution or admission")
