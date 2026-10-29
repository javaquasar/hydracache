"""Reviewed CI build coordinator. Never reads a private key or starts a workload."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess

SOURCE = "62114be0f5da3218706e30d7424acfb5d0579d07"
TREE = "a9f059d752f5497c8dc5f07fbee4db6b22a366f4"
COMMAND = ["cargo", "+1.94.0", "build", "--manifest-path",
           "tools/get-owner-scheduled-controls-074/Cargo.toml", "--release", "--locked",
           "--no-default-features", "--bin", "timing-controls-074", "--target",
           "x86_64-unknown-linux-gnu", "--message-format=json"]
HASHES = {
    "Cargo.lock.root": "be46eaf8e97e507bda5e3f2b671d0d622e6a639ce1d4613769a4693723d62b19",
    "Cargo.lock.observer": "e3be470f5a1bff4e917e841fc5bfbd257ff12559206bbc1481c3c4bca528eb1d",
    "embedded.json": "5c219730c8a782ed47dfa0b291a29f44f753c3688f8548c136174a89cee55149",
    "direct.json": "5485b10f50906833006ad3f3a40b4909760ef01e3f2c86395e4366bf83cc6ef9",
    "resp2.json": "f0aba16dfe0354801d0b3c83ee9fc2d1b4751be5f50280634690ed4bcf3030ae",
    "resp3.json": "b680683c291d0a4dadbac1901f051ba0fedcac0ac9127d197e0261fe6fdabdd7",
}


def canonical(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()


def build_environment(environment):
    # No runner/controller credentials, signing secrets, inherited Rust/Cargo
    # overrides or target/profile wrappers reach build scripts or validation.
    return {k: v for k, v in environment.items()
            if k in {"PATH", "HOME", "LANG", "TMPDIR"}}


def source_observation(source, environment):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(source), *args],
                                       env=environment, timeout=30).decode().strip()
    commit = git("rev-parse", "HEAD")
    tree = git("rev-parse", "HEAD^{tree}")
    status = git("status", "--porcelain=v1", "--untracked-files=normal")
    if commit != SOURCE or tree != TREE or status:
        raise ValueError("observer source identity/cleanliness refused")
    return commit, tree


def checked_inputs(source, configs):
    paths = {"Cargo.lock.root": source / "Cargo.lock",
             "Cargo.lock.observer": source / "tools/get-owner-scheduled-controls-074/Cargo.lock"}
    paths.update({name: configs / name for name in HASHES if name.endswith(".json")})
    result = {}
    for name, path in paths.items():
        if path.is_symlink() or not path.is_file() or path.stat().st_size > 1048576:
            raise ValueError("fixed input leaf refused")
        raw = path.read_bytes()
        if hashlib.sha256(raw).hexdigest() != HASHES[name]:
            raise ValueError("fixed input bytes refused")
        result[name] = raw
    return result


def build(source, configs, output):
    source, configs, output = source.resolve(), configs.resolve(), output.resolve()
    environment = build_environment(os.environ)
    before = source_observation(source, environment)
    inputs = checked_inputs(source, configs)
    if (source / "tools/get-owner-scheduled-controls-074/target").exists():
        raise ValueError("fresh observer checkout required; existing target refused")
    output.mkdir(mode=0o700, parents=False, exist_ok=False)
    rustc = subprocess.check_output(["rustc", "+1.94.0", "--version"], env=environment, timeout=30).decode().strip()
    cargo = subprocess.check_output(["cargo", "+1.94.0", "--version"], env=environment, timeout=30).decode().strip()
    if rustc != "rustc 1.94.0 (4a4ef493e 2026-03-02)" or cargo != "cargo 1.94.0 (85eff7c80 2026-01-15)":
        raise ValueError("toolchain refused")
    with (output / "build-log.jsonl").open("xb") as log, (output / "compiler-stderr.log").open("xb") as errors:
        subprocess.run(COMMAND, cwd=source, env=environment, stdout=log, stderr=errors,
                       timeout=1800, check=True)
    after = source_observation(source, environment)
    if checked_inputs(source, configs) != inputs:
        raise ValueError("fixed inputs drifted across build")
    binary = source / "tools/get-owner-scheduled-controls-074/target/x86_64-unknown-linux-gnu/release/timing-controls-074"
    if binary.is_symlink() or not binary.is_file() or not 0 < binary.stat().st_size <= 134217728:
        raise ValueError("observer binary refused")
    shutil.copyfile(binary, output / "timing-controls-074")
    for name, raw in inputs.items():
        (output / name).write_bytes(raw)
    for surface in ("embedded", "direct", "resp2", "resp3"):
        raw = subprocess.check_output([str(binary), "--validate", str(configs / (surface + ".json"))],
                                      cwd=source, env=environment, timeout=30)
        report = json.loads(raw)
        if report.get("valid") is not True or report.get("fixture_started") is not False or report.get("admission_allowed") is not False:
            raise ValueError("validation-only observer refused")
        (output / ("validate-" + surface + ".json")).write_bytes(raw)
    if source_observation(source, environment) != after or checked_inputs(source, configs) != inputs:
        raise ValueError("source or inputs drifted across validation")
    (output / "observation.json").write_bytes(canonical({
        "schema_version": "diagnostic-builder-observation-074-v1",
        "source_commit_before": before[0], "source_commit_after": after[0],
        "source_tree_before": before[1], "source_tree_after": after[1],
        "source_clean_before": True, "source_clean_after": True,
        "rustc_version": rustc, "cargo_version": cargo, "build_command": COMMAND,
    }))


def policy(environment):
    value = {"schema_version": "diagnostic-builder-policy-074-v1", "repository_id": 1217101761,
             "builder_id": "hydracache-linux-observer-074-v1",
             "builder_key_hex": environment["BUILDER_PUBLIC_KEY_HEX"],
             "controller_key_hex": environment["CONTROLLER_PUBLIC_KEY_HEX"]}
    for key in (value["builder_key_hex"], value["controller_key_hex"]):
        if len(key) != 64 or any(c not in "0123456789abcdef" for c in key):
            raise ValueError("public key configuration refused")
    if value["builder_key_hex"] == value["controller_key_hex"]:
        raise ValueError("separate builder key required")
    raw = canonical(value)
    if hashlib.sha256(raw).hexdigest() != environment["BUILDER_POLICY_SHA256"]:
        raise ValueError("externally reviewed policy pin refused")
    return raw


def check_protection(environment, branches):
    if environment.get("name") != "performance-diagnostic-builder-074":
        raise ValueError("wrong signing environment")
    if environment.get("can_admins_bypass") is not False:
        raise ValueError("administrator review bypass must be disabled")
    if environment.get("deployment_branch_policy") != {
            "protected_branches": False, "custom_branch_policies": True}:
        raise ValueError("explicit branch restriction required")
    reviewers = [r for r in environment.get("protection_rules", [])
                 if r.get("type") == "required_reviewers"]
    if len(reviewers) != 1 or not reviewers[0].get("reviewers"):
        raise ValueError("human signing review required")
    policies = branches.get("branch_policies", [])
    if branches.get("total_count") != 1 or len(policies) != 1 or \
            policies[0].get("name") != "feat/0.74-resp-native-throughput" or \
            policies[0].get("type") != "branch":
        raise ValueError("one exact reviewed branch required")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_subparsers(dest="mode", required=True)
    command = modes.add_parser("build")
    for name in ("source", "configs", "output"):
        command.add_argument(name, type=Path)
    command = modes.add_parser("policy")
    command.add_argument("output", type=Path)
    command = modes.add_parser("check-protection")
    command.add_argument("environment", type=Path)
    command.add_argument("branches", type=Path)
    args = parser.parse_args()
    if args.mode == "build":
        build(args.source, args.configs, args.output)
    elif args.mode == "policy":
        raw = policy(os.environ)
        with args.output.open("xb") as file:
            file.write(raw)
    else:
        check_protection(json.loads(args.environment.read_bytes()), json.loads(args.branches.read_bytes()))
