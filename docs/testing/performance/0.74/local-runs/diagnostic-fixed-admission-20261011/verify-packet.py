"""Verify local fixed diagnostic admission; never authorize host or worker start."""
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


def capture(name):
    return (packet / name).read_text(encoding="utf-8")


def raw_files(directory, entries):
    assert {p.name for p in directory.glob("*.log")} == {e["name"] for e in entries}
    assert len(entries) == len({e["name"] for e in entries})
    for e in entries:
        assert Path(e["name"]).name == e["name"]
        raw = (directory / e["name"]).read_bytes()
        assert len(raw) == e["bytes"] and digest(raw) == e["sha256"], e["name"]


def totals(name):
    rows = re.findall(r"test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;", capture(name))
    assert rows, name
    return [sum(int(p) for p, _ in rows), sum(int(i) for _, i in rows)]


assert m["schema_version"] == 1
assert m["state"] == "retained-fixed-context-campaign-admission-local"
assert m["canonical_fixed_predicate_in_real_algorithms_tested"] is True
assert m["context_after_short_fence_drop_excludes_modeled_campaign"] is True
expected_closed = {
    "host_operation_performed", "installer_or_ownership_changed",
    "trusted_state_publication_implemented", "fixed_publisher_implemented",
    "authority_intent_join_implemented", "host_enrollment_proved",
    "publisher_serialization_proved", "worker_execution_allowed",
    "qualification_allowed", "ship_allowed", "performance_claim_allowed",
}
assert set(m["closed_permissions"]) == expected_closed
assert all(v is False for v in m["closed_permissions"].values())
assert m["test_environment"]["root_for_local_tests"] is False
raw_files(packet, m["files"])

for prefix in ("baseline", "implementation"):
    assert git("rev-parse", m[prefix + "_source"] + "^{tree}").decode().strip() == m[prefix + "_tree"]
for older, newer in (("baseline_source", "preregistration_source"),
                     ("preregistration_source", "implementation_source")):
    assert git("merge-base", m[older], m[newer]).decode().strip() == m[older]
for path, expected in m["implementation_source_sha256"].items():
    raw = git("show", m["implementation_source"] + ":" + path)
    assert digest(raw) == expected, path
    assert (repo / path).read_bytes().replace(b"\r\n", b"\n") == raw, path
for path in m["unchanged_legacy_sources"]:
    assert git("show", m["baseline_source"] + ":" + path) == git("show", m["implementation_source"] + ":" + path), path

for name, counts in m["test_totals"].items():
    assert totals(name) == counts, name
assert totals("21-final-linux-regression.log") == [526, 1]
assert totals("25-windows-regression.log") == [196, 0]
assert totals("26-root-regression.log") == [155, 0]
assert totals("39-final-linux-doctests.log") == [18, 0]
assert "reads_a_real_loaded_service_over_dbus ... ignored" in capture("21-final-linux-regression.log")
for name in ("18-final-repeat-1.log", "19-final-repeat-2.log", "20-final-repeat-3.log"):
    assert totals(name) == [13, 0]
    for test in m["repeated_unit_test_names"]:
        assert "::" + test + " ... " in capture(name), (name, test)
    assert "seed=0x747" in capture(name)
assert len(m["repeated_unit_test_names"]) == 13
assert len(set(m["repeated_unit_test_names"])) == 13
for name, marker in m["required_log_markers"].items():
    assert marker in capture(name), (name, marker)
for name in m["green_compile_logs"]:
    assert "Finished `dev` profile" in capture(name) and "error:" not in capture(name), name
assert (packet / "32-format.log").read_bytes() == b""
for e in m["files"]:
    if e["observed_exit"] is None:
        assert 13 <= int(e["name"][:2]) <= 17 and e["stage"] == "intermediate"
        assert e["exit_note"]
    else:
        assert e["observed_exit"] == e["expected_exit"], e["name"]

# Historical receipts bind their recorded source, not today's extended source.
historical = repo / m["historical_packet"]
prior_raw = (historical / "manifest.json").read_bytes()
assert digest(prior_raw) == m["historical_manifest_sha256"]
prior = json.loads(prior_raw)
raw_files(historical, prior["files"])
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
assert len(m["new_tests"]) == 14
assert {(t["source"], t["function"]) for t in m["new_tests"]} <= {
    (t["source"], t["function"]) for t in w11["required_tests"]}
for t in m["new_tests"]:
    assert "fn " + t["function"] + "(" in (repo / t["source"]).read_text(encoding="utf-8"), t
assert packet.relative_to(repo).as_posix() + "/manifest.json" in w11["required_artifacts"]
for path in (
    "tools/long-run-supervisor-074/src/host_execution.rs",
    "tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission.rs",
    "tools/long-run-supervisor-074/src/diagnostic_fixed_campaign_admission_tests.rs",
    "tools/long-run-supervisor-074/src/host_execution_fixed_admission_tests.rs",
    "docs/testing/performance/0.74/diagnostic-fixed-admission-local-contract.toml",
):
    assert path in w11["required_sources"], path
contract = tomllib.loads((repo / "docs/testing/performance/0.74/diagnostic-fixed-admission-local-contract.toml").read_text(encoding="utf-8"))
assert contract["baseline_source"] == m["baseline_source"]
for key in (
    "canonical_production_campaign_root_dispatch",
    "all_fixed_leaf_entries_block_campaign_admission",
    "missing_or_unsafe_storage_refuses_without_repair",
    "original_storage_directories_and_common_lock_retained",
    "admission_and_recovery_check_under_original_common_lock",
    "pre_and_post_marker_checks_required",
    "late_failure_retains_existing_or_created_marker",
    "legacy_diagnostic_lock_entry_checks_fixed_context",
    "readonly_absence_is_not_a_fenced_capability",
    "fresh_descriptor_relative_directory_scan_each_check",
    "unknown_pending_terminal_symlink_context_is_not_ignored",
    "nonproduction_fixtures_and_nonlinux_behavior_unchanged",
    "legacy_campaign_ownership_is_not_root_authority",
):
    assert contract[key] is True, key
for key in (
    "installer_or_ownership_changed", "trusted_state_publication_implemented",
    "authority_intent_join_implemented", "fixed_intent_publication_implemented",
    "positive_host_enrollment_proved", "worker_execution_allowed",
    "qualification_allowed", "ship_allowed", "server_operation_performed",
    "performance_claim_allowed",
):
    assert contract[key] is False, key
for path in m["documentation_link_sources"]:
    for target in re.findall(r"\]\(([^)]+)\)", (repo / path).read_text(encoding="utf-8")):
        if target.startswith(("http:", "https:", "#", "/")):
            continue
        assert ((repo / path).parent / unquote(target.split("#")[0])).exists(), (path, target)
print("fixed diagnostic admission packet verified; host enrollment, publisher and start remain closed")
