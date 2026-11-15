"""Bind the two reviewed Linux role builds to their exact workflow run."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import subprocess

ROLES = {
    "I74": ("080d9b7897f2de1be51fd21f78eb525b4628366a", "a4e2d5f2b63ce01d1a9f517f800678156ba64a88"),
    "C74": ("572bbcb2f110548a5f668a9f7ce2aa10c9371964", "e76179a97551cdebae04f92e18e33f1498f3ebc1"),
}
ENVIRONMENT = "performance-product-build-074"
LOCK = "5cc6a2096fe9a505d619a0efb6a2a36ec59dd20175cdb936b62344fd9aaeabb0"
REPOSITORY = "javaquasar/hydracache"
REPOSITORY_ID = "1217101761"
WORKFLOW = f"{REPOSITORY}/.github/workflows/performance-product-build-074.yml@refs/heads/main"
PREDICATE = f"https://github.com/{REPOSITORY}/attestations/product-build-074/v1"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def load(path):
    def unique(pairs):
        result = dict(pairs)
        require(len(result) == len(pairs), "duplicate JSON property")
        return result

    require(path.stat().st_size <= 256 * 1024, "JSON input exceeds 256 KiB")
    return json.loads(path.read_bytes(), object_pairs_hook=unique)


def protection(branch, environment, policies):
    require(branch.get("name") == "main" and branch.get("protected") is True,
            "protected main is required")
    require(environment.get("name") == ENVIRONMENT, "unexpected signing environment")
    require(environment.get("deployment_branch_policy") == {
        "protected_branches": False, "custom_branch_policies": True,
    }, "signing environment must restrict deployments to main")
    rules = [rule for rule in environment["protection_rules"] if rule.get("type") == "required_reviewers"]
    require(len(rules) == 1 and rules[0].get("prevent_self_review") is True and rules[0].get("reviewers"),
            "signing requires an independent deployment reviewer")
    require(environment.get("can_admins_bypass") is False, "administrator bypass must be disabled")
    entries = policies["branch_policies"]
    require(policies["total_count"] == len(entries) == 1 and
            entries[0].get("name") == "main" and entries[0].get("type") == "branch",
            "signing accepts exactly the main branch")


def workflow_binding():
    require(os.environ.get("GITHUB_REPOSITORY") == REPOSITORY
            and os.environ.get("GITHUB_REPOSITORY_ID") == REPOSITORY_ID
            and os.environ.get("GITHUB_REF") == "refs/heads/main"
            and os.environ.get("GITHUB_WORKFLOW_REF") == WORKFLOW,
            "unexpected workflow identity")
    values = {key: os.environ[key] for key in ("GITHUB_SHA", "GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")}
    require(re.fullmatch("[0-9a-f]{40}", values["GITHUB_SHA"]) and
            all(re.fullmatch("[1-9][0-9]*", values[key]) for key in ("GITHUB_RUN_ID", "GITHUB_RUN_ATTEMPT")),
            "invalid workflow run identity")
    return {"workflow_commit": values["GITHUB_SHA"], "run_id": values["GITHUB_RUN_ID"],
            "run_attempt": values["GITHUB_RUN_ATTEMPT"]}


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def verify_source(role, root):
    def git(*args):
        return subprocess.check_output(["git", "-C", str(root), *args], text=True).strip()

    require((git("rev-parse", "HEAD"), git("rev-parse", "HEAD^{tree}")) == ROLES[role],
            "unexpected source commit or tree")
    require(not git("status", "--porcelain=v1", "--untracked-files=all"),
            "dirty role checkout")
    require(sha256(root / "Cargo.lock") == LOCK, "unexpected Cargo.lock")


def receipt(role, directory, binding=None):
    binary = directory / f"{role}-server"
    info = binary.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and 64 <= info.st_size < 512 * 1024**2,
            "expected one regular binary")
    with binary.open("rb") as stream:
        header = stream.read(64)
    require(header[:7] == b"\x7fELF\x02\x01\x01" and header[18:20] == b"\x3e\x00"
            and header[16:18] in (b"\x02\x00", b"\x03\x00"), "expected a Linux x86-64 executable ELF")
    commit, tree = ROLES[role]
    return {"schema": "hydracache-product-build-074-v1", "repository_id": REPOSITORY_ID,
            "role": role, "source_commit": commit, "source_tree": tree, "cargo_lock_sha256": LOCK,
            "package": "hydracache-server", "profile": "release", "rust_toolchain": "1.94.0",
            "target": "x86_64-unknown-linux-gnu", **(workflow_binding() if binding is None else binding),
            "binary": {"name": binary.name, "sha256": sha256(binary), "bytes": info.st_size}}


def write_receipt(role, source, directory):
    verify_source(role, source)
    with (directory / "build-receipt.json").open("x", encoding="utf-8") as stream:
        json.dump(receipt(role, directory), stream, indent=2)
        stream.write("\n")


def verify_artifact(role, directory):
    require({path.name for path in directory.iterdir()} == {f"{role}-server", "build-receipt.json"},
            "unexpected files in build artifact")
    document = directory / "build-receipt.json"
    info = document.lstat()
    require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1, "expected a regular receipt file")
    require(load(document) == receipt(role, directory), "build receipt or binary does not match this run")


def verify_signed_artifact(role, directory, commit, run_id, attempt):
    require(re.fullmatch("[0-9a-f]{40}", commit) and
            all(re.fullmatch("[1-9][0-9]*", value) for value in (run_id, attempt)),
            "expected the approved workflow commit and exact run")
    bundle = directory / "build-receipt.sigstore.json"
    document = directory / "build-receipt.json"
    expected = receipt(role, directory, {"workflow_commit": commit, "run_id": run_id, "run_attempt": attempt})
    require(load(document) == expected, "receipt or image differs from the expected role and run")
    output = subprocess.check_output(
        ["gh", "attestation", "verify", str(directory / f"{role}-server"), "--bundle", str(bundle),
         "--repo", REPOSITORY, "--signer-workflow", WORKFLOW.rsplit("@", 1)[0], "--signer-digest", commit,
         "--source-ref", "refs/heads/main", "--source-digest", commit, "--deny-self-hosted-runners",
         "--predicate-type", PREDICATE, "--format", "json"],
        timeout=60,
    )
    require(any(record["verificationResult"]["statement"]["predicate"] == expected
                for record in json.loads(output)), "signed predicate differs from the reviewed build receipt")


def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_subparsers(dest="mode", required=True)
    policies = modes.add_parser("protection")
    for name in ("branch", "environment", "policies"):
        policies.add_argument(name, type=Path)
    for name in ("source", "receipt", "verify"):
        mode = modes.add_parser(name)
        mode.add_argument("role", choices=ROLES)
        if name in ("source", "receipt"):
            mode.add_argument("source", type=Path)
        if name in ("receipt", "verify"):
            mode.add_argument("artifact", type=Path)
    signed = modes.add_parser("verify-signed")
    signed.add_argument("role", choices=ROLES)
    signed.add_argument("artifact", type=Path)
    for name in ("workflow_commit", "run_id", "run_attempt"):
        signed.add_argument(name)
    args = parser.parse_args()
    if args.mode == "protection":
        protection(load(args.branch), load(args.environment), load(args.policies))
    elif args.mode == "source":
        verify_source(args.role, args.source)
    elif args.mode == "receipt":
        write_receipt(args.role, args.source, args.artifact)
    elif args.mode == "verify":
        verify_artifact(args.role, args.artifact)
    else:
        verify_signed_artifact(args.role, args.artifact, args.workflow_commit, args.run_id, args.run_attempt)


if __name__ == "__main__":
    main()
