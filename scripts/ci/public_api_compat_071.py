#!/usr/bin/env python3
import argparse
import datetime
import json
import pathlib
import subprocess
import sys
from typing import Any


def classify(returncode: int) -> str:
    if returncode == 0:
        return "compatible"
    if returncode == 100:
        return "semver-violation"
    return "analysis-or-tool-failure"


def validate_manifest(manifest: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    expected = {
        "schema_version": 1,
        "release": "0.71",
        "baseline_tag": "v0.70.0",
        "baseline_commit": "75719b0bf5de2250cf4eb16a30073dd7429538e3",
        "tool": "cargo-semver-checks",
        "tool_version": "0.49.0",
    }
    for field, value in expected.items():
        if manifest.get(field) != value:
            problems.append(f"public API manifest {field} mismatch")
    packages = manifest.get("packages")
    if not isinstance(packages, list) or not packages or len(packages) != len(set(packages)):
        problems.append("public API manifest package set is empty or duplicated")
    if manifest.get("profiles") != ["default", "all-features"]:
        problems.append("public API manifest must run default and all-features profiles")
    return problems


def publishable_packages(metadata: dict[str, Any]) -> set[str]:
    members = set(metadata.get("workspace_members", []))
    return {
        package["name"]
        for package in metadata.get("packages", [])
        if package.get("id") in members and package.get("publish") != []
    }


def execute(root: pathlib.Path, manifest: dict[str, Any], output: pathlib.Path) -> int:
    rows = []
    output.parent.mkdir(parents=True, exist_ok=True)
    logs = output.parent / "logs"
    logs.mkdir(exist_ok=True)
    for package in manifest["packages"]:
        for profile in manifest["profiles"]:
            command = [
                "cargo",
                "semver-checks",
                "check-release",
                "--package",
                package,
                "--baseline-rev",
                manifest["baseline_tag"],
            ]
            if profile == "all-features":
                command.append("--all-features")
            completed = subprocess.run(
                command,
                cwd=root,
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                timeout=1800,
                check=False,
            )
            log = logs / f"{package}-{profile}.log"
            log.write_text(completed.stdout, encoding="utf-8")
            rows.append(
                {
                    "package": package,
                    "profile": profile,
                    "returncode": completed.returncode,
                    "classification": classify(completed.returncode),
                    "log": str(log.relative_to(output.parent)).replace("\\", "/"),
                }
            )
    receipt = {
        "schema_version": 1,
        "release": manifest["release"],
        "baseline_tag": manifest["baseline_tag"],
        "baseline_commit": manifest["baseline_commit"],
        "tool": manifest["tool"],
        "tool_version": manifest["tool_version"],
        "candidate_sha": git(root, "rev-parse", "HEAD"),
        "generated_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "result": "success" if all(row["returncode"] == 0 for row in rows) else "failure",
        "rows": rows,
    }
    output.write_text(json.dumps(receipt, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return 0 if receipt["result"] == "success" else 1


def git(root: pathlib.Path, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=root, text=True).strip()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=pathlib.Path, required=True)
    parser.add_argument("--output", type=pathlib.Path)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[2]
    manifest = json.loads(args.manifest.read_text(encoding="utf-8"))
    found = validate_manifest(manifest)
    if found:
        raise SystemExit("\n".join(found))
    actual = git(root, "rev-parse", f"{manifest['baseline_tag']}^{{commit}}")
    if actual != manifest["baseline_commit"]:
        raise SystemExit(f"public API baseline tag mismatch: {actual}")
    metadata = json.loads(
        subprocess.check_output(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            cwd=root,
            text=True,
        )
    )
    declared = set(manifest["packages"])
    actual_packages = publishable_packages(metadata)
    if declared != actual_packages:
        raise SystemExit(
            "public API package inventory mismatch: "
            f"missing={sorted(actual_packages - declared)} extra={sorted(declared - actual_packages)}"
        )
    if args.check:
        print("public API compatibility 0.71 manifest: OK")
        return 0
    if args.output is None:
        parser.error("--output is required unless --check is used")
    return execute(root, manifest, args.output)


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except subprocess.TimeoutExpired as error:
        print(f"public API compatibility timed out: {error}", file=sys.stderr)
        raise SystemExit(1)
