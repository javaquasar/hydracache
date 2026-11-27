"""Create the RustSec input for reviewing the upstream sources of local forks."""

import argparse
import json
from pathlib import Path
import re
import tomllib


def upstream_lock(forks):
    if not forks:
        raise ValueError("fork inventory is empty")
    blocks = ["version = 4"]
    for name, version in sorted(forks.items()):
        if not re.fullmatch(r"hydra-[a-z0-9-]+", name):
            raise ValueError(f"invalid fork package name {name!r}")
        match = re.fullmatch(r"(\d+\.\d+\.\d+)-hydra\.[1-9]\d*", version)
        if match is None:
            raise ValueError(f"invalid fork version {name} {version!r}")
        blocks.append(
            "[[package]]\n"
            f"name = {json.dumps(name.removeprefix('hydra-'))}\n"
            f"version = {json.dumps(match[1])}\n"
            'source = "registry+https://github.com/rust-lang/crates.io-index"'
        )
    return "\n\n".join(blocks) + "\n"


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("inventory", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    with args.inventory.open("rb") as inventory:
        content = upstream_lock(tomllib.load(inventory)["forks"])
    args.output.write_text(content, encoding="utf-8")


if __name__ == "__main__":
    main()
