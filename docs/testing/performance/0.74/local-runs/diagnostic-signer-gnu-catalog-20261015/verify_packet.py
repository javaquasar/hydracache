"""Offline evidence consistency check; never a runtime trust or admission loader."""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

PACKET = Path(__file__).resolve().parent
ROOT = PACKET.parents[5]
SOURCE = "f10f5d64bdc752dabb40b403d9c055dd5a1d59ac"
TREE = "5018405379547bc02f0b61f9adfde961e6f4851b"
PREREG = "1076bef53595b229ba0e617a43811e44f0ef6689"
FROZEN = {
    "Cargo.lock": "99c8cf65fd709e390727d1441117b85e84fa9fe120cac9a94e35c5d44fc2fcc7",
    "docs/testing/performance/0.74/qualification-manifest.toml":
        "11917570528020b5e1eb275a5ad9509ba358e6a6ca044647d1235b685659b0bc",
}
CLOSED = (
    "complete_elf_validation", "loaded_closure_proved", "runtime_custody_authenticated",
    "original_start_authenticated", "signing_allowed", "admission_allowed",
)
ROLES = ["signer", "ld-linux-x86-64.so.2", "libc.so.6", "libgcc_s.so.1"]
NEEDED = [[ROLES[3], ROLES[2], ROLES[1]], [], [ROLES[1]], [ROLES[2], ROLES[1]]]
PASSES = {
    "baseline-windows.log": [9], "baseline-linux.log": [10],
    "first-green-windows.log": [6], "windows-regression.log": [9, 6],
    "windows-msrv.log": [6], "linux-msrv.log": [6, 2],
    "linux-checks.log": [6, 6, 2, 10, 11, 2], "root-final.log": [153],
}


def sha(raw):
    return hashlib.sha256(raw).hexdigest()


def unique(pairs):
    result = {}
    for key, value in pairs:
        assert key not in result, key
        result[key] = value
    return result


def read_json(name):
    return json.loads((PACKET / name).read_bytes(), object_pairs_hook=unique)


def git(*args):
    env = os.environ.copy()
    # Linked worktree metadata uses Windows paths; translate only in Linux WSL.
    marker = ROOT / ".git"
    if os.name != "nt" and marker.is_file():
        raw = marker.read_text().strip().removeprefix("gitdir: ")
        if re.match(r"^[A-Za-z]:/", raw):
            env["GIT_DIR"] = "/mnt/" + raw[0].lower() + raw[2:]
            env["GIT_WORK_TREE"] = str(ROOT)
    return subprocess.check_output(["git", *args], cwd=ROOT, env=env)


def verify():
    m = read_json("manifest.json")
    assert m["schema_version"] == "diagnostic-signer-gnu-catalog-packet-074-v1"
    assert m["source_commit"] == SOURCE and m["source_tree"] == TREE
    assert m["preregistration"] == PREREG
    assert git("rev-parse", SOURCE + "^{tree}").decode().strip() == TREE
    assert m["frozen_inputs"] == FROZEN
    for path, digest in FROZEN.items():
        assert sha((ROOT / path).read_bytes()) == digest, path
        assert sha(git("show", SOURCE + ":" + path)) == digest, path
    expected = {p.name for p in PACKET.iterdir() if p.suffix in (".log", ".json", ".py", ".md")}
    expected.remove("manifest.json")
    assert set(m["files"]) == expected
    for name, row in m["files"].items():
        assert Path(name).name == name
        raw = (PACKET / name).read_bytes()
        assert len(raw) == row["bytes"] and sha(raw) == row["sha256"], name
    for name, counts in PASSES.items():
        found = re.findall(r"test result: ok\. (\d+) passed; 0 failed;", (PACKET / name).read_text())
        assert [int(n) for n in found] == counts, name
    assert "error[E0432]" in (PACKET / "red-missing-api.log").read_text()
    assert "E0603" not in (PACKET / "red-missing-api.log").read_text()
    assert "FAILED. 152 passed; 1 failed" in (PACKET / "windows-guard-development-failure.log").read_text()
    assert "171 discovered gates, all registered" in (PACKET / "gated-check.log").read_text()
    final = (PACKET / "root-final.log").read_text()
    assert "local, non-promotable" in final and "ship admission is closed" in final
    c, s = read_json("actual-catalog.json"), read_json("actual-status.json")
    assert c["schema_version"] == "diagnostic-signer-gnu-declarations-074-v1"
    assert all(c[k] is False for k in CLOSED)
    assert s["schema_version"] == "diagnostic-signer-gnu-catalog-inventory-074-v1"
    assert s["tooling_source"] == SOURCE and s["tooling_tree"] == TREE
    for key in ("new_receipt_signed", "explicit_inspected_image_execution_requested",
                "loaded_runtime_attested", "signing_or_token_authority_exposed",
                "server_or_github_operation", "qualification_allowed", "ship_allowed"):
        assert s[key] is False, key
    assert s["git_clean_before_and_after_observations"] is True
    assert s["inputs_and_inspector_hash_metadata_unchanged"] is True
    assert type(s["positive_exit_code"]) is int and s["positive_exit_code"] == 0
    assert type(s["negative_native_exit_code"]) is int and s["negative_native_exit_code"] == 2
    assert s["initial_negative_shell_exit_code"] == 1 and s["negative_readonly_invocations"] == 2
    assert s["signer_source_reported_by_prior_unsigned_inventory"] == "16e9420313ef842e19183c7f6f189eb2e10207cf"
    before = (PACKET / "inputs-before.log").read_bytes()
    assert before == (PACKET / "inputs-after.log").read_bytes()
    rows = before.decode().splitlines()
    assert len(rows) == 10 and len(s["input_paths"]) == 4 and len(c["images"]) == 4
    paths = s["input_paths"] + [s["inspector_binary"]]
    hashes = []
    for index, path in enumerate(paths):
        digest, captured_path = rows[index].split("  ", 1)
        assert captured_path == path and re.fullmatch(r"[0-9a-f]{64}", digest)
        hashes.append(digest)
        stat = rows[index + 5].split("|")
        assert stat[0] == path and len(stat) == 10
        if index < 4:
            image = c["images"][index]
            assert image["role"] == ROLES[index] and image["sha256"] == digest
            assert type(image["bytes"]) is int and image["bytes"] == int(stat[3])
            assert 64 <= image["bytes"] <= 8388608 and stat[7] == "1"
            assert image["needed"] == NEEDED[index] and image["elf_kind"] == 3
            assert image["soname"] == (None if index == 0 else ROLES[index])
            assert image["interpreter"] == ("/lib64/ld-linux-x86-64.so.2" if index in (0, 2) else None)
            assert image["program_headers"] == [12, 11, 14, 12][index]
    assert hashes[4] == s["inspector_sha256"]
    assert "observed_child_exit=2" in (PACKET / "role-refusal-native-exit.log").read_text()
    print("Verified local GNU declaration packet; custody, signing, start and ship remain closed.")


if __name__ == "__main__":
    verify()
