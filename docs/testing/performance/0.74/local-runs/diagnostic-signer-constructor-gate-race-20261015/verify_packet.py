"""Unsigned local fixture evidence; no execution attestation or admission."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "07b88def29d47c67574499727271f751533080fe"
TREE = "da7e9255216ec976d957c3a6310d8e4afc7188cb"
BASELINE = "da47b2431a3281d2cfa6f76d1020bd3ed8c43c57"
PREREG = "8d4e865c5b2f18f61bd3af536c94e5ab10709556"
INSTRUMENTATION = "c6df158dd3e832ff2540ef846772cf47d77c07ce"
CLOSED = set(["exact_historical_syscall_trace_observed","production_guard_or_normal_launcher_changed","arbitrary_same_uid_mutators_supported","gate_credentials_authenticate_runtime_custody","real_credentials_or_signing","authenticated_original_start","all_writers_revoked","completion_or_later_cells_enabled","server_or_github_mutated","qualification_allowed","ship_allowed","performance_claim_allowed","full_release_gate_passed","all_linux_kernels_supported","clean_checkout_execution_attested"])
BOUNDS = {
    "gate_setup_revision": 2,
    "setup_gate_bytes": 1,
    "setup_gate_descriptors": 1,
    "report_bytes": 32,
    "report_descriptors": 5,
    "control_bytes": 128,
    "max_received_packets": 10,
    "logical_stall_millis": 5000,
    "outer_stall_millis": 30000,
    "capture_bytes": 8192,
    "before_arm_refusal_exit": 81,
    "no_arm_control_exit": 83,
    "no_parent_check_control_exit": 84
}
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml": "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
}
PRIOR = {
    "docs/testing/performance/0.74/local-runs/diagnostic-signer-owned-root-20261015/baseline-old-root-failed.log": "932dbd554732825d78d36dd34b2b360b47b866afa1e021a05cdba4418cc61a99"
}
IMPLEMENTATION = set(["crates/xtask/tests/performance_contract_074.rs","docs/COMPAT.md","docs/testing/performance/0.74/diagnostic-signer-constructor-loss-contract.toml","tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs"])
ORACLES = set(["tools/long-run-supervisor-074/src/lib.rs","tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs","tools/long-run-supervisor-074/src/diagnostic_signer_creator_task.rs","tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs","tools/long-run-supervisor-074/src/diagnostic_signer_tracer_loss_channel.rs","tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs","tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs","tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs","tools/long-run-supervisor-074/src/diagnostic_signer_owned_probe.rs","tools/long-run-supervisor-074/src/diagnostic_signer_owned_root.rs","tools/long-run-supervisor-074/src/diagnostic_signer_isolation.rs","tools/long-run-supervisor-074/src/diagnostic_signer_isolation_linux.rs"])
EXPECTED_LOGS = {
    "first-exec-regressions.log": {"sha256":"4098f27fb0fa9ea84ba6f1046486fb41a09bcb2732833710b3fde594b05cde69","exit_code":0,"bytes":2035},
    "baseline-current.log": {
        "sha256": "07c212aff10ff864773eb4cd2c5908167cd895e98e40f0cf14a441db394ac741",
        "bytes": 29429,
        "exit_code": 0
    },
    "broad-final.log": {
        "sha256": "aa6e4e649fca87cd740446749d9c4227a47e1e2ea1bffe5818d66008e2a4b6a4",
        "bytes": 96547,
        "exit_code": 0
    },
    "diagnostic-current.log": {
        "sha256": "8b29540e50814febf01cf5cf3a33d9035e7ec303e10bb9157ca8ab05a3eaef1c",
        "bytes": 29141,
        "exit_code": 0
    },
    "first-fixed-provisional.log": {
        "sha256": "3d7d77a13d0cd01f8294951d605b3137632a4523e971a6f3bcb2e44796aad43c",
        "bytes": 6210,
        "exit_code": 0
    },
    "initial-compile-failed.log": {
        "sha256": "09ca99bb33213b0f85cd9f7c71df9906bc68af642efe89612d8c98c50554aa90",
        "bytes": 973,
        "exit_code": 1
    },
    "instrumentation-check-lint.log": {
        "sha256": "6789ec0a1f17524c020b840624b25ccea5baf5ce9637e41b78a030ea2197cbff",
        "bytes": 409,
        "exit_code": 0
    },
    "linux-check-lint-final.log": {
        "sha256": "311726df6552032cf9ec784db3af874803915cef3e347e82c0d372e461ce15f7",
        "bytes": 410,
        "exit_code": 0
    },
    "msrv-broad-final.log": {
        "sha256": "6382328be7699699b32e207ded36f17aa1a17fe3301c3124501f2cb98274104e",
        "bytes": 32808,
        "exit_code": 0
    },
    "msrv-check-final.log": {
        "sha256": "2020d81e5eb34b02641f3daa80fa609c4386f1857a8922504ddc8466b4b88516",
        "bytes": 205,
        "exit_code": 0
    },
    "normal-regressions.log": {
        "sha256": "6ec1d7e30d7f8bfa36f32f76d5a52e01e16861822106a4d6429cac542b88b8af",
        "bytes": 9180,
        "exit_code": 0
    },
    "ordering-before.log": {
        "sha256": "6e51bd81c339f6f2de682b2a50e2d8866cb08f0fbab560ba0a3179a43f639349",
        "bytes": 2127,
        "exit_code": 1
    },
    "static-final.log": {
        "sha256": "06ccc58d450d44a2275a26d60e50d22ce6a211c1bc831d8a8421acf1f881cd36",
        "bytes": 1169,
        "exit_code": 0
    },
    "windows-check-lint-final.log": {
        "sha256": "49f7c3631fd785a58bfb69b286bc2868fcdedb511a9bd181f76aa2351aef92dd",
        "bytes": 792,
        "exit_code": 0
    },
    "windows-portable-final.log": {
        "sha256": "280fc8c29a21d4065910499da6a68081ca950d2c2310e5dec1997ecd6fa5a756",
        "bytes": 3049,
        "exit_code": 0
    },
    "windows-root-final.log": {
        "sha256": "fb16109d8a2897f9cc0b3fe188b7e5389df3201bf06b94e9d5ddf84693563092",
        "bytes": 654,
        "exit_code": 0
    }
}
RESULTS = {
    "first-exec-regressions.log": [["ok", 11, 0]],
    "baseline-current.log": [["ok", 79, 0]],
    "diagnostic-current.log": [["ok", 79, 0]],
    "ordering-before.log": [["ok", 1, 0], ["FAILED", 0, 1]],
    "initial-compile-failed.log": [],
    "first-fixed-provisional.log": [["ok", 11, 0]],
    "broad-final.log": [["ok", 83, 0]] * 3,
    "msrv-broad-final.log": [["ok", 83, 0]],
    "normal-regressions.log": [["ok", n, 0] for n in [12, 11, 5, 6, 7, 1, 5, 1, 3, 3]],
    "linux-check-lint-final.log": [],
    "msrv-check-final.log": [],
    "windows-root-final.log": [["ok", 159, 0]],
    "windows-portable-final.log": [["ok", n, 0] for n in [7, 1, 1, 7, 1, 1]],
    "windows-check-lint-final.log": [],
    "static-final.log": [],
    "instrumentation-check-lint.log": [],
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
    assert m["schema_version"] == "diagnostic-signer-constructor-gate-race-074-v1"
    assert (m["source_commit"], m["source_tree"], m["baseline"], m["preregistration"],
            m["instrumentation_commit"]) == (SOURCE, TREE, BASELINE, PREREG, INSTRUMENTATION)
    assert m["scoped_final_checks_passed"] is True
    assert m["historical_cut_inferred_from_closed_oracle"] is True
    assert set(m["closed_claims"]) == CLOSED and all(v is False for v in m["closed_claims"].values())
    assert m["bounds"] == BOUNDS and all(type(v) is int for v in m["bounds"].values())
    assert m["frozen_inputs"] == FROZEN and m["prior_negative_evidence"] == PRIOR
    assert set(m["implementation_files"]) == IMPLEMENTATION and len(IMPLEMENTATION) == 4
    assert set(m["unchanged_sources"]) == ORACLES
    assert m["captures"] == EXPECTED_LOGS
    assert set(logs) == set(EXPECTED_LOGS) == set(RESULTS)
    for name, info in m["captures"].items():
        assert type(info["bytes"]) is int and type(info["exit_code"]) is int, name
        raw = logs[name]
        assert (sha(raw), len(raw)) == (info["sha256"], info["bytes"]), name
        results = [[tag, int(p), int(f)] for tag, p, f in re.findall(
            r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", raw.decode())]
        assert results == RESULTS[name], (name, results)
    before = logs["ordering-before.log"].decode()
    assert "constructor_control EOF_before_creator_death=true then_guard_SIGKILL=true prearm_exit81_proof=false" in before
    assert "left: 0" in before and "right: 1" in before
    for name, groups in [("broad-final.log", 3), ("msrv-broad-final.log", 1)]:
        raw = logs[name].decode()
        for marker, per_group in [
            ("constructor_gate cut=0 fault=0 creator_terminal=true child_nonterminal=true peer_retained=true", 9),
            ("constructor_gate cut=0 fault=2 creator_terminal=true child_nonterminal=true peer_retained=true", 1),
            ("constructor_gate cut=1 fault=1 creator_terminal=true child_nonterminal=true peer_retained=true", 1),
            ("constructor_eof_control creator_live=true guard_readback_path_reached=true first_stop=true exitkill_installed=false", 1),
            ("constructor_gate independent_prearm_observers=4 all_joined=true parent_subreaper_unchanged=true", 1),
            ("constructor_gate fault_cases=11 original_peer_required=true unknown_poll_events_refuse=true", 1),
        ]:
            assert raw.count(marker) == groups * per_group, (name, marker)
        assert "... FAILED" not in raw and "constructor monitor refused" not in raw
    static = logs["static-final.log"].decode()
    for marker in ["175 discovered gates", "doc-check: OK", "OK (local, non-promotable)",
                   "expected_ship_refusal_exit=1"]:
        assert marker in static, marker


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
    for field in ["implementation_files", "unchanged_sources"]:
        for path, expected in m[field].items():
            blob = git("show", SOURCE + ":" + path).replace(b"\r\n", b"\n")
            assert sha(blob) == expected, path
            assert (ROOT / path).read_bytes().replace(b"\r\n", b"\n") == blob, path
            if field == "unchanged_sources":
                assert blob == git("show", BASELINE + ":" + path).replace(b"\r\n", b"\n"), path
    for path, expected in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == sha(git("show", SOURCE + ":" + path)) == expected
    for path, expected in PRIOR.items():
        raw = (ROOT / path).read_bytes()
        assert sha(raw) == sha(git("show", BASELINE + ":" + path)) == expected
        assert b"constructor death (2, 9) refused" in raw
        assert b"78 passed; 1 failed;" in raw
    contract_path = "docs/testing/performance/0.74/diagnostic-signer-constructor-loss-contract.toml"
    contract = tomllib.loads(git("show", SOURCE + ":" + contract_path).decode())
    assert all(contract[key] == value for key, value in BOUNDS.items())
    old = git("show", BASELINE + ":tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs").decode()
    assert "constructor_loss_case(seed as u8 % 5, 0, false)" in old
    assert "Fault::None if self.cut == Cut::BeforeArm => (libc::CLD_EXITED, 81)" in old
    assert "Fault::None => (libc::CLD_KILLED, libc::SIGKILL)" in old
    print("constructor gate packet OK; scoped ordering corrected; historical trace not observed; admission=false")


if __name__ == "__main__":
    verify()
