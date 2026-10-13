"""Read-only collector retention verification; never production admission."""
import hashlib
import json
import re
import subprocess
import tomllib
from pathlib import Path

P = Path(__file__).resolve().parent
R = P.parents[5]
M = json.loads((P / "manifest.json").read_text(encoding="utf-8"))


def lf(raw):
    return raw.replace(b"\r\n", b"\n")


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def git(commit, path):
    return lf(subprocess.check_output(["git", "show", f"{commit}:{path}"], cwd=R))


def passed(log):
    return sum(map(int, re.findall(r"test result: ok\. (\d+) passed;", log)))


assert M["baseline_source"] == "802d1a20ea66d94df219a8c787803e82fe58e950"
assert M["implementation_source"] == "186d30bab68ce541050c8c638c047bdc49021086"
assert M["preregistration_sources"] == ["0700f9464e0ff15d87bdd21b80c01ff94ef25471","84fc664278fe428571e786c7d0839b7aabb79e9b"]
for path in ("docs/testing/performance/0.74/diagnostic-collector-design.md",
             "docs/testing/performance/0.74/diagnostic-collector-contract.toml"):
    raw = git(M["preregistration_sources"][-1], path)
    assert raw == git(M["implementation_source"], path)
    assert raw == lf((R / path).read_bytes())
for commit in [M["baseline_source"], M["implementation_source"], *M["preregistration_sources"]]:
    assert re.fullmatch(r"[0-9a-f]{40}", commit)
    subprocess.check_call(["git", "cat-file", "-e", commit + "^{commit}"], cwd=R)
assert sha(lf((P / "verify_packet.py").read_bytes())) == M["verifier_sha256_lf"]
sources = {"tools/long-run-supervisor-074/src/lib.rs",
           "tools/long-run-supervisor-074/src/diagnostic_collector.rs",
           "tools/long-run-supervisor-074/src/diagnostic_collector_tests.rs",
           "tools/long-run-supervisor-074/src/diagnostic_collector_crash_tests.rs",
           "tools/long-run-supervisor-074/tests/diagnostic_collector.rs",
           "docs/testing/gated-test-registry.toml", "docs/COMPAT.md", "docs/GATES.md"}
assert {entry["path"] for entry in M["source_files"]} == sources
for entry in M["source_files"]:
    raw = git(M["implementation_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((R / entry["path"]).read_bytes()) == raw
changed = set(subprocess.check_output([
    "git", "diff", "--name-only", M["baseline_source"], M["implementation_source"]
], cwd=R, text=True).splitlines())
assert changed == sources | {
    "docs/testing/performance/0.74/diagnostic-collector-design.md",
    "docs/testing/performance/0.74/diagnostic-collector-contract.toml",
    "docs/testing/performance/0.74/diagnostic-server-readiness-intermediate-plan-20261012.md"}
lib = "tools/long-run-supervisor-074/src/lib.rs"
declaration = b'#[cfg(target_os = "linux")]\npub mod diagnostic_collector;\n'
raw = git(M["implementation_source"], lib)
assert raw.count(declaration) == 1
assert raw.replace(declaration, b"") == git(M["baseline_source"], lib)
for prefix in ("crates", "scripts", "Cargo.toml", "Cargo.lock"):
    assert not subprocess.check_output([
        "git", "diff", "--name-only", M["baseline_source"], M["implementation_source"], "--", prefix
    ], cwd=R)
for entry in M["frozen_files"] + M["unchanged_sources"]:
    raw = git(M["baseline_source"], entry["path"])
    assert sha(raw) == entry["sha256_lf"]
    assert lf((R / entry["path"]).read_bytes()) == raw
assert len(M["frozen_files"]) == 8
assert sha((R / "docs/testing/performance/0.74/qualification-manifest.toml").read_bytes()) == M["qualification_sha256_raw"]
contract = tomllib.loads((R / "docs/testing/performance/0.74/diagnostic-collector-contract.toml").read_text())
assert contract["stream_bytes"] == 8388608 and contract["combined_bytes"] == 16777216
assert contract["pump_quantum_per_stream"] == 8192
for field in ("production_constructor_enrolled", "authenticated_original_start_implemented",
              "production_issuer_enrolled", "original_cgroup_revocation_implemented",
              "server_operation_allowed", "qualification_allowed", "ship_allowed", "performance_claim_allowed"):
    assert contract[field] is False
logs = {}
for entry in M["captures"]:
    assert entry["path"] == entry["id"] + ".log" and entry["id"] not in logs
    raw = (P / entry["path"]).read_bytes()
    assert len(raw) == entry["bytes"] and sha(raw) == entry["sha256"]
    if entry["id"] in M["retained_development_failures"]:
        assert entry["expected_exit"] == 0 and entry["observed_exit"] == 101
    else:
        assert entry["observed_exit"] == entry["expected_exit"]
    logs[entry["id"]] = raw.decode("utf-8")
assert len(logs) == M["capture_count"]
assert "E0432" in logs["red"]
assert "13 passed; 1 failed" in logs["first-tests"]
assert "collector_same_length_content_mutation_refuses" in logs["first-tests"]
assert "needless_range_loop" in logs["linux-check-msrv-clippy"]
assert "exited before DirectoryCreated" in logs["manual-1"]
for repetition in (1, 2, 3):
    fast = logs[f"exact-fast-{repetition}"]
    assert passed(fast) == 20 and "1 ignored" in fast
    manual = logs[f"exact-manual-{repetition}"]
    assert passed(manual) == 1 and manual.count("actual_sigkill=1 fresh_refusals=2 unchanged_retention=true") == 15
    assert "sigkill_cases=15 controls=1 fresh_refusals=32 all_owned_helpers_reaped=true production_authority=false" in manual
assert passed(logs["linux-lib"]) == 624 and "12 ignored" in logs["linux-lib"]
assert passed(logs["linux-final-integrations-and-doctests"]) == 123
assert "Doc-tests hydracache_long_run_supervisor_074" in logs["linux-final-integrations-and-doctests"]
assert "57 passed; 0 failed" in logs["linux-final-integrations-and-doctests"]
assert passed(logs["windows-all-tests"]) == 204
assert passed(logs["root-contracts"]) == 166
assert passed(logs["root-artifacts-final"]) == 13
assert "invalid qualification manifest" in logs["qualification-refusal"]
assert "contract input digest mismatch" in logs["qualification-refusal"]
assert "gated-test-check: OK" in logs["gated-check"]
assert "doc-check: OK" in logs["doc-check"]
for key in ("format", "linux-final-check-msrv-clippy", "linux-rustdoc",
            "windows-check", "windows-msrv", "windows-clippy", "book", "performance-contract"):
    assert key in logs
assert M["qualification_artifact_created"] is False
assert M["production_authority"] is False and M["performance_claim"] is False
print(f"collector packet verified: {len(logs)} raw captures; local EOF/retention only; production closed")
