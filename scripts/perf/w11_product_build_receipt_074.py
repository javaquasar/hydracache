"""Join independently admitted batch evidence to the protected product build.

The workflow supplies the approval from its reviewed source and evidence
inputs. A build artifact has no authority to select its own pins.
"""

from dataclasses import dataclass
from hashlib import sha256
import json
import os
import re
import stat
from uuid import UUID

from . import product_build_receipt_074 as build


RECEIPT_SCHEMA = "hydracache-product-build-074-v2"
EVIDENCE_SCHEMA = "w11-batch-boundary-074-v1"
PREDICATE = f"https://github.com/{build.REPOSITORY}/attestations/product-build-074/v2"
SELECTOR = "w11-256"
PROFILE_SHA256 = "204b0eb3a41fc762cb1a58cecce87119f6779870495710fbf596cad3ca5214aa"
EVIDENCE = "batch-boundary-evidence.json"
RECEIPT = "build-receipt.json"
BUNDLE = "build-receipt.sigstore.json"
MAX_JSON_BYTES = 256 * 1024
ACCEPTED = (128, 129, 256)
REFUSED = (257,)
COMMANDS = ("MGET", "MSET")


@dataclass(frozen=True)
class ApprovedRole:
    role: str
    source_commit: str
    source_tree: str
    cargo_lock_sha256: str
    boundary_evidence_sha256: str

    def __post_init__(self):
        build.require(self.role in build.ROLES, "role must be I74 or C74")
        build.require(is_hex(self.source_commit, 40) and is_hex(self.source_tree, 40),
                      "approved successor source is required")
        build.require(all(self.source_commit != commit and self.source_tree != tree
                          for commit, tree in build.ROLES.values()),
                      "legacy 128-entry sources lack W11 build admission")
        build.require(is_hex(self.cargo_lock_sha256, 64)
                      and is_hex(self.boundary_evidence_sha256, 64),
                      "approved lock and boundary evidence digests are required")


def is_hex(value, length):
    return isinstance(value, str) and re.fullmatch(f"[0-9a-f]{{{length}}}", value) is not None


def canonical(value):
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()


def config(selector):
    return {
        "resp_array_items": 1024,
        "resp_frame_bytes": 1048576,
        "selector": selector,
        "shared_batch_entries": 128 if selector is None else 256,
        "shared_batch_payload_bytes": 8388608,
        "shared_value_bytes": 16777216,
    }


def read_json_bytes(path):
    info = path.lstat()
    build.require(stat.S_ISREG(info.st_mode) and info.st_nlink == 1
                  and 0 < info.st_size <= MAX_JSON_BYTES, "expected one bounded regular JSON file")
    with path.open("rb") as stream:
        opened = os.fstat(stream.fileno())
        build.require(stat.S_ISREG(opened.st_mode) and opened.st_nlink == 1
                      and (opened.st_dev, opened.st_ino, opened.st_size)
                      == (info.st_dev, info.st_ino, info.st_size), "JSON file changed during open")
        data = stream.read(MAX_JSON_BYTES + 1)
    build.require(len(data) == info.st_size, "JSON bytes changed during read")
    return data


def generation(observed, image_sha256):
    build.require(isinstance(observed, dict)
                  and set(observed) == {"boot_id", "pid", "start_ticks", "elf_sha256"},
                  "incomplete process identity")
    boot = observed["boot_id"]
    build.require(isinstance(boot, str) and str(UUID(boot)) == boot,
                  "expected the observed Linux boot identity")
    pid, ticks = observed["pid"], observed["start_ticks"]
    build.require(type(pid) is int and 0 < pid <= 0xFFFFFFFF
                  and type(ticks) is int and 0 < ticks <= 0xFFFFFFFFFFFFFFFF
                  and observed["elf_sha256"] == image_sha256, "wrong daemon generation or image")
    return boot, pid, ticks


def validate_run(observed, selector, image_sha256, accepted, refused):
    build.require(isinstance(observed, dict)
                  and set(observed) == {"effective_config", "before", "after", "cases"},
                  "unexpected boundary run fields")
    build.require(canonical(observed["effective_config"]) == canonical(config(selector)),
                  "wrong effective startup profile or resource guard")
    original = generation(observed["before"], image_sha256)
    build.require(original == generation(observed["after"], image_sha256),
                  "boundary checks span different process generations")

    expected = {(command, count, outcome)
                for command in COMMANDS
                for outcome, counts in (("accepted", accepted), ("refused", refused))
                for count in counts}
    cases = observed["cases"]
    build.require(isinstance(cases, list) and len(cases) == len(expected),
                  "incomplete batch boundaries")
    for case in cases:
        build.require(isinstance(case, dict) and set(case) == {
            "command", "entries", "outcome", "request_sha256", "response_sha256", "oracle_sha256",
        }, "incomplete boundary observation")
        command, count, outcome = case["command"], case["entries"], case["outcome"]
        build.require(isinstance(command, str) and type(count) is int and isinstance(outcome, str)
                      and (command, count, outcome) in expected, "wrong or duplicate batch outcome")
        build.require(all(is_hex(case[name], 64) for name in (
            "request_sha256", "response_sha256", "oracle_sha256",
        )), "missing wire or semantic evidence reference")
        expected.remove((command, count, outcome))
    return original


def expected_receipt(approved, directory, binding):
    build.require(set(binding) == {"workflow_commit", "run_id", "run_attempt"},
                  "unexpected workflow binding fields")
    commit, run_id, attempt = (binding[key] for key in ("workflow_commit", "run_id", "run_attempt"))
    build.require(is_hex(commit, 40) and all(
        isinstance(value, str) and 1 <= len(value) <= 20
        and re.fullmatch("[1-9][0-9]*", value) and int(value) <= 0xFFFFFFFFFFFFFFFF
        for value in (run_id, attempt)
    ), "workflow run identity differs from the Start reader")
    source = (approved.source_commit, approved.source_tree)
    result = build.receipt_for_source(approved.role, directory, source,
                                      approved.cargo_lock_sha256, binding)
    evidence_bytes = read_json_bytes(directory / EVIDENCE)
    digest = sha256(evidence_bytes).hexdigest()
    build.require(digest == approved.boundary_evidence_sha256,
                  "batch boundary evidence differs from its admission pin")
    evidence = build.decode_json(evidence_bytes)
    build.require(isinstance(evidence, dict) and set(evidence) == {
        "schema", "build", "default", SELECTOR,
    } and evidence["schema"] == EVIDENCE_SCHEMA, "unsupported batch evidence document")
    result["schema"] = RECEIPT_SCHEMA
    build.require(canonical(evidence["build"]) == canonical(result),
                  "batch evidence belongs to a different build, role or workflow run")
    image = result["binary"]["sha256"]
    default = validate_run(evidence["default"], None, image, (128,), (129,))
    opt_in = validate_run(evidence[SELECTOR], SELECTOR, image, ACCEPTED, REFUSED)
    build.require(default != opt_in, "default and opt-in require separate startup observations")
    profile_sha256 = sha256(canonical(config(SELECTOR))).hexdigest()
    build.require(profile_sha256 == PROFILE_SHA256, "profile digest differs from the Start reader")
    result["w11_profile"] = {"selector": SELECTOR, "config_sha256": profile_sha256,
                             "boundary_evidence_sha256": digest,
                             "accepted": list(ACCEPTED), "refused": list(REFUSED)}
    return result


def check_inventory(approved, directory, *names):
    allowed = {f"{approved.role}-server", EVIDENCE, *names}
    build.require({file.name for file in directory.iterdir()} == allowed,
                  "unexpected files in the W11 build artifact")


def write_receipt(approved, source, directory):
    check_inventory(approved, directory)
    build.verify_source_identity(source, (approved.source_commit, approved.source_tree),
                                 approved.cargo_lock_sha256)
    expected = expected_receipt(approved, directory, build.workflow_binding())
    with (directory / RECEIPT).open("xb") as stream:
        stream.write(canonical(expected) + b"\n")


def verify_receipt(approved, directory):
    check_inventory(approved, directory, RECEIPT)
    expected = expected_receipt(approved, directory, build.workflow_binding())
    build.require(canonical(build.decode_json(read_json_bytes(directory / RECEIPT)))
                  == canonical(expected), "W11 receipt differs from the expected role and run")


def verify_signed_receipt(approved, directory, commit, run_id, attempt):
    binding = build.approved_workflow_run(commit, run_id, attempt)
    check_inventory(approved, directory, RECEIPT, BUNDLE)
    bundle = (directory / BUNDLE).lstat()
    build.require(stat.S_ISREG(bundle.st_mode) and bundle.st_nlink == 1 and bundle.st_size > 0,
                  "expected a regular Sigstore bundle")
    expected = expected_receipt(approved, directory, binding)
    receipt = read_json_bytes(directory / RECEIPT)
    build.require(canonical(build.decode_json(receipt)) == canonical(expected),
                  "W11 receipt differs from the expected role and run")
    build.verify_attestation(approved.role, directory, commit, expected, PREDICATE)
    return sha256(receipt).hexdigest()
