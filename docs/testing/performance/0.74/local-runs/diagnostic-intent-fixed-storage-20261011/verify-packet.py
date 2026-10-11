"""Read-only safety evidence; never publication, enrollment or admission."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tomllib
from urllib.parse import unquote

packet = Path(__file__).resolve().parent
repo = next(p for p in packet.parents if (p / "Cargo.toml").is_file())
m = json.loads((packet / "manifest.json").read_bytes())


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


def raw_files(directory, entries):
    assert len({e["name"] for e in entries}) == len(entries)
    assert {p.name for p in directory.glob("*.log")} == {e["name"] for e in entries}
    for e in entries:
        assert Path(e["name"]).name == e["name"], e
        raw = (directory / e["name"]).read_bytes()
        assert len(raw) == e["bytes"] and digest(raw) == e["sha256"], e["name"]


assert m["state"] == "retained-readonly-fixed-intent-storage-not-publisher"
assert len(m["files"]) == 27
raw_files(packet, m["files"])
for prefix in ("baseline", "implementation"):
    assert git("rev-parse", m[prefix + "_source"] + "^{tree}").decode().strip() == m[prefix + "_tree"]
for path, expected in m["implementation_source_sha256"].items():
    committed = git("show", m["implementation_source"] + ":" + path)
    assert digest(committed) == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == committed, path
for name, expected in [("01-baseline.log", (19, 0)), ("04-first-tests.log", (7, 0)),
                       ("05-linux-scoped.log", (479, 1)), ("12-doctests.log", (8, 0)),
                       ("17-root-tests.log", (150, 0))]:
    assert totals(name) == expected, name
names = m["new_unit_test_names"]
assert len(names) == len(set(names)) == 7
for name in ("06-repeat-1.log", "07-repeat-2.log", "08-repeat-3.log"):
    assert totals(name) == (7, 0), name
    for test in names:
        assert f"::{test} ... ok" in text(name), (name, test)
assert "test fixed_intent_storage_api_is_readonly_without_public_selectors ... ok" in text("05-linux-scoped.log")
assert "test diagnostic_fixed_intent_storage_is_readonly_not_backend_enrollment ... ok" in text("17-root-tests.log")
for name, marker in [("02-missing-api.log", "could not find `fixed_intent_storage`"),
                     ("03-format-red.log", "Diff in"),
                     ("20-qualification-red.log", "contract input digest mismatch"),
                     ("21-local-contract.log", "OK (local, non-promotable)"),
                     ("22-ship-red.log", "ship admission is closed"),
                     ("23-doc-check.log", "doc-check: OK"),
                     ("24-book.log", "Book building has started"),
                     ("25-final-doc-check.log", "doc-check: OK"),
                     ("26-final-book.log", "Book building has started"),
                     ("27-final-local-contract.log", "OK (local, non-promotable)")]:
    assert marker in text(name), (name, marker)
for name in ("09-linux-clippy.log", "10-linux-check.log", "11-linux-msrv.log",
             "13-linux-rustdoc.log", "14-windows-check.log", "15-windows-clippy.log",
             "16-windows-msrv.log", "18-xtask-clippy.log"):
    assert "Finished `dev` profile" in text(name) and "error:" not in text(name), name
assert (packet / "19-format-check.log").read_bytes() == b""

# Retain prior source-bound proof as historical, never substitute current source.
prior_path = repo / m["historical_packet"]
prior_raw = (prior_path / "manifest.json").read_bytes()
assert digest(prior_raw) == m["historical_manifest_sha256"]
prior = json.loads(prior_raw)
raw_files(prior_path, prior["files"])
assert git("rev-parse", prior["implementation_source"] + "^{tree}").decode().strip() == prior["implementation_tree"]
for path, expected in prior["implementation_source_sha256"].items():
    assert digest(git("show", prior["implementation_source"] + ":" + path)) == expected, path
q_path = repo / "docs/testing/performance/0.74/qualification-manifest.toml"
assert digest(q_path.read_bytes()) == m["qualification_manifest_sha256"]
q = tomllib.loads(q_path.read_text(encoding="utf-8"))
assert len(q["contract_inputs"]) == 8
mismatches = []
for entry in q["contract_inputs"]:
    old = git("show", m["baseline_source"] + ":" + entry["path"])
    assert (repo / entry["path"]).read_bytes().replace(b"\r\n", b"\n") == old, entry["path"]
    if digest(old) != entry["sha256"]:
        mismatches.append(entry["path"])
assert mismatches == ["docs/plans/V0_74_RESP_THROUGHPUT_AND_PIPELINE_EFFICIENCY_PLAN.md"]
registry = tomllib.loads((repo / "docs/testing/release-evidence/0.74.toml").read_text(encoding="utf-8"))
w11 = next(w for w in registry["work_item"] if w["id"] == "W11")
assert set(names + ["fixed_intent_storage_api_is_readonly_without_public_selectors",
                    "diagnostic_fixed_intent_storage_is_readonly_not_backend_enrollment"]) <= {t["function"] for t in w11["required_tests"]}
assert packet.relative_to(repo).as_posix() + "/manifest.json" in w11["required_artifacts"]
contract = tomllib.loads((repo / "docs/testing/performance/0.74/diagnostic-intent-fixed-storage-local-contract.toml").read_text(encoding="utf-8"))
assert contract["read_only"] and contract["private_leaf_mode"] == 0o700
for key in ("intent_publication_implemented", "host_fence_or_authority_join_implemented",
            "host_enrollment_proved", "execution_or_admission_allowed", "host_mutation_allowed", "performance_claim_allowed"):
    assert contract[key] is False, key
provision_path = "scripts/perf/long-run-supervisor-074/hydracache-performance-074.tmpfiles.conf"
assert (repo / provision_path).read_bytes().replace(b"\r\n", b"\n") == git("show", m["baseline_source"] + ":" + provision_path)
for path in (packet / "README.md", repo / "docs/testing/performance/0.74/README.md",
             repo / "docs/testing/performance/0.74/diagnostic-intent-fixed-storage-local-design.md",
             repo / "docs/testing/performance/0.74/production-readiness-and-server-launch-20261011.md",
             repo / "docs/articles/007-optimization-starts-with-measurement.md"):
    for target in re.findall(r"\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
        if "://" in target or target.startswith("#"):
            continue
        assert (path.parent / unquote(target.split("#")[0])).exists(), (path, target)
print("verified 27 closed captures, 7 repeated storage tests, historical journal and unchanged frozen inputs/provisioning; no publisher or admission")
