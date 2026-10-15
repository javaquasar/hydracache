"""Unsigned local consistency evidence, never execution or authority attestation."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
EXPECTED = json.loads(r'''{
  "schema_version": "diagnostic-signer-owned-root-exec-074-v1",
  "source_commit": "5ca2cf2ca28f6f39a010a205e9ab649e91ebe977",
  "source_tree": "9f56af0eab06092c8d84976fe1af286c8e2a8f58",
  "baseline": "73b5ccd515d8f47d0295a0f6b87628fed937c0e2",
  "preregistration": "caeba7d839bd80dd039a4fb5476f8523c4d8ee00",
  "isolated_scoped_checks_passed": true,
  "overlapping_broad_outer_guard_failed": true,
  "closed_claims": {
    "normal_restriction_installer_composed": false,
    "full_normal_entry_parent_loss_proved": false,
    "hostile_same_uid_custody_authenticated": false,
    "loaded_memory_bytes_attested": false,
    "original_start_authenticated": false,
    "all_writers_revoked": false,
    "real_credentials_or_signing": false,
    "completion_or_later_cells_enabled": false,
    "server_or_github_mutated": false,
    "qualification_allowed": false,
    "ship_allowed": false,
    "performance_claim_allowed": false,
    "full_release_gate_passed": false,
    "clean_checkout_execution_attested": false,
    "overlapping_broad_load_robustness_proved": false
  },
  "bounds": {
    "maximum_image_bytes": 8388608,
    "maximum_proc_text_bytes": 65536,
    "maximum_map_lines": 128,
    "wait_seconds": 5,
    "initial_images": 2,
    "loaded_images": 4,
    "private_marker_version": 1,
    "private_marker_bytes": 16
  },
  "frozen_inputs": {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml": "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
  },
  "implementation_files": {
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root.rs": "d1db87290959534ae2827c985f7196f5acf844f86e9113c0b48d206ee8fa4ac3",
    "docs/testing/performance/0.74/diagnostic-signer-owned-root-exec-contract.toml": "6bc6e02c90171434d9bb69cca64ef3bbfade9ae59fced05a1d00004526637a57",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_tests.rs": "6a30cec47443f912715a0f5dcd0477552761790ad3c2b50e35efeae7a45b8547",
    "crates/xtask/tests/performance_contract_074.rs": "dcac9dab605359ce4a80c5264b4ea5fa5d1dd44ee2a85cfb15d1f958ea1e09bb",
    "tools/long-run-supervisor-074/tests/diagnostic_signer_root_probe.rs": "cbb8ccb2a890b880dd23400fee44da78d89f8ba412928a2df16ddb514e48683c",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_maps.rs": "f84ada2c1f6c5a7e1fdde6ef4654cf41a03d2fd99889d9a55ffc90e398710732",
    "docs/COMPAT.md": "4fd7e20b215da67124faf84a62011aef75a267ecaa86423fee1d119d2d3e86e0",
    "tools/long-run-supervisor-074/tests/diagnostic_signer_root_cli.rs": "b4eb36425341daca936ffab5c6fff4fd8f64ec9bd59d12a2af6c58633f71d65b",
    "docs/testing/performance/0.74/diagnostic-signer-owned-root-exec-design.md": "aeb402c73eaf3d53dbf19a29eda10938b22412f11eba4b464984092925e463db",
    "tools/long-run-supervisor-074/src/bin/diagnostic_signer_root_probe.rs": "74caa3183275bef0829f4064c015c2a86527fa2d8ea75501b62936ecb325d07f",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_exec_tests.rs": "4b5101b2bc61427a4839b1b6ccceba506d372680c4ffc6505601e61154366d66",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_exec.rs": "5472a57b710901d5d2c788d5508ba2bfce3a1caab3a2ee35e17e680469133c83"
  },
  "captures": {
    "normal-regressions.log": {
      "bytes": 6087,
      "sha256": "0cf267f661974d3a39bfb067dee07de7a7834980c7f9dad926c52cb65829a288",
      "exit_code": 0,
      "results": [
        [
          "ok",
          12,
          0
        ],
        [
          "ok",
          5,
          0
        ],
        [
          "ok",
          6,
          0
        ],
        [
          "ok",
          7,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          5,
          0
        ]
      ]
    },
    "windows-portable-msrv.log": {
      "bytes": 1384,
      "sha256": "6a6f0bdcb82b14a6599b77060825a6867e122a0f104cf88118545611c2df5d82",
      "exit_code": 0,
      "results": [
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          1,
          0
        ]
      ]
    },
    "windows-lint.log": {
      "bytes": 283,
      "sha256": "0bcf9a7fa96205e004d09c4e1ffcbfa5183fad088487f5aa18002e0465ca1625",
      "exit_code": 0,
      "results": []
    },
    "msrv-check.log": {
      "bytes": 204,
      "sha256": "9451f6cf069d12a000a7f2820047ee39c4b9310e403b9cabe380735a8ba4c021",
      "exit_code": 0,
      "results": []
    },
    "linux-lint.log": {
      "bytes": 292,
      "sha256": "6d3ffbbfd0480d780dd4ed7141c955d6797b1725ecaf8ed2e91dfd163bc594d7",
      "exit_code": 0,
      "results": []
    },
    "committed-current.log": {
      "bytes": 3616,
      "sha256": "090cf4f2683ea3dc782711038cedfb35ea62b30b6fb84b4e4cb6c350c9a4ae4b",
      "exit_code": 0,
      "results": [
        [
          "ok",
          18,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          6,
          0
        ]
      ]
    },
    "development-marker-size-failed.log": {
      "bytes": 575,
      "sha256": "8ea7abf10dfe3de63f6f40d65c8d1c2d54d866bb74f607f4a22deea4040ac360",
      "exit_code": 1,
      "results": []
    },
    "development-synthetic-exec-failed.log": {
      "bytes": 2656,
      "sha256": "40ca947b09d3d84dacf296f4cb807ce81574588dd15f5a88439405fab8e55766",
      "exit_code": 1,
      "results": [
        [
          "FAILED",
          15,
          1
        ]
      ]
    },
    "development-fmt-failed.log": {
      "bytes": 413,
      "sha256": "ec5fab0ce179593e20100acd45c66a36167e15c0ec6d2f63c495423e02a24e9d",
      "exit_code": 1,
      "results": []
    },
    "old-root-regressions.log": {
      "bytes": 32000,
      "sha256": "ad2c6d48304a9113ef4f365b49f2d56e6b152b077f0bace20be622069d503943",
      "exit_code": 0,
      "results": [
        [
          "ok",
          83,
          0
        ]
      ]
    },
    "baseline-prepare.log": {
      "bytes": 2346,
      "sha256": "69d4d01bf6fab3f55f098f24ee11cf64ac506d09feb1d396edfd16bed7b2da31",
      "exit_code": 0,
      "results": [
        [
          "ok",
          11,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          3,
          0
        ]
      ]
    },
    "root-contracts.log": {
      "bytes": 555,
      "sha256": "407ea08f8352e192129bd5b5a9f8b3e9d1c9f79100bbd061aabe891357da2fb8",
      "exit_code": 0,
      "results": [
        [
          "ok",
          160,
          0
        ]
      ]
    },
    "isolated-current-2.log": {
      "bytes": 3673,
      "sha256": "0a404a5a5ce6b08b22ec526852bf9e575c6c5ce223b4ddff043ecce0dba124a7",
      "exit_code": 0,
      "results": [
        [
          "ok",
          18,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          6,
          0
        ]
      ]
    },
    "static-contracts.log": {
      "bytes": 1394,
      "sha256": "8b6c521a39e2b747e396187e8f93deb6d75b2c3333762caffcf68db0821e9853",
      "exit_code": 0,
      "results": []
    },
    "development-preflight-failed.log": {
      "bytes": 1173,
      "sha256": "e04f6c42394cb85f33095e9fc2b012972d5e28a569eaa4624a7a641a30c12330",
      "exit_code": 1,
      "results": [
        [
          "FAILED",
          0,
          1
        ]
      ]
    },
    "development-lint-unit-failed.log": {
      "bytes": 9252,
      "sha256": "86d44a12cfab5a1e0c0b36c74fcd7c73404f6b6cbc3b7a4a82052c8455335403",
      "exit_code": 1,
      "results": []
    },
    "isolated-current-1.log": {
      "bytes": 3616,
      "sha256": "ecc439e8a4843789e461325d9fa9234abbbf6c8c4d4485d4c7bd2afd665e1319",
      "exit_code": 0,
      "results": [
        [
          "ok",
          18,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          6,
          0
        ]
      ]
    },
    "overlapping-broad-timeouts.log": {
      "bytes": 4288,
      "sha256": "a0bec2f2a448ffb7765839f58c9d854a98e250b77471f9de364486ec3a942037",
      "exit_code": 1,
      "results": [
        [
          "ok",
          18,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "FAILED",
          4,
          2
        ]
      ]
    },
    "development-lint-duplicate-failed.log": {
      "bytes": 1355,
      "sha256": "339c5bbda7095c0c49193f8406ed528412020a5aec1426d332e0f3966de8e192",
      "exit_code": 1,
      "results": []
    },
    "first-exec-and-entry-regressions.log": {
      "bytes": 2722,
      "sha256": "3a62965ca28d25f2ccabaecf21fe01fb2b4482d1dc19b463de42a630462e1c53",
      "exit_code": 0,
      "results": [
        [
          "ok",
          11,
          0
        ],
        [
          "ok",
          3,
          0
        ]
      ]
    },
    "windows-check.log": {
      "bytes": 283,
      "sha256": "5f39de6ccdfa4785032f2791609276544d9dcbbfaaa95f992235d0463e8eea03",
      "exit_code": 0,
      "results": []
    },
    "msrv-current.log": {
      "bytes": 3964,
      "sha256": "4fa098b1035be8b25a4523c698aa2e67c35ca4ae0b332bf2ccb878c094d4939b",
      "exit_code": 0,
      "results": [
        [
          "ok",
          18,
          0
        ],
        [
          "ok",
          1,
          0
        ],
        [
          "ok",
          6,
          0
        ]
      ]
    },
    "linux-check.log": {
      "bytes": 1609,
      "sha256": "68b2a390dc07edeff30e51268c393f9f65631952d94d4d0b171560b66508b01a",
      "exit_code": 0,
      "results": []
    },
    "new-mode-red.log": {
      "bytes": 1173,
      "sha256": "1ce49d250f4a2de13863b60720f6c038ee5a74e7827d51932ed9f8a8ad6c41ba",
      "exit_code": 1,
      "results": [
        [
          "FAILED",
          0,
          1
        ]
      ]
    }
  },
  "unchanged_sources": {
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs": "29d8abde786bd644f1de38e7af8921745dc6af450d72e8a8a9128a2cabb9edad",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs": "347adc3ed03207570bdc2ee1c450bbf25817efa2ffe2d82e8ea1c171626857e5",
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs": "c83169be6d5fff97adbb87fa5c65c1436b70ab5f9c077bd1118b73415d057371",
    "tools/long-run-supervisor-074/src/diagnostic_signer_creator_task.rs": "0106d7be6c478891ba67590db46478c6be034aa33d752339530ee07ff7de0781",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation_linux.rs": "adf38288b9e77e1b509b6cec5326a897e80f24241ba53fa49671b5080cd8cc97",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation.rs": "bd952ee03b85617d86dcbc824dd79ce6b1b6503c7413c81b618eb589e04e480e",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_maps.rs": "66680bb06d3149746778b738c8fd7c0be8e5495c9ac5d0d2303fdb60dbdae831",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_probe.rs": "515bf7766ee15e2bd629a36dd5f1f3d3e613d9b137fb519c02b6aa7e6ef4d4cf",
    "tools/long-run-supervisor-074/src/diagnostic_signer_gnu.rs": "e0c331986047ab4c046dce0c9795e600b7ed4107fbd89729c38675b45e33a3a5",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs": "13d69f18d0e00422b3e461835dd1e8019a4c310fa9a5acd31c37691e9221210a",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs": "1a4c41df0b617255d078a070046822ebe4dc008a7e0699553fd863a6237dc1c4",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs": "259f9aa219e58cb980a25f13c23e4003e277eb746ce276774aaf317a013a446d",
    "tools/long-run-supervisor-074/src/lib.rs": "36c054f56ae80437bea158a17da6bc02b2a11873f57b41fb3bc6f0dc90b909b1"
  }
}''')

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

def git(*args):
    env = os.environ.copy()
    marker = ROOT / ".git"
    if os.name != "nt" and marker.is_file():
        path = marker.read_text().strip().removeprefix("gitdir: ")
        if re.match(r"^[A-Za-z]:/", path):
            env.update(GIT_DIR="/mnt/" + path[0].lower() + path[2:], GIT_WORK_TREE=str(ROOT))
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)

def verify_evidence(manifest, logs):
    assert json.dumps(manifest, sort_keys=True) == json.dumps(EXPECTED, sort_keys=True), "exact declared source, bounds, claims and capture pins required"
    assert set(logs) == set(EXPECTED["captures"])
    for name, capture in EXPECTED["captures"].items():
        raw = logs[name]
        assert len(raw) == capture["bytes"] and sha(raw) == capture["sha256"], name
        text = raw.decode()
        assert "Warning: truncated output" not in text
        results = [[tag, int(p), int(f)] for tag, p, f in re.findall(
            r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;", text)]
        assert results == capture["results"], (name, results)
    for name in ["isolated-current-1.log", "isolated-current-2.log", "committed-current.log"]:
        raw = logs[name].decode()
        assert EXPECTED["captures"][name]["results"] == [["ok",18,0],["ok",1,0],["ok",6,0]]
        assert "normal_root_exec=true initial_images=2 loaded_images=4 original_reaped=true stream_eof=true admission=false" in raw
        assert "independent_root_exec_owners=4 extra_libm_refusal=false parent_context_unchanged=true admission=false" in raw
        assert "ordinary_root_maps seed=0x0740c07b mutations=4096 admission=false" in raw
        assert "Original root child quiescence refused; root retained." in raw
    negative = logs["overlapping-broad-timeouts.log"].decode()
    assert negative.count("normal root preparation stalled") == 2
    assert EXPECTED["captures"]["overlapping-broad-timeouts.log"]["results"] == [["ok",18,0],["ok",1,0],["FAILED",4,2]]
    msrv = logs["msrv-current.log"].decode()
    assert "extra_libm_root_exec=refused admission=false" in msrv
    assert "independent_root_exec_owners=4 extra_libm_refusal=true" in msrv
    assert "normal_root_exec=true" not in msrv
    static = logs["static-contracts.log"].decode()
    for marker in ["175 discovered gates", "doc-check: OK", "OK (local, non-promotable)", "require_ship_exit=1 expected_closed=true"]:
        assert marker in static, marker

def verify():
    manifest = load_manifest()
    verify_evidence(manifest, {p.name: p.read_bytes() for p in PACKET.glob("*.log")})
    source = manifest["source_commit"]
    assert git("rev-parse", source + "^{tree}").decode().strip() == manifest["source_tree"]
    assert set(git("diff", "--name-only", manifest["preregistration"], source).decode().splitlines()) == set(manifest["implementation_files"])
    for group in ["implementation_files", "unchanged_sources"]:
        for path, expected in manifest[group].items():
            blob = git("show", source + ":" + path).replace(b"\r\n", b"\n")
            assert sha(blob) == expected, path
            if group == "unchanged_sources":
                assert blob == git("show", manifest["baseline"] + ":" + path).replace(b"\r\n", b"\n"), path
    for path, expected in manifest["frozen_inputs"].items():
        assert sha(git("show", source + ":" + path)) == sha(git("show", manifest["baseline"] + ":" + path)) == expected
        assert sha((ROOT / path).read_bytes()) == expected
    contract = tomllib.loads(git("show", source + ":docs/testing/performance/0.74/diagnostic-signer-owned-root-exec-contract.toml").decode())
    assert all(contract[key] == value for key, value in manifest["bounds"].items())
    for key in set(manifest["closed_claims"]) & set(contract):
        assert contract[key] is False, key
    print("owned root exec packet OK; isolated checks pass; overlapping load timeout retained; admission=false")

if __name__ == "__main__":
    verify()
