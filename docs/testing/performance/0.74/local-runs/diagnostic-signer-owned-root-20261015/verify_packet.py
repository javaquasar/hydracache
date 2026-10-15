"""Unsigned development capture consistency; not an execution or trust receipt."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "97e1bde1481394b387b47bd99497afb2d2b8ac93"
TREE = "81d80384fbdce09fbe0c7ba45b55ff39a821f1e6"
BASELINE = "81c327d1fed7a2284d4ebd96a8e80f2961e0787b"
PREREG = "7c27c3c6b66efdcefa61b916bfe3210852354255"
CLOSED = set(["all_regressions_passed","broad_constructor_loss_failure_fixed","msrv_normal_root_materialized","source_pathnames_authenticated","hostile_same_uid_cleanup_race_prevented","normal_child_launched","initial_namespace_provenance_authenticated","loaded_runtime_attested","independent_runtime_custody_authenticated","real_credentials_or_signing","authenticated_original_start","all_writers_revoked","completion_or_later_cells_enabled","server_or_github_mutated","qualification_allowed","ship_allowed","performance_claim_allowed","full_release_gate_passed","clean_checkout_execution_attested"])
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml": "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
}
IMPLEMENTATION = set(["crates/xtask/tests/performance_contract_074.rs","docs/testing/gated-test-registry.toml","docs/testing/performance/0.74/diagnostic-signer-owned-root-contract.toml","tools/long-run-supervisor-074/src/bin/diagnostic_signer_root_probe.rs","tools/long-run-supervisor-074/src/diagnostic_signer_owned_root.rs","tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_tests.rs","tools/long-run-supervisor-074/tests/diagnostic_signer_root_cli.rs","tools/long-run-supervisor-074/tests/diagnostic_signer_root_probe.rs"])
ORACLES = set(["tools/long-run-supervisor-074/src/lib.rs","tools/long-run-supervisor-074/src/diagnostic_signer_owned_probe.rs","tools/long-run-supervisor-074/src/diagnostic_signer_isolation.rs","tools/long-run-supervisor-074/src/diagnostic_signer_isolation_linux.rs","tools/long-run-supervisor-074/src/diagnostic_signer_gnu.rs","tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs","tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs","tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs","tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs"])
EXPECTED_LOGS = {
    "baseline-old-root-failed.log": {
        "sha256": "932dbd554732825d78d36dd34b2b360b47b866afa1e021a05cdba4418cc61a99",
        "bytes": 29708,
        "exit_code": 1
    },
    "current-final.log": {
        "sha256": "f79216e4349bdc433b87595be13f7a4c91e709625f053dcd41b9f2561a6997ff",
        "bytes": 2839,
        "exit_code": 0
    },
    "current-provisional.log": {
        "sha256": "44f857bb467ed4a1709a51d4da8f280777a7ebff90405601ec70825bc1c42fc9",
        "bytes": 2303,
        "exit_code": 0
    },
    "dynamic-declarations.log": {
        "sha256": "0a4efba9078702963c2c28d17baf9bc89eee723c917a695d410c0e38ad36d331",
        "bytes": 3288,
        "exit_code": 0
    },
    "gnu-owner-regressions.log": {
        "sha256": "f3a4b825aad8310353e02397a79de90527e14c14b197f16d38f60e6595e3930c",
        "bytes": 4314,
        "exit_code": 0
    },
    "missing-target.log": {
        "sha256": "28d73418cf91b56528dbb255e3124fa46d166cc2697f1a48dbb7e4c6a7b9ca8c",
        "bytes": 295,
        "exit_code": 1
    },
    "msrv-extra-libm-failed.log": {
        "sha256": "da534b2a65b15c9b431e092fef097c80604d7aea5ddba048d68857128670ba7e",
        "bytes": 4294,
        "exit_code": 1
    },
    "msrv-final.log": {
        "sha256": "419c03b15e653332f5c44609fa710f7b63daf549c599904db2c656f2d3363f3e",
        "bytes": 3068,
        "exit_code": 0
    },
    "old-root-constructor-focused.log": {
        "sha256": "fc3f8784eb8da798bdd4aaba88100c9eaadb32be3df520727637d4b7b3c16ef1",
        "bytes": 6042,
        "exit_code": 0
    },
    "root-contracts-final.log": {
        "sha256": "d379234f285b40a2bfc6f4bfe8c3cb141e081aa213b3d26d96a1307f34b48b25",
        "bytes": 470,
        "exit_code": 0
    },
    "source-name-assumption-failed.log": {
        "sha256": "dbdb5d18924be6e3f393ba6037be8c24123a84658fa90422574eaf5ae75cd20e",
        "bytes": 2121,
        "exit_code": 101
    },
    "static-final.log": {
        "sha256": "b1a2ab36e294353c6b721c7787bfe1ab816be949ad01c0393b888626cff71485",
        "bytes": 1325,
        "exit_code": 0
    },
    "unit-symlink-setup-failed.log": {
        "sha256": "93fd2f1717eade074caeebaa207f2c1adb3c73a8d01f1bcac627fbe7e6ea2aa9",
        "bytes": 1896,
        "exit_code": 1
    },
    "windows-check-lint-final.log": {
        "sha256": "d05bcb5be5094c360331e56ed4c70116b39d2243f000e946f1d084ee7c7c4044",
        "bytes": 707,
        "exit_code": 0
    },
    "windows-cli-lint.log": {
        "sha256": "654a8377f1b4c552ba15200b8f2c5d2d02b22b6aaeb49e7b22f03885222dbf7d",
        "bytes": 1256,
        "exit_code": 0
    },
    "windows-concurrent-build-failed.log": {
        "sha256": "ea0165b683885e6c44030a6fdd5b1154648e870443c00610a63cbda56925ae37",
        "bytes": 495,
        "exit_code": 1
    }
}
RESULTS = {
    "baseline-old-root-failed.log": [("FAILED", 0, 1), ("FAILED", 78, 1)],
    "missing-target.log": [],
    "unit-symlink-setup-failed.log": [("FAILED", 8, 1)],
    "current-provisional.log": [("ok", 9, 0), ("ok", 1, 0), ("ok", 3, 0)],
    "msrv-extra-libm-failed.log": [("ok", 9, 0), ("ok", 1, 0), ("FAILED", 1, 2)],
    "dynamic-declarations.log": [],
    "source-name-assumption-failed.log": [("FAILED", 10, 1)],
    "current-final.log": [("ok", 11, 0), ("ok", 1, 0), ("ok", 3, 0)],
    "msrv-final.log": [("ok", 11, 0), ("ok", 1, 0), ("ok", 3, 0)],
    "old-root-constructor-focused.log": [("ok", 7, 0), ("ok", 7, 0)],
    "gnu-owner-regressions.log": [("ok", 12, 0), ("ok", 6, 0), ("ok", 5, 0)],
    "windows-cli-lint.log": [("ok", 1, 0), ("ok", 1, 0)],
    "windows-check-lint-final.log": [],
    "root-contracts-final.log": [("ok", 159, 0)],
    "windows-concurrent-build-failed.log": [],
    "static-final.log": [],
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def unique(pairs):
    out = {}
    for key, value in pairs:
        assert key not in out, key
        out[key] = value
    return out


def load_manifest():
    return json.loads((PACKET / "manifest.json").read_bytes(), object_pairs_hook=unique)


def verify_evidence(m, logs):
    assert m["schema_version"] == "diagnostic-signer-owned-root-074-v1"
    assert (m["source_commit"], m["source_tree"], m["baseline"], m["preregistration"]) == (
        SOURCE, TREE, BASELINE, PREREG)
    assert m["scoped_final_checks_passed"] is True
    assert set(m["closed_claims"]) == CLOSED
    assert all(v is False for v in m["closed_claims"].values())
    assert m["frozen_inputs"] == FROZEN
    assert set(m["implementation_files"]) == IMPLEMENTATION and len(IMPLEMENTATION) == 8
    assert set(m["unchanged_sources"]) == ORACLES
    assert m["captures"] == EXPECTED_LOGS
    for info in m["captures"].values():
        assert type(info["bytes"]) is int and type(info["exit_code"]) is int
    assert set(logs) == set(EXPECTED_LOGS) == set(RESULTS)
    for name, data in logs.items():
        expected = EXPECTED_LOGS[name]
        assert (sha(data), len(data)) == (expected["sha256"], expected["bytes"]), name
        text = data.decode("utf-8")
        found = [(tag, int(passed), int(failed)) for tag, passed, failed in re.findall(
            r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", text)]
        assert found == RESULTS[name], (name, found)
    baseline = logs["baseline-old-root-failed.log"].decode()
    assert "signer_constructor_loss_seeded_fresh_and_independent_observers ... FAILED" in baseline
    assert "constructor death (2, 9) refused" in baseline
    current = logs["current-final.log"].decode()
    assert "normal_fixed_root_prepare=true real_runtime_sources=4 child_launch=false admission=false" in current
    assert "independent_normal_root_preparations=4 parent_context_unchanged=true admission=false" in current
    assert "extra_libm_profile=refused" not in current
    msrv = logs["msrv-final.log"].decode()
    assert msrv.count("extra_libm_profile=refused unchanged_four_role_graph=true admission=false") == 5
    assert "normal_fixed_root_prepare=true" not in msrv
    dynamic = logs["dynamic-declarations.log"].decode()
    assert dynamic.count("Shared library: [libm.so.6]") == 1
    static = logs["static-final.log"].decode()
    for marker in ["175 discovered gates", "doc-check: OK", "OK (local, non-promotable)",
                   "expected_ship_refusal_exit=1"]:
        assert marker in static, marker
    assert "Access is denied. (os error 5)" in logs["windows-concurrent-build-failed.log"].decode()


def git(*args):
    env = os.environ.copy()
    marker = ROOT / ".git"
    if os.name != "nt" and marker.is_file():
        path = marker.read_text().strip().removeprefix("gitdir: ")
        if re.match(r"^[A-Za-z]:/", path):
            env.update(GIT_DIR="/mnt/" + path[0].lower() + path[2:], GIT_WORK_TREE=str(ROOT))
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)


def verify():
    m = load_manifest()
    verify_evidence(m, {p.name: p.read_bytes() for p in PACKET.glob("*.log")})
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert set(git("diff", "--name-only", PREREG, SOURCE).decode().splitlines()) == IMPLEMENTATION
    for path, expected in m["implementation_files"].items():
        blob = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert sha(blob) == expected, path
        assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == blob, path
    for path, expected in m["unchanged_sources"].items():
        blob = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
        assert blob == git("show", BASELINE + ":" + path).replace(b"\r\n", b"\n"), path
        assert sha(blob) == expected, path
        assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == blob, path
    for path, expected in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == expected, path
    print("owned-root packet OK; scoped checks only; broad regression unresolved; admission=false")


if __name__ == "__main__":
    verify()
