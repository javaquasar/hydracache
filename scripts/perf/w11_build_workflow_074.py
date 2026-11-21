"""Move exact W11 build, observation, and receipt bytes between CI jobs."""

import argparse
from dataclasses import dataclass
from hashlib import sha256
from pathlib import Path
import shutil
import stat

from . import performance_signed_role_probe_074 as probe
from . import product_build_receipt_074 as build
from . import w11_batch_boundary_074 as batch
from . import w11_config_readback_074 as readback
from . import w11_product_build_receipt_074 as receipt


APPROVAL = Path(__file__).with_name("w11-product-build-approval-074.json")
SUBJECT = "build-subject.json"
W11_LOCK_SHA256 = "46571cd5c8e2a7aa5b659575d7828f935e7538e64d33cf148391b2fe43204a19"
ROLE_VERSION = b"0.74.0-rc.1"
CLIENT_CPU = 1
DAEMON_CPU = 2


@dataclass(frozen=True)
class SourceRole:
    role: str
    commit: str
    tree: str
    lock: str

    @property
    def source(self):
        return self.commit, self.tree

    def approval(self, proof_digest):
        return receipt.ApprovedRole(self.role, self.commit, self.tree, self.lock, proof_digest)


def approved_plan(path=APPROVAL):
    value = build.decode_json(receipt.read_json_bytes(path))
    build.require(isinstance(value, dict)
                  and set(value) == {"schema", "status", "profile_sha256", "observer_transport",
                                     "observer_runtime_sha256", "roles"}
                  and value["schema"] == "hydracache-w11-approval-074-v1"
                  and value["status"] == "approved"
                  and value["profile_sha256"] == receipt.PROFILE_SHA256,
                  "protected main must record approval of both exact W11 roles and the profile")
    build.require(value["observer_transport"] == "ssh", "an approved SSH observer is required")
    build.require(receipt.is_hex(value["observer_runtime_sha256"], 64),
                  "an approved offline guest runtime is required")
    roles = value["roles"]
    build.require(isinstance(roles, dict) and set(roles) == set(build.ROLES), "both role pins are required")
    result = {}
    for role, pin in roles.items():
        build.require(isinstance(pin, dict) and set(pin) == {"commit", "tree", "lock"},
                      "invalid role source pin")
        record = SourceRole(role, pin["commit"], pin["tree"], pin["lock"])
        build.require(receipt.is_hex(record.commit, 40) and receipt.is_hex(record.tree, 40)
                      and record.lock == W11_LOCK_SHA256
                      and all(record.commit != old_commit and record.tree != old_tree
                              for old_commit, old_tree in build.ROLES.values()),
                      "an approved successor source with the reviewed W11 dependency lock is required")
        result[role] = record
    build.require(result["I74"].commit != result["C74"].commit
                  and result["I74"].tree != result["C74"].tree, "the two role pins must be distinct")
    return result, value["observer_runtime_sha256"]


def require_directory(path):
    build.require(stat.S_ISDIR(path.lstat().st_mode) and path.absolute() == path.resolve(strict=True),
                  "expected an ordinary artifact directory")


def read_canonical(path):
    raw = receipt.read_json_bytes(path)
    value = build.decode_json(raw)
    build.require(raw == receipt.canonical(value) + b"\n", f"noncanonical {path.name}")
    return value


def subject_for(source, directory, binding):
    subject = build.receipt_for_source(source.role, directory, source.source, source.lock, binding)
    subject["schema"] = receipt.RECEIPT_SCHEMA
    return subject


def check_build(source, directory, binding):
    require_directory(directory)
    build.require({path.name for path in directory.iterdir()} == {f"{source.role}-server", SUBJECT},
                  "unexpected files in unsigned build")
    expected = subject_for(source, directory, binding)
    build.require(read_canonical(directory / SUBJECT) == expected, "unsigned build identity differs")
    probe.read_pinned(directory / expected["binary"]["name"], expected["binary"]["sha256"])
    return expected


def check_names(path, names):
    require_directory(path)
    build.require({entry.name for entry in path.iterdir()} == set(names), f"unexpected files in {path.name}")


def auth_replies(directory):
    replies = {name: batch.read_capture(directory / name, 4096) for name in batch.AUTH_FILES}
    build.require(replies["no-certificate.resp"] == b""
                  and replies["denied.resp"] == probe.NOAUTH + probe.WRONGPASS + probe.NOAUTH + batch.OK
                  and replies["fresh-connection.resp"] == probe.NOAUTH + batch.OK,
                  "retained certificate or AUTH checks differ")
    probe.verify_resp3(replies["authenticated.resp"], expected_version=ROLE_VERSION)
    return {name: sha256(data).hexdigest() for name, data in replies.items()}


def check_profile_archive(directory, selector, observed):
    cases = batch.case_parameters(selector)
    case_names = [f"{command}-{count}" for command, count, _ in cases]
    check_names(directory, ["auth", "effective-config.json", "ready.json", "listener.json", "run.json", *case_names])
    check_names(directory / "auth", batch.AUTH_FILES)
    auth = auth_replies(directory / "auth")
    config = batch.read_capture(directory / "effective-config.json", readback.MAX_BYTES)
    build.require(config == receipt.canonical(receipt.config(selector))
                  and config == receipt.canonical(observed["effective_config"]), "config readback differs")
    build.require(batch.read_capture(directory / "ready.json", len(readback.READY)) == readback.READY,
                  "daemon readiness bytes differ")
    observed_pid = observed["before"]["pid"]
    run = read_canonical(directory / "run.json")
    build.require(set(run) == {"daemon_pid", "client_cpu", "daemon_cpu", "elapsed_ns", "exit"}
                  and all(type(value) is int for value in run.values())
                  and run["daemon_pid"] == observed_pid and run["client_cpu"] == CLIENT_CPU
                  and run["daemon_cpu"] == DAEMON_CPU and run["exit"] == 0
                  and 0 < run["elapsed_ns"] <= probe.PROCESS_LIMIT * 10**9,
                  "daemon drain, placement or proof deadline differs")
    listener = read_canonical(directory / "listener.json")
    build.require(set(listener) == {"network_namespace_inode", "socket_inode", "daemon_pid", "port"}
                  and all(type(value) is int and value > 0 for value in listener.values())
                  and listener["daemon_pid"] == observed_pid and listener["port"] <= 65535,
                  "listener does not belong to the observed generation")
    checked = []
    for (command, count, outcome), name in zip(cases, case_names, strict=True):
        case = directory / name
        steps = batch.case_steps(selector, command, count, outcome)
        check_names(case, ["oracle.json", *(f"{step.name}.{suffix}.resp"
                    for step in steps for suffix in ("request", "response"))])
        checked.append(batch.read_case_archive(selector, command, count, outcome, case, auth))
    build.require(checked == observed["cases"], "case receipt differs from independent wire replay")


def verify_evidence_archive(archives, document, expected):
    check_names(archives, [SUBJECT, batch.DEFAULT, receipt.SELECTOR])
    build.require(read_canonical(archives / SUBJECT) == expected, "raw archive build identity differs")
    build.require(set(document) == {"schema", "build", batch.DEFAULT, receipt.SELECTOR}
                  and document["schema"] == receipt.EVIDENCE_SCHEMA and document["build"] == expected,
                  "batch proof does not belong to this role, binary or workflow attempt")
    image = expected["binary"]["sha256"]
    first, second = document[batch.DEFAULT], document[receipt.SELECTOR]
    a = receipt.validate_run(first, None, image, (128,), (129,))
    b = receipt.validate_run(second, receipt.SELECTOR, image, receipt.ACCEPTED, receipt.REFUSED)
    build.require(a != b, "default and opt-in reused a daemon generation")
    check_profile_archive(archives / batch.DEFAULT, None, first)
    check_profile_archive(archives / receipt.SELECTOR, receipt.SELECTOR, second)
    # The preceding verification bounds the entire tree and each member.
    paths = sorted(path for path in archives.rglob("*") if path.is_file())
    members = {path.relative_to(archives).as_posix(): build.sha256(path) for path in paths}
    return sha256(receipt.canonical(members)).hexdigest()


def verify_packet(source, directory, archives, proof_digest, archive_digest, binding):
    require_directory(directory)
    approved = source.approval(proof_digest)
    receipt.check_inventory(approved, directory, receipt.RECEIPT)
    expected = receipt.expected_receipt(approved, directory, binding)
    build.require(read_canonical(directory / receipt.RECEIPT) == expected, "verified receipt differs")
    archive = read_canonical(directory / receipt.EVIDENCE)
    subject = subject_for(source, directory, binding)
    build.require(verify_evidence_archive(archives, archive, subject) == archive_digest,
                  "raw evidence differs from the independent verification job")
    probe.read_pinned(directory / subject["binary"]["name"], subject["binary"]["sha256"])
    return expected


def finalize(source, incoming, observations, directory, binding):
    subject = check_build(source, incoming, binding)
    check_names(observations, ["archives", receipt.EVIDENCE])
    evidence = observations / receipt.EVIDENCE
    archives = observations / "archives"
    document = read_canonical(evidence)
    archive_digest = verify_evidence_archive(archives, document, subject)
    proof_digest = build.sha256(evidence)
    directory.mkdir(mode=0o700)
    for path in (incoming / subject["binary"]["name"], evidence):
        with path.open("rb") as original, (directory / path.name).open("xb") as output:
            shutil.copyfileobj(original, output)
    expected = receipt.expected_receipt(source.approval(proof_digest), directory, binding)
    with (directory / receipt.RECEIPT).open("xb") as output:
        output.write(receipt.canonical(expected) + b"\n")
    verify_packet(source, directory, archives, proof_digest, archive_digest, binding)
    return proof_digest, archive_digest


def main():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="operation", required=True)
    sub.add_parser("matrix")
    source_args = sub.add_parser("source")
    source_args.add_argument("role", choices=build.ROLES)
    source_args.add_argument("checkout", type=Path)
    built = sub.add_parser("built")
    built.add_argument("role", choices=build.ROLES)
    built.add_argument("checkout", type=Path)
    built.add_argument("artifact", type=Path)
    finish = sub.add_parser("finalize")
    finish.add_argument("role", choices=build.ROLES)
    finish.add_argument("artifact", type=Path)
    finish.add_argument("observations", type=Path)
    finish.add_argument("verified", type=Path)
    verify = sub.add_parser("verify")
    verify.add_argument("role", choices=build.ROLES)
    verify.add_argument("verified", type=Path)
    verify.add_argument("archives", type=Path)
    verify.add_argument("proof_sha256")
    verify.add_argument("archives_sha256")
    args = parser.parse_args()
    binding = build.workflow_binding()
    roles, _ = approved_plan()
    if args.operation == "matrix":
        matrix = {"include": [{"role": row.role, "commit": row.commit} for row in roles.values()]}
        print("matrix=" + receipt.canonical(matrix).decode("ascii"))
    elif args.operation in {"source", "built"}:
        source = roles[args.role]
        build.verify_source_identity(args.checkout, source.source, source.lock)
        if args.operation == "built":
            require_directory(args.artifact)
            subject = subject_for(source, args.artifact, binding)
            with (args.artifact / SUBJECT).open("xb") as output:
                output.write(receipt.canonical(subject) + b"\n")
            check_build(source, args.artifact, binding)
    elif args.operation == "finalize":
        proof, archives = finalize(roles[args.role], args.artifact, args.observations, args.verified, binding)
        print("proof_sha256=" + proof)
        print("archives_sha256=" + archives)
    else:
        verify_packet(roles[args.role], args.verified, args.archives,
                      args.proof_sha256, args.archives_sha256, binding)


if __name__ == "__main__":
    main()
