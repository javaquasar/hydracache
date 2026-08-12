#!/usr/bin/env python3
"""Align only local subject package versions in the CI instruction lockfile."""

from __future__ import annotations

import argparse
import pathlib
import re
import tomllib


LOCAL_PACKAGES = {"hydracache", "hydracache-core", "hydracache-macros"}
PACKAGE_HEADER = "[[package]]"
NAME_RE = re.compile(r'^name = "([^"]+)"$')
VERSION_RE = re.compile(r'^(version = ")([^"]+)("\s*)$')


def package_blocks(lines: list[str]) -> list[tuple[int, int]]:
    starts = [index for index, line in enumerate(lines) if line.rstrip() == PACKAGE_HEADER]
    return [
        (start, starts[position + 1] if position + 1 < len(starts) else len(lines))
        for position, start in enumerate(starts)
    ]


def registry_packages(snapshot: dict) -> list[tuple]:
    return sorted(
        (
            package["name"],
            package["version"],
            package.get("source"),
            package.get("checksum"),
            tuple(package.get("dependencies", [])),
        )
        for package in snapshot["package"]
        if package.get("source") is not None
    )


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--lock", required=True, type=pathlib.Path)
    parser.add_argument("--subject-manifest", required=True, type=pathlib.Path)
    args = parser.parse_args()

    original = args.lock.read_text(encoding="utf-8")
    before = tomllib.loads(original)
    with args.subject_manifest.open("rb") as handle:
        subject_version = tomllib.load(handle)["workspace"]["package"]["version"]

    lines = original.splitlines(keepends=True)
    updated: set[str] = set()
    for start, end in package_blocks(lines):
        block = lines[start:end]
        name = next(
            (match.group(1) for line in block if (match := NAME_RE.match(line.rstrip()))),
            None,
        )
        if name not in LOCAL_PACKAGES:
            continue
        if any(line.startswith("source = ") for line in block):
            raise SystemExit(f"refusing to rewrite registry package {name}")
        version_indexes = [
            index for index in range(start, end) if VERSION_RE.match(lines[index].rstrip("\n\r"))
        ]
        if len(version_indexes) != 1:
            raise SystemExit(f"package {name} has {len(version_indexes)} version rows")
        index = version_indexes[0]
        newline = "\r\n" if lines[index].endswith("\r\n") else "\n"
        lines[index] = f'version = "{subject_version}"{newline}'
        updated.add(name)

    if updated != LOCAL_PACKAGES:
        raise SystemExit(
            f"local package rows were {sorted(updated)}, expected {sorted(LOCAL_PACKAGES)}"
        )

    rendered = "".join(lines)
    after = tomllib.loads(rendered)
    if registry_packages(before) != registry_packages(after):
        raise SystemExit("subject lock synchronization changed a registry package")
    actual = {
        package["name"]: package["version"]
        for package in after["package"]
        if package["name"] in LOCAL_PACKAGES and package.get("source") is None
    }
    if set(actual) != LOCAL_PACKAGES or set(actual.values()) != {subject_version}:
        raise SystemExit(
            f"subject lock synchronization produced {actual}, expected {subject_version}"
        )

    args.lock.write_text(rendered, encoding="utf-8", newline="")
    print(f"synchronized local subject packages to {subject_version}")


if __name__ == "__main__":
    main()
