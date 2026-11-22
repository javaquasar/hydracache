"""Build and verify the protected single-role I74 maintenance receipt."""
import argparse
import json
from pathlib import Path
import re
import stat

from . import product_build_receipt_074 as build


SOURCE = ("2efd4dce889fa8b1001e4696711eab5979b03ea0", "2693efc2a4bc82244023f627b7333d3d8d7e1c68")
LOCK = "d7cd621a1a95dc3c2d8356d21390cbb37dd606118bcc457171638e67eaed6172"
SCHEMA = "hydracache-product-maintenance-074-v1"
PREDICATE = f"https://github.com/{build.REPOSITORY}/attestations/product-maintenance-074/v1"
RECEIPT = "build-receipt.json"
BUNDLE = "build-receipt.sigstore.json"


def receipt(directory, binding):
    value = build.receipt_for_source("I74", directory, SOURCE, LOCK, binding)
    value["schema"] = SCHEMA
    value["purpose"] = "maintenance-release-candidate"
    return value


def receipt_bytes(value):
    return (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode("utf-8")


def write_receipt(source, directory):
    build.verify_source_identity(source, SOURCE, LOCK)
    value = receipt(directory, build.workflow_binding())
    with (directory / RECEIPT).open("xb") as stream:
        stream.write(receipt_bytes(value))
    return value["binary"]["sha256"]


def verify(directory, binding, *, built_sha256=None, signed=False):
    members = {"I74-server", RECEIPT}
    if signed:
        members.add(BUNDLE)
    build.require({p.name for p in directory.iterdir()} == members, "unexpected maintenance artifact inventory")
    for name in members - {"I74-server"}:
        info = (directory / name).lstat()
        build.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1 and info.st_size > 0,
                      f"invalid maintenance artifact member {name}")
    expected = receipt(directory, binding)
    encoded = receipt_bytes(expected)
    with (directory / RECEIPT).open("rb") as stream:
        build.require(stream.read(len(encoded) + 1) == encoded,
                      "maintenance receipt differs from the pinned source, binary or workflow run")
    if signed:
        build.verify_attestation("I74", directory, binding["workflow_commit"], expected, PREDICATE)
    else:
        build.require(re.fullmatch(r"[0-9a-f]{64}", built_sha256 or "") and
                      expected["binary"]["sha256"] == built_sha256, "binary differs from the build job output")


def main():
    parser = argparse.ArgumentParser()
    modes = parser.add_subparsers(dest="mode", required=True)
    modes.add_parser("source").add_argument("source", type=Path)
    make = modes.add_parser("receipt")
    make.add_argument("source", type=Path)
    make.add_argument("directory", type=Path)
    unsigned = modes.add_parser("verify")
    unsigned.add_argument("directory", type=Path)
    unsigned.add_argument("built_sha256")
    signed = modes.add_parser("verify-signed")
    signed.add_argument("directory", type=Path)
    for name in ("workflow_commit", "run_id", "run_attempt"):
        signed.add_argument(name)
    args = parser.parse_args()
    if args.mode == "source":
        build.verify_source_identity(args.source, SOURCE, LOCK)
    elif args.mode == "receipt":
        print(write_receipt(args.source, args.directory))
    elif args.mode == "verify":
        verify(args.directory, build.workflow_binding(), built_sha256=args.built_sha256)
    else:
        binding = build.approved_workflow_run(args.workflow_commit, args.run_id, args.run_attempt)
        verify(args.directory, binding, signed=True)


if __name__ == "__main__":
    main()
