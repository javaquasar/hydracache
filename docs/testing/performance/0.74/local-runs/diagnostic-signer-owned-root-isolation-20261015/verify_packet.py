"""Unsigned historical consistency packet; no execution receipt or authority."""
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
  "schema_version": "diagnostic-signer-owned-root-isolation-074-v1",
  "source_commit": "3c774c88ba99a2ea82d2d0840bdebdab97a0173f",
  "source_tree": "6af3d57900d3d0e5e05bfeb98c4fef46c42cc802",
  "baseline": "37b9b2da5df2d096921761a5e0aab80c7c5779a7",
  "preregistration": "bf313f579afbff2e5aa38054a77f6549c28c6595",
  "scoped_checks_passed": true,
  "fixed_policy_source_unchanged": true,
  "old_entry_proc_checks_preserved": true,
  "closed_claims": {
    "direct_post_install_dumpability_query": false,
    "full_normal_entry_creator_loss_proved": false,
    "overlapping_broad_load_robustness_proved": false,
    "credential_handoff_authenticated": false,
    "runtime_custody_authenticated": false,
    "mapped_memory_attested": false,
    "original_worker_start_authenticated": false,
    "all_writers_revoked": false,
    "real_credentials_or_signing": false,
    "completion_or_later_cells_enabled": false,
    "server_or_github_mutated": false,
    "qualification_allowed": false,
    "ship_allowed": false,
    "performance_claim_allowed": false,
    "full_release_gate_passed": false,
    "clean_checkout_execution_attested": false
  },
  "bounds": {
    "policy_instructions": 64,
    "policy_bytes": 512,
    "header_bytes": 16,
    "maximum_syscall_events": 128,
    "install_syscall_pairs": 8,
    "maximum_proc_text_bytes": 65536,
    "maximum_map_lines": 128,
    "directory_read_bytes": 512,
    "directory_reads": 3,
    "directory_records": 8,
    "maximum_image_bytes": 8388608,
    "wait_seconds": 5,
    "private_marker_version": 1,
    "private_marker_bytes": 16,
    "maximum_failure_drain_bytes": 48,
    "memory_mutation_seed": 121684092,
    "memory_mutations": 256
  },
  "frozen_inputs": {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml": "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc"
  },
  "unchanged_sources": {
    "tools/long-run-supervisor-074/src/diagnostic_signer_handoff_helper.rs": "c83169be6d5fff97adbb87fa5c65c1436b70ab5f9c077bd1118b73415d057371",
    "tools/long-run-supervisor-074/src/diagnostic_signer_gnu.rs": "e0c331986047ab4c046dce0c9795e600b7ed4107fbd89729c38675b45e33a3a5",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_maps.rs": "66680bb06d3149746778b738c8fd7c0be8e5495c9ac5d0d2303fdb60dbdae831",
    "tools/long-run-supervisor-074/src/diagnostic_signer_namespace_helper.rs": "13d69f18d0e00422b3e461835dd1e8019a4c310fa9a5acd31c37691e9221210a",
    "tools/long-run-supervisor-074/src/diagnostic_signer_rooted_exec_fixture.rs": "1a4c41df0b617255d078a070046822ebe4dc008a7e0699553fd863a6237dc1c4",
    "tools/long-run-supervisor-074/src/lib.rs": "36c054f56ae80437bea158a17da6bc02b2a11873f57b41fb3bc6f0dc90b909b1",
    "tools/long-run-supervisor-074/src/diagnostic_signer_seccomp_fixture.rs": "6c7d04b1bb8c87d67ad97d18090c21b52b84557e55eeadf8eba4935198c0130d",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_probe.rs": "515bf7766ee15e2bd629a36dd5f1f3d3e613d9b137fb519c02b6aa7e6ef4d4cf",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_guard.rs": "29d8abde786bd644f1de38e7af8921745dc6af450d72e8a8a9128a2cabb9edad",
    "tools/long-run-supervisor-074/src/diagnostic_signer_root_fixture.rs": "347adc3ed03207570bdc2ee1c450bbf25817efa2ffe2d82e8ea1c171626857e5",
    "tools/long-run-supervisor-074/src/diagnostic_signer_creator_task.rs": "0106d7be6c478891ba67590db46478c6be034aa33d752339530ee07ff7de0781",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation.rs": "bd952ee03b85617d86dcbc824dd79ce6b1b6503c7413c81b618eb589e04e480e",
    "tools/long-run-supervisor-074/src/diagnostic_signer_constructor_loss_fixture.rs": "259f9aa219e58cb980a25f13c23e4003e277eb746ce276774aaf317a013a446d"
  },
  "prior_negative": {
    "sha256": "a0bec2f2a448ffb7765839f58c9d854a98e250b77471f9de364486ec3a942037",
    "path": "docs/testing/performance/0.74/local-runs/diagnostic-signer-owned-root-exec-20261015/overlapping-broad-timeouts.log"
  },
  "captures": {
    "root-contracts.log": {
      "sha256": "abb871fc66ee0ed270d4b81212e923bd805671e0f8fb800401975b3decac12d0",
      "bytes": 556,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          161,
          0
        ]
      ]
    },
    "msrv-check.log": {
      "sha256": "7dc957f39b5b772acd04fab1c897f42de43ad66ed81949c34779dfb28c3bdbbb",
      "bytes": 204,
      "exit_code": 0,
      "capture_complete": true,
      "results": []
    },
    "old-regressions.log": {
      "sha256": "129cf2bf3eafecefd0ef1f6e91e4e04039ebf5c5f576f39a4955c2ecd04a6273",
      "bytes": 39241,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          83,
          0
        ],
        [
          "ok",
          11,
          0
        ],
        [
          "ok",
          3,
          0
        ],
        [
          "ok",
          12,
          0
        ],
        [
          "ok",
          6,
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
    "development-instrumented-success-refused.log": {
      "sha256": "df10e8e870f8cc633dd1b96dcca00877e6cb89311bc8e4bde212922ddccf7f63",
      "bytes": 2592,
      "exit_code": 1,
      "capture_complete": true,
      "results": [
        [
          "FAILED",
          0,
          1
        ]
      ]
    },
    "windows-check-lint.log": {
      "sha256": "8c74dded178585df9f8dc1b2f285b8fefc44ae0ebd1af4dc313c8e8add06d105",
      "bytes": 566,
      "exit_code": 0,
      "capture_complete": true,
      "results": []
    },
    "development-first-refusal.log": {
      "sha256": "0eb644cfc73570ecea83d993c4feb579cc3636985c31a4c92fa636875ba228b0",
      "bytes": 1761,
      "exit_code": 1,
      "capture_complete": true,
      "results": [
        [
          "FAILED",
          0,
          1
        ]
      ]
    },
    "committed-current-1.log": {
      "sha256": "a75585ec940a369d71b67b891991d77f4b27a9a5f925df8bbdf42011ed8c57f3",
      "bytes": 6730,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          24,
          0
        ],
        [
          "ok",
          5,
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
          6,
          0
        ]
      ]
    },
    "new-mode-red.log": {
      "sha256": "fe08de09117059c6e3813588ac8ae38c7aab7e693a884d80be605696821ff11b",
      "bytes": 1294,
      "exit_code": 1,
      "capture_complete": true,
      "results": [
        [
          "FAILED",
          0,
          1
        ]
      ]
    },
    "targeted-final.log": {
      "sha256": "5e4fbfd3138fa9d866114a6c97176b32cafbea79738f72558cb059b43a5ffd4f",
      "bytes": 6862,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          24,
          0
        ],
        [
          "ok",
          5,
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
          6,
          0
        ]
      ]
    },
    "msrv-final.log": {
      "sha256": "faabd52cbdbe73162fcbf6cdc1f79cae0596c6e40fa9b2158b4d19aa90162c79",
      "bytes": 6531,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          24,
          0
        ],
        [
          "ok",
          5,
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
          6,
          0
        ]
      ]
    },
    "development-clippy-ice-partial.log": {
      "sha256": "d6dd68b8d01c32bd3e85bf313d0fa8328ecc30ecb28290dcc324889bd8a78856",
      "bytes": 14237,
      "exit_code": 1,
      "capture_complete": false,
      "results": []
    },
    "windows-portable.log": {
      "sha256": "ea2fe8d0f7f2715ae7d8bb9c70c67e534aef88c2efd5b14c2291d6c64be09f28",
      "bytes": 2620,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
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
          7,
          0
        ],
        [
          "ok",
          1,
          0
        ]
      ]
    },
    "development-forbidden-readback.log": {
      "sha256": "2c35771362b547bdcba9bcf6e95a2667ea7794e36c6caaa6263531aba3ccefe6",
      "bytes": 2360,
      "exit_code": 1,
      "capture_complete": true,
      "results": [
        [
          "FAILED",
          0,
          1
        ]
      ]
    },
    "static-contracts.log": {
      "sha256": "58ccc69baf70a6dc1f6d72fc8b05dc2f9d83aa4141f207a5cdeff128c59cbc57",
      "bytes": 1181,
      "exit_code": 0,
      "capture_complete": true,
      "results": []
    },
    "baseline.log": {
      "sha256": "dd7b913e976391a2638464a067fc77cb34bb2e414721b014356cba190043e081",
      "bytes": 5277,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          18,
          0
        ],
        [
          "ok",
          5,
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
          6,
          0
        ]
      ]
    },
    "development-three-modes.log": {
      "sha256": "0f3448bef6eb5bbd1e14f5c8497cf248bc778412dbe1f39211902c45f8a9e846",
      "bytes": 1042,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          1,
          0
        ]
      ]
    },
    "linux-check-lint.log": {
      "sha256": "8177e7500629899290fe00953d5fc85aa1dfd7437072edcff16740f6704ee4e4",
      "bytes": 144,
      "exit_code": 0,
      "capture_complete": true,
      "results": []
    },
    "development-clippy-short.log": {
      "sha256": "878cb8623c186724493369e51b28264bd8bf3bf8d943d5fc6a955851fe700335",
      "bytes": 795,
      "exit_code": 1,
      "capture_complete": true,
      "results": []
    },
    "committed-current-2.log": {
      "sha256": "819686f67c12dd5aad4815129cad7eb43b3344d8ae734f4f9c11dcdd278c88ac",
      "bytes": 6730,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          24,
          0
        ],
        [
          "ok",
          5,
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
          6,
          0
        ]
      ]
    },
    "provisional-targeted.log": {
      "sha256": "d9a747875b6e5e361ecdcd2ec571f13e0e44a34649bd622fb29a36e1553d3963",
      "bytes": 6862,
      "exit_code": 0,
      "capture_complete": true,
      "results": [
        [
          "ok",
          24,
          0
        ],
        [
          "ok",
          5,
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
          6,
          0
        ]
      ]
    }
  },
  "implementation_files": {
    "tools/long-run-supervisor-074/tests/diagnostic_signer_root_cli.rs": "57967e01692f520441074c22a8dbd0648a45a5f124651b9d6dd0310ce2608cd7",
    "tools/long-run-supervisor-074/src/bin/diagnostic_signer_root_probe.rs": "f9a1f0235968b42fff0e97619262594b1c85e0609adc26f3d5a3b2df067532ae",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_exec.rs": "7ee7c8d53c902cb289762653993d7b0d143333ded69708104daa4ec5bba0ccf5",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_maps.rs": "d6c2194679db6f4fd61503da55d0a4ad611bcc2475d20f623e605667aa0aa27f",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_isolation.rs": "3cf4f1c100897f8e52f44d2c9c80f20ac3421502d5b19a3a9133bf1dd6aa6534",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation_core.rs": "282492bb23e4d2e7ac5e14ff0a1ddf18f55721d4a4bc4e9e9604702412072d2c",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root.rs": "2652cc842654701c3a08a15472936c14127b5a39a34283a54171bba534579ea9",
    "docs/testing/performance/0.74/diagnostic-signer-owned-root-isolation-preregistration.md": "34165d712c6a5699dd47f276471d982606d8dd4da7a44c24512ab021830d34c2",
    "tools/long-run-supervisor-074/tests/diagnostic_signer_root_probe.rs": "51870eac2f5b181a1bf17b3427064f82bc3da3cb67e4ab925a5beea1c32c8fa8",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_exec_tests.rs": "5561657515b8056e29b67c2d33b18c9f3c4ad857cf63cf96822c95726de89ef6",
    "tools/long-run-supervisor-074/src/diagnostic_signer_owned_root_isolation_tests.rs": "a18a991134a00d43c848ab4f21d6500ad3d168b8059c7d70e702570926e4d708",
    "tools/long-run-supervisor-074/src/diagnostic_signer_isolation_linux.rs": "bdf801d18589ae4ec0fb4fb24db1cf9680ef7595954d74a660bd16b855967db9",
    "crates/xtask/tests/performance_contract_074.rs": "73b9218262b387f3b2612361e8aeee9d3fc9619710e7706117b5c3112522c085",
    "docs/testing/performance/0.74/diagnostic-signer-owned-root-isolation-design.md": "a5495e5f9ae16f64c206e9574c9317365f20f4742abc3fda1bd5927601a973bb",
    "docs/COMPAT.md": "61af4467087487fa470e77bb4d35b1ac84864ce916abbd2e9ef49db94cdac373",
    "docs/testing/performance/0.74/diagnostic-signer-owned-root-isolation-contract.toml": "ccd6ae9a7dae4c65a2c5102832c18b29548d6edc387e066fda87955efe157878"
  }
}''')

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

def unique(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, key
        result[key] = value
    return result

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
    assert json.dumps(manifest,sort_keys=True) == json.dumps(EXPECTED,sort_keys=True)
    assert set(logs) == set(EXPECTED["captures"])
    for name, capture in EXPECTED["captures"].items():
        raw = logs[name]
        assert (sha(raw),len(raw)) == (capture["sha256"],capture["bytes"]), name
        text = raw.decode()
        if capture["capture_complete"]:
            assert "Warning: truncated output" not in text, name
        results = [[tag,int(p),int(f)] for tag,p,f in re.findall(
            r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed;",text)]
        assert results == capture["results"], (name,results)
    for name in ["targeted-final.log","committed-current-1.log","committed-current-2.log"]:
        raw = logs[name].decode()
        assert EXPECTED["captures"][name]["results"] == [["ok",24,0],["ok",5,0],["ok",7,0],["ok",1,0],["ok",6,0]]
        for mode in ["root-native","root-x32","root-i386"]:
            assert raw.count("normal_root_policy mode="+mode+" actual_tsync=true exact_bpf=true nd0=true original_reaped=true stream_eof=true admission=false") == 1
        for mode in ["root-kill-installed","root-drop-installed"]:
            assert raw.count("normal_root_policy_cleanup mode="+mode+" installed_stop=true original_reaped=true stream_eof=true admission=false") == 1
        assert "root_policy_memory seed=0x0740c07c mutations=256 original_child=true ptrace_words=true authority=false" in raw
        assert "Original root child quiescence refused; root retained." in raw
    negative = logs["development-forbidden-readback.log"].decode()
    assert "step=5 event=Entry { nr: 157, args: [3, 0, 0, 0, 0," in negative
    assert "step=5 event=Exit { value: -1, error: true }" in negative
    assert EXPECTED["captures"]["development-forbidden-readback.log"]["exit_code"] == 1
    partial = logs["development-clippy-ice-partial.log"].decode()
    assert "compiler unexpectedly panicked" in partial
    assert EXPECTED["captures"]["development-clippy-ice-partial.log"]["capture_complete"] is False
    msrv = logs["msrv-final.log"].decode()
    assert "extra_libm_root_policy_modes=5 refused=true unchanged_four_role_graph=true admission=false" in msrv
    assert "normal_root_policy mode=" not in msrv
    static = logs["static-contracts.log"].decode()
    for marker in ["175 discovered gates","doc-check: OK","OK (local, non-promotable)","require_ship_exit=1 expected_closed=true"]:
        assert marker in static, marker

def verify():
    manifest = load_manifest()
    verify_evidence(manifest,{p.name:p.read_bytes() for p in PACKET.glob("*.log")})
    source = manifest["source_commit"]
    assert git("rev-parse",source+"^{tree}").decode().strip() == manifest["source_tree"]
    assert set(git("diff","--name-only",manifest["preregistration"],source).decode().splitlines()) == set(manifest["implementation_files"])
    for group in ["implementation_files","unchanged_sources"]:
        for path, expected in manifest[group].items():
            blob = git("show",source+":"+path).replace(b"\r\n",b"\n")
            assert sha(blob) == expected, path
            if group == "unchanged_sources":
                assert blob == git("show",manifest["baseline"]+":"+path).replace(b"\r\n",b"\n"), path
    for path, expected in manifest["frozen_inputs"].items():
        assert sha(git("show",source+":"+path)) == sha(git("show",manifest["baseline"]+":"+path)) == expected
        assert sha((ROOT/path).read_bytes()) == expected
    prior = manifest["prior_negative"]
    assert sha((ROOT/prior["path"]).read_bytes()) == sha(git("show",manifest["baseline"]+":"+prior["path"])) == prior["sha256"]
    assert b"4 passed; 2 failed;" in (ROOT/prior["path"]).read_bytes()
    contract = tomllib.loads(git("show",source+":docs/testing/performance/0.74/diagnostic-signer-owned-root-isolation-contract.toml").decode())
    assert all(contract[k] == v for k,v in manifest["bounds"].items())
    assert all(contract[k] is False for k in set(contract)&set(manifest["closed_claims"]))
    base = "tools/long-run-supervisor-074/src/"
    old = git("show",manifest["baseline"]+":"+base+"diagnostic_signer_isolation_linux.rs").decode()
    new = git("show",source+":"+base+"diagnostic_signer_isolation_linux.rs").decode()
    core = git("show",source+":"+base+"diagnostic_signer_isolation_core.rs").decode()
    normalized = new.replace('#[path = "diagnostic_signer_isolation_core.rs"]\nmod core;\n',"")
    assert old.split("    let limit = libc::rlimit")[0] == normalized.split("    let mut filters = program()")[0]
    assert old.split("    let after = status()?;")[1].split("fn native()")[0] == new.split("    let after = status()?;")[1].split("pub(super) fn run(")[0]
    assert old.split("fn native()")[1].split("pub(super) fn run(")[0] == core.split("fn native()")[1].split("pub(super) fn exercise(")[0]
    assert "PR_GET_DUMPABLE" not in core
    print("ordinary root isolation packet OK; policy/proc checks retained; direct readback refused; admission=false")

if __name__ == "__main__":
    verify()
